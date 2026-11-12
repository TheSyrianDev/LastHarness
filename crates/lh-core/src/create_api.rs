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

/// Z-Image needs about 6.7 GB free RAM, Wan about 8.5 GB (contract, Memory section).
const ZIMAGE_RAM: u64 = 6_700_000_000;
const WAN_RAM: u64 = 8_500_000_000;

#[derive(Deserialize)]
pub struct VideoBody {
    pub prompt: String,
    #[serde(default = "default_frames")]
    pub frames: u32,
    #[serde(default = "default_steps_video")]
    pub steps: u32,
}

fn default_frames() -> u32 {
    17
}
fn default_steps_video() -> u32 {
    20
}

pub fn routes<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route("/v1/create/image", post(start_image))
        .route("/v1/create/video", post(start_video))
        .route("/v1/create/jobs", get(list_jobs))
}

fn workflow_template(file: &str) -> Result<Value, ApiError> {
    let home = std::env::var_os("HOME").ok_or(ApiError(StatusCode::INTERNAL_SERVER_ERROR, "no home folder is set".into()))?;
    let path = std::path::PathBuf::from(home).join("AI/Rig/comfyui/workflows").join(file);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .ok_or(ApiError(StatusCode::INTERNAL_SERVER_ERROR, format!("cannot read the workflow {file}")))
}

fn new_job(prompt: &str) -> u64 {
    let mut list = jobs().lock().unwrap();
    let id = list.len() as u64 + 1;
    list.push(CreateJob { id, prompt: prompt.to_string(), state: "running".into(), files: vec![], message: String::new() });
    id
}

fn seed_now() -> u64 {
    create_job::fresh_seed(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(1))
}

/// Run a filled workflow on a thread and record the result in the job list.
fn spawn_job(id: u64, workflow: Value, output_node: &'static str, ram: u64, label: &'static str) {
    let Some(home) = std::env::var_os("HOME") else { return };
    let lock = std::path::PathBuf::from(&home).join("AI/.machine-busy");
    let out = create_job::output_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    std::thread::spawn(move || {
        let res = create_job::run_image_job(&lock, &workflow, output_node, ram, &ComfyHttp::local(), &out, label);
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
}

async fn start_video(Json(b): Json<VideoBody>) -> Result<Json<Value>, ApiError> {
    let prompt = b.prompt.trim().to_string();
    if prompt.is_empty() || prompt.len() > 2000 {
        return Err(ApiError(StatusCode::BAD_REQUEST, "write a prompt of 1 to 2000 characters".into()));
    }
    if b.frames % 4 != 1 || !(5..=49).contains(&b.frames) {
        return Err(ApiError(StatusCode::BAD_REQUEST, "frames are 5 to 49 and must be 4n+1 (17, 21, 25 ...)".into()));
    }
    if !(1..=30).contains(&b.steps) {
        return Err(ApiError(StatusCode::BAD_REQUEST, "steps are 1 to 30".into()));
    }
    let template = workflow_template("wan22-ti2v-5b-gguf.json")?;
    let workflow = create_job::video_workflow(&template, &prompt, "blurry, low quality", seed_now(), 832, 480, b.frames, b.steps, 5.0, "lastharness/wan22")
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, e))?;
    let id = new_job(&prompt);
    spawn_job(id, workflow, "12", WAN_RAM, "video");
    Ok(Json(json!({ "id": id, "state": "running" })))
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
    let template = workflow_template("z-image-turbo-gguf.json")?;
    let workflow = create_job::image_workflow(&template, &prompt, seed_now(), b.width, b.height, b.steps, 1.0, "lastharness/zimage")
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, e))?;
    let id = new_job(&prompt);
    spawn_job(id, workflow, "10", ZIMAGE_RAM, "image");
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
