//! Export from one core and import into another.
use std::sync::Arc;
use std::time::Duration;

use lh_core::{auth, router, AppState, LearnMode, LearnRuntime, MemoryRuntime, Settings, TeamRuntime};
use lh_learn::Learn;
use lh_memory::embed::AnyEmbedder;
use lh_memory::{Memory, RecallConfig, Trust};
use lh_provider::{ChatRequest, Event, Response};
use lh_store::Store;
use lh_tools::Model;
use lh_tui::client::Client;
use lh_tui::data_cli::{export, import};
use serde_json::json;

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
    let learn = Arc::new(Learn::new(store.clone()).with_skills_dir(d.path().join("skills")));
    let team = TeamRuntime::open(&d.path().join("team"), learn.clone(), 1000).unwrap();
    let settings = Settings {
        provider_name: None,
        learn: Some(LearnRuntime { learn, trust: Trust::Local, mode: LearnMode::Propose, threshold: 3, skill_min_calls: 6 }),
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
        team: Some(team),
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
fn args(s: &[&str]) -> Vec<String> {
    s.iter().map(|s| s.to_string()).collect()
}

#[tokio::test]
async fn memory_rules_style_and_roles_move_to_another_machine_without_replacing_what_is_there() {
    let (a_dir, a) = core().await;
    a.call("POST", "/v1/memory/pages", Some(json!({"layer": "facts", "title": "Home city", "text": "Lives in Amman."}))).await.unwrap();
    a.call("POST", "/v1/learn/rules", Some(json!({"text": "Always add links after research.", "topic": "links"}))).await.unwrap();
    a.call("POST", "/v1/learn/rules", Some(json!({"text": "My doctor is Dr. Amal; for health questions mention her diabetes plan."}))).await.unwrap();
    a.call("PUT", "/v1/learn/style", Some(json!({"text": "Terse. Plain words."}))).await.unwrap();
    let writer = a.call("GET", "/v1/team/roles/writer", None).await.unwrap()["text"].as_str().unwrap().to_string() + "Always cite sources.\n";
    a.call("PUT", "/v1/team/roles/writer", Some(json!({"text": writer}))).await.unwrap();
    let dest = a_dir.path().join("out");
    let out = export(&a, &args(&[dest.to_str().unwrap()])).await.unwrap();
    assert!(out.contains("rules: 2") && out.contains("memory_pages: 1") && out.contains("roles: 5") && out.contains("private"), "{out}");
    assert!(export(&a, &args(&[dest.to_str().unwrap()])).await.unwrap_err().contains("not empty"), "an existing folder is never overwritten");

    let (_b_dir, b) = core().await;
    b.call("POST", "/v1/learn/rules", Some(json!({"text": "Always add links after research."}))).await.unwrap(); // already there
    b.call("PUT", "/v1/learn/style", Some(json!({"text": "My own style."}))).await.unwrap();
    let shown = import(&b, &args(&[dest.to_str().unwrap()])).await.unwrap();
    assert!(shown.contains("2 rule(s), 1 of them new") && shown.contains("writer (you have one)") && shown.contains("Nothing was changed"), "{shown}");
    assert_eq!(b.call("GET", "/v1/learn/rules", None).await.unwrap()["rules"].as_array().unwrap().len(), 1);

    let done = import(&b, &args(&[dest.to_str().unwrap(), "--apply"])).await.unwrap();
    assert!(done.contains("rules_added: 1") && done.contains("memory_pages_added: 1") && done.contains("style_set: false"), "{done}");
    let rules = b.call("GET", "/v1/learn/rules", None).await.unwrap();
    let doctor = rules["rules"].as_array().unwrap().iter().find(|r| r["text"].as_str().unwrap().contains("Dr. Amal")).unwrap();
    assert_eq!(doctor["privacy"], "sensitive", "sensitive stays sensitive");
    assert_eq!(b.call("GET", "/v1/learn/style", None).await.unwrap()["style"]["text"], "My own style.", "your own style stays");
    let pages = b.call("GET", "/v1/memory/pages", None).await.unwrap();
    assert_eq!(pages["pages"].as_array().unwrap().len(), 1);
    // running it again adds nothing
    let again = import(&b, &args(&[dest.to_str().unwrap(), "--apply"])).await.unwrap();
    assert!(again.contains("rules_added: 0") && again.contains("memory_pages_added: 0"), "{again}");
    let replaced = import(&b, &args(&[dest.to_str().unwrap(), "--apply", "--replace-style", "--replace", "writer"])).await.unwrap();
    assert!(replaced.contains("style_set: true") && replaced.contains("role writer"), "{replaced}");
    assert!(import(&b, &args(&[a_dir.path().to_str().unwrap()])).await.unwrap_err().contains("not a LastHarness export"));
}
