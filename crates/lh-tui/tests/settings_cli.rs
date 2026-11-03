//! `lh settings` against a real core, and with no core at all.
use std::sync::Arc;
use std::time::Duration;

use lh_core::{auth, router, AppState, Settings, TeamRuntime};
use lh_provider::{ChatRequest, Event, Response};
use lh_store::Store;
use lh_tools::Model;
use lh_tui::client::Client;
use lh_tui::settings_cli::run;

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
fn args(s: &str) -> Vec<String> {
    s.split_whitespace().map(str::to_string).collect()
}

#[tokio::test]
async fn settings_are_listed_checked_changed_and_kept_in_step_with_the_departments() {
    let (d, c, cfg) = core().await;
    let all = run(&c, &[], &cfg).await.unwrap();
    for k in ["provider.url", "learning.mode", "team.autonomy", "spending.daily_cap_usd", "reports.crash"] {
        assert!(all.contains(k), "{k} missing:\n{all}");
    }
    assert!(all.contains("[memory]") && all.contains("(default)"), "{all}");
    let only = run(&c, &args("spending"), &cfg).await.unwrap();
    assert!(only.contains("spending.mode") && !only.contains("team.autonomy"));

    // a live setting applies at once, in the store the departments read and in the file
    let out = run(&c, &args("set team.parallel 4"), &cfg).await.unwrap();
    assert_eq!(out, "Changed: team.parallel.");
    assert_eq!(c.call("GET", "/v1/team/settings", None).await.unwrap()["parallel"], 4);
    assert!(std::fs::read_to_string(&cfg).unwrap().contains("parallel = 4"));
    // the other way round: changing through the team endpoint updates the file
    c.call("PUT", "/v1/team/settings", Some(serde_json::json!({"autonomy": "check-ins"}))).await.unwrap();
    assert!(run(&c, &args("team"), &cfg).await.unwrap().contains("check-ins"));
    assert!(std::fs::read_to_string(&cfg).unwrap().contains("autonomy = \"check-ins\""));

    // a startup setting says so
    let out = run(&c, &args("set provider.url http://localhost:9292/v1"), &cfg).await.unwrap();
    assert!(out.contains("Takes effect when the core next starts: provider.url"), "{out}");
    // several at once; one wrong value changes nothing
    assert!(run(&c, &args("set learning.mode=silent memory.enabled=off"), &cfg).await.is_ok());
    let err = run(&c, &args("set learning.mode=off team.parallel=99"), &cfg).await.unwrap_err();
    assert!(err.contains("from 1 to 16"), "{err}");
    assert!(run(&c, &args("learning"), &cfg).await.unwrap().contains("silent"), "the valid one in the failed batch was not applied");
    assert!(run(&c, &args("set nothing.here 1"), &cfg).await.unwrap_err().contains("no setting called"));
    // reset goes back to the default
    run(&c, &args("reset team.parallel"), &cfg).await.unwrap();
    assert_eq!(c.call("GET", "/v1/team/settings", None).await.unwrap()["parallel"], 1);
    // the crash-report preview says what would be sent and that nothing is
    let p = c.call("GET", "/v1/reports/preview", None).await.unwrap();
    assert_eq!(p["setting"], "off");
    assert!(p["note"].as_str().unwrap().contains("Nothing is sent"));
    let log = c.call("GET", "/v1/activity?kind=setting_changed", None).await.unwrap();
    assert!(log["entries"].as_array().unwrap().len() >= 5, "setting changes are in the activity log");
    drop(d);
}

#[tokio::test]
async fn with_no_core_running_the_file_is_read_and_changed_directly() {
    let d = tempfile::tempdir().unwrap();
    let cfg = d.path().join("config.toml");
    let nobody = Client::new("http://127.0.0.1:9", "x"); // nothing listens on port 9
    assert!(run(&nobody, &args("memory"), &cfg).await.unwrap().contains("memory.enabled"));
    let out = run(&nobody, &args("set core.idle_after 120"), &cfg).await.unwrap();
    assert!(out.contains("The core is not running") && out.contains("next start"), "{out}");
    assert!(std::fs::read_to_string(&cfg).unwrap().contains("idle_after = 120"));
    assert!(run(&nobody, &args("set core.idle_after soon"), &cfg).await.unwrap_err().contains("whole number"));
}
