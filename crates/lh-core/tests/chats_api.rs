//! Stored chats through the real local API: the list, delete, and opening a chat again (after a restart, with another model, and the privacy guard).
use std::sync::{Arc, Mutex};
use std::time::Duration;

use lh_core::{auth, router, AppState, MemoryRuntime, Settings};
use lh_memory::embed::AnyEmbedder;
use lh_memory::{By, Layer, Memory, NewPage, Privacy, RecallConfig, Trust};
use lh_provider::{ChatRequest, Event, FinishReason, Response, SseParser};
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
        if !resp.text.is_empty() {
            on(Event::Text(resp.text.clone()));
        }
        Ok(resp)
    }
}
fn text(t: &str) -> Response {
    Response { text: t.into(), finish: Some(FinishReason::Stop), ..Default::default() }
}

struct Server {
    base: String,
    token: String,
    http: reqwest::Client,
    seen: Arc<Mutex<Vec<ChatRequest>>>,
    mem: Arc<Memory>,
}
impl Server {
    fn post(&self, path: &str, body: Value) -> reqwest::RequestBuilder {
        self.http.post(format!("{}{path}", self.base)).bearer_auth(&self.token).json(&body)
    }
    fn get(&self, path: &str) -> reqwest::RequestBuilder {
        self.http.get(format!("{}{path}", self.base)).bearer_auth(&self.token)
    }
    async fn json(&self, rb: reqwest::RequestBuilder) -> Value {
        rb.send().await.unwrap().json().await.unwrap()
    }
    async fn session(&self, extra: Value) -> String {
        let mut body = json!({"model": "m", "mode": "chat"});
        for (k, v) in extra.as_object().unwrap() {
            body[k] = v.clone();
        }
        self.json(self.post("/v1/sessions", body)).await["id"].as_str().unwrap().to_string()
    }
    /// Run one turn and return the events.
    async fn turn(&self, id: &str, msg: &str) -> Vec<Value> {
        let mut resp = self.post(&format!("/v1/sessions/{id}/turns"), json!({"text": msg})).send().await.unwrap();
        let (mut p, mut seen) = (SseParser::new(), vec![]);
        'outer: while let Some(chunk) = tokio::time::timeout(Duration::from_secs(10), resp.chunk()).await.expect("no event in 10 s").unwrap() {
            for d in p.push(&chunk) {
                let v: Value = serde_json::from_str(&d).unwrap();
                let end = matches!(v["type"].as_str(), Some("done" | "error" | "refused"));
                seen.push(v);
                if end {
                    break 'outer;
                }
            }
        }
        seen
    }
}

fn start_at(dir: &std::path::Path, replies: Vec<Response>, trust: Trust) -> Server {
    start_priced(dir, replies, trust, vec![])
}

fn start_priced(dir: &std::path::Path, replies: Vec<Response>, trust: Trust, prices: Vec<lh_spend::Price>) -> Server {
    let store = Arc::new(Store::open(&dir.join("lh.db")).unwrap());
    let mem = Arc::new(Memory::open(store.clone(), dir.join("memory")).unwrap());
    let mut r = replies;
    r.reverse();
    let seen: Arc<Mutex<Vec<ChatRequest>>> = Default::default();
    let model = Script { replies: Mutex::new(r), seen: seen.clone() };
    let spend = lh_spend::SpendMeter::new(store.clone(), lh_spend::Config { prices, ..Default::default() }, "test", true);
    let settings = Settings {
        provider_name: None,
        learn: None,
        maintain_when_idle: false,
        memory: Some(MemoryRuntime { memory: mem.clone(), embedder: Arc::new(AnyEmbedder::Off), trust, cfg: RecallConfig::default() }),
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
    let app = router(AppState::new(model, settings), token.clone(), port);
    tokio::spawn(async move { axum::serve(tokio::net::TcpListener::from_std(l).unwrap(), app).await.unwrap() });
    Server { base: format!("http://127.0.0.1:{port}"), token, http: reqwest::Client::new(), seen, mem }
}

fn model_of(s: &Server) -> String {
    s.seen.lock().unwrap().last().unwrap().model.clone()
}

#[tokio::test]
async fn chats_are_listed_titled_and_deleted() {
    let dir = tempfile::tempdir().unwrap();
    let s = start_at(dir.path(), vec![text("Hi back")], Trust::Local);
    let id = s.session(json!({})).await;
    s.turn(&id, "hello   there\nworld").await;
    let list = s.json(s.get("/v1/chats")).await;
    let chats = list["chats"].as_array().unwrap();
    assert_eq!(chats.len(), 1);
    assert_eq!(chats[0]["id"], id.as_str());
    assert_eq!(chats[0]["title"], "hello there world");
    assert_eq!(chats[0]["mode"], "chat");
    assert_eq!(chats[0]["model"], "m");
    assert_eq!(chats[0]["messages"], 2);
    assert_eq!(chats[0]["live"], true);
    let one = s.json(s.get(&format!("/v1/chats/{id}"))).await;
    assert_eq!(one["history"][1]["content"], "Hi back");

    let r = s.http.delete(format!("{}/v1/chats/{id}", s.base)).bearer_auth(&s.token).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 204);
    assert!(s.json(s.get("/v1/chats")).await["chats"].as_array().unwrap().is_empty());
    assert!(s.mem.search_conversations("hello", 5).unwrap().is_empty(), "the search index forgot the chat too");
    let r = s.http.delete(format!("{}/v1/chats/{id}", s.base)).bearer_auth(&s.token).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 404);
    // the session is gone too
    let r = s.post(&format!("/v1/sessions/{id}/turns"), json!({"text": "x"})).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 404);
}

#[tokio::test]
async fn a_restart_does_not_reuse_the_id_of_a_stored_chat() {
    let dir = tempfile::tempdir().unwrap();
    let first = {
        let s = start_at(dir.path(), vec![text("one")], Trust::Local);
        let id = s.session(json!({})).await;
        s.turn(&id, "first chat").await;
        id
    };
    let s = start_at(dir.path(), vec![text("two")], Trust::Local);
    let id2 = s.session(json!({})).await;
    assert_ne!(first, id2);
    s.turn(&id2, "second chat").await;
    let chats = s.json(s.get("/v1/chats")).await;
    assert_eq!(chats["chats"].as_array().unwrap().len(), 2, "two separate chats, not one mixed together");
}

#[tokio::test]
async fn a_stored_chat_continues_after_a_restart_with_a_warning_first() {
    let dir = tempfile::tempdir().unwrap();
    let id = {
        let s = start_at(dir.path(), vec![text("It is blue.")], Trust::Local);
        let id = s.session(json!({})).await;
        s.turn(&id, "What colour is the sky?").await;
        id
    };
    let s = start_at(dir.path(), vec![text("Because of scattering.")], Trust::Local);
    let list = s.json(s.get("/v1/chats")).await;
    assert_eq!(list["chats"][0]["live"], false);
    // without confirmation: only the warning
    let w = s.json(s.post(&format!("/v1/chats/{id}/resume"), json!({}))).await;
    assert_eq!(w["needs_confirmation"], true);
    assert_eq!(w["warning"]["kind"], "local");
    assert_eq!(w["warning"]["same_model"], true);
    assert!(w["warning"]["context_tokens"].as_u64().unwrap() > 200);
    assert!(w["warning"]["message"].as_str().unwrap().contains("read the whole chat again"));
    assert!(s.json(s.get("/v1/status")).await["sessions"] == 0, "nothing was opened yet");
    // confirmed
    let r = s.json(s.post(&format!("/v1/chats/{id}/resume"), json!({"confirm": true}))).await;
    assert_eq!(r["session"], id.as_str());
    assert_eq!(r["resumed_messages"], 2);
    s.turn(&id, "And why is that?").await;
    let sent = s.seen.lock().unwrap().last().unwrap().messages.clone();
    let texts: Vec<String> = sent.iter().map(|m| m.content.clone().unwrap_or_default()).collect();
    assert!(texts.iter().any(|t| t.contains("What colour is the sky?")), "{texts:?}");
    assert!(texts.iter().any(|t| t == "It is blue."), "{texts:?}");
    assert!(texts.last().unwrap().contains("And why is that?"));
    // what is said next lands in the same stored chat
    let one = s.json(s.get(&format!("/v1/chats/{id}"))).await;
    assert_eq!(one["messages"], 4);
    assert_eq!(s.json(s.get("/v1/chats")).await["chats"].as_array().unwrap().len(), 1);
    // a loaded session with the same model is simply continued
    let again = s.json(s.post(&format!("/v1/chats/{id}/resume"), json!({}))).await;
    assert_eq!(again["live"], true);
    assert!(again.get("needs_confirmation").is_none());
}

#[tokio::test]
async fn a_chat_can_continue_with_another_model() {
    let dir = tempfile::tempdir().unwrap();
    let s = start_at(dir.path(), vec![text("A"), text("B"), text("C")], Trust::Local);
    let id = s.session(json!({})).await;
    s.turn(&id, "first").await;
    assert_eq!(model_of(&s), "m");
    // loaded session, other model: warning, then the switch
    let w = s.json(s.post(&format!("/v1/chats/{id}/resume"), json!({"model": "other"}))).await;
    assert_eq!(w["needs_confirmation"], true);
    assert_eq!(w["warning"]["same_model"], false);
    s.json(s.post(&format!("/v1/chats/{id}/resume"), json!({"model": "other", "confirm": true}))).await;
    s.turn(&id, "second").await;
    assert_eq!(model_of(&s), "other");
    assert_eq!(s.json(s.get("/v1/chats")).await["chats"][0]["model"], "other");
}

#[tokio::test]
async fn a_chat_that_repeats_a_sensitive_note_never_goes_to_a_cloud_model() {
    let dir = tempfile::tempdir().unwrap();
    let secret = "the vault code is purple elephant seven";
    let id = {
        let s = start_at(dir.path(), vec![text("Noted.")], Trust::Local);
        let mut n = NewPage::new(Layer::Facts, "vault", secret);
        n.privacy = Privacy::Sensitive;
        s.mem.create(n, By::User).unwrap();
        let id = s.session(json!({})).await;
        s.turn(&id, &format!("Remember that {secret}, please.")).await;
        id
    };
    // the same chat, now behind a cloud endpoint
    let cloud = start_at(dir.path(), vec![], Trust::AnyCloud);
    let w = cloud.json(cloud.post(&format!("/v1/chats/{id}/resume"), json!({}))).await;
    assert_eq!(w["warning"]["kind"], "cloud");
    assert!(w["warning"]["message"].as_str().unwrap().contains("spends credits"));
    let r = cloud.post(&format!("/v1/chats/{id}/resume"), json!({"confirm": true})).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 409);
    let body: Value = r.json().await.unwrap();
    assert!(body["error"].as_str().unwrap().contains("sensitive"), "{body}");
    assert_eq!(cloud.json(cloud.get("/v1/status")).await["sessions"], 0, "no session was opened");
    assert!(cloud.seen.lock().unwrap().is_empty(), "nothing reached the model");
    // a chat without sensitive text may go to the cloud, after confirmation
    let plain = start_at(dir.path(), vec![text("Hi")], Trust::AnyCloud);
    let id2 = plain.session(json!({})).await;
    plain.turn(&id2, "Tell me a joke about clouds.").await;
    let ok = plain.json(plain.post(&format!("/v1/chats/{id2}/resume"), json!({"model": "cloud-model", "confirm": true}))).await;
    assert_eq!(ok["live"], true);
}

#[tokio::test]
async fn the_cloud_warning_gives_a_cost_only_when_the_model_has_a_listed_price() {
    let dir = tempfile::tempdir().unwrap();
    let id = {
        let s = start_at(dir.path(), vec![text("Hi")], Trust::Local);
        let id = s.session(json!({})).await;
        s.turn(&id, "Tell me about clouds").await;
        id
    };
    let price = lh_spend::Price { model: "priced-*".into(), input: 3.0, output: 15.0, cached_input: None, image: None };
    let cloud = start_priced(dir.path(), vec![], Trust::AnyCloud, vec![price]);
    let w = cloud.json(cloud.post(&format!("/v1/chats/{id}/resume"), json!({"model": "priced-big"}))).await;
    let tokens = w["warning"]["context_tokens"].as_u64().unwrap();
    assert_eq!(w["warning"]["estimated_cost_micro_usd"].as_i64().unwrap(), (tokens * 3) as i64, "tokens times dollars per million");
    let msg = w["warning"]["message"].as_str().unwrap();
    assert!(msg.contains("about $0.00") && msg.contains("$3 per million input tokens"), "{msg}");
    let w = cloud.json(cloud.post(&format!("/v1/chats/{id}/resume"), json!({"model": "unlisted"}))).await;
    assert!(w["warning"]["estimated_cost_micro_usd"].is_null());
    assert!(w["warning"]["message"].as_str().unwrap().contains("No price is listed"));
    // a local endpoint never talks about money
    let local = start_priced(dir.path(), vec![], Trust::Local, vec![lh_spend::Price { model: "*".into(), input: 3.0, output: 3.0, cached_input: None, image: None }]);
    let w = local.json(local.post(&format!("/v1/chats/{id}/resume"), json!({"model": "any"}))).await;
    assert!(w["warning"]["estimated_cost_micro_usd"].is_null() && !w["warning"]["message"].as_str().unwrap().contains('$'));
}
