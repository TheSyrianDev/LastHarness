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
    seen: std::sync::Arc<Mutex<Vec<ChatRequest>>>,
}
impl Script {
    fn new(r: Vec<Option<Response>>) -> Self {
        let mut r = r;
        r.reverse();
        Self { replies: Mutex::new(r), seen: Default::default() }
    }
}
impl Model for Script {
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
    let app = router(AppState::new(Script::new(replies), Settings { image_model: None, web: Default::default(), idle_after: Duration::ZERO, compact_at: Some(compact_at), store: store.clone(), spend, bypass: false, sudo_enabled: false, allow_rules: vec![] }), token.clone(), port);
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
    // strangers are turned away before any body is read: broken JSON still gets 401, not 400
    let bad = s.http.post(format!("{}/v1/sessions", s.base)).header("Content-Type", "application/json").body("{not json at all").send().await.unwrap();
    assert_eq!(bad.status(), 401);
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
    let m: Value = s.get("/v1/models").send().await.unwrap().json().await.unwrap();
    assert_eq!(m["models"], json!(["tiny", "big"]));
    assert_eq!(s.get("/v1/sessions/nope").send().await.unwrap().status(), 404);
    let d = tempfile::tempdir().unwrap();
    let id = s.session(d.path()).await;
    assert_eq!(s.post(&format!("/v1/sessions/{id}/answers/a1"), json!({"answer":"yes"})).send().await.unwrap().status(), 404);
    assert_eq!(s.post(&format!("/v1/sessions/{id}/answers/a1"), json!({"answer":"maybe"})).send().await.unwrap().status(), 400);
}

fn free_meter() -> lh_spend::SpendMeter {
    lh_spend::SpendMeter::new(std::sync::Arc::new(lh_store::Store::open_memory().unwrap()), lh_spend::Config::default(), "test", true)
}

#[tokio::test]
async fn a_spending_cap_stops_the_turn_before_the_model_is_called() {
    let cfg = lh_spend::Config {
        mode: lh_spend::Mode::Caps,
        daily_cap_usd: Some(0.01),
        prices: vec![lh_spend::Price { model: "m".into(), input: 100.0, output: 100.0, cached_input: None, image: None }],
        ..Default::default()
    };
    let store = std::sync::Arc::new(lh_store::Store::open_memory().unwrap());
    let meter = lh_spend::SpendMeter::new(store.clone(), cfg, "cloud", false);
    let s = start_with(vec![], meter, store, 0).await; // an empty script panics if the model is called
    let d = tempfile::tempdir().unwrap();
    let id = s.session(d.path()).await;
    let (mut resp, mut p, mut seen) = (s.post(&format!("/v1/sessions/{id}/turns"), json!({"text":"hi"})).send().await.unwrap(), SseParser::new(), vec![]);
    read_until(&mut resp, &mut p, &mut seen, |v| v["type"] == "refused").await;
    assert!(seen.last().unwrap()["message"].as_str().unwrap().contains("spending cap"));
    let h: Value = s.get(&format!("/v1/sessions/{id}")).send().await.unwrap().json().await.unwrap();
    assert_eq!(h["busy"], false);
    assert_eq!(h["messages"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn the_activity_log_records_tool_calls_and_a_write_can_be_undone() {
    let store = std::sync::Arc::new(lh_store::Store::open_memory().unwrap());
    let s = start_with(vec![call("c1", "write_file", json!({"path":"n.txt","content":"v2"})), text("ok")], free_meter(), store, 0).await;
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("n.txt"), "v1").unwrap();
    let id = s.session(d.path()).await;
    let (mut resp, mut p, mut seen) = (s.post(&format!("/v1/sessions/{id}/turns"), json!({"text":"edit"})).send().await.unwrap(), SseParser::new(), vec![]);
    read_until(&mut resp, &mut p, &mut seen, |v| v["type"] == "ask").await;
    let ask = seen.last().unwrap()["id"].as_str().unwrap().to_string();
    s.post(&format!("/v1/sessions/{id}/answers/{ask}"), json!({"answer":"yes"})).send().await.unwrap();
    read_until(&mut resp, &mut p, &mut seen, |v| v["type"] == "done").await;
    assert_eq!(std::fs::read_to_string(d.path().join("n.txt")).unwrap(), "v2");

    let log: Value = s.get("/v1/activity").send().await.unwrap().json().await.unwrap();
    let kinds: Vec<_> = log["entries"].as_array().unwrap().iter().map(|e| e["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["tool_call", "permission"], "newest first");
    let call_entry = &log["entries"][0];
    assert_eq!(call_entry["detail"]["tool"], "write_file");
    assert_eq!(log["entries"][1]["detail"]["outcome"], "allowed_by_user");

    let seq = call_entry["seq"].as_i64().unwrap();
    let r: Value = s.post(&format!("/v1/activity/{seq}/undo"), json!({})).send().await.unwrap().json().await.unwrap();
    assert_eq!(r["result"], "restored");
    assert_eq!(std::fs::read_to_string(d.path().join("n.txt")).unwrap(), "v1");
    let again: Value = s.post(&format!("/v1/activity/{seq}/undo"), json!({})).send().await.unwrap().json().await.unwrap();
    assert_eq!(again["result"], "refused");
}

#[tokio::test]
async fn a_long_conversation_is_summarized_and_the_client_is_told() {
    let long = "x".repeat(600);
    let s = start_with(vec![text(&long), text("SUMMARY OF TURN ONE"), text("ok")], free_meter(), std::sync::Arc::new(lh_store::Store::open_memory().unwrap()), 50).await;
    let d = tempfile::tempdir().unwrap();
    let id = s.session(d.path()).await;
    let url = format!("/v1/sessions/{id}/turns");
    let (mut r1, mut p1, mut seen1) = (s.post(&url, json!({"text":"first"})).send().await.unwrap(), SseParser::new(), vec![]);
    read_until(&mut r1, &mut p1, &mut seen1, |v| v["type"] == "done").await;
    assert!(!kinds(&seen1).contains(&"compacted"), "one turn is never compacted");
    let (mut r2, mut p2, mut seen2) = (s.post(&url, json!({"text":"second"})).send().await.unwrap(), SseParser::new(), vec![]);
    read_until(&mut r2, &mut p2, &mut seen2, |v| v["type"] == "done").await;
    assert_eq!(kinds(&seen2)[0], "compacted");
    assert_eq!(seen2[0]["messages_summarized"], 2);
    let h: Value = s.get(&format!("/v1/sessions/{id}")).send().await.unwrap().json().await.unwrap();
    let msgs = h["messages"].as_array().unwrap();
    assert_eq!(msgs.len(), 3, "summary, the new question, the answer");
    assert!(msgs[0]["content"].as_str().unwrap().contains("SUMMARY OF TURN ONE"));
    assert_eq!(msgs[1]["content"], "second");
}

#[tokio::test]
async fn sampling_and_limits_from_the_session_reach_the_model() {
    let script = Script::new(vec![text("ok")]);
    let seen = script.seen.clone();
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    let token = auth::new_token();
    let settings = Settings { image_model: None, web: Default::default(), idle_after: Duration::ZERO, compact_at: Some(0), store: std::sync::Arc::new(lh_store::Store::open_memory().unwrap()), spend: free_meter(), bypass: false, sudo_enabled: false, allow_rules: vec![] };
    let app = router(AppState::new(script, settings), token.clone(), port);
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    let s = Server { base: format!("http://127.0.0.1:{port}"), token, http: reqwest::Client::new() };
    let d = tempfile::tempdir().unwrap();
    let body = json!({"model":"m","project_dir":d.path(),"max_tokens":321,"system":"Be brief.",
        "sampling":{"temperature":0.3,"top_k":20,"extra":{"chat_template_kwargs":{"enable_thinking":false}}}});
    let id = s.post("/v1/sessions", body).send().await.unwrap().json::<Value>().await.unwrap()["id"].as_str().unwrap().to_string();
    let (mut resp, mut p, mut got) = (s.post(&format!("/v1/sessions/{id}/turns"), json!({"text":"hi"})).send().await.unwrap(), SseParser::new(), vec![]);
    read_until(&mut resp, &mut p, &mut got, |v| v["type"] == "done").await;
    let r = seen.lock().unwrap()[0].clone();
    assert_eq!(r.max_tokens, Some(321));
    assert_eq!((r.sampling.temperature, r.sampling.top_k), (Some(0.3), Some(20)));
    assert_eq!(r.sampling.extra["chat_template_kwargs"]["enable_thinking"], false);
    assert_eq!(r.messages[0], Message::system("Be brief."));
}

#[tokio::test]
async fn folderless_sessions_chat_without_tools_or_work_from_home() {
    let script = Script::new(vec![text("hello"), text("hello again")]);
    let seen = script.seen.clone();
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    let token = auth::new_token();
    let settings = Settings { image_model: None, web: Default::default(), idle_after: Duration::ZERO, compact_at: Some(0), store: std::sync::Arc::new(lh_store::Store::open_memory().unwrap()), spend: free_meter(), bypass: false, sudo_enabled: false, allow_rules: vec![] };
    let app = router(AppState::new(script, settings), token.clone(), port);
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    let s = Server { base: format!("http://127.0.0.1:{port}"), token, http: reqwest::Client::new() };

    let chat = s.post("/v1/sessions", json!({"model":"m"})).send().await.unwrap().json::<Value>().await.unwrap()["id"].as_str().unwrap().to_string();
    let assistant = s.post("/v1/sessions", json!({"model":"m","mode":"assistant"})).send().await.unwrap().json::<Value>().await.unwrap()["id"].as_str().unwrap().to_string();
    for id in [&chat, &assistant] {
        let (mut resp, mut p, mut got) = (s.post(&format!("/v1/sessions/{id}/turns"), json!({"text":"hi"})).send().await.unwrap(), SseParser::new(), vec![]);
        read_until(&mut resp, &mut p, &mut got, |v| v["type"] == "done").await;
    }
    let reqs = seen.lock().unwrap().clone();
    assert!(reqs[0].tools.is_empty(), "plain chat tells the model about no tools");
    assert!(reqs[1].tools.iter().any(|t| t.name == "read_file"), "the assistant has the normal tools");
    let bad = s.post("/v1/sessions", json!({"model":"m","mode":"wizard"})).send().await.unwrap();
    assert_eq!(bad.status(), 400);
}
