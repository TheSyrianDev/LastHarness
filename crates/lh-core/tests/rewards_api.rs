//! Reward records: a test command that passes leaves a "tests" reward, readable at /v1/rewards.
use std::sync::Arc;
use std::time::Duration;

use lh_core::{auth, router, AppState, Settings};
use lh_provider::{ChatRequest, Event, FinishReason, Response, SseParser, ToolCall};
use lh_store::Store;
use lh_tools::Model;
use serde_json::{json, Value};

/// Runs a command that looks like a test run (and passes), then answers.
struct Fake;
impl Model for Fake {
    async fn complete(&self, r: &ChatRequest, on: &mut (dyn FnMut(Event) + Send)) -> Result<Response, lh_provider::Error> {
        if !r.messages.iter().any(|m| m.role == lh_provider::Role::Tool) {
            let call = ToolCall { id: "c1".into(), name: "run_command".into(), arguments: json!({"command": "make test"}).to_string(), repaired: false };
            return Ok(Response { tool_calls: vec![call], finish: Some(FinishReason::ToolCalls), ..Default::default() });
        }
        on(Event::Text("ok".into()));
        Ok(Response { text: "ok".into(), finish: Some(FinishReason::Stop), ..Default::default() })
    }
}

fn settings(store: Arc<Store>) -> Settings {
    Settings {
        provider_name: None,
        learn: None,
        maintain_when_idle: false,
        memory: None,
        image_model: None,
        web: Default::default(),
        idle_after: Duration::ZERO,
        compact_at: None,
        spend: lh_spend::SpendMeter::new(store.clone(), lh_spend::Config::default(), "test", true),
        store,
        bypass: true,
        sudo_enabled: false,
        allow_rules: vec![],
        team: None,
        mcp: None,
        ask_all: false,
        config: None,
    }
}

#[tokio::test]
async fn a_passing_test_run_is_kept_as_a_reward() {
    let dir = tempfile::tempdir().unwrap();
    // a project whose tests pass: `make test` exits 0
    std::fs::write(dir.path().join("Makefile"), "test:\n\t@true\n").unwrap();
    let store = Arc::new(Store::open(&dir.path().join("lh.db")).unwrap());
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    l.set_nonblocking(true).unwrap();
    let port = l.local_addr().unwrap().port();
    let token = auth::new_token();
    let app = router(AppState::new(Fake, settings(store.clone())), token.clone(), port);
    tokio::spawn(async move { axum::serve(tokio::net::TcpListener::from_std(l).unwrap(), app).await.unwrap() });
    let base = format!("http://127.0.0.1:{port}");
    let http = reqwest::Client::new();
    let v: Value = http.post(format!("{base}/v1/sessions")).bearer_auth(&token).json(&json!({"model": "m", "project_dir": dir.path()})).send().await.unwrap().json().await.unwrap();
    let id = v["id"].as_str().unwrap();
    let mut resp = http.post(format!("{base}/v1/sessions/{id}/turns")).bearer_auth(&token).json(&json!({"text": "run the tests"})).send().await.unwrap();
    let mut p = SseParser::new();
    'outer: while let Some(chunk) = tokio::time::timeout(Duration::from_secs(60), resp.chunk()).await.unwrap().unwrap() {
        for d in p.push(&chunk) {
            if serde_json::from_str::<Value>(&d).unwrap()["type"] == "done" {
                break 'outer;
            }
        }
    }
    let r: Value = http.get(format!("{base}/v1/rewards")).bearer_auth(&token).send().await.unwrap().json().await.unwrap();
    assert_eq!(r["rewards"][0]["signal"], "tests", "{r}");
    assert_eq!(r["rewards"][0]["value"], 1.0);
    assert_eq!(r["totals"]["tests"]["count"], 1);
}
