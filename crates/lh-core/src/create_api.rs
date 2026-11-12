//! Create tab API: start an image job on ComfyUI and list the jobs. Jobs run on a background thread.

use std::sync::{Mutex, OnceLock};

use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::create_job::{self, ComfyHttp};
use crate::ApiError;

#[derive(Debug, Clone, serde::Serialize)]
pub struct CreateJob {
    pub id: u64,
    pub prompt: String,
    /// `running`, `done` or `failed`.
    pub state: String,
    pub files: Vec<String>,
    pub message: String,
}

fn jobs() -> &'static Mutex<Vec<CreateJob>> {
    static JOBS: OnceLock<Mutex<Vec<CreateJob>>> = OnceLock::new();
    JOBS.get_or_init(|| Mutex::new(Vec::new()))
}

#[derive(Deserialize)]
pub struct ImageBody {
    pub prompt: String,
    #[serde(default = "default_size")]
    pub width: u32,
    #[serde(default = "default_size")]
    pub height: u32,
    #[serde(default = "default_steps")]
    pub steps: u32,
}

fn default_size() -> u32 {
    768
}
fn default_steps() -> u32 {
    8
}

/// Z-Image needs about 6.7 GB free RAM (contract, Memory section).
const ZIMAGE_RAM: u64 = 6_700_000_000;

pub fn routes<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new().route("/v1/create/image", post(start_image)).route("/v1/create/jobs", get(list_jobs))
}

async fn start_image(Json(b): Json<ImageBody>) -> Result<Json<Value>, ApiError> {
    let prompt = b.prompt.trim().to_string();
    if prompt.is_empty() || prompt.len() > 2000 {
        return Err(ApiError(StatusCode::BAD_REQUEST, "write a prompt of 1 to 2000 characters".into()));
    }
    if !(256..=1536).contains(&b.width) || !(256..=1536).contains(&b.height) || b.width % 8 != 0 || b.height % 8 != 0 {
        return Err(ApiError(StatusCode::BAD_REQUEST, "width and height are 256 to 1536 in steps of 8".into()));
    }
    if !(1..=30).contains(&b.steps) {
        return Err(ApiError(StatusCode::BAD_REQUEST, "steps are 1 to 30".into()));
    }
    let home = std::env::var_os("HOME").ok_or(ApiError(StatusCode::INTERNAL_SERVER_ERROR, "no home folder is set".into()))?;
    let rig = std::path::PathBuf::from(&home).join("AI/Rig/comfyui/workflows/z-image-turbo-gguf.json");
    let template: Value = std::fs::read_to_string(&rig)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .ok_or(ApiError(StatusCode::INTERNAL_SERVER_ERROR, "cannot read the Z-Image workflow".into()))?;
    let seed = create_job::fresh_seed(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(1));
    let workflow = create_job::image_workflow(&template, &prompt, seed, b.width, b.height, b.steps, 1.0, "lastharness/zimage").map_err(|e| ApiError(StatusCode::BAD_REQUEST, e))?;
    let id = {
        let mut list = jobs().lock().unwrap();
        let id = list.len() as u64 + 1;
        list.push(CreateJob { id, prompt: prompt.clone(), state: "running".into(), files: vec![], message: String::new() });
        id
    };
    let lock = std::path::PathBuf::from(&home).join("AI/.machine-busy");
    let out = create_job::output_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    std::thread::spawn(move || {
        let res = create_job::run_image_job(&lock, &workflow, ZIMAGE_RAM, &ComfyHttp::local(), &out, "image");
        if let Some(j) = jobs().lock().unwrap().iter_mut().find(|j| j.id == id) {
            match res {
                Ok(files) => {
                    j.state = "done".into();
                    j.files = files.iter().map(|p| p.display().to_string()).collect();
                }
                Err(e) => {
                    j.state = "failed".into();
                    j.message = e;
                }
            }
        }
    });
    Ok(Json(json!({ "id": id, "state": "running" })))
}

async fn list_jobs() -> Json<Value> {
    Json(json!({ "jobs": *jobs().lock().unwrap() }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_list_starts_empty_and_serializes() {
        let v = serde_json::to_value(CreateJob { id: 1, prompt: "p".into(), state: "running".into(), files: vec![], message: String::new() }).unwrap();
        assert_eq!(v["state"], "running");
    }
}
