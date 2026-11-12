//! `/v1/providers`: the registered providers, and the proposals the model has made that wait for the user.
//! Like the single-provider settings, none of this can be changed from a paired device, and no response
//! ever carries a key or a key reference.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Extension, Json, Router};
use lh_tools::Model;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::registry::{KeyChange, NewProvider, Registry, Update};
use crate::{ApiError, AppState};

pub fn routes<M: Model + 'static>() -> Router<Arc<AppState<M>>> {
    Router::new()
        .route("/v1/providers", get(list::<M>).post(add::<M>))
        .route("/v1/providers/proposals", get(proposals::<M>))
        .route("/v1/providers/proposals/{id}/approve", post(approve::<M>))
        .route("/v1/providers/proposals/{id}/reject", post(reject::<M>))
        .route("/v1/providers/{id}", delete(remove::<M>).patch(edit::<M>))
        .route("/v1/providers/{id}/default", post(make_default::<M>))
        .route("/v1/providers/{id}/test", post(test_entry::<M>))
        .route("/v1/providers/{id}/models", post(refresh_models::<M>))
        .route("/v1/providers/{id}/models/toggle", post(toggle_model::<M>))
        .route("/v1/providers/{id}/default-model", post(default_model::<M>))
}

/// The registry of this core, opened on first use: from the settings file when there is one.
pub(crate) fn registry<M>(st: &AppState<M>) -> Arc<Registry> {
    st.registry
        .get_or_init(|| {
            let secrets = crate::provider_api::secrets(st);
            Arc::new(match &st.settings.config {
                Some(c) => Registry::from_config(c, secrets),
                None => Registry::open(None, None, secrets),
            })
        })
        .clone()
}

fn bad(m: impl Into<String>) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, m.into())
}

fn no_device(via: Option<Extension<crate::web::Via>>) -> Result<(), ApiError> {
    if matches!(via.map(|v| v.0), Some(crate::web::Via::Device(_))) {
        return Err(ApiError(StatusCode::FORBIDDEN, "providers and their keys cannot be changed from a browser; use the app on the machine".into()));
    }
    Ok(())
}

fn shown(r: &Registry) -> Value {
    let d = r.default_id();
    json!({"default": d, "providers": r.list().iter().map(|e| e.public(e.id == d)).collect::<Vec<_>>()})
}

/// Local model servers listed when they answer on loopback: (port, name). They are not saved; add one to keep it.
pub const LOCAL_ENDPOINTS: [(u16, &str); 2] = [(9292, "llama-swap"), (8398, "LastEngine")];

/// The ports among `ports` on 127.0.0.1 that accept a connection within 300 ms.
pub async fn reachable_ports(ports: &[u16]) -> Vec<u16> {
    let mut found = Vec::new();
    for &port in ports {
        let up = tokio::time::timeout(std::time::Duration::from_millis(300), tokio::net::TcpStream::connect(("127.0.0.1", port))).await;
        if matches!(up, Ok(Ok(_))) {
            found.push(port);
        }
    }
    found
}

async fn list<M: Model + 'static>(State(st): State<Arc<AppState<M>>>) -> Json<Value> {
    let r = registry(&st);
    let mut v = shown(&r);
    let known: Vec<String> = r.list().iter().map(|e| e.url.clone()).collect();
    let ports: Vec<u16> = LOCAL_ENDPOINTS.iter().map(|e| e.0).collect();
    for port in reachable_ports(&ports).await {
        let url = format!("http://127.0.0.1:{port}/v1");
        // a saved provider already at this port is listed once, as saved
        if known.iter().any(|u| u.contains(&format!(":{port}"))) {
            continue;
        }
        let name = LOCAL_ENDPOINTS.iter().find(|e| e.0 == port).map(|e| e.1).unwrap_or("local");
        let row = json!({"id": format!("detected-{port}"), "name": name, "preset": null, "url": url, "api": "openai", "trust": "local", "local": true, "detected": true, "key_saved": false, "default": false, "models": [], "enabled": true, "disabled_models": [], "default_model": null});
        if let Some(list) = v["providers"].as_array_mut() {
            list.push(row);
        }
    }
    Json(v)
}

#[derive(Deserialize)]
struct AddBody {
    #[serde(flatten)]
    p: NewProvider,
    key: Option<String>,
}

async fn add<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, via: Option<Extension<crate::web::Via>>, Json(b): Json<AddBody>) -> Result<Json<Value>, ApiError> {
    no_device(via)?;
    let r = registry(&st);
    let e = r.add(&b.p, b.key.as_deref()).map_err(bad)?;
    let _ = st.settings.store.log("provider_added", "providers", "user", &json!({"id": e.id, "url": e.url, "api": e.api, "trust": e.trust, "key": b.key.as_ref().map(|_| "saved")}));
    Ok(Json(json!({"added": e.public(r.default_id() == e.id)})))
}

async fn remove<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, via: Option<Extension<crate::web::Via>>, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    no_device(via)?;
    registry(&st).remove(&id).map_err(bad)?;
    let _ = st.settings.store.log("provider_removed", "providers", "user", &json!({"id": id}));
    Ok(Json(json!({"removed": id})))
}

async fn make_default<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, via: Option<Extension<crate::web::Via>>, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    no_device(via)?;
    registry(&st).set_default(&id).map_err(bad)?;
    let _ = st.settings.store.log("provider_default", "providers", "user", &json!({"id": id}));
    Ok(Json(json!({"default": id})))
}

async fn proposals<M: Model + 'static>(State(st): State<Arc<AppState<M>>>) -> Json<Value> {
    Json(json!({"proposals": registry(&st).proposals()}))
}

#[derive(Deserialize, Default)]
struct ApproveBody {
    /// Typed by the user in the app; the model never sees it.
    key: Option<String>,
}

async fn approve<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, via: Option<Extension<crate::web::Via>>, Path(id): Path<String>, body: Option<Json<ApproveBody>>) -> Result<Json<Value>, ApiError> {
    no_device(via)?;
    let key = body.and_then(|b| b.0.key);
    let r = registry(&st);
    let e = r.approve(&id, key.as_deref()).map_err(bad)?;
    let _ = st.settings.store.log("provider_added", "providers", "user", &json!({"id": e.id, "url": e.url, "from_proposal": id, "key": key.as_ref().map(|_| "saved")}));
    Ok(Json(json!({"added": e.public(r.default_id() == e.id)})))
}

async fn reject<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, via: Option<Extension<crate::web::Via>>, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    no_device(via)?;
    if !registry(&st).reject(&id) {
        return Err(ApiError(StatusCode::NOT_FOUND, "No such proposal.".into()));
    }
    let _ = st.settings.store.log("provider_proposal_rejected", "providers", "user", &json!({"id": id}));
    Ok(Json(json!({"rejected": id})))
}

#[derive(Deserialize)]
struct EditBody {
    name: Option<String>,
    url: Option<String>,
    api: Option<String>,
    trust: Option<String>,
    /// Absent: keep the saved key. A typed key replaces it. `remove_key: true` deletes it.
    key: Option<String>,
    #[serde(default)]
    remove_key: bool,
    /// Off: the provider's models are not routed. Absent: keep as is.
    enabled: Option<bool>,
}

async fn edit<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, via: Option<Extension<crate::web::Via>>, Path(id): Path<String>, Json(b): Json<EditBody>) -> Result<Json<Value>, ApiError> {
    no_device(via)?;
    let key = match (b.remove_key, b.key.filter(|k| !k.trim().is_empty())) {
        (true, _) => KeyChange::Remove,
        (false, Some(k)) => KeyChange::Replace(k),
        (false, None) => KeyChange::Keep,
    };
    let key_word = match key {
        KeyChange::Keep => None,
        KeyChange::Replace(_) => Some("saved"),
        KeyChange::Remove => Some("removed"),
    };
    let r = registry(&st);
    let e = r.update(&id, &Update { name: b.name, url: b.url, api: b.api, trust: b.trust, key: Some(key), enabled: b.enabled }).map_err(bad)?;
    let _ = st.settings.store.log("provider_edited", "providers", "user", &json!({"id": e.id, "url": e.url, "api": e.api, "trust": e.trust, "key": key_word}));
    Ok(Json(json!({"provider": e.public(r.default_id() == e.id)})))
}

#[derive(Deserialize, Default)]
struct TestEntryBody {
    /// A key typed but not saved yet. Without it the entry's saved key is used.
    key: Option<String>,
}

async fn test_entry<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, via: Option<Extension<crate::web::Via>>, Path(id): Path<String>, body: Option<Json<TestEntryBody>>) -> Result<Json<Value>, ApiError> {
    no_device(via)?;
    let r = registry(&st);
    let e = r.get(&id).ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "No such provider.".into()))?;
    let key = body.and_then(|b| b.0.key).map(|k| k.trim().to_string()).filter(|k| !k.is_empty()).or_else(|| r.key_for(&id));
    Ok(Json(crate::provider_api::probe(&e.url, key, &e.api).await))
}

async fn refresh_models<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, via: Option<Extension<crate::web::Via>>, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    no_device(via)?;
    let r = registry(&st);
    let e = r.get(&id).ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "No such provider.".into()))?;
    let c = r.client(&id).ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "No such provider.".into()))?;
    let models = match tokio::time::timeout(std::time::Duration::from_secs(5), c.models()).await {
        Ok(Ok(m)) => m,
        Ok(Err(err)) => return Ok(Json(json!({"ok": false, "message": err.explain(), "models": e.models}))),
        Err(_) => return Ok(Json(json!({"ok": false, "message": "The server did not answer in time.", "models": e.models}))),
    };
    r.set_models(&id, models.clone());
    let _ = st.settings.store.log("provider_models", "providers", "user", &json!({"id": id, "count": models.len()}));
    Ok(Json(json!({"ok": true, "message": format!("{} model{}.", models.len(), if models.len() == 1 { "" } else { "s" }), "models": models})))
}

#[derive(Deserialize)]
struct ToggleBody {
    model: String,
    enabled: bool,
}

async fn toggle_model<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, via: Option<Extension<crate::web::Via>>, Path(id): Path<String>, Json(b): Json<ToggleBody>) -> Result<Json<Value>, ApiError> {
    no_device(via)?;
    let r = registry(&st);
    let e = r.set_model_enabled(&id, &b.model, b.enabled).map_err(bad)?;
    let _ = st.settings.store.log("provider_model_toggled", "providers", "user", &json!({"id": id, "model": b.model, "enabled": b.enabled}));
    Ok(Json(json!({"provider": e.public(r.default_id() == e.id)})))
}

#[derive(Deserialize)]
struct DefaultModelBody {
    /// Null clears the default model.
    model: Option<String>,
}

async fn default_model<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, via: Option<Extension<crate::web::Via>>, Path(id): Path<String>, Json(b): Json<DefaultModelBody>) -> Result<Json<Value>, ApiError> {
    no_device(via)?;
    let r = registry(&st);
    let e = r.set_default_model(&id, b.model.as_deref()).map_err(bad)?;
    let _ = st.settings.store.log("provider_default_model", "providers", "user", &json!({"id": id, "model": b.model}));
    Ok(Json(json!({"provider": e.public(r.default_id() == e.id)})))
}

#[cfg(test)]
mod tests {
    use super::reachable_ports;

    #[tokio::test]
    async fn loopback_port_is_detected_only_when_something_listens() {
        // a fake server on a free loopback port answers; a port with nothing on it does not
        let fake = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let up = fake.local_addr().unwrap().port();
        let down = {
            let t = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            t.local_addr().unwrap().port()
        };
        assert_eq!(reachable_ports(&[up, down]).await, vec![up]);
        assert!(reachable_ports(&[]).await.is_empty());
    }
}
