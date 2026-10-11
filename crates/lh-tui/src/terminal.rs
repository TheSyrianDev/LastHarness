//! `lh`: terminal chat for the LastHarness core.
//!
//! lh [--url http://127.0.0.1:7842] [--token-file PATH] [--model NAME] [--ask-all] [--incognito]
//! lh memory <command>     (see `lh memory help`)
//! lh learn <command>      (see `lh learn help`)

use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use futures_util::StreamExt;
use crate::app::{App, Suggestion};
use crate::client::{Client, ServerEvent};
use crate::ui;
use ratatui::prelude::*;
use tokio::sync::mpsc;

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("lh: {msg}");
    std::process::exit(2)
}

fn default_token_path() -> PathBuf {
    match std::env::var_os("XDG_RUNTIME_DIR") {
        Some(d) => PathBuf::from(d).join("lastharness/token"),
        None => PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state/lastharness/token"),
    }
}

enum Msg {
    Event(ServerEvent),
    End,
    Review(Result<Vec<Suggestion>, String>),
}

#[tokio::main]
pub async fn run() {
    let mut a = std::env::args().skip(1).peekable();
    let (mut url, mut token_file, mut model, mut ask_all, mut incognito) = ("http://127.0.0.1:7842".to_string(), default_token_path(), None, false, false);
    let mut memory_cmd: Option<Vec<String>> = None;
    let mut learn_cmd: Option<Vec<String>> = None;
    while let Some(arg) = a.next() {
        let mut val = || a.next().unwrap_or_else(|| fail(format!("{arg} needs a value")));
        match arg.as_str() {
            "--url" => url = val(),
            "--token-file" => token_file = PathBuf::from(val()),
            "--model" => model = Some(val()),
            "--ask-all" => ask_all = true,
            "--incognito" => incognito = true,
            "learn" => {
                learn_cmd = Some(a.by_ref().collect());
                break;
            }
            "memory" => {
                memory_cmd = Some(a.by_ref().collect());
                break;
            }
            other => fail(format!("unknown option {other}")),
        }
    }
    let client = Arc::new(Client::from_token_file(&url, &token_file).unwrap_or_else(|e| fail(e)));
    if let Some(cmd) = learn_cmd {
        match crate::learn_cli::run(&client, &cmd).await {
            Ok(text) => println!("{text}"),
            Err(e) => fail(e),
        }
        return;
    }
    if let Some(cmd) = memory_cmd {
        match crate::memory_cli::run(&client, &cmd).await {
            Ok(text) => println!("{text}"),
            Err(e) => fail(e),
        }
        return;
    }
    let model = match model {
        Some(m) => m,
        None => pick_model(&client).await,
    };
    let dir = std::env::current_dir().unwrap_or_else(|e| fail(format!("cannot read the current folder: {e}")));
    let session = client.create_session(&model, &dir, ask_all, incognito).await.unwrap_or_else(|e| fail(e));

    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |p| {
        let _ = disable_raw_mode();
        let _ = crossterm::execute!(io::stdout(), LeaveAlternateScreen);
        hook(p);
    }));
    enable_raw_mode().unwrap_or_else(|e| fail(e));
    crossterm::execute!(io::stdout(), EnterAlternateScreen).unwrap_or_else(|e| fail(e));
    let mut term = Terminal::new(CrosstermBackend::new(io::stdout())).unwrap_or_else(|e| fail(e));
    let mut app = App::new(model);
    app.incognito = incognito;
    let result = event_loop(&mut term, client, session, app).await;
    let _ = disable_raw_mode();
    let _ = crossterm::execute!(io::stdout(), LeaveAlternateScreen);
    if let Err(e) = result {
        fail(e);
    }
}

async fn pick_model(client: &Client) -> String {
    let models = client.models().await.unwrap_or_else(|e| fail(e));
    if models.is_empty() {
        fail("the server lists no models. Pass one with --model NAME.");
    }
    for (i, m) in models.iter().enumerate() {
        println!("{:>3}  {m}", i + 1);
    }
    loop {
        print!("Pick a model (number): ");
        let _ = io::stdout().flush();
        let mut line = String::new();
        if io::stdin().read_line(&mut line).unwrap_or(0) == 0 {
            fail("no model chosen");
        }
        if let Some(m) = line.trim().parse::<usize>().ok().and_then(|n| models.get(n.wrapping_sub(1))) {
            return m.clone();
        }
    }
}

async fn event_loop(term: &mut Terminal<CrosstermBackend<io::Stdout>>, client: Arc<Client>, session: String, mut app: App) -> io::Result<()> {
    let mut keys = EventStream::new();
    let (tx, mut rx) = mpsc::unbounded_channel::<Msg>();
    let mut turn: Option<tokio::task::JoinHandle<()>> = None;
    loop {
        term.draw(|f| ui::draw(f, &app))?;
        tokio::select! {
            Some(msg) = rx.recv() => match msg {
                Msg::Event(e) => app.apply(e),
                Msg::Review(Ok(list)) if list.is_empty() => app.entries.push(crate::app::Entry::Notice("Nothing new to remember from this conversation.".into())),
                Msg::Review(Ok(list)) => app.review = list,
                Msg::Review(Err(e)) => app.entries.push(crate::app::Entry::Error(e)),
                Msg::End => { turn = None; if app.busy { app.apply(ServerEvent::Error("The core closed the connection before the answer was finished.".into())); } }
            },
            Some(Ok(ev)) = keys.next() => {
                let Event::Key(k) = ev else { continue };
                if k.kind != KeyEventKind::Press { continue }
                let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
                if !app.review.is_empty() {
                    let (c, id) = (client.clone(), app.review[0].id);
                    match k.code {
                        KeyCode::Char('p') => { let s = &mut app.review[0]; s.sensitive = !s.sensitive; }
                        KeyCode::Char('s') => { let first = app.review.remove(0); app.review.push(first); }
                        KeyCode::Esc => app.review.clear(),
                        KeyCode::Char('n') => {
                            let sg = app.review.remove(0);
                            let base = if sg.learn { "learn" } else { "memory" };
                            tokio::spawn(async move { let _ = c.call("POST", &format!("/v1/{base}/proposals/{id}/reject"), Some(serde_json::json!({}))).await; });
                        }
                        KeyCode::Char('y') => {
                            let sg = app.review.remove(0);
                            tokio::spawn(async move {
                                let privacy = if sg.sensitive { "sensitive" } else { "normal" };
                                let (path, body) = if sg.learn { (format!("/v1/learn/proposals/{id}/accept"), serde_json::json!({"edit": {"privacy": privacy}})) } else { (format!("/v1/memory/proposals/{id}/accept"), serde_json::json!({"privacy": privacy})) };
                                let _ = c.call("POST", &path, Some(body)).await;
                            });
                        }
                        _ => {}
                    }
                    continue;
                }
                if let Some(q) = app.question.clone() {
                    let yes = match k.code { KeyCode::Char('y' | 'Y') => true, KeyCode::Char('n' | 'N') | KeyCode::Esc => false, _ => continue };
                    app.question = None;
                    let (c, s) = (client.clone(), session.clone());
                    tokio::spawn(async move { let _ = c.answer(&s, &q.id, yes).await; });
                    continue;
                }
                match k.code {
                    KeyCode::Char('d') if ctrl => break,
                    KeyCode::Char('c') if ctrl && !app.busy => break,
                    KeyCode::Char('c') if ctrl => { cancel(&mut turn, &mut app); }
                    KeyCode::Esc if app.busy => { cancel(&mut turn, &mut app); }
                    KeyCode::Char('t') if ctrl => app.show_thinking = !app.show_thinking,
                    KeyCode::PageUp => app.scroll = app.scroll.saturating_add(10),
                    KeyCode::PageDown => app.scroll = app.scroll.saturating_sub(10),
                    KeyCode::Backspace => { app.input.pop(); }
                    KeyCode::Char(c) if !ctrl => app.input.push(c),
                    KeyCode::Enter if !app.busy && app.input.trim() == "/learn" => {
                        app.input.clear();
                        let (c, s, tx) = (client.clone(), session.clone(), tx.clone());
                        tokio::spawn(async move {
                            let r = c.call("POST", &format!("/v1/sessions/{s}/learn"), Some(serde_json::json!({}))).await
                                .map(|v| v["proposals"].as_array().map(|a| a.iter().map(Suggestion::from_learn_json).collect()).unwrap_or_default());
                            let _ = tx.send(Msg::Review(r));
                        });
                    }
                    KeyCode::Enter if !app.busy && (app.input.trim() == "/good" || app.input.trim().starts_with("/bad")) => {
                        let line = std::mem::take(&mut app.input);
                        let up = line.trim() == "/good";
                        let note = line.trim().trim_start_matches("/bad").trim().to_string();
                        app.entries.push(crate::app::Entry::Notice(if up { "Noted: that was good.".into() } else { "Noted: that was not right. Say what to do instead and it can become a rule.".into() }));
                        let (c, s) = (client.clone(), session.clone());
                        tokio::spawn(async move { let _ = c.call("POST", &format!("/v1/sessions/{s}/feedback"), Some(serde_json::json!({"kind": if up { "up" } else { "down" }, "note": note}))).await; });
                    }
                    KeyCode::Enter if !app.busy && app.input.trim() == "/review" => {
                        app.input.clear();
                        let (c, s, tx) = (client.clone(), session.clone(), tx.clone());
                        tokio::spawn(async move {
                            let r = c.call("POST", &format!("/v1/sessions/{s}/review"), Some(serde_json::json!({}))).await
                                .map(|v| v["proposals"].as_array().map(|a| a.iter().map(Suggestion::from_json).collect()).unwrap_or_default());
                            let _ = tx.send(Msg::Review(r));
                        });
                    }
                    KeyCode::Enter if !app.busy && !app.input.trim().is_empty() => {
                        let text = std::mem::take(&mut app.input);
                        app.start_turn(text.clone());
                        let (c, s, tx) = (client.clone(), session.clone(), tx.clone());
                        turn = Some(tokio::spawn(async move {
                            match c.turn(&s, &text).await {
                                Err(e) => { let _ = tx.send(Msg::Event(ServerEvent::Error(e))); }
                                Ok(mut stream) => loop {
                                    match stream.next().await {
                                        Ok(Some(e)) => { let _ = tx.send(Msg::Event(e)); }
                                        Ok(None) => break,
                                        Err(e) => { let _ = tx.send(Msg::Event(ServerEvent::Error(e))); break }
                                    }
                                },
                            }
                            let _ = tx.send(Msg::End);
                        }));
                    }
                    _ => {}
                }
            },
            else => break,
        }
    }
    if let Some(t) = turn {
        t.abort();
    }
    Ok(())
}

/// Dropping the task drops the connection, which tells the core to stop the turn.
fn cancel(turn: &mut Option<tokio::task::JoinHandle<()>>, app: &mut App) {
    if let Some(t) = turn.take() {
        t.abort();
    }
    app.cancelled();
}
