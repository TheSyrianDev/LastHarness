//! Stop must reach the model server: when the front end drops its stream, the core drops the request it made to the
//! model, so the server sees the connection close and can stop generating. Checked against a fake model server that
//! thinks for ever and notices when its connection is dropped.
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::State;
use axum::response::Response;
use axum::routing::post;
use axum::Router;
use axum::body::Bytes;
use lh_core::{auth, router, AppState, Settings};
use lh_provider::{Provider, SseParser};
use serde_json::{json, Value};

#[derive(Clone, Default)]
struct Fake {
    started: Arc<AtomicUsize>,
    sent: Arc<AtomicUsize>,
    dropped: Arc<AtomicBool>,
}

/// Held by the response body: when the body is dropped (the client closed the connection) it says so.
struct Notice(Arc<AtomicBool>);
impl Drop for Notice {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

async fn completions(State(f): State<Fake>) -> Response {
    f.started.fetch_add(1, Ordering::SeqCst);
    let (sent, notice) = (f.sent.clone(), Notice(f.dropped.clone()));
    // a delta every 20 ms, thinking and never answering, like a small model that over-thinks
    let body = futures_util::stream::unfold((0u64, sent, notice), |(n, sent, notice)| async move {
        tokio::time::sleep(Duration::from_millis(20)).await;
        sent.fetch_add(1, Ordering::SeqCst);
        let line = format!("data: {}\n\n", json!({"choices": [{"index": 0, "delta": {"reasoning_content": "hmm "}}]}));
        Some((Ok::<Bytes, std::io::Error>(Bytes::from(line)), (n + 1, sent, notice)))
    });
    Response::builder().header("content-type", "text/event-stream").body(Body::from_stream(body)).unwrap()
}

async fn fake_model() -> (String, Fake) {
    let f = Fake::default();
    let app = Router::new().route("/v1/chat/completions", post(completions)).with_state(f.clone());
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    (format!("http://127.0.0.1:{port}/v1"), f)
}

async fn core(provider: &str) -> (String, String, reqwest::Client) {
    let store = Arc::new(lh_store::Store::open_memory().unwrap());
    let spend = lh_spend::SpendMeter::new(store.clone(), lh_spend::Config::default(), "test", true);
    let settings = Settings { provider_name: None, learn: None, maintain_when_idle: false, memory: None, image_model: None, web: Default::default(), idle_after: Duration::ZERO, compact_at: Some(0), store, spend, bypass: false, sudo_enabled: false, allow_rules: vec![], team: None, mcp: None, ask_all: false, config: None };
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    let token = auth::new_token();
    let app = router(AppState::new(Provider::new(provider, None), settings), token.clone(), port);
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    (format!("http://127.0.0.1:{port}"), token, reqwest::Client::new())
}

async fn wait_for(what: &str, secs: u64, mut ok: impl FnMut() -> bool) -> Duration {
    let t = Instant::now();
    while !ok() {
        assert!(t.elapsed() < Duration::from_secs(secs), "{what}: not within {secs} s");
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    t.elapsed()
}

#[tokio::test]
async fn dropping_the_front_ends_stream_closes_the_connection_to_the_model_server() {
    let (provider, fake) = fake_model().await;
    let (base, token, http) = core(&provider).await;
    let dir = tempfile::tempdir().unwrap();
    let v: Value = http.post(format!("{base}/v1/sessions")).bearer_auth(&token).json(&json!({"model": "m", "project_dir": dir.path()})).send().await.unwrap().json().await.unwrap();
    let id = v["id"].as_str().unwrap().to_string();

    let mut resp = http.post(format!("{base}/v1/sessions/{id}/turns")).bearer_auth(&token).json(&json!({"text": "write a whole site"})).send().await.unwrap();
    let mut p = SseParser::new();
    let mut reasoning = 0;
    // read thinking events for a moment, as the window does
    while reasoning < 15 {
        let chunk = tokio::time::timeout(Duration::from_secs(10), resp.chunk()).await.expect("no event").unwrap().expect("stream ended");
        for d in p.push(&chunk) {
            let e: Value = serde_json::from_str(&d).unwrap();
            if e["type"] == "reasoning" {
                reasoning += 1;
            }
        }
    }
    assert_eq!(fake.started.load(Ordering::SeqCst), 1);
    assert!(!fake.dropped.load(Ordering::SeqCst), "still thinking, the connection is open");

    // Stop: the window drops its stream and says cancel
    drop(resp);
    let r = http.post(format!("{base}/v1/sessions/{id}/cancel")).bearer_auth(&token).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 204);
    let took = wait_for("the model server saw the connection close", 5, || fake.dropped.load(Ordering::SeqCst)).await;
    assert!(took < Duration::from_secs(3), "it closed after {took:?}");

    // and the model server is no longer being written to
    let after = fake.sent.load(Ordering::SeqCst);
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(fake.sent.load(Ordering::SeqCst) <= after + 2, "the fake server's stream keeps being polled after the close");

    // the session is free again: a new turn is accepted (not "a turn is already running")
    let again = http.post(format!("{base}/v1/sessions/{id}/turns")).bearer_auth(&token).json(&json!({"text": "again"})).send().await.unwrap();
    assert_eq!(again.status().as_u16(), 200, "the session was left busy after Stop");
}
