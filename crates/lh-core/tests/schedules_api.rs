//! Scheduled tasks: added with a plain time, listed, turned off and on, run unattended, removed.
use std::sync::Arc;
use std::time::Duration;

use lh_core::{auth, router, AppState, Settings};
use lh_provider::{ChatRequest, Event, FinishReason, Response, ToolCall};
use lh_store::Store;
use lh_tools::Model;
use serde_json::{json, Value};

/// Asks to write a file on the first call of a turn, then answers "done".
struct Fake;
impl Model for Fake {
    async fn complete(&self, r: &ChatRequest, on: &mut (dyn FnMut(Event) + Send)) -> Result<Response, lh_provider::Error> {
        let wrote = r.messages.iter().any(|m| m.role == lh_provider::Role::Tool);
        if !wrote && !r.tools.is_empty() {
            let call = ToolCall { id: "c1".into(), name: "write_file".into(), arguments: json!({"path": "out.txt", "content": "hi"}).to_string(), repaired: false };
            return Ok(Response { tool_calls: vec![call], finish: Some(FinishReason::ToolCalls), ..Default::default() });
        }
        on(Event::Text("done".into()));
        Ok(Response { text: "done".into(), finish: Some(FinishReason::Stop), ..Default::default() })
    }
}

fn settings(store: Arc<Store>, bypass: bool) -> Settings {
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
        bypass,
        sudo_enabled: false,
        allow_rules: vec![],
        team: None,
        mcp: None,
        ask_all: false,
        config: None,
    }
}

#[tokio::test]
async fn a_task_is_added_run_unattended_and_removed() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("p");
    std::fs::create_dir(&project).unwrap();
    let store = Arc::new(Store::open(&dir.path().join("lh.db")).unwrap());
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    l.set_nonblocking(true).unwrap();
    let port = l.local_addr().unwrap().port();
    let token = auth::new_token();
    let app = router(AppState::new(Fake, settings(store.clone(), false)), token.clone(), port);
    tokio::spawn(async move { axum::serve(tokio::net::TcpListener::from_std(l).unwrap(), app).await.unwrap() });
    let base = format!("http://127.0.0.1:{port}");
    let http = reqwest::Client::new();
    let call = |m: reqwest::Method, path: &str, body: Option<Value>| {
        let mut rb = http.request(m, format!("{base}{path}")).bearer_auth(&token);
        if let Some(b) = body {
            rb = rb.json(&b);
        }
        rb.send()
    };

    let r = call(reqwest::Method::POST, "/v1/schedules", Some(json!({"name": "", "prompt": "x", "folder": project, "model": "m", "when": "now and then"}))).await.unwrap();
    assert_eq!(r.status(), 400, "a time it cannot read is refused");
    let v: Value = call(reqwest::Method::POST, "/v1/schedules", Some(json!({"name": "notes", "prompt": "write out.txt", "folder": project, "model": "m", "when": "every 1h"}))).await.unwrap().json().await.unwrap();
    let id = v["id"].as_i64().unwrap();
    assert_eq!((v["when"].as_str(), v["enabled"].as_bool()), (Some("every 1h"), Some(true)));

    assert_eq!(lh_core::schedules_api::run_due(&store, &base, &token).await, 0, "not due yet");
    store.with(|c| c.execute("update schedules set next_run = 0", [])).unwrap();
    assert_eq!(lh_core::schedules_api::run_due(&store, &base, &token).await, 1);
    let list: Value = call(reqwest::Method::GET, "/v1/schedules", None).await.unwrap().json().await.unwrap();
    // the write asks once per chat, and nobody is there to say yes
    assert_eq!(list["schedules"][0]["last_result"], "ok, but said no to write");
    assert!(!project.join("out.txt").exists());
    assert!(store.recent(10, Some("schedule_changed_files")).unwrap().is_empty(), "nothing changed, so no notice");

    let v: Value = call(reqwest::Method::POST, &format!("/v1/schedules/{id}/enabled"), Some(json!({"enabled": false}))).await.unwrap().json().await.unwrap();
    assert_eq!(v["enabled"], false);
    assert_eq!(call(reqwest::Method::DELETE, &format!("/v1/schedules/{id}"), None).await.unwrap().status(), 204);
    assert_eq!(call(reqwest::Method::DELETE, &format!("/v1/schedules/{id}"), None).await.unwrap().status(), 404);
}

#[tokio::test]
async fn a_task_that_changed_files_leaves_a_notice() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("p");
    std::fs::create_dir(&project).unwrap();
    let store = Arc::new(Store::open(&dir.path().join("lh.db")).unwrap());
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    l.set_nonblocking(true).unwrap();
    let port = l.local_addr().unwrap().port();
    let token = auth::new_token();
    // the user switched permission questions off, so the write goes through
    let app = router(AppState::new(Fake, settings(store.clone(), true)), token.clone(), port);
    tokio::spawn(async move { axum::serve(tokio::net::TcpListener::from_std(l).unwrap(), app).await.unwrap() });
    let base = format!("http://127.0.0.1:{port}");
    let r = reqwest::Client::new().post(format!("{base}/v1/schedules")).bearer_auth(&token).json(&json!({"name": "notes", "prompt": "write out.txt", "folder": project, "model": "m", "when": "every 1h"})).send().await.unwrap();
    assert!(r.status().is_success());
    store.with(|c| c.execute("update schedules set next_run = 0", [])).unwrap();
    assert_eq!(lh_core::schedules_api::run_due(&store, &base, &token).await, 1);
    assert_eq!(std::fs::read_to_string(project.join("out.txt")).unwrap(), "hi");
    let notes = store.recent(10, Some("schedule_changed_files")).unwrap();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].detail["name"], "notes");
    let files = notes[0].detail["files"].as_array().unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].as_str().unwrap().ends_with("out.txt"));
}
