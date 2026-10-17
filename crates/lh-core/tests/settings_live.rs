//! Settings that apply while the core runs: change one over the API and watch the next request behave
//! differently, with no restart. Also checks the schema says honestly which settings still need one.
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use lh_core::{auth, router, AppState, MemoryRuntime, Settings};
use lh_memory::embed::AnyEmbedder;
use lh_memory::{By, Layer, Memory, NewPage, Privacy, RecallConfig, Trust};
use lh_provider::{ChatRequest, Event, FinishReason, Response, SseParser, ToolCall};
use lh_store::Store;
use lh_tools::Model;
use serde_json::{json, Value};

/// Chat replies come from a queue (then "ok"); the model `titler` answers with a fixed title.
struct Fake {
    replies: Mutex<VecDeque<Response>>,
    seen: Arc<Mutex<Vec<ChatRequest>>>,
}
impl Model for Fake {
    async fn complete(&self, r: &ChatRequest, on: &mut (dyn FnMut(Event) + Send)) -> Result<Response, lh_provider::Error> {
        self.seen.lock().unwrap().push(r.clone());
        if r.model == "titler" {
            return Ok(Response { text: "Named By Model".into(), finish: Some(FinishReason::Stop), ..Default::default() });
        }
        let next = self.replies.lock().unwrap().pop_front().unwrap_or_else(|| text("ok"));
        if !next.text.is_empty() {
            on(Event::Text(next.text.clone()));
        }
        Ok(next)
    }
}
fn text(t: &str) -> Response {
    Response { text: t.into(), finish: Some(FinishReason::Stop), ..Default::default() }
}
fn write_call(id: &str, path: &str) -> Response {
    Response { tool_calls: vec![ToolCall { id: id.into(), name: "write_file".into(), arguments: json!({"path": path, "content": "x"}).to_string(), repaired: false }], finish: Some(FinishReason::ToolCalls), ..Default::default() }
}

struct Server {
    base: String,
    token: String,
    http: reqwest::Client,
    seen: Arc<Mutex<Vec<ChatRequest>>>,
    mem: Arc<Memory>,
    cfg: std::path::PathBuf,
}

/// `cloud`: the endpoint counts as a paid cloud service with a price for the model `m`, so spending rules bite.
fn start(dir: &std::path::Path, replies: Vec<Response>, cloud: bool) -> Server {
    let store = Arc::new(Store::open(&dir.join("lh.db")).unwrap());
    let mem = Arc::new(Memory::open(store.clone(), dir.join("memory")).unwrap());
    let seen: Arc<Mutex<Vec<ChatRequest>>> = Default::default();
    let model = Fake { replies: Mutex::new(replies.into()), seen: seen.clone() };
    let spend_cfg = lh_spend::Config { prices: vec![lh_spend::Price { model: "m".into(), input: 2.5, output: 10.0, cached_input: None, image: None }], ..Default::default() };
    let spend = lh_spend::SpendMeter::new(store.clone(), spend_cfg, "test", !cloud);
    let cfg = dir.join("config.toml");
    let settings = Settings {
        provider_name: None,
        learn: None,
        maintain_when_idle: false,
        memory: Some(MemoryRuntime { memory: mem.clone(), embedder: Arc::new(AnyEmbedder::Off), trust: Trust::Local, cfg: RecallConfig::default() }),
        image_model: None,
        web: Default::default(),
        idle_after: Duration::ZERO,
        compact_at: Some(0),
        store,
        spend,
        bypass: false,
        sudo_enabled: false,
        allow_rules: vec![],
        team: None,
        mcp: None,
        ask_all: false,
        config: Some(cfg.clone()),
    };
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    l.set_nonblocking(true).unwrap();
    let port = l.local_addr().unwrap().port();
    let token = auth::new_token();
    let state = AppState::new(model, settings);
    state.apply_settings(&lh_config::ConfigFile::load(&cfg).unwrap(), &[]); // what the core does at start
    let app = router(state, token.clone(), port);
    tokio::spawn(async move { axum::serve(tokio::net::TcpListener::from_std(l).unwrap(), app).await.unwrap() });
    Server { base: format!("http://127.0.0.1:{port}"), token, http: reqwest::Client::new(), seen, mem, cfg }
}

impl Server {
    fn req(&self, m: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        self.http.request(m, format!("{}{path}", self.base)).bearer_auth(&self.token)
    }
    async fn json(&self, rb: reqwest::RequestBuilder) -> Value {
        rb.send().await.unwrap().json().await.unwrap()
    }
    /// Change settings over the API and return the answer.
    async fn set(&self, body: Value) -> Value {
        let r = self.req(reqwest::Method::PUT, "/v1/settings").json(&body).send().await.unwrap();
        assert_eq!(r.status(), 200, "{body}");
        r.json().await.unwrap()
    }
    async fn session(&self, body: Value) -> String {
        self.json(self.req(reqwest::Method::POST, "/v1/sessions").json(&body)).await["id"].as_str().unwrap().to_string()
    }
    /// Run a turn; answers any question with `answer`. Returns every event until the turn ends.
    async fn turn(&self, id: &str, msg: &str, answer: &str) -> Vec<Value> {
        let mut resp = self.req(reqwest::Method::POST, &format!("/v1/sessions/{id}/turns")).json(&json!({"text": msg})).send().await.unwrap();
        let (mut p, mut seen) = (SseParser::new(), vec![]);
        'outer: while let Some(chunk) = tokio::time::timeout(Duration::from_secs(10), resp.chunk()).await.expect("no event in 10 s").unwrap() {
            for d in p.push(&chunk) {
                let v: Value = serde_json::from_str(&d).unwrap();
                if v["type"] == "ask" {
                    let ask = v["id"].as_str().unwrap().to_string();
                    self.req(reqwest::Method::POST, &format!("/v1/sessions/{id}/answers/{ask}")).json(&json!({"answer": answer})).send().await.unwrap();
                }
                let end = matches!(v["type"].as_str(), Some("done" | "error" | "refused" | "step_limit"));
                seen.push(v);
                if end {
                    break 'outer;
                }
            }
        }
        seen
    }
    async fn titles(&self) -> Vec<String> {
        self.json(self.req(reqwest::Method::GET, "/v1/chats")).await["chats"].as_array().unwrap().iter().map(|c| c["title"].as_str().unwrap().to_string()).collect()
    }
}
fn asked(events: &[Value]) -> bool {
    events.iter().any(|e| e["type"] == "ask")
}

#[tokio::test]
async fn permissions_apply_to_the_next_tool_call_of_a_chat_that_is_already_open() {
    let d = tempfile::tempdir().unwrap();
    let proj = tempfile::tempdir().unwrap();
    let s = start(d.path(), vec![write_call("c1", "a.txt"), text("one"), write_call("c2", "b.txt"), text("two"), write_call("c3", "c.txt"), text("three")], false);
    let id = s.session(json!({"model": "m", "project_dir": proj.path()})).await;

    // tiered: a write asks (and we say no)
    let ev = s.turn(&id, "write a", "no").await;
    assert!(asked(&ev));
    assert!(!proj.path().join("a.txt").exists());

    // bypass on, same chat: the next write goes straight through
    let r = s.set(json!({"permissions.bypass": true})).await;
    assert_eq!(r["restart_needed"], json!([]), "nothing to restart");
    let ev = s.turn(&id, "write b", "no").await;
    assert!(!asked(&ev), "{ev:?}");
    assert!(proj.path().join("b.txt").exists());

    // bypass off again: asks again
    s.set(json!({"permissions.bypass": false})).await;
    let ev = s.turn(&id, "write c", "no").await;
    assert!(asked(&ev));
    assert!(!proj.path().join("c.txt").exists());

    // the change is in the activity log, with when it took effect
    let log = s.json(s.req(reqwest::Method::GET, "/v1/activity?kind=setting_changed")).await;
    let entries = log["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    assert!(entries.iter().any(|e| e["detail"]["key"] == "permissions.bypass" && e["detail"]["value"] == "true" && e["detail"]["applies"] == "now"), "{log}");
}

/// The system prompt of an open chat tells the model the permission mode that is in force now, not the one the
/// chat started with.
#[tokio::test]
async fn an_open_chat_is_told_when_the_permission_mode_changes() {
    let d = tempfile::tempdir().unwrap();
    let proj = tempfile::tempdir().unwrap();
    let s = start(d.path(), vec![], false);
    let id = s.session(json!({"model": "m", "project_dir": proj.path()})).await;
    let system = |s: &Server| s.seen.lock().unwrap().iter().rev().find(|r| r.model != "titler").and_then(|r| r.messages[0].content.clone()).unwrap_or_default();
    s.turn(&id, "hi", "no").await;
    assert!(!system(&s).contains("permission prompts off"));
    s.set(json!({"permissions.bypass": true})).await;
    s.turn(&id, "again", "no").await;
    assert!(system(&s).contains("permission prompts off"), "{}", system(&s));
    s.set(json!({"permissions.bypass": false})).await;
    s.turn(&id, "and again", "no").await;
    assert!(!system(&s).contains("permission prompts off"));
}

/// "Undo the last turn" puts back the files that turn wrote and nothing from earlier turns.
#[tokio::test]
async fn the_last_turn_can_be_undone_as_a_whole() {
    let d = tempfile::tempdir().unwrap();
    let proj = tempfile::tempdir().unwrap();
    let s = start(d.path(), vec![write_call("c1", "a.txt"), text("one"), write_call("c2", "b.txt"), write_call("c3", "c.txt"), text("two")], false);
    s.set(json!({"permissions.bypass": true})).await;
    let id = s.session(json!({"model": "m", "project_dir": proj.path()})).await;
    s.turn(&id, "write a", "yes").await;
    s.turn(&id, "write b and c", "yes").await;
    assert!(proj.path().join("b.txt").exists() && proj.path().join("c.txt").exists());
    let r = s.json(s.req(reqwest::Method::POST, &format!("/v1/sessions/{id}/undo_turn"))).await;
    let files = r["files"].as_array().unwrap();
    assert_eq!(files.len(), 2, "{r}");
    assert!(files.iter().all(|f| f["result"] == "removed"), "{r}");
    assert!(!proj.path().join("b.txt").exists() && !proj.path().join("c.txt").exists());
    assert!(proj.path().join("a.txt").exists(), "the turn before is kept");
}

#[tokio::test]
async fn ask_for_everything_follows_the_setting() {
    let d = tempfile::tempdir().unwrap();
    let proj = tempfile::tempdir().unwrap();
    std::fs::write(proj.path().join("r.txt"), "hello").unwrap();
    let read = |id: &str| Response { tool_calls: vec![ToolCall { id: id.into(), name: "read_file".into(), arguments: json!({"path": "r.txt"}).to_string(), repaired: false }], finish: Some(FinishReason::ToolCalls), ..Default::default() };
    let s = start(d.path(), vec![read("c1"), text("a"), read("c2"), text("b")], false);
    let id = s.session(json!({"model": "m", "project_dir": proj.path()})).await;
    assert!(!asked(&s.turn(&id, "read", "no").await), "tiered: reads inside the project are free");
    s.set(json!({"permissions.mode": "ask-all"})).await;
    assert!(asked(&s.turn(&id, "read", "no").await), "ask-all: even a read asks");
}

#[tokio::test]
async fn a_spending_cap_changed_over_the_api_bites_on_the_next_call() {
    let d = tempfile::tempdir().unwrap();
    let s = start(d.path(), vec![], true);
    let id = s.session(json!({"model": "m", "mode": "chat"})).await;
    let ev = s.turn(&id, "hello", "no").await;
    assert_eq!(ev.last().unwrap()["type"], "done", "estimate mode lets a cheap call through: {ev:?}");

    // a tiny per-call cap, with caps mode on: refused
    let r = s.set(json!({"spending.mode": "caps", "spending.per_request_cap_usd": "0.0001"})).await;
    assert_eq!(r["restart_needed"], json!([]));
    let ev = s.turn(&id, "hello again", "no").await;
    assert_eq!(ev.last().unwrap()["type"], "refused", "{ev:?}");

    // clearing the cap lets it through again, still without a restart
    s.set(json!({"spending.per_request_cap_usd": ""})).await;
    let ev = s.turn(&id, "and now?", "no").await;
    assert_eq!(ev.last().unwrap()["type"], "done", "{ev:?}");
}

#[tokio::test]
async fn the_title_model_can_be_switched_on_while_running() {
    let d = tempfile::tempdir().unwrap();
    let s = start(d.path(), vec![], false);
    let a = s.session(json!({"model": "m", "mode": "chat"})).await;
    s.turn(&a, "plan my trip", "no").await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(s.titles().await, ["plan my trip"], "no title model: first words");

    s.set(json!({"chats.title_model": "titler"})).await;
    // a chat that is already open gets named by the model on its next first reply; a new one does too
    let b = s.session(json!({"model": "m", "mode": "chat"})).await;
    s.turn(&b, "what is rust", "no").await;
    let mut titles = vec![];
    for _ in 0..40 {
        titles = s.titles().await;
        if titles.iter().any(|t| t == "Named By Model") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(titles.iter().any(|t| t == "Named By Model"), "{titles:?}");
    assert!(s.seen.lock().unwrap().iter().any(|r| r.model == "titler"));
}

#[tokio::test]
async fn memory_recall_caps_apply_to_the_next_message() {
    let d = tempfile::tempdir().unwrap();
    let s = start(d.path(), vec![], false);
    for n in ["one", "two", "three"] {
        let mut p = NewPage::new(Layer::Facts, format!("Berlin trip {n}"), format!("Berlin plan {n}: see the sights."));
        p.gist = format!("Berlin trip {n}");
        p.privacy = Privacy::Normal;
        s.mem.create(p, By::User).unwrap();
    }
    let count_notes = |s: &Server| {
        let seen = s.seen.lock().unwrap();
        let user = seen.last().unwrap().messages.last().unwrap().content.clone().unwrap();
        user.matches("Berlin trip").count()
    };
    let a = s.session(json!({"model": "m", "mode": "chat"})).await;
    s.turn(&a, "tell me about Berlin", "no").await;
    assert_eq!(count_notes(&s), 3, "default limits let all three in");

    s.set(json!({"memory.recall_items": 1})).await;
    let b = s.session(json!({"model": "m", "mode": "chat"})).await;
    s.turn(&b, "tell me about Berlin", "no").await;
    assert_eq!(count_notes(&s), 1, "one note after the cap is lowered");

    // an already open chat takes the new limit on its next message too
    s.set(json!({"memory.recall_items": 2})).await;
    s.turn(&b, "more about Berlin sights", "no").await;
    assert!(count_notes(&s) <= 2);
}

#[tokio::test]
async fn the_list_says_what_applies_when() {
    let d = tempfile::tempdir().unwrap();
    let s = start(d.path(), vec![], false);
    let list = s.json(s.req(reqwest::Method::GET, "/v1/settings")).await;
    let live = |k: &str| list["settings"].as_array().unwrap().iter().find(|i| i["key"] == k).unwrap_or_else(|| panic!("{k}"))["live"].as_bool().unwrap();
    for k in ["spending.mode", "spending.daily_cap_usd", "permissions.mode", "permissions.bypass", "permissions.sudo", "chats.title_model", "tree.summarize", "kvcache.dir", "core.idle_after", "core.compact_at", "memory.recall_items", "web.searxng", "media.image_model"] {
        assert!(live(k), "{k} should apply at once");
    }
    for k in ["provider.url", "provider.key", "memory.enabled", "memory.trust", "learning.mode", "web.listen"] {
        assert!(!live(k), "{k} needs a restart");
    }
    // a restart-only change is reported as such; a new-chats-only one says so
    let r = s.set(json!({"provider.name": "x", "web.searxng": "http://localhost:8080"})).await;
    assert_eq!(r["restart_needed"], json!(["provider.name"]));
    assert_eq!(r["new_chats_only"], json!(["web.searxng"]));
    assert!(std::fs::read_to_string(&s.cfg).unwrap().contains("searxng"));
}
