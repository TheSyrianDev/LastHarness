//! `/v1/models/search` and `/v1/models/installed`: the Models page's search by fit and the local model list.
//! Both are read only. Search needs the `hf-catalog` build; without it, or when the Hub cannot be reached,
//! the answer says so in words and the groups are empty.

use std::sync::Arc;

use axum::extract::Query;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use lh_tools::Model;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::models_catalog;
#[cfg(feature = "hf-catalog")]
use crate::models_catalog::FitHit;
use crate::{ApiError, AppState};
use std::sync::{Mutex, OnceLock};

pub fn routes<M: Model + 'static>() -> Router<Arc<AppState<M>>> {
    Router::new()
        .route("/v1/models/search", get(search))
        .route("/v1/models/installed", get(installed))
        .route("/v1/models/download", post(start_download))
        .route("/v1/models/downloads", get(list_downloads))
        .route("/v1/models/run", post(run_model))
        .route("/v1/models/delete", post(delete_model))
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
        "best_files": h.best_files,
        "other_quants": h.other_quants.iter().map(|(q, b, f)| json!({ "quant": q, "bytes": b, "fit": f.label() })).collect::<Vec<_>>(),
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
                .map(|m| json!({ "id": m.id, "file": m.file, "size_bytes": m.size_bytes, "kind": m.kind, "ctx_max": m.ctx_max, "registered": router_config_has(m.file.rsplit('/').next().unwrap_or("")) }))
                .collect();
            Json(json!({ "models": rows, "message": shown }))
        }
        Err(e) => Json(json!({ "models": [], "message": if path.exists() { e.to_string() } else { shown } })),
    }
}

/// One file download started from the Models page. Kept in memory for this run of the core.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Job {
    pub id: u64,
    pub repo: String,
    pub file: String,
    pub dest: String,
    /// `running`, `done`, `cancelled` or `failed`.
    pub state: String,
    pub downloaded: u64,
    pub total: u64,
    pub message: String,
}

fn jobs() -> &'static Mutex<Vec<Job>> {
    static JOBS: OnceLock<Mutex<Vec<Job>>> = OnceLock::new();
    JOBS.get_or_init(|| Mutex::new(Vec::new()))
}

#[derive(Deserialize)]
struct DownloadBody {
    repo: String,
    file: String,
}

/// A safe Hub repo id (`owner/name`) and a plain GGUF file path inside it. No `..`, no absolute path.
fn valid_target(repo: &str, file: &str) -> bool {
    let ok_char = |c: char| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.');
    let parts: Vec<&str> = repo.split('/').collect();
    let repo_ok = parts.len() == 2 && parts.iter().all(|p| !p.is_empty() && p.chars().all(ok_char) && !p.starts_with('.'));
    let file_ok = file.ends_with(".gguf") && !file.starts_with('/') && !file.split('/').any(|seg| seg.is_empty() || seg == "..") && file.chars().all(|c| ok_char(c) || c == '/');
    repo_ok && file_ok
}

/// Where a repo's files go: `~/AI/Models/<repo name in lowercase>/`.
fn dest_dir(repo: &str) -> Option<std::path::PathBuf> {
    let home = std::env::var_os("HOME")?;
    let name = repo.rsplit('/').next()?.to_lowercase();
    Some(std::path::PathBuf::from(home).join("AI/Models").join(name))
}

#[cfg(feature = "hf-catalog")]
async fn start_download(via: Option<axum::Extension<crate::web::Via>>, Json(b): Json<DownloadBody>) -> Result<Json<Value>, ApiError> {
    crate::registry_api::no_device(via)?;
    if !valid_target(&b.repo, &b.file) {
        return Err(ApiError(StatusCode::BAD_REQUEST, "that is not a GGUF file in a Hub repo".into()));
    }
    let dir = dest_dir(&b.repo).ok_or(ApiError(StatusCode::INTERNAL_SERVER_ERROR, "no home folder is set".into()))?;
    let id = {
        let mut list = jobs().lock().unwrap();
        let id = list.len() as u64 + 1;
        list.push(Job { id, repo: b.repo.clone(), file: b.file.clone(), dest: dir.display().to_string(), state: "running".into(), downloaded: 0, total: 0, message: String::new() });
        id
    };
    let (repo, file) = (b.repo.clone(), b.file.clone());
    std::thread::spawn(move || {
        let update = move |f: &dyn Fn(&mut Job)| {
            if let Some(j) = jobs().lock().unwrap().iter_mut().find(|j| j.id == id) {
                f(j);
            }
        };
        let _ = std::fs::create_dir_all(&dir);
        let sink: hfd_core::EventSink = std::sync::Arc::new(move |ev: hfd_core::Event| {
            if let hfd_core::Event::Progress { downloaded, total, .. } = ev {
                update(&|j: &mut Job| {
                    j.downloaded = downloaded;
                    j.total = total;
                });
            }
        });
        let cancel = hfd_core::CancelToken::new();
        let res = models_catalog::download_file(&repo, &file, &dir, &cancel, sink);
        if let Some(j) = jobs().lock().unwrap().iter_mut().find(|j| j.id == id) {
            match res {
                Ok(()) => {
                    j.state = "done".into();
                    j.downloaded = j.total;
                }
                Err(e) => {
                    j.state = "failed".into();
                    j.message = e.to_string();
                }
            }
        }
    });
    Ok(Json(json!({ "id": id, "state": "running" })))
}

#[cfg(not(feature = "hf-catalog"))]
async fn start_download(via: Option<axum::Extension<crate::web::Via>>, Json(_b): Json<DownloadBody>) -> Result<Json<Value>, ApiError> {
    crate::registry_api::no_device(via)?;
    Err(ApiError(StatusCode::NOT_IMPLEMENTED, "downloads are not in this build (it needs the hf-catalog feature)".into()))
}

async fn list_downloads() -> Json<Value> {
    Json(json!({ "downloads": *jobs().lock().unwrap() }))
}

/// Router files live in the Rig folder (`~/AI/Rig/router`). Missing HOME means no router.
fn rig_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join("AI/Rig"))
}

fn router_config_has(file: &str) -> bool {
    rig_dir()
        .and_then(|r| std::fs::read_to_string(r.join("router/config.yaml")).ok())
        .map(|c| crate::model_run::is_registered(&c, file))
        .unwrap_or(false)
}

#[derive(Deserialize)]
struct DeleteBody {
    folder: String,
    file: String,
}

/// Delete one installed GGUF under `~/AI/Models/<folder>/`. Refused while the file is registered with llama-swap,
/// so a running or configured model is never removed from under the router. Plain names only.
async fn delete_model(via: Option<axum::Extension<crate::web::Via>>, Json(b): Json<DeleteBody>) -> Result<Json<Value>, ApiError> {
    crate::registry_api::no_device(via)?;
    let models = std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join("AI/Models"))
        .ok_or(ApiError(StatusCode::INTERNAL_SERVER_ERROR, "no home folder is set".into()))?;
    if router_config_has(&b.file) {
        return Err(ApiError(StatusCode::CONFLICT, "this model is registered with llama-swap; remove it from the router first".into()));
    }
    remove_gguf(&models, &b.folder, &b.file).map_err(|m| ApiError(StatusCode::BAD_REQUEST, m))?;
    Ok(Json(json!({ "deleted": b.file })))
}

/// Removes `<models>/<folder>/<file>`. Both names must be plain and the file a `.gguf`.
fn remove_gguf(models: &std::path::Path, folder: &str, file: &str) -> Result<(), String> {
    let plain = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if !plain(folder) || !plain(file) || !file.ends_with(".gguf") {
        return Err("folder and file must be plain names, file a .gguf".into());
    }
    let path = models.join(folder).join(file);
    std::fs::remove_file(&path).map_err(|e| format!("could not delete {}: {e}", path.display()))
}

#[cfg(test)]
mod delete_tests {
    use super::remove_gguf;

    #[test]
    fn deletes_only_a_plain_gguf_inside_the_models_folder() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("m")).unwrap();
        std::fs::write(d.path().join("m/a.gguf"), b"x").unwrap();
        std::fs::write(d.path().join("m/a.txt"), b"x").unwrap();
        assert!(remove_gguf(d.path(), "m", "../m/a.gguf").is_err());
        assert!(remove_gguf(d.path(), "m", "a.txt").is_err());
        assert!(d.path().join("m/a.gguf").exists() && d.path().join("m/a.txt").exists());
        remove_gguf(d.path(), "m", "a.gguf").unwrap();
        assert!(!d.path().join("m/a.gguf").exists());
        assert!(remove_gguf(d.path(), "m", "a.gguf").is_err(), "missing file is an error");
    }
}

#[derive(Deserialize)]
struct RunBody {
    folder: String,
    file: String,
}

/// Register a downloaded GGUF with llama-swap: the steps in `model_run::run` (lock, backup, matrix, config,
/// validate, restart, list). Runs on a blocking thread because it restarts a service and waits.
async fn run_model(via: Option<axum::Extension<crate::web::Via>>, Json(b): Json<RunBody>) -> Result<Json<Value>, ApiError> {
    crate::registry_api::no_device(via)?;
    let plain = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if !plain(&b.folder) || !plain(&b.file) || !b.file.ends_with(".gguf") {
        return Err(ApiError(StatusCode::BAD_REQUEST, "folder and file must be plain names, file a .gguf".into()));
    }
    let rig = rig_dir().ok_or(ApiError(StatusCode::INTERNAL_SERVER_ERROR, "no home folder is set".into()))?;
    let lock = std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".machine-busy")).unwrap_or_default();
    let ctx = 32768;
    let id = crate::model_run::default_id(&b.file);
    let plan = crate::model_run::RunPlan { id, folder: b.folder.clone(), file: b.file.clone(), ctx };
    let res = tokio::task::spawn_blocking(move || crate::model_run::run(&rig, &lock, &plan, &crate::model_run::SystemSteps))
        .await
        .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    match res {
        Ok(rep) => Ok(Json(json!({ "id": rep.id, "backup": rep.backup.display().to_string(), "listed": true }))),
        Err(e) => Err(ApiError(StatusCode::CONFLICT, e)),
    }
}
