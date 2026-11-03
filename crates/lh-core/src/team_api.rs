//! Roles, model types, departments, team settings and runs over the local API.
//!
//! A run is one goal given to one department. Its events are kept in order, so a client that connects
//! late (or again) still sees everything, and questions to the user wait inside the run until answered.

use std::collections::{BTreeMap, HashMap};
use std::convert::Infallible;
use std::path::Path as FsPath;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event as Sse, KeepAlive, Sse as SseResponse};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::stream::{self, Stream};
use lh_team::*;
use std::result::Result;
use lh_tools::{Answer, AskRequest, Asker, BoxFut, ExtraTool, Mode, Model, Risk, ToolError};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::{oneshot, Notify};

use crate::session::Session;
use crate::{ApiError, AppState};

const QUESTION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3600);

pub struct TeamRuntime {
    dir: std::path::PathBuf,
    pub roles: Arc<RoleBook>,
    pub depts: Arc<DeptBook>,
    pub registry: Arc<Registry>,
    /// Token limit for workers and leads. Thinking models need room.
    pub max_tokens: u32,
    runs: Mutex<HashMap<String, Arc<Run>>>,
    next: AtomicU64,
}

impl TeamRuntime {
    /// Opens the role and department folders under `dir` and writes the starter roles the first time.
    pub fn open(dir: &FsPath, learn: Arc<lh_learn::Learn>, max_tokens: u32) -> Result<Self, lh_team::Error> {
        let roles = Arc::new(RoleBook::open(dir.join("roles"), learn.clone())?);
        roles.install_starters()?;
        let depts = Arc::new(DeptBook::open(dir.join("departments"), learn)?);
        Ok(Self { dir: dir.to_path_buf(), roles, depts, registry: Arc::new(Registry::default()), max_tokens, runs: Mutex::new(HashMap::new()), next: AtomicU64::new(1) })
    }
}

/// One goal in one department.
pub struct Run {
    pub id: String,
    pub dept: String,
    pub goal: String,
    /// The chat session that started this run, if a chat did.
    pub session: Option<String>,
    log: Mutex<Vec<Value>>,
    notify: Notify,
    finished: AtomicBool,
    questions: Mutex<HashMap<String, oneshot::Sender<Value>>>,
    next_q: AtomicU64,
}

impl Run {
    fn new(id: String, dept: String, goal: String, session: Option<String>) -> Arc<Self> {
        Arc::new(Self { id, dept, goal, session, log: Mutex::new(vec![]), notify: Notify::new(), finished: AtomicBool::new(false), questions: Mutex::new(HashMap::new()), next_q: AtomicU64::new(1) })
    }
    fn push(&self, v: Value) {
        self.log.lock().unwrap().push(v);
        self.notify.notify_waiters();
    }
    fn finish(&self, outcome: Value) {
        self.push(outcome);
        self.finished.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
    }
    /// Ask the user and wait. `None` if nobody answered in time.
    async fn question(&self, mut ev: Value) -> Option<Value> {
        let id = format!("q{}", self.next_q.fetch_add(1, Ordering::SeqCst));
        ev["id"] = json!(id);
        let (tx, rx) = oneshot::channel();
        self.questions.lock().unwrap().insert(id.clone(), tx);
        self.push(ev);
        let a = tokio::time::timeout(QUESTION_TIMEOUT, rx).await.ok().and_then(|r| r.ok());
        self.questions.lock().unwrap().remove(&id);
        a
    }
    pub fn events(&self) -> Vec<Value> {
        self.log.lock().unwrap().clone()
    }
    pub fn is_finished(&self) -> bool {
        self.finished.load(Ordering::SeqCst)
    }
}

fn step_from(v: &Value) -> Option<PlanStep> {
    let s = |k: &str| v[k].as_str().map(str::to_string);
    Some(PlanStep {
        id: s("id")?,
        title: s("title")?,
        assignee: s("assignee")?,
        brief: s("brief").unwrap_or_default(),
        depends_on: v["depends_on"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default(),
        milestone: s("milestone"),
    })
}

struct RunApprover(Arc<Run>, Option<Arc<Session>>);
impl Approver for RunApprover {
    fn plan<'a>(&'a self, dept: &'a str, plan: &'a Plan) -> BoxFut<'a, PlanDecision> {
        Box::pin(async move {
            if let Some(sess) = &self.1 {
                let list = plan.steps.iter().map(|s| format!("{}. {} ({})", s.id, s.title, s.assignee)).collect::<Vec<_>>().join("; ");
                let a = sess.ask_user("call_department", format!("run this plan in {dept}: {list}"), "the department planned the work and waits for a yes".into(), dept, String::new()).await;
                return if a == Answer::Yes { PlanDecision::Approve } else { PlanDecision::Reject("the user did not approve the plan".into()) };
            }
            let steps: Vec<Value> = plan.steps.iter().map(|s| json!({"id": s.id, "title": s.title, "assignee": s.assignee, "brief": s.brief, "depends_on": s.depends_on, "milestone": s.milestone})).collect();
            let a = self.0.question(json!({"type": "plan_needed", "dept": dept, "goal": plan.goal, "steps": steps})).await;
            match a.as_ref().and_then(|v| v["decision"].as_str()) {
                Some("approve") => PlanDecision::Approve,
                Some("edit") => match a.as_ref().and_then(|v| v["steps"].as_array()).map(|l| l.iter().map(step_from).collect::<Option<Vec<_>>>()) {
                    Some(Some(s)) => PlanDecision::Edit(s),
                    _ => PlanDecision::Reject("the edited plan could not be read".into()),
                },
                Some("reject") => PlanDecision::Reject(a.as_ref().and_then(|v| v["note"].as_str()).unwrap_or("").to_string()),
                _ => PlanDecision::Reject("nobody answered".into()),
            }
        })
    }
    fn checkin<'a>(&'a self, dept: &'a str, milestone: &'a str, summaries: &'a [String]) -> BoxFut<'a, bool> {
        Box::pin(async move {
            if let Some(sess) = &self.1 {
                return sess.ask_user("call_department", format!("{dept} finished {milestone}: {}. Go on?", summaries.join("; ")), "check-in".into(), dept, String::new()).await == Answer::Yes;
            }
            let a = self.0.question(json!({"type": "checkin_needed", "dept": dept, "milestone": milestone, "summaries": summaries})).await;
            a.as_ref().and_then(|v| v["decision"].as_str()) == Some("continue")
        })
    }
}

/// Permission questions from workers go to whoever follows the run. In away mode nobody is asked: the
/// answer is no, it is said in the run's events, and the step is set aside.
pub struct RunAsker {
    run: Arc<Run>,
    store: Arc<lh_store::Store>,
    parent: Option<Arc<Session>>,
}
impl Asker for RunAsker {
    async fn ask(&self, r: &AskRequest<'_>) -> Answer {
        if TeamSettings::new(&self.store).away_since().ok().flatten().is_some() {
            self.run.push(json!({"type": "away_denied", "tool": r.tool, "what": r.what, "worker": r.worker, "reason": r.reason}));
            let _ = self.store.log("team", &self.run.dept, "assistant", &json!({"away_decision": "said no", "tool": r.tool, "what": r.what, "worker": r.worker}));
            return Answer::No;
        }
        if let Some(sess) = &self.parent {
            return sess.ask_user(r.tool, r.what.clone(), r.reason.clone(), r.worker, r.detail.clone()).await;
        }
        let a = self.run.question(json!({"type": "ask", "tool": r.tool, "what": r.what, "reason": r.reason, "worker": r.worker, "detail": r.detail})).await;
        if a.as_ref().and_then(|v| v["answer"].as_str()) == Some("yes") {
            Answer::Yes
        } else {
            Answer::No
        }
    }
}

fn outcome_json(o: &DeptOutcome) -> Value {
    let steps = |s: &[StepSummary]| s.iter().map(|x| json!({"id": x.id, "title": x.title, "worker": x.worker, "state": format!("{:?}", x.state).to_lowercase(), "summary": x.summary})).collect::<Vec<_>>();
    match o {
        DeptOutcome::Done { result, steps: s, parked } => json!({"type": "outcome", "status": "done", "result": result, "steps": steps(s), "parked": parked}),
        DeptOutcome::Failed { report, steps: s } => json!({"type": "outcome", "status": "failed", "report": report, "steps": steps(s)}),
        DeptOutcome::Rejected(w) => json!({"type": "outcome", "status": "rejected", "note": w}),
        DeptOutcome::Stopped { steps: s } => json!({"type": "outcome", "status": "stopped", "steps": steps(s)}),
    }
}

fn team<M>(st: &AppState<M>) -> Result<&TeamRuntime, ApiError> {
    st.settings.team.as_ref().ok_or(ApiError(StatusCode::NOT_FOUND, "departments are switched off".into()))
}

fn err(e: lh_team::Error) -> ApiError {
    use lh_team::Error::*;
    let code = match &e {
        NotFound(_) => StatusCode::NOT_FOUND,
        Invalid(_) | NeedsReassign(_) => StatusCode::BAD_REQUEST,
        Unavailable(_) => StatusCode::CONFLICT,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    ApiError(code, e.to_string())
}

fn bad(m: impl Into<String>) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, m.into())
}

fn role_json(r: &Role) -> Value {
    let (kind, value) = match &r.target {
        Target::Type(t) => ("type", t.clone()),
        Target::Model(m) => ("model", m.clone()),
    };
    let memory = match &r.memory {
        MemorySlice::None => "none".to_string(),
        MemorySlice::All => "all".to_string(),
        MemorySlice::Layers(l) => l.iter().map(|x| x.as_str()).collect::<Vec<_>>().join(" "),
    };
    let sm = &r.sampling;
    json!({"name": r.name, "description": r.description, "target": {"kind": kind, "value": value}, "tools": r.tools, "memory": memory, "sampling": {"temperature": sm.temperature, "top_p": sm.top_p, "top_k": sm.top_k, "min_p": sm.min_p, "repeat_penalty": sm.repeat_penalty}, "instructions": r.instructions, "max_steps": r.max_steps, "thinking": r.thinking, "text": r.render()})
}

fn dept_json(d: &Department) -> Value {
    let members: Vec<String> = d.members.iter().map(|m| match m { Member::Role(r) => r.clone(), Member::Dept(x) => format!("dept:{x}") }).collect();
    json!({"name": d.name, "description": d.description, "lead": d.lead, "members": members, "parallel": d.parallel, "attempts": d.attempts, "instructions": d.instructions, "text": d.render()})
}

fn type_json(t: &ModelType) -> Value {
    json!({"name": t.name, "model": t.model, "fallbacks": t.fallbacks, "stock": t.stock})
}

pub(crate) fn routes<M: Model + 'static>() -> Router<Arc<AppState<M>>> {
    Router::new()
        .route("/v1/team/roles", get(list_roles::<M>))
        .route("/v1/team/roles/{name}", get(get_role::<M>).put(put_role::<M>).delete(delete_role::<M>))
        .route("/v1/team/roles/{name}/history", get(role_history::<M>))
        .route("/v1/team/roles/{name}/rollback", post(role_rollback::<M>))
        .route("/v1/team/types", get(list_types::<M>).post(add_type::<M>))
        .route("/v1/team/types/{name}", axum::routing::put(put_type::<M>).delete(delete_type::<M>))
        .route("/v1/team/departments", get(list_depts::<M>))
        .route("/v1/team/departments/{name}", get(get_dept::<M>).put(put_dept::<M>).delete(delete_dept::<M>))
        .route("/v1/team/departments/{name}/history", get(dept_history::<M>))
        .route("/v1/team/departments/{name}/rollback", post(dept_rollback::<M>))
        .route("/v1/team/settings", get(get_settings::<M>).put(put_settings::<M>))
        .route("/v1/team/plugins/preview", post(plugin_preview::<M>))
        .route("/v1/team/plugins/install", post(plugin_install::<M>))
        .route("/v1/team/away", get(get_away::<M>).post(set_away::<M>))
        .route("/v1/team/workers", get(workers::<M>))
        .route("/v1/team/workers/{id}/{action}", post(worker_action::<M>))
        .route("/v1/team/runs", get(list_runs::<M>).post(start_run::<M>))
        .route("/v1/team/runs/{id}", get(get_run::<M>))
        .route("/v1/team/runs/{id}/events", get(run_events::<M>))
        .route("/v1/team/runs/{id}/answers/{q}", post(run_answer::<M>))
        .route("/v1/team/runs/{id}/stop", post(stop_run::<M>))
}

#[derive(Deserialize)]
struct Text {
    text: String,
}
#[derive(Deserialize)]
struct Version {
    version: i64,
}

async fn list_roles<M: Model + 'static>(State(st): State<Arc<AppState<M>>>) -> Result<Json<Value>, ApiError> {
    let t = team(&st)?;
    let v: Vec<Value> = t.roles.list().map_err(err)?.into_iter().map(|e| match e.role {
        Ok(r) => role_json(&r),
        Err(why) => json!({"name": e.name, "error": why}),
    }).collect();
    Ok(Json(json!({"roles": v})))
}

async fn get_role<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(n): Path<String>) -> Result<Json<Value>, ApiError> {
    Ok(Json(role_json(&team(&st)?.roles.get(&n).map_err(err)?)))
}

async fn put_role<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(n): Path<String>, Json(b): Json<Text>) -> Result<Json<Value>, ApiError> {
    let mut r = Role::parse(&b.text, Some(&n)).map_err(bad)?;
    r.name = n;
    let v = team(&st)?.roles.save(&r, "user").map_err(err)?;
    Ok(Json(json!({"version": v, "role": role_json(&r)})))
}

async fn delete_role<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(n): Path<String>) -> Result<StatusCode, ApiError> {
    let t = team(&st)?;
    // a role that is a lead or a member somewhere would leave a broken department behind
    for (dn, d) in t.depts.list().map_err(err)? {
        if let Ok(d) = d {
            if d.lead == n || d.members.contains(&Member::Role(n.clone())) {
                return Err(bad(format!("the department {dn} uses this role; change the department first")));
            }
        }
    }
    t.roles.delete(&n, "user").map_err(err)?;
    Ok(StatusCode::NO_CONTENT)
}

fn history_json(h: Vec<lh_learn::HistoryEntry>) -> Value {
    json!({"versions": h.iter().map(|e| json!({"version": e.version, "op": e.op, "by": e.by, "at": e.at, "note": e.note})).collect::<Vec<_>>()})
}

async fn role_history<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(n): Path<String>) -> Result<Json<Value>, ApiError> {
    Ok(Json(history_json(team(&st)?.roles.history(&n).map_err(err)?)))
}

async fn role_rollback<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(n): Path<String>, Json(b): Json<Version>) -> Result<Json<Value>, ApiError> {
    let v = team(&st)?.roles.rollback(&n, b.version, "user").map_err(err)?;
    Ok(Json(json!({"version": v})))
}

async fn list_types<M: Model + 'static>(State(st): State<Arc<AppState<M>>>) -> Result<Json<Value>, ApiError> {
    let v: Vec<Value> = ModelTypes::new(&st.settings.store).list().map_err(err)?.iter().map(type_json).collect();
    Ok(Json(json!({"types": v})))
}

#[derive(Deserialize)]
struct NewType {
    name: String,
    model: Option<String>,
}
async fn add_type<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Json(b): Json<NewType>) -> Result<Json<Value>, ApiError> {
    Ok(Json(type_json(&ModelTypes::new(&st.settings.store).add(&b.name, b.model.as_deref()).map_err(err)?)))
}

#[derive(Deserialize)]
struct TypeEdit {
    #[serde(default, deserialize_with = "present")]
    model: Option<Option<String>>,
    fallbacks: Option<Vec<String>>,
}
/// Tells "not given" from "given as null".
fn present<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Option<String>>, D::Error> {
    Option::<String>::deserialize(d).map(Some)
}
async fn put_type<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(n): Path<String>, Json(b): Json<TypeEdit>) -> Result<Json<Value>, ApiError> {
    let t = ModelTypes::new(&st.settings.store);
    let mut out = t.get(&n).map_err(err)?;
    if let Some(m) = b.model {
        out = t.set_model(&n, m.as_deref()).map_err(err)?;
    }
    if let Some(f) = b.fallbacks {
        out = t.set_fallbacks(&n, &f).map_err(err)?;
    }
    Ok(Json(type_json(&out)))
}

#[derive(Deserialize, Default)]
struct DeleteType {
    /// Role name to its new target: `{"type": "mid-tier"}` or `{"model": "name"}`.
    #[serde(default)]
    reassign: BTreeMap<String, Value>,
}
async fn delete_type<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(n): Path<String>, body: Option<Json<DeleteType>>) -> Result<Json<Value>, ApiError> {
    let t = team(&st)?;
    let mut map = BTreeMap::new();
    for (role, v) in body.map(|b| b.0.reassign).unwrap_or_default() {
        let target = match (v["type"].as_str(), v["model"].as_str()) {
            (Some(x), None) => Target::Type(x.to_string()),
            (None, Some(x)) => Target::Model(x.to_string()),
            _ => return Err(bad(format!("the new home of {role} must be {{\"type\": ...}} or {{\"model\": ...}}"))),
        };
        map.insert(role, target);
    }
    match ModelTypes::new(&st.settings.store).delete(&n, &t.roles, &map) {
        Ok(()) => Ok(Json(json!({"deleted": n}))),
        Err(lh_team::Error::NeedsReassign(roles)) => Err(ApiError(StatusCode::CONFLICT, format!("these roles use the type and need another one first: {}", roles.join(", ")))),
        Err(e) => Err(err(e)),
    }
}

async fn list_depts<M: Model + 'static>(State(st): State<Arc<AppState<M>>>) -> Result<Json<Value>, ApiError> {
    let v: Vec<Value> = team(&st)?.depts.list().map_err(err)?.into_iter().map(|(n, d)| match d {
        Ok(d) => dept_json(&d),
        Err(why) => json!({"name": n, "error": why}),
    }).collect();
    Ok(Json(json!({"departments": v})))
}

async fn get_dept<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(n): Path<String>) -> Result<Json<Value>, ApiError> {
    Ok(Json(dept_json(&team(&st)?.depts.get(&n).map_err(err)?)))
}

async fn put_dept<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(n): Path<String>, Json(b): Json<Text>) -> Result<Json<Value>, ApiError> {
    let t = team(&st)?;
    let mut d = Department::parse(&b.text, Some(&n)).map_err(bad)?;
    d.name = n;
    let v = t.depts.save(&d, &t.roles, "user").map_err(err)?;
    Ok(Json(json!({"version": v, "department": dept_json(&d)})))
}

async fn delete_dept<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(n): Path<String>) -> Result<StatusCode, ApiError> {
    team(&st)?.depts.delete(&n, "user").map_err(err)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn dept_history<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(n): Path<String>) -> Result<Json<Value>, ApiError> {
    // departments keep their versions under the same history table, keyed by kind
    let h = lh_learn::Learn::new(st.settings.store.clone()).history("department", &n).map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(history_json(h)))
}

async fn dept_rollback<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(n): Path<String>, Json(b): Json<Version>) -> Result<Json<Value>, ApiError> {
    let v = team(&st)?.depts.rollback(&n, b.version, "user").map_err(err)?;
    Ok(Json(json!({"version": v})))
}

fn settings_json(s: &TeamSettings) -> Result<Value, ApiError> {
    Ok(json!({
        "parallel": s.parallel().map_err(err)?,
        "autonomy": s.autonomy().map_err(err)?.as_str(),
        "attempts": s.attempts().map_err(err)?,
        "unavailable": match s.unavailable().map_err(err)? { UnavailablePolicy::Stop => "stop", UnavailablePolicy::Fallback => "fallback" },
        "away_since": s.away_since().map_err(err)?,
        "limits": {"parallel_max": lh_team::MAX_PARALLEL, "attempts_max": lh_team::MAX_ATTEMPTS},
    }))
}

async fn get_settings<M: Model + 'static>(State(st): State<Arc<AppState<M>>>) -> Result<Json<Value>, ApiError> {
    Ok(Json(settings_json(&TeamSettings::new(&st.settings.store))?))
}

async fn put_settings<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Json(b): Json<Value>) -> Result<Json<Value>, ApiError> {
    let s = TeamSettings::new(&st.settings.store);
    if let Some(n) = b["parallel"].as_u64() {
        s.set_parallel(n as usize).map_err(err)?;
        crate::settings_api::mirror_team(&st, "team.parallel", &n.to_string());
    }
    if let Some(n) = b["attempts"].as_u64() {
        s.set_attempts(n as u32).map_err(err)?;
        crate::settings_api::mirror_team(&st, "team.attempts", &n.to_string());
    }
    if let Some(a) = b["autonomy"].as_str() {
        s.set_autonomy(Autonomy::parse(a).ok_or_else(|| bad("autonomy must be plan-first, check-ins or fire-and-forget"))?).map_err(err)?;
        crate::settings_api::mirror_team(&st, "team.autonomy", a);
    }
    if let Some(u) = b["unavailable"].as_str() {
        s.set_unavailable(match u { "stop" => UnavailablePolicy::Stop, "fallback" => UnavailablePolicy::Fallback, _ => return Err(bad("unavailable must be stop or fallback")) }).map_err(err)?;
        crate::settings_api::mirror_team(&st, "team.unavailable", u);
    }
    Ok(Json(settings_json(&s)?))
}

async fn get_away<M: Model + 'static>(State(st): State<Arc<AppState<M>>>) -> Result<Json<Value>, ApiError> {
    let since = TeamSettings::new(&st.settings.store).away_since().map_err(err)?;
    let entries = st.settings.store.recent(500, Some("team")).map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let decisions: Vec<Value> = entries.iter().filter(|e| e.detail.get("away_decision").is_some() && since.is_none_or(|s| e.at >= s)).map(|e| json!({"at": e.at, "dept": e.session, "decision": e.detail["away_decision"], "detail": e.detail})).collect();
    let parked: Vec<Value> = st.settings.team.iter().flat_map(|t| t.runs.lock().unwrap().values().cloned().collect::<Vec<_>>()).flat_map(|r| r.events()).filter(|e| e["type"] == "outcome").flat_map(|e| e["parked"].as_array().cloned().unwrap_or_default()).collect();
    Ok(Json(json!({"away": since.is_some(), "since": since, "decisions": decisions, "parked": parked})))
}

#[derive(Deserialize)]
struct Away {
    on: bool,
}
async fn set_away<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Json(b): Json<Away>) -> Result<Json<Value>, ApiError> {
    TeamSettings::new(&st.settings.store).set_away(b.on).map_err(err)?;
    get_away(State(st)).await
}

async fn workers<M: Model + 'static>(State(st): State<Arc<AppState<M>>>) -> Result<Json<Value>, ApiError> {
    let v: Vec<Value> = team(&st)?.registry.list().iter().map(|w| json!({"id": w.id, "dept": w.dept, "role": w.role, "model": w.model, "step": w.step, "status": w.status.as_str(), "attempt": w.attempt})).collect();
    Ok(Json(json!({"workers": v})))
}

async fn worker_action<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path((id, action)): Path<(String, String)>) -> Result<StatusCode, ApiError> {
    let r = &team(&st)?.registry;
    let ok = match action.as_str() {
        "pause" => r.pause(&id),
        "resume" => r.resume(&id),
        "stop" => r.stop(&id),
        _ => return Err(bad("action must be pause, resume or stop")),
    };
    if ok { Ok(StatusCode::NO_CONTENT) } else { Err(ApiError(StatusCode::NOT_FOUND, "no such worker, or it already finished".into())) }
}

#[derive(Deserialize)]
struct NewRun {
    department: String,
    goal: String,
    project_dir: Option<String>,
    /// The chat session the run is for, so a front end can show it with that chat.
    session: Option<String>,
}

#[derive(Deserialize)]
struct RunFilter {
    session: Option<String>,
}

/// Start a run in the background. Used by the API and by the `call_department` tool.
pub(crate) async fn start<M: Model + 'static>(st: &Arc<AppState<M>>, dept: String, goal: String, project: Option<String>, parent: Option<Arc<Session>>, chat: Option<String>) -> Result<Arc<Run>, ApiError> {
    let t = team(st)?;
    t.depts.get(&dept).map_err(err)?;
    let project = match project {
        Some(p) => Some(std::fs::canonicalize(&p).map_err(|e| bad(format!("project_dir: {e}")))?.to_string_lossy().to_string()),
        None => None,
    };
    let id = format!("r{}", t.next.fetch_add(1, Ordering::SeqCst));
    // a run the chat's own assistant started belongs to that chat
    let chat = chat.or_else(|| parent.as_ref().and_then(|p| st.sessions.lock().unwrap().iter().find(|(_, s)| Arc::ptr_eq(s, p)).map(|(k, _)| k.clone())));
    let run = Run::new(id.clone(), dept.clone(), goal.clone(), chat);
    t.runs.lock().unwrap().insert(id, run.clone());
    let s = &st.settings;
    let provider_name = s.provider_name.clone();
    let mcp_tools = match &s.mcp {
        Some(m) => m.tools_for(project.as_deref()).await,
        None => vec![],
    };
    let st2 = st.clone();
    let toolbox: Toolbox = Arc::new(move |role, job| {
        let s = &st2.settings;
        let live = st2.live();
        let mut tools = lh_tools::default_tools().with_web(live.web.clone());
        if let Some(g) = crate::outbound_guard(s) {
            tools = tools.with_outbound_guard(g);
        }
        if let Some(m) = &live.image_model {
            tools = tools.with_images(m.clone());
        }
        if let Some(rt) = &s.memory {
            let layers = match &role.memory {
                MemorySlice::None => None,
                MemorySlice::All => Some(None),
                MemorySlice::Layers(l) => Some(Some(l.clone())),
            };
            if let Some(layers) = layers {
                tools = tools.with_memory(Arc::new(lh_tools::MemoryHandle { memory: rt.memory.clone(), embedder: rt.embedder.clone(), dest: rt.destination(job.project.clone()), cfg: live.recall.clone(), layers, judge: None }));
            }
        }
        if let Some(lr) = &s.learn {
            tools = tools.with_skills(lr.learn.clone());
        }
        for x in &mcp_tools {
            tools = tools.with_extra(x.clone());
        }
        tools_for_role(&tools, role)
    });
    let backend = ModelBackend {
        model: st.model.clone(),
        meter: Arc::new(s.spend.for_session(&run.id)),
        asker: Arc::new(RunAsker { run: run.clone(), store: s.store.clone(), parent: parent.clone() }),
        env: WorkerEnv {
            home: std::env::var_os("HOME").map(Into::into).unwrap_or_else(std::env::temp_dir),
            mode: Mode::Tiered, // bypass and sudo come from `live`, at every tool call
            sudo_enabled: s.sudo_enabled,
            allow_rules: s.allow_rules.clone(),
            live: Some(st.live_perms()),
        },
        toolbox,
        provider_name,
        max_tokens: t.max_tokens,
    };
    let r2 = run.clone();
    let engine = Engine::new(backend, s.store.clone(), t.roles.clone(), t.depts.clone(), t.registry.clone(), Arc::new(move |e| r2.push(e.to_json())), Arc::new(RunApprover(run.clone(), parent)));
    let r3 = run.clone();
    tokio::spawn(async move {
        let out = engine.run_department(dept, goal, RunCtx { project, depth: 0 }).await;
        r3.finish(outcome_json(&out));
    });
    Ok(run)
}

async fn start_run<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, via: Option<axum::Extension<crate::web::Via>>, Json(b): Json<NewRun>) -> Result<(StatusCode, Json<Value>), ApiError> {
    if let (Some(axum::Extension(crate::web::Via::Device(_))), Some(d)) = (&via, &b.project_dir) {
        crate::web::device_folder_ok(std::path::Path::new(d)).map_err(|e| ApiError(StatusCode::FORBIDDEN, e))?;
    }
    let run = start(&st, b.department, b.goal, b.project_dir, None, b.session).await?;
    Ok((StatusCode::CREATED, Json(json!({"id": run.id}))))
}

fn run_of<M>(st: &AppState<M>, id: &str) -> Result<Arc<Run>, ApiError> {
    team(st)?.runs.lock().unwrap().get(id).cloned().ok_or(ApiError(StatusCode::NOT_FOUND, "no such run".into()))
}

fn run_summary(r: &Run) -> Value {
    json!({"id": r.id, "department": r.dept, "goal": r.goal, "session": r.session, "finished": r.is_finished()})
}

/// Every run, or with `?session=` only the runs that chat started.
async fn list_runs<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Query(f): Query<RunFilter>) -> Result<Json<Value>, ApiError> {
    let mut v: Vec<Arc<Run>> = team(&st)?.runs.lock().unwrap().values().filter(|r| f.session.is_none() || r.session == f.session).cloned().collect();
    v.sort_by_key(|r| r.id[1..].parse::<u64>().unwrap_or(0));
    Ok(Json(json!({"runs": v.iter().map(|r| run_summary(r)).collect::<Vec<_>>()})))
}

async fn get_run<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    let r = run_of(&st, &id)?;
    let mut j = run_summary(&r);
    j["events"] = json!(r.events());
    Ok(Json(j))
}

async fn run_events<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<String>) -> Result<SseResponse<impl Stream<Item = Result<Sse, Infallible>>>, ApiError> {
    let run = run_of(&st, &id)?;
    let s = stream::unfold((run, 0usize), |(run, i)| async move {
        loop {
            let item = {
                let notified = run.notify.notified();
                tokio::pin!(notified);
                notified.as_mut().enable();
                let (item, done) = {
                    let log = run.log.lock().unwrap();
                    (log.get(i).cloned(), run.finished.load(Ordering::SeqCst))
                };
                if item.is_none() {
                    if done {
                        return None;
                    }
                    notified.await;
                }
                item
            };
            if let Some(v) = item {
                let kind = v["type"].as_str().unwrap_or("event").to_string();
                return Some((Ok(Sse::default().event(kind).data(v.to_string())), (run, i + 1)));
            }
        }
    });
    Ok(SseResponse::new(s).keep_alive(KeepAlive::default()))
}

async fn run_answer<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path((id, q)): Path<(String, String)>, Json(b): Json<Value>) -> Result<StatusCode, ApiError> {
    let run = run_of(&st, &id)?;
    let tx = run.questions.lock().unwrap().remove(&q);
    if tx.is_some_and(|tx| tx.send(b).is_ok()) {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError(StatusCode::NOT_FOUND, "no such open question".into()))
    }
}

async fn stop_run<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<String>) -> Result<StatusCode, ApiError> {
    let run = run_of(&st, &id)?;
    // answer every open question with a no, then stop the workers that belong to this department
    run.questions.lock().unwrap().clear();
    let t = team(&st)?;
    for w in t.registry.list().into_iter().filter(|w| w.dept == run.dept) {
        t.registry.stop(&w.id);
    }
    Ok(StatusCode::NO_CONTENT)
}

/// `call_department` and `start_project`: how the assistant hands work to a department. A project is
/// made in its own folder and its workers are locked to it; they never get either of these tools, so
/// a project cannot start another project.
pub(crate) struct DeptTool<M> {
    pub st: std::sync::Weak<AppState<M>>,
    pub session: Arc<std::sync::OnceLock<std::sync::Weak<Session>>>,
    pub project: bool,
}

impl<M> std::fmt::Debug for DeptTool<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeptTool").field("project", &self.project).finish()
    }
}

fn slug(name: &str) -> String {
    let s: String = name.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect();
    s.split('-').filter(|w| !w.is_empty()).collect::<Vec<_>>().join("-")
}

impl<M: Model + 'static> ExtraTool for DeptTool<M> {
    fn def(&self) -> lh_provider::ToolDef {
        if self.project {
            lh_provider::ToolDef {
                name: "start_project".into(),
                description: "Make a new project in its own folder with a department. Use when the user asks to make or build something as a project.".into(),
                parameters: json!({"type": "object", "properties": {"name": {"type": "string"}, "goal": {"type": "string"}, "department": {"type": "string"}, "location": {"type": "string", "description": "Folder to create; leave out to use ~/Projects/<name>"}}, "required": ["name", "goal", "department"]}),
            }
        } else {
            lh_provider::ToolDef {
                name: "call_department".into(),
                description: "Give a goal to a department and get its result. For jobs in the current folder that are not a new project.".into(),
                parameters: json!({"type": "object", "properties": {"department": {"type": "string"}, "goal": {"type": "string"}}, "required": ["department", "goal"]}),
            }
        }
    }
    fn risk(&self) -> Risk {
        Risk::Writes
    }
    fn describe(&self, a: &Value) -> String {
        if self.project {
            format!("start project {} with {}", a["name"].as_str().unwrap_or("?"), a["department"].as_str().unwrap_or("?"))
        } else {
            format!("ask {} to: {}", a["department"].as_str().unwrap_or("?"), lh_team::excerpt(a["goal"].as_str().unwrap_or(""), 120))
        }
    }
    fn guideline(&self) -> Option<String> {
        self.project.then(|| "Departments do bigger jobs: call_department for a job here, start_project for a new project.".to_string())
    }
    fn run(&self, a: Value) -> BoxFut<'_, Result<String, ToolError>> {
        Box::pin(async move {
            let st = self.st.upgrade().ok_or(ToolError::Failed("the core is shutting down".into()))?;
            let session = self.session.get().and_then(|w| w.upgrade());
            let arg = |k: &str| a[k].as_str().map(str::trim).filter(|v| !v.is_empty()).map(str::to_string);
            let (dept, goal) = (arg("department").ok_or(ToolError::Args("department is missing".into()))?, arg("goal").ok_or(ToolError::Args("goal is missing".into()))?);
            if let Some(t) = &st.settings.team {
                if t.depts.get(&dept).is_err() {
                    let names: Vec<String> = t.depts.list().map(|l| l.into_iter().map(|x| x.0).collect()).unwrap_or_default();
                    return Err(ToolError::Failed(format!("there is no department called {dept}; the departments are: {}", if names.is_empty() { "none yet".to_string() } else { names.join(", ") })));
                }
            }
            let mut project = session.as_ref().and_then(|s| s.project.clone());
            let mut note = String::new();
            if self.project {
                let name = arg("name").ok_or(ToolError::Args("name is missing".into()))?;
                let path = match arg("location") {
                    Some(l) => std::path::PathBuf::from(l),
                    None => std::env::var_os("HOME").map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir).join("Projects").join(slug(&name)),
                };
                if path.exists() && std::fs::read_dir(&path).map(|mut d| d.next().is_some()).unwrap_or(true) {
                    return Err(ToolError::Failed(format!("{} already exists and is not empty; pick another name or location", path.display())));
                }
                std::fs::create_dir_all(&path).map_err(|e| ToolError::Failed(format!("cannot create {}: {e}", path.display())))?;
                note = format!("Project {name} created in {}.\n", path.display());
                project = Some(path.to_string_lossy().to_string());
            }
            let run = start(&st, dept, goal, project, session, None).await.map_err(|e| ToolError::Failed(e.1))?;
            // wait for the run to end; its questions reach the user through the session
            let mut i = 0;
            loop {
                let notified = run.notify.notified();
                tokio::pin!(notified);
                notified.as_mut().enable();
                if run.is_finished() {
                    break;
                }
                let _ = i;
                i += 1;
                tokio::time::timeout(std::time::Duration::from_secs(5), notified).await.ok();
            }
            let out = run.events().into_iter().rev().find(|e| e["type"] == "outcome").unwrap_or(Value::Null);
            let parked = out["parked"].as_array().map(|p| p.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join("\n- ")).unwrap_or_default();
            let tail = if parked.is_empty() { String::new() } else { format!("\nSet aside for you:\n- {parked}") };
            match out["status"].as_str() {
                Some("done") => Ok(format!("{note}{}{tail}", out["result"].as_str().unwrap_or(""))),
                Some("failed") => Err(ToolError::Failed(format!("{note}{}", out["report"].as_str().unwrap_or("the department failed")))),
                Some("rejected") => Err(ToolError::Failed(format!("{note}the plan was not approved: {}", out["note"].as_str().unwrap_or("")))),
                _ => Err(ToolError::Failed(format!("{note}the work was stopped"))),
            }
        })
    }
}

#[derive(Deserialize)]
struct PluginReq {
    path: String,
    /// Install for real; without it nothing is installed.
    #[serde(default)]
    accept: bool,
    /// Roles and departments the plugin may replace.
    #[serde(default)]
    overwrite: Vec<String>,
}

fn items_json(v: &[lh_team::plugin::Item]) -> Value {
    json!(v.iter().map(|i| json!({"name": i.name, "description": i.description, "exists": i.exists})).collect::<Vec<_>>())
}

async fn plugin_preview<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Json(b): Json<PluginReq>) -> Result<Json<Value>, ApiError> {
    let t = team(&st)?;
    let p = lh_team::plugin::preview(FsPath::new(&b.path), &t.roles, &t.depts).map_err(err)?;
    Ok(Json(json!({"name": p.name, "description": p.description, "roles": items_json(&p.roles), "departments": items_json(&p.departments), "themes": p.themes, "problems": p.problems})))
}

async fn plugin_install<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Json(b): Json<PluginReq>) -> Result<Json<Value>, ApiError> {
    if !b.accept {
        return Err(bad("a plugin is installed only when you accept it after the preview (accept: true)"));
    }
    let t = team(&st)?;
    let r = lh_team::plugin::install(FsPath::new(&b.path), &t.roles, &t.depts, &t.dir.join("themes"), &b.overwrite, "plugin").map_err(err)?;
    let _ = st.settings.store.log("plugin", &b.path, "user", &json!({"installed": r.installed, "skipped": r.skipped}));
    Ok(Json(json!({"installed": r.installed, "skipped": r.skipped})))
}
