use futures_util::StreamExt;
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
    base_url: String,
    api_key: Option<String>,
    compat: Compat,
    retry: Retry,
    http: reqwest::Client,
}

impl Provider {
    /// `base_url` like `http://localhost:9292/v1`.
    pub fn new(base_url: impl Into<String>, api_key: Option<String>) -> Self {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        let compat = Compat::detect(&base_url);
        Self { base_url, api_key, compat, retry: Retry::default(), http: reqwest::Client::new() }
    }

    pub fn with_compat(mut self, compat: Compat) -> Self {
        self.compat = compat;
        self
    }

    pub fn with_retry(mut self, retry: Retry) -> Self {
        self.retry = retry;
        self
    }

    pub async fn models(&self) -> Result<Vec<String>, Error> {
        let v: Value = self.get("/models").await?;
        Ok(v["data"].as_array().map(|a| a.iter().filter_map(|m| m["id"].as_str().map(str::to_string)).collect()).unwrap_or_default())
    }

    /// One request, one reply, no streaming.
    pub async fn chat(&self, req: &ChatRequest) -> Result<Response, Error> {
        let v = self.post("/chat/completions", &self.body(req, false)).await?.json::<Value>().await?;
        if v["choices"][0].is_null() {
            return Err(Error::Format(v.to_string()));
        }
        Ok(parse_response(&v, &self.compat))
    }

    /// Streams the reply, calling `on_event` for each piece, and returns the whole reply at the end.
    /// To cancel, drop the future (for example with `tokio::select!`).
    pub async fn chat_stream(&self, req: &ChatRequest, mut on_event: impl FnMut(Event)) -> Result<Response, Error> {
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

    pub(crate) fn body(&self, req: &ChatRequest, stream: bool) -> Value {
        let mut b = json!({
            "model": req.model,
            "messages": req.messages.iter().map(|m| m.to_json()).collect::<Vec<_>>(),
            "stream": stream,
        });
        if let Some(n) = req.max_tokens {
            b[self.compat.max_tokens_field] = json!(n);
        }
        if let Some(t) = req.temperature {
            b["temperature"] = json!(t);
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

    fn auth(&self, rb: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.api_key {
            Some(k) => rb.bearer_auth(k),
            None => rb,
        }
    }

    async fn get(&self, path: &str) -> Result<Value, Error> {
        let resp = self
            .with_retries(|| async {
                let r = self.auth(self.http.get(format!("{}{}", self.base_url, path))).send().await?;
                check(r).await
            })
            .await?;
        Ok(resp.json().await?)
    }

    async fn post(&self, path: &str, body: &Value) -> Result<reqwest::Response, Error> {
        self.with_retries(|| async {
            let r = self.auth(self.http.post(format!("{}{}", self.base_url, path))).json(body).send().await?;
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

async fn check(r: reqwest::Response) -> Result<reqwest::Response, Error> {
    if r.status().is_success() {
        return Ok(r);
    }
    let status = r.status().as_u16();
    let body = r.text().await.unwrap_or_default();
    Err(Error::Http { status, body: body.chars().take(500).collect() })
}
