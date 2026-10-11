//! Embeddings for meaning-based search. `Embedder` is the interface; `HttpEmbedder` talks to any
//! OpenAI-style `/v1/embeddings` endpoint; `ManagedEmbedder` starts a local `llama-server` with a
//! small multilingual model when first needed and stops it when the core goes idle.

use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use crate::Trust;

#[derive(Debug, thiserror::Error)]
pub enum EmbedError {
    #[error("the embedding server cannot be reached: {0}")]
    Network(String),
    #[error("the embedding server answered badly: {0}")]
    Format(String),
    #[error("the embedding model could not be started: {0}")]
    Start(String),
    #[error("{0}")]
    Refused(String),
}

pub trait Embedder: Send + Sync {
    /// Name of the model; vectors from different models are never mixed.
    fn model_id(&self) -> &str;
    /// Where the texts are sent. Sensitive pages are only embedded by `Trust::Local` embedders.
    fn trust(&self) -> Trust;
    fn embed_documents(&self, texts: &[String]) -> impl std::future::Future<Output = Result<Vec<Vec<f32>>, EmbedError>> + Send;
    fn embed_query(&self, text: &str) -> impl std::future::Future<Output = Result<Vec<f32>, EmbedError>> + Send;
    /// The core went idle: stop anything that holds memory.
    fn release(&self) -> impl std::future::Future<Output = ()> + Send {
        async {}
    }
}

fn normalized(mut v: Vec<f32>) -> Vec<f32> {
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if n > 0.0 {
        v.iter_mut().for_each(|x| *x /= n);
    }
    v
}

/// Task prefixes some models need in front of the text.
#[derive(Clone, Debug, Default)]
pub struct Prefixes {
    pub query: String,
    pub document: String,
}

impl Prefixes {
    /// What EmbeddingGemma was trained with.
    pub fn embedding_gemma() -> Self {
        Self { query: "task: search result | query: ".into(), document: "title: none | text: ".into() }
    }
}

pub struct HttpEmbedder {
    base_url: String,
    model: String,
    api_key: Option<String>,
    prefixes: Prefixes,
    trust: Trust,
    http: reqwest::Client,
}

impl HttpEmbedder {
    /// `base_url` like `http://127.0.0.1:8080/v1`. A server that is not on this machine is refused
    /// unless `allow_cloud` is set: memory text never goes to a cloud endpoint by accident.
    pub fn new(base_url: &str, model: &str, api_key: Option<String>, prefixes: Prefixes, allow_cloud: bool) -> Result<Self, EmbedError> {
        let local = lh_provider::is_local_url(base_url);
        if !local && !allow_cloud {
            return Err(EmbedError::Refused("this embedding server is not on this machine. Embeddings only go to a cloud endpoint if you allow it.".into()));
        }
        Ok(Self {
            base_url: base_url.trim_end_matches('/').into(),
            model: model.into(),
            api_key,
            prefixes,
            trust: if local { Trust::Local } else { Trust::AnyCloud },
            http: reqwest::Client::builder().timeout(Duration::from_secs(120)).build().map_err(|e| EmbedError::Network(e.to_string()))?,
        })
    }

    async fn call(&self, inputs: Vec<String>) -> Result<Vec<Vec<f32>>, EmbedError> {
        let mut rb = self.http.post(format!("{}/embeddings", self.base_url)).json(&json!({"model": self.model, "input": inputs}));
        if let Some(k) = &self.api_key {
            rb = rb.bearer_auth(k);
        }
        let r = rb.send().await.map_err(|e| EmbedError::Network(e.to_string()))?;
        if !r.status().is_success() {
            return Err(EmbedError::Format(format!("status {}", r.status())));
        }
        let v: Value = r.json().await.map_err(|e| EmbedError::Format(e.to_string()))?;
        let mut rows: Vec<(u64, Vec<f32>)> = v["data"]
            .as_array()
            .ok_or_else(|| EmbedError::Format("no `data`".into()))?
            .iter()
            .map(|d| Ok((d["index"].as_u64().unwrap_or(0), d["embedding"].as_array().ok_or_else(|| EmbedError::Format("no `embedding`".into()))?.iter().map(|x| x.as_f64().unwrap_or(0.0) as f32).collect())))
            .collect::<Result<_, EmbedError>>()?;
        rows.sort_by_key(|r| r.0);
        if rows.len() != inputs.len() || rows.iter().any(|r| r.1.is_empty()) {
            return Err(EmbedError::Format("wrong number of vectors".into()));
        }
        Ok(rows.into_iter().map(|r| normalized(r.1)).collect())
    }
}

impl Embedder for HttpEmbedder {
    fn model_id(&self) -> &str {
        &self.model
    }
    fn trust(&self) -> Trust {
        self.trust
    }
    async fn embed_documents(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        let mut out = Vec::new();
        for batch in texts.chunks(16) {
            out.extend(self.call(batch.iter().map(|t| format!("{}{t}", self.prefixes.document)).collect()).await?);
        }
        Ok(out)
    }
    async fn embed_query(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        Ok(self.call(vec![format!("{}{text}", self.prefixes.query)]).await?.remove(0))
    }
}

/// How to start the local embedding server.
#[derive(Clone, Debug)]
pub struct ManagedConfig {
    /// The `llama-server` program.
    pub server: PathBuf,
    /// Extra library folders it needs (`LD_LIBRARY_PATH`), for prebuilt CUDA builds.
    pub library_path: Option<String>,
    pub model_file: PathBuf,
    pub model_name: String,
    pub prefixes: Prefixes,
    pub threads: u32,
}

/// A local embedding server that starts on first use and stops on `release` (idle mode).
pub struct ManagedEmbedder {
    cfg: ManagedConfig,
    running: Mutex<Option<(Child, HttpEmbedder)>>,
}

impl ManagedEmbedder {
    pub fn new(cfg: ManagedConfig) -> Self {
        Self { cfg, running: Mutex::new(None) }
    }

    async fn start(&self) -> Result<(Child, HttpEmbedder), EmbedError> {
        let port = std::net::TcpListener::bind("127.0.0.1:0").and_then(|l| l.local_addr()).map_err(|e| EmbedError::Start(e.to_string()))?.port();
        let mut cmd = Command::new(&self.cfg.server);
        cmd.arg("-m").arg(&self.cfg.model_file).args(["--embedding", "--host", "127.0.0.1", "--port", &port.to_string(), "-ngl", "0", "-c", "2048", "-ub", "2048", "-np", "1", "-t", &self.cfg.threads.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        if let Some(l) = &self.cfg.library_path {
            cmd.env("LD_LIBRARY_PATH", l);
        }
        // The embedder is meant to run on the CPU. A CUDA build would still open a GPU context, which
        // fails (or steals memory) when a chat model fills the GPU, so the GPU is hidden from it.
        cmd.env("CUDA_VISIBLE_DEVICES", "");
        let mut child = cmd.spawn().map_err(|e| EmbedError::Start(format!("{}: {e}", self.cfg.server.display())))?;
        let health = format!("http://127.0.0.1:{port}/health");
        let client = reqwest::Client::new();
        for _ in 0..300 {
            if let Ok(Some(status)) = child.try_wait() {
                return Err(EmbedError::Start(format!("the embedding server stopped at once ({status})")));
            }
            if client.get(&health).timeout(Duration::from_secs(2)).send().await.is_ok_and(|r| r.status().is_success()) {
                let http = HttpEmbedder::new(&format!("http://127.0.0.1:{port}/v1"), &self.cfg.model_name, None, self.cfg.prefixes.clone(), false)?;
                return Ok((child, http));
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        let _ = child.kill().await;
        Err(EmbedError::Start("the embedding server did not become ready in 30 s".into()))
    }

    pub async fn is_running(&self) -> bool {
        self.running.lock().await.is_some()
    }
}

impl Embedder for ManagedEmbedder {
    fn model_id(&self) -> &str {
        &self.cfg.model_name
    }
    fn trust(&self) -> Trust {
        Trust::Local
    }
    async fn embed_documents(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        let mut g = self.running.lock().await;
        if g.is_none() {
            *g = Some(self.start().await?);
        }
        g.as_ref().unwrap().1.embed_documents(texts).await
    }
    async fn embed_query(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        let mut g = self.running.lock().await;
        if g.is_none() {
            *g = Some(self.start().await?);
        }
        g.as_ref().unwrap().1.embed_query(text).await
    }
    async fn release(&self) {
        if let Some((mut child, _)) = self.running.lock().await.take() {
            let _ = child.kill().await;
        }
    }
}

/// The embedder the app uses, chosen by settings. `Off` means meaning-based search is not set up;
/// callers fall back to keyword search.
pub enum AnyEmbedder {
    Off,
    Managed(Box<ManagedEmbedder>),
    Http(HttpEmbedder),
}

impl Embedder for AnyEmbedder {
    fn model_id(&self) -> &str {
        match self {
            AnyEmbedder::Off => "none",
            AnyEmbedder::Managed(e) => e.model_id(),
            AnyEmbedder::Http(e) => e.model_id(),
        }
    }
    fn trust(&self) -> Trust {
        match self {
            AnyEmbedder::Off | AnyEmbedder::Managed(_) => Trust::Local,
            AnyEmbedder::Http(e) => e.trust(),
        }
    }
    async fn embed_documents(&self, t: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        match self {
            AnyEmbedder::Off => Err(EmbedError::Refused("no embedding model is set up".into())),
            AnyEmbedder::Managed(e) => e.embed_documents(t).await,
            AnyEmbedder::Http(e) => e.embed_documents(t).await,
        }
    }
    async fn embed_query(&self, t: &str) -> Result<Vec<f32>, EmbedError> {
        match self {
            AnyEmbedder::Off => Err(EmbedError::Refused("no embedding model is set up".into())),
            AnyEmbedder::Managed(e) => e.embed_query(t).await,
            AnyEmbedder::Http(e) => e.embed_query(t).await,
        }
    }
    async fn release(&self) {
        if let AnyEmbedder::Managed(e) = self {
            e.release().await;
        }
    }
}

impl AnyEmbedder {
    pub fn is_on(&self) -> bool {
        !matches!(self, AnyEmbedder::Off)
    }
}
