//! The sensitive-data rule, walked path by path. One sensitive note and one sensitive rule are set up, and
//! then every way text can leave the machine is tried: the model endpoint (cloud trust), its memory tools,
//! compaction, the learning pass, web fetch and search, MCP tools, image prompts, and a messaging
//! channel. In each, the sensitive text must not get out.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use lh_core::{auth, channels, router, AppState, LearnMode, LearnRuntime, McpRuntime, MemoryRuntime, Settings};
use lh_memory::embed::AnyEmbedder;
use lh_memory::{Memory, RecallConfig, Trust};
use lh_provider::{ChatRequest, Event, FinishReason, Response, SseParser, ToolCall};
use lh_store::Store;
use lh_tools::Model;
use serde_json::{json, Value};

const NOTE: &str = "Takes metformin for type two diabetes every morning with breakfast.";
const RULE: &str = "My doctor is Dr. Amal; for health questions mention her diabetes plan.";
// the pieces a leak would carry
const MARKERS: &[&str] = &["metformin", "Dr. Amal", "type two diabetes", "diabetes plan"];

struct Script {
    replies: Arc<Mutex<Vec<Response>>>,
    seen: Arc<Mutex<Vec<ChatRequest>>>,
}
impl Model for Script {
    async fn complete(&self, r: &ChatRequest, on: &mut (dyn FnMut(Event) + Send)) -> Result<Response, lh_provider::Error> {
        self.seen.lock().unwrap().push(r.clone());
        let resp = self.replies.lock().unwrap().pop().unwrap_or(Response { text: "ok".into(), finish: Some(FinishReason::Stop), ..Default::default() });
        on(Event::Text(resp.text.clone()));
        Ok(resp)
    }
}
fn text(t: &str) -> Response {
    Response { text: t.into(), finish: Some(FinishReason::Stop), ..Default::default() }
}
fn call(name: &str, args: Value) -> Response {
    Response { tool_calls: vec![ToolCall { id: format!("c-{name}"), name: name.into(), arguments: args.to_string(), repaired: false }], finish: Some(FinishReason::ToolCalls), ..Default::default() }
}

struct Rig {
    http: reqwest::Client,
    base: String,
    token: String,
    seen: Arc<Mutex<Vec<ChatRequest>>>,
    replies: Arc<Mutex<Vec<Response>>>,
    note_id: String,
    store: Arc<Store>,
    dir: tempfile::TempDir,
}
impl Rig {
    async fn call(&self, m: reqwest::Method, p: &str, b: Value) -> Value {
        self.http.request(m, format!("{}{p}", self.base)).bearer_auth(&self.token).json(&b).send().await.unwrap().json().await.unwrap_or(Value::Null)
    }
    async fn session(&self, extra: Value) -> String {
        let mut body = json!({"model": "m", "mode": "assistant"});
        for (k, v) in extra.as_object().unwrap() {
            body[k] = v.clone();
        }
        self.call(reqwest::Method::POST, "/v1/sessions", body).await["id"].as_str().unwrap().to_string()
    }
    /// Run a turn, answering every question with `answer`. Returns the events.
    async fn turn(&self, id: &str, msg: &str, answer: &str) -> Vec<Value> {
        let mut resp = self.http.post(format!("{}/v1/sessions/{id}/turns", self.base)).bearer_auth(&self.token).json(&json!({"text": msg})).send().await.unwrap();
        let (mut p, mut out) = (SseParser::new(), vec![]);
        'o: while let Some(c) = tokio::time::timeout(Duration::from_secs(20), resp.chunk()).await.expect("no event in 20 s").unwrap() {
            for d in p.push(&c) {
                let v: Value = serde_json::from_str(&d).unwrap();
                if v["type"] == "ask" {
                    self.call(reqwest::Method::POST, &format!("/v1/sessions/{id}/answers/{}", v["id"].as_str().unwrap()), json!({"answer": answer})).await;
                }
                let end = matches!(v["type"].as_str(), Some("done" | "error" | "refused" | "step_limit"));
                out.push(v);
                if end {
                    break 'o;
                }
            }
        }
        out
    }
    /// Every request the model endpoint has received, as one string: messages, tool results, tool definitions.
    fn everything_sent(&self) -> String {
        self.seen.lock().unwrap().iter().map(|r| format!("{:?} {:?}", r.messages, r.tools.iter().map(|t| (&t.name, &t.description)).collect::<Vec<_>>())).collect::<Vec<_>>().join("\n")
    }
    /// The model's next answers, in order.
    fn script(&self, mut replies: Vec<Response>) {
        replies.reverse();
        *self.replies.lock().unwrap() = replies;
    }
    fn assert_clean(&self, what: &str) {
        let all = self.everything_sent();
        for m in MARKERS {
            assert!(!all.contains(m), "{what}: {m:?} reached the model endpoint:\n{all}");
        }
    }
}

async fn rig(replies: Vec<Response>, trust: Trust, searxng: Option<String>, compact_at: Option<u64>) -> Rig {
    let shared: Arc<Mutex<Vec<Response>>> = Default::default();
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(Store::open(&dir.path().join("lh.db")).unwrap());
    let memory = Arc::new(Memory::open(store.clone(), dir.path().join("memory")).unwrap());
    let learn = Arc::new(lh_learn::Learn::new(store.clone()).with_skills_dir(dir.path().join("skills")));
    let seen: Arc<Mutex<Vec<ChatRequest>>> = Default::default();
    let settings = Settings {
        ask_all: false,
        config: None,
        mcp: Some(McpRuntime::open(dir.path().join("mcp.json"))),
        team: None,
        provider_name: None,
        learn: Some(LearnRuntime { learn, trust, mode: LearnMode::Propose, threshold: 3, skill_min_calls: 6 }),
        maintain_when_idle: false,
        memory: Some(MemoryRuntime { memory, embedder: Arc::new(AnyEmbedder::Off), trust, cfg: RecallConfig::default() }),
        image_model: None,
        web: lh_tools::WebConfig { searxng_url: searxng, allow_private: true, use_proxy: false },
        idle_after: Duration::ZERO,
        compact_at,
        store: store.clone(),
        spend: lh_spend::SpendMeter::new(store.clone(), lh_spend::Config::default(), "test", true),
        bypass: false,
        sudo_enabled: false,
        allow_rules: vec![],
    };
    let state = AppState::new(Script { replies: shared.clone(), seen: seen.clone() }, settings);
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    l.set_nonblocking(true).unwrap();
    let port = l.local_addr().unwrap().port();
    let token = auth::new_token();
    let app = router(state, token.clone(), port);
    tokio::spawn(async move { axum::serve(tokio::net::TcpListener::from_std(l).unwrap(), app).await.unwrap() });
    let base = format!("http://127.0.0.1:{port}");
    let mut r = Rig { http: reqwest::Client::new(), base, token, seen, replies: shared, note_id: String::new(), store, dir };
    // the secrets, and ordinary things beside them
    let made = r.call(reqwest::Method::POST, "/v1/memory/pages", json!({"layer": "facts", "title": "Health", "text": NOTE, "gist": "takes metformin for diabetes", "privacy": "sensitive", "pinned": true})).await;
    r.note_id = made["page"]["id"].as_str().or(made["id"].as_str()).expect("page id").to_string();
    r.call(reqwest::Method::POST, "/v1/memory/pages", json!({"layer": "facts", "title": "City", "text": "Lives in Amman and likes cycling.", "gist": "lives in Amman", "privacy": "normal", "pinned": true})).await;
    r.call(reqwest::Method::POST, "/v1/learn/rules", json!({"text": RULE})).await;
    r.call(reqwest::Method::POST, "/v1/learn/rules", json!({"text": "Always add links after research."})).await;
    r.script(replies);
    r
}

#[tokio::test]
async fn path_1_the_model_endpoint_never_receives_a_sensitive_note_or_rule_in_any_form() {
    // a cloud endpoint, a model that goes looking for it with every memory tool, many turns, and compaction
    let r = rig(vec![], Trust::AnyCloud, None, Some(200)).await;
    r.script(vec![call("recall", json!({"query": "my health medication morning breakfast doctor"})), text("nothing found"), call("open_note", json!({"id": r.note_id})), text("not found")]);
    let id = r.session(json!({})).await;
    r.turn(&id, "What do you know about my health and my doctor?", "yes").await;
    r.turn(&id, &format!("open the note {}", r.note_id), "yes").await;
    for i in 0..6 {
        r.turn(&id, &format!("a longer message to push the conversation toward compaction number {i}: {}", "word ".repeat(80)), "yes").await;
    }
    assert!(r.seen.lock().unwrap().len() >= 6, "several requests were made");
    r.assert_clean("cloud model, memory tools, many turns");
    // the ordinary note and rule did get through, so the check above is not passing for the wrong reason
    let all = r.everything_sent();
    assert!(all.contains("Amman") && all.contains("Always add links after research"), "ordinary notes and rules are used");
    // tool results that came back from recall and open_note held no sensitive text either
    r.assert_clean("including tool results");

    // the learning pass and the memory review are model requests too
    r.call(reqwest::Method::POST, &format!("/v1/sessions/{id}/learn"), json!({})).await;
    r.assert_clean("learning pass");
    r.call(reqwest::Method::POST, &format!("/v1/sessions/{id}/review"), json!({})).await;
    r.assert_clean("memory review");
    let _ = &r.dir;
}

#[tokio::test]
async fn path_2_web_fetch_and_search_refuse_to_carry_a_sensitive_note_out() {
    // even a local model, which may read the note, cannot put it in an address or a search
    let r = rig(
        vec![
            call("web_search", json!({"query": "doctor advice takes metformin for type two diabetes every morning"})),
            text("done"),
            call("web_fetch", json!({"url": "https://example.com/search?q=takes+metformin+for+type+two+diabetes+every+morning"})),
            text("done"),
            call("web_search", json!({"query": "best sourdough starter"})),
            text("done"),
        ],
        Trust::Local,
        Some("http://127.0.0.1:9".into()),
        Some(0),
    )
    .await;
    let id = r.session(json!({})).await;
    for (msg, what) in [("search for my health", "search"), ("fetch something about me", "fetch")] {
        let events = r.turn(&id, msg, "yes").await;
        assert!(!events.iter().any(|e| e["type"] == "ask"), "{what}: the user is not even asked");
        let result = events.iter().find(|e| e["type"] == "tool_result").unwrap()["text"].as_str().unwrap().to_string();
        assert!(result.contains("not sent") && result.contains("sensitive"), "{what}: {result}");
    }
    // an ordinary search is not stopped by the guard (it gets as far as asking the user)
    let events = r.turn(&id, "search for bread", "no").await;
    assert!(events.iter().any(|e| e["type"] == "ask"), "an ordinary query reaches the permission step");
}

#[tokio::test]
async fn path_3_mcp_tools_cannot_be_handed_sensitive_text() {
    let r = rig(vec![call("mcp__mock__echo", json!({"text": "remember: takes metformin for type two diabetes every morning"})), text("done"), call("mcp__mock__echo", json!({"text": "hello there"})), text("done")], Trust::Local, None, Some(0)).await;
    let script = format!("{}/../lh-mcp/tests/mock_server.py", env!("CARGO_MANIFEST_DIR"));
    r.call(reqwest::Method::POST, "/v1/mcp/servers", json!({"name": "mock", "transport": {"kind": "stdio", "command": "python3", "args": ["-I", script]}})).await;
    r.call(reqwest::Method::POST, "/v1/mcp/servers/mock/approve", json!({})).await;
    let id = r.session(json!({})).await;
    let events = r.turn(&id, "tell the tool about me", "yes").await;
    assert!(!events.iter().any(|e| e["type"] == "ask"), "stopped before the user is asked");
    let result = events.iter().find(|e| e["type"] == "tool_result").unwrap()["text"].as_str().unwrap().to_string();
    assert!(result.contains("not sent"), "{result}");
    // harmless arguments go through (after a yes)
    let events = r.turn(&id, "say hello to the tool", "yes").await;
    let result = events.iter().find(|e| e["type"] == "tool_result").unwrap()["text"].as_str().unwrap().to_string();
    assert!(result.contains("echo: hello there"), "{result}");
}

#[tokio::test]
async fn path_4_a_messaging_channel_gets_neither_the_sensitive_context_nor_a_reply_that_repeats_it() {
    // mock Telegram
    #[derive(Clone, Default)]
    struct Tg {
        queue: Arc<Mutex<Vec<Value>>>,
        sent: Arc<Mutex<Vec<String>>>,
    }
    let tg = Tg::default();
    let app = Router::new()
        .route("/botT/getUpdates", get(|State(tg): State<Tg>, Query(_q): Query<std::collections::HashMap<String, String>>| async move {
            let b: Vec<Value> = std::mem::take(&mut *tg.queue.lock().unwrap());
            if b.is_empty() {
                tokio::time::sleep(Duration::from_millis(60)).await;
            }
            Json(json!({"ok": true, "result": b}))
        }))
        .route("/botT/sendMessage", post(|State(tg): State<Tg>, Json(b): Json<Value>| async move {
            tg.sent.lock().unwrap().push(b["text"].as_str().unwrap_or("").to_string());
            Json(json!({"ok": true}))
        }))
        .with_state(tg.clone());
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let tg_url = format!("http://{}", l.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    // the model "helpfully" repeats the note in its reply to the phone
    let r = rig(vec![text("Reminder: you take metformin for type two diabetes every morning with breakfast."), text("Hello! How can I help?")], Trust::Local, None, Some(0)).await;
    std::env::set_var("LH_SENS_TG", "T");
    channels::spawn_runner_every(r.store.clone(), r.base.clone(), r.token.clone(), Duration::from_millis(50));
    r.call(reqwest::Method::POST, "/v1/channels", json!({"name": "phone", "kind": "telegram", "settings": {"token": "env:LH_SENS_TG", "api_base": tg_url}, "allow": ["42"], "model": "m"})).await;
    r.call(reqwest::Method::POST, "/v1/channels/phone/enable", json!({"acknowledge": true})).await;
    let say = |t: &str| tg.queue.lock().unwrap().push(json!({"update_id": rand_id(), "message": {"text": t, "from": {"id": 42, "is_bot": false}, "chat": {"id": 42}}}));
    say("remind me about my health");
    for _ in 0..200 {
        if !tg.sent.lock().unwrap().is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let sent = tg.sent.lock().unwrap().clone();
    assert!(sent[0].contains("held back") && !sent[0].contains("metformin"), "{sent:?}");
    // and the model that wrote it never had the note in its context in the first place (channel trust is the lowest)
    r.assert_clean("channel conversation");
}

fn rand_id() -> i64 {
    use std::sync::atomic::{AtomicI64, Ordering};
    static N: AtomicI64 = AtomicI64::new(1000);
    N.fetch_add(1, Ordering::SeqCst)
}

#[tokio::test]
async fn path_5_the_activity_log_and_exports_to_other_machines_keep_the_marking() {
    // a sensitive note exported and imported stays sensitive, and a sensitive rule does too
    let r = rig(vec![], Trust::Local, None, Some(0)).await;
    let out = r.dir.path().join("export");
    r.call(reqwest::Method::POST, "/v1/export", json!({"dir": out})).await;
    let rules: Vec<Value> = serde_json::from_str(&std::fs::read_to_string(out.join("rules.json")).unwrap()).unwrap();
    let doctor = rules.iter().find(|x| x["text"].as_str().unwrap().contains("Dr. Amal")).unwrap();
    assert_eq!(doctor["privacy"], "sensitive");
    let pages: String = std::fs::read_dir(out.join("memory/facts")).unwrap().flatten().map(|e| std::fs::read_to_string(e.path()).unwrap()).collect();
    assert!(pages.contains("privacy: sensitive"), "the marking travels with the note: {pages}");
}
