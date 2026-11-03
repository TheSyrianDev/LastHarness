//! The chat tree calls every front end uses (`lh_client::Client`), against a real core.
use std::sync::Arc;
use std::time::Duration;

use lh_client::tree::Moved;
use lh_core::{auth, router, AppState, MemoryRuntime, Settings};
use lh_memory::embed::AnyEmbedder;
use lh_memory::{Memory, RecallConfig, Trust};
use lh_provider::{ChatRequest, Event, FinishReason, Response};
use lh_store::Store;
use lh_tools::Model;
use lh_tui::client::{Client, ServerEvent};

struct Echo;
impl Model for Echo {
    async fn complete(&self, r: &ChatRequest, on: &mut (dyn FnMut(Event) + Send)) -> Result<Response, lh_provider::Error> {
        let last = r.messages.iter().rev().find_map(|m| m.content.clone()).unwrap_or_default();
        let text = format!("re: {last}");
        on(Event::Text(text.clone()));
        Ok(Response { text, finish: Some(FinishReason::Stop), ..Default::default() })
    }
}

async fn core() -> (tempfile::TempDir, Client) {
    let d = tempfile::tempdir().unwrap();
    let store = Arc::new(Store::open(&d.path().join("lh.db")).unwrap());
    let memory = Arc::new(Memory::open(store.clone(), d.path().join("memory")).unwrap());
    let settings = Settings {
        provider_name: None,
        learn: None,
        maintain_when_idle: false,
        memory: Some(MemoryRuntime { memory, embedder: Arc::new(AnyEmbedder::Off), trust: Trust::Local, cfg: RecallConfig::default() }),
        image_model: None,
        web: Default::default(),
        idle_after: Duration::ZERO,
        compact_at: Some(0),
        store: store.clone(),
        spend: lh_spend::SpendMeter::new(store, lh_spend::Config::default(), "t", true),
        bypass: false,
        sudo_enabled: false,
        allow_rules: vec![],
        team: None,
        mcp: None,
        ask_all: false,
        config: None,
    };
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    let token = auth::new_token();
    let app = router(AppState::new(Echo, settings), token.clone(), port);
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    (d, Client::new(format!("http://127.0.0.1:{port}"), token))
}

async fn say(c: &Client, id: &str, text: &str) {
    let mut s = c.turn(id, text).await.unwrap();
    while let Some(e) = s.next().await.unwrap() {
        if matches!(e, ServerEvent::Done { .. }) {
            break;
        }
    }
}

#[tokio::test]
async fn the_tree_calls_work_end_to_end() {
    let (d, c) = core().await;
    let id = c.create_session("m", d.path(), false, false).await.unwrap();
    say(&c, &id, "q1").await;
    say(&c, &id, "q2").await;
    let t = c.chat_tree(&id).await.unwrap();
    assert_eq!(t.rows().len(), 5, "the start and four messages");
    assert_eq!(t.summarize, "ask");
    let a1 = t.nodes.iter().find(|n| n.preview == "re: q1").unwrap().id;
    let q2 = t.nodes.iter().find(|n| n.preview == "q2").unwrap().id;

    c.label_node(&id, a1, "first answer").await.unwrap();
    assert_eq!(c.chat_tree(&id).await.unwrap().node(a1).unwrap().label.as_deref(), Some("first answer"));

    // ask each time: the core asks, and nothing has moved
    assert_eq!(c.move_in_tree(&id, q2, None).await.unwrap(), Moved::NeedsChoice { leaving_messages: 2 });
    assert_eq!(c.chat_tree(&id).await.unwrap().leaf, t.leaf);
    match c.move_in_tree(&id, q2, Some(false)).await.unwrap() {
        Moved::Done { input, summary, left_messages } => assert_eq!((input.as_deref(), summary, left_messages), (Some("q2"), None, 2)),
        other => panic!("{other:?}"),
    }
    // the words are back, and the next message starts a branch
    say(&c, &id, "q2, reworded").await;
    let t = c.chat_tree(&id).await.unwrap();
    assert_eq!(t.nodes.iter().filter(|n| n.role == "user").count(), 3);
    assert_eq!(t.active.len(), 4);

    let (forked, input) = c.fork_chat(&id, t.nodes.iter().find(|n| n.preview == "q2, reworded").unwrap().id).await.unwrap();
    assert_eq!(input.as_deref(), Some("q2, reworded"));
    let cloned = c.clone_chat(&id).await.unwrap();
    assert_ne!(forked, cloned);
    // a private chat has no tree, and says so
    let private = c.create_session("m", d.path(), false, true).await.unwrap();
    say(&c, &private, "secret").await;
    assert!(c.chat_tree(&private).await.unwrap_err().contains("private"));
}
