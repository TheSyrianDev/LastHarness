//! `lh settings` against a real core, and with no core at all.
use std::sync::Arc;
use std::time::Duration;

use lh_core::{auth, router, AppState, Settings, TeamRuntime};
use lh_provider::{ChatRequest, Event, Response};
use lh_store::Store;
use lh_tools::Model;
use lh_tui::client::Client;
use lh_tui::providers_cli::{run, Keys};

struct Nothing;
impl Model for Nothing {
    async fn complete(&self, _: &ChatRequest, _: &mut (dyn FnMut(Event) + Send)) -> Result<Response, lh_provider::Error> {
        Ok(Response::default())
    }
}

async fn core() -> (tempfile::TempDir, Client, std::path::PathBuf) {
    let d = tempfile::tempdir().unwrap();
    let store = Arc::new(Store::open(&d.path().join("lh.db")).unwrap());
    let team = TeamRuntime::open(&d.path().join("team"), Arc::new(lh_learn::Learn::new(store.clone())), 1000).unwrap();
    let cfg = d.path().join("config.toml");
    let settings = Settings {
        config: Some(cfg.clone()),
        ask_all: false,
        mcp: None,
        team: Some(team),
        provider_name: None,
        learn: None,
        maintain_when_idle: false,
        memory: None,
        image_model: None,
        web: Default::default(),
        idle_after: Duration::ZERO,
        compact_at: Some(0),
        store: store.clone(),
        spend: lh_spend::SpendMeter::new(store, lh_spend::Config::default(), "t", true),
        bypass: false,
        sudo_enabled: false,
        allow_rules: vec![],
    };
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    let token = auth::new_token();
    let app = router(AppState::new(Nothing, settings), token.clone(), port);
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    (d, Client::new(format!("http://127.0.0.1:{port}"), token), cfg)
}
struct Typed(Vec<String>, usize);
impl Keys for Typed {
    fn key(&mut self, _: &str) -> String {
        self.1 += 1;
        self.0.pop().unwrap_or_default()
    }
}
fn args(s: &str) -> Vec<String> {
    s.split_whitespace().map(str::to_string).collect()
}

#[tokio::test]
async fn providers_are_listed_added_defaulted_and_removed_and_the_key_is_never_printed() {
    let (d, c, _) = core().await;
    let mut k = Typed(vec!["sk-or-TOPSECRET".into()], 0);
    assert!(run(&c, &args("providers"), &mut k).await.unwrap().contains("No providers"));
    let out = run(&c, &args("/provider add openrouter"), &mut k).await.unwrap();
    assert_eq!(k.1, 1, "a preset that needs a key asks once");
    let out2 = run(&c, &args("provider add http://localhost:9292/v1 --name Box"), &mut Typed(vec![], 0)).await.unwrap();
    let list = run(&c, &args("/providers"), &mut k).await.unwrap();
    for t in [&out, &out2, &list] {
        assert!(!t.contains("TOPSECRET"), "{t}");
    }
    assert!(list.contains("openrouter") && list.contains("key saved") && list.contains("Box"), "{list}");
    assert!(list.lines().next().unwrap().starts_with("* "), "default marked: {list}");
    assert!(run(&c, &args("provider default box"), &mut k).await.unwrap().contains("default"));
    assert!(run(&c, &args("provider remove openrouter"), &mut k).await.is_ok());
    assert!(run(&c, &args("provider remove nope"), &mut k).await.is_err());
    assert!(run(&c, &args("providers presets"), &mut k).await.unwrap().contains("openrouter"));
    assert!(run(&c, &args("proposals"), &mut k).await.unwrap().contains("No proposals"));
    assert!(run(&c, &args("approve zzz"), &mut k).await.unwrap_err().contains("No proposal"));
    drop(d);
}
