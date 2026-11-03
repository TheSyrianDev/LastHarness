//! MCP client: use tools from MCP servers. Client only; LastHarness is not an MCP server.
//!
//! A server is reached one of two ways: a local program talking over its standard input and output, or a
//! remote server over HTTP (a POST per message; the reply is JSON or an event stream). Only tools are
//! supported for now. Resources, prompts and elicitation come when something needs them.
//!
//! Each tool becomes an `ExtraTool`, so it goes through the same permission rules as every other tool:
//! it asks every time unless the user made an allow rule, and what is sent to a remote server is checked
//! for sensitive data first.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use lh_tools::{BoxFut, ExtraTool, Risk, ToolError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::oneshot;

const PROTOCOL: &str = "2025-06-18";
const INIT_TIMEOUT: Duration = Duration::from_secs(30);
const CALL_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Msg(String),
    #[error("the server answered with an error: {0}")]
    Server(String),
    #[error("the server did not answer in time")]
    Timeout,
}
pub type Result<T> = std::result::Result<T, Error>;

fn msg(s: impl Into<String>) -> Error {
    Error::Msg(s.into())
}

/// How to reach a server. What the user approves is exactly this.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Transport {
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        /// Variables the server is given, as `NAME = "env:VAR"` (read from the core's environment when it
        /// starts) or a plain value. It gets nothing else from the core's environment but a few harmless ones.
        #[serde(default)]
        env: std::collections::BTreeMap<String, String>,
    },
    Http {
        url: String,
        /// Name of an environment variable that holds a bearer token. The token itself is never stored here.
        #[serde(default)]
        bearer_env: Option<String>,
    },
}

impl Transport {
    /// The exact command line or link, as shown when the user is asked to approve.
    pub fn display(&self) -> String {
        match self {
            Transport::Stdio { command, args, .. } => std::iter::once(command.as_str()).chain(args.iter().map(String::as_str)).collect::<Vec<_>>().join(" "),
            Transport::Http { url, .. } => url.clone(),
        }
    }
    pub fn is_remote(&self) -> bool {
        matches!(self, Transport::Http { .. })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub schema: Value,
}

enum Link {
    Stdio {
        stdin: tokio::sync::Mutex<tokio::process::ChildStdin>,
        pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>,
        _child: tokio::process::Child,
    },
    Http {
        http: reqwest::Client,
        url: String,
        bearer: Option<String>,
        session: Mutex<Option<String>>,
    },
}

pub struct Connection {
    pub name: String,
    pub transport: Transport,
    pub tools: Vec<McpTool>,
    link: Link,
    next: AtomicU64,
}

impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection").field("name", &self.name).field("tools", &self.tools.len()).finish()
    }
}

fn rpc_result(v: &Value) -> Result<Value> {
    if let Some(e) = v.get("error") {
        return Err(Error::Server(e["message"].as_str().unwrap_or("unknown error").to_string()));
    }
    Ok(v["result"].clone())
}

impl Connection {
    /// Start or reach the server, shake hands and list its tools.
    pub async fn connect(name: &str, transport: Transport) -> Result<Arc<Connection>> {
        let link = match &transport {
            Transport::Stdio { command, args, env } => {
                let mut given: Vec<(String, String)> = std::env::vars().filter(|(k, _)| matches!(k.as_str(), "PATH" | "HOME" | "USER" | "LANG" | "TERM" | "TZ" | "TMPDIR") || k.starts_with("LC_")).collect();
                for (k, v) in env {
                    match v.strip_prefix("env:") {
                        Some(var) => given.push((k.clone(), std::env::var(var).map_err(|_| msg(format!("the environment variable {var}, which {k} should be read from, is not set")))?)),
                        None => given.push((k.clone(), v.clone())),
                    }
                }
                let mut child = tokio::process::Command::new(command)
                    .args(args)
                    .env_clear()
                    .envs(given)
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::null())
                    .kill_on_drop(true)
                    .spawn()
                    .map_err(|e| msg(format!("cannot start `{}`: {e}", transport.display())))?;
                let stdin = child.stdin.take().ok_or_else(|| msg("no input channel to the server"))?;
                let stdout = child.stdout.take().ok_or_else(|| msg("no output channel from the server"))?;
                let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>> = Default::default();
                let p2 = pending.clone();
                tokio::spawn(async move {
                    let mut lines = BufReader::new(stdout).lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        let Ok(v) = serde_json::from_str::<Value>(&line) else { continue };
                        // a reply has an id and no method; requests and notifications from the server are ignored
                        if v.get("method").is_none() {
                            if let Some(id) = v["id"].as_u64() {
                                if let Some(tx) = p2.lock().unwrap().remove(&id) {
                                    let _ = tx.send(v);
                                }
                            }
                        }
                    }
                    p2.lock().unwrap().clear(); // the server ended: waiting calls fail
                });
                Link::Stdio { stdin: tokio::sync::Mutex::new(stdin), pending, _child: child }
            }
            Transport::Http { url, bearer_env } => Link::Http {
                http: reqwest::Client::new(),
                url: url.clone(),
                bearer: bearer_env.as_ref().and_then(|e| std::env::var(e).ok()),
                session: Mutex::new(None),
            },
        };
        let mut c = Connection { name: name.to_string(), transport, tools: vec![], link, next: AtomicU64::new(1) };
        tokio::time::timeout(INIT_TIMEOUT, c.handshake()).await.map_err(|_| Error::Timeout)??;
        Ok(Arc::new(c))
    }

    async fn handshake(&mut self) -> Result<()> {
        self.request("initialize", json!({"protocolVersion": PROTOCOL, "capabilities": {}, "clientInfo": {"name": "lastharness", "version": env!("CARGO_PKG_VERSION")}})).await?;
        self.notify("notifications/initialized").await?;
        let mut cursor: Option<String> = None;
        loop {
            let r = self.request("tools/list", cursor.as_ref().map(|c| json!({"cursor": c})).unwrap_or(json!({}))).await?;
            for t in r["tools"].as_array().into_iter().flatten() {
                if let Some(name) = t["name"].as_str() {
                    self.tools.push(McpTool { name: name.into(), description: t["description"].as_str().unwrap_or("").into(), schema: t.get("inputSchema").cloned().unwrap_or(json!({"type": "object"})) });
                }
            }
            match r["nextCursor"].as_str() {
                Some(c) if self.tools.len() < 500 => cursor = Some(c.to_string()),
                _ => return Ok(()),
            }
        }
    }

    async fn notify(&self, method: &str) -> Result<()> {
        let body = json!({"jsonrpc": "2.0", "method": method});
        match &self.link {
            Link::Stdio { stdin, .. } => {
                let mut s = stdin.lock().await;
                s.write_all(format!("{body}\n").as_bytes()).await.map_err(|e| msg(format!("the server is gone: {e}")))?;
                s.flush().await.map_err(|e| msg(e.to_string()))
            }
            Link::Http { .. } => self.http_post(&body).await.map(|_| ()),
        }
    }

    async fn request(&self, method: &str, params: Value) -> Result<Value> {
        let id = self.next.fetch_add(1, Ordering::SeqCst);
        let body = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        match &self.link {
            Link::Stdio { stdin, pending, .. } => {
                let (tx, rx) = oneshot::channel();
                pending.lock().unwrap().insert(id, tx);
                {
                    let mut s = stdin.lock().await;
                    s.write_all(format!("{body}\n").as_bytes()).await.map_err(|e| msg(format!("the server is gone: {e}")))?;
                    s.flush().await.map_err(|e| msg(e.to_string()))?;
                }
                let v = tokio::time::timeout(CALL_TIMEOUT, rx).await.map_err(|_| Error::Timeout)?.map_err(|_| msg("the server ended before it answered"))?;
                rpc_result(&v)
            }
            Link::Http { .. } => {
                let v = self.http_post(&body).await?.ok_or_else(|| msg("the server sent no answer"))?;
                rpc_result(&v)
            }
        }
    }

    /// POST one message. A reply may be JSON or an event stream; the one with our id is returned.
    async fn http_post(&self, body: &Value) -> Result<Option<Value>> {
        let Link::Http { http, url, bearer, session } = &self.link else { return Ok(None) };
        let mut rb = http.post(url).timeout(CALL_TIMEOUT).header("Accept", "application/json, text/event-stream").header("MCP-Protocol-Version", PROTOCOL).json(body);
        if let Some(b) = bearer {
            rb = rb.bearer_auth(b);
        }
        if let Some(s) = session.lock().unwrap().clone() {
            rb = rb.header("Mcp-Session-Id", s);
        }
        let r = rb.send().await.map_err(|e| msg(format!("cannot reach {url}: {e}")))?;
        if let Some(s) = r.headers().get("mcp-session-id").and_then(|v| v.to_str().ok()) {
            *session.lock().unwrap() = Some(s.to_string());
        }
        let status = r.status();
        if !status.is_success() {
            return Err(msg(match status.as_u16() {
                401 | 403 => format!("{url} refused the request ({status}); it may need a key"),
                _ => format!("{url} answered {status}"),
            }));
        }
        let want = body.get("id").cloned();
        if want.is_none() {
            return Ok(None); // a notification: nothing comes back that we need
        }
        let is_stream = r.headers().get("content-type").and_then(|v| v.to_str().ok()).is_some_and(|t| t.contains("text/event-stream"));
        if !is_stream {
            return r.json::<Value>().await.map(Some).map_err(|e| msg(format!("the server's answer was not JSON: {e}")));
        }
        let mut r = r;
        let mut p = lh_provider::SseParser::new();
        while let Some(chunk) = r.chunk().await.map_err(|e| msg(e.to_string()))? {
            for d in p.push(&chunk) {
                if let Ok(v) = serde_json::from_str::<Value>(&d) {
                    if v.get("id") == want.as_ref() && v.get("method").is_none() {
                        return Ok(Some(v));
                    }
                }
            }
        }
        Err(msg("the event stream ended without an answer"))
    }

    /// Call a tool and return its text. Non-text parts are named, not shown.
    pub async fn call(&self, tool: &str, args: Value) -> Result<String> {
        let r = self.request("tools/call", json!({"name": tool, "arguments": args})).await?;
        let mut parts = vec![];
        for c in r["content"].as_array().into_iter().flatten() {
            match c["type"].as_str() {
                Some("text") => parts.push(c["text"].as_str().unwrap_or("").to_string()),
                Some(other) => parts.push(format!("[{other} content not shown]")),
                None => {}
            }
        }
        let text = parts.join("\n");
        if r["isError"] == true {
            return Err(Error::Server(if text.is_empty() { "the tool reported an error".into() } else { text }));
        }
        Ok(text)
    }
}

/// The name a model sees: `mcp__<server>__<tool>`, limited to letters, digits, `_` and `-`.
pub fn qualified_name(server: &str, tool: &str) -> String {
    let clean = |s: &str| s.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).collect::<String>();
    let mut n = format!("mcp__{}__{}", clean(server), clean(tool));
    n.truncate(64);
    n
}

struct McpExtraTool {
    conn: Arc<Connection>,
    tool: McpTool,
    name: String,
}

impl std::fmt::Debug for McpExtraTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "McpExtraTool({})", self.name)
    }
}

impl ExtraTool for McpExtraTool {
    fn def(&self) -> lh_provider::ToolDef {
        let mut d = self.tool.description.clone();
        d.truncate(300);
        lh_provider::ToolDef { name: self.name.clone(), description: d, parameters: self.tool.schema.clone() }
    }
    fn risk(&self) -> Risk {
        Risk::Asks
    }
    fn describe(&self, args: &Value) -> String {
        let a = args.to_string();
        format!("{} on {}{}: {}", self.tool.name, self.conn.name, if self.conn.transport.is_remote() { " (remote)" } else { "" }, if a.len() > 160 { format!("{}...", a.chars().take(160).collect::<String>()) } else { a })
    }
    fn run(&self, args: Value) -> BoxFut<'_, std::result::Result<String, ToolError>> {
        Box::pin(async move {
            // Agents send prompts and arguments, never a dump of the user's private notes; what goes to a
            // remote server is checked for anything that looks sensitive.
            if self.conn.transport.is_remote() {
                let c = lh_memory::detect(&args.to_string());
                if c.privacy == lh_memory::Privacy::Sensitive {
                    return Err(ToolError::Failed(format!("not sent: the arguments look sensitive ({}), and this server is remote", c.reason)));
                }
            }
            self.conn.call(&self.tool.name, args).await.map_err(|e| ToolError::Failed(e.to_string()))
        })
    }
}

/// Every tool of the connection as a tool the turn loop can use.
pub fn tools(conn: &Arc<Connection>) -> Vec<Arc<dyn ExtraTool>> {
    conn.tools.iter().map(|t| Arc::new(McpExtraTool { conn: conn.clone(), name: qualified_name(&conn.name, &t.name), tool: t.clone() }) as Arc<dyn ExtraTool>).collect()
}
