use futures_util::StreamExt;
use std::time::Duration;

use serde_json::{json, Value};

use crate::{parse_response, Assembler, ChatRequest, Compat, Event, Response, SseParser};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("cannot reach the server: {0}")]
    Network(#[from] reqwest::Error),
    #[error("server answered {status}: {body}")]
    Http { status: u16, body: String },
    #[error("unexpected reply: {0}")]
    Format(String),
}

impl Error {
    /// Worth trying again: connection problems, timeouts, rate limits and server-side failures.
    pub fn is_retryable(&self) -> bool {
        match self {
            Error::Network(e) => e.is_connect() || e.is_timeout() || e.is_request(),
            Error::Http { status, .. } => matches!(*status, 408 | 409 | 425 | 429 | 500 | 502 | 503 | 504),
            Error::Format(_) => false,
        }
    }

    /// One plain sentence for the user, with a hint where the cause is usually obvious.
    pub fn explain(&self) -> String {
        match self {
            Error::Network(_) => "Cannot reach the server. Check that it is running and that the address is right.".into(),
            Error::Http { status: 401 | 403, .. } => "The server refused the key. Check the API key for this provider.".into(),
            Error::Http { status: 404, .. } => "The server does not know this model or address. Check the model name.".into(),
            Error::Http { status: 429, .. } => "The server says too many requests or the limit is used up.".into(),
            Error::Http { status, .. } if *status >= 500 => "The server failed while answering. It may be loading a model or out of memory.".into(),
            Error::Http { status, body } => format!("The server rejected the request ({status}): {body}"),
            Error::Format(_) => "The server answered in a format this app does not understand.".into(),
        }
    }
}

/// Retry policy for the first request of a call. Once a stream has started, nothing is retried:
/// a repeated request would duplicate text the caller already saw.
#[derive(Clone, Debug)]
pub struct Retry {
    pub attempts: u32,
    pub first_delay: std::time::Duration,
}

impl Default for Retry {
    fn default() -> Self {
        Self { attempts: 3, first_delay: std::time::Duration::from_millis(500) }
    }
}

/// One endpoint: base URL (up to and including `/v1`), optional key, and its quirks.
pub struct Provider {
    pub(crate) base_url: String,
    pub(crate) api_key: Option<String>,
    compat: Compat,
    retry: Retry,
    pub(crate) probed: std::sync::Mutex<std::collections::HashMap<String, crate::Capabilities>>,
    http: std::sync::Mutex<reqwest::Client>,
    /// Talk Ollama's native API (`/api/chat`, NDJSON) instead of the OpenAI-compatible one.
    pub(crate) ollama: bool,
}

impl Provider {
    /// `base_url` like `http://localhost:9292/v1`.
    pub fn new(base_url: impl Into<String>, api_key: Option<String>) -> Self {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        let compat = Compat::detect(&base_url);
        Self { base_url, api_key, compat, retry: Retry::default(), probed: Default::default(), http: std::sync::Mutex::new(reqwest::Client::new()), ollama: false }
    }

    pub fn with_compat(mut self, compat: Compat) -> Self {
        self.compat = compat;
        self
    }

    /// Close idle connections and start with a fresh client, so an idle core holds no sockets.
    pub fn release_connections(&self) {
        *self.http.lock().unwrap() = reqwest::Client::new();
    }

    pub(crate) fn http(&self) -> reqwest::Client {
        self.http.lock().unwrap().clone()
    }

    pub fn with_retry(mut self, retry: Retry) -> Self {
        self.retry = retry;
        self
    }

    pub async fn models(&self) -> Result<Vec<String>, Error> {
        if self.ollama {
            return self.ollama_models().await;
        }
        let v: Value = self.get("/models").await?;
        Ok(v["data"].as_array().map(|a| a.iter().filter_map(|m| m["id"].as_str().map(str::to_string)).collect()).unwrap_or_default())
    }

    /// Each model with the modes and load state a llama-swap server reports (other servers: the id only).
    pub async fn model_details(&self) -> Result<Vec<Value>, Error> {
        if self.ollama {
            return Ok(self.ollama_models().await?.into_iter().map(|id| serde_json::json!({"id": id})).collect());
        }
        let v: Value = self.get("/models").await?;
        Ok(v["data"].as_array().map(|a| a.iter().filter_map(details_of).collect()).unwrap_or_default())
    }

    /// What this model supports, asked of the server once and remembered. Asking a llama-swap
    /// server may load the model, which the next chat request needs anyway.
    pub async fn capabilities(&self, model: &str) -> crate::Capabilities {
        if let Some(c) = self.probed.lock().unwrap().get(model) {
            return c.clone();
        }
        if self.ollama {
            return self.ollama_capabilities(model).await;
        }
        let root = self.base_url.strip_suffix("/v1").unwrap_or(&self.base_url).to_string();
        let mut found = crate::Capabilities::default();
        for (path, label) in [(format!("/upstream/{model}/props"), "llama-swap"), ("/props".to_string(), "llama.cpp")] {
            let Ok(r) = self.auth(self.http().get(format!("{root}{path}")).timeout(Duration::from_secs(180))).send().await else { continue };
            if !r.status().is_success() {
                continue;
            }
            if let Some(c) = r.json::<Value>().await.ok().and_then(|v| crate::Capabilities::from_props(&v, label)) {
                found = c;
                break;
            }
        }
        // The parameter count comes from the model list of the server that runs the model.
        if !found.source.is_empty() {
            let list = if found.source == "llama-swap" { format!("{root}/upstream/{model}/v1/models") } else { format!("{root}/v1/models") };
            if let Ok(r) = self.auth(self.http().get(list).timeout(Duration::from_secs(60))).send().await {
                if let Ok(v) = r.json::<Value>().await {
                    found.parameters = v["data"][0]["meta"]["n_params"].as_u64().filter(|n| *n > 0);
                }
            }
        }
        // Only a real answer is remembered; an unknown result is tried again next time.
        if !found.source.is_empty() {
            self.probed.lock().unwrap().insert(model.to_string(), found.clone());
        }
        found
    }

    /// The name of the service behind the endpoint, if it says (llama-swap reports itself as the owner
    /// of its models). `None` when it does not; nothing is guessed from the address.
    pub async fn service_name(&self) -> Option<String> {
        let v = self.get("/models").await.ok()?;
        let owner = v["data"][0]["owned_by"].as_str()?.trim();
        // generic owners say nothing about the service
        (!owner.is_empty() && !matches!(owner, "organization" | "organization-owner" | "system" | "user")).then(|| owner.to_string())
    }

    /// One request, one reply, no streaming.
    pub async fn chat(&self, req: &ChatRequest) -> Result<Response, Error> {
        if self.ollama {
            return self.ollama_chat(req).await;
        }
        let v = self.post("/chat/completions", &self.body(req, false)).await?.json::<Value>().await?;
        if v["choices"][0].is_null() {
            return Err(Error::Format(v.to_string()));
        }
        Ok(parse_response(&v, &self.compat))
    }

    /// Streams the reply, calling `on_event` for each piece, and returns the whole reply at the end.
    /// To cancel, drop the future (for example with `tokio::select!`).
    pub async fn chat_stream(&self, req: &ChatRequest, mut on_event: impl FnMut(Event)) -> Result<Response, Error> {
        if self.ollama {
            return self.ollama_chat_stream(req, on_event).await;
        }
        let resp = self.post("/chat/completions", &self.body(req, true)).await?;
        let mut bytes = resp.bytes_stream();
        let (mut sse, mut asm) = (SseParser::new(), Assembler::new(self.compat.clone()));
        while let Some(chunk) = bytes.next().await {
            for data in sse.push(&chunk?) {
                for ev in asm.feed(&data) {
                    on_event(ev);
                }
            }
        }
        Ok(asm.finish())
    }

    pub(crate) async fn post_json(&self, path: &str, body: &Value) -> Result<Value, Error> {
        Ok(self.post(path, body).await?.json().await?)
    }

    /// Download an address the provider returned (for example an image). Size-limited.
    pub(crate) async fn download(&self, url: &str) -> Result<Vec<u8>, Error> {
        let mut resp = check(self.http().get(url).timeout(Duration::from_secs(120)).send().await?).await?;
        let mut out = Vec::new();
        while let Some(chunk) = resp.chunk().await? {
            out.extend_from_slice(&chunk);
            if out.len() > 25 * 1024 * 1024 {
                return Err(Error::Format("the download is larger than 25 MB".into()));
            }
        }
        Ok(out)
    }

    pub(crate) fn body(&self, req: &ChatRequest, stream: bool) -> Value {
        let mut b = json!({
            "model": req.model,
            "messages": req.messages.iter().map(|m| m.to_json()).collect::<Vec<_>>(),
            "stream": stream,
        });
        if let Some(n) = req.max_tokens {
            b[self.compat.max_tokens_field] = json!(n);
        }
        let sm = &req.sampling;
        if let Some(t) = sm.temperature {
            b["temperature"] = json!(t);
        }
        if let Some(t) = sm.top_p {
            b["top_p"] = json!(t);
        }
        if self.compat.extended_sampling {
            if let Some(t) = sm.top_k {
                b["top_k"] = json!(t);
            }
            if let Some(t) = sm.min_p {
                b["min_p"] = json!(t);
            }
            if let Some(t) = sm.repeat_penalty {
                b["repeat_penalty"] = json!(t);
            }
        }
        for (k, v) in &sm.extra {
            if !matches!(k.as_str(), "model" | "messages" | "stream" | "tools") {
                b[k] = v.clone(); // the fields this client owns cannot be overridden
            }
        }
        // Tools are left out entirely when empty: `tool_choice: "none"` is not honoured by every
        // server (spike 1: models wrote the call as text instead).
        if !req.tools.is_empty() {
            b["tools"] = req
                .tools
                .iter()
                .map(|t| json!({ "type": "function", "function": { "name": t.name, "description": t.description, "parameters": t.parameters } }))
                .collect();
        }
        if stream && self.compat.stream_usage {
            b["stream_options"] = json!({ "include_usage": true });
        }
        b
    }

    pub(crate) fn auth(&self, rb: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.api_key {
            Some(k) => rb.bearer_auth(k),
            None => rb,
        }
    }

    pub(crate) async fn get(&self, path: &str) -> Result<Value, Error> {
        let resp = self
            .with_retries(|| async {
                let r = self.auth(self.http().get(format!("{}{}", self.base_url, path))).send().await?;
                check(r).await
            })
            .await?;
        Ok(resp.json().await?)
    }

    pub(crate) async fn post(&self, path: &str, body: &Value) -> Result<reqwest::Response, Error> {
        self.with_retries(|| async {
            let r = self.auth(self.http().post(format!("{}{}", self.base_url, path))).json(body).send().await?;
            check(r).await
        })
        .await
    }

    async fn with_retries<F, Fut>(&self, mut once: F) -> Result<reqwest::Response, Error>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<reqwest::Response, Error>>,
    {
        let mut delay = self.retry.first_delay;
        let mut left = self.retry.attempts.max(1);
        loop {
            match once().await {
                Err(e) if left > 1 && e.is_retryable() => {
                    left -= 1;
                    tokio::time::sleep(delay).await;
                    delay *= 2;
                }
                other => return other,
            }
        }
    }
}

pub(crate) async fn check(r: reqwest::Response) -> Result<reqwest::Response, Error> {
    if r.status().is_success() {
        return Ok(r);
    }
    let status = r.status().as_u16();
    let body = r.text().await.unwrap_or_default();
    Err(Error::Http { status, body: body.chars().take(500).collect() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Message, Sampling};

    fn req() -> ChatRequest {
        let mut extra = serde_json::Map::new();
        extra.insert("chat_template_kwargs".into(), json!({"enable_thinking": false}));
        extra.insert("stream".into(), json!(false)); // must not win
        ChatRequest {
            model: "m".into(),
            messages: vec![Message::user("hi")],
            max_tokens: Some(64),
            sampling: Sampling { temperature: Some(0.2), top_p: Some(0.9), top_k: Some(40), min_p: Some(0.05), repeat_penalty: Some(1.1), extra },
            ..Default::default()
        }
    }

    #[test]
    fn local_servers_get_every_setting() {
        let b = Provider::new("http://localhost:9292/v1", None).body(&req(), true);
        assert_eq!((b["temperature"].as_f64().map(|x| (x * 10.0).round()), b["top_k"].as_u64(), b["repeat_penalty"].as_f64().map(|x| (x * 10.0).round())), (Some(2.0), Some(40), Some(11.0)));
        assert_eq!(b["chat_template_kwargs"]["enable_thinking"], false);
        assert_eq!(b["stream"], true, "the client's own fields cannot be overridden");
        assert_eq!(b["max_tokens"], 64);
    }

    #[test]
    fn openai_style_servers_do_not_get_unsupported_fields() {
        let b = Provider::new("https://api.openai.com/v1", None).body(&req(), false);
        assert!(b.get("top_k").is_none() && b.get("min_p").is_none() && b.get("repeat_penalty").is_none());
        assert!(b.get("top_p").is_some() && b.get("temperature").is_some());
        assert_eq!(b["max_completion_tokens"], 64);
        assert!(b.get("max_tokens").is_none());
    }

    #[test]
    fn unset_means_the_server_default() {
        let r = ChatRequest { model: "m".into(), messages: vec![Message::user("hi")], ..Default::default() };
        let b = Provider::new("http://localhost:9292/v1", None).body(&r, false);
        for k in ["temperature", "top_p", "top_k", "min_p", "repeat_penalty", "max_tokens", "tools"] {
            assert!(b.get(k).is_none(), "{k}");
        }
    }
}

#[cfg(test)]
mod probe_tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// Answers by request path: 200 with the body for known paths, 404 otherwise.
    async fn serve(routes: Vec<(&'static str, &'static str)>) -> String {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", l.local_addr().unwrap());
        tokio::spawn(async move {
            loop {
                let (mut s, _) = l.accept().await.unwrap();
                let routes = routes.clone();
                tokio::spawn(async move {
                    let mut b = vec![0u8; 4096];
                    let n = s.read(&mut b).await.unwrap_or(0);
                    let line = String::from_utf8_lossy(&b[..n]).lines().next().unwrap_or("").to_string();
                    let path = line.split_whitespace().nth(1).unwrap_or("").to_string();
                    let (status, body) = routes.iter().find(|(p, _)| *p == path).map_or(("404 Not Found", ""), |(_, b)| ("200 OK", *b));
                    let _ = s.write_all(format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await;
                    let _ = s.shutdown().await;
                });
            }
        });
        base
    }
    const PROPS: &str = r#"{"default_generation_settings":{"n_ctx":8192},"chat_template_caps":{"supports_tools":true},"modalities":{"vision":true}}"#;

    #[tokio::test]
    async fn probes_llama_swap_then_plain_llama_cpp_then_gives_up_quietly() {
        let swap = serve(vec![("/upstream/m1/props", PROPS)]).await;
        let c = Provider::new(format!("{swap}/v1"), None).capabilities("m1").await;
        assert_eq!((c.context_tokens, c.tools, c.vision, c.source.as_str()), (Some(8192), Some(true), Some(true), "llama-swap"));

        let direct = serve(vec![("/props", PROPS)]).await;
        let p = Provider::new(format!("{direct}/v1"), None);
        assert_eq!(p.capabilities("anything").await.source, "llama.cpp");

        let none = serve(vec![]).await;
        let p = Provider::new(format!("{none}/v1"), None);
        assert_eq!(p.capabilities("m").await, crate::Capabilities::default());
    }
}

/// One entry of an OpenAI-style model list, with llama-swap's extra facts when present.
pub fn details_of(m: &Value) -> Option<Value> {
    let id = m["id"].as_str()?;
    let mut d = serde_json::json!({"id": id});
    if let Some(modes) = m["meta"]["llamaswap"]["modes"].as_array() {
        d["modes"] = Value::Array(modes.iter().filter(|x| x.is_string()).cloned().collect());
    }
    if let Some(dm) = m["meta"]["llamaswap"]["defaultMode"].as_str() {
        d["default_mode"] = Value::from(dm);
    }
    if let Some(st) = m["status"]["value"].as_str() {
        d["loaded"] = Value::from(st == "ready" || st == "loaded");
    }
    Some(d)
}

#[cfg(test)]
mod details_tests {
    #[test]
    fn llama_swap_modes_and_state_are_kept_and_others_give_the_id() {
        let m = serde_json::json!({"id": "Q-balanced", "meta": {"llamaswap": {"modes": ["think", "instruct"], "defaultMode": "think"}}, "status": {"value": "ready"}});
        assert_eq!(super::details_of(&m).unwrap(), serde_json::json!({"id": "Q-balanced", "modes": ["think", "instruct"], "default_mode": "think", "loaded": true}));
        assert_eq!(super::details_of(&serde_json::json!({"id": "gpt"})).unwrap(), serde_json::json!({"id": "gpt"}));
    }
}
