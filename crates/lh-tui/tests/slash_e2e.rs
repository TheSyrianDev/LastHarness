//! `/attach`, `/detach` and `/review` end to end: the shared parser, the terminal's staging, and a real core over HTTP.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use lh_client::slash::{self, Slash};
use lh_client::{attach::Stage, Client};
use lh_core::{auth, router, AppState, Settings};
use lh_provider::*;
use lh_tools::Model;
use lh_tui::app::{App, Entry};
use serde_json::json;

struct Script {
    replies: Mutex<Vec<Option<Response>>>,
    seen: Arc<Mutex<Vec<ChatRequest>>>,
}

impl Model for Script {
    async fn models(&self) -> Result<Vec<String>, Error> {
        Ok(vec!["alpha".into()])
    }
    async fn complete(&self, r: &ChatRequest, on: &mut (dyn FnMut(Event) + Send)) -> Result<Response, Error> {
        self.seen.lock().unwrap().push(r.clone());
        let Some(r) = self.replies.lock().unwrap().pop().expect("unscripted call") else { return std::future::pending().await };
        if !r.text.is_empty() {
            on(Event::Text(r.text.clone()));
        }
        Ok(r)
    }
}

fn free_meter() -> lh_spend::SpendMeter {
    lh_spend::SpendMeter::new(Arc::new(lh_store::Store::open_memory().unwrap()), lh_spend::Config::default(), "test", true)
}

/// A real core on a random port. Replies are used last to first.
async fn start(mut replies: Vec<Option<Response>>) -> (Client, Arc<Mutex<Vec<ChatRequest>>>) {
    replies.reverse();
    let seen = Arc::new(Mutex::new(vec![]));
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    let token = auth::new_token();
    let script = Script { replies: Mutex::new(replies), seen: seen.clone() };
    // /review needs the memory store on; the temp dir is leaked so it outlives the test server
    let dir = tempfile::tempdir().unwrap().keep();
    let store = Arc::new(lh_store::Store::open_memory().unwrap());
    let memory = Arc::new(lh_memory::Memory::open(store.clone(), dir.join("memory")).unwrap());
    let settings = Settings { provider_name: None, learn: None, maintain_when_idle: false, memory: Some(lh_core::MemoryRuntime { memory, embedder: Arc::new(lh_memory::embed::AnyEmbedder::Off), trust: lh_memory::Trust::Local, cfg: lh_memory::RecallConfig::default() }), image_model: None, web: Default::default(), idle_after: Duration::ZERO, compact_at: Some(0), store, spend: free_meter(), bypass: false, sudo_enabled: false, allow_rules: vec![], team: None, mcp: None, ask_all: false, config: None };
    let app = router(AppState::new(script, settings), token.clone(), port);
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    (Client::new(format!("http://127.0.0.1:{port}"), token), seen)
}

fn text(t: &str) -> Option<Response> {
    Some(Response { text: t.into(), finish: Some(FinishReason::Stop), ..Default::default() })
}

#[tokio::test]
async fn attach_sends_the_file_to_the_model_then_detach_and_review_work() {
    let (c, seen) = start(vec![text("got it"), text("[]")]).await;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("notes.txt");
    std::fs::write(&file, "hello attach").unwrap();
    let id = c.create_session("alpha", dir.path(), false, false).await.unwrap();

    // /attach <path>: the shared parser gives a screen command, the terminal stages the file
    let line = format!("/attach {}", file.display());
    assert_eq!(slash::parse(&line), Some(Ok(Slash::Attach(file.display().to_string()))));
    let mut app = App::new("alpha");
    assert!(app.attach_command(&line));
    assert_eq!(app.stage.files.len(), 1);
    assert!(app.chips().starts_with("[1 notes.txt 12 B]"), "{}", app.chips());

    // the send carries the staged file; the core hands it to the model
    let staged = app.stage.take();
    let mut stream = c.turn_with(&id, "read it", &staged).await.unwrap();
    while let Some(ev) = stream.next().await.unwrap() {
        app.apply(ev);
    }
    assert!(app.stage.is_empty());
    let sent = seen.lock().unwrap();
    let user = sent[0].messages.iter().rev().find(|m| m.role == Role::User).expect("user message");
    assert_eq!(user.attachments.len(), 1);
    assert_eq!(user.attachments[0].name, "notes.txt");
    drop(sent);

    // /attach with a missing file: a plain sentence, nothing staged
    assert!(app.attach_command("/attach /no/such/file.txt"));
    assert!(matches!(app.entries.last(), Some(Entry::Notice(t)) if t.contains("no file")));
    assert!(app.stage.is_empty());

    // /detach: out-of-range is refused, a real number removes the staged file
    assert!(app.attach_command(&format!("/attach {}", file.display())));
    assert!(app.attach_command("/detach 9"));
    assert!(matches!(app.entries.last(), Some(Entry::Notice(t)) if t.contains("Say which one")));
    assert_eq!(app.stage.files.len(), 1);
    assert!(app.attach_command("/detach 1"));
    assert!(app.stage.is_empty());
    assert_eq!(slash::parse("/detach 1"), Some(Ok(Slash::Detach("1".into()))));

    // /review: the screen asks the core for suggestions; the core runs its review over the session
    assert_eq!(slash::parse("/review"), Some(Ok(Slash::Review)));
    assert!(Slash::Review.is_screen());
    let review = c.call("POST", &format!("/v1/sessions/{id}/review"), Some(json!({}))).await.unwrap();
    assert!(review.is_object(), "{review}");
}

#[tokio::test]
async fn screen_commands_are_refused_by_the_core_path() {
    // the core runner refuses these; the terminal never sends them as turns
    assert!(slash::run(&Client::new("http://127.0.0.1:1", "x"), "s", Slash::Review).await.unwrap_err().contains("handled by the screen"));
    assert!(slash::run(&Client::new("http://127.0.0.1:1", "x"), "s", Slash::Detach("1".into())).await.unwrap_err().contains("/detach"));
}

#[tokio::test]
async fn stage_refuses_a_directory_and_keeps_order() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.txt");
    let b = dir.path().join("b.txt");
    std::fs::write(&a, "a").unwrap();
    std::fs::write(&b, "bb").unwrap();
    let mut s = Stage::default();
    assert!(s.add(dir.path()).unwrap_err().contains("is not a file"));
    s.add(&b).unwrap();
    s.add(&a).unwrap();
    s.remove(0);
    assert_eq!(s.files[0].name, "a.txt");
}
