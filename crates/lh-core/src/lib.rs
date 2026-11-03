//! The core's local API: plain HTTP on localhost, server-sent events (SSE) for streaming.
//!
//! ```text
//! POST   /v1/sessions                       {model, project_dir? | mode?: chat|assistant, system?, ask_all?, max_tokens?,
//!                                            sampling?: {temperature, top_p, top_k, min_p, repeat_penalty, extra}}  -> {id}
//! GET    /v1/sessions/{id}                  -> {busy, messages}
//! DELETE /v1/sessions/{id}
//! POST   /v1/sessions/{id}/turns            {text}  -> SSE: reasoning, text, tool_start, ask, tool_result,
//!                                                          usage, finish, done | step_limit | refused | error
//! POST   /v1/sessions/{id}/model            {model}  switch model (the prompt is rebuilt; a cold read follows)
//! POST   /v1/sessions/{id}/answers/{ask}    {answer: "yes"|"no"}
//! POST   /v1/sessions/{id}/cancel
//! POST   /v1/sessions/{id}/undo_turn          -> {files: [{seq, path, result: restored|removed|refused, message?}]}
//! GET    /v1/activity?limit=&kind=          -> {entries: [...]}  newest first
//! GET    /v1/events                         -> SSE: app-wide news (`title` when a chat is named)
//! POST   /v1/activity/{seq}/undo            -> {result: restored|removed|refused, message}
//! GET    /v1/capabilities?model=NAME        -> {context_tokens, tools, vision, reasoning_effort, source}
//! GET/POST /v1/learn/...                    rules, style, skills, suggestions, signals, habits
//! POST   /v1/sessions/{id}/learn            -> {proposals, applied}  end-of-session learn pass
//! POST   /v1/sessions/{id}/feedback         {kind: up|down, note?}
//! GET    /v1/models                         -> {models: [name, ...]}
//! GET    /v1/status                         -> {idle_before_request, sessions}
//! GET    /v1/health
//! ```
//! Every request needs `Authorization: Bearer <token>` and a loopback `Host` header.

pub mod auth;
pub mod idle;
pub mod channels;
pub mod schedules_api;
mod rewards;
mod chats_api;
mod export_api;
pub mod kv_cache;
mod learn_api;
mod mcp_api;
mod memory_api;
pub mod provider_api;
mod session;
pub mod web;
mod settings_api;
mod show_tool;
mod team_api;
mod titles;
mod tree_api;

pub use learn_api::{LearnMode, LearnRuntime};
pub use mcp_api::McpRuntime;
pub use memory_api::MemoryRuntime;
pub use team_api::{Run, TeamRuntime};

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
    /// Ask for everything unless the client says otherwise (the permission mode setting).
    pub ask_all: bool,
    /// The settings file, so the settings screens can change it.
    pub config: Option<PathBuf>,
    /// MCP servers and folder trust. `None` switches them off.
    pub mcp: Option<McpRuntime>,
    /// Roles, departments and runs. `None` switches departments off.
    pub team: Option<TeamRuntime>,
    /// Name of the service behind the endpoint for the first line of the prompt. When unset, what the
    /// server reports is used, and if it reports nothing the part is left out.
    pub provider_name: Option<String>,
    /// Learning: rules, style, skills. `None` switches it off.
    pub learn: Option<LearnRuntime>,
    /// Run the memory maintenance pass (it only makes suggestions) when the core goes idle.
    pub maintain_when_idle: bool,
    /// Memory. `None` switches it off entirely.
    pub memory: Option<MemoryRuntime>,
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

/// The settings that can change while the core runs, as one snapshot that is swapped whole. A handler takes
/// `st.live()` once and reads from that, so one request never sees half of a change.
///
/// Which settings reach a chat that is already open, and which only new chats:
/// - Running chats, from the next model call or tool call: permissions (bypass, sudo, ask-for-everything),
///   spending (mode, approval limit, caps), `core.compact_at`, memory recall limits, `chats.title_model`,
///   `tree.summarize`, the KV cache settings, and `core.idle_after`.
/// - New chats only, because the chat's tools and first prompt are built when it starts: `web.searxng`,
///   `web.use_proxy`, `media.image_model`. The system prompt of an open chat also still describes the
///   permission mode and sudo as they were when it started; what is enforced follows the settings.
/// - Not live (restart): the provider, memory on/off and trust, learning, the browser address, the embedding model.
#[derive(Clone, Debug)]
pub struct Live {
    pub compact_at: Option<u64>,
    /// Recall limits for notes added to a message and for the "about you" block. Starts as the memory
    /// runtime's own value; `memory.recall_items` and `memory.recall_chars` replace two of its limits.
    pub recall: lh_memory::RecallConfig,
    pub tree_summarize: String,
    pub title_model: String,
    /// `memory.check_model`: the local model that looks for notes that disagree when the app is idle. Empty: off.
    pub check_model: String,
    /// `tools.sandbox`: run commands under bubblewrap.
    pub sandbox: bool,
    /// `tools.sandbox_network*`: may sandboxed commands use the network.
    pub sandbox_net: lh_tools::SandboxNet,
    /// `lhd --no-sandbox`: commands run without the sandbox, whatever `tools.sandbox` says.
    pub no_sandbox: bool,
    /// `tools.codemode`: offer the script tool.
    pub codemode: bool,
    pub web: lh_tools::WebConfig,
    pub image_model: Option<String>,
}

pub struct AppState<M> {
    model: Arc<M>,
    settings: Settings,
    sessions: Mutex<HashMap<String, Arc<Session>>>,
    next: AtomicU64,
    pub idle: Arc<idle::Idle>,
    /// Saving the KV cache per local chat; replaced when the kvcache settings change. `None`: not set up.
    kv: std::sync::RwLock<Option<Arc<kv_cache::KvCache>>>,
    /// Where the KV cache talks to (provider address and key), so it can be rebuilt when its settings change.
    kv_endpoint: std::sync::OnceLock<(String, Option<String>)>,
    live: std::sync::RwLock<Arc<Live>>,
    /// Permission settings shared with every chat and worker, read at each tool call.
    perms: Arc<lh_tools::LivePerms>,
    /// Where API keys are kept: the system secret store unless a test sets another.
    pub secrets: std::sync::OnceLock<Arc<dyn lh_secrets::SecretStore>>,
    /// News for every open client that is not tied to one turn, such as a chat getting its title. See `/v1/events`.
    pub events: tokio::sync::broadcast::Sender<Value>,
    /// Pairs of note versions already shown to the contradiction check in this run.
    pub(crate) checked_pairs: Mutex<std::collections::HashSet<(String, i64, String, i64)>>,
}

impl<M: Model + 'static> AppState<M> {
    pub(crate) fn tree_summarize(&self) -> String {
        self.live().tree_summarize.clone()
    }

    /// The settings in force now. Take it once per request.
    pub fn live(&self) -> Arc<Live> {
        self.live.read().unwrap().clone()
    }

    pub(crate) fn live_perms(&self) -> Arc<lh_tools::LivePerms> {
        self.perms.clone()
    }

    /// `lhd --no-sandbox`: run every chat's commands without the sandbox.
    pub fn turn_sandbox_off(&self) {
        self.set_live(|l| l.no_sandbox = true);
    }

    fn set_live(&self, f: impl FnOnce(&mut Live)) {
        let mut g = self.live.write().unwrap();
        let mut l = (**g).clone();
        f(&mut l);
        *g = Arc::new(l);
    }

    /// The KV cache saver, or none.
    pub fn set_kv(&self, kv: Option<Arc<kv_cache::KvCache>>) {
        *self.kv.write().unwrap() = kv;
    }

    pub fn kv(&self) -> Option<Arc<kv_cache::KvCache>> {
        self.kv.read().unwrap().clone()
    }

    /// Where the KV cache saver reaches the local server. Without this the kvcache settings are not applied.
    pub fn set_kv_endpoint(&self, provider_url: &str, key: Option<String>) {
        let _ = self.kv_endpoint.set((provider_url.to_string(), key));
    }

    pub fn set_title_model(&self, m: &str) {
        self.set_live(|l| l.title_model = m.to_string());
    }

    pub fn set_tree_summarize(&self, m: &str) {
        self.set_live(|l| l.tree_summarize = m.to_string());
    }

    /// Bring everything that can change while running in step with the settings file. Used at start (with
    /// `changed` empty, meaning everything) and after each change through the settings endpoint. A command-line
    /// flag keeps winning: the flags are the values in `Settings` (`bypass`, `sudo_enabled`, `web`, `image_model`
    /// and so on); the file adds to them and cannot loosen what a flag switched on.
    pub fn apply_settings(&self, cfg: &lh_config::ConfigFile, changed: &[String]) {
        let all = changed.is_empty();
        let touched = |prefix: &str| all || changed.iter().any(|k| k.starts_with(prefix));
        let s = &self.settings;
        if touched("permissions.") {
            self.perms.set(s.bypass || cfg.get_bool("permissions.bypass"), s.ask_all || cfg.get("permissions.mode") == "ask-all", s.sudo_enabled || cfg.get_bool("permissions.sudo"));
            self.perms.set_show_diff(lh_tools::ShowDiff::parse(&cfg.get("permissions.show_diff")).unwrap_or_default());
        }
        if touched("spending.") {
            s.spend.reconfigure(|c| {
                if cfg.is_set("spending.mode") {
                    c.mode = match cfg.get("spending.mode").as_str() {
                        "none" => lh_spend::Mode::None,
                        "caps" => lh_spend::Mode::Caps,
                        _ => lh_spend::Mode::Estimate,
                    };
                }
                if cfg.is_set("spending.approve_over_usd") {
                    c.approve_over_usd = cfg.get_num("spending.approve_over_usd").unwrap_or(c.approve_over_usd);
                }
                for (key, slot) in [("spending.per_request_cap_usd", &mut c.per_request_cap_usd), ("spending.daily_cap_usd", &mut c.daily_cap_usd), ("spending.monthly_cap_usd", &mut c.monthly_cap_usd)] {
                    if cfg.is_set(key) {
                        *slot = cfg.get_num(key);
                    }
                }
            });
        }
        if touched("core.idle_after") {
            self.idle.set_after(std::time::Duration::from_secs(cfg.get_num("core.idle_after").unwrap_or(600)));
        }
        let base = s.memory.as_ref().map(|m| m.cfg.clone()).unwrap_or_default();
        self.set_live(|l| {
            if touched("core.compact_at") {
                l.compact_at = s.compact_at.or_else(|| match cfg.get("core.compact_at").as_str() {
                    "auto" => None,
                    n => n.parse().ok(),
                });
            }
            if touched("memory.recall_") {
                l.recall = base.clone();
                if cfg.is_set("memory.recall_items") {
                    l.recall.max_items = cfg.get_num("memory.recall_items").unwrap_or(l.recall.max_items);
                }
                if cfg.is_set("memory.recall_chars") {
                    l.recall.budget_chars = cfg.get_num("memory.recall_chars").unwrap_or(l.recall.budget_chars);
                }
            }
            if touched("memory.check_model") {
                l.check_model = cfg.get("memory.check_model");
            }
            if touched("tools.codemode") {
                l.codemode = cfg.get("tools.codemode") == "true";
            }
            if touched("tools.sandbox") {
                l.sandbox = cfg.get("tools.sandbox") != "false";
                l.sandbox_net = lh_tools::SandboxNet::from_settings(&cfg.get("tools.sandbox_network"), &cfg.get("tools.sandbox_network_allow"), &cfg.get("tools.sandbox_network_model"));
            }
            if touched("tree.") {
                l.tree_summarize = cfg.get("tree.summarize");
            }
            if touched("chats.") {
                l.title_model = cfg.get("chats.title_model");
            }
            if touched("web.") {
                l.web.searxng_url = s.web.searxng_url.clone().or_else(|| cfg.get_opt("web.searxng"));
                l.web.use_proxy = s.web.use_proxy || cfg.get_bool("web.use_proxy");
            }
            if touched("media.") {
                l.image_model = s.image_model.clone().or_else(|| cfg.get_opt("media.image_model"));
            }
        });
        if touched("kvcache.") {
            if let Some((url, key)) = self.kv_endpoint.get() {
                let kv = cfg.get_bool("kvcache.enabled").then(|| cfg.get_opt("kvcache.dir")).flatten().map(|dir| Arc::new(kv_cache::KvCache::new(url, key.clone(), PathBuf::from(dir), cfg.get_num("kvcache.max_gb").unwrap_or(5))));
                self.set_kv(kv);
            }
        }
    }

    /// The KV cache saver, if it is set up and allowed for this session: a local endpoint, a stored (not private) chat.
    pub(crate) fn kv_for(&self, s: &Session) -> Option<Arc<kv_cache::KvCache>> {
        let kv = self.kv()?;
        let local = self.settings.memory.as_ref().is_some_and(|m| m.trust == lh_memory::Trust::Local);
        (local && !s.incognito && s.conv.is_some() && !kv.is_off()).then_some(kv)
    }

    /// Builds the state and starts the idle manager. Must be called inside a Tokio runtime.
    pub fn new(model: M, settings: Settings) -> Arc<Self> {
        let model = Arc::new(model);
        let idle = idle::Idle::new(settings.idle_after);
        let live = Live { compact_at: settings.compact_at, recall: settings.memory.as_ref().map(|m| m.cfg.clone()).unwrap_or_default(), tree_summarize: "ask".into(), title_model: String::new(), check_model: String::new(), sandbox: true, sandbox_net: Default::default(), no_sandbox: false, codemode: false, web: settings.web.clone(), image_model: settings.image_model.clone() };
        let perms = Arc::new(lh_tools::LivePerms::new(settings.bypass, settings.ask_all, settings.sudo_enabled));
        {
            let (model, store) = (model.clone(), settings.store.clone());
            let mem = settings.memory.as_ref().map(|m| (m.memory.clone(), m.embedder.clone()));
            let maintain = settings.maintain_when_idle;
            idle.on_change(move |going_idle| {
                if going_idle {
                    model.release_idle();
                    if let Some((memory, embedder)) = &mem {
                        if maintain {
                            let m = memory.clone();
                            tokio::task::spawn_blocking(move || {
                                let _ = m.maintenance();
                            });
                        }
                        memory.release_vectors();
                        let e = embedder.clone();
                        tokio::spawn(async move { lh_memory::embed::Embedder::release(&*e).await });
                    }
                    store.shrink();
                    idle::trim_memory();
                }
            });
        }
        tokio::spawn(idle.clone().run());
        let first = settings.memory.as_ref().map_or(1, |m| m.memory.next_session_number());
        let st = Arc::new(Self { model, settings, sessions: Mutex::new(HashMap::new()), next: AtomicU64::new(first), idle, kv: Default::default(), kv_endpoint: Default::default(), live: std::sync::RwLock::new(Arc::new(live)), perms, secrets: std::sync::OnceLock::new(), events: tokio::sync::broadcast::channel(64).0, checked_pairs: Default::default() });
        // when idle, look for notes that disagree, if a local model is set for it
        let weak = Arc::downgrade(&st);
        st.idle.on_change(move |going_idle| {
            let Some(st) = weak.upgrade().filter(|_| going_idle) else { return };
            let model = st.live().check_model.clone();
            if model.is_empty() {
                return;
            }
            tokio::spawn(async move {
                let found = memory_api::check_contradictions(&st, &model).await;
                if !found.is_empty() {
                    let _ = st.settings.store.log("memory_contradictions", "-", "maintenance", &json!({"pairs": found.len()}));
                }
            });
        });
        st
    }

    pub(crate) fn session(&self, id: &str) -> Result<Arc<Session>, ApiError> {
        self.sessions.lock().unwrap().get(id).cloned().ok_or(ApiError(StatusCode::NOT_FOUND, "no such session".into()))
    }
}

/// Every API route, with no guard on it. The doors below each add their own.
fn api_routes<M: Model + 'static>() -> Router<Arc<AppState<M>>> {
    Router::new()
        .route("/v1/health", get(|| async { Json(json!({"ok": true})) }))
        .route("/v1/models", get(list_models::<M>))
        .route("/v1/capabilities", get(capabilities::<M>))
        .route("/v1/activity", get(activity::<M>))
        .route("/v1/activity/{seq}/undo", post(undo::<M>))
        .route("/v1/spend", get(spend::<M>))
        .route("/v1/events", get(app_events::<M>))
        .route("/v1/sessions", post(create_session::<M>))
        .route("/v1/sessions/{id}", get(get_session::<M>).delete(delete_session::<M>))
        .route("/v1/sessions/{id}/turns", post(start_turn::<M>))
        .route("/v1/sessions/{id}/model", post(switch_model::<M>))
        .route("/v1/sessions/{id}/answers/{ask}", post(answer::<M>))
        .route("/v1/sessions/{id}/cancel", post(cancel::<M>))
        .route("/v1/sessions/{id}/undo_turn", post(undo_turn::<M>))
        .route("/v1/status", get(status::<M>))
        .merge(memory_api::routes::<M>())
        .merge(learn_api::routes::<M>())
        .merge(team_api::routes::<M>())
        .merge(mcp_api::routes::<M>())
        .merge(settings_api::routes::<M>())
        .merge(provider_api::routes::<M>())
        .merge(export_api::routes::<M>())
        .merge(channels::routes::<M>())
        .merge(schedules_api::routes::<M>())
        .merge(rewards::routes::<M>())
        .merge(chats_api::routes::<M>())
        .merge(tree_api::routes::<M>())
}

/// Only a request that passed its guard counts as activity: this layer sits outside the guard, so it
/// must not wake the core for strangers. See `auth::require`.
async fn touch_idle(idle: Arc<idle::Idle>, req: axum::extract::Request, next: axum::middleware::Next) -> Response {
    let r = next.run(req).await;
    if r.status() != StatusCode::UNAUTHORIZED && r.status() != StatusCode::FORBIDDEN {
        idle.touch();
    }
    r
}

/// The machine's own door: loopback, with the token from the token file. `port` is the port the listener
/// is bound to; it decides which `Host` headers are accepted.
pub fn router<M: Model + 'static>(state: Arc<AppState<M>>, token: String, port: u16) -> Router {
    router_with_web(state, token, port, None)
}

/// The same, plus the commands for pairing and listing browsers when the web door is switched on.
pub fn router_with_web<M: Model + 'static>(state: Arc<AppState<M>>, token: String, port: u16, web: Option<Arc<web::Web>>) -> Router {
    let idle = state.idle.clone();
    let guard = Arc::new(auth::Guard { token, hosts: vec![format!("127.0.0.1:{port}"), format!("localhost:{port}"), format!("[::1]:{port}")] });
    let mut r = api_routes::<M>().with_state(state);
    if let Some(w) = web {
        r = r.merge(web::local_routes(w));
    }
    r.layer(axum::middleware::from_fn_with_state(guard, auth::require)).layer(axum::middleware::from_fn(move |req, next| touch_idle(idle.clone(), req, next)))
}

/// The browser's door (served over HTTPS): the pages, pairing and sign-in, and the API for paired
/// devices only, restricted to what the browser screens need.
pub fn web_router<M: Model + 'static>(state: Arc<AppState<M>>, web: Arc<web::Web>) -> Router {
    let idle = state.idle.clone();
    let api = api_routes::<M>().with_state(state).merge(web::device_routes(web.clone())).layer(axum::middleware::from_fn_with_state(web.clone(), web::require_device));
    web::public_routes(web).merge(api).layer(axum::middleware::from_fn(move |req, next| touch_idle(idle.clone(), req, next))).layer(axum::middleware::from_fn(web::headers))
}

pub(crate) struct ApiError(pub(crate) StatusCode, pub(crate) String);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({"error": self.1}))).into_response()
    }
}

async fn status<M: Model + 'static>(State(st): State<Arc<AppState<M>>>) -> Json<Value> {
    Json(json!({"sessions": st.sessions.lock().unwrap().len(), "idle_after_secs": st.idle.after().as_secs()}))
}

async fn list_models<M: Model + 'static>(State(st): State<Arc<AppState<M>>>) -> Result<Json<Value>, ApiError> {
    let details = st.model.model_details().await.map_err(|e| ApiError(StatusCode::BAD_GATEWAY, e.explain()))?;
    let models: Vec<&str> = details.iter().filter_map(|d| d["id"].as_str()).collect();
    Ok(Json(json!({"models": models, "details": details})))
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
    /// A temporary session: no memory is read or written, no conversation is stored.
    #[serde(default)]
    incognito: bool,
    /// The session is for a messaging channel: the lowest trust level, so nothing sensitive is used.
    #[serde(default)]
    channel: bool,
    /// `lh -p --no-sandbox`: this session's commands run without the sandbox.
    #[serde(default)]
    no_sandbox: bool,
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

async fn create_session<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, via: Option<axum::Extension<web::Via>>, Json(mut b): Json<NewSession>) -> Result<Json<Value>, ApiError> {
    if let Some(axum::Extension(web::Via::Device(_))) = via {
        if let Some(d) = &b.project_dir {
            web::device_folder_ok(d).map_err(|e| ApiError(StatusCode::FORBIDDEN, e))?;
        } else if b.mode.as_deref() == Some("assistant") {
            b.ask_all = true; // from a browser, the home folder is not read without asking
        }
    }
    new_session(&st, b, None).await
}

/// Build a session. `forced_id` is used when a stored chat is opened again: the session then carries the
/// chat's own id, so what is said next is stored in the same conversation.
async fn new_session<M: Model + 'static>(st: &Arc<AppState<M>>, b: NewSession, forced_id: Option<String>) -> Result<Json<Value>, ApiError> {
    let (dir, tools_on) = match (&b.project_dir, b.mode.as_deref()) {
        (Some(d), _) => (d.clone(), true),
        (None, Some("assistant")) => (std::env::var_os("HOME").map(PathBuf::from).ok_or(ApiError(StatusCode::BAD_REQUEST, "no home folder".into()))?, true),
        (None, None | Some("chat")) => (std::env::temp_dir(), false),
        (None, Some(other)) => return Err(ApiError(StatusCode::BAD_REQUEST, format!("mode must be \"chat\" or \"assistant\", not \"{other}\""))),
    };
    let mut perms = Permissions::new(&dir).map_err(|e| ApiError(StatusCode::BAD_REQUEST, format!("project_dir: {e}")))?;
    // bypass, sudo and the ask-for-everything setting come from the live settings at each tool call; the
    // chat itself only asks for ask-all when the client says so
    perms.live = Some(st.live_perms());
    perms.live_ask_all = true;
    perms.allow_rules = st.settings.allow_rules.clone();
    perms.mode = if b.ask_all { Mode::AskAll } else { Mode::Tiered };
    let mut req = ChatRequest { model: b.model, max_tokens: b.max_tokens, ..Default::default() };
    if let Some(s) = b.sampling {
        req.sampling = lh_provider::Sampling { temperature: s.temperature, top_p: s.top_p, top_k: s.top_k, min_p: s.min_p, repeat_penalty: s.repeat_penalty, extra: s.extra };
    }
    if let Some(s) = b.system {
        req.messages.push(Message::system(s));
    }
    let id = forced_id.unwrap_or_else(|| format!("s{}", st.next.fetch_add(1, Ordering::SeqCst)));
    let model_name = req.model.clone();
    let mode_name = match (&b.project_dir, tools_on) {
        (Some(_), _) => "project",
        (None, true) => "assistant",
        (None, false) => "chat",
    };
    let project = b.project_dir.as_ref().and_then(|d| std::fs::canonicalize(d).ok()).map(|d| d.to_string_lossy().to_string());
    let live = st.live();
    let mut tools = session_tools(&st.settings, &live, tools_on, live.no_sandbox || b.no_sandbox);
    if live.no_sandbox || b.no_sandbox {
        let _ = st.settings.store.log("sandbox_off", &id, "user", &json!({"why": if b.no_sandbox { "--no-sandbox (session)" } else { "lhd --no-sandbox" }}));
    }
    let (mut dest, mut conv) = (None, None);
    let mut about = String::new();
    if let (Some(rt), false) = (&st.settings.memory, b.incognito) {
        let mut d = rt.destination(project.clone());
        if b.channel {
            d.trust = lh_memory::Trust::AnyCloud; // a chat service can read everything sent through it
        }
        let _ = rt.memory.scan_if_changed(); // pick up pages edited by hand
        let (text, refs) = rt.memory.about_you(&d, &live.recall).map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        if !text.is_empty() {
            let pages: Vec<Value> = refs.iter().map(|r| json!({"page": r.id, "version": r.version})).collect();
            let _ = st.settings.store.log("memory_sent", &id, "assistant", &json!({"with": "about_you", "trust": format!("{:?}", rt.trust), "pages": pages}));
        }
        about = text;
        let _ = rt.memory.start_conversation(&id, project.as_deref());
        let _ = rt.memory.set_conversation_meta(&id, &model_name, mode_name);
        conv = Some(id.clone());
        if tools_on {
            // on a model on this machine, that model also judges what remember saves, as the review does
            let judge: Option<lh_tools::PrivacyJudge> = (d.trust == lh_memory::Trust::Local).then(|| {
                let (model, name) = (st.model.clone(), model_name.clone());
                let judge: lh_tools::PrivacyJudge = Arc::new(move |note: String| {
                    let (model, name) = (model.clone(), name.clone());
                    Box::pin(async move {
                        let mut req = ChatRequest { model: name, messages: vec![Message::user(format!("{}{note}", lh_memory::JUDGE_PROMPT))], max_tokens: Some(1500), ..Default::default() };
                        req.sampling.extra.insert("chat_template_kwargs".into(), json!({"enable_thinking": false}));
                        match model.complete(&req, &mut |_| {}).await {
                            Ok(r) => lh_memory::parse_judgement(&r.text),
                            Err(_) => lh_memory::Judgement::Unsure,
                        }
                    })
                });
                judge
            });
            tools = tools.with_memory(Arc::new(lh_tools::MemoryHandle { memory: rt.memory.clone(), embedder: rt.embedder.clone(), dest: d.clone(), cfg: live.recall.clone(), layers: None, judge }));
        }
        dest = Some(d);
        let (m, e) = (rt.memory.clone(), rt.embedder.clone());
        tokio::spawn(async move {
            let _ = m.index_pending(&*e).await;
        });
    }
    // What the user has taught: rules, the style profile and skills. They are how the user wants things
    // done, not memory of the user, so they apply in incognito sessions too (they are only never learned there).
    let mut learned_block = String::new();
    if let Some(lr) = &st.settings.learn {
        let learn_trust = if b.channel { lh_memory::Trust::AnyCloud } else { lr.trust };
        let ctx = lh_learn::RuleContext { project: project.clone(), role: None, trust: learn_trust };
        let rules = lr.learn.applicable_rules(&ctx).map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let style = lr.learn.style().ok().flatten().map(|s| s.text);
        let skills: Vec<(String, String)> = if tools_on { lr.learn.list_skills().unwrap_or_default().into_iter().filter(|e| e.error.is_none()).map(|e| (e.name, e.description)).collect() } else { vec![] };
        if !skills.is_empty() {
            tools = tools.with_skills(lr.learn.clone());
        }
        let used = json!({"rules": rules.iter().map(|r| r.id.clone()).collect::<Vec<_>>(), "style": style.is_some(), "skills": skills.iter().map(|s| s.0.clone()).collect::<Vec<_>>()});
        learned_block = lh_learn::render_learned(&lh_learn::Learned { rules, style, skills }, learn_trust);
        if !learned_block.is_empty() {
            let _ = st.settings.store.log("learned_applied", &id, "assistant", &used);
        }
    }
    // The system message is put together on the first turn (see `build_system`): the built-in base prompt
    // first, then the client's own text untouched, then what was learned, then memory.
    let client_block = match req.messages.first() {
        Some(m) if m.role == lh_provider::Role::System => {
            let t = m.content.clone().unwrap_or_default();
            req.messages.remove(0);
            t
        }
        _ => String::new(),
    };
    let kind = match (&b.project_dir, tools_on) {
        (Some(_), _) => lh_tools::SessionKind::Project,
        (None, true) => lh_tools::SessionKind::Assistant,
        (None, false) => lh_tools::SessionKind::Chat,
    };
    let mut pending_trust = None;
    if let (Some(m), true) = (&st.settings.mcp, tools_on) {
        for t in m.tools_for(project.as_deref()).await {
            tools = tools.with_extra(t);
        }
        pending_trust = project.as_deref().and_then(|p| m.pending_trust(p));
    }
    // tools that need to reach back into the session (departments, show) share one slot, filled once it exists
    let slot = tools_on.then(|| {
        let slot = Arc::new(std::sync::OnceLock::new());
        tools = std::mem::take(&mut tools).with_extra(Arc::new(show_tool::ShowTool { session: slot.clone() }));
        if st.settings.team.is_some() {
            tools = std::mem::take(&mut tools).with_extra(Arc::new(team_api::DeptTool { st: Arc::downgrade(st), session: slot.clone(), project: false })).with_extra(Arc::new(team_api::DeptTool { st: Arc::downgrade(st), session: slot.clone(), project: true }));
        }
        slot
    });
    let parts = session::SystemParts { kind, folder: tools_on.then(|| perms.root().to_path_buf()), guidelines: tools.guidelines(), client_block, learned_block, about };
    let mut session = Session::new(req, perms, tools, parts);
    (session.incognito, session.dest, session.conv, session.project) = (b.incognito, dest, conv, project);
    let session = Arc::new(session);
    if let Some(slot) = slot {
        let _ = slot.set(Arc::downgrade(&session));
    }
    st.sessions.lock().unwrap().insert(id.clone(), session);
    Ok(Json(json!({"id": id, "incognito": b.incognito, "needs_trust": pending_trust})))
}

/// Build (or rebuild) the system message for the model this session uses. Names the model and the
/// service from what they report, and leaves out what is not reported.
async fn build_system<M: Model + 'static>(st: &AppState<M>, s: &Session, messages: &mut Vec<Message>, model_id: &str) {
    let caps = st.model.capabilities(model_id).await;
    let provider = match &st.settings.provider_name {
        Some(p) => Some(p.clone()),
        None => st.model.provider_name().await,
    };
    let p = &s.parts;
    // the settings can change while a chat is open, so describe what is in force now
    let (permissions, sudo_enabled) = (s.perms.effective_mode(), s.perms.sudo_allowed());
    *s.shown_perms.lock().unwrap() = Some((permissions, sudo_enabled));
    let base = lh_tools::base_prompt(&lh_tools::BaseContext {
        identity: lh_tools::Identity { model: Some(model_id.to_string()), parameters: caps.parameters, provider },
        kind: p.kind,
        folder: p.folder.as_deref(),
        permissions,
        sudo_enabled,
        date: &lh_tools::today(),
        guidelines: &p.guidelines,
    });
    let Some(mut sys) = lh_learn::compose_system(Some(&base), &[&p.client_block, &p.learned_block, &p.about]) else { return };
    match messages.first_mut().filter(|m| m.role == lh_provider::Role::System) {
        Some(old) => {
            // a summary written by compaction belongs to the conversation, not to the base; keep it
            if let Some(i) = old.content.as_deref().and_then(|c| c.find(lh_tools::SUMMARY_MARKER)) {
                sys.push_str(&old.content.as_deref().unwrap_or("")[i..]);
            }
            old.content = Some(sys);
        }
        None => messages.insert(0, Message::system(sys)),
    }
}

#[derive(Deserialize)]
struct SwitchModel {
    model: String,
}

/// The user switches model. The first line of the prompt names the model, so it is rebuilt on the next
/// turn, and because the start of the prompt changes the server has to read the whole conversation again.
async fn switch_model<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<String>, Json(b): Json<SwitchModel>) -> Result<StatusCode, ApiError> {
    let s = st.session(&id)?;
    if !s.try_start() {
        return Err(ApiError(StatusCode::CONFLICT, "a turn is running in this session".into()));
    }
    let mut req = s.req.lock().await;
    let from = std::mem::replace(&mut req.model, b.model.clone());
    drop(req);
    if let (Some(rt), Some(conv)) = (&st.settings.memory, &s.conv) {
        let _ = rt.memory.set_conversation_model(conv, &b.model);
    }
    s.known_tokens.store(0, Ordering::SeqCst);
    s.identity_stale.store(true, Ordering::SeqCst);
    s.finish_turn();
    let _ = st.settings.store.log("model_switched", &id, "user", &json!({"from": from, "to": b.model, "note": "the first line of the prompt changes, so the server reads the whole conversation again (a cold prefix) on the next turn"}));
    Ok(StatusCode::NO_CONTENT)
}

/// The text of every sensitive note and rule, archived ones too. Nothing that goes out of the machine may
/// repeat it.
pub(crate) fn sensitive_texts(store: &lh_store::Store) -> Vec<String> {
    let mut out: Vec<String> = store
        .with(|c| {
            let mut q = c.prepare("select body, gist, title from pages where privacy = 'sensitive' union all select text, '', '' from rules where privacy = 'sensitive'")?;
            let rows = q.query_map([], |r| Ok([r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?]))?;
            Ok(rows.flatten().flatten().collect())
        })
        .unwrap_or_default();
    out.retain(|t: &String| !t.trim().is_empty());
    out
}

/// A guard for the tools that send things out: it refuses text that repeats a sensitive note or rule.
pub(crate) fn outbound_guard(s: &Settings) -> Option<lh_tools::GuardFn> {
    let store = s.store.clone();
    Some(Arc::new(move |text: &str| lh_memory::leaks(text, &sensitive_texts(&store))))
}

fn session_tools(s: &Settings, live: &Live, tools_on: bool, no_sandbox: bool) -> lh_tools::ToolSet {
    if !tools_on {
        return lh_tools::ToolSet::default(); // plain chat: the model is told about no tools
    }
    let mut t = default_tools().with_web(live.web.clone()).with_sandbox(live.sandbox && !no_sandbox).with_sandbox_net(live.sandbox_net.clone()).with_codemode(live.codemode);
    if let Some(g) = outbound_guard(s) {
        t = t.with_outbound_guard(g);
    }
    // hooks.toml beside the settings file, read for each new chat so an edit counts from the next chat on
    if let Some(cfg) = &s.config {
        match lh_tools::Hooks::load(&cfg.with_file_name("hooks.toml")) {
            Ok(h) => t = t.with_hooks(Arc::new(h)),
            Err(e) => {
                let _ = s.store.log("hooks_problem", "-", "lhd", &json!({"error": e}));
            }
        }
    }
    match &live.image_model {
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
        TurnEvent::Diff { id, path, diff } => ("diff".into(), json!({"type":"diff","id":id,"path":path,"diff":diff})),
        TurnEvent::Decision { .. } => return None,
    })
}

/// Writes what happened to the activity log. Failures are reported on stderr and never stop the turn.
struct Recorder {
    store: Arc<lh_store::Store>,
    session: String,
    calls: HashMap<String, (String, String)>, // id -> (tool, arguments)
    /// Incognito sessions still leave a security record, but without any text from the conversation.
    incognito: bool,
    sess: Arc<Session>,
    /// What each tool did this turn, kept for the chat tree (never sent to a model again).
    notes: Arc<Mutex<Vec<String>>>,
}

impl Recorder {
    fn record(&mut self, ev: &TurnEvent) {
        let r = match ev {
            TurnEvent::ToolStart { id, name, arguments } => {
                self.sess.tool_calls.fetch_add(1, Ordering::SeqCst);
                if name == "use_skill" {
                    if let Some(n) = serde_json::from_str::<Value>(arguments).ok().and_then(|v| v["name"].as_str().map(str::to_string)) {
                        self.sess.skills_used.lock().unwrap().insert(n);
                    }
                }
                self.calls.insert(id.clone(), (name.clone(), arguments.clone()));
                Ok(())
            }
            TurnEvent::Decision { tool, action, outcome, .. } => {
                let o = match outcome {
                    Outcome::Allowed => json!("allowed"),
                    Outcome::AllowedByUser => json!("allowed_by_user"),
                    Outcome::Declined => {
                        rewards::reward(&self.store, &self.session, "declined", -0.5, self.incognito);
                        json!("declined_by_user")
                    }
                    Outcome::Denied(why) => json!({"denied": why}),
                };
                let detail = if self.incognito { json!({"tool": tool, "outcome": o, "incognito": true}) } else { json!({"tool": tool, "action": action, "outcome": o}) };
                self.store.log("permission", &self.session, "assistant", &detail).map(|_| ())
            }
            TurnEvent::ToolResult { id, text, undo } => {
                let (tool, args) = self.calls.remove(id).unwrap_or_default();
                if tool == "run_command" {
                    let cmd = serde_json::from_str::<Value>(&args).ok().and_then(|v| v["command"].as_str().map(str::to_string)).unwrap_or_default();
                    if rewards::is_test_command(&cmd) {
                        if let Some(code) = rewards::exit_code(text) {
                            rewards::reward(&self.store, &self.session, "tests", if code == 0 { 1.0 } else { -1.0 }, self.incognito);
                        }
                    }
                }
                let preview: String = text.chars().take(300).collect();
                if !self.incognito {
                    let one = |t: &str, n: usize| t.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(n).collect::<String>();
                    self.notes.lock().unwrap().push(format!("{tool} {} → {}", one(&args, 120), one(text, 160)));
                }
                let detail = if self.incognito { json!({"tool": tool, "incognito": true}) } else { json!({"tool": tool, "arguments": args, "result": preview}) };
                let logged = self.store.log("tool_call", &self.session, "assistant", &detail);
                match (logged, undo) {
                    (Ok(seq), Some(u)) if !self.incognito => {
                        let after = std::fs::read(&u.path).map(|b| lh_store::content_hash(&b)).unwrap_or(0);
                        self.store.log_file_change(seq, &u.path, u.previous.as_deref(), after)
                    }
                    (Ok(_), _) => Ok(()),
                    (Err(e), _) => Err(e),
                }
            }
            TurnEvent::Model(_) | TurnEvent::Diff { .. } => Ok(()),
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
    s.turn_start.store(st.settings.store.last_seq().unwrap_or(0), Ordering::SeqCst);
    let (tx, rx) = mpsc::unbounded_channel::<(String, Value)>();
    let task = {
        let live = st.live();
        let compact_at = live.compact_at;
        let st2 = st.clone();
        let (s, model, tx, meter, store, id) = (s.clone(), st.model.clone(), tx.clone(), st.settings.spend.for_session(&id), st.settings.store.clone(), id.clone());
        tokio::spawn(async move {
            // Work on a copy; the session history only changes if the whole turn succeeds, so a
            // cancelled turn never leaves a tool call without its result.
            let mut work = s.req.lock().await.clone();
            let model_id = work.model.clone();
            // a chat opened again from storage gets its saved KV cache back first; a save from the last turn finishes first
            if let Some(kv) = st2.kv_for(&s) {
                let _gate = s.kv_gate.lock().await;
                if s.kv_restore.swap(false, Ordering::SeqCst) {
                    let (conv, t0) = (s.conv.clone().unwrap_or_default(), std::time::Instant::now());
                    match kv.restore(&conv, &model_id).await {
                        Ok(tokens) => {
                            let _ = store.log("kv_cache_restored", &id, "assistant", &json!({"tokens": tokens, "secs": t0.elapsed().as_secs_f32()}));
                        }
                        Err(why) => {
                            let _ = store.log("kv_cache_skipped", &id, "assistant", &json!({"why": why}));
                        }
                    }
                }
            }
            let stale = s.identity_stale.load(Ordering::SeqCst);
            if stale || s.perms_changed() {
                build_system(&st2, &s, &mut work.messages, &model_id).await;
            }
            let mut user_text = b.text.clone();
            if let (Some(rt), Some(dest), false) = (&st2.settings.memory, &s.dest, s.incognito) {
                // pages edited by hand since the last message count from this one on
                let _ = rt.memory.scan_if_changed();
                // notes that match this message, from the keyword search and, if set up, the embedding search
                let semantic = if rt.embedder.is_on() { rt.memory.semantic_search(&*rt.embedder, &b.text, 10).await.unwrap_or_default() } else { vec![] };
                let already = s.injected.lock().unwrap().clone();
                if let Ok(rec) = rt.memory.recall(&b.text, dest, &live.recall, &semantic, &already) {
                    if !rec.notes.is_empty() {
                        user_text = format!("{}\n{}", rec.render(), b.text);
                        let pages: Vec<Value> = rec.refs().iter().map(|r| json!({"page": r.id, "version": r.version})).collect();
                        s.injected.lock().unwrap().extend(rec.refs().into_iter().map(|r| (r.id, r.version)));
                        let _ = store.log("memory_sent", &id, "assistant", &json!({"with": "recall", "trust": format!("{:?}", rt.trust), "pages": pages, "withheld": rec.withheld}));
                    }
                }
            }
            work.messages.push(Message::user(user_text));
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
            *s.turn_events.lock().unwrap() = Some(tx.downgrade());
            let asker = ApiAsker { session: s.clone(), events: tx.clone() };
            let opts = TurnOptions::default();
            let events = tx.clone();
            let notes: Arc<Mutex<Vec<String>>> = Default::default();
            let mut rec = Recorder { store: store.clone(), session: id.clone(), calls: HashMap::new(), incognito: s.incognito, sess: s.clone(), notes: notes.clone() };
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
                    if stale {
                        s.identity_stale.store(false, Ordering::SeqCst);
                    }
                    // save the model's work on this chat, so it can be opened again without reading it all over
                    if let (Some(kv), Some(conv)) = (st2.kv_for(&s), s.conv.clone()) {
                        let (gate, model, store, sid) = (s.kv_gate.clone().lock_owned().await, model_id.clone(), store.clone(), id.clone());
                        tokio::spawn(async move {
                            let _gate = gate;
                            let t0 = std::time::Instant::now();
                            match kv.save(&conv, &model).await {
                                Ok(sv) => {
                                    let _ = store.log("kv_cache_saved", &sid, "assistant", &json!({"bytes": sv.bytes, "tokens": sv.tokens, "secs": t0.elapsed().as_secs_f32()}));
                                }
                                Err(why) => {
                                    let _ = store.log("kv_cache_problem", &sid, "assistant", &json!({"why": why}));
                                }
                            }
                        });
                    }
                    if let (Some(lr), false) = (&st2.settings.learn, s.incognito) {
                        let _ = lr.learn.scan_messages(&id, s.project.as_deref(), std::slice::from_ref(&b.text));
                    }
                    if let (Some(rt), Some(conv)) = (&st2.settings.memory, &s.conv) {
                        let _ = rt.memory.append_message(conv, "user", &b.text);
                        let first = rt.memory.set_title_if_empty(conv, &b.text).ok().flatten();
                        for note in notes.lock().unwrap().iter() {
                            let _ = rt.memory.append_message(conv, "tool", note);
                        }
                        if !r.text.is_empty() {
                            let _ = rt.memory.append_message(conv, "assistant", &r.text);
                        }
                        if let (Some(fallback), Some(tm), false, false) = (first, Some(st2.live().title_model.clone()).filter(|m| !m.is_empty()), s.incognito, r.text.is_empty()) {
                            let job = titles::Job { model: tm.clone(), conv: conv.clone(), fallback, user: b.text.clone(), reply: r.text.clone(), session: id.clone() };
                            let st3 = st2.clone();
                            // the turn's stream has closed by the time the title comes, so it goes to /v1/events
                            tokio::spawn(async move {
                                if let Some(title) = titles::run(&*st3.model, &st3.settings, &job).await {
                                    let _ = st3.events.send(json!({"type":"title","conversation":job.conv,"session":job.session,"title":title}));
                                }
                            });
                        }
                    }
                    ("done".to_string(), json!({"type":"done","empty":empty}))
                }
                Ok(TurnEnd::Refused(why)) => ("refused".into(), json!({"type":"refused","message":why})),
                Ok(TurnEnd::Stopped) => ("error".into(), json!({"type":"error","message":"stopped","detail":"stopped","retryable":false})),
                Ok(TurnEnd::StepLimit) => {
                    *s.req.lock().await = work;
                    if stale {
                        s.identity_stale.store(false, Ordering::SeqCst);
                    }
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

/// Put back every file the last turn of this chat wrote. Files changed by someone else since are left alone and
/// listed, so nothing the user did is lost.
async fn undo_turn<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    let s = st.session(&id)?;
    if !s.try_start() {
        return Err(ApiError(StatusCode::CONFLICT, "a turn is running in this session".into()));
    }
    let r = st.settings.store.undo_since(&id, s.turn_start.load(Ordering::SeqCst));
    s.finish_turn();
    if r.as_ref().is_ok_and(|f| !f.is_empty()) {
        rewards::reward(&st.settings.store, &id, "undone", -1.0, s.incognito);
    }
    use lh_store::UndoResult::*;
    let files: Vec<Value> = r
        .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .into_iter()
        .map(|(seq, path, r)| match r {
            Restored => json!({"seq": seq, "path": path, "result": "restored"}),
            Removed => json!({"seq": seq, "path": path, "result": "removed"}),
            Refused(m) => json!({"seq": seq, "path": path, "result": "refused", "message": m}),
        })
        .collect();
    Ok(Json(json!({"files": files})))
}

#[derive(Deserialize)]
struct ActivityQuery {
    limit: Option<u32>,
    kind: Option<String>,
}

async fn activity<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Query(q): Query<ActivityQuery>) -> Result<Json<Value>, ApiError> {
    let rows = st.settings.store.recent(q.limit.unwrap_or(50).min(500), q.kind.as_deref()).map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let undoable = st.settings.store.undoable(&rows.iter().map(|e| e.seq).collect::<Vec<_>>()).unwrap_or_default();
    let entries: Vec<Value> = rows.iter().map(|e| json!({"seq": e.seq, "at": e.at, "kind": e.kind, "session": e.session, "actor": e.actor, "detail": e.detail, "undoable": undoable.contains(&e.seq)})).collect();
    Ok(Json(json!({"entries": entries})))
}

/// Events for the whole app, as server-sent events, for as long as the client listens. A client that falls behind
/// misses the oldest events, which only ever repeat what a fresh list call would show.
async fn app_events<M: Model + 'static>(State(st): State<Arc<AppState<M>>>) -> SseResponse<impl Stream<Item = Result<Sse, Infallible>>> {
    let rx = st.events.subscribe();
    let out = stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(v) => {
                    let name = v["type"].as_str().unwrap_or("event").to_string();
                    return Some((Ok(Sse::default().event(name).data(v.to_string())), rx));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
            }
        }
    });
    SseResponse::new(out).keep_alive(KeepAlive::default())
}

/// Spending today, this month and per provider this month, from the spend ledger.
async fn spend<M: Model + 'static>(State(st): State<Arc<AppState<M>>>) -> Json<Value> {
    let s = st.settings.spend.summary();
    Json(json!({
        "today": {"micro_usd": s.today, "calls": s.today_calls, "text": lh_spend::dollars(s.today)},
        "this_month": {"micro_usd": s.this_month, "calls": s.month_calls, "text": lh_spend::dollars(s.this_month)},
        "unpriced_calls": s.unpriced,
        "providers": s.by_provider.iter().map(|(p, c, n)| json!({"provider": p, "micro_usd": c, "calls": n, "text": lh_spend::dollars(*c)})).collect::<Vec<_>>(),
    }))
}

async fn undo<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(seq): Path<i64>) -> Result<Json<Value>, ApiError> {
    use lh_store::UndoResult::*;
    let r = st.settings.store.undo(seq, "api").map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if !matches!(r, Refused(_)) {
        rewards::reward(&st.settings.store, "-", "undone", -1.0, false);
    }
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
