//! `lh channels ...` against a real core.
use std::sync::Arc;
use std::time::Duration;

use lh_core::{auth, router, AppState, Settings};
use lh_provider::{ChatRequest, Event, Response};
use lh_store::Store;
use lh_tools::Model;
use lh_tui::channels_cli::run;
use lh_tui::client::Client;

struct Nothing;
impl Model for Nothing {
    async fn complete(&self, _: &ChatRequest, _: &mut (dyn FnMut(Event) + Send)) -> Result<Response, lh_provider::Error> {
        Ok(Response::default())
    }
}

async fn core() -> Client {
    let store = Arc::new(Store::open_memory().unwrap());
    let settings = Settings {
        config: None,
        ask_all: false,
        mcp: None,
        team: None,
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
    Client::new(format!("http://127.0.0.1:{port}"), token)
}
fn args(s: &str) -> Vec<String> {
    s.split_whitespace().map(str::to_string).collect()
}

#[tokio::test]
async fn a_channel_is_added_off_and_switched_on_only_after_the_warning_is_confirmed() {
    let c = core().await;
    std::env::set_var("LH_CLI_TG", "x");
    let mut no = |_: &str| false;
    assert!(run(&c, &[], &mut no).await.unwrap().contains("No channels"));
    assert!(run(&c, &args("kinds"), &mut no).await.unwrap().contains("slack") );
    let out = run(&c, &args("add phone --kind telegram --allow 42,43 --model m --set token=env:LH_CLI_TG"), &mut no).await.unwrap();
    assert!(out.contains("It is OFF") && out.contains("cannot protect them"), "{out}");
    let list = run(&c, &[], &mut no).await.unwrap();
    assert!(list.contains("phone") && list.contains(" off ") && list.contains("allowed: 42,43"), "{list}");

    // answering no leaves it off; the question carried the warning
    let mut seen = String::new();
    let mut ask = |q: &str| { seen = q.to_string(); false };
    assert_eq!(run(&c, &args("enable phone"), &mut ask).await.unwrap(), "Left off.");
    assert!(seen.contains("Telegram bot chats are not end-to-end encrypted"), "{seen}");
    assert!(run(&c, &[], &mut no).await.unwrap().contains(" off "));
    // yes switches it on (the runner is not running in this test, so it is "on, not running")
    let mut yes = |_: &str| true;
    assert!(run(&c, &args("enable phone"), &mut yes).await.unwrap().starts_with("Switched on."));
    assert!(run(&c, &[], &mut no).await.unwrap().contains("on, not running"));
    assert!(run(&c, &args("edit phone --allow 42 --mode assistant --set api_base=http://x"), &mut no).await.unwrap().contains("allowed: 42"));
    assert!(run(&c, &args("disable phone"), &mut no).await.unwrap().starts_with("Switched off."));
    assert_eq!(run(&c, &args("remove phone"), &mut no).await.unwrap(), "Removed.");
    assert!(run(&c, &args("add x"), &mut no).await.is_err());
    assert!(run(&c, &args("enable ghost --yes"), &mut no).await.is_err());
}
