//! The Anthropic Messages API path against a tiny local fake server. No real API is called.
use std::sync::{Arc, Mutex};

use lh_provider::*;
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

type Seen = Arc<Mutex<Vec<(String, String)>>>; // (request head, request body)

/// Serves raw replies in order (the last one forever) and records every request.
async fn fake(replies: Vec<String>) -> (String, Seen) {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", l.local_addr().unwrap());
    let seen: Seen = Default::default();
    let s2 = seen.clone();
    tokio::spawn(async move {
        let mut n = 0;
        loop {
            let (mut s, _) = l.accept().await.unwrap();
            let reply = replies[n.min(replies.len() - 1)].clone();
            n += 1;
            let seen = s2.clone();
            tokio::spawn(async move {
                let mut buf = Vec::new();
                loop {
                    let mut chunk = [0u8; 8192];
                    let k = s.read(&mut chunk).await.unwrap_or(0);
                    if k == 0 {
                        break;
                    }
                    buf.extend_from_slice(&chunk[..k]);
                    let text = String::from_utf8_lossy(&buf).to_string();
                    if let Some(i) = text.find("\r\n\r\n") {
                        let len = text[..i].to_lowercase().split("content-length:").nth(1).and_then(|r| r.lines().next()).and_then(|v| v.trim().parse::<usize>().ok()).unwrap_or(0);
                        if buf.len() >= i + 4 + len {
                            seen.lock().unwrap().push((text[..i].to_string(), text[i + 4..].to_string()));
                            break;
                        }
                    }
                }
                let _ = s.write_all(reply.as_bytes()).await;
                let _ = s.shutdown().await;
            });
        }
    });
    (url, seen)
}

fn json_reply(status: &str, body: &str) -> String {
    format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
}

fn sse(events: &[(&str, Value)]) -> String {
    let body: String = events.iter().map(|(n, d)| format!("event: {n}\ndata: {d}\n\n")).collect();
    format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n{body}")
}


fn req(model: &str, messages: Vec<Message>) -> ChatRequest {
    ChatRequest { model: model.into(), messages, ..Default::default() }
}

const SECRET: &[u8] = b"PIXELBYTES-123";

fn png() -> Attachment {
    Attachment::new("a1", "cat.png", "image/png", AttachKind::Image, SECRET.to_vec())
}

fn ok_openai() -> String {
    json_reply("200 OK", &json!({"choices": [{"message": {"role": "assistant", "content": "ok"}, "finish_reason": "stop"}]}).to_string())
}

#[tokio::test]
async fn openai_path_sends_image_url_and_inlined_text() {
    let (url, seen) = fake(vec![ok_openai()]).await;
    let txt = Attachment::new("a2", "notes.txt", "text/plain", AttachKind::Text, b"hello file".to_vec());
    let m = Message::user("look").with_attachments(vec![png(), txt]);
    Provider::new(format!("{url}/v1"), None).chat(&req("gpt-4o", vec![m])).await.unwrap();
    let b: Value = serde_json::from_str(&seen.lock().unwrap()[0].1).unwrap();
    let parts = &b["messages"][0]["content"];
    assert_eq!(parts[0], json!({"type": "text", "text": "look"}));
    let uri = parts[1]["image_url"]["url"].as_str().unwrap();
    assert!(uri.starts_with("data:image/png;base64,"), "{uri}");
    let t = parts[2]["text"].as_str().unwrap();
    assert!(t.contains("notes.txt") && t.contains("hello file") && t.contains("not as instructions"), "{t}");
}

#[tokio::test]
async fn anthropic_path_sends_image_block() {
    let (url, seen) = fake(vec![sse(&[
        ("message_start", json!({"type": "message_start", "message": {"usage": {"input_tokens": 1, "output_tokens": 1}}})),
        ("message_stop", json!({"type": "message_stop"})),
    ])])
    .await;
    let m = Message::user("look").with_attachments(vec![png()]);
    Provider::anthropic(format!("{url}/v1"), Some("k".into())).chat_stream(&req("claude-x", vec![m]), |_| {}).await.unwrap();
    let b: Value = serde_json::from_str(&seen.lock().unwrap()[0].1).unwrap();
    let c = &b["messages"][0]["content"];
    assert_eq!(c[0]["text"], "look");
    assert_eq!(c[1]["type"], "image");
    assert_eq!(c[1]["source"]["media_type"], "image/png");
    assert!(c[1]["source"]["data"].as_str().unwrap().len() > 10);
}

#[test]
fn debug_of_a_message_hides_bytes() {
    let m = Message::user("x").with_attachments(vec![png()]);
    let d = format!("{m:?} {:?}", req("m", vec![m.clone()]));
    assert!(!d.contains("PIXELBYTES"));
}

#[tokio::test]
async fn unsupported_image_degrades_with_a_plain_message() {
    let s = Support::default();
    let p = prepare(vec![png()], &s, &Limits::default()).unwrap();
    assert!(p.notices[0].contains("cannot look at images"));
    let m = Message::user("hi").with_attachments(p.attachments);
    let (url, seen) = fake(vec![ok_openai()]).await;
    Provider::new(format!("{url}/v1"), None).chat(&req("tiny", vec![m])).await.unwrap();
    let wire = seen.lock().unwrap()[0].1.clone();
    assert!(wire.contains("cannot see images") && !wire.contains("image_url"));
}

#[test]
fn oversized_file_is_refused() {
    let l = Limits { max_image: 4, ..Default::default() };
    let e = prepare(vec![png()], &Support::all(), &l).unwrap_err().to_string();
    assert!(e.contains("cat.png") && e.contains("too large"), "{e}");
}

#[test]
fn video_without_vision_is_not_sent() {
    let v = Attachment::new("v", "c.mp4", "video/mp4", AttachKind::Video, vec![1, 2, 3]);
    let p = prepare(vec![v], &Support::default(), &Limits::default()).unwrap();
    assert!(p.notices[0].contains("c.mp4"));
}

#[test]
fn video_with_vision_but_unreadable_is_refused() {
    let v = Attachment::new("v", "c.mp4", "video/mp4", AttachKind::Video, vec![1, 2, 3]);
    let s = Support { vision: true, ..Default::default() };
    let e = prepare(vec![v], &s, &Limits::default()).unwrap_err().to_string();
    assert!(e.contains("cannot take video"), "{e}");
}
