//! The MCP client against a mock stdio server and mock HTTP servers.
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use lh_mcp::*;
use lh_tools::ToolError;
use serde_json::{json, Value};

fn mock_path() -> String {
    format!("{}/tests/mock_server.py", env!("CARGO_MANIFEST_DIR"))
}

#[tokio::test]
async fn a_local_server_is_started_listed_and_called() {
    let t = Transport::Stdio { command: "python3".into(), args: vec!["-I".into(), mock_path()], env: Default::default() };
    assert!(t.display().starts_with("python3 -I ") && t.display().ends_with("mock_server.py"), "the exact command is shown for approval");
    let c = Connection::connect("mock", t).await.unwrap();
    assert_eq!(c.tools.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(), ["echo", "boom"], "both pages of the tool list");
    assert_eq!(c.call("echo", json!({"text": "hi"})).await.unwrap(), "echo: hi\n[image content not shown]");
    let e = c.call("boom", json!({})).await.unwrap_err();
    assert!(e.to_string().contains("it broke"), "{e}");
    assert!(c.call("nope", json!({})).await.unwrap_err().to_string().contains("unknown tool"));

    let tools = lh_mcp::tools(&c);
    let echo = tools.iter().find(|t| t.def().name == "mcp__mock__echo").expect("qualified name");
    assert_eq!(echo.risk(), lh_tools::Risk::Asks, "every call asks");
    assert!(echo.describe(&json!({"text": "hi"})).contains("echo on mock"));
    assert_eq!(echo.run(json!({"text": "again"})).await.unwrap(), "echo: again\n[image content not shown]");
    // concurrent calls do not mix up their answers
    let (a, b) = tokio::join!(c.call("echo", json!({"text": "A"})), c.call("echo", json!({"text": "B"})));
    assert!(a.unwrap().starts_with("echo: A") && b.unwrap().starts_with("echo: B"));
}

#[tokio::test]
async fn a_program_that_cannot_start_or_does_not_speak_mcp_gives_a_clear_error() {
    let e = Connection::connect("x", Transport::Stdio { command: "/no/such/program".into(), args: vec![], env: Default::default() }).await.unwrap_err();
    assert!(e.to_string().contains("cannot start `/no/such/program`"), "{e}");
    let e = Connection::connect("x", Transport::Stdio { command: "true".into(), args: vec![], env: Default::default() }).await.unwrap_err();
    // `true` exits at once: either it is seen to end before answering, or our first write finds the pipe already closed
    assert!(e.to_string().contains("ended before it answered") || e.to_string().contains("the server is gone"), "{e}");
}

/// Method, authorization header, session header of each request.
type Request = (String, Option<String>, Option<String>);

#[derive(Clone, Default)]
struct Seen(Arc<Mutex<Vec<Request>>>);

async fn mcp(State(seen): State<Seen>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    let hv = |k: &str| h.get(k).and_then(|v| v.to_str().ok()).map(str::to_string);
    seen.0.lock().unwrap().push((b["method"].as_str().unwrap_or("").to_string(), hv("authorization"), hv("mcp-session-id")));
    let id = b["id"].clone();
    let ok = |r: Value| json!({"jsonrpc": "2.0", "id": id, "result": r});
    match b["method"].as_str() {
        Some("initialize") => ([("Mcp-Session-Id", "sess-1")], Json(ok(json!({"protocolVersion": "2025-06-18", "capabilities": {}})))).into_response(),
        Some("notifications/initialized") => StatusCode::ACCEPTED.into_response(),
        Some("tools/list") => Json(ok(json!({"tools": [{"name": "gen", "description": "Make an image.", "inputSchema": {"type": "object"}}]}))).into_response(),
        Some("tools/call") => {
            // answered as an event stream, with an unrelated message first
            let first = json!({"jsonrpc": "2.0", "method": "notifications/progress", "params": {}});
            let body = format!("event: message\ndata: {first}\n\nevent: message\ndata: {}\n\n", ok(json!({"content": [{"type": "text", "text": format!("made: {}", b["params"]["arguments"]["prompt"].as_str().unwrap_or(""))}]})));
            ([("content-type", "text/event-stream")], body).into_response()
        }
        _ => StatusCode::BAD_REQUEST.into_response(),
    }
}

async fn serve(app: Router) -> String {
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/mcp", l.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    url
}

#[tokio::test]
async fn a_remote_server_keeps_its_session_reads_event_streams_and_never_gets_sensitive_data() {
    let seen = Seen::default();
    let url = serve(Router::new().route("/mcp", post(mcp)).with_state(seen.clone())).await;
    std::env::set_var("LH_TEST_MCP_KEY", "secret-token");
    let c = Connection::connect("higgs", Transport::Http { url, bearer_env: Some("LH_TEST_MCP_KEY".into()) }).await.unwrap();
    assert_eq!(c.tools.len(), 1);
    assert_eq!(c.call("gen", json!({"prompt": "a red fox"})).await.unwrap(), "made: a red fox");
    let log = seen.0.lock().unwrap().clone();
    assert!(log.iter().all(|(_, auth, _)| auth.as_deref() == Some("Bearer secret-token")), "the key comes from the environment variable named in the config");
    assert_eq!(log[0].2, None, "no session before the first reply");
    assert!(log[1..].iter().all(|(_, _, s)| s.as_deref() == Some("sess-1")), "the session id is sent back: {log:?}");

    let tool = &lh_mcp::tools(&c)[0];
    let blocked = tool.run(json!({"prompt": "my password is hunter2 and my IBAN is DE89370400440532013000"})).await;
    let Err(ToolError::Failed(why)) = blocked else { panic!("sensitive text must not be sent: {blocked:?}") };
    assert!(why.contains("sensitive") && why.contains("remote"), "{why}");
    assert!(seen.0.lock().unwrap().iter().filter(|(m, ..)| m == "tools/call").count() == 1, "the blocked call never left");
}

#[tokio::test]
async fn a_key_error_is_reported_as_one() {
    let url = serve(Router::new().route("/mcp", post(|| async { StatusCode::UNAUTHORIZED }))).await;
    let e = Connection::connect("x", Transport::Http { url, bearer_env: None }).await.unwrap_err();
    assert!(e.to_string().contains("may need a key"), "{e}");
}

#[test]
fn names_are_safe_for_models() {
    assert_eq!(qualified_name("my server", "do.it"), "mcp__my_server__do_it");
    assert!(qualified_name("s", &"x".repeat(100)).len() <= 64);
}

#[tokio::test]
async fn a_local_server_gets_only_the_variables_it_was_given() {
    std::env::set_var("LH_TEST_CORE_SECRET", "core-secret-value");
    std::env::set_var("LH_TEST_SERVER_KEY", "its-own-key");
    let d = tempfile::tempdir().unwrap();
    let script = d.path().join("env_server.py");
    std::fs::write(&script, r#"
import json, os, sys
for line in sys.stdin:
    m = json.loads(line); i = m.get("id"); meth = m.get("method")
    if meth == "initialize": print(json.dumps({"jsonrpc": "2.0", "id": i, "result": {"protocolVersion": "2025-06-18", "capabilities": {}}}), flush=True)
    elif meth == "tools/list": print(json.dumps({"jsonrpc": "2.0", "id": i, "result": {"tools": [{"name": "env", "inputSchema": {"type": "object"}}]}}), flush=True)
    elif meth == "tools/call": print(json.dumps({"jsonrpc": "2.0", "id": i, "result": {"content": [{"type": "text", "text": json.dumps(dict(os.environ))}]}}), flush=True)
"#).unwrap();
    let mut env = std::collections::BTreeMap::new();
    env.insert("SERVER_KEY".to_string(), "env:LH_TEST_SERVER_KEY".to_string());
    env.insert("PLAIN".to_string(), "value".to_string());
    let c = Connection::connect("e", Transport::Stdio { command: "python3".into(), args: vec!["-I".into(), script.display().to_string()], env }).await.unwrap();
    let seen = c.call("env", json!({})).await.unwrap();
    assert!(!seen.contains("core-secret-value") && !seen.contains("LH_TEST_CORE_SECRET"), "{seen}");
    assert!(seen.contains("its-own-key") && seen.contains("\"PLAIN\": \"value\""), "{seen}");
    let mut missing = std::collections::BTreeMap::new();
    missing.insert("K".to_string(), "env:LH_TEST_NOT_SET_ANYWHERE".to_string());
    let e = Connection::connect("e", Transport::Stdio { command: "python3".into(), args: vec![], env: missing }).await.unwrap_err().to_string();
    assert!(e.contains("is not set"), "{e}");
}
