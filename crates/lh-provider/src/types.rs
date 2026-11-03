use serde_json::{json, Value};

#[derive(Clone, Debug, PartialEq)]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::Tool => "tool",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    /// Raw JSON text as the model wrote it (or as repaired, see `repaired`).
    pub arguments: String,
    /// True when the arguments were incomplete and `repair_json` fixed them.
    pub repaired: bool,
}

impl ToolCall {
    pub fn parsed(&self) -> Result<Value, serde_json::Error> {
        serde_json::from_str(&self.arguments)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    pub role: Role,
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub tool_call_id: Option<String>,
}

impl Message {
    pub fn system(text: impl Into<String>) -> Self {
        Self { role: Role::System, content: Some(text.into()), tool_calls: vec![], tool_call_id: None }
    }
    pub fn user(text: impl Into<String>) -> Self {
        Self { role: Role::User, content: Some(text.into()), tool_calls: vec![], tool_call_id: None }
    }
    pub fn assistant(text: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        let t: String = text.into();
        Self { role: Role::Assistant, content: if t.is_empty() { None } else { Some(t) }, tool_calls, tool_call_id: None }
    }
    pub fn tool_result(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self { role: Role::Tool, content: Some(text.into()), tool_calls: vec![], tool_call_id: Some(id.into()) }
    }

    pub(crate) fn to_json(&self) -> Value {
        let mut m = json!({ "role": self.role.as_str(), "content": self.content });
        if !self.tool_calls.is_empty() {
            m["tool_calls"] = self
                .tool_calls
                .iter()
                .map(|c| json!({ "id": c.id, "type": "function", "function": { "name": c.name, "arguments": c.arguments } }))
                .collect();
        }
        if let Some(id) = &self.tool_call_id {
            m["tool_call_id"] = json!(id);
        }
        m
    }
}

#[derive(Clone, Debug)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    /// JSON Schema for the arguments.
    pub parameters: Value,
}

/// Sampling and template settings, set per role. Everything is optional: unset means the
/// server's own default (the user's llama-swap config often sets these per model).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sampling {
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub top_k: Option<u32>,
    pub min_p: Option<f32>,
    pub repeat_penalty: Option<f32>,
    /// Extra top-level fields sent as they are, for template and reasoning controls that differ
    /// between servers: `chat_template_kwargs`, `reasoning_effort`, `thinking_budget_tokens`, ...
    pub extra: serde_json::Map<String, Value>,
}

#[derive(Clone, Debug, Default)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolDef>,
    pub max_tokens: Option<u32>,
    pub sampling: Sampling,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Usage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub cached_tokens: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FinishReason {
    Stop,
    ToolCalls,
    /// Hit the token limit. With thinking models the answer may be empty.
    Length,
    Other(String),
}

impl FinishReason {
    pub(crate) fn from_str(s: &str) -> Self {
        match s {
            "stop" => Self::Stop,
            "tool_calls" | "function_call" => Self::ToolCalls,
            "length" => Self::Length,
            o => Self::Other(o.to_string()),
        }
    }
}

/// What the caller sees while a reply streams in.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Text(String),
    Reasoning(String),
    /// A tool call is complete (sent when the server reports the finish).
    ToolCall(ToolCall),
    Usage(Usage),
    Finish(FinishReason),
}

/// A whole reply, built from a stream or from a non-streamed response.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Response {
    pub text: String,
    pub reasoning: String,
    pub tool_calls: Vec<ToolCall>,
    pub finish: Option<FinishReason>,
    pub usage: Option<Usage>,
}

impl Response {
    /// The assistant message to put back into the history for the next request.
    pub fn into_message(self) -> Message {
        Message::assistant(self.text, self.tool_calls)
    }
}
