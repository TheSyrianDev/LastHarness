//! Native Ollama API (`/api/*`). Verified against docs/api.md in the Ollama repository:
//! chat streams NDJSON (one JSON object per line, not SSE), tool-call arguments are JSON objects,
//! thinking arrives in `message.thinking`, sampling goes in `options`, and the model's context
//! length sits in `/api/show` under `model_info["<arch>.context_length"]`.
//! LM Studio and llama-server keep using the OpenAI-compatible path.

use futures_util::StreamExt;
use serde_json::{json, Map, Value};
use std::time::Duration;

use crate::{Capabilities, ChatRequest, Error, Event, FinishReason, Message, Provider, Response, Role, ToolCall, Usage};

impl Provider {
    /// A provider that speaks Ollama's native API. `base_url` is the server root, like
    /// `http://localhost:11434` (a trailing `/v1` is dropped).
    pub fn ollama(base_url: impl Into<String>, api_key: Option<String>) -> Self {
        let b: String = base_url.into();
        let b = b.trim_end_matches('/');
        let mut p = Self::new(b.strip_suffix("/v1").unwrap_or(b), api_key);
        p.ollama = true;
        p
    }

    /// Asks `GET /api/version`. Returns the Ollama version when the server there is Ollama, `None`
    /// for anything else (including an unreachable address).
    pub async fn detect_ollama(base_url: &str, api_key: Option<String>) -> Option<String> {
        let p = Self::ollama(base_url, api_key);
        let r = p.auth(p.http().get(format!("{}/api/version", p.base_url)).timeout(Duration::from_secs(5))).send().await.ok()?;
        if !r.status().is_success() {
            return None;
        }
        r.json::<Value>().await.ok()?["version"].as_str().map(str::to_string)
    }

    pub(crate) async fn ollama_models(&self) -> Result<Vec<String>, Error> {
        let v = self.get("/api/tags").await?;
        let list = v["models"].as_array().ok_or_else(|| Error::Format(v.to_string()))?;
        Ok(list.iter().filter_map(|m| m["name"].as_str().or_else(|| m["model"].as_str()).map(str::to_string)).collect())
    }

    pub(crate) async fn ollama_capabilities(&self, model: &str) -> Capabilities {
        let Ok(r) = self.post("/api/show", &json!({ "model": model })).await else { return Capabilities::default() };
        let Ok(v) = r.json::<Value>().await else { return Capabilities::default() };
        let Some(c) = capabilities_from_show(&v) else { return Capabilities::default() };
        self.probed.lock().unwrap().insert(model.to_string(), c.clone());
        c
    }

    pub(crate) async fn ollama_chat(&self, req: &ChatRequest) -> Result<Response, Error> {
        let v = self.post("/api/chat", &ollama_body(req, false)).await?.json::<Value>().await?;
        if let Some(e) = v["error"].as_str() {
            return Err(Error::Http { status: 500, body: e.to_string() });
        }
        let mut asm = OllamaAssembler::default();
        asm.feed(&v);
        Ok(asm.res)
    }

    /// Streams NDJSON. To cancel, drop the future; the connection closes and Ollama stops generating.
    pub(crate) async fn ollama_chat_stream(&self, req: &ChatRequest, mut on_event: impl FnMut(Event)) -> Result<Response, Error> {
        let resp = self.post("/api/chat", &ollama_body(req, true)).await?;
        let mut bytes = resp.bytes_stream();
        let (mut buf, mut asm) = (Vec::<u8>::new(), OllamaAssembler::default());
        while let Some(chunk) = bytes.next().await {
            buf.extend_from_slice(&chunk?);
            while let Some(i) = buf.iter().position(|b| *b == b'\n') {
                let line: Vec<u8> = buf.drain(..=i).collect();
                handle_line(&line, &mut asm, &mut on_event)?;
            }
        }
        handle_line(&buf, &mut asm, &mut on_event)?; // last line without a newline
        Ok(asm.res)
    }
}

fn handle_line(line: &[u8], asm: &mut OllamaAssembler, on_event: &mut impl FnMut(Event)) -> Result<(), Error> {
    let line = String::from_utf8_lossy(line);
    let line = line.trim();
    if line.is_empty() {
        return Ok(());
    }
    let v: Value = serde_json::from_str(line).map_err(|_| Error::Format(line.chars().take(200).collect()))?;
    if let Some(e) = v["error"].as_str() {
        return Err(Error::Http { status: 500, body: e.to_string() });
    }
    for ev in asm.feed(&v) {
        on_event(ev);
    }
    Ok(())
}

#[derive(Default)]
struct OllamaAssembler {
    res: Response,
    n: usize,
}

impl OllamaAssembler {
    fn feed(&mut self, v: &Value) -> Vec<Event> {
        let mut out = vec![];
        let m = &v["message"];
        if let Some(t) = m["thinking"].as_str().filter(|t| !t.is_empty()) {
            self.res.reasoning.push_str(t);
            out.push(Event::Reasoning(t.into()));
        }
        if let Some(t) = m["content"].as_str().filter(|t| !t.is_empty()) {
            self.res.text.push_str(t);
            out.push(Event::Text(t.into()));
        }
        for c in m["tool_calls"].as_array().into_iter().flatten() {
            let f = &c["function"];
            self.n += 1;
            let call = ToolCall {
                id: c["id"].as_str().map(str::to_string).unwrap_or_else(|| format!("call_{}", self.n)),
                name: f["name"].as_str().unwrap_or("").to_string(),
                arguments: match &f["arguments"] {
                    Value::Null => "{}".into(),
                    Value::String(s) => s.clone(), // tolerate a server that sends text
                    o => o.to_string(),
                },
                repaired: false,
            };
            self.res.tool_calls.push(call.clone());
            out.push(Event::ToolCall(call));
        }
        if v["done"].as_bool() == Some(true) {
            let u = Usage { prompt_tokens: v["prompt_eval_count"].as_u64().unwrap_or(0), completion_tokens: v["eval_count"].as_u64().unwrap_or(0), cached_tokens: 0 };
            self.res.usage = Some(u);
            out.push(Event::Usage(u));
            let f = if !self.res.tool_calls.is_empty() {
                FinishReason::ToolCalls
            } else {
                match v["done_reason"].as_str() {
                    Some("length") => FinishReason::Length,
                    Some("stop") | None => FinishReason::Stop,
                    Some(o) => FinishReason::Other(o.into()),
                }
            };
            self.res.finish = Some(f.clone());
            out.push(Event::Finish(f));
        }
        out
    }
}

fn message_json(m: &Message, all: &[Message]) -> Value {
    let mut j = json!({ "role": m.role.as_str(), "content": crate::attach::flat_text(m) });
    let images: Vec<Value> = m.attachments.iter().filter(|a| a.kind == crate::AttachKind::Image).map(|a| json!(crate::attach::b64(&a.data))).collect();
    if !images.is_empty() {
        j["images"] = json!(images);
    }
    if !m.tool_calls.is_empty() {
        j["tool_calls"] = m
            .tool_calls
            .iter()
            .map(|c| json!({ "function": { "name": c.name, "arguments": c.parsed().unwrap_or_else(|_| json!({})) } }))
            .collect();
    }
    if m.role == Role::Tool {
        // Ollama names the tool instead of an id; find it from the call this result answers.
        let name = m.tool_call_id.as_ref().and_then(|id| all.iter().flat_map(|a| &a.tool_calls).find(|c| &c.id == id)).map(|c| c.name.clone());
        if let Some(n) = name {
            j["tool_name"] = json!(n);
        }
    }
    j
}

/// Request body. `sampling.extra` keys `think`, `keep_alive` and `format` go to the top level;
/// any other extra key goes into `options`.
pub(crate) fn ollama_body(req: &ChatRequest, stream: bool) -> Value {
    let mut o = Map::new();
    let sm = &req.sampling;
    if let Some(n) = req.max_tokens {
        o.insert("num_predict".into(), json!(n));
    }
    if let Some(t) = sm.temperature {
        o.insert("temperature".into(), json!(t));
    }
    if let Some(t) = sm.top_p {
        o.insert("top_p".into(), json!(t));
    }
    if let Some(t) = sm.top_k {
        o.insert("top_k".into(), json!(t));
    }
    if let Some(t) = sm.min_p {
        o.insert("min_p".into(), json!(t));
    }
    if let Some(t) = sm.repeat_penalty {
        o.insert("repeat_penalty".into(), json!(t));
    }
    let mut b = json!({
        "model": req.model,
        "messages": req.messages.iter().map(|m| message_json(m, &req.messages)).collect::<Vec<_>>(),
        "stream": stream,
    });
    for (k, v) in &sm.extra {
        match k.as_str() {
            "model" | "messages" | "stream" | "tools" | "options" => {}
            "think" | "keep_alive" | "format" => b[k] = v.clone(),
            _ => {
                o.insert(k.clone(), v.clone()); // num_ctx and the other model options
            }
        }
    }
    if !o.is_empty() {
        b["options"] = Value::Object(o);
    }
    if !req.tools.is_empty() {
        b["tools"] = req
            .tools
            .iter()
            .map(|t| json!({ "type": "function", "function": { "name": t.name, "description": t.description, "parameters": t.parameters } }))
            .collect();
    }
    b
}

fn capabilities_from_show(v: &Value) -> Option<Capabilities> {
    let info = v["model_info"].as_object();
    let caps: Option<Vec<&str>> = v["capabilities"].as_array().map(|a| a.iter().filter_map(Value::as_str).collect());
    if info.is_none() && caps.is_none() {
        return None; // not a show reply
    }
    let has = |name: &str| caps.as_ref().map(|c| c.contains(&name));
    Some(Capabilities {
        context_tokens: info.and_then(|i| i.iter().find(|(k, _)| k.ends_with(".context_length")).and_then(|(_, v)| v.as_u64())),
        tools: has("tools"),
        vision: has("vision"),
        audio: None,
        thinking: has("thinking"),
        reasoning_effort: None,
        parameters: info.and_then(|i| i.get("general.parameter_count")).and_then(Value::as_u64).filter(|n| *n > 0),
        source: "ollama".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Sampling, ToolDef};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    type Seen = std::sync::Arc<std::sync::Mutex<Vec<(String, String)>>>;

    /// Fake Ollama: records (path, body) and answers by path. Body may be several NDJSON lines.
    async fn serve(routes: Vec<(&'static str, u16, String)>) -> (String, Seen) {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", l.local_addr().unwrap());
        let seen: Seen = Default::default();
        let s2 = seen.clone();
        tokio::spawn(async move {
            loop {
                let (mut s, _) = l.accept().await.unwrap();
                let (routes, seen) = (routes.clone(), s2.clone());
                tokio::spawn(async move {
                    let mut data = Vec::new();
                    let mut b = vec![0u8; 8192];
                    loop {
                        let n = s.read(&mut b).await.unwrap_or(0);
                        data.extend_from_slice(&b[..n]);
                        let text = String::from_utf8_lossy(&data).to_string();
                        if let Some((head, body)) = text.split_once("\r\n\r\n") {
                            let len = head.to_lowercase().split("content-length:").nth(1).and_then(|r| r.lines().next()).and_then(|x| x.trim().parse::<usize>().ok()).unwrap_or(0);
                            if body.len() >= len || n == 0 {
                                break;
                            }
                        } else if n == 0 {
                            break;
                        }
                    }
                    let text = String::from_utf8_lossy(&data).to_string();
                    let path = text.split_whitespace().nth(1).unwrap_or("").to_string();
                    let body = text.split_once("\r\n\r\n").map(|x| x.1).unwrap_or("").to_string();
                    seen.lock().unwrap().push((path.clone(), body));
                    let (code, out) = routes.iter().find(|(p, _, _)| *p == path).map_or((404, "{\"error\":\"nope\"}".to_string()), |(_, c, b)| (*c, b.clone()));
                    let _ = s.write_all(format!("HTTP/1.1 {code} X\r\nContent-Type: application/x-ndjson\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{out}", out.len()).as_bytes()).await;
                    let _ = s.shutdown().await;
                });
            }
        });
        (base, seen)
    }

    #[tokio::test]
    async fn detects_ollama_and_lists_models() {
        let (base, _) = serve(vec![("/api/version", 200, r#"{"version":"0.5.1"}"#.into()), ("/api/tags", 200, r#"{"models":[{"name":"llama3:8b","model":"llama3:8b"},{"name":"qwen3:4b"}]}"#.into())]).await;
        assert_eq!(Provider::detect_ollama(&base, None).await.as_deref(), Some("0.5.1"));
        assert_eq!(Provider::ollama(format!("{base}/v1"), None).models().await.unwrap(), vec!["llama3:8b", "qwen3:4b"]);
        let (other, _) = serve(vec![]).await;
        assert_eq!(Provider::detect_ollama(&other, None).await, None);
        assert_eq!(Provider::detect_ollama("http://127.0.0.1:1", None).await, None);
    }

    #[tokio::test]
    async fn probes_show() {
        let show = r#"{"capabilities":["completion","tools","thinking"],"model_info":{"general.parameter_count":8030261248,"llama.context_length":131072}}"#;
        let (base, seen) = serve(vec![("/api/show", 200, show.into())]).await;
        let p = Provider::ollama(&base, None);
        let c = p.capabilities("llama3").await;
        assert_eq!((c.context_tokens, c.tools, c.vision, c.thinking, c.parameters, c.source.as_str()), (Some(131072), Some(true), Some(false), Some(true), Some(8030261248), "ollama"));
        p.capabilities("llama3").await; // remembered
        assert_eq!(seen.lock().unwrap().len(), 1);
        let (none, _) = serve(vec![]).await;
        assert_eq!(Provider::ollama(&none, None).capabilities("m").await, Capabilities::default());
    }

    fn req() -> ChatRequest {
        let mut extra = Map::new();
        extra.insert("think".into(), json!(true));
        extra.insert("num_ctx".into(), json!(8192));
        extra.insert("stream".into(), json!(false));
        ChatRequest {
            model: "m".into(),
            messages: vec![
                Message::user("hi"),
                Message::assistant("", vec![ToolCall { id: "call_1".into(), name: "ls".into(), arguments: r#"{"path":"."}"#.into(), repaired: false }]),
                Message::tool_result("call_1", "a b"),
            ],
            tools: vec![ToolDef { name: "ls".into(), description: "list".into(), parameters: json!({"type":"object"}) }],
            max_tokens: Some(64),
            sampling: Sampling { temperature: Some(0.5), top_p: Some(0.9), top_k: Some(40), min_p: Some(0.05), repeat_penalty: Some(1.1), extra },
        }
    }

    #[test]
    fn body_uses_ollama_shapes() {
        let b = ollama_body(&req(), true);
        assert_eq!(b["stream"], true);
        assert_eq!(b["think"], true);
        let o = &b["options"];
        assert_eq!((o["num_predict"].as_u64(), o["top_k"].as_u64(), o["num_ctx"].as_u64()), (Some(64), Some(40), Some(8192)));
        assert!(o["min_p"].is_number() && o["top_p"].is_number() && o["temperature"].is_number() && o["repeat_penalty"].is_number());
        assert!(b.get("max_tokens").is_none() && b.get("temperature").is_none());
        assert_eq!(b["messages"][1]["tool_calls"][0]["function"]["arguments"], json!({"path":"."}), "arguments are an object");
        assert_eq!(b["messages"][2]["tool_name"], "ls");
        assert_eq!(b["tools"][0]["function"]["name"], "ls");
        let bare = ollama_body(&ChatRequest { model: "m".into(), messages: vec![Message::user("x")], ..Default::default() }, false);
        assert!(bare.get("options").is_none() && bare.get("tools").is_none());
    }

    #[tokio::test]
    async fn streams_ndjson_with_thinking_text_tools_and_usage() {
        let nd = [
            r#"{"message":{"role":"assistant","content":"","thinking":"hm"},"done":false}"#,
            r#"{"message":{"role":"assistant","content":"Hel"},"done":false}"#,
            r#"{"message":{"role":"assistant","content":"lo","tool_calls":[{"function":{"name":"ls","arguments":{"path":"/tmp"}}}]},"done":false}"#,
            r#"{"message":{"role":"assistant","content":""},"done":true,"done_reason":"stop","prompt_eval_count":11,"eval_count":7}"#,
        ]
        .join("\n");
        let (base, seen) = serve(vec![("/api/chat", 200, nd)]).await;
        let mut evs = vec![];
        let r = Provider::ollama(&base, None).chat_stream(&req(), |e| evs.push(e)).await.unwrap();
        assert_eq!((r.text.as_str(), r.reasoning.as_str()), ("Hello", "hm"));
        assert_eq!(r.tool_calls.len(), 1);
        assert_eq!(r.tool_calls[0].parsed().unwrap(), json!({"path":"/tmp"}));
        assert_eq!(r.finish, Some(FinishReason::ToolCalls));
        assert_eq!(r.usage, Some(Usage { prompt_tokens: 11, completion_tokens: 7, cached_tokens: 0 }));
        assert!(matches!(evs.first(), Some(Event::Reasoning(_))) && matches!(evs.last(), Some(Event::Finish(_))));
        let sent: Value = serde_json::from_str(&seen.lock().unwrap()[0].1).unwrap();
        assert_eq!(sent["stream"], true);
    }

    #[tokio::test]
    async fn non_streamed_chat_and_length_finish() {
        let one = r#"{"message":{"role":"assistant","content":"cut"},"done":true,"done_reason":"length","prompt_eval_count":1,"eval_count":2}"#;
        let (base, _) = serve(vec![("/api/chat", 200, one.into())]).await;
        let r = Provider::ollama(&base, None).chat(&req()).await.unwrap();
        assert_eq!((r.text.as_str(), r.finish), ("cut", Some(FinishReason::Length)));
    }

    #[tokio::test]
    async fn errors_are_clear() {
        let (base, _) = serve(vec![("/api/chat", 404, r#"{"error":"model 'zzz' not found"}"#.into())]).await;
        let e = Provider::ollama(&base, None).chat(&req()).await.unwrap_err();
        assert!(matches!(&e, Error::Http { status: 404, body } if body.contains("not found")));
        assert!(e.explain().contains("model"));
        // an error line in the middle of a 200 stream
        let nd = format!("{}\n{}", r#"{"message":{"content":"a"},"done":false}"#, r#"{"error":"out of memory"}"#);
        let (base, _) = serve(vec![("/api/chat", 200, nd)]).await;
        let e = Provider::ollama(&base, None).chat_stream(&req(), |_| {}).await.unwrap_err();
        assert!(matches!(&e, Error::Http { body, .. } if body == "out of memory"));
    }

    #[tokio::test]
    async fn cancel_by_dropping_the_future() {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", l.local_addr().unwrap());
        tokio::spawn(async move {
            let (mut s, _) = l.accept().await.unwrap();
            let mut b = vec![0u8; 8192];
            let _ = s.read(&mut b).await;
            let line = "{\"message\":{\"content\":\"x\"},\"done\":false}\n";
            let _ = s.write_all(format!("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n{:x}\r\n{line}\r\n", line.len()).as_bytes()).await;
            tokio::time::sleep(Duration::from_secs(30)).await; // never finishes
        });
        let p = Provider::ollama(&base, None);
        let got = std::sync::Mutex::new(0);
        let out = tokio::time::timeout(Duration::from_millis(500), p.chat_stream(&req(), |_| *got.lock().unwrap() += 1)).await;
        assert!(out.is_err(), "still streaming, so the timeout cancels it");
        assert_eq!(*got.lock().unwrap(), 1);
    }
}
