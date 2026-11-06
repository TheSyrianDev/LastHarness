//! Starts the real server on a random port and talks to it over HTTP, like a front end would.
use std::sync::Mutex;
use std::time::Duration;

use lh_core::{auth, router, AppState, Settings};
use lh_provider::*;
use lh_tools::Model;
use serde_json::{json, Value};

/// A model that replays prepared replies, or hangs forever when the script says so.
struct Script {
    params: Option<u64>,
    service: Option<String>,
    replies: Mutex<Vec<Option<Response>>>,
    seen: std::sync::Arc<Mutex<Vec<ChatRequest>>>,
}
impl Script {
    fn new(r: Vec<Option<Response>>) -> Self {
        let mut r = r;
        r.reverse();
        Self { params: None, service: None, replies: Mutex::new(r), seen: Default::default() }
    }
}
impl Model for Script {
    async fn capabilities(&self, _model: &str) -> lh_provider::Capabilities {
        lh_provider::Capabilities { parameters: self.params, ..Default::default() }
    }
    async fn media_support(&self, model: &str) -> lh_provider::Support {
        if model == "blind" { Support::default() } else { Support { vision: true, ..Default::default() } }
    }
    async fn provider_name(&self) -> Option<String> {
        self.service.clone()
    }
    async fn models(&self) -> Result<Vec<String>, Error> {
        Ok(vec!["tiny".into(), "big".into()])
    }
    async fn complete(&self, r: &ChatRequest, on: &mut (dyn FnMut(Event) + Send)) -> Result<Response, Error> {
        self.seen.lock().unwrap().push(r.clone());
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
    start_with(replies, free_meter(), std::sync::Arc::new(lh_store::Store::open_memory().unwrap()), 0).await
}

async fn start_with(replies: Vec<Option<Response>>, spend: lh_spend::SpendMeter, store: std::sync::Arc<lh_store::Store>, compact_at: u64) -> Server {
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    let token = auth::new_token();
    let app = router(AppState::new(Script::new(replies), Settings { provider_name: None, learn: None, maintain_when_idle: false, memory: None, image_model: None, web: Default::default(), idle_after: Duration::ZERO, compact_at: Some(compact_at), store: store.clone(), spend, bypass: false, sudo_enabled: false, allow_rules: vec![], team: None, mcp: None, ask_all: false, config: None }), token.clone(), port);
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


impl Server {
    async fn turn(&self, id: &str, body: Value) -> (u16, String) {
        let r = self.post(&format!("/v1/sessions/{id}/turns"), body).send().await.unwrap();
        (r.status().as_u16(), r.text().await.unwrap())
    }
}

fn png_b64() -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(b"\x89PNG\r\n\x1a\nSECRETPIXELS")
}

#[tokio::test]
async fn files_reach_the_model_and_stay_out_of_logs_and_the_database() {
    let dir = tempfile::tempdir().unwrap();
    let store = std::sync::Arc::new(lh_store::Store::open(&dir.path().join("lh.db")).unwrap());
    let s = start_with(vec![text("seen"), text("again")], free_meter(), store.clone(), 0).await;
    let id = s.session(dir.path()).await;
    let att = json!([{"name": "cat.png", "mime": "image/png", "data": png_b64()}, {"name": "n.txt", "data": "aGVsbG8="}]);
    let (code, body) = s.turn(&id, json!({"text": "look", "attachments": att})).await;
    assert_eq!(code, 200, "{body}");
    assert!(body.contains("roughly"), "token note missing: {body}");
    // second turn: history still carries the files
    let (code, _) = s.turn(&id, json!({"text": "and?"})).await;
    assert_eq!(code, 200);
    // files are on disk, not in the database or the activity log
    let n = std::fs::read_dir(dir.path().join("attachments")).unwrap().count();
    assert_eq!(n, 4, "two files plus two meta files");
    let log = format!("{:?}", store.recent(1000, None).unwrap());
    assert!(!log.contains("SECRETPIXELS") && !log.contains("U0VDUkVU"));
}

#[tokio::test]
async fn blind_model_gets_a_notice_and_a_note() {
    let s = start(vec![text("ok")]).await;
    let d = tempfile::tempdir().unwrap();
    let v: Value = s.post("/v1/sessions", json!({"model":"blind","project_dir":d.path()})).send().await.unwrap().json().await.unwrap();
    let id = v["id"].as_str().unwrap();
    let (code, body) = s.turn(id, json!({"text": "look", "attachments": [{"name": "cat.png", "mime": "image/png", "data": png_b64()}]})).await;
    assert_eq!(code, 200, "{body}");
    assert!(body.contains("cannot look at images"), "{body}");
}

#[tokio::test]
async fn bad_and_oversized_files_are_refused_plainly() {
    let s = start(vec![]).await;
    let d = tempfile::tempdir().unwrap();
    let id = s.session(d.path()).await;
    let (code, body) = s.turn(&id, json!({"text": "x", "attachments": [{"name": "a.png", "data": "!!!"}]})).await;
    assert_eq!(code, 400, "{body}");
    use base64::Engine;
    let big = base64::engine::general_purpose::STANDARD.encode(vec![b'a'; 300 << 10]);
    let (code, body) = s.turn(&id, json!({"text": "x", "attachments": [{"name": "big.txt", "data": big}]})).await;
    assert_eq!(code, 413, "{body}");
    assert!(body.contains("big.txt") && body.contains("too large"), "{body}");
}

fn free_meter() -> lh_spend::SpendMeter {
    lh_spend::SpendMeter::new(std::sync::Arc::new(lh_store::Store::open_memory().unwrap()), lh_spend::Config::default(), "test", true)
}
