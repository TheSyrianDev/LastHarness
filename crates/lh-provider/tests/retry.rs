//! Retry and error behaviour against a tiny local fake server. No real model needed.
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use lh_provider::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Serves the given raw HTTP replies in order, one per connection, then the last one forever.
async fn fake(replies: Vec<String>) -> (String, Arc<AtomicUsize>) {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v1", l.local_addr().unwrap());
    let hits = Arc::new(AtomicUsize::new(0));
    let h = hits.clone();
    tokio::spawn(async move {
        loop {
            let (mut s, _) = l.accept().await.unwrap();
            let n = h.fetch_add(1, Ordering::SeqCst);
            let reply = replies[n.min(replies.len() - 1)].clone();
            tokio::spawn(async move {
                let mut buf = vec![0u8; 65536];
                let _ = s.read(&mut buf).await;
                let _ = s.write_all(reply.as_bytes()).await;
                let _ = s.shutdown().await;
            });
        }
    });
    (url, hits)
}

fn http(status: &str, body: &str) -> String {
    format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
}

fn req() -> ChatRequest {
    ChatRequest { model: "m".into(), messages: vec![Message::user("hi")], ..Default::default() }
}

fn fast() -> Retry {
    Retry { attempts: 3, first_delay: Duration::from_millis(5) }
}

const OK: &str = r#"{"choices":[{"message":{"content":"hello"},"finish_reason":"stop"}]}"#;

#[tokio::test]
async fn retries_a_503_then_succeeds() {
    let (url, hits) = fake(vec![http("503 Service Unavailable", "loading"), http("200 OK", OK)]).await;
    let r = Provider::new(url, None).with_retry(fast()).chat(&req()).await.unwrap();
    assert_eq!(r.text, "hello");
    assert_eq!(hits.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn gives_up_after_the_attempt_cap() {
    let (url, hits) = fake(vec![http("500 Internal Server Error", "boom")]).await;
    let e = Provider::new(url, None).with_retry(fast()).chat(&req()).await.unwrap_err();
    assert!(matches!(e, Error::Http { status: 500, .. }));
    assert_eq!(hits.load(Ordering::SeqCst), 3);
    assert!(e.explain().contains("failed while answering"));
}

#[tokio::test]
async fn does_not_retry_a_wrong_key() {
    let (url, hits) = fake(vec![http("401 Unauthorized", "no")]).await;
    let e = Provider::new(url, Some("bad".into())).with_retry(fast()).chat(&req()).await.unwrap_err();
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    assert!(e.explain().contains("API key"));
}

#[tokio::test]
async fn nothing_listening_is_a_clear_network_error() {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v1", l.local_addr().unwrap());
    drop(l);
    let e = Provider::new(url, None).with_retry(Retry { attempts: 2, first_delay: Duration::from_millis(5) }).models().await.unwrap_err();
    assert!(e.is_retryable());
    assert!(e.explain().starts_with("Cannot reach the server"));
}
