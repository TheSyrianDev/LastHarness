//! Naming a chat with `chats.title_model`: set, failing, private, and cloud with sensitive text.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use lh_core::{auth, router, AppState, MemoryRuntime, Settings};
use lh_memory::embed::AnyEmbedder;
use lh_memory::{Memory, RecallConfig, Trust};
use lh_provider::{ChatRequest, Event, FinishReason, Response, SseParser};
use lh_store::Store;
use lh_tools::Model;
use serde_json::{json, Value};

/// Chat requests get "Hi back"; requests for the model `titler` get `title` (or an error when it is `None`).
struct Fake {
    title: Option<String>,
    titler_calls: Arc<Mutex<Vec<ChatRequest>>>,
}
impl Model for Fake {
    async fn complete(&self, r: &ChatRequest, on: &mut (dyn FnMut(Event) + Send)) -> Result<Response, lh_provider::Error> {
        if r.model == "titler" {
            self.titler_calls.lock().unwrap().push(r.clone());
            return match &self.title {
                Some(t) => Ok(Response { text: t.clone(), finish: Some(FinishReason::Stop), ..Default::default() }),
                None => Err(lh_provider::Error::Format("down".into())),
            };
        }
        on(Event::Text("Hi back".into()));
        Ok(Response { text: "Hi back".into(), finish: Some(FinishReason::Stop), ..Default::default() })
    }
}

struct Server {
    base: String,
    token: String,
    http: reqwest::Client,
    calls: Arc<Mutex<Vec<ChatRequest>>>,
    store: Arc<Store>,
}
impl Server {
    async fn json(&self, rb: reqwest::RequestBuilder) -> Value {
        rb.bearer_auth(&self.token).send().await.unwrap().json().await.unwrap()
    }
    async fn session(&self, incognito: bool) -> String {
        let b = json!({"model": "m", "mode": "chat", "incognito": incognito});
        self.json(self.http.post(format!("{}/v1/sessions", self.base)).json(&b)).await["id"].as_str().unwrap().to_string()
    }
    async fn turn(&self, id: &str, msg: &str) {
        let mut resp = self.http.post(format!("{}/v1/sessions/{id}/turns", self.base)).bearer_auth(&self.token).json(&json!({"text": msg})).send().await.unwrap();
        let mut p = SseParser::new();
        while let Some(chunk) = tokio::time::timeout(Duration::from_secs(10), resp.chunk()).await.unwrap().unwrap() {
            for d in p.push(&chunk) {
                if serde_json::from_str::<Value>(&d).unwrap()["type"] == "done" {
                    return;
                }
            }
        }
    }
    async fn titles(&self) -> Vec<String> {
        self.json(self.http.get(format!("{}/v1/chats", self.base))).await["chats"].as_array().unwrap().iter().map(|c| c["title"].as_str().unwrap().to_string()).collect()
    }
    /// Wait for the background job: a title other than the first words, or give up.
    async fn wait_title(&self, not: &str) -> String {
        for _ in 0..40 {
            let t = self.titles().await.pop().unwrap_or_default();
            if t != not {
                return t;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        not.to_string()
    }
}

fn start(dir: &std::path::Path, title: Option<&str>, setting: &str, trust: Trust) -> Server {
    let store = Arc::new(Store::open(&dir.join("lh.db")).unwrap());
    let mem = Arc::new(Memory::open(store.clone(), dir.join("memory")).unwrap());
    let calls: Arc<Mutex<Vec<ChatRequest>>> = Default::default();
    let model = Fake { title: title.map(String::from), titler_calls: calls.clone() };
    let spend = lh_spend::SpendMeter::new(store.clone(), lh_spend::Config::default(), "test", true);
    let settings = Settings {
        provider_name: None,
        learn: None,
        maintain_when_idle: false,
        memory: Some(MemoryRuntime { memory: mem, embedder: Arc::new(AnyEmbedder::Off), trust, cfg: RecallConfig::default() }),
        image_model: None,
        web: Default::default(),
        idle_after: Duration::ZERO,
        compact_at: Some(0),
        store: store.clone(),
        spend,
        bypass: false,
        sudo_enabled: false,
        allow_rules: vec![],
        team: None,
        mcp: None,
        ask_all: false,
        config: None,
    };
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    l.set_nonblocking(true).unwrap();
    let port = l.local_addr().unwrap().port();
    let token = auth::new_token();
    let state = AppState::new(model, settings);
    state.set_title_model(setting);
    let app = router(state, token.clone(), port);
    tokio::spawn(async move { axum::serve(tokio::net::TcpListener::from_std(l).unwrap(), app).await.unwrap() });
    Server { base: format!("http://127.0.0.1:{port}"), token, http: reqwest::Client::new(), calls, store }
}

/// The title comes after the turn's stream has closed, so open clients hear of it on `/v1/events`.
#[tokio::test]
async fn the_title_is_announced_to_listening_clients() {
    let dir = tempfile::tempdir().unwrap();
    let s = start(dir.path(), Some("Trip to Rome"), "titler", Trust::Local);
    let mut events = s.http.get(format!("{}/v1/events", s.base)).bearer_auth(&s.token).send().await.unwrap();
    assert!(events.status().is_success());
    let id = s.session(false).await;
    s.turn(&id, "plan my trip").await;
    let mut p = SseParser::new();
    loop {
        let chunk = tokio::time::timeout(Duration::from_secs(10), events.chunk()).await.expect("a title event in time").unwrap().unwrap();
        for d in p.push(&chunk) {
            let v: Value = serde_json::from_str(&d).unwrap();
            if v["type"] == "title" {
                assert_eq!(v["title"], "Trip to Rome");
                assert_eq!(v["session"], id.as_str());
                return;
            }
        }
    }
}

#[tokio::test]
async fn model_names_the_chat() {
    let dir = tempfile::tempdir().unwrap();
    let s = start(dir.path(), Some("\"Trip to Rome\"."), "titler", Trust::Local);
    let id = s.session(false).await;
    s.turn(&id, "plan my trip").await;
    assert_eq!(s.wait_title("plan my trip").await, "Trip to Rome");
    let req = s.calls.lock().unwrap()[0].clone();
    assert!(req.max_tokens.unwrap() <= 64);
    assert_eq!(req.sampling.extra["chat_template_kwargs"]["enable_thinking"], false);
    let logged = s.store.recent(50, Some("chat_title_request")).unwrap();
    assert_eq!(logged.len(), 1, "the request is in the activity log");
    // a second message does not name it again
    s.turn(&id, "and then?").await;
    assert_eq!(s.calls.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn failure_or_unset_keeps_first_words() {
    let dir = tempfile::tempdir().unwrap();
    let s = start(dir.path(), None, "titler", Trust::Local);
    let id = s.session(false).await;
    s.turn(&id, "plan my trip").await;
    assert_eq!(s.wait_title("plan my trip").await, "plan my trip");
    assert_eq!(s.calls.lock().unwrap().len(), 1);

    let dir = tempfile::tempdir().unwrap();
    let s = start(dir.path(), Some("X"), "", Trust::Local);
    let id = s.session(false).await;
    s.turn(&id, "hello").await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(s.calls.lock().unwrap().is_empty());
    assert_eq!(s.titles().await, vec!["hello"]);
}

#[tokio::test]
async fn incognito_and_cloud_sensitive_are_not_titled() {
    let dir = tempfile::tempdir().unwrap();
    let s = start(dir.path(), Some("X"), "titler", Trust::Local);
    let id = s.session(true).await;
    s.turn(&id, "secret chat").await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(s.calls.lock().unwrap().is_empty());

    let dir = tempfile::tempdir().unwrap();
    let s = start(dir.path(), Some("X"), "titler", Trust::AnyCloud);
    let id = s.session(false).await;
    s.turn(&id, "my password is hunter2 please remember").await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(s.calls.lock().unwrap().is_empty());
}
