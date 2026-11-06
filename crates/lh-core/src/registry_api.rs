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

use crate::registry::{NewProvider, Registry};
use crate::{ApiError, AppState};

pub fn routes<M: Model + 'static>() -> Router<Arc<AppState<M>>> {
    Router::new()
        .route("/v1/providers", get(list::<M>).post(add::<M>))
        .route("/v1/providers/proposals", get(proposals::<M>))
        .route("/v1/providers/proposals/{id}/approve", post(approve::<M>))
        .route("/v1/providers/proposals/{id}/reject", post(reject::<M>))
        .route("/v1/providers/{id}", delete(remove::<M>))
        .route("/v1/providers/{id}/default", post(make_default::<M>))
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

async fn list<M: Model + 'static>(State(st): State<Arc<AppState<M>>>) -> Json<Value> {
    Json(shown(&registry(&st)))
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
