//! MCP servers through the real local API: approval first, tools through the permission rules, and a
//! project folder's own servers only after the user trusts it.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use lh_core::{auth, router, AppState, McpRuntime, Settings};
use lh_provider::{ChatRequest, Event, FinishReason, Response, SseParser, ToolCall};
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

struct Server {
    base: String,
    token: String,
    http: reqwest::Client,
    seen: Arc<Mutex<Vec<ChatRequest>>>,
    dir: tempfile::TempDir,
}
impl Server {
    fn req(&self, m: reqwest::Method, p: &str) -> reqwest::RequestBuilder {
        self.http.request(m, format!("{}{p}", self.base)).bearer_auth(&self.token)
    }
    async fn call(&self, m: reqwest::Method, p: &str, b: Value) -> (u16, Value) {
        let r = self.req(m, p).json(&b).send().await.unwrap();
        (r.status().as_u16(), r.json().await.unwrap_or(Value::Null))
    }
    async fn get(&self, p: &str) -> Value {
        self.req(reqwest::Method::GET, p).send().await.unwrap().json().await.unwrap()
    }
    async fn session(&self, folder: &std::path::Path) -> Value {
        self.call(reqwest::Method::POST, "/v1/sessions", json!({"model": "m", "project_dir": folder})).await.1
    }
}

fn start(mut replies: Vec<Response>) -> Server {
    replies.reverse();
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(Store::open(&dir.path().join("lh.db")).unwrap());
    let seen: Arc<Mutex<Vec<ChatRequest>>> = Default::default();
    let settings = Settings {
        mcp: Some(McpRuntime::open(dir.path().join("mcp.json"))),
        ask_all: false,
        config: None,
        team: None,
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
    let app = router(AppState::new(Script { replies: Mutex::new(replies), seen: seen.clone() }, settings), token.clone(), port);
    tokio::spawn(async move { axum::serve(tokio::net::TcpListener::from_std(l).unwrap(), app).await.unwrap() });
    Server { base: format!("http://127.0.0.1:{port}"), token, http: reqwest::Client::new(), seen, dir }
}

fn mock() -> Value {
    json!({"kind": "stdio", "command": "python3", "args": ["-I", format!("{}/../lh-mcp/tests/mock_server.py", env!("CARGO_MANIFEST_DIR"))]})
}
fn tool_names(s: &Server, i: usize) -> Vec<String> {
    s.seen.lock().unwrap()[i].tools.iter().map(|t| t.name.clone()).collect()
}
async fn one_turn(s: &Server, id: &str, answer: &str) -> Vec<Value> {
    let mut resp = s.req(reqwest::Method::POST, &format!("/v1/sessions/{id}/turns")).json(&json!({"text": "go"})).send().await.unwrap();
    let (mut p, mut out) = (SseParser::new(), vec![]);
    'o: while let Some(c) = tokio::time::timeout(Duration::from_secs(10), resp.chunk()).await.expect("no event in 10 s").unwrap() {
        for d in p.push(&c) {
            let v: Value = serde_json::from_str(&d).unwrap();
            if v["type"] == "ask" {
                s.call(reqwest::Method::POST, &format!("/v1/sessions/{id}/answers/{}", v["id"].as_str().unwrap()), json!({"answer": answer})).await;
            }
            let end = matches!(v["type"].as_str(), Some("done" | "error" | "refused"));
            out.push(v);
            if end {
                break 'o;
            }
        }
    }
    out
}

#[tokio::test]
async fn a_server_runs_only_after_approval_and_its_tools_ask_every_time() {
    let call = Response { tool_calls: vec![ToolCall { id: "c1".into(), name: "mcp__mock__echo".into(), arguments: json!({"text": "hi"}).to_string(), repaired: false }], finish: Some(FinishReason::ToolCalls), ..Default::default() };
    let done = |t: &str| Response { text: t.into(), finish: Some(FinishReason::Stop), ..Default::default() };
    let s = start(vec![done("first"), call, done("it said hi"), done("x")]);
    let proj = s.dir.path().join("p");
    std::fs::create_dir_all(&proj).unwrap();

    let (code, added) = s.call(reqwest::Method::POST, "/v1/mcp/servers", json!({"name": "mock", "transport": mock()})).await;
    assert_eq!(code, 201);
    assert_eq!(added["approved"], false);
    assert!(added["runs"].as_str().unwrap().starts_with("python3 -I "), "the exact command is shown: {added}");
    assert_eq!(s.call(reqwest::Method::POST, "/v1/mcp/servers", json!({"name": "mock", "transport": mock()})).await.0, 409);
    assert_eq!(s.call(reqwest::Method::POST, "/v1/mcp/servers", json!({"name": "bad name!", "transport": mock()})).await.0, 400);

    let id = s.session(&proj).await["id"].as_str().unwrap().to_string();
    one_turn(&s, &id, "yes").await;
    assert!(!tool_names(&s, 0).iter().any(|n| n.starts_with("mcp__")), "an unapproved server contributes nothing: {:?}", tool_names(&s, 0));

    let (code, ok) = s.call(reqwest::Method::POST, "/v1/mcp/servers/mock/approve", json!({})).await;
    assert_eq!((code, ok["tools"].as_array().unwrap().len()), (200, 2), "{ok}");
    let list = s.get("/v1/mcp/servers").await;
    assert_eq!(list["servers"][0]["tools"][0]["name"], "mcp__mock__echo");

    // a new session has the tools; a call asks, and a "no" is final
    let id = s.session(&proj).await["id"].as_str().unwrap().to_string();
    let events = one_turn(&s, &id, "yes").await;
    assert!(tool_names(&s, 1).contains(&"mcp__mock__echo".to_string()));
    let ask = events.iter().find(|e| e["type"] == "ask").expect("an MCP tool asks");
    assert_eq!(ask["tool"], "mcp__mock__echo");
    assert!(ask["what"].as_str().unwrap().contains("echo on mock"));
    let result = s.seen.lock().unwrap().last().unwrap().messages.iter().rev().find(|m| m.role == lh_provider::Role::Tool).unwrap().content.clone().unwrap();
    assert!(result.contains("echo: hi"), "{result}");
    let log = s.get("/v1/activity?kind=mcp_server").await;
    assert_eq!(log["entries"].as_array().unwrap().len(), 2, "adding and approving are both logged");

    assert_eq!(s.req(reqwest::Method::DELETE, "/v1/mcp/servers/mock").send().await.unwrap().status(), 204);
    assert_eq!(s.get("/v1/mcp/servers").await["servers"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn a_folders_own_servers_load_only_after_the_user_trusts_the_folder() {
    let s = start(vec![
        Response { text: "a".into(), finish: Some(FinishReason::Stop), ..Default::default() },
        Response { text: "b".into(), finish: Some(FinishReason::Stop), ..Default::default() },
        Response { text: "c".into(), finish: Some(FinishReason::Stop), ..Default::default() },
        Response { text: "d".into(), finish: Some(FinishReason::Stop), ..Default::default() },
    ]);
    let proj = s.dir.path().join("repo");
    std::fs::create_dir_all(proj.join(".lastharness")).unwrap();
    std::fs::write(proj.join(".lastharness/mcp.json"), json!({"servers": [{"name": "tools", "transport": mock()}]}).to_string()).unwrap();

    let first = s.session(&proj).await;
    assert_eq!(first["needs_trust"]["mcp"][0]["name"], "tools", "the client is told to ask: {first}");
    assert!(first["needs_trust"]["mcp"][0]["runs"].as_str().unwrap().contains("mock_server.py"));
    one_turn(&s, first["id"].as_str().unwrap(), "no").await;
    assert!(!tool_names(&s, 0).iter().any(|n| n.starts_with("mcp__")), "untrusted: nothing runs");

    let q = format!("/v1/trust?folder={}", proj.display());
    assert_eq!(s.get(&q).await["decision"], Value::Null);
    s.call(reqwest::Method::POST, "/v1/trust", json!({"folder": proj, "decision": "trust"})).await;
    let second = s.session(&proj).await;
    assert_eq!(second["needs_trust"], Value::Null);
    one_turn(&s, second["id"].as_str().unwrap(), "no").await;
    assert!(tool_names(&s, 1).iter().any(|n| n.ends_with("__tools__echo") || n.contains("tools_echo") || n.contains("__echo")), "trusted: {:?}", tool_names(&s, 1));

    // the folder's list changes after trust (a pull adds a server): it is no longer trusted, and the client is asked again
    std::fs::write(proj.join(".lastharness/mcp.json"), json!({"servers": [{"name": "tools", "transport": mock()}, {"name": "extra", "transport": mock()}]}).to_string()).unwrap();
    let changed = s.session(&proj).await;
    assert_eq!(changed["needs_trust"]["mcp"].as_array().unwrap().len(), 2, "{changed}");
    one_turn(&s, changed["id"].as_str().unwrap(), "no").await;
    assert!(!tool_names(&s, 2).iter().any(|n| n.starts_with("mcp__")), "a changed list is not trusted: {:?}", tool_names(&s, 2));
    assert_eq!(s.get(&q).await["decision"], "changed");
    s.call(reqwest::Method::POST, "/v1/trust", json!({"folder": proj, "decision": "trust"})).await;

    // "use my own": the folder's servers are switched off again, and the client is not asked again
    s.call(reqwest::Method::POST, "/v1/trust", json!({"folder": proj, "decision": "own"})).await;
    let third = s.session(&proj).await;
    assert_eq!(third["needs_trust"], Value::Null);
    one_turn(&s, third["id"].as_str().unwrap(), "no").await;
    assert!(!tool_names(&s, 3).iter().any(|n| n.starts_with("mcp__")));
    assert_eq!(s.call(reqwest::Method::POST, "/v1/trust", json!({"folder": proj, "decision": "maybe"})).await.0, 400);

    // every decision can be listed, and revoked even after the folder is gone
    let all = s.get("/v1/trust").await;
    assert_eq!(all["folders"].as_array().unwrap().len(), 1, "{all}");
    assert_eq!(all["folders"][0]["decision"], "own");
    assert_eq!(all["folders"][0]["exists"], true);
    let gone = s.dir.path().join("gone");
    std::fs::create_dir_all(&gone).unwrap();
    s.call(reqwest::Method::POST, "/v1/trust", json!({"folder": gone, "decision": "trust"})).await;
    let gone_real = std::fs::canonicalize(&gone).unwrap();
    std::fs::remove_dir(&gone).unwrap();
    assert_eq!(s.get("/v1/trust").await["folders"].as_array().unwrap().len(), 2);
    assert_eq!(s.call(reqwest::Method::POST, "/v1/trust", json!({"folder": gone_real, "decision": "forget"})).await.0, 200);
    assert_eq!(s.get("/v1/trust").await["folders"].as_array().unwrap().len(), 1);
}
