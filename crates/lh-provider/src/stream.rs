use std::collections::BTreeMap;

use serde_json::Value;

use crate::{repair_json, Compat, Event, FinishReason, Response, ToolCall, Usage};

#[derive(Default)]
pub(crate) struct Partial {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) args: String,
}

/// Turns streamed chunks (the JSON after `data:`) into events and, at the end, one `Response`.
/// Tolerant: unknown fields are ignored, unparsable chunks are skipped, tool-call fragments are
/// joined by `index`, and cut-off arguments are repaired.
pub struct Assembler {
    compat: Compat,
    calls: BTreeMap<usize, Partial>,
    response: Response,
}

impl Assembler {
    pub fn new(compat: Compat) -> Self {
        Self { compat, calls: BTreeMap::new(), response: Response::default() }
    }

    /// Feed one `data:` payload. Returns the events it produced.
    pub fn feed(&mut self, data: &str) -> Vec<Event> {
        let mut ev = Vec::new();
        if data.trim() == "[DONE]" {
            self.flush_calls(&mut ev);
            return ev;
        }
        let Ok(v) = serde_json::from_str::<Value>(data) else { return ev };
        let choice = &v["choices"][0];
        let delta = &choice["delta"];

        if let Some(t) = delta["content"].as_str().filter(|t| !t.is_empty()) {
            self.response.text.push_str(t);
            ev.push(Event::Text(t.to_string()));
        }
        if let Some(t) = reasoning_of(delta, &self.compat).filter(|t| !t.is_empty()) {
            self.response.reasoning.push_str(&t);
            ev.push(Event::Reasoning(t));
        }
        if let Some(arr) = delta["tool_calls"].as_array() {
            for tc in arr {
                let idx = tc["index"].as_u64().unwrap_or(0) as usize;
                let p = self.calls.entry(idx).or_default();
                if let Some(id) = tc["id"].as_str().filter(|s| !s.is_empty()) {
                    p.id = id.to_string();
                }
                if let Some(n) = tc["function"]["name"].as_str() {
                    p.name.push_str(n);
                }
                if let Some(a) = tc["function"]["arguments"].as_str() {
                    p.args.push_str(a);
                }
            }
        }
        if let Some(r) = choice["finish_reason"].as_str() {
            self.flush_calls(&mut ev);
            let fr = FinishReason::from_str(r);
            self.response.finish = Some(fr.clone());
            ev.push(Event::Finish(fr));
        }
        if let Some(u) = usage_of(&v) {
            self.response.usage = Some(u);
            ev.push(Event::Usage(u));
        }
        ev
    }

    fn flush_calls(&mut self, ev: &mut Vec<Event>) {
        for (idx, p) in std::mem::take(&mut self.calls) {
            let call = finish_call(idx, p);
            self.response.tool_calls.push(call.clone());
            ev.push(Event::ToolCall(call));
        }
    }

    /// End of stream. Flushes calls the server never closed and returns the whole reply.
    pub fn finish(mut self) -> Response {
        let mut ev = Vec::new();
        self.flush_calls(&mut ev);
        self.response
    }
}

pub(crate) fn finish_call(idx: usize, p: Partial) -> ToolCall {
    let id = if p.id.is_empty() { format!("call_{idx}") } else { p.id };
    let raw = if p.args.trim().is_empty() { "{}".to_string() } else { p.args };
    if serde_json::from_str::<Value>(&raw).is_ok() {
        return ToolCall { id, name: p.name, arguments: raw, repaired: false };
    }
    match repair_json(&raw) {
        Some(fixed) => ToolCall { id, name: p.name, arguments: fixed, repaired: true },
        None => ToolCall { id, name: p.name, arguments: raw, repaired: false },
    }
}

fn reasoning_of(obj: &Value, compat: &Compat) -> Option<String> {
    compat.reasoning_fields.iter().find_map(|f| obj[*f].as_str().map(str::to_string))
}

fn usage_of(v: &Value) -> Option<Usage> {
    let u = v.get("usage").filter(|u| u.is_object())?;
    Some(Usage {
        prompt_tokens: u["prompt_tokens"].as_u64().unwrap_or(0),
        completion_tokens: u["completion_tokens"].as_u64().unwrap_or(0),
        cached_tokens: u["prompt_tokens_details"]["cached_tokens"].as_u64().unwrap_or(0),
    })
}

/// Parse a non-streamed chat completion. Accepts tool-call arguments as a JSON string (OpenAI
/// style) or as an object (some servers).
pub fn parse_response(v: &Value, compat: &Compat) -> Response {
    let choice = &v["choices"][0];
    let msg = &choice["message"];
    let mut r = Response {
        text: msg["content"].as_str().unwrap_or("").to_string(),
        reasoning: reasoning_of(msg, compat).unwrap_or_default(),
        finish: choice["finish_reason"].as_str().map(FinishReason::from_str),
        usage: usage_of(v),
        ..Default::default()
    };
    if let Some(arr) = msg["tool_calls"].as_array() {
        for (i, tc) in arr.iter().enumerate() {
            let args = match &tc["function"]["arguments"] {
                Value::String(s) => s.clone(),
                Value::Null => String::new(),
                other => other.to_string(),
            };
            r.tool_calls.push(finish_call(
                i,
                Partial {
                    id: tc["id"].as_str().unwrap_or("").to_string(),
                    name: tc["function"]["name"].as_str().unwrap_or("").to_string(),
                    args,
                },
            ));
        }
    }
    r
}
