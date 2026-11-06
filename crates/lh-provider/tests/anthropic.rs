//! The Anthropic Messages API path against a tiny local fake server. No real API is called.
use std::sync::{Arc, Mutex};
use std::time::Duration;

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

fn req(messages: Vec<Message>) -> ChatRequest {
    ChatRequest { model: "claude-test".into(), messages, ..Default::default() }
}

fn fast() -> Retry {
    Retry { attempts: 3, first_delay: Duration::from_millis(5) }
}

fn start(input: u64) -> (&'static str, Value) {
    ("message_start", json!({"type": "message_start", "message": {"id": "m", "role": "assistant", "content": [], "usage": {"input_tokens": input, "cache_read_input_tokens": 4, "output_tokens": 1}}}))
}

#[tokio::test]
async fn streams_text_with_usage_and_headers() {
    let (url, seen) = fake(vec![sse(&[
        start(10),
        ("content_block_start", json!({"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}})),
        ("ping", json!({"type": "ping"})),
        ("content_block_delta", json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Hel"}})),
        ("content_block_delta", json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "lo"}})),
        ("content_block_stop", json!({"type": "content_block_stop", "index": 0})),
        ("message_delta", json!({"type": "message_delta", "delta": {"stop_reason": "end_turn"}, "usage": {"output_tokens": 7}})),
        ("message_stop", json!({"type": "message_stop"})),
    ])])
    .await;
    let mut events = Vec::new();
    let r = Provider::anthropic(format!("{url}/v1"), Some("sk-ant-x".into()))
        .chat_stream(&req(vec![Message::system("be brief"), Message::user("hi")]), |e| events.push(e))
        .await
        .unwrap();
    assert_eq!(r.text, "Hello");
    assert_eq!(r.finish, Some(FinishReason::Stop));
    let u = r.usage.unwrap();
    assert_eq!((u.prompt_tokens, u.completion_tokens, u.cached_tokens), (14, 7, 4));
    assert_eq!(events.iter().filter(|e| matches!(e, Event::Text(_))).count(), 2);
    let (head, body) = seen.lock().unwrap()[0].clone();
    let head = head.to_lowercase();
    assert!(head.starts_with("post /v1/messages "), "{head}");
    assert!(head.contains("x-api-key: sk-ant-x") && head.contains("anthropic-version: 2023-06-01"), "{head}");
    assert!(!head.contains("authorization"));
    let b: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(b["system"], "be brief");
    assert_eq!(b["max_tokens"], 8192);
    assert_eq!(b["stream"], true);
    assert_eq!(b["messages"], json!([{"role": "user", "content": [{"type": "text", "text": "hi"}]}]));
}

#[tokio::test]
async fn streams_thinking_as_reasoning() {
    let (url, _) = fake(vec![sse(&[
        start(1),
        ("content_block_start", json!({"type": "content_block_start", "index": 0, "content_block": {"type": "thinking", "thinking": ""}})),
        ("content_block_delta", json!({"type": "content_block_delta", "index": 0, "delta": {"type": "thinking_delta", "thinking": "hmm"}})),
        ("content_block_delta", json!({"type": "content_block_delta", "index": 0, "delta": {"type": "signature_delta", "signature": "abc"}})),
        ("content_block_stop", json!({"type": "content_block_stop", "index": 0})),
        ("content_block_start", json!({"type": "content_block_start", "index": 1, "content_block": {"type": "text", "text": ""}})),
        ("content_block_delta", json!({"type": "content_block_delta", "index": 1, "delta": {"type": "text_delta", "text": "ok"}})),
        ("message_delta", json!({"type": "message_delta", "delta": {"stop_reason": "max_tokens"}, "usage": {"output_tokens": 3}})),
    ])])
    .await;
    let mut got = Vec::new();
    let r = Provider::anthropic(url, None).chat_stream(&req(vec![Message::user("x")]), |e| got.push(e)).await.unwrap();
    assert_eq!((r.reasoning.as_str(), r.text.as_str()), ("hmm", "ok"));
    assert_eq!(r.finish, Some(FinishReason::Length));
    assert!(got.iter().any(|e| matches!(e, Event::Reasoning(t) if t == "hmm")));
}

#[tokio::test]
async fn tool_call_round_trip() {
    let (url, seen) = fake(vec![
        sse(&[
            start(5),
            ("content_block_start", json!({"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}})),
            ("content_block_delta", json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Looking."}})),
            ("content_block_stop", json!({"type": "content_block_stop", "index": 0})),
            ("content_block_start", json!({"type": "content_block_start", "index": 1, "content_block": {"type": "tool_use", "id": "toolu_1", "name": "read", "input": {}}})),
            ("content_block_delta", json!({"type": "content_block_delta", "index": 1, "delta": {"type": "input_json_delta", "partial_json": "{\"path\":"}})),
            ("content_block_delta", json!({"type": "content_block_delta", "index": 1, "delta": {"type": "input_json_delta", "partial_json": "\"a.txt\"}"}})),
            ("content_block_stop", json!({"type": "content_block_stop", "index": 1})),
            ("message_delta", json!({"type": "message_delta", "delta": {"stop_reason": "tool_use"}, "usage": {"output_tokens": 9}})),
        ]),
        json_reply("200 OK", r#"{"type":"message","content":[{"type":"text","text":"done"}],"stop_reason":"end_turn","usage":{"input_tokens":3,"output_tokens":1}}"#),
    ])
    .await;
    let p = Provider::anthropic(url, Some("k".into()));
    let tools = vec![ToolDef { name: "read".into(), description: "Read a file".into(), parameters: json!({"type": "object", "properties": {"path": {"type": "string"}}}) }];
    let mut first = req(vec![Message::user("open a.txt")]);
    first.tools = tools.clone();
    let mut calls = 0;
    let r = p.chat_stream(&first, |e| calls += matches!(e, Event::ToolCall(_)) as usize).await.unwrap();
    assert_eq!(calls, 1);
    assert_eq!(r.finish, Some(FinishReason::ToolCalls));
    assert_eq!(r.tool_calls.len(), 1);
    assert_eq!((r.tool_calls[0].id.as_str(), r.tool_calls[0].name.as_str()), ("toolu_1", "read"));
    assert_eq!(r.tool_calls[0].parsed().unwrap(), json!({"path": "a.txt"}));

    let mut second = req(vec![Message::user("open a.txt"), r.into_message(), Message::tool_result("toolu_1", "file text")]);
    second.tools = tools;
    let r2 = p.chat(&second).await.unwrap();
    assert_eq!(r2.text, "done");

    let g = seen.lock().unwrap();
    let b1: Value = serde_json::from_str(&g[0].1).unwrap();
    assert_eq!(b1["tools"][0], json!({"name": "read", "description": "Read a file", "input_schema": {"type": "object", "properties": {"path": {"type": "string"}}}}));
    let b2: Value = serde_json::from_str(&g[1].1).unwrap();
    assert_eq!(b2["stream"], false);
    let m = b2["messages"].as_array().unwrap();
    assert_eq!(m.len(), 3);
    assert_eq!(m[1]["role"], "assistant");
    assert_eq!(m[1]["content"][1], json!({"type": "tool_use", "id": "toolu_1", "name": "read", "input": {"path": "a.txt"}}));
    assert_eq!(m[2], json!({"role": "user", "content": [{"type": "tool_result", "tool_use_id": "toolu_1", "content": "file text"}]}));
}

#[tokio::test]
async fn retries_overload_and_gives_up_on_a_wrong_key() {
    let ok = r#"{"type":"message","content":[{"type":"text","text":"hi"}],"stop_reason":"end_turn","usage":{"input_tokens":1,"output_tokens":1}}"#;
    let (url, seen) = fake(vec![json_reply("529 Overloaded", r#"{"type":"error","error":{"type":"overloaded_error","message":"busy"}}"#), json_reply("200 OK", ok)]).await;
    let r = Provider::anthropic(url, Some("k".into())).with_retry(fast()).chat(&req(vec![Message::user("x")])).await.unwrap();
    assert_eq!(r.text, "hi");
    assert_eq!(seen.lock().unwrap().len(), 2);

    let (url, seen) = fake(vec![json_reply("401 Unauthorized", r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#)]).await;
    let e = Provider::anthropic(url, Some("bad".into())).with_retry(fast()).chat(&req(vec![Message::user("x")])).await.unwrap_err();
    assert!(matches!(e, Error::Http { status: 401, .. }));
    assert!(e.explain().contains("API key"));
    assert_eq!(seen.lock().unwrap().len(), 1);

    let (url, seen) = fake(vec![json_reply("500 Internal Server Error", "{}")]).await;
    let e = Provider::anthropic(url, None).with_retry(fast()).chat(&req(vec![Message::user("x")])).await.unwrap_err();
    assert!(matches!(e, Error::Http { status: 500, .. }));
    assert_eq!(seen.lock().unwrap().len(), 3);
}

#[tokio::test]
async fn in_stream_error_event_is_an_error() {
    let (url, _) = fake(vec![sse(&[start(1), ("error", json!({"type": "error", "error": {"type": "rate_limit_error", "message": "slow down"}}))])]).await;
    let e = Provider::anthropic(url, None).chat_stream(&req(vec![Message::user("x")]), |_| {}).await.unwrap_err();
    assert!(matches!(e, Error::Http { status: 429, .. }), "{e:?}");
}

#[tokio::test]
async fn lists_models_and_rejects_a_bad_key() {
    let (url, seen) = fake(vec![json_reply("200 OK", r#"{"data":[{"id":"claude-a"},{"id":"claude-b"}]}"#)]).await;
    let m = Provider::anthropic(url, Some("k".into())).models().await.unwrap();
    assert_eq!(m, vec!["claude-a", "claude-b"]);
    assert!(seen.lock().unwrap()[0].0.to_lowercase().contains("x-api-key: k"));
    let (url, _) = fake(vec![json_reply("401 Unauthorized", "{}")]).await;
    assert!(Provider::anthropic(url, Some("bad".into())).models().await.is_err());
}
