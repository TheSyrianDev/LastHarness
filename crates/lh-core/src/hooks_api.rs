//! Hooks: commands run before or after a tool call, kept in `hooks.toml` beside the settings file (the same file
//! the desktop app edits). The core reads the file again for each new chat, so a change counts from the next chat.
//!
//! GET    /v1/hooks                  -> {file, before: [hook], after: [hook], error?}
//! POST   /v1/hooks                  {when: before|after, tools, run, secs} -> {before, after}
//! PUT    /v1/hooks/{when}/{index}   {tools, run, secs}                     -> {before, after}
//! DELETE /v1/hooks/{when}/{index}                                          -> {before, after}
//!
//! A hook is {tools: [names], run, timeout_secs}; no tools means every tool. Fields arrive as typed in the form:
//! tools as a comma list and secs as text, checked the same way the desktop app checks them.

use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::routing::{get, put};
use axum::{Json, Router};
use lh_tools::hooks::{from_form, Hook};
use lh_tools::{Hooks, Model};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::web::Via;
use crate::{ApiError, AppState};

pub(crate) fn routes<M: Model + 'static>() -> Router<Arc<AppState<M>>> {
    Router::new().route("/v1/hooks", get(list::<M>).post(add::<M>)).route("/v1/hooks/{when}/{index}", put(edit::<M>).delete(remove::<M>))
}

#[derive(Deserialize)]
struct Form {
    #[serde(default)]
    when: String,
    #[serde(default)]
    tools: String,
    #[serde(default)]
    run: String,
    #[serde(default)]
    secs: String,
}

fn file<M>(st: &AppState<M>) -> Result<PathBuf, ApiError> {
    let cfg = st.settings.config.clone().ok_or(ApiError(StatusCode::NOT_FOUND, "this core was started without a settings file".into()))?;
    Ok(cfg.with_file_name("hooks.toml"))
}

fn hook_json(h: &Hook) -> Value {
    json!({"tools": h.tools, "run": h.run, "timeout_secs": h.timeout.as_secs()})
}

fn view(h: &Hooks) -> Value {
    json!({"before": h.before.iter().map(hook_json).collect::<Vec<_>>(), "after": h.after.iter().map(hook_json).collect::<Vec<_>>()})
}

fn bad(e: String) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, e)
}

fn list_of<'a>(h: &'a mut Hooks, when: &str) -> Result<&'a mut Vec<Hook>, ApiError> {
    match when {
        "before" => Ok(&mut h.before),
        "after" => Ok(&mut h.after),
        _ => Err(bad("`when` is before or after".into())),
    }
}

/// Load, change, save and log. A file that does not read is never overwritten.
fn change<M>(st: &AppState<M>, via: Option<Extension<Via>>, op: &str, when: &str, f: impl FnOnce(&mut Vec<Hook>) -> Result<(), ApiError>) -> Result<Json<Value>, ApiError> {
    let p = file(st)?;
    let mut h = Hooks::load(&p).map_err(|e| ApiError(StatusCode::CONFLICT, format!("the hooks file does not read, so it was left alone: {e}")))?;
    f(list_of(&mut h, when)?)?;
    h.save(&p).map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let device = match via.map(|v| v.0) {
        Some(Via::Device(d)) => d,
        _ => "local".into(),
    };
    let _ = st.settings.store.log("hooks", when, "user", &json!({"op": op, "device": device}));
    Ok(Json(view(&h)))
}

async fn list<M: Model + 'static>(State(st): State<Arc<AppState<M>>>) -> Result<Json<Value>, ApiError> {
    let p = file(&st)?;
    Ok(Json(match Hooks::load(&p) {
        Ok(h) => {
            let mut v = view(&h);
            v["file"] = json!(p);
            v
        }
        Err(e) => json!({"file": p, "before": [], "after": [], "error": e}),
    }))
}

async fn add<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, via: Option<Extension<Via>>, Json(b): Json<Form>) -> Result<Json<Value>, ApiError> {
    let hook = from_form(&b.tools, &b.run, &b.secs).map_err(bad)?;
    change(&st, via, "added", &b.when, |l| {
        l.push(hook);
        Ok(())
    })
}

async fn edit<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, via: Option<Extension<Via>>, Path((when, index)): Path<(String, usize)>, Json(b): Json<Form>) -> Result<Json<Value>, ApiError> {
    let hook = from_form(&b.tools, &b.run, &b.secs).map_err(bad)?;
    change(&st, via, "edited", &when, |l| {
        let slot = l.get_mut(index).ok_or(ApiError(StatusCode::NOT_FOUND, format!("no {when} hook {index}")))?;
        *slot = hook;
        Ok(())
    })
}

async fn remove<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, via: Option<Extension<Via>>, Path((when, index)): Path<(String, usize)>) -> Result<Json<Value>, ApiError> {
    change(&st, via, "removed", &when, |l| {
        if index >= l.len() {
            return Err(ApiError(StatusCode::NOT_FOUND, format!("no {when} hook {index}")));
        }
        l.remove(index);
        Ok(())
    })
}
