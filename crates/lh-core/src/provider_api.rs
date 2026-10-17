//! The model provider from the settings screens: what is set, a connection test that says what is wrong in
//! plain words, and saving the address and the key. The key goes to the system's secret store (the same place
//! `lh setup` puts it); the settings file keeps only a reference, and no call ever returns the key.

use std::sync::Arc;

use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use lh_config::ConfigFile;
use lh_provider::Provider;
use lh_secrets::SecretStore;
use lh_tools::Model;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{ApiError, AppState};

/// The name the key is stored under, as `lh setup` does.
const KEY_NAME: &str = "provider-key";

pub(crate) fn routes<M: Model + 'static>() -> Router<Arc<AppState<M>>> {
    Router::new().route("/v1/provider", get(show::<M>).put(save::<M>)).route("/v1/provider/test", post(test::<M>))
}

fn load<M>(st: &AppState<M>) -> Result<ConfigFile, ApiError> {
    let path = st.settings.config.clone().ok_or(ApiError(StatusCode::NOT_FOUND, "this core was started without a settings file".into()))?;
    ConfigFile::load(path).map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

fn secrets<M>(st: &AppState<M>) -> Arc<dyn SecretStore> {
    st.secrets.get_or_init(|| Arc::new(lh_secrets::OsStore::new("lastharness"))).clone()
}

fn no_device(via: Option<Extension<crate::web::Via>>) -> Result<(), ApiError> {
    if matches!(via.map(|v| v.0), Some(crate::web::Via::Device(_))) {
        return Err(ApiError(StatusCode::FORBIDDEN, "the provider and its key cannot be changed from a browser; use the app on the machine".into()));
    }
    Ok(())
}

/// What a web address needs to look like, said in plain words. `None` when it is fine.
pub fn check_url(url: &str) -> Option<String> {
    let u = url.trim();
    if u.is_empty() {
        return Some("Type the address of the model server first.".into());
    }
    let rest = u.strip_prefix("http://").or_else(|| u.strip_prefix("https://"));
    if rest.is_none_or(str::is_empty) || u.contains(' ') {
        return Some("The address should start with http:// or https://, for example http://localhost:9292/v1".into());
    }
    None
}

async fn show<M: Model + 'static>(State(st): State<Arc<AppState<M>>>) -> Result<Json<Value>, ApiError> {
    let cfg = load(&st)?;
    let saved_url = cfg.get_opt("provider.url").unwrap_or_default();
    // the core may have been started with --provider and nothing saved: show what it actually uses
    let started_with = st.kv_endpoint.get().map(|(u, _)| u.clone()).filter(|_| saved_url.is_empty());
    let url = started_with.clone().unwrap_or(saved_url);
    let key = cfg.get_opt("provider.key").unwrap_or_default();
    let name = cfg.get_opt("provider.name").filter(|n| !n.is_empty()).or_else(|| started_with.as_ref().and(st.settings.provider_name.clone())).unwrap_or_default();
    let (reachable, models) = match st.model.models().await {
        Ok(m) => (true, m.len()),
        Err(_) => (false, 0),
    };
    Ok(Json(json!({
        "url": url,
        "name": name,
        "from_start": started_with.is_some(),
        "key_saved": !key.is_empty(),
        "local": !url.is_empty() && lh_provider::is_local_url(&url),
        "reachable": reachable,
        "models": models,
    })))
}

#[derive(Deserialize)]
struct TestBody {
    url: String,
    /// A key typed but not saved yet. Without it the saved key is used.
    key: Option<String>,
}

/// Try an address (and key): list its models, or say in plain words what went wrong. Nothing is saved.
async fn test<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, via: Option<Extension<crate::web::Via>>, Json(b): Json<TestBody>) -> Result<Json<Value>, ApiError> {
    no_device(via)?;
    if let Some(m) = check_url(&b.url) {
        return Ok(Json(json!({"ok": false, "message": m, "models": [], "ollama": null})));
    }
    let key = match b.key.filter(|k| !k.trim().is_empty()) {
        Some(k) => Some(k.trim().to_string()),
        None => saved_key(&st),
    };
    let url = b.url.trim().to_string();
    // Ollama lists its models at its own address, so look for it first
    let ollama = Provider::detect_ollama(&url, key.clone()).await;
    let provider = if ollama.is_some() { Provider::ollama(&url, key) } else { Provider::new(&url, key) };
    Ok(Json(match provider.models().await {
        Ok(models) => {
            let plural = |n: usize| if n == 1 { "" } else { "s" };
            let message = match (models.len(), &ollama) {
                (0, _) => "Reached the server, but it lists no models yet.".to_string(),
                (n, Some(v)) => format!("Reached Ollama {v}. It has {n} model{}.", plural(n)),
                (n, None) => format!("Reached the server. It offers {n} model{}.", plural(n)),
            };
            json!({"ok": true, "message": message, "models": models, "ollama": ollama})
        }
        Err(e) => json!({"ok": false, "message": e.explain(), "models": [], "ollama": ollama}),
    }))
}

/// The saved key, if there is one and the secret store gives it up.
fn saved_key<M>(st: &AppState<M>) -> Option<String> {
    let cfg = load(st).ok()?;
    let r = cfg.get_opt("provider.key").filter(|r| !r.is_empty())?;
    lh_secrets::resolve(&*secrets(st), &r).ok().flatten()
}

#[derive(Deserialize)]
struct SaveBody {
    url: Option<String>,
    name: Option<String>,
    /// A new key to keep. An empty text removes the saved key. Left out, the key stays as it is.
    key: Option<String>,
}

async fn save<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, via: Option<Extension<crate::web::Via>>, Json(b): Json<SaveBody>) -> Result<Json<Value>, ApiError> {
    no_device(via)?;
    let mut cfg = load(&st)?;
    let bad = |m: String| ApiError(StatusCode::BAD_REQUEST, m);
    if let Some(u) = &b.url {
        if let Some(m) = check_url(u) {
            return Err(bad(m));
        }
        cfg.set("provider.url", u.trim()).map_err(|e| bad(e.to_string()))?;
    }
    if let Some(n) = &b.name {
        cfg.set("provider.name", n.trim()).map_err(|e| bad(e.to_string()))?;
    }
    if let Some(k) = &b.key {
        let store = secrets(&st);
        let fail = |e: lh_secrets::Error| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
        if k.trim().is_empty() {
            store.delete(KEY_NAME).map_err(fail)?;
            cfg.set("provider.key", "").map_err(|e| bad(e.to_string()))?;
        } else {
            store.set(KEY_NAME, k.trim()).map_err(fail)?;
            cfg.set("provider.key", &format!("secret:{KEY_NAME}")).map_err(|e| bad(e.to_string()))?;
        }
    }
    cfg.save().map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let _ = st.settings.store.log("setting_changed", "provider", "user", &json!({"url": b.url, "name": b.name, "key": b.key.as_ref().map(|k| if k.trim().is_empty() { "removed" } else { "saved" })}));
    Ok(Json(json!({"saved": true, "restart_needed": true})))
}

#[cfg(test)]
mod tests {
    use super::check_url;

    #[test]
    fn addresses_are_checked_in_plain_words() {
        assert!(check_url("").unwrap().contains("Type the address"));
        assert!(check_url("localhost:9292").unwrap().contains("http://"));
        assert!(check_url("http://").is_some());
        assert!(check_url("http://a b").is_some());
        assert!(check_url(" http://localhost:9292/v1 ").is_none());
        assert!(check_url("https://api.example.com/v1").is_none());
    }
}
