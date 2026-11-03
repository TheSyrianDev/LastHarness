//! `lh`: terminal chat for the LastHarness core.
//!
//! lh [--url http://127.0.0.1:7842] [--token-file PATH] [--model NAME] [--ask-all] [--incognito]
//! lh -p "question" [--json] [--model NAME] [--bypass-permissions]   one turn, answer on standard output (see `headless`)
//! lh memory <command>     (see `lh memory help`)
//! lh learn <command>      (see `lh learn help`)
//! lh team <command>       (see `lh team help`)
//! lh service [preview|install|remove|status]  start the core at login (systemd user service)
//! lh channels [add|enable|...]  messaging channels (see `lh channels help`)
//! lh devices [accept PIN|remove ID]
//! lh schedule [add|on|off|remove]  tasks the core runs at set times
//! lh export <folder>; lh import <folder> [--apply]
//! lh setup                first-run questions; lh settings [group|set|reset]
//! lh mcp <command>        (see `lh mcp help`); lh trust <folder>

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

/// What was said on a branch: (role, text) in order.
type Said = Vec<(String, String)>;

enum Msg {
    Event(ServerEvent),
    End,
    Review(Result<Vec<Suggestion>, String>),
    /// The chat tree, opened or refreshed.
    Tree(Result<lh_client::tree::ChatTree, String>),
    /// A line of feedback for the tree screen (or the chat when it is closed).
    TreeNote(Result<String, String>),
    /// The answer to going to a message in the tree.
    Moved(i64, Result<lh_client::tree::Moved, String>),
    /// The active branch after a move: what was said, and the words for the input box.
    Branch(Result<(Said, Option<String>), String>),
}

/// Ask the core for the active branch of this chat, after a move.
fn fetch_branch(client: Arc<Client>, session: String, input: Option<String>, tx: mpsc::UnboundedSender<Msg>) {
    tokio::spawn(async move {
        let r = client.call("GET", &format!("/v1/chats/{session}"), None).await.map(|v| {
            let said = v["history"].as_array().map(|a| a.iter().map(|m| (m["role"].as_str().unwrap_or("").to_string(), m["content"].as_str().unwrap_or("").to_string())).collect()).unwrap_or_default();
            (said, input)
        });
        let _ = tx.send(Msg::Branch(r));
    });
}

#[tokio::main]
pub async fn run() {
    let mut a = std::env::args().skip(1).peekable();
    let (mut url, mut token_file, mut model, mut ask_all, mut incognito) = ("http://127.0.0.1:7842".to_string(), default_token_path(), None, false, false);
    let mut memory_cmd: Option<Vec<String>> = None;
    let mut learn_cmd: Option<Vec<String>> = None;
    let mut team_cmd: Option<Vec<String>> = None;
    let mut mcp_cmd: Option<(bool, Vec<String>)> = None;
    let mut settings_cmd: Option<Vec<String>> = None;
    let mut do_setup = false;
    let mut data_cmd: Option<(bool, Vec<String>)> = None;
    let mut devices_cmd: Option<Vec<String>> = None;
    let mut channels_cmd: Option<Vec<String>> = None;
    let mut service_cmd: Option<Vec<String>> = None;
    let mut schedule_cmd: Option<Vec<String>> = None;
    let (mut print, mut as_json, mut bypass): (Option<String>, bool, bool) = (None, false, false);
    let mut no_sandbox = false;
    while let Some(arg) = a.next() {
        let mut val = || a.next().unwrap_or_else(|| fail(format!("{arg} needs a value")));
        match arg.as_str() {
            "--url" => url = val(),
            "--token-file" => token_file = PathBuf::from(val()),
            "--model" => model = Some(val()),
            "--ask-all" => ask_all = true,
            "--incognito" => incognito = true,
            "-p" | "--print" => print = Some(val()),
            "--json" => as_json = true,
            "--bypass-permissions" => bypass = true,
            "--no-sandbox" => no_sandbox = true,
            "--help" | "-h" => {
                println!("lh [--url URL] [--token-file PATH] [--model NAME] [--ask-all] [--incognito]    chat with the core in the terminal\nlh -p \"question\" [--json] [--model NAME]    one turn: the answer on standard output (\"-\" reads the question from standard input; piped text is added to it)\n    Permission questions are answered no (exit code 3). --bypass-permissions answers them yes: the run may write files and run commands without asking. The sandbox stays on.\n    --no-sandbox runs this turn's commands without the sandbox (they can change any of your files and reach the network).\nOther commands: setup, settings, memory, learn, team, mcp, trust, devices, channels, schedule, export, import, service. Run `lh <command> help` for each.");
                return;
            }
            "learn" => {
                learn_cmd = Some(a.by_ref().collect());
                break;
            }
            "service" => {
                service_cmd = Some(a.by_ref().collect());
                break;
            }
            "channels" => {
                channels_cmd = Some(a.by_ref().collect());
                break;
            }
            "schedule" => {
                schedule_cmd = Some(a.by_ref().collect());
                break;
            }
            "devices" => {
                devices_cmd = Some(a.by_ref().collect());
                break;
            }
            "export" | "import" => {
                data_cmd = Some((arg == "export", a.by_ref().collect()));
                break;
            }
            "setup" => {
                do_setup = true;
                break;
            }
            "settings" => {
                settings_cmd = Some(a.by_ref().collect());
                break;
            }
            "mcp" | "trust" => {
                mcp_cmd = Some((arg == "trust", a.by_ref().collect()));
                break;
            }
            "team" => {
                team_cmd = Some(a.by_ref().collect());
                break;
            }
            "memory" => {
                memory_cmd = Some(a.by_ref().collect());
                break;
            }
            other => fail(format!("unknown option {other}")),
        }
    }
    if do_setup {
        // the questions read the keyboard and probe the network with blocking calls, so they get their own thread
        let r = tokio::task::spawn_blocking(|| {
            let mut cfg = lh_config::ConfigFile::load(lh_config::ConfigFile::default_path()).map_err(|e| e.to_string())?;
            crate::setup::run(&mut crate::setup::Terminal, &mut cfg, &crate::setup::probe_http, &crate::setup::save_secret_os)
        })
        .await;
        match r {
            Ok(Ok(())) => {}
            Ok(Err(e)) => fail(e),
            Err(e) => fail(e),
        }
        return;
    }
    if let Some(cmd) = service_cmd {
        let mut ask = |q: &str| {
            use std::io::Write;
            print!("{q} ");
            let _ = std::io::stdout().flush();
            let mut l = String::new();
            let _ = std::io::stdin().read_line(&mut l);
            l.trim().to_lowercase().starts_with('y')
        };
        match crate::service_cli::run(&cmd, crate::service_cli::find_lhd(), &mut ask, &mut crate::service_cli::real_systemctl) {
            Ok(text) => println!("{text}"),
            Err(e) => fail(e),
        }
        return;
    }
    // help needs no running core
    let wants_help = |c: &Option<Vec<String>>| c.as_ref().is_some_and(|c| matches!(c.first().map(String::as_str), Some("help" | "--help" | "-h")));
    for (cmd, text) in [
        (&learn_cmd, crate::learn_cli::HELP),
        (&memory_cmd, crate::memory_cli::HELP),
        (&team_cmd, crate::team_cli::HELP),
        (&settings_cmd, crate::settings_cli::HELP),
        (&devices_cmd, crate::devices_cli::HELP),
        (&channels_cmd, crate::channels_cli::HELP),
        (&schedule_cmd, crate::schedule_cli::HELP),
    ] {
        if wants_help(cmd) {
            println!("{text}");
            return;
        }
    }
    if wants_help(&mcp_cmd.as_ref().map(|m| m.1.clone())) {
        println!("{}", crate::mcp_cli::HELP);
        return;
    }
    if wants_help(&data_cmd.as_ref().map(|m| m.1.clone())) {
        println!("{}", crate::data_cli::HELP);
        return;
    }
    let client = Arc::new(Client::from_token_file(&url, &token_file).unwrap_or_else(|e| fail(e)));
    if let Some(cmd) = learn_cmd {
        match crate::learn_cli::run(&client, &cmd).await {
            Ok(text) => println!("{text}"),
            Err(e) => fail(e),
        }
        return;
    }
    if let Some(cmd) = channels_cmd {
        let r = {
            let mut ask = |q: &str| {
                use std::io::Write;
                print!("{q} ");
                let _ = std::io::stdout().flush();
                let mut l = String::new();
                let _ = std::io::stdin().read_line(&mut l);
                l.trim().to_lowercase().starts_with('y')
            };
            crate::channels_cli::run(&client, &cmd, &mut ask).await
        };
        match r {
            Ok(text) => println!("{text}"),
            Err(e) => fail(e),
        }
        return;
    }
    if let Some(cmd) = schedule_cmd {
        match crate::schedule_cli::run(&client, &cmd, model.clone()).await {
            Ok(text) => println!("{text}"),
            Err(e) => fail(e),
        }
        return;
    }
    if let Some(cmd) = devices_cmd {
        match crate::devices_cli::run(&client, &cmd).await {
            Ok(text) => println!("{text}"),
            Err(e) => fail(e),
        }
        return;
    }
    if let Some((is_export, cmd)) = data_cmd {
        let r = if is_export { crate::data_cli::export(&client, &cmd).await } else { crate::data_cli::import(&client, &cmd).await };
        match r {
            Ok(text) => println!("{text}"),
            Err(e) => fail(e),
        }
        return;
    }
    if let Some(cmd) = settings_cmd {
        match crate::settings_cli::run(&client, &cmd, &lh_config::ConfigFile::default_path()).await {
            Ok(text) => println!("{text}"),
            Err(e) => fail(e),
        }
        return;
    }
    if let Some((is_trust, cmd)) = mcp_cmd {
        let r = if is_trust { crate::mcp_cli::trust(&client, &cmd).await } else { crate::mcp_cli::run(&client, &cmd).await };
        match r {
            Ok(text) => println!("{text}"),
            Err(e) => fail(e),
        }
        return;
    }
    if let Some(cmd) = team_cmd {
        match crate::team_cli::run(&client, &cmd).await {
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
    if bypass && print.is_none() {
        fail("--bypass-permissions works only with -p");
    }
    if no_sandbox && print.is_none() {
        fail("--no-sandbox works only with -p (for the core: lhd --no-sandbox)");
    }
    if let Some(given) = print {
        use std::io::IsTerminal;
        if bypass {
            eprintln!("{}", crate::headless::BYPASS_WARNING);
        }
        if no_sandbox {
            eprintln!("{}", crate::headless::NO_SANDBOX_WARNING);
        }
        let text = crate::headless::prompt(&given, &mut io::stdin(), io::stdin().is_terminal());
        if text.trim().is_empty() {
            fail("the question is empty");
        }
        // no one is there to pick a model: --model, then LH_MODEL, then the only model the server has
        let model = match model.or_else(|| std::env::var("LH_MODEL").ok().filter(|m| !m.is_empty())) {
            Some(m) => m,
            None => match client.models().await.unwrap_or_else(|e| fail(e)).as_slice() {
                [only] => only.clone(),
                _ => fail("pick a model with --model NAME or the LH_MODEL variable"),
            },
        };
        let dir = std::env::current_dir().unwrap_or_else(|e| fail(format!("cannot read the current folder: {e}")));
        let session = client.create_session_with(&model, &dir, ask_all, incognito, no_sandbox).await.unwrap_or_else(|e| fail(e));
        let end = crate::headless::run(&client, &session, &text, as_json, bypass, &mut io::stdout(), &mut io::stderr()).await;
        let _ = client.call("DELETE", &format!("/v1/sessions/{session}"), None).await;
        std::process::exit(end.code());
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
                Msg::Tree(Ok(t)) => match app.tree.as_mut() { Some(open) => open.replace_tree(t), None => app.tree = Some(crate::tree_ui::TreeScreen::new(t)) },
                Msg::Tree(Err(e)) => app.entries.push(crate::app::Entry::Error(e)),
                Msg::TreeNote(r) => {
                    let text = r.unwrap_or_else(|e| format!("error: {e}"));
                    match app.tree.as_mut() { Some(open) => open.note = text, None => app.entries.push(crate::app::Entry::Notice(text)) }
                }
                Msg::Moved(node, Ok(lh_client::tree::Moved::NeedsChoice { leaving_messages })) => {
                    if let Some(open) = app.tree.as_mut() { open.prompt = crate::tree_ui::Prompt::Summary { node, leaving: leaving_messages }; }
                }
                Msg::Moved(_, Ok(lh_client::tree::Moved::Done { input, .. })) => {
                    app.tree = None;
                    fetch_branch(client.clone(), session.clone(), input, tx.clone());
                }
                Msg::Moved(_, Err(e)) => match app.tree.as_mut() { Some(open) => open.note = format!("error: {e}"), None => app.entries.push(crate::app::Entry::Error(e)) },
                Msg::Branch(Ok((history, input))) => app.show_branch(&history, input),
                Msg::Branch(Err(e)) => app.entries.push(crate::app::Entry::Error(e)),
                Msg::End => { turn = None; if app.busy { app.apply(ServerEvent::Error("The core closed the connection before the answer was finished.".into())); } }
            },
            Some(Ok(ev)) = keys.next() => {
                let Event::Key(k) = ev else { continue };
                if k.kind != KeyEventKind::Press { continue }
                let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
                if let Some(open) = app.tree.as_mut() {
                    use crate::tree_ui::Action;
                    let (c, s, tx) = (client.clone(), session.clone(), tx.clone());
                    match open.key(k.code, k.modifiers) {
                        Action::Nothing => {}
                        Action::Close => app.tree = None,
                        Action::Go(node) => { tokio::spawn(async move { let _ = tx.send(Msg::Moved(node, c.move_in_tree(&s, node, None).await)); }); }
                        Action::GoWithSummary(node, yes) => { open.note = if yes { "Writing the summary…".into() } else { String::new() }; tokio::spawn(async move { let _ = tx.send(Msg::Moved(node, c.move_in_tree(&s, node, Some(yes)).await)); }); }
                        Action::Label(node, text) => { tokio::spawn(async move { let r = match c.label_node(&s, node, &text).await { Ok(()) => c.chat_tree(&s).await, Err(e) => Err(e) }; let _ = tx.send(Msg::Tree(r)); }); }
                        Action::Fork(node) => { tokio::spawn(async move { let r = c.fork_chat(&s, node).await.map(|(id, input)| format!("Made a new chat ({id}) from here{}. Open it from the chat list in the desktop app.", if input.is_some() { ", with that message ready to send again" } else { "" })); let _ = tx.send(Msg::TreeNote(r)); }); }
                    }
                    continue;
                }
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
                    KeyCode::Enter if !app.busy && app.input.trim() == "/tree" => {
                        app.input.clear();
                        let (c, s, tx) = (client.clone(), session.clone(), tx.clone());
                        tokio::spawn(async move { let _ = tx.send(Msg::Tree(c.chat_tree(&s).await)); });
                    }
                    KeyCode::Enter if !app.busy && app.input.trim() == "/clone" => {
                        app.input.clear();
                        let (c, s, tx) = (client.clone(), session.clone(), tx.clone());
                        tokio::spawn(async move { let r = c.clone_chat(&s).await.map(|id| format!("Copied this branch into a new chat ({id}). Open it from the chat list in the desktop app.")); let _ = tx.send(Msg::TreeNote(r)); });
                    }
                    KeyCode::Enter if !app.busy && matches!(app.input.trim(), "/learn" | "/refine") => {
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
