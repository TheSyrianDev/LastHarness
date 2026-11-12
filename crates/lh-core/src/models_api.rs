//! `/v1/models/search` and `/v1/models/installed`: the Models page's search by fit and the local model list.
//! Both are read only. Search needs the `hf-catalog` build; without it, or when the Hub cannot be reached,
//! the answer says so in words and the groups are empty.

use std::sync::Arc;

use axum::extract::Query;
use axum::routing::get;
use axum::{Json, Router};
use lh_tools::Model;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::models_catalog;
#[cfg(feature = "hf-catalog")]
use crate::models_catalog::FitHit;
use crate::AppState;

pub fn routes<M: Model + 'static>() -> Router<Arc<AppState<M>>> {
    Router::new().route("/v1/models/search", get(search)).route("/v1/models/installed", get(installed))
}

#[derive(Deserialize)]
struct SearchQ {
    #[serde(default)]
    q: String,
}

#[cfg(feature = "hf-catalog")]
/// Free RAM from `/proc/meminfo` (MemAvailable). 0 when it cannot be read, which makes every size "too big".
fn free_ram() -> u64 {
    std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|t| t.lines().find(|l| l.starts_with("MemAvailable:")).and_then(|l| l.split_whitespace().nth(1).and_then(|n| n.parse::<u64>().ok())))
        .map(|kb| kb * 1024)
        .unwrap_or(0)
}

#[cfg(feature = "hf-catalog")]
fn hit_json(h: &FitHit) -> Value {
    json!({
        "repo": h.repo,
        "name": h.name,
        "downloads": h.downloads,
        "fit": h.fit.label(),
        "best_quant": h.best_quant,
        "best_bytes": h.best_bytes,
    })
}

#[cfg(feature = "hf-catalog")]
fn group(v: &[FitHit]) -> Value {
    Value::Array(v.iter().map(hit_json).collect())
}

fn unavailable(msg: &str) -> Json<Value> {
    Json(json!({ "available": false, "message": msg, "fits": [], "tight": [], "too_big": [], "unknown": [] }))
}

async fn search(Query(p): Query<SearchQ>) -> Json<Value> {
    let q = p.q.trim().to_string();
    if q.is_empty() {
        return Json(json!({ "available": true, "message": "Type a model name to search the Hub.", "fits": [], "tight": [], "too_big": [], "unknown": [] }));
    }
    #[cfg(feature = "hf-catalog")]
    {
        let ram = free_ram();
        let res = tokio::task::spawn_blocking(move || -> Result<_, String> {
            let cat = models_catalog::HfdCatalog::from_env().map_err(|e| e.to_string())?;
            models_catalog::group_hits_by_fit(&cat, &q, ram, 0).map_err(|e| e.to_string())
        })
        .await;
        return match res {
            Ok(Ok(g)) => Json(json!({
                "available": true,
                "message": if g.fits.len() + g.tight.len() + g.too_big.len() + g.unknown.len() == 0 { "No GGUF models matched." } else { "" },
                "fits": group(&g.fits), "tight": group(&g.tight), "too_big": group(&g.too_big), "unknown": group(&g.unknown),
            })),
            Ok(Err(e)) => Json(json!({ "available": false, "message": format!("Search could not reach the Hub: {e}"), "fits": [], "tight": [], "too_big": [], "unknown": [] })),
            Err(_) => unavailable("Search stopped unexpectedly. Try again."),
        };
    }
    #[cfg(not(feature = "hf-catalog"))]
    {
        let _ = q;
        unavailable("Model search is not in this build (it needs the hf-catalog feature).")
    }
}

fn registry_path() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join("AI/Rig/registry/models.json"))
}

async fn installed() -> Json<Value> {
    let Some(path) = registry_path() else {
        return Json(json!({ "models": [], "message": "No home folder is set, so the local registry cannot be found." }));
    };
    let shown = if path.exists() { String::new() } else { "No local models are registered yet.".to_string() };
    match models_catalog::read_installed(&path) {
        Ok(list) => {
            let rows: Vec<Value> = list
                .iter()
                .map(|m| json!({ "id": m.id, "file": m.file, "size_bytes": m.size_bytes, "kind": m.kind, "ctx_max": m.ctx_max }))
                .collect();
            Json(json!({ "models": rows, "message": shown }))
        }
        Err(e) => Json(json!({ "models": [], "message": if path.exists() { e.to_string() } else { shown } })),
    }
}
