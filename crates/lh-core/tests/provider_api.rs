//! The provider settings over the real local API: what is set, the connection test in plain words, and the
//! key going to the secret store and never into the settings file.
use std::sync::Arc;
use std::time::Duration;

use lh_core::{auth, router, AppState, Settings};
use lh_provider::{ChatRequest, Event, Response};
use lh_secrets::{MemoryStore, SecretStore};
use lh_store::Store;
use lh_tools::Model;
use serde_json::{json, Value};

struct Quiet;
impl Model for Quiet {
    async fn complete(&self, _: &ChatRequest, _: &mut (dyn FnMut(Event) + Send)) -> Result<Response, lh_provider::Error> {
        unreachable!("no chat in these tests")
    }
    async fn models(&self) -> Result<Vec<String>, lh_provider::Error> {
        Ok(vec!["m1".into(), "m2".into()])
    }
}

struct Server {
    base: String,
    token: String,
    http: reqwest::Client,
    secrets: Arc<MemoryStore>,
    cfg: std::path::PathBuf,
    _dir: tempfile::TempDir,
}
impl Server {
    async fn call(&self, method: &str, path: &str, body: Option<Value>) -> (u16, Value) {
        let mut rb = self.http.request(method.parse().unwrap(), format!("{}{path}", self.base)).bearer_auth(&self.token);
        if let Some(b) = body {
            rb = rb.json(&b);
        }
        let r = rb.send().await.unwrap();
        (r.status().as_u16(), r.json().await.unwrap_or(Value::Null))
    }
}

fn start() -> Server {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("config.toml");
    let store = Arc::new(Store::open(&dir.path().join("lh.db")).unwrap());
    let settings = Settings {
        provider_name: None,
        learn: None,
        maintain_when_idle: false,
        memory: None,
        image_model: None,
        web: Default::default(),
        idle_after: Duration::ZERO,
        compact_at: Some(0),
        store: store.clone(),
        spend: lh_spend::SpendMeter::new(store, lh_spend::Config::default(), "test", true),
        bypass: false,
        sudo_enabled: false,
        allow_rules: vec![],
        team: None,
        mcp: None,
        ask_all: false,
        config: Some(cfg.clone()),
    };
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    l.set_nonblocking(true).unwrap();
    let port = l.local_addr().unwrap().port();
    let token = auth::new_token();
    let state = AppState::new(Quiet, settings);
    let secrets = Arc::new(MemoryStore::default());
    let as_dyn: Arc<dyn SecretStore> = secrets.clone();
    assert!(state.secrets.set(as_dyn).is_ok());
    let app = router(state, token.clone(), port);
    tokio::spawn(async move { axum::serve(tokio::net::TcpListener::from_std(l).unwrap(), app).await.unwrap() });
    Server { base: format!("http://127.0.0.1:{port}"), token, http: reqwest::Client::new(), secrets, cfg, _dir: dir }
}

/// A fake model server: `/models` answers only with the right key when one is required.
async fn fake_server(needs_key: Option<&'static str>) -> String {
    use axum::http::{HeaderMap, StatusCode};
    let app = axum::Router::new().route(
        "/v1/models",
        axum::routing::get(move |h: HeaderMap| async move {
            let ok = needs_key.is_none_or(|k| h.get("authorization").and_then(|v| v.to_str().ok()) == Some(&format!("Bearer {k}")));
            if ok {
                (StatusCode::OK, axum::Json(json!({"data": [{"id": "alpha"}, {"id": "beta"}, {"id": "gamma"}]})))
            } else {
                (StatusCode::UNAUTHORIZED, axum::Json(json!({"error": "no"})))
            }
        }),
    );
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    format!("http://127.0.0.1:{port}/v1")
}

#[tokio::test]
async fn the_provider_is_shown_without_its_key_and_saved_with_the_key_in_the_secret_store() {
    let s = start();
    let (code, v) = s.call("GET", "/v1/provider", None).await;
    assert_eq!((code, v["url"].as_str(), v["key_saved"].as_bool(), v["reachable"].as_bool(), v["models"].as_u64()), (200, Some(""), Some(false), Some(true), Some(2)));

    let (code, v) = s.call("PUT", "/v1/provider", Some(json!({"url": "http://localhost:9292/v1", "name": "Mine", "key": "sk-secret-123"}))).await;
    assert_eq!((code, v["restart_needed"].as_bool()), (200, Some(true)));
    assert_eq!(s.secrets.get("provider-key").unwrap().as_deref(), Some("sk-secret-123"));
    let file = std::fs::read_to_string(&s.cfg).unwrap();
    assert!(file.contains("secret:provider-key") && !file.contains("sk-secret-123"), "the file holds a reference only: {file}");
    let (_, v) = s.call("GET", "/v1/provider", None).await;
    assert_eq!((v["url"].as_str(), v["name"].as_str(), v["key_saved"].as_bool(), v["local"].as_bool()), (Some("http://localhost:9292/v1"), Some("Mine"), Some(true), Some(true)));
    assert!(!v.to_string().contains("sk-secret"), "the key is never sent back");

    // leaving the key out keeps it; an empty key removes it
    s.call("PUT", "/v1/provider", Some(json!({"name": "Other"}))).await;
    assert!(s.secrets.get("provider-key").unwrap().is_some());
    s.call("PUT", "/v1/provider", Some(json!({"key": ""}))).await;
    assert!(s.secrets.get("provider-key").unwrap().is_none());
    assert_eq!(s.call("GET", "/v1/provider", None).await.1["key_saved"], false);

    let (code, v) = s.call("PUT", "/v1/provider", Some(json!({"url": "localhost"}))).await;
    assert_eq!(code, 400);
    assert!(v["error"].as_str().unwrap().contains("http://"));
}

#[tokio::test]
async fn the_connection_test_lists_models_or_says_what_is_wrong() {
    let s = start();
    let open = fake_server(None).await;
    let (_, v) = s.call("POST", "/v1/provider/test", Some(json!({"url": open}))).await;
    assert_eq!((v["ok"].as_bool(), v["models"].as_array().map(Vec::len), v["ollama"].is_null()), (Some(true), Some(3), true));
    assert_eq!(v["message"], "Reached the server. It offers 3 models.");

    let locked = fake_server(Some("good-key")).await;
    let (_, v) = s.call("POST", "/v1/provider/test", Some(json!({"url": locked}))).await;
    assert_eq!(v["ok"], false);
    assert!(v["message"].as_str().unwrap().contains("refused the key"), "{v}");
    let (_, v) = s.call("POST", "/v1/provider/test", Some(json!({"url": locked, "key": "good-key"}))).await;
    assert_eq!(v["ok"], true, "a typed key is tried without being saved");
    assert!(s.secrets.get("provider-key").unwrap().is_none());

    // with no key typed the saved one is used
    s.call("PUT", "/v1/provider", Some(json!({"key": "good-key"}))).await;
    let (_, v) = s.call("POST", "/v1/provider/test", Some(json!({"url": locked}))).await;
    assert_eq!(v["ok"], true, "{v}");

    let (_, v) = s.call("POST", "/v1/provider/test", Some(json!({"url": "http://127.0.0.1:1/v1"}))).await;
    assert!(v["message"].as_str().unwrap().contains("Cannot reach the server"), "{v}");
    let (_, v) = s.call("POST", "/v1/provider/test", Some(json!({"url": "nonsense"}))).await;
    assert_eq!(v["ok"], false);
    assert!(v["message"].as_str().unwrap().contains("http://"));
}

#[tokio::test]
async fn ollama_is_recognised_and_its_models_listed_through_its_own_api() {
    use axum::routing::get;
    let app = axum::Router::new()
        .route("/api/version", get(|| async { axum::Json(json!({"version": "0.5.1"})) }))
        .route("/api/tags", get(|| async { axum::Json(json!({"models": [{"name": "llama3:8b"}]})) }));
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    let s = start();
    let (_, v) = s.call("POST", "/v1/provider/test", Some(json!({"url": format!("http://127.0.0.1:{port}")}))).await;
    assert_eq!((v["ok"].as_bool(), v["ollama"].as_str(), v["models"][0].as_str()), (Some(true), Some("0.5.1"), Some("llama3:8b")));
    assert_eq!(v["message"], "Reached Ollama 0.5.1. It has 1 model.");
}

#[tokio::test]
async fn presets_are_listed_without_keys_and_a_preset_fills_the_address() {
    let s = start();
    let (code, v) = s.call("GET", "/v1/providers/presets", None).await;
    assert_eq!(code, 200);
    let list = v["presets"].as_array().unwrap();
    assert!(list.len() >= 20);
    let openai = list.iter().find(|p| p["id"] == "openai").unwrap();
    assert_eq!(openai["url"], "https://api.openai.com/v1");
    assert_eq!(openai["key_env"], "OPENAI_API_KEY");
    assert_eq!(openai["compat"]["max_tokens_field"], "max_completion_tokens");
    assert!(!v.to_string().contains("sk-"));
    let (code, _) = s.call("PUT", "/v1/provider", Some(json!({"preset": "groq"}))).await;
    assert_eq!(code, 200);
    let (_, p) = s.call("GET", "/v1/provider", None).await;
    assert_eq!(p["url"], "https://api.groq.com/openai/v1");
    assert_eq!(p["name"], "Groq");
    let (code, _) = s.call("PUT", "/v1/provider", Some(json!({"preset": "anthropic"}))).await;
    assert_eq!(code, 200);
    let (_, p) = s.call("GET", "/v1/provider", None).await;
    assert_eq!(p["api"], "anthropic");
    assert_eq!(p["url"], "https://api.anthropic.com");
    s.call("PUT", "/v1/provider", Some(json!({"preset": "groq"}))).await;
    let (_, p) = s.call("GET", "/v1/provider", None).await;
    assert_eq!(p["api"], "openai");
    let (code, _) = s.call("PUT", "/v1/provider", Some(json!({"preset": "nope"}))).await;
    assert_eq!(code, 400);
}
