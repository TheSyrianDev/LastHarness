//! Anthropic Messages API (`POST {base}/v1/messages`, SSE streaming). Also spoken by services that offer an
//! Anthropic-compatible address (MiniMax). Same `Response` / `Event` types as the OpenAI-style path.
//!
//! Known limits: thinking blocks are shown but not sent back (our `Message` keeps no signature), so
//! extended thinking together with tools is not supported; images, PDFs and text files are sent as blocks (see attach.rs).

use futures_util::StreamExt;
use serde_json::{json, Value};

use crate::stream::{finish_call, Partial};
use crate::{Error, Event, FinishReason, Message, Provider, Response, Role, SseParser, Usage};
use crate::ChatRequest;

pub(crate) const VERSION: &str = "2023-06-01";
/// `max_tokens` is required by the API; used when the request sets none.
const DEFAULT_MAX_TOKENS: u32 = 8192;
/// Shown when the model list cannot be fetched (official address only). Aliases, not dated ids.
const FALLBACK_MODELS: &[&str] = &["claude-fable-5-1", "claude-opus-5-5", "claude-sonnet-5-5", "claude-haiku-5-5"];

impl Provider {
    /// `base_url` like `https://api.anthropic.com` (a trailing `/v1` is accepted and dropped).
    pub fn anthropic(base_url: impl Into<String>, api_key: Option<String>) -> Self {
        let b: String = base_url.into();
        let b = b.trim_end_matches('/');
        let mut p = Self::new(b.strip_suffix("/v1").unwrap_or(b), api_key);
        p.anthropic = true;
        p
    }

    pub(crate) async fn anthropic_models(&self) -> Result<Vec<String>, Error> {
        match self.get("/v1/models?limit=1000").await {
            Ok(v) => Ok(v["data"].as_array().map(|a| a.iter().filter_map(|m| m["id"].as_str().map(str::to_string)).collect()).unwrap_or_default()),
            // a refused key must still show; any other failure on the official address falls back to a short list
            Err(Error::Http { status: 401 | 403, body }) => Err(Error::Http { status: 401, body }),
            Err(_) if self.base_url.starts_with("https://api.anthropic.com") => Ok(FALLBACK_MODELS.iter().map(|s| s.to_string()).collect()),
            Err(e) => Err(e),
        }
    }

    pub(crate) async fn anthropic_chat(&self, req: &ChatRequest) -> Result<Response, Error> {
        let v = self.post("/v1/messages", &anthropic_body(req, false)).await?.json::<Value>().await?;
        if v["type"] == "error" || v["content"].is_null() {
            return Err(Error::Format(v.to_string()));
        }
        Ok(parse_message(&v))
    }

    pub(crate) async fn anthropic_chat_stream(&self, req: &ChatRequest, mut on_event: impl FnMut(Event)) -> Result<Response, Error> {
        let resp = self.post("/v1/messages", &anthropic_body(req, true)).await?;
        let mut bytes = resp.bytes_stream();
        let (mut sse, mut asm) = (SseParser::new(), AnthropicAssembler::default());
        while let Some(chunk) = bytes.next().await {
            for data in sse.push(&chunk?) {
                for ev in asm.feed(&data) {
                    on_event(ev);
                }
                if let Some(e) = asm.error.take() {
                    return Err(e);
                }
            }
        }
        Ok(asm.finish())
    }
}

/// Our messages as the Messages API wants them: system text on top, tool results as `tool_result`
/// blocks inside user messages, neighbours of the same role merged.
pub(crate) fn anthropic_body(req: &ChatRequest, stream: bool) -> Value {
    let mut system: Vec<&str> = Vec::new();
    let mut msgs: Vec<(&'static str, Vec<Value>)> = Vec::new();
    let mut push = |role: &'static str, block: Value| match msgs.last_mut() {
        Some((r, blocks)) if *r == role => blocks.push(block),
        _ => msgs.push((role, vec![block])),
    };
    for m in &req.messages {
        match m.role {
            Role::System => system.extend(m.content.as_deref().filter(|t| !t.is_empty())),
            Role::User => {
                push("user", json!({"type": "text", "text": text_or_space(m)}));
                for b in crate::attach::anthropic_blocks(&m.attachments) {
                    push("user", b);
                }
            }
            Role::Tool => push("user", json!({"type": "tool_result", "tool_use_id": m.tool_call_id, "content": text_or_space(m)})),
            Role::Assistant => {
                if let Some(t) = m.content.as_deref().filter(|t| !t.trim().is_empty()) {
                    push("assistant", json!({"type": "text", "text": t}));
                }
                for c in &m.tool_calls {
                    let input = c.parsed().ok().filter(Value::is_object).unwrap_or_else(|| json!({}));
                    push("assistant", json!({"type": "tool_use", "id": c.id, "name": c.name, "input": input}));
                }
            }
        }
    }
    // tool_result blocks must come first in a user message
    for (role, blocks) in &mut msgs {
        if *role == "user" {
            blocks.sort_by_key(|b| b["type"] != "tool_result");
        }
    }
    let mut b = json!({
        "model": req.model,
        "max_tokens": req.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
        "messages": msgs.into_iter().map(|(r, c)| json!({"role": r, "content": c})).collect::<Vec<_>>(),
        "stream": stream,
    });
    if !system.is_empty() {
        b["system"] = json!(system.join("\n\n"));
    }
    let sm = &req.sampling;
    if let Some(t) = sm.temperature {
        b["temperature"] = json!(t);
    }
    if let Some(t) = sm.top_p {
        b["top_p"] = json!(t);
    }
    if let Some(t) = sm.top_k {
        b["top_k"] = json!(t);
    }
    for (k, v) in &sm.extra {
        if !matches!(k.as_str(), "model" | "messages" | "stream" | "tools" | "system" | "max_tokens") {
            b[k] = v.clone(); // for example `thinking`; the fields this client owns cannot be overridden
        }
    }
    if !req.tools.is_empty() {
        b["tools"] = req.tools.iter().map(|t| json!({"name": t.name, "description": t.description, "input_schema": t.parameters})).collect();
    }
    b
}

/// The API rejects empty text blocks.
fn text_or_space(m: &Message) -> String {
    m.content.clone().filter(|t| !t.is_empty()).unwrap_or_else(|| " ".into())
}

fn finish_of(s: &str) -> FinishReason {
    match s {
        "end_turn" | "stop_sequence" | "pause_turn" => FinishReason::Stop,
        "tool_use" => FinishReason::ToolCalls,
        "max_tokens" => FinishReason::Length,
        o => FinishReason::Other(o.to_string()),
    }
}

/// `input_tokens` leaves out cached tokens; we count them in the prompt (as the OpenAI side does) and
/// also report the cache reads on their own.
fn add_usage(u: &mut Usage, v: &Value) {
    let n = |k: &str| v[k].as_u64();
    let (input, read, create) = (n("input_tokens"), n("cache_read_input_tokens"), n("cache_creation_input_tokens"));
    if input.is_some() || read.is_some() || create.is_some() {
        u.cached_tokens = read.unwrap_or(0);
        u.prompt_tokens = input.unwrap_or(0) + read.unwrap_or(0) + create.unwrap_or(0);
    }
    if let Some(o) = n("output_tokens") {
        u.completion_tokens = o; // cumulative in message_delta
    }
}

fn error_of(v: &Value) -> Error {
    let status = match v["error"]["type"].as_str().unwrap_or("") {
        "overloaded_error" => 529,
        "rate_limit_error" => 429,
        "api_error" => 500,
        "authentication_error" => 401,
        "permission_error" => 403,
        "not_found_error" => 404,
        _ => 400,
    };
    Error::Http { status, body: v["error"]["message"].as_str().unwrap_or("").chars().take(500).collect() }
}

pub(crate) fn parse_message(v: &Value) -> Response {
    let mut r = Response { finish: v["stop_reason"].as_str().map(finish_of), ..Default::default() };
    for (i, b) in v["content"].as_array().into_iter().flatten().enumerate() {
        match b["type"].as_str() {
            Some("text") => r.text.push_str(b["text"].as_str().unwrap_or("")),
            Some("thinking") => r.reasoning.push_str(b["thinking"].as_str().unwrap_or("")),
            Some("tool_use") => r.tool_calls.push(finish_call(i, Partial { id: b["id"].as_str().unwrap_or("").into(), name: b["name"].as_str().unwrap_or("").into(), args: b["input"].to_string() })),
            _ => {}
        }
    }
    if v["usage"].is_object() {
        let mut u = Usage::default();
        add_usage(&mut u, &v["usage"]);
        r.usage = Some(u);
    }
    r
}

/// Turns streamed Messages API events (the JSON after `data:`) into our events and one `Response`.
#[derive(Default)]
pub(crate) struct AnthropicAssembler {
    calls: std::collections::BTreeMap<usize, Partial>,
    response: Response,
    usage: Usage,
    pub(crate) error: Option<Error>,
}

impl AnthropicAssembler {
    pub(crate) fn feed(&mut self, data: &str) -> Vec<Event> {
        let mut ev = Vec::new();
        let Ok(v) = serde_json::from_str::<Value>(data) else { return ev };
        let idx = v["index"].as_u64().unwrap_or(0) as usize;
        match v["type"].as_str().unwrap_or("") {
            "message_start" => {
                add_usage(&mut self.usage, &v["message"]["usage"]);
                self.response.usage = Some(self.usage);
                ev.push(Event::Usage(self.usage));
            }
            "content_block_start" => {
                let b = &v["content_block"];
                match b["type"].as_str() {
                    Some("tool_use") => {
                        self.calls.insert(idx, Partial { id: b["id"].as_str().unwrap_or("").into(), name: b["name"].as_str().unwrap_or("").into(), args: String::new() });
                    }
                    Some("text") => self.text(b["text"].as_str(), &mut ev),
                    Some("thinking") => self.reasoning(b["thinking"].as_str(), &mut ev),
                    _ => {}
                }
            }
            "content_block_delta" => {
                let d = &v["delta"];
                match d["type"].as_str() {
                    Some("text_delta") => self.text(d["text"].as_str(), &mut ev),
                    Some("thinking_delta") => self.reasoning(d["thinking"].as_str(), &mut ev),
                    Some("input_json_delta") => {
                        if let (Some(p), Some(j)) = (self.calls.get_mut(&idx), d["partial_json"].as_str()) {
                            p.args.push_str(j);
                        }
                    }
                    _ => {} // signature_delta and unknown kinds
                }
            }
            "content_block_stop" => {
                if let Some(p) = self.calls.remove(&idx) {
                    let call = finish_call(idx, p);
                    self.response.tool_calls.push(call.clone());
                    ev.push(Event::ToolCall(call));
                }
            }
            "message_delta" => {
                if let Some(s) = v["delta"]["stop_reason"].as_str() {
                    let fr = finish_of(s);
                    self.response.finish = Some(fr.clone());
                    ev.push(Event::Finish(fr));
                }
                if v["usage"].is_object() {
                    add_usage(&mut self.usage, &v["usage"]);
                    self.response.usage = Some(self.usage);
                    ev.push(Event::Usage(self.usage));
                }
            }
            "error" => self.error = Some(error_of(&v)),
            _ => {} // ping, message_stop, unknown
        }
        ev
    }

    fn text(&mut self, t: Option<&str>, ev: &mut Vec<Event>) {
        if let Some(t) = t.filter(|t| !t.is_empty()) {
            self.response.text.push_str(t);
            ev.push(Event::Text(t.into()));
        }
    }

    fn reasoning(&mut self, t: Option<&str>, ev: &mut Vec<Event>) {
        if let Some(t) = t.filter(|t| !t.is_empty()) {
            self.response.reasoning.push_str(t);
            ev.push(Event::Reasoning(t.into()));
        }
    }

    /// End of stream. Tool calls the server never closed are kept (repaired if cut off).
    pub(crate) fn finish(mut self) -> Response {
        for (idx, p) in std::mem::take(&mut self.calls) {
            self.response.tool_calls.push(finish_call(idx, p));
        }
        self.response
    }
}
