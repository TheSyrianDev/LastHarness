//! Several registered providers over the real local API: add, default, remove, proposals the user approves,
//! the model's tools (list, select, propose) and that no key ever leaves the secret store.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use lh_core::{auth, router, AppState, Settings};
use lh_provider::{ChatRequest, Event, FinishReason, Response};
use lh_secrets::{MemoryStore, SecretStore};
use lh_store::Store;
use lh_tools::Model;
use serde_json::{json, Value};

struct Script {
    replies: Mutex<Vec<Response>>,
    seen: Arc<Mutex<Vec<ChatRequest>>>,
}
impl Model for Script {
    async fn complete(&self, r: &ChatRequest, on: &mut (dyn FnMut(Event) + Send)) -> Result<Response, lh_provider::Error> {
        self.seen.lock().unwrap().push(r.clone());
        let resp = self.replies.lock().unwrap().pop().expect("unscripted model call");
        on(Event::Text(resp.text.clone()));
        Ok(resp)
    }
}
fn text(t: &str) -> Response {
    Response { text: t.into(), finish: Some(FinishReason::Stop), ..Default::default() }
}
fn call(name: &str, args: Value) -> Response {
    Response { tool_calls: vec![lh_provider::ToolCall { id: "c1".into(), name: name.into(), arguments: args.to_string(), repaired: false }], finish: Some(FinishReason::ToolCalls), ..Default::default() }
}

struct Server {
    base: String,
    token: String,
    http: reqwest::Client,
    seen: Arc<Mutex<Vec<ChatRequest>>>,
    secrets: Arc<MemoryStore>,
    dir: tempfile::TempDir,
}
impl Server {
    async fn call(&self, m: &str, p: &str, b: Option<Value>) -> (u16, Value) {
        let mut rb = self.http.request(m.parse().unwrap(), format!("{}{p}", self.base)).bearer_auth(&self.token);
        if let Some(b) = b {
            rb = rb.json(&b);
        }
        let r = rb.send().await.unwrap();
        (r.status().as_u16(), r.json().await.unwrap_or(Value::Null))
    }
    /// Run one chat turn to its end and return the raw event stream.
    async fn turn(&self, id: &str, t: &str) -> String {
        self.http.post(format!("{}/v1/sessions/{id}/turns", self.base)).bearer_auth(&self.token).json(&json!({"text": t})).send().await.unwrap().text().await.unwrap()
    }
}

fn start(replies: Vec<Response>, config: bool) -> Server {
    let mut replies = replies;
    replies.reverse();
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(Store::open(&dir.path().join("lh.db")).unwrap());
    let seen: Arc<Mutex<Vec<ChatRequest>>> = Default::default();
    let settings = Settings {
        team: None,
        mcp: None,
        ask_all: false,
        config: config.then(|| dir.path().join("config.toml")),
        provider_name: None,
        learn: None,
        maintain_when_idle: false,
        memory: None,
        image_model: None,
        web: Default::default(),
        idle_after: Duration::ZERO,
        compact_at: Some(0),
        store: store.clone(),
        spend: lh_spend::SpendMeter::new(store, lh_spend::Config::default(), "test", true),
        bypass: false,
        sudo_enabled: false,
        allow_rules: vec![],
    };
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    l.set_nonblocking(true).unwrap();
    let port = l.local_addr().unwrap().port();
    let token = auth::new_token();
    let state = AppState::new(Script { replies: Mutex::new(replies), seen: seen.clone() }, settings);
    let secrets = Arc::new(MemoryStore::default());
    let as_dyn: Arc<dyn SecretStore> = secrets.clone();
    assert!(state.secrets.set(as_dyn).is_ok());
    let app = router(state, token.clone(), port);
    tokio::spawn(async move { axum::serve(tokio::net::TcpListener::from_std(l).unwrap(), app).await.unwrap() });
    Server { base: format!("http://127.0.0.1:{port}"), token, http: reqwest::Client::new(), seen, secrets, dir }
}

#[tokio::test]
async fn providers_are_added_defaulted_removed_and_the_key_stays_in_the_secret_store() {
    let s = start(vec![], true);
    let (c, a) = s.call("POST", "/v1/providers", Some(json!({"url": "http://localhost:9292/v1", "name": "Box"}))).await;
    assert_eq!((c, a["added"]["default"].clone()), (200, json!(true)), "{a}");
    let (_, b) = s.call("POST", "/v1/providers", Some(json!({"preset": "openrouter", "key": "sk-or-SECRET-123"}))).await;
    assert_eq!(b["added"]["id"], "openrouter");
    let (_, l) = s.call("GET", "/v1/providers", None).await;
    assert_eq!((l["default"].clone(), l["providers"].as_array().unwrap().len()), (json!("box"), 2));
    assert_eq!(l["providers"][1]["key_saved"], true);
    // nothing on the wire, in the registry file, or in the activity log carries the key
    let file = std::fs::read_to_string(s.dir.path().join("providers.json")).unwrap();
    let (_, act) = s.call("GET", "/v1/activity", None).await;
    for text in [l.to_string(), b.to_string(), file.clone(), act.to_string()] {
        assert!(!text.contains("sk-or-SECRET-123"), "key leaked: {text}");
    }
    assert!(file.contains("secret:provider-openrouter"));
    assert_eq!(s.secrets.get("provider-openrouter").unwrap().as_deref(), Some("sk-or-SECRET-123"));
    assert_eq!(s.call("DELETE", "/v1/providers/box", None).await.0, 400, "the default cannot be removed");
    assert_eq!(s.call("POST", "/v1/providers/openrouter/default", None).await.0, 200);
    assert_eq!(s.call("DELETE", "/v1/providers/box", None).await.0, 200);
    assert_eq!(s.call("POST", "/v1/providers", Some(json!({"url": "ftp://x"}))).await.0, 400);
    s.call("DELETE", "/v1/providers/openrouter", None).await;
}

#[tokio::test]
async fn the_old_single_provider_setting_becomes_the_first_entry() {
    let s = start(vec![], true);
    std::fs::write(s.dir.path().join("config.toml"), "[provider]\nurl = \"http://localhost:9292/v1\"\nkey = \"secret:provider-key\"\n").unwrap();
    let (_, l) = s.call("GET", "/v1/providers", None).await;
    assert_eq!((l["default"].clone(), l["providers"][0]["url"].clone(), l["providers"][0]["key_saved"].clone()), (json!("main"), json!("http://localhost:9292/v1"), json!(true)));
    assert!(!l.to_string().contains("secret:"));
}

#[tokio::test]
async fn the_model_lists_selects_and_proposes_but_only_the_user_registers() {
    let s = start(
        vec![
            call("list_models", json!({})),
            call("select_model", json!({"model": "box:m2"})),
            call("propose_provider", json!({"preset": "openrouter", "reason": "cheap big models", "key": "sk-leak-attempt"})),
            text("done"),
            text("second"),
        ],
        true,
    );
    s.call("POST", "/v1/providers", Some(json!({"url": "http://127.0.0.1:1/v1", "name": "Box"}))).await;
    let (_, sess) = s.call("POST", "/v1/sessions", Some(json!({"model": "m1", "mode": "assistant"}))).await;
    let id = sess["id"].as_str().unwrap().to_string();
    let out = s.turn(&id, "use another model").await;
    assert!(out.contains("list_models"), "{out}");
    // the tool results reached the model: the registered provider, never a key
    let seen = s.seen.lock().unwrap().clone();
    let all = format!("{:?}", seen);
    assert!(all.contains("box"));
    assert!(all.contains("box:m2"), "{all}");
    // nothing registered by the proposal
    let (_, l) = s.call("GET", "/v1/providers", None).await;
    assert_eq!(l["providers"].as_array().unwrap().len(), 1);
    let (_, pr) = s.call("GET", "/v1/providers/proposals", None).await;
    assert_eq!(pr["proposals"].as_array().unwrap().len(), 1);
    assert!(!pr.to_string().contains("sk-leak-attempt"));
    // the switch is in the activity log and the next turn uses it
    let (_, act) = s.call("GET", "/v1/activity", None).await;
    assert!(act.to_string().contains("model_selected") && act.to_string().contains("provider_proposed"), "{act}");
    s.turn(&id, "go on").await;
    assert_eq!(s.seen.lock().unwrap().last().unwrap().model, "box:m2");
    // the user approves and types the key
    let (c, ok) = s.call("POST", "/v1/providers/proposals/p1/approve", Some(json!({"key": "sk-user-typed"}))).await;
    assert_eq!(c, 200, "{ok}");
    assert_eq!(s.secrets.get("provider-openrouter").unwrap().as_deref(), Some("sk-user-typed"));
    assert_eq!(s.call("POST", "/v1/providers/proposals/p1/reject", None).await.0, 404);
}
