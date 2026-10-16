//! The chat tree through the real local API: branches, moving, summaries of a branch left, labels, fork and clone,
//! and what the model is sent, the privacy guard, the saved KV cache and private chats.
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

fn start_at(dir: &std::path::Path, replies: Vec<Response>, trust: Trust, summarize: &str) -> Server {
    let store = Arc::new(Store::open(&dir.join("lh.db")).unwrap());
    let mem = Arc::new(Memory::open(store.clone(), dir.join("memory")).unwrap());
    let mut r = replies;
    r.reverse();
    let seen: Arc<Mutex<Vec<ChatRequest>>> = Default::default();
    let model = Script { replies: Mutex::new(r), seen: seen.clone() };
    let spend = lh_spend::SpendMeter::new(store.clone(), lh_spend::Config::default(), "test", true);
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
    let state = AppState::new(model, settings);
    state.set_tree_summarize(summarize);
    let app = router(state, token.clone(), port);
    tokio::spawn(async move { axum::serve(tokio::net::TcpListener::from_std(l).unwrap(), app).await.unwrap() });
    Server { base: format!("http://127.0.0.1:{port}"), token, http: reqwest::Client::new(), seen, mem }
}


async fn tree(s: &Server, id: &str) -> Value {
    s.json(s.get(&format!("/v1/chats/{id}/tree"))).await
}

fn node_with(t: &Value, preview: &str) -> i64 {
    t["nodes"].as_array().unwrap().iter().find(|n| n["preview"].as_str().unwrap() == preview).unwrap_or_else(|| panic!("no node {preview}: {t}"))["id"].as_i64().unwrap()
}

fn texts_sent(s: &Server) -> Vec<String> {
    s.seen.lock().unwrap().last().unwrap().messages.iter().filter_map(|m| m.content.clone()).collect()
}

#[tokio::test]
async fn going_back_and_writing_again_starts_a_branch_and_the_model_reads_only_that_branch() {
    let dir = tempfile::tempdir().unwrap();
    let s = start_at(dir.path(), vec![text("a1"), text("a2"), text("a2b")], Trust::Local, "never");
    let id = s.session(json!({})).await;
    s.turn(&id, "q1").await;
    s.turn(&id, "q2").await;
    let t = tree(&s, &id).await;
    assert_eq!(t["nodes"].as_array().unwrap().len(), 4);
    assert_eq!(t["active"].as_array().unwrap().len(), 4);
    assert_eq!(t["summarize"], "never");

    // choose the second question: its words come back, the chat is at the answer before it, the model's history shrinks
    let q2 = node_with(&t, "q2");
    let m = s.json(s.post(&format!("/v1/chats/{id}/tree/move"), json!({"node": q2}))).await;
    assert_eq!(m["input"], "q2");
    assert_eq!(m["left_messages"], 2);
    assert_eq!(m["summary"], Value::Null, "the setting says never");
    let live = s.json(s.get(&format!("/v1/sessions/{id}"))).await;
    let live: Vec<String> = live["messages"].as_array().unwrap().iter().filter(|x| x["role"] != "system").map(|x| x["content"].as_str().unwrap().to_string()).collect();
    assert_eq!(live, vec!["q1", "a1"], "the loaded session follows the branch");

    // write again: a new branch
    s.turn(&id, "q2, reworded").await;
    let sent = texts_sent(&s);
    assert!(sent.iter().any(|t| t == "q1") && sent.iter().any(|t| t == "a1") && sent.iter().any(|t| t == "q2, reworded"));
    assert!(!sent.iter().any(|t| t == "q2" || t == "a2"), "the other branch is never sent: {sent:?}");
    let t = tree(&s, &id).await;
    assert_eq!(t["nodes"].as_array().unwrap().len(), 6, "nothing was lost");
    assert_eq!(t["active"].as_array().unwrap().len(), 4);
    // the chat list and the history show the active branch
    let chat = s.json(s.get(&format!("/v1/chats/{id}"))).await;
    let said: Vec<&str> = chat["history"].as_array().unwrap().iter().map(|m| m["content"].as_str().unwrap()).collect();
    assert_eq!(said, vec!["q1", "a1", "q2, reworded", "a2b"]);
    assert_eq!(chat["messages"], 4);

    // going back to the old branch's answer continues after it with an empty input
    let a2 = node_with(&t, "a2");
    let back = s.json(s.post(&format!("/v1/chats/{id}/tree/move"), json!({"node": a2}))).await;
    assert_eq!(back["input"], Value::Null);
    assert_eq!(back["active"].as_array().unwrap().len(), 4);
}

#[tokio::test]
async fn leaving_a_branch_asks_first_and_can_leave_a_summary_where_you_come_back() {
    let dir = tempfile::tempdir().unwrap();
    let s = start_at(dir.path(), vec![text("a1"), text("a2"), text("The other path found that the answer is 42."), text("a3")], Trust::Local, "ask");
    let id = s.session(json!({})).await;
    s.turn(&id, "q1").await;
    s.turn(&id, "q2").await;
    let t = tree(&s, &id).await;
    let a1 = node_with(&t, "a1");
    // nothing is decided for the person: the answer is asked for and nothing has moved
    let q = s.json(s.post(&format!("/v1/chats/{id}/tree/move"), json!({"node": a1}))).await;
    assert_eq!(q["needs_choice"], true);
    assert_eq!(q["leaving_messages"], 2);
    assert_eq!(tree(&s, &id).await["leaf"], t["leaf"], "asking changes nothing");
    assert_eq!(s.seen.lock().unwrap().len(), 2, "and the model was not called");
    // yes: the model writes it and it is attached to the branch entered
    let m = s.json(s.post(&format!("/v1/chats/{id}/tree/move"), json!({"node": a1, "summarize": true}))).await;
    assert_eq!(m["summary"], "The other path found that the answer is 42.");
    let ask = s.seen.lock().unwrap().last().unwrap().messages[0].content.clone().unwrap();
    assert!(ask.contains("q2") && ask.contains("a2"), "the summary request holds the branch being left");
    let t = tree(&s, &id).await;
    let nodes = t["nodes"].as_array().unwrap();
    let summary = nodes.iter().find(|n| n["role"] == "summary").unwrap();
    assert_eq!(summary["parent"].as_i64().unwrap(), a1);
    assert_eq!(t["leaf"], summary["id"], "the chat is at the summary, under the branch entered");
    // the model sees it as context on the next turn, and only on this branch
    s.turn(&id, "q3").await;
    let sent = texts_sent(&s);
    assert!(sent.iter().any(|x| x.contains("Summary of another path") && x.contains("42")), "{sent:?}");
    assert!(!sent.iter().any(|x| x == "q2" || x == "a2"));
    // no: nothing is written
    let a3 = node_with(&tree(&s, &id).await, "a3");
    let before = tree(&s, &id).await["nodes"].as_array().unwrap().len();
    let m = s.json(s.post(&format!("/v1/chats/{id}/tree/move"), json!({"node": a1, "summarize": false}))).await;
    assert_eq!(m["summary"], Value::Null);
    assert_eq!(tree(&s, &id).await["nodes"].as_array().unwrap().len(), before);
    let _ = a3;
}

#[tokio::test]
async fn moving_to_where_you_already_are_leaves_nothing_behind_so_nothing_is_asked() {
    let dir = tempfile::tempdir().unwrap();
    let s = start_at(dir.path(), vec![text("a1")], Trust::Local, "ask");
    let id = s.session(json!({})).await;
    s.turn(&id, "q1").await;
    let a1 = node_with(&tree(&s, &id).await, "a1");
    let m = s.json(s.post(&format!("/v1/chats/{id}/tree/move"), json!({"node": a1}))).await;
    assert!(m["needs_choice"].is_null() && m["left_messages"] == 0, "{m}");
}

#[tokio::test]
async fn an_unknown_node_or_chat_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let s = start_at(dir.path(), vec![text("a1")], Trust::Local, "never");
    let id = s.session(json!({})).await;
    s.turn(&id, "q1").await;
    let r = s.post(&format!("/v1/chats/{id}/tree/move"), json!({"node": 99999})).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 404);
    let r = s.post("/v1/chats/s999/tree/move", json!({"node": 1})).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 404, "a chat that does not exist");
}

#[tokio::test]
async fn private_chats_have_no_tree_because_they_store_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let s = start_at(dir.path(), vec![text("a1")], Trust::Local, "never");
    let id = s.session(json!({"incognito": true})).await;
    s.turn(&id, "a secret question").await;
    let r = s.get(&format!("/v1/chats/{id}/tree")).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 404);
    assert!(r.json::<Value>().await.unwrap()["error"].as_str().unwrap().contains("private"));
    assert!(s.mem.messages(&id).unwrap().is_empty(), "and nothing was stored");
    assert_eq!(s.post(&format!("/v1/chats/{id}/fork"), json!({"node": 1})).send().await.unwrap().status().as_u16(), 404);
}

#[tokio::test]
async fn labels_fork_and_clone() {
    let dir = tempfile::tempdir().unwrap();
    let s = start_at(dir.path(), vec![text("a1"), text("a2")], Trust::Local, "never");
    let id = s.session(json!({})).await;
    s.turn(&id, "q1").await;
    s.turn(&id, "q2").await;
    let t = tree(&s, &id).await;
    let a1 = node_with(&t, "a1");
    let r = s.post(&format!("/v1/chats/{id}/tree/label"), json!({"node": a1, "label": "good start"})).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 204);
    assert_eq!(tree(&s, &id).await["nodes"].as_array().unwrap().iter().find(|n| n["id"] == a1).unwrap()["label"], "good start");
    // fork at the second question: a new chat holds what came before, and the words come back
    let f = s.json(s.post(&format!("/v1/chats/{id}/fork"), json!({"node": node_with(&t, "q2")}))).await;
    assert_eq!(f["input"], "q2");
    let fid = f["id"].as_str().unwrap().to_string();
    assert_ne!(fid, id);
    let forked = s.json(s.get(&format!("/v1/chats/{fid}"))).await;
    assert_eq!(forked["messages"], 2);
    assert!(forked["title"].as_str().unwrap().ends_with("(fork)"));
    // the fork can be opened and carried on like any stored chat, and does not touch the original
    let ok = s.json(s.post(&format!("/v1/chats/{fid}/resume"), json!({"model": "m", "confirm": true}))).await;
    assert_eq!(ok["resumed_messages"], 2, "{ok}");
    assert_eq!(s.json(s.get(&format!("/v1/chats/{id}"))).await["messages"], 4);
    // clone copies the active branch
    let c = s.json(s.post(&format!("/v1/chats/{id}/clone"), json!({}))).await;
    assert_eq!(c["messages"], 4);
    assert_eq!(s.json(s.get(&format!("/v1/chats/{}", c["id"].as_str().unwrap()))).await["messages"], 4);
}

#[tokio::test]
async fn a_chat_stored_before_branches_opens_as_one_straight_branch_after_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let id = {
        let s = start_at(dir.path(), vec![text("a1")], Trust::Local, "never");
        let id = s.session(json!({})).await;
        s.turn(&id, "q1").await;
        id
    };
    let s = start_at(dir.path(), vec![text("a2")], Trust::Local, "never");
    let ok = s.json(s.post(&format!("/v1/chats/{id}/resume"), json!({"confirm": true}))).await;
    assert_eq!(ok["resumed_messages"], 2);
    s.turn(&id, "q2").await;
    assert_eq!(tree(&s, &id).await["active"].as_array().unwrap().len(), 4);
}

#[tokio::test]
async fn tool_results_show_in_the_tree_but_are_not_what_the_model_reads_after_moving() {
    let dir = tempfile::tempdir().unwrap();
    let s = start_at(dir.path(), vec![text("a1")], Trust::Local, "never");
    let id = s.session(json!({})).await;
    s.turn(&id, "q1").await;
    // a tool note stored by the core for this chat
    let q1 = node_with(&tree(&s, &id).await, "q1");
    s.mem.set_leaf(&id, q1).unwrap();
    s.mem.append_message(&id, "tool", "show {\"title\":\"x\"} → Opened x").unwrap();
    let a = s.mem.append_message(&id, "assistant", "done").unwrap();
    let t = tree(&s, &id).await;
    assert!(t["nodes"].as_array().unwrap().iter().any(|n| n["role"] == "tool"));
    let m = s.json(s.post(&format!("/v1/chats/{id}/tree/move"), json!({"node": a}))).await;
    assert_eq!(m["active"].as_array().unwrap().len(), 3);
    let live = s.json(s.get(&format!("/v1/sessions/{id}"))).await;
    let roles: Vec<&str> = live["messages"].as_array().unwrap().iter().filter(|x| x["role"] != "system").map(|x| x["role"].as_str().unwrap()).collect();
    assert_eq!(roles, vec!["user", "assistant"], "tool notes are display only");
}

#[tokio::test]
async fn a_summary_of_a_branch_that_repeats_a_sensitive_note_is_never_sent_to_a_cloud_model() {
    let dir = tempfile::tempdir().unwrap();
    let secret = "the vault code is purple elephant seven";
    let id = {
        let s = start_at(dir.path(), vec![text("Noted."), text("Fine.")], Trust::Local, "ask");
        let mut n = NewPage::new(Layer::Facts, "vault", secret);
        n.privacy = Privacy::Sensitive;
        s.mem.create(n, By::User).unwrap();
        let id = s.session(json!({})).await;
        s.turn(&id, "hello").await;
        s.turn(&id, &format!("Remember that {secret}, please.")).await;
        id
    };
    let cloud = start_at(dir.path(), vec![], Trust::AnyCloud, "ask");
    let t = tree(&cloud, &id).await;
    let a1 = node_with(&t, "Noted.");
    let r = cloud.post(&format!("/v1/chats/{id}/tree/move"), json!({"node": a1, "summarize": true})).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 409);
    assert!(r.json::<Value>().await.unwrap()["error"].as_str().unwrap().contains("sensitive"));
    assert!(cloud.seen.lock().unwrap().is_empty(), "nothing reached the model");
    assert_eq!(tree(&cloud, &id).await["leaf"], t["leaf"], "and nothing moved");
}
