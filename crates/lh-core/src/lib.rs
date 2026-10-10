//! The core's local API: plain HTTP on localhost, server-sent events (SSE) for streaming.
//!
//! ```text
//! POST   /v1/sessions                       {model, project_dir, system?, ask_all?}  -> {id}
//! GET    /v1/sessions/{id}                  -> {busy, messages}
//! DELETE /v1/sessions/{id}
//! POST   /v1/sessions/{id}/turns            {text}  -> SSE: reasoning, text, tool_start, ask, tool_result,
//!                                                          usage, finish, done | step_limit | error
//! POST   /v1/sessions/{id}/answers/{ask}    {answer: "yes"|"no"}
//! POST   /v1/sessions/{id}/cancel
//! GET    /v1/models                         -> {models: [name, ...]}
//! GET    /v1/health
//! ```
//! Every request needs `Authorization: Bearer <token>` and a loopback `Host` header.

pub mod auth;
mod session;

use std::collections::HashMap;
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event as Sse, KeepAlive, Sse as SseResponse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::stream::{self, Stream};
use lh_provider::{ChatRequest, Event, FinishReason, Message, Role};
use lh_tools::{default_tools, run_turn, Answer, Mode, Model, Permissions, TurnEnd, TurnEvent, TurnOptions};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::mpsc;

use session::{ApiAsker, Session};

pub struct Settings {
    /// `--dangerously-skip-permissions`
    pub bypass: bool,
    pub sudo_enabled: bool,
    pub allow_rules: Vec<String>,
}

pub struct AppState<M> {
    model: Arc<M>,
    settings: Settings,
    sessions: Mutex<HashMap<String, Arc<Session>>>,
    next: AtomicU64,
}

impl<M: Model + 'static> AppState<M> {
    pub fn new(model: M, settings: Settings) -> Arc<Self> {
        Arc::new(Self { model: Arc::new(model), settings, sessions: Mutex::new(HashMap::new()), next: AtomicU64::new(1) })
    }

    fn session(&self, id: &str) -> Result<Arc<Session>, ApiError> {
        self.sessions.lock().unwrap().get(id).cloned().ok_or(ApiError(StatusCode::NOT_FOUND, "no such session".into()))
    }
}

/// `port` is the port the listener is bound to; it decides which `Host` headers are accepted.
pub fn router<M: Model + 'static>(state: Arc<AppState<M>>, token: String, port: u16) -> Router {
    let guard = Arc::new(auth::Guard { token, hosts: vec![format!("127.0.0.1:{port}"), format!("localhost:{port}"), format!("[::1]:{port}")] });
    Router::new()
        .route("/v1/health", get(|| async { Json(json!({"ok": true})) }))
        .route("/v1/models", get(list_models::<M>))
        .route("/v1/sessions", post(create_session::<M>))
        .route("/v1/sessions/{id}", get(get_session::<M>).delete(delete_session::<M>))
        .route("/v1/sessions/{id}/turns", post(start_turn::<M>))
        .route("/v1/sessions/{id}/answers/{ask}", post(answer::<M>))
        .route("/v1/sessions/{id}/cancel", post(cancel::<M>))
        .layer(axum::middleware::from_fn_with_state(guard, auth::require))
        .with_state(state)
}

struct ApiError(StatusCode, String);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({"error": self.1}))).into_response()
    }
}

async fn list_models<M: Model + 'static>(State(st): State<Arc<AppState<M>>>) -> Result<Json<Value>, ApiError> {
    let models = st.model.models().await.map_err(|e| ApiError(StatusCode::BAD_GATEWAY, e.explain()))?;
    Ok(Json(json!({"models": models})))
}

#[derive(Deserialize)]
struct NewSession {
    model: String,
    project_dir: PathBuf,
    system: Option<String>,
    #[serde(default)]
    ask_all: bool,
}

async fn create_session<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Json(b): Json<NewSession>) -> Result<Json<Value>, ApiError> {
    let mut perms = Permissions::new(&b.project_dir).map_err(|e| ApiError(StatusCode::BAD_REQUEST, format!("project_dir: {e}")))?;
    perms.sudo_enabled = st.settings.sudo_enabled;
    perms.allow_rules = st.settings.allow_rules.clone();
    perms.mode = if st.settings.bypass {
        Mode::Bypass
    } else if b.ask_all {
        Mode::AskAll
    } else {
        Mode::Tiered
    };
    let mut req = ChatRequest { model: b.model, ..Default::default() };
    if let Some(s) = b.system {
        req.messages.push(Message::system(s));
    }
    let id = format!("s{}", st.next.fetch_add(1, Ordering::SeqCst));
    st.sessions.lock().unwrap().insert(id.clone(), Arc::new(Session::new(req, perms, default_tools())));
    Ok(Json(json!({"id": id})))
}

async fn get_session<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    let s = st.session(&id)?;
    let req = s.req.lock().await;
    let messages: Vec<Value> = req
        .messages
        .iter()
        .map(|m| {
            let role = match m.role {
                Role::System => "system",
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::Tool => "tool",
            };
            json!({"role": role, "content": m.content, "tool_calls": m.tool_calls.iter().map(|c| json!({"id": c.id, "name": c.name, "arguments": c.arguments})).collect::<Vec<_>>()})
        })
        .collect();
    Ok(Json(json!({"busy": s.busy.load(Ordering::SeqCst), "messages": messages})))
}

async fn delete_session<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<String>) -> StatusCode {
    match st.sessions.lock().unwrap().remove(&id) {
        Some(_) => StatusCode::NO_CONTENT,
        None => StatusCode::NOT_FOUND,
    }
}

#[derive(Deserialize)]
struct NewTurn {
    text: String,
}

/// Ends the turn when the client goes away: stops the work and turns open questions into "no".
struct TurnGuard {
    session: Arc<Session>,
    task: tokio::task::AbortHandle,
}
impl Drop for TurnGuard {
    fn drop(&mut self) {
        self.task.abort();
        self.session.finish_turn();
    }
}

fn describe(ev: TurnEvent) -> (String, Value) {
    match ev {
        TurnEvent::Model(Event::Text(t)) => ("text".into(), json!({"type":"text","text":t})),
        TurnEvent::Model(Event::Reasoning(t)) => ("reasoning".into(), json!({"type":"reasoning","text":t})),
        TurnEvent::Model(Event::ToolCall(_)) => ("tool_call".into(), json!({"type":"tool_call"})),
        TurnEvent::Model(Event::Usage(u)) => (
            "usage".into(),
            json!({"type":"usage","prompt_tokens":u.prompt_tokens,"completion_tokens":u.completion_tokens,"cached_tokens":u.cached_tokens}),
        ),
        TurnEvent::Model(Event::Finish(f)) => {
            let reason = match f {
                FinishReason::Stop => "stop".to_string(),
                FinishReason::ToolCalls => "tool_calls".into(),
                FinishReason::Length => "length".into(),
                FinishReason::Other(o) => o,
            };
            ("finish".into(), json!({"type":"finish","reason":reason}))
        }
        TurnEvent::ToolStart { id, name, arguments } => ("tool_start".into(), json!({"type":"tool_start","id":id,"name":name,"arguments":arguments})),
        TurnEvent::ToolResult { id, text } => ("tool_result".into(), json!({"type":"tool_result","id":id,"text":text})),
    }
}

async fn start_turn<M: Model + 'static>(
    State(st): State<Arc<AppState<M>>>,
    Path(id): Path<String>,
    Json(b): Json<NewTurn>,
) -> Result<SseResponse<impl Stream<Item = Result<Sse, Infallible>>>, ApiError> {
    let s = st.session(&id)?;
    if !s.try_start() {
        return Err(ApiError(StatusCode::CONFLICT, "a turn is already running in this session".into()));
    }
    let (tx, rx) = mpsc::unbounded_channel::<(String, Value)>();
    let task = {
        let (s, model, tx) = (s.clone(), st.model.clone(), tx.clone());
        tokio::spawn(async move {
            // Work on a copy; the session history only changes if the whole turn succeeds, so a
            // cancelled turn never leaves a tool call without its result.
            let mut work = s.req.lock().await.clone();
            work.messages.push(Message::user(b.text));
            let asker = ApiAsker { session: s.clone(), events: tx.clone() };
            let opts = TurnOptions::default();
            let events = tx.clone();
            let end = run_turn(&*model, &mut work, &s.tools, &s.perms, &asker, &opts, move |e| {
                let _ = events.send(describe(e));
            })
            .await;
            let last = match end {
                Ok(TurnEnd::Done(r)) => {
                    let empty = r.text.is_empty();
                    *s.req.lock().await = work;
                    ("done".to_string(), json!({"type":"done","empty":empty}))
                }
                Ok(TurnEnd::StepLimit) => {
                    *s.req.lock().await = work;
                    ("step_limit".into(), json!({"type":"step_limit"}))
                }
                Err(e) => ("error".into(), json!({"type":"error","message":e.explain(),"detail":e.to_string(),"retryable":e.is_retryable()})),
            };
            let _ = tx.send(last);
        })
        .abort_handle()
    };
    drop(tx);
    let guard = TurnGuard { session: s, task };
    let out = stream::unfold((rx, guard), |(mut rx, g)| async move {
        let (name, data) = rx.recv().await?;
        Some((Ok(Sse::default().event(name).data(data.to_string())), (rx, g)))
    });
    Ok(SseResponse::new(out).keep_alive(KeepAlive::default()))
}

#[derive(Deserialize)]
struct Reply {
    answer: String,
}

async fn answer<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path((id, ask)): Path<(String, String)>, Json(b): Json<Reply>) -> Result<StatusCode, ApiError> {
    let a = match b.answer.as_str() {
        "yes" => Answer::Yes,
        "no" => Answer::No,
        _ => return Err(ApiError(StatusCode::BAD_REQUEST, "answer must be \"yes\" or \"no\"".into())),
    };
    if st.session(&id)?.answer(&ask, a) {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError(StatusCode::NOT_FOUND, "no such open question".into()))
    }
}

async fn cancel<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<String>) -> Result<StatusCode, ApiError> {
    // Dropping the client's stream does the work; this only frees questions that are waiting.
    st.session(&id)?.finish_turn();
    Ok(StatusCode::NO_CONTENT)
}
