//! The client against a real core: list models, run a turn with a tool question, cancel.
use std::sync::Mutex;
use std::time::Duration;

use lh_core::{auth, router, AppState, Settings};
use lh_provider::*;
use lh_tools::Model;
use lh_tui::app::{App, Entry};
use lh_tui::client::{Client, ServerEvent};

struct Script(Mutex<Vec<Option<Response>>>);
impl Model for Script {
    async fn models(&self) -> Result<Vec<String>, Error> {
        Ok(vec!["alpha".into(), "beta".into()])
    }
    async fn complete(&self, _r: &ChatRequest, on: &mut (dyn FnMut(Event) + Send)) -> Result<Response, Error> {
        let Some(r) = self.0.lock().unwrap().pop().expect("unscripted call") else { return std::future::pending().await };
        if !r.text.is_empty() {
            on(Event::Text(r.text.clone()));
        }
        Ok(r)
    }
}

async fn start(mut replies: Vec<Option<Response>>) -> Client {
    replies.reverse();
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    let token = auth::new_token();
    let app = router(AppState::new(Script(Mutex::new(replies)), Settings { bypass: false, sudo_enabled: false, allow_rules: vec![] }), token.clone(), port);
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    Client::new(format!("http://127.0.0.1:{port}"), token)
}

fn reply(text: &str, calls: Vec<ToolCall>) -> Option<Response> {
    let finish = if calls.is_empty() { FinishReason::Stop } else { FinishReason::ToolCalls };
    Some(Response { text: text.into(), tool_calls: calls, finish: Some(finish), ..Default::default() })
}

#[tokio::test]
async fn models_turn_question_and_wrong_token() {
    let call = ToolCall { id: "c1".into(), name: "write_file".into(), arguments: r#"{"path":"out.txt","content":"hi"}"#.into(), repaired: false };
    let c = start(vec![reply("", vec![call]), reply("All done.", vec![])]).await;
    assert_eq!(c.models().await.unwrap(), ["alpha", "beta"]);

    let dir = tempfile::tempdir().unwrap();
    let id = c.create_session("alpha", dir.path(), false).await.unwrap();
    let mut app = App::new("alpha");
    app.start_turn("write a file".into());
    let mut stream = c.turn(&id, "write a file").await.unwrap();
    while let Some(ev) = stream.next().await.unwrap() {
        if let ServerEvent::Ask { id: ask, .. } = &ev {
            c.answer(&id, ask, true).await.unwrap();
        }
        app.apply(ev);
    }
    assert!(!app.busy);
    assert_eq!(std::fs::read_to_string(dir.path().join("out.txt")).unwrap(), "hi");
    assert!(app.entries.iter().any(|e| matches!(e, Entry::Tool { name, result: Some(_), .. } if name == "write_file")));
    assert!(matches!(app.entries.last(), Some(Entry::Assistant(t)) if t == "All done."));

    let bad = Client::new("http://127.0.0.1:1", "x");
    assert!(bad.models().await.unwrap_err().starts_with("Cannot reach the core"));
}

#[tokio::test]
async fn dropping_the_stream_frees_the_session() {
    let c = start(vec![None, reply("back", vec![])]).await; // first call hangs
    let dir = tempfile::tempdir().unwrap();
    let id = c.create_session("alpha", dir.path(), false).await.unwrap();
    let first = c.turn(&id, "one").await.unwrap();
    assert!(c.turn(&id, "two").await.err().unwrap().contains("already running"));
    drop(first);
    let mut tries = 0;
    let mut stream = loop {
        match c.turn(&id, "again").await {
            Ok(s) => break s,
            Err(_) if tries < 50 => {
                tries += 1;
                tokio::time::sleep(Duration::from_millis(100)).await
            }
            Err(e) => panic!("{e}"),
        }
    };
    let mut got = String::new();
    while let Some(ev) = stream.next().await.unwrap() {
        if let ServerEvent::Text(t) = ev {
            got.push_str(&t);
        }
    }
    assert_eq!(got, "back");
}
