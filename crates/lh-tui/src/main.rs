//! `lh`: terminal chat for the LastHarness core.
//!
//! lh [--url http://127.0.0.1:7842] [--token-file PATH] [--model NAME] [--ask-all]

use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use futures_util::StreamExt;
use lh_tui::app::App;
use lh_tui::client::{Client, ServerEvent};
use lh_tui::ui;
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
}

#[tokio::main]
async fn main() {
    let mut a = std::env::args().skip(1);
    let (mut url, mut token_file, mut model, mut ask_all) = ("http://127.0.0.1:7842".to_string(), default_token_path(), None, false);
    while let Some(arg) = a.next() {
        let mut val = || a.next().unwrap_or_else(|| fail(format!("{arg} needs a value")));
        match arg.as_str() {
            "--url" => url = val(),
            "--token-file" => token_file = PathBuf::from(val()),
            "--model" => model = Some(val()),
            "--ask-all" => ask_all = true,
            other => fail(format!("unknown option {other}")),
        }
    }
    let client = Arc::new(Client::from_token_file(&url, &token_file).unwrap_or_else(|e| fail(e)));
    let model = match model {
        Some(m) => m,
        None => pick_model(&client).await,
    };
    let dir = std::env::current_dir().unwrap_or_else(|e| fail(format!("cannot read the current folder: {e}")));
    let session = client.create_session(&model, &dir, ask_all).await.unwrap_or_else(|e| fail(e));

    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |p| {
        let _ = disable_raw_mode();
        let _ = crossterm::execute!(io::stdout(), LeaveAlternateScreen);
        hook(p);
    }));
    enable_raw_mode().unwrap_or_else(|e| fail(e));
    crossterm::execute!(io::stdout(), EnterAlternateScreen).unwrap_or_else(|e| fail(e));
    let mut term = Terminal::new(CrosstermBackend::new(io::stdout())).unwrap_or_else(|e| fail(e));
    let result = run(&mut term, client, session, App::new(model)).await;
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

async fn run(term: &mut Terminal<CrosstermBackend<io::Stdout>>, client: Arc<Client>, session: String, mut app: App) -> io::Result<()> {
    let mut keys = EventStream::new();
    let (tx, mut rx) = mpsc::unbounded_channel::<Msg>();
    let mut turn: Option<tokio::task::JoinHandle<()>> = None;
    loop {
        term.draw(|f| ui::draw(f, &app))?;
        tokio::select! {
            Some(msg) = rx.recv() => match msg {
                Msg::Event(e) => app.apply(e),
                Msg::End => { turn = None; if app.busy { app.apply(ServerEvent::Error("The core closed the connection before the answer was finished.".into())); } }
            },
            Some(Ok(ev)) = keys.next() => {
                let Event::Key(k) = ev else { continue };
                if k.kind != KeyEventKind::Press { continue }
                let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
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
