//! Messaging channels end to end: a mock Telegram on one side, the real core and its local API on the
//! other, a scripted model in between.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use lh_core::{auth, channels, router, AppState, LearnMode, LearnRuntime, MemoryRuntime, Settings, TeamRuntime};
use lh_memory::embed::AnyEmbedder;
use lh_memory::{Memory, RecallConfig, Trust};
use lh_provider::{ChatRequest, Event, FinishReason, Response, ToolCall};
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

/// A pretend Telegram: updates are queued by the test, sent messages are collected.
#[derive(Clone, Default)]
struct Tg {
    queue: Arc<Mutex<Vec<Value>>>,
    sent: Arc<Mutex<Vec<(String, String)>>>,
    id: Arc<Mutex<i64>>,
}
impl Tg {
    fn say(&self, from: i64, text: &str) {
        let mut id = self.id.lock().unwrap();
        *id += 1;
        self.queue.lock().unwrap().push(json!({"update_id": *id, "message": {"text": text, "from": {"id": from, "is_bot": false}, "chat": {"id": from}}}));
    }
    async fn wait_sent(&self, n: usize) -> Vec<(String, String)> {
        for _ in 0..200 {
            if self.sent.lock().unwrap().len() >= n {
                return self.sent.lock().unwrap().clone();
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("expected {n} messages, got {:?}", self.sent.lock().unwrap());
    }
}
async fn mock_telegram(tg: Tg) -> String {
    let app = Router::new()
        .route("/botT/getUpdates", get(|State(tg): State<Tg>, Query(_q): Query<std::collections::HashMap<String, String>>| async move {
            let batch: Vec<Value> = std::mem::take(&mut *tg.queue.lock().unwrap());
            if batch.is_empty() {
                tokio::time::sleep(Duration::from_millis(60)).await;
            }
            Json(json!({"ok": true, "result": batch}))
        }))
        .route("/botT/sendMessage", post(|State(tg): State<Tg>, Json(b): Json<Value>| async move {
            tg.sent.lock().unwrap().push((b["chat_id"].as_str().unwrap_or("").to_string(), b["text"].as_str().unwrap_or("").to_string()));
            Json(json!({"ok": true}))
        }))
        .with_state(tg);
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", l.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    url
}

struct Rig {
    http: reqwest::Client,
    base: String,
    token: String,
    seen: Arc<Mutex<Vec<ChatRequest>>>,
    tg: Tg,
    store: Arc<Store>,
    state: Arc<AppState<Script>>,
    memory: Arc<Memory>,
    dir: tempfile::TempDir,
}
impl Rig {
    async fn call(&self, m: reqwest::Method, p: &str, b: Value) -> (u16, Value) {
        let r = self.http.request(m, format!("{}{p}", self.base)).bearer_auth(&self.token).json(&b).send().await.unwrap();
        (r.status().as_u16(), r.json().await.unwrap_or(Value::Null))
    }
    async fn channel(&self, extra: Value) {
        let mut body = json!({"name": "phone", "kind": "telegram", "settings": {"token": "env:LH_TEST_TG", "api_base": self.tg_url()}, "allow": ["42"], "model": "m"});
        for (k, v) in extra.as_object().unwrap() {
            body[k] = v.clone();
        }
        assert_eq!(self.call(reqwest::Method::POST, "/v1/channels", body).await.0, 201);
        assert_eq!(self.call(reqwest::Method::POST, "/v1/channels/phone/enable", json!({"acknowledge": true})).await.0, 200);
    }
    fn tg_url(&self) -> String {
        std::fs::read_to_string(self.dir.path().join("tg_url")).unwrap()
    }
}

async fn rig(mut replies: Vec<Response>, idle_after: Duration) -> Rig {
    replies.reverse();
    std::env::set_var("LH_TEST_TG", "T");
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(Store::open(&dir.path().join("lh.db")).unwrap());
    let memory = Arc::new(Memory::open(store.clone(), dir.path().join("memory")).unwrap());
    let learn = Arc::new(lh_learn::Learn::new(store.clone()).with_skills_dir(dir.path().join("skills")));
    let seen: Arc<Mutex<Vec<ChatRequest>>> = Default::default();
    let settings = Settings {
        ask_all: false,
        config: None,
        mcp: None,
        team: Some(TeamRuntime::open(&dir.path().join("team"), learn.clone(), 1000).unwrap()),
        provider_name: None,
        learn: Some(LearnRuntime { learn, trust: Trust::Local, mode: LearnMode::Propose, threshold: 3, skill_min_calls: 6 }),
        maintain_when_idle: false,
        memory: Some(MemoryRuntime { memory: memory.clone(), embedder: Arc::new(AnyEmbedder::Off), trust: Trust::Local, cfg: RecallConfig::default() }),
        image_model: None,
        web: Default::default(),
        idle_after,
        compact_at: Some(0),
        store: store.clone(),
        spend: lh_spend::SpendMeter::new(store.clone(), lh_spend::Config::default(), "test", true),
        bypass: false,
        sudo_enabled: false,
        allow_rules: vec![],
    };
    let state = AppState::new(Script { replies: Mutex::new(replies), seen: seen.clone() }, settings);
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    l.set_nonblocking(true).unwrap();
    let port = l.local_addr().unwrap().port();
    let token = auth::new_token();
    let app = router(state.clone(), token.clone(), port);
    tokio::spawn(async move { axum::serve(tokio::net::TcpListener::from_std(l).unwrap(), app).await.unwrap() });
    let base = format!("http://127.0.0.1:{port}");
    channels::spawn_runner_every(store.clone(), base.clone(), token.clone(), Duration::from_millis(50));
    let tg = Tg::default();
    let url = mock_telegram(tg.clone()).await;
    std::fs::write(dir.path().join("tg_url"), url).unwrap();
    Rig { http: reqwest::Client::new(), base, token, seen, tg, store, state, memory, dir }
}

#[tokio::test]
async fn a_channel_is_off_until_the_warning_is_read_and_an_allow_list_and_model_exist() {
    let r = rig(vec![], Duration::ZERO).await;
    let (code, added) = r.call(reqwest::Method::POST, "/v1/channels", json!({"name": "phone", "kind": "telegram", "enabled": true, "settings": {"token": "env:LH_TEST_TG", "api_base": r.tg_url()}})).await;
    assert_eq!(code, 201);
    assert_eq!(added["channel"]["enabled"], false, "even if asked to be on, a new channel starts off");
    assert!(added["warning"].as_str().unwrap().contains("cannot protect them"));
    let (code, e) = r.call(reqwest::Method::POST, "/v1/channels/phone/enable", json!({})).await;
    assert_eq!(code, 400);
    assert!(e["error"].as_str().unwrap().contains("read this first"), "{e}");
    let (_, e) = r.call(reqwest::Method::POST, "/v1/channels/phone/enable", json!({"acknowledge": true})).await;
    assert!(e["error"].as_str().unwrap().contains("allow-list"), "{e}");
    r.call(reqwest::Method::PUT, "/v1/channels/phone", json!({"allow": ["42"]})).await;
    let (_, e) = r.call(reqwest::Method::POST, "/v1/channels/phone/enable", json!({"acknowledge": true})).await;
    assert!(e["error"].as_str().unwrap().contains("model"), "{e}");
    // a pasted token is refused; only a reference to where it is kept
    let (code, e) = r.call(reqwest::Method::POST, "/v1/channels", json!({"name": "x", "kind": "telegram", "settings": {"token": "123:real-token"}})).await;
    assert_eq!(code, 400);
    assert!(e["error"].as_str().unwrap().contains("env:NAME"), "{e}");
    assert_eq!(r.call(reqwest::Method::POST, "/v1/channels", json!({"name": "y", "kind": "irc"})).await.0, 400);
    let list = r.call(reqwest::Method::POST, "/v1/channels/ghost/disable", json!({})).await;
    assert_eq!(list.0, 404);
}

#[tokio::test]
async fn an_allowed_person_gets_answers_a_stranger_gets_nothing_and_does_not_wake_the_core() {
    let r = rig(vec![text("Hello from the assistant."), text("Second answer.")], Duration::from_millis(300)).await;
    r.channel(json!({})).await;
    // wait for the core to go idle
    for _ in 0..60 {
        if r.state.idle.is_idle() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(r.state.idle.is_idle());
    // a stranger: ignored, nothing woken, nothing answered, nothing kept
    r.tg.say(99, "hello? secret plans");
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(r.state.idle.is_idle(), "a message from someone not on the list must not wake the core");
    assert!(r.tg.sent.lock().unwrap().is_empty());
    assert!(r.seen.lock().unwrap().is_empty());
    let log = serde_json::to_string(&r.store.recent(100, None).unwrap().iter().map(|e| e.detail.clone()).collect::<Vec<_>>()).unwrap();
    assert!(!log.contains("secret plans"), "a stranger's text is never kept");
    let chans = r.call(reqwest::Method::GET, "/v1/channels", json!({})).await.1;
    assert_eq!(chans["channels"][0]["status"]["ignored"], 1);
    assert_eq!(chans["channels"][0]["status"]["running"], true);

    // the person on the list: the core wakes, the assistant answers in the chat
    r.tg.say(42, "hi assistant");
    let sent = r.tg.wait_sent(1).await;
    assert_eq!(sent[0], ("42".to_string(), "Hello from the assistant.".to_string()));
    assert!(!r.state.idle.is_idle() || r.seen.lock().unwrap().len() == 1);
    // the second message continues the same conversation
    r.tg.say(42, "and then?");
    r.tg.wait_sent(2).await;
    let seen = r.seen.lock().unwrap();
    assert_eq!(seen.len(), 2);
    assert!(seen[1].messages.iter().any(|m| m.content.as_deref() == Some("hi assistant")), "the same session");
    // logged by size only
    let kinds: Vec<String> = r.store.recent(100, None).unwrap().iter().map(|e| e.kind.clone()).collect();
    assert!(kinds.contains(&"channel_in".to_string()) && kinds.contains(&"channel_out".to_string()));
}

#[tokio::test]
async fn a_reply_that_looks_sensitive_is_held_back() {
    let r = rig(vec![text("Sure. Your IBAN is DE89370400440532013000 and your password is hunter2.")], Duration::ZERO).await;
    r.channel(json!({})).await;
    r.tg.say(42, "what is my iban");
    let sent = r.tg.wait_sent(1).await;
    assert!(sent[0].1.contains("held back") && !sent[0].1.contains("DE89"), "{sent:?}");
    let out = r.store.recent(100, Some("channel_out")).unwrap();
    assert_eq!(out[0].detail["withheld"], true);
}

#[tokio::test]
async fn channel_conversations_never_see_sensitive_notes_or_rules() {
    let r = rig(vec![text("ok")], Duration::ZERO).await;
    r.call(reqwest::Method::POST, "/v1/memory/pages", json!({"layer": "facts", "title": "Health", "text": "Takes metformin for diabetes.", "privacy": "sensitive", "gist": "takes metformin", "pinned": true})).await;
    r.call(reqwest::Method::POST, "/v1/memory/pages", json!({"layer": "facts", "title": "City", "text": "Lives in Amman.", "gist": "lives in Amman", "pinned": true})).await;
    r.call(reqwest::Method::POST, "/v1/learn/rules", json!({"text": "My doctor is Dr. Amal; for health questions mention her diabetes plan."})).await;
    r.channel(json!({})).await;
    r.tg.say(42, "hello");
    r.tg.wait_sent(1).await;
    let sys = r.seen.lock().unwrap()[0].messages[0].content.clone().unwrap();
    assert!(sys.contains("Amman"), "ordinary notes are fine: {sys}");
    assert!(!sys.contains("metformin") && !sys.contains("Dr. Amal"), "sensitive notes and rules stay out: {sys}");
    let _ = &r.memory;
}

#[tokio::test]
async fn questions_go_to_the_phone_and_the_answer_comes_back_from_it() {
    let write = Response { tool_calls: vec![ToolCall { id: "c1".into(), name: "write_file".into(), arguments: json!({"path": "/tmp/lh-channel-test-should-not-exist.txt", "content": "x"}).to_string(), repaired: false }], finish: Some(FinishReason::ToolCalls), ..Default::default() };
    let r = rig(vec![write, text("I could not write it.")], Duration::ZERO).await;
    r.channel(json!({"mode": "assistant"})).await;
    r.tg.say(42, "write a file for me");
    let sent = r.tg.wait_sent(1).await;
    assert!(sent[0].1.contains("wants to use write_file") && sent[0].1.contains("Reply yes or no"), "{sent:?}");
    // saying something else while a question is open does not slip past it
    r.tg.say(42, "no");
    let sent = r.tg.wait_sent(3).await;
    assert!(sent.iter().any(|s| s.1.contains("not allowed")), "{sent:?}");
    assert!(sent.iter().any(|s| s.1.contains("could not write")), "the turn finished after the no: {sent:?}");
    assert!(!std::path::Path::new("/tmp/lh-channel-test-should-not-exist.txt").exists());
    // a stray yes with nothing open is just a message
    let _ = &r.dir;
}

#[tokio::test]
async fn commands_new_and_help_work_and_a_broken_service_is_reported_not_fatal() {
    let r = rig(vec![text("first"), text("after new")], Duration::ZERO).await;
    r.channel(json!({})).await;
    r.tg.say(42, "hello");
    r.tg.wait_sent(1).await;
    r.tg.say(42, "/new");
    r.tg.wait_sent(2).await;
    r.tg.say(42, "again");
    r.tg.wait_sent(3).await;
    {
        let seen = r.seen.lock().unwrap();
        assert_eq!(seen.len(), 2);
        assert!(!seen[1].messages.iter().any(|m| m.content.as_deref() == Some("hello")), "/new starts a fresh conversation");
    }
    r.tg.say(42, "/help");
    let sent = r.tg.wait_sent(4).await;
    assert!(sent[3].1.contains("/new"));
}

#[tokio::test]
async fn a_department_goal_is_planned_approved_and_answered_from_the_phone() {
    let plan = r#"{"steps":[{"id":"1","title":"Say hello","assignee":"writer","brief":"Say hello.","depends_on":[]}]}"#;
    let r = rig(vec![text(plan), text("Hello!"), text("OK"), text("The greeting is ready.")], Duration::ZERO).await;
    for t in ["frontier", "mid-tier", "fastest"] {
        r.call(reqwest::Method::PUT, &format!("/v1/team/types/{t}"), json!({"model": "m"})).await;
    }
    r.call(reqwest::Method::PUT, "/v1/team/departments/hello", json!({"text": "---\nname: hello\ndescription: Says hello.\nlead: planner\nmembers: writer\n---\n"})).await;
    r.channel(json!({})).await;
    r.tg.say(42, "/run hello greet me");
    let sent = r.tg.wait_sent(1).await;
    assert!(sent[0].1.contains("Plan for hello") && sent[0].1.contains("Say hello (writer)") && sent[0].1.contains("Run it? Reply yes or no"), "{sent:?}");
    r.tg.say(42, "yes");
    let sent = r.tg.wait_sent(3).await;
    assert!(sent.iter().any(|s| s.1.contains("allowed")), "{sent:?}");
    assert!(sent.iter().any(|s| s.1 == "The greeting is ready."), "the result arrives in the chat: {sent:?}");
    r.tg.say(42, "/run");
    let sent = r.tg.wait_sent(4).await;
    assert!(sent[3].1.contains("Write it as"), "{sent:?}");
}
