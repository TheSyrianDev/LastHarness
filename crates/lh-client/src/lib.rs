//! The client every front end uses to talk to the core (`lhd`): plain HTTP and server-sent events on the local API.

pub mod qr;
pub mod tree;

use std::collections::VecDeque;
use std::path::Path;

use lh_provider::SseParser;
use serde_json::{json, Value};

/// One thing the core says during a turn.
#[derive(Clone, Debug, PartialEq)]
pub enum ServerEvent {
    Reasoning(String),
    Text(String),
    ToolStart { id: String, name: String, arguments: String },
    ToolResult { id: String, text: String },
    /// `detail`: more to look at before answering, such as the diff of a file write; often empty.
    Ask { id: String, tool: String, what: String, reason: String, worker: String, detail: String },
    Usage { prompt_tokens: u64, completion_tokens: u64, cached_tokens: u64 },
    Finish(String),
    Done { empty: bool },
    StepLimit,
    /// The spending rules stopped the turn.
    Refused(String),
    /// Older turns were replaced by a summary.
    Compacted { messages_summarized: u64 },
    Notice(String),
    Error(String),
    /// The assistant opened something beside the chat: a title, how to show it, and the text (or, for an image, a path).
    Show { title: String, kind: String, content: String, path: Option<String> },
    /// A file write that was allowed without asking, with what changed (a unified diff).
    Diff { id: String, path: String, diff: String },
    /// Something this client does not know about; ignored.
    Other,
}

impl ServerEvent {
    pub fn from_json(v: &Value) -> Self {
        let s = |k: &str| v[k].as_str().unwrap_or("").to_string();
        let n = |k: &str| v[k].as_u64().unwrap_or(0);
        match v["type"].as_str().unwrap_or("") {
            "reasoning" => Self::Reasoning(s("text")),
            "text" => Self::Text(s("text")),
            "tool_start" => Self::ToolStart { id: s("id"), name: s("name"), arguments: s("arguments") },
            "tool_result" => Self::ToolResult { id: s("id"), text: s("text") },
            "ask" => Self::Ask { id: s("id"), tool: s("tool"), what: s("what"), reason: s("reason"), worker: s("worker"), detail: s("detail") },
            "usage" => Self::Usage { prompt_tokens: n("prompt_tokens"), completion_tokens: n("completion_tokens"), cached_tokens: n("cached_tokens") },
            "finish" => Self::Finish(s("reason")),
            "done" => Self::Done { empty: v["empty"].as_bool().unwrap_or(false) },
            "step_limit" => Self::StepLimit,
            "refused" => Self::Refused(s("message")),
            "compacted" => Self::Compacted { messages_summarized: n("messages_summarized") },
            "notice" => Self::Notice(s("message")),
            "error" => Self::Error(s("message")),
            "show" => Self::Show { title: s("title"), kind: s("kind"), content: s("content"), path: v["path"].as_str().map(str::to_string) },
            "diff" => Self::Diff { id: s("id"), path: s("path"), diff: s("diff") },
            _ => Self::Other,
        }
    }
}

pub struct Client {
    base: String,
    token: String,
    http: reqwest::Client,
}

pub type Result<T> = std::result::Result<T, String>;

fn net(base: &str, e: reqwest::Error) -> String {
    if e.is_connect() {
        format!("Cannot reach the core at {base}. Start it with: lhd --provider http://localhost:9292/v1")
    } else {
        e.to_string()
    }
}

impl Client {
    pub fn new(base: impl Into<String>, token: impl Into<String>) -> Self {
        Self { base: base.into().trim_end_matches('/').to_string(), token: token.into().trim().to_string(), http: reqwest::Client::new() }
    }

    pub fn from_token_file(base: impl Into<String>, path: &Path) -> Result<Self> {
        let token = std::fs::read_to_string(path).map_err(|e| format!("cannot read the token file {}: {e}. Is lhd running?", path.display()))?;
        Ok(Self::new(base, token))
    }

    fn req(&self, m: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        self.http.request(m, format!("{}{path}", self.base)).bearer_auth(&self.token)
    }

    async fn send(&self, rb: reqwest::RequestBuilder) -> Result<reqwest::Response> {
        let r = rb.send().await.map_err(|e| net(&self.base, e))?;
        if r.status().is_success() {
            return Ok(r);
        }
        let status = r.status();
        let body: Value = r.json().await.unwrap_or(Value::Null);
        Err(match status.as_u16() {
            401 => "The core refused the token. Restart lhd, or point --token-file at the right file.".into(),
            _ => body["error"].as_str().map(str::to_string).unwrap_or_else(|| format!("the core answered {status}")),
        })
    }

    pub async fn models(&self) -> Result<Vec<String>> {
        let v: Value = self.send(self.req(reqwest::Method::GET, "/v1/models")).await?.json().await.map_err(|e| e.to_string())?;
        Ok(v["models"].as_array().map(|a| a.iter().filter_map(|m| m.as_str().map(str::to_string)).collect()).unwrap_or_default())
    }

    pub async fn create_session(&self, model: &str, project_dir: &Path, ask_all: bool, incognito: bool) -> Result<String> {
        self.create_session_with(model, project_dir, ask_all, incognito, false).await
    }

    /// `create_session`, and `no_sandbox` runs this session's commands without the sandbox (`lh -p --no-sandbox`).
    pub async fn create_session_with(&self, model: &str, project_dir: &Path, ask_all: bool, incognito: bool, no_sandbox: bool) -> Result<String> {
        let body = json!({"model": model, "project_dir": project_dir, "ask_all": ask_all, "incognito": incognito, "no_sandbox": no_sandbox});
        let v: Value = self.send(self.req(reqwest::Method::POST, "/v1/sessions").json(&body)).await?.json().await.map_err(|e| e.to_string())?;
        v["id"].as_str().map(str::to_string).ok_or_else(|| "the core sent no session id".into())
    }

    /// Start a turn. Dropping the returned stream cancels it on the core.
    pub async fn turn(&self, session: &str, text: &str) -> Result<TurnStream> {
        let rb = self.req(reqwest::Method::POST, &format!("/v1/sessions/{session}/turns")).json(&json!({"text": text}));
        Ok(TurnStream { resp: self.send(rb).await?, parser: SseParser::new(), queue: VecDeque::new() })
    }

    /// Any other call to the core, for the memory commands. Returns the JSON reply ({} for no body).
    pub async fn call(&self, method: &str, path: &str, body: Option<Value>) -> Result<Value> {
        let m = reqwest::Method::from_bytes(method.as_bytes()).map_err(|e| e.to_string())?;
        let mut rb = self.req(m, path);
        if let Some(b) = body {
            rb = rb.json(&b);
        }
        let r = self.send(rb).await?;
        Ok(r.json().await.unwrap_or(Value::Null))
    }

    /// A stream of JSON events from any SSE endpoint (a department run).
    pub async fn events(&self, path: &str) -> Result<JsonStream> {
        Ok(JsonStream { resp: self.send(self.req(reqwest::Method::GET, path)).await?, parser: SseParser::new(), queue: VecDeque::new() })
    }

    pub async fn answer(&self, session: &str, ask: &str, yes: bool) -> Result<()> {
        let body = json!({"answer": if yes { "yes" } else { "no" }});
        self.send(self.req(reqwest::Method::POST, &format!("/v1/sessions/{session}/answers/{ask}")).json(&body)).await.map(|_| ())
    }
}

pub struct TurnStream {
    resp: reqwest::Response,
    parser: SseParser,
    queue: VecDeque<ServerEvent>,
}

impl TurnStream {
    /// Next event, or `None` when the core closed the stream.
    pub async fn next(&mut self) -> Result<Option<ServerEvent>> {
        loop {
            if let Some(e) = self.queue.pop_front() {
                return Ok(Some(e));
            }
            match self.resp.chunk().await.map_err(|e| format!("the connection to the core broke: {e}"))? {
                None => return Ok(None),
                Some(bytes) => {
                    for d in self.parser.push(&bytes) {
                        if let Ok(v) = serde_json::from_str::<Value>(&d) {
                            self.queue.push_back(ServerEvent::from_json(&v));
                        }
                    }
                }
            }
        }
    }
}

pub struct JsonStream {
    resp: reqwest::Response,
    parser: SseParser,
    queue: VecDeque<Value>,
}

impl JsonStream {
    pub async fn next(&mut self) -> Result<Option<Value>> {
        loop {
            if let Some(e) = self.queue.pop_front() {
                return Ok(Some(e));
            }
            match self.resp.chunk().await.map_err(|e| format!("the connection to the core broke: {e}"))? {
                None => return Ok(None),
                Some(bytes) => self.queue.extend(self.parser.push(&bytes).iter().filter_map(|d| serde_json::from_str::<Value>(d).ok())),
            }
        }
    }
}
