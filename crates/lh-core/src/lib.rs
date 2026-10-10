//! The core's local API: plain HTTP on localhost, server-sent events (SSE) for streaming.
//!
//! ```text
//! POST   /v1/sessions                       {model, project_dir? | mode?: chat|assistant, system?, ask_all?, max_tokens?,
//!                                            sampling?: {temperature, top_p, top_k, min_p, repeat_penalty, extra}}  -> {id}
//! GET    /v1/sessions/{id}                  -> {busy, messages}
//! DELETE /v1/sessions/{id}
//! POST   /v1/sessions/{id}/turns            {text}  -> SSE: reasoning, text, tool_start, ask, tool_result,
//!                                                          usage, finish, done | step_limit | refused | error
//! POST   /v1/sessions/{id}/answers/{ask}    {answer: "yes"|"no"}
//! POST   /v1/sessions/{id}/cancel
//! GET    /v1/activity?limit=&kind=          -> {entries: [...]}  newest first
//! POST   /v1/activity/{seq}/undo            -> {result: restored|removed|refused, message}
//! GET    /v1/capabilities?model=NAME        -> {context_tokens, tools, vision, reasoning_effort, source}
//! GET    /v1/models                         -> {models: [name, ...]}
//! GET    /v1/status                         -> {idle_before_request, sessions}
//! GET    /v1/health
//! ```
//! Every request needs `Authorization: Bearer <token>` and a loopback `Host` header.

pub mod auth;
pub mod idle;
mod session;

use std::collections::HashMap;
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event as Sse, KeepAlive, Sse as SseResponse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::stream::{self, Stream};
use lh_provider::{ChatRequest, Event, FinishReason, Message, Role};
use lh_tools::{compact_if_needed, default_tools, run_turn, CompactOptions, Answer, Mode, Model, Outcome, Permissions, TurnEnd, TurnEvent, TurnOptions};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::mpsc;

use session::{ApiAsker, Session};

pub struct Settings {
    /// Model used by the `generate_image` tool. `None` switches image generation off.
    pub image_model: Option<String>,
    /// Web search and fetch settings.
    pub web: lh_tools::WebConfig,
    /// Go idle after this long without activity. Zero means never.
    pub idle_after: std::time::Duration,
    /// Compact the conversation at about this many tokens; 0 turns compaction off. `None` means
    /// three quarters of the model's context size as the server reports it (24,000 if it does not).
    pub compact_at: Option<u64>,
    /// Local store: activity log and spend ledger.
    pub store: Arc<lh_store::Store>,
    /// Checks and records the cost of every model call.
    pub spend: lh_spend::SpendMeter,
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
    pub idle: Arc<idle::Idle>,
}

impl<M: Model + 'static> AppState<M> {
    /// Builds the state and starts the idle manager. Must be called inside a Tokio runtime.
    pub fn new(model: M, settings: Settings) -> Arc<Self> {
        let model = Arc::new(model);
        let idle = idle::Idle::new(settings.idle_after);
        {
            let (model, store) = (model.clone(), settings.store.clone());
            idle.on_change(move |going_idle| {
                if going_idle {
                    model.release_idle();
                    store.shrink();
                    idle::trim_memory();
                }
            });
        }
        tokio::spawn(idle.clone().run());
        Arc::new(Self { model, settings, sessions: Mutex::new(HashMap::new()), next: AtomicU64::new(1), idle })
    }

    fn session(&self, id: &str) -> Result<Arc<Session>, ApiError> {
        self.sessions.lock().unwrap().get(id).cloned().ok_or(ApiError(StatusCode::NOT_FOUND, "no such session".into()))
    }
}

/// `port` is the port the listener is bound to; it decides which `Host` headers are accepted.
pub fn router<M: Model + 'static>(state: Arc<AppState<M>>, token: String, port: u16) -> Router {
    let idle = state.idle.clone();
    let guard = Arc::new(auth::Guard { token, hosts: vec![format!("127.0.0.1:{port}"), format!("localhost:{port}"), format!("[::1]:{port}")] });
    Router::new()
        .route("/v1/health", get(|| async { Json(json!({"ok": true})) }))
        .route("/v1/models", get(list_models::<M>))
        .route("/v1/capabilities", get(capabilities::<M>))
        .route("/v1/activity", get(activity::<M>))
        .route("/v1/activity/{seq}/undo", post(undo::<M>))
        .route("/v1/sessions", post(create_session::<M>))
        .route("/v1/sessions/{id}", get(get_session::<M>).delete(delete_session::<M>))
        .route("/v1/sessions/{id}/turns", post(start_turn::<M>))
        .route("/v1/sessions/{id}/answers/{ask}", post(answer::<M>))
        .route("/v1/sessions/{id}/cancel", post(cancel::<M>))
        .route("/v1/status", get(status::<M>))
        .layer(axum::middleware::from_fn_with_state(guard, auth::require))
        .layer(axum::middleware::from_fn(move |req: axum::extract::Request, next: axum::middleware::Next| {
            let idle = idle.clone();
            async move {
                // Only a request that passed the token check counts as activity: this layer sits outside
                // the auth layer, so it must not wake the core for strangers. See `auth::require`.
                let r = next.run(req).await;
                if r.status() != StatusCode::UNAUTHORIZED && r.status() != StatusCode::FORBIDDEN {
                    idle.touch();
                }
                r
            }
        }))
        .with_state(state)
}

struct ApiError(StatusCode, String);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({"error": self.1}))).into_response()
    }
}

async fn status<M: Model + 'static>(State(st): State<Arc<AppState<M>>>) -> Json<Value> {
    Json(json!({"sessions": st.sessions.lock().unwrap().len(), "idle_after_secs": st.settings.idle_after.as_secs()}))
}

async fn list_models<M: Model + 'static>(State(st): State<Arc<AppState<M>>>) -> Result<Json<Value>, ApiError> {
    let models = st.model.models().await.map_err(|e| ApiError(StatusCode::BAD_GATEWAY, e.explain()))?;
    Ok(Json(json!({"models": models})))
}

#[derive(Deserialize)]
struct NewSession {
    model: String,
    /// A project is a folder. Without one the session is folderless (see `mode`).
    project_dir: Option<PathBuf>,
    /// `chat` (no tools; the default without a folder) or `assistant` (works from the home folder
    /// under the same permissions). Ignored when `project_dir` is given.
    mode: Option<String>,
    system: Option<String>,
    #[serde(default)]
    ask_all: bool,
    /// Per-role sampling and template settings.
    sampling: Option<SamplingBody>,
    max_tokens: Option<u32>,
}

#[derive(Deserialize, Default)]
struct SamplingBody {
    temperature: Option<f32>,
    top_p: Option<f32>,
    top_k: Option<u32>,
    min_p: Option<f32>,
    repeat_penalty: Option<f32>,
    #[serde(default)]
    extra: serde_json::Map<String, Value>,
}

async fn create_session<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Json(b): Json<NewSession>) -> Result<Json<Value>, ApiError> {
    let (dir, tools_on) = match (&b.project_dir, b.mode.as_deref()) {
        (Some(d), _) => (d.clone(), true),
        (None, Some("assistant")) => (std::env::var_os("HOME").map(PathBuf::from).ok_or(ApiError(StatusCode::BAD_REQUEST, "no home folder".into()))?, true),
        (None, None | Some("chat")) => (std::env::temp_dir(), false),
        (None, Some(other)) => return Err(ApiError(StatusCode::BAD_REQUEST, format!("mode must be \"chat\" or \"assistant\", not \"{other}\""))),
    };
    let mut perms = Permissions::new(&dir).map_err(|e| ApiError(StatusCode::BAD_REQUEST, format!("project_dir: {e}")))?;
    perms.sudo_enabled = st.settings.sudo_enabled;
    perms.allow_rules = st.settings.allow_rules.clone();
    perms.mode = if st.settings.bypass {
        Mode::Bypass
    } else if b.ask_all {
        Mode::AskAll
    } else {
        Mode::Tiered
    };
    let mut req = ChatRequest { model: b.model, max_tokens: b.max_tokens, ..Default::default() };
    if let Some(s) = b.sampling {
        req.sampling = lh_provider::Sampling { temperature: s.temperature, top_p: s.top_p, top_k: s.top_k, min_p: s.min_p, repeat_penalty: s.repeat_penalty, extra: s.extra };
    }
    if let Some(s) = b.system {
        req.messages.push(Message::system(s));
    }
    let id = format!("s{}", st.next.fetch_add(1, Ordering::SeqCst));
    st.sessions.lock().unwrap().insert(id.clone(), Arc::new(Session::new(req, perms, session_tools(&st.settings, tools_on))));
    Ok(Json(json!({"id": id})))
}

fn session_tools(s: &Settings, tools_on: bool) -> lh_tools::ToolSet {
    if !tools_on {
        return lh_tools::ToolSet::default(); // plain chat: the model is told about no tools
    }
    let t = default_tools().with_web(s.web.clone());
    match &s.image_model {
        Some(m) => t.with_images(m.clone()),
        None => t,
    }
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
    _busy: idle::Busy,
}
impl Drop for TurnGuard {
    fn drop(&mut self) {
        self.task.abort();
        self.session.finish_turn();
    }
}

/// The wire form of an event, or `None` for events only the activity log cares about.
fn describe(ev: &TurnEvent) -> Option<(String, Value)> {
    let ev = ev.clone();
    Some(match ev {
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
        TurnEvent::ToolResult { id, text, .. } => ("tool_result".into(), json!({"type":"tool_result","id":id,"text":text})),
        TurnEvent::Decision { .. } => return None,
    })
}

/// Writes what happened to the activity log. Failures are reported on stderr and never stop the turn.
struct Recorder {
    store: Arc<lh_store::Store>,
    session: String,
    calls: HashMap<String, (String, String)>, // id -> (tool, arguments)
}

impl Recorder {
    fn record(&mut self, ev: &TurnEvent) {
        let r = match ev {
            TurnEvent::ToolStart { id, name, arguments } => {
                self.calls.insert(id.clone(), (name.clone(), arguments.clone()));
                Ok(())
            }
            TurnEvent::Decision { tool, action, outcome, .. } => {
                let o = match outcome {
                    Outcome::Allowed => json!("allowed"),
                    Outcome::AllowedByUser => json!("allowed_by_user"),
                    Outcome::Declined => json!("declined_by_user"),
                    Outcome::Denied(why) => json!({"denied": why}),
                };
                self.store.log("permission", &self.session, "assistant", &json!({"tool": tool, "action": action, "outcome": o})).map(|_| ())
            }
            TurnEvent::ToolResult { id, text, undo } => {
                let (tool, args) = self.calls.remove(id).unwrap_or_default();
                let preview: String = text.chars().take(300).collect();
                let logged = self.store.log("tool_call", &self.session, "assistant", &json!({"tool": tool, "arguments": args, "result": preview}));
                match (logged, undo) {
                    (Ok(seq), Some(u)) => {
                        let after = std::fs::read(&u.path).map(|b| lh_store::content_hash(&b)).unwrap_or(0);
                        self.store.log_file_change(seq, &u.path, u.previous.as_deref(), after)
                    }
                    (Ok(_), None) => Ok(()),
                    (Err(e), _) => Err(e),
                }
            }
            TurnEvent::Model(_) => Ok(()),
        };
        if let Err(e) = r {
            eprintln!("lhd: could not write to the activity log: {e}");
        }
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
        let compact_at = st.settings.compact_at;
        let (s, model, tx, meter, store, id) = (s.clone(), st.model.clone(), tx.clone(), st.settings.spend.for_session(&id), st.settings.store.clone(), id.clone());
        tokio::spawn(async move {
            // Work on a copy; the session history only changes if the whole turn succeeds, so a
            // cancelled turn never leaves a tool call without its result.
            let mut work = s.req.lock().await.clone();
            work.messages.push(Message::user(b.text));
            let known = s.known_tokens.load(Ordering::SeqCst);
            let threshold = match compact_at {
                Some(n) => n,
                None => model.capabilities(&work.model).await.context_tokens.map_or(24_000, |n| n / 4 * 3),
            };
            let copts = CompactOptions { threshold_tokens: threshold, keep_last_turns: 1 };
            match compact_if_needed(&*model, &meter, &mut work, (known > 0).then_some(known), &copts).await {
                Ok(Some(c)) => {
                    s.known_tokens.store(0, Ordering::SeqCst);
                    let detail = json!({"type":"compacted","tokens_before":c.tokens_before,"tokens_after":c.tokens_after,"messages_summarized":c.messages_summarized});
                    let _ = store.log("compaction", &id, "assistant", &detail);
                    let _ = tx.send(("compacted".into(), detail));
                }
                Ok(None) => {}
                Err(e) => {
                    let _ = tx.send(("notice".into(), json!({"type":"notice","message": format!("The conversation could not be shortened: {e}")})));
                }
            }
            let asker = ApiAsker { session: s.clone(), events: tx.clone() };
            let opts = TurnOptions::default();
            let events = tx.clone();
            let mut rec = Recorder { store: store.clone(), session: id.clone(), calls: HashMap::new() };
            let end = run_turn(&*model, &meter, &mut work, &s.tools, &s.perms, &asker, &opts, move |e| {
                rec.record(&e);
                if let Some(msg) = describe(&e) {
                    let _ = events.send(msg);
                }
            })
            .await;
            let last = match end {
                Ok(TurnEnd::Done(r)) => {
                    let empty = r.text.is_empty();
                    if let Some(u) = r.usage {
                        s.known_tokens.store(u.prompt_tokens + u.completion_tokens, Ordering::SeqCst);
                    }
                    *s.req.lock().await = work;
                    ("done".to_string(), json!({"type":"done","empty":empty}))
                }
                Ok(TurnEnd::Refused(why)) => ("refused".into(), json!({"type":"refused","message":why})),
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
    let guard = TurnGuard { session: s, task, _busy: st.idle.busy() };
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

#[derive(Deserialize)]
struct ActivityQuery {
    limit: Option<u32>,
    kind: Option<String>,
}

async fn activity<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Query(q): Query<ActivityQuery>) -> Result<Json<Value>, ApiError> {
    let rows = st.settings.store.recent(q.limit.unwrap_or(50).min(500), q.kind.as_deref()).map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let entries: Vec<Value> = rows.iter().map(|e| json!({"seq": e.seq, "at": e.at, "kind": e.kind, "session": e.session, "actor": e.actor, "detail": e.detail})).collect();
    Ok(Json(json!({"entries": entries})))
}

async fn undo<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(seq): Path<i64>) -> Result<Json<Value>, ApiError> {
    use lh_store::UndoResult::*;
    let r = st.settings.store.undo(seq, "api").map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(match r {
        Restored => json!({"result": "restored", "message": "The file is back to how it was."}),
        Removed => json!({"result": "removed", "message": "The file the assistant created was removed."}),
        Refused(why) => json!({"result": "refused", "message": why}),
    }))
}

#[derive(Deserialize)]
struct CapQuery {
    model: String,
}

async fn capabilities<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Query(q): Query<CapQuery>) -> Json<Value> {
    let c = st.model.capabilities(&q.model).await;
    Json(json!({"context_tokens": c.context_tokens, "tools": c.tools, "vision": c.vision, "reasoning_effort": c.reasoning_effort, "source": c.source}))
}
