//! The single settings surface: every behaviour choice, its value, what is allowed, and whether a change
//! applies now or at the next start. Values live in the settings file; the team settings also live in the
//! store (where the departments read them), and both are kept in step.

use std::sync::Arc;

use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use lh_config::{ConfigFile, Kind};
use lh_team::{Autonomy, TeamSettings, UnavailablePolicy};
use lh_tools::Model;
use serde_json::{json, Map, Value};

use crate::{ApiError, AppState};

pub(crate) fn routes<M: Model + 'static>() -> Router<Arc<AppState<M>>> {
    Router::new().route("/v1/settings", get(list::<M>).put(change::<M>)).route("/v1/reports/preview", get(report_preview::<M>))
}

/// Live settings that an open chat does not pick up, because its tools are built when it starts.
const NEW_CHATS_ONLY: &[&str] = &["web.searxng", "web.use_proxy", "media.image_model", "tools.codemode", "tools.code_lookup", "tools.generate_image"];

fn load<M>(st: &AppState<M>) -> Result<ConfigFile, ApiError> {
    let path = st.settings.config.clone().ok_or(ApiError(StatusCode::NOT_FOUND, "this core was started without a settings file".into()))?;
    ConfigFile::load(path).map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// The team settings the departments really use come from the store; the file may be behind.
fn team_value<M>(st: &AppState<M>, key: &str) -> Option<String> {
    let t = TeamSettings::new(&st.settings.store);
    Some(match key {
        "team.autonomy" => t.autonomy().ok()?.as_str().to_string(),
        "team.parallel" => t.parallel().ok()?.to_string(),
        "team.attempts" => t.attempts().ok()?.to_string(),
        "team.unavailable" => if t.unavailable().ok()? == UnavailablePolicy::Fallback { "fallback" } else { "stop" }.to_string(),
        _ => return None,
    })
}

fn kind_json(k: Kind) -> Value {
    match k {
        Kind::Bool => json!({"type": "bool"}),
        Kind::Int { min, max } => json!({"type": "int", "min": min, "max": max}),
        Kind::OptFloat => json!({"type": "number", "optional": true}),
        Kind::AutoInt => json!({"type": "auto-int"}),
        Kind::Choice(o) => json!({"type": "choice", "options": o}),
        Kind::Text => json!({"type": "text"}),
    }
}

async fn list<M: Model + 'static>(State(st): State<Arc<AppState<M>>>) -> Result<Json<Value>, ApiError> {
    let cfg = load(&st)?;
    let items: Vec<Value> = cfg.all().into_iter().map(|(d, value, set)| {
        let value = team_value(&st, d.key).unwrap_or(value);
        json!({"key": d.key, "group": d.group(), "value": value, "default": d.default, "is_set": set, "kind": kind_json(d.kind), "live": d.live, "help": d.help})
    }).collect();
    Ok(Json(json!({"file": cfg.path(), "settings": items})))
}

/// Change settings. Everything is checked first; if any value is wrong nothing is changed.
async fn change<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, via: Option<Extension<crate::web::Via>>, Json(b): Json<Map<String, Value>>) -> Result<Json<Value>, ApiError> {
    // A paired browser may change how the assistant behaves, not where it sends data, what it may run, or
    // how much it may spend. Those stay on the machine's own terminal.
    if matches!(via.map(|v| v.0), Some(crate::web::Via::Device(_))) {
        if let Some(k) = b.keys().find(|k| ["provider", "permissions", "web", "media", "spending"].iter().any(|g| k.split('.').next() == Some(g)) || k.as_str() == "memory.trust") {
            return Err(ApiError(StatusCode::FORBIDDEN, format!("{k} cannot be changed from a browser; use `lh settings` on the machine")));
        }
    }
    let mut cfg = load(&st)?;
    let mut changes = vec![];
    for (k, v) in &b {
        let raw = match v {
            Value::String(s) => s.clone(),
            Value::Null => String::new(),
            other => other.to_string(),
        };
        let norm = lh_config::normalize(k, &raw).map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
        changes.push((k.clone(), norm));
    }
    let t = TeamSettings::new(&st.settings.store);
    for (k, v) in &changes {
        match k.as_str() {
            "team.autonomy" => t.set_autonomy(Autonomy::parse(v).unwrap_or(Autonomy::PlanFirst)).map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?,
            "team.parallel" => t.set_parallel(v.parse().unwrap_or(1)).map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?,
            "team.attempts" => t.set_attempts(v.parse().unwrap_or(3)).map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?,
            "team.unavailable" => t.set_unavailable(if v == "fallback" { UnavailablePolicy::Fallback } else { UnavailablePolicy::Stop }).map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?,
            _ => {}
        }
        cfg.set(k, v).map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    }
    cfg.save().map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    // From here on the running core uses the new values (see `Live` for what reaches open chats).
    let keys: Vec<String> = changes.iter().map(|(k, _)| k.clone()).collect();
    st.apply_settings(&cfg, &keys);
    let restart: Vec<&str> = changes.iter().filter(|(k, _)| lh_config::def(k).is_some_and(|d| !d.live)).map(|(k, _)| k.as_str()).collect();
    let new_chats: Vec<&str> = changes.iter().filter(|(k, _)| NEW_CHATS_ONLY.contains(&k.as_str())).map(|(k, _)| k.as_str()).collect();
    for (k, v) in &changes {
        // permissions and spending changes are logged like any other, with when they take effect
        let applies = if restart.contains(&k.as_str()) { "after restart" } else if NEW_CHATS_ONLY.contains(&k.as_str()) { "new chats" } else { "now" };
        let _ = st.settings.store.log("setting_changed", k, "user", &json!({"key": k, "value": if k == "provider.key" { "(a secret name)" } else { v.as_str() }, "applies": applies}));
    }
    Ok(Json(json!({"changed": keys, "restart_needed": restart, "new_chats_only": new_chats})))
}

/// What an anonymous crash report would contain. Nothing is sent from here: this only shows it.
async fn report_preview<M: Model + 'static>(State(st): State<Arc<AppState<M>>>) -> Result<Json<Value>, ApiError> {
    let cfg = load(&st)?;
    let pending = cfg.path().parent().map(|p| p.join("crash-pending.json")).and_then(|p| std::fs::read_to_string(p).ok()).and_then(|t| serde_json::from_str::<Value>(&t).ok());
    Ok(Json(json!({
        "setting": cfg.get("reports.crash"),
        "pending": pending,
        "contains": ["app version", "operating system and architecture", "the place in the code where it stopped (file and line)"],
        "never_contains": ["messages, notes or memory", "file names or paths from your computer", "model names, keys or addresses"],
        "note": "Nothing is sent by this version: there is no place to send it to yet.",
    })))
}

/// Keep the settings file in step when the team settings are changed through the team endpoints.
pub(crate) fn mirror_team<M>(st: &AppState<M>, key: &str, value: &str) {
    if let Ok(mut cfg) = load(st) {
        if cfg.set(key, value).is_ok() {
            let _ = cfg.save();
        }
    }
}
