//! Scheduled tasks: list, add, turn on or off, remove, and the runner that starts due tasks.
//!
//! A task runs as an ordinary chat in its folder, started through this core's own local API like any other
//! client, so it is stored and can be opened in the app afterwards. Nobody watches it, so every permission
//! question is answered no; the permission settings decide what it may do without asking.

use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use lh_store::schedules::{local_offset, Schedule, When};
use lh_tools::Model;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::channels::Api;
use crate::{ApiError, AppState};

fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

pub(crate) fn routes<M: Model + 'static>() -> Router<Arc<AppState<M>>> {
    Router::new()
        .route("/v1/schedules", get(list::<M>).post(add::<M>))
        .route("/v1/schedules/{id}", axum::routing::delete(remove::<M>))
        .route("/v1/schedules/{id}/enabled", post(enable::<M>))
}

fn db(e: lh_store::Error) -> ApiError {
    ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

async fn list<M: Model + 'static>(State(st): State<Arc<AppState<M>>>) -> Result<Json<Value>, ApiError> {
    let all = st.settings.store.schedules().map_err(db)?;
    Ok(Json(json!({"schedules": all.iter().map(Schedule::json).collect::<Vec<_>>()})))
}

#[derive(Deserialize)]
struct NewSchedule {
    name: String,
    prompt: String,
    folder: String,
    model: String,
    when: String,
}

async fn add<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Json(b): Json<NewSchedule>) -> Result<Json<Value>, ApiError> {
    let bad = |m: String| ApiError(StatusCode::BAD_REQUEST, m);
    let when = When::parse(&b.when).map_err(bad)?;
    if b.prompt.trim().is_empty() {
        return Err(bad("write what the task should do".into()));
    }
    if b.model.trim().is_empty() {
        return Err(bad("choose a model for the task".into()));
    }
    if !std::path::Path::new(&b.folder).is_dir() {
        return Err(bad(format!("the folder {} does not exist", b.folder)));
    }
    let name = if b.name.trim().is_empty() { b.prompt.chars().take(40).collect() } else { b.name.trim().to_string() };
    let id = st.settings.store.add_schedule(&name, b.prompt.trim(), &b.folder, b.model.trim(), &when, now(), local_offset()).map_err(db)?;
    let _ = st.settings.store.log("schedule_added", "-", "user", &json!({"id": id, "name": name, "when": when.text(), "folder": b.folder}));
    let s = st.settings.store.schedule(id).map_err(db)?.expect("just added");
    Ok(Json(s.json()))
}

async fn remove<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<i64>) -> Result<StatusCode, ApiError> {
    match st.settings.store.remove_schedule(id).map_err(db)? {
        true => {
            let _ = st.settings.store.log("schedule_removed", "-", "user", &json!({"id": id}));
            Ok(StatusCode::NO_CONTENT)
        }
        false => Err(ApiError(StatusCode::NOT_FOUND, "no such task".into())),
    }
}

#[derive(Deserialize)]
struct Enabled {
    enabled: bool,
}

async fn enable<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<i64>, Json(b): Json<Enabled>) -> Result<Json<Value>, ApiError> {
    if !st.settings.store.enable_schedule(id, b.enabled, now(), local_offset()).map_err(db)? {
        return Err(ApiError(StatusCode::NOT_FOUND, "no such task".into()));
    }
    Ok(Json(st.settings.store.schedule(id).map_err(db)?.expect("exists").json()))
}

/// Look for due tasks every half minute and run each one in turn.
pub fn spawn_runner(store: Arc<lh_store::Store>, base: String, token: String) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            run_due(&store, &base, &token).await;
        }
    });
}

/// Run every task that is due now, one after another. Returns how many ran.
pub async fn run_due(store: &lh_store::Store, base: &str, token: &str) -> usize {
    let api = Arc::new(Api { base: base.to_string(), token: token.to_string(), http: reqwest::Client::new() });
    let due = match store.take_due(now(), local_offset()) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("lhd: cannot read scheduled tasks: {e}");
            return 0;
        }
    };
    for s in &due {
        let start = store.last_seq().unwrap_or(0);
        let (result, session) = run_one(&api, s).await;
        let _ = store.finish_schedule(s.id, &result);
        let _ = store.log("schedule_ran", "-", "lhd", &json!({"id": s.id, "name": s.name, "result": result}));
        // a notice the app can show: this task changed files while nobody watched
        let files = session.and_then(|sid| store.files_written(&sid, start).ok().map(|f| (sid, f)));
        if let Some((sid, files)) = files.filter(|(_, f)| !f.is_empty()) {
            let _ = store.log("schedule_changed_files", &sid, "lhd", &json!({"id": s.id, "name": s.name, "files": files}));
        }
    }
    due.len()
}

/// Run one task as a chat and say how it went: "ok", "failed: …", or "ok, but said no to …". Also gives the chat's id.
async fn run_one(api: &Arc<Api>, s: &Schedule) -> (String, Option<String>) {
    let v = match api.call(reqwest::Method::POST, "/v1/sessions", Some(json!({"model": s.model, "project_dir": s.folder}))).await {
        Ok(v) => v,
        Err(e) => return (format!("failed: {e}"), None),
    };
    let Some(session) = v["id"].as_str().map(str::to_string) else { return ("failed: the core gave no chat".into(), None) };
    let (mut end, mut refused) = (String::from("failed: the turn ended without an answer"), Vec::<String>::new());
    let text = format!("(Scheduled task \"{}\". Nobody is watching; do what you can without asking.)\n\n{}", s.name, s.prompt);
    let r = api
        .stream(reqwest::Method::POST, &format!("/v1/sessions/{session}/turns"), Some(json!({"text": text})), |ev| match ev["type"].as_str().unwrap_or("") {
            "ask" => {
                refused.push(ev["tool"].as_str().unwrap_or("a tool").to_string());
                // answer at once, so the turn goes on without waiting for the question to time out
                let (api, session, ask) = (api.clone(), session.clone(), ev["id"].as_str().unwrap_or("").to_string());
                tokio::spawn(async move {
                    let _ = api.call(reqwest::Method::POST, &format!("/v1/sessions/{session}/answers/{ask}"), Some(json!({"answer": "no"}))).await;
                });
            }
            "done" => end = "ok".into(),
            "error" | "refused" => end = format!("failed: {}", ev["message"].as_str().unwrap_or("")),
            "step_limit" => end = "failed: it took more steps than one turn allows".into(),
            _ => {}
        })
        .await;
    if let Err(e) = r {
        end = format!("failed: {e}");
    }
    let _ = api.call(reqwest::Method::DELETE, &format!("/v1/sessions/{session}"), None).await;
    refused.dedup();
    let said = match (end.as_str(), refused.is_empty()) {
        ("ok", false) => format!("ok, but said no to {}", refused.join(", ")),
        _ => end,
    };
    (said, Some(session))
}
