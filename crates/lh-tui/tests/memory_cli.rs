//! `lh memory ...` against a real core.
use std::sync::Arc;
use std::time::Duration;

use lh_core::{auth, router, AppState, MemoryRuntime, Settings};
use lh_memory::embed::AnyEmbedder;
use lh_memory::{Memory, RecallConfig, Trust};
use lh_provider::{ChatRequest, Event, Response};
use lh_store::Store;
use lh_tools::Model;
use lh_tui::client::Client;
use lh_tui::memory_cli::run;

struct Nothing;
impl Model for Nothing {
    async fn complete(&self, _: &ChatRequest, _: &mut (dyn FnMut(Event) + Send)) -> Result<Response, lh_provider::Error> {
        Ok(Response::default())
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
    let app = router(AppState::new(Nothing, settings), token.clone(), port);
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    (d, Client::new(format!("http://127.0.0.1:{port}"), token))
}
fn args(s: &str) -> Vec<String> {
    s.split_whitespace().map(str::to_string).collect()
}
async fn ok(c: &Client, s: &str) -> String {
    run(c, &args(s)).await.unwrap_or_else(|e| panic!("{s}: {e}"))
}

#[tokio::test]
async fn the_viewer_commands_work_end_to_end() {
    let (d, c) = core().await;
    assert_eq!(ok(&c, "list").await, "Nothing is remembered yet.");
    assert!(ok(&c, "help").await.contains("lh memory <command>"));

    let saved = run(&c, &["add".into(), "facts".into(), "Trip".into(), "--".into(), "Flight".into(), "to".into(), "Berlin".into(), "in".into(), "June.".into()]).await.unwrap();
    assert!(saved.starts_with("Saved: p_") && saved.contains("facts") && saved.contains("Trip"), "{saved}");
    let id = saved.split_whitespace().nth(1).unwrap().to_string();
    let secret = run(&c, &["add".into(), "facts".into(), "Pass".into(), "--".into(), "The".into(), "password".into(), "is".into(), "hunter2".into()]).await.unwrap();
    assert!(secret.contains("[sensitive]"), "{secret}");

    let listed = ok(&c, "list facts").await;
    assert_eq!(listed.lines().count(), 2);
    assert!(ok(&c, "search flights").await.contains("Trip"));
    assert_eq!(ok(&c, "search nothingmatches").await, "No matches.");
    assert!(run(&c, &args("search")).await.is_err());

    assert!(ok(&c, &format!("pin {id}")).await.contains("[pinned]"));
    assert!(ok(&c, &format!("sensitive {id}")).await.contains("[sensitive, pinned]"));
    assert!(ok(&c, &format!("normal {id}")).await.contains("Marked normal"));
    let shown = ok(&c, &format!("show {id}")).await;
    assert!(shown.contains("Flight to Berlin in June.") && shown.contains("History:") && shown.contains("create") && shown.contains("reclassify"), "{shown}");

    assert!(ok(&c, &format!("archive {id}")).await.contains("archived]"));
    assert_eq!(ok(&c, "search flights").await, "No matches.");
    assert!(ok(&c, "search flights --archived").await.contains("Trip"));
    assert!(ok(&c, "list --all").await.lines().count() == 2);
    assert!(ok(&c, &format!("restore {id}")).await.starts_with("Restored"));

    std::fs::write(d.path().join("memory/facts/trip.md"), std::fs::read_to_string(d.path().join("memory/facts/trip.md")).unwrap().replace("Berlin", "Rome")).unwrap();
    assert_eq!(ok(&c, "scan").await, "New files: 0, edited: 1, removed (archived): 0");

    assert!(ok(&c, "sent").await.contains("No memory has been sent"));
    assert_eq!(ok(&c, "proposals").await, "No suggestions waiting.");
    assert!(ok(&c, "maintenance").await.contains("Nothing was changed"));
    let out = d.path().join("export");
    assert!(ok(&c, &format!("export {}", out.display())).await.contains("Exported"));
    assert!(run(&c, &args("export relative")).await.is_err());
    assert_eq!(ok(&c, &format!("delete {id}")).await, "Deleted for good, including its older versions.");
    assert!(run(&c, &args(&format!("show {id}"))).await.is_err());
    assert!(run(&c, &args("bogus")).await.unwrap_err().contains("unknown memory command"));
    assert!(run(&c, &args("archive")).await.unwrap_err().contains("missing page id"));
}
