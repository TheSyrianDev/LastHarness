//! Starts the real server on a random port and talks to it over HTTP, like a front end would.
use std::sync::Mutex;
use std::time::Duration;

use lh_core::{auth, router, AppState, Settings};
use lh_provider::*;
use lh_tools::Model;
use serde_json::{json, Value};

/// A model that replays prepared replies, or hangs forever when the script says so.
struct Script {
    replies: Mutex<Vec<Option<Response>>>,
}
impl Script {
    fn new(r: Vec<Option<Response>>) -> Self {
        let mut r = r;
        r.reverse();
        Self { replies: Mutex::new(r) }
    }
}
impl Model for Script {
    async fn complete(&self, _r: &ChatRequest, on: &mut (dyn FnMut(Event) + Send)) -> Result<Response, Error> {
        let next = self.replies.lock().unwrap().pop().expect("model asked more often than scripted");
        let Some(r) = next else { return std::future::pending().await };
        if !r.text.is_empty() {
            on(Event::Text(r.text.clone()));
        }
        on(Event::Finish(r.finish.clone().unwrap()));
        Ok(r)
    }
}

fn text(t: &str) -> Option<Response> {
    Some(Response { text: t.into(), finish: Some(FinishReason::Stop), ..Default::default() })
}
fn call(id: &str, name: &str, args: Value) -> Option<Response> {
    Some(Response {
        tool_calls: vec![ToolCall { id: id.into(), name: name.into(), arguments: args.to_string(), repaired: false }],
        finish: Some(FinishReason::ToolCalls),
        ..Default::default()
    })
}

struct Server {
    base: String,
    token: String,
    http: reqwest::Client,
}

async fn start(replies: Vec<Option<Response>>) -> Server {
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    let token = auth::new_token();
    let app = router(AppState::new(Script::new(replies), Settings { bypass: false, sudo_enabled: false, allow_rules: vec![] }), token.clone(), port);
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    Server { base: format!("http://127.0.0.1:{port}"), token, http: reqwest::Client::new() }
}

impl Server {
    fn post(&self, path: &str, body: Value) -> reqwest::RequestBuilder {
        self.http.post(format!("{}{path}", self.base)).bearer_auth(&self.token).json(&body)
    }
    fn get(&self, path: &str) -> reqwest::RequestBuilder {
        self.http.get(format!("{}{path}", self.base)).bearer_auth(&self.token)
    }
    async fn session(&self, dir: &std::path::Path) -> String {
        let v: Value = self.post("/v1/sessions", json!({"model":"m","project_dir":dir})).send().await.unwrap().json().await.unwrap();
        v["id"].as_str().unwrap().to_string()
    }
}

/// Reads SSE events from a response until `stop` returns true for one of them.
async fn read_until(resp: &mut reqwest::Response, p: &mut SseParser, seen: &mut Vec<Value>, stop: impl Fn(&Value) -> bool) {
    while let Some(chunk) = tokio::time::timeout(Duration::from_secs(10), resp.chunk()).await.expect("no event for 10 s").unwrap() {
        for d in p.push(&chunk) {
            let v: Value = serde_json::from_str(&d).unwrap();
            let done = stop(&v);
            seen.push(v);
            if done {
                return;
            }
        }
    }
}
fn kinds(seen: &[Value]) -> Vec<&str> {
    seen.iter().map(|v| v["type"].as_str().unwrap()).collect()
}

#[tokio::test]
async fn auth_is_enforced() {
    let s = start(vec![]).await;
    let url = format!("{}/v1/health", s.base);
    assert_eq!(s.http.get(&url).send().await.unwrap().status(), 401);
    assert_eq!(s.http.get(&url).bearer_auth("wrong").send().await.unwrap().status(), 401);
    assert_eq!(s.http.get(&url).bearer_auth(&s.token).send().await.unwrap().status(), 200);
    // DNS rebinding: right token is useless if the page reached us under another name
    assert_eq!(s.http.get(&url).bearer_auth(&s.token).header("Host", "evil.example:80").send().await.unwrap().status(), 403);
    // a browser page on another site sends Origin
    assert_eq!(s.http.get(&url).bearer_auth(&s.token).header("Origin", "http://evil.example").send().await.unwrap().status(), 403);
}

#[tokio::test]
async fn a_turn_streams_and_is_kept_in_the_history() {
    let s = start(vec![text("hello there")]).await;
    let d = tempfile::tempdir().unwrap();
    let id = s.session(d.path()).await;
    let (mut resp, mut p, mut seen) = (s.post(&format!("/v1/sessions/{id}/turns"), json!({"text":"hi"})).send().await.unwrap(), SseParser::new(), vec![]);
    assert_eq!(resp.headers()["content-type"], "text/event-stream");
    read_until(&mut resp, &mut p, &mut seen, |v| v["type"] == "done").await;
    assert_eq!(kinds(&seen), ["text", "finish", "done"]);
    assert_eq!(seen[0]["text"], "hello there");
    let h: Value = s.get(&format!("/v1/sessions/{id}")).send().await.unwrap().json().await.unwrap();
    let roles: Vec<_> = h["messages"].as_array().unwrap().iter().map(|m| (m["role"].as_str().unwrap(), m["content"].as_str().unwrap())).collect();
    assert_eq!(roles, [("user", "hi"), ("assistant", "hello there")]);
}

#[tokio::test]
async fn a_write_asks_over_the_api_and_runs_after_yes() {
    let s = start(vec![call("c1", "write_file", json!({"path":"note.txt","content":"saved"})), text("done")]).await;
    let d = tempfile::tempdir().unwrap();
    let id = s.session(d.path()).await;
    let (mut resp, mut p, mut seen) = (s.post(&format!("/v1/sessions/{id}/turns"), json!({"text":"write it"})).send().await.unwrap(), SseParser::new(), vec![]);
    read_until(&mut resp, &mut p, &mut seen, |v| v["type"] == "ask").await;
    let ask = seen.last().unwrap().clone();
    assert_eq!(ask["tool"], "write_file");
    assert!(!d.path().join("note.txt").exists(), "nothing happens before the answer");
    let r = s.post(&format!("/v1/sessions/{id}/answers/{}", ask["id"].as_str().unwrap()), json!({"answer":"yes"})).send().await.unwrap();
    assert_eq!(r.status(), 204);
    read_until(&mut resp, &mut p, &mut seen, |v| v["type"] == "done").await;
    assert_eq!(std::fs::read_to_string(d.path().join("note.txt")).unwrap(), "saved");
    assert!(kinds(&seen).contains(&"tool_result"));
}

#[tokio::test]
async fn a_second_turn_is_refused_and_a_dropped_client_stops_the_first() {
    let s = start(vec![None, text("second")]).await; // first model call hangs
    let d = tempfile::tempdir().unwrap();
    let id = s.session(d.path()).await;
    let url = format!("/v1/sessions/{id}/turns");
    let first = s.post(&url, json!({"text":"one"})).send().await.unwrap();
    assert_eq!(s.post(&url, json!({"text":"two"})).send().await.unwrap().status(), 409);
    drop(first); // client goes away
    let mut busy = true;
    for _ in 0..50 {
        let h: Value = s.get(&format!("/v1/sessions/{id}")).send().await.unwrap().json().await.unwrap();
        busy = h["busy"].as_bool().unwrap();
        if !busy {
            assert_eq!(h["messages"].as_array().unwrap().len(), 0, "a cancelled turn leaves no half-finished history");
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(!busy, "the session is free again after the client dropped");
    let (mut resp, mut p, mut seen) = (s.post(&url, json!({"text":"again"})).send().await.unwrap(), SseParser::new(), vec![]);
    read_until(&mut resp, &mut p, &mut seen, |v| v["type"] == "done").await;
}

#[tokio::test]
async fn bad_requests_get_clear_errors() {
    let s = start(vec![]).await;
    let r = s.post("/v1/sessions", json!({"model":"m","project_dir":"/definitely/not/here"})).send().await.unwrap();
    assert_eq!(r.status(), 400);
    assert_eq!(s.get("/v1/sessions/nope").send().await.unwrap().status(), 404);
    let d = tempfile::tempdir().unwrap();
    let id = s.session(d.path()).await;
    assert_eq!(s.post(&format!("/v1/sessions/{id}/answers/a1"), json!({"answer":"yes"})).send().await.unwrap().status(), 404);
    assert_eq!(s.post(&format!("/v1/sessions/{id}/answers/a1"), json!({"answer":"maybe"})).send().await.unwrap().status(), 400);
}
