//! The engine: runs a goal through a department.
//!
//! The lead plans (and may be asked to correct the plan), the user approves it if the autonomy level
//! says so, then steps run on the members: one at a time or several at once up to the limit, each as soon
//! as the steps it waits for are done. The lead checks every result against its step. Small mistakes are
//! fixed in place by a fresh instance of the same worker; larger misses and failures go to the lead,
//! who may retry, change the model, do the step itself, hand it to a regular subagent, hand it to
//! another member, or give up. Every step has an attempt cap. A failure the lead cannot resolve goes up
//! to whoever called the department, and so on to the top.
//!
//! The engine never talks to a model itself. A `Backend` does that, which keeps this logic testable.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use lh_store::Store;
use lh_tools::BoxFut;
use serde_json::{json, Value};
use tokio::task::JoinSet;

use crate::control::{Registry, Status, StepControl, WorkerInfo};
use crate::{check_prompt, excerpt, failure_prompt, final_prompt, parse_check, parse_plan, parse_remedy, plan_prompt, Autonomy, Check, Department, DeptBook, Member, ModelTypes, Plan, PlanStep, Remedy, Resolution, Role, RoleBook, Target, TeamSettings};

/// Everything a worker needs to do one step.
#[derive(Clone, Debug)]
pub struct StepJob {
    pub id: String,
    pub dept: String,
    pub role: Role,
    pub model: String,
    pub brief: String,
    /// Results of the steps this one waited for: (title, result).
    pub inputs: Vec<(String, String)>,
    /// Set when a fresh instance is asked to correct a small mistake in an earlier result.
    pub fix: Option<Fix>,
    pub attempt: u32,
    pub project: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Fix {
    pub previous: String,
    pub instruction: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StepResult {
    Done { result: String, summary: String },
    Failed(String),
    /// Stopped by the user.
    Stopped,
    /// Nobody was there to decide something the worker needed; set aside for the user.
    Parked(String),
}

/// The part that talks to models. The real one runs the turn loop; tests use a script.
pub trait Backend: Send + Sync + 'static {
    /// Run one worker on one step. It should call `ctl.checkpoint()` between model calls.
    fn run_step(&self, job: StepJob, ctl: StepControl) -> impl std::future::Future<Output = StepResult> + Send;
    /// One model call for a role: the lead planning, checking, deciding. Text in, text out.
    fn ask(&self, role: &Role, model: &str, prompt: String) -> impl std::future::Future<Output = Result<String, String>> + Send;
    /// The models the server offers. Empty means it cannot say.
    fn models(&self) -> impl std::future::Future<Output = Vec<String>> + Send;
}

#[derive(Clone, Debug)]
pub enum TeamEvent {
    Planning { dept: String },
    Plan { dept: String, steps: Vec<PlanStep> },
    PlanDecided { dept: String, approved: bool, note: String },
    StepStarted { dept: String, step: String, title: String, worker: String, model: String, attempt: u32 },
    StepChecked { step: String, verdict: String },
    StepRetry { step: String, remedy: String, reason: String },
    StepDone { step: String, summary: String },
    StepFailed { step: String, report: String },
    StepParked { step: String, why: String },
    StepStopped { step: String },
    StepBlocked { step: String, because: String },
    ModelNote { note: String },
    ModelUnavailable { role: String, message: String },
    Milestone { dept: String, name: String },
    Finished { dept: String, ok: bool },
}

impl TeamEvent {
    pub fn to_json(&self) -> Value {
        use TeamEvent::*;
        match self {
            Planning { dept } => json!({"type":"planning","dept":dept}),
            Plan { dept, steps } => json!({"type":"plan","dept":dept,"steps":steps.iter().map(|s| json!({"id":s.id,"title":s.title,"assignee":s.assignee,"brief":s.brief,"depends_on":s.depends_on,"milestone":s.milestone})).collect::<Vec<_>>()}),
            PlanDecided { dept, approved, note } => json!({"type":"plan_decided","dept":dept,"approved":approved,"note":note}),
            StepStarted { dept, step, title, worker, model, attempt } => json!({"type":"step_started","dept":dept,"step":step,"title":title,"worker":worker,"model":model,"attempt":attempt}),
            StepChecked { step, verdict } => json!({"type":"step_checked","step":step,"verdict":verdict}),
            StepRetry { step, remedy, reason } => json!({"type":"step_retry","step":step,"remedy":remedy,"reason":reason}),
            StepDone { step, summary } => json!({"type":"step_done","step":step,"summary":summary}),
            StepFailed { step, report } => json!({"type":"step_failed","step":step,"report":report}),
            StepParked { step, why } => json!({"type":"step_parked","step":step,"why":why}),
            StepStopped { step } => json!({"type":"step_stopped","step":step}),
            StepBlocked { step, because } => json!({"type":"step_blocked","step":step,"because":because}),
            ModelNote { note } => json!({"type":"model_note","note":note}),
            ModelUnavailable { role, message } => json!({"type":"model_unavailable","role":role,"message":message}),
            Milestone { dept, name } => json!({"type":"milestone","dept":dept,"name":name}),
            Finished { dept, ok } => json!({"type":"finished","dept":dept,"ok":ok}),
        }
    }
}

pub type Events = Arc<dyn Fn(TeamEvent) + Send + Sync>;

pub enum PlanDecision {
    Approve,
    /// The user changed the plan; it is checked again before it runs.
    Edit(Vec<PlanStep>),
    Reject(String),
}

/// Whoever approves plans and milestones: the user, or the assistant on the user's behalf in away mode.
pub trait Approver: Send + Sync {
    fn plan<'a>(&'a self, dept: &'a str, plan: &'a Plan) -> BoxFut<'a, PlanDecision>;
    /// `true` to go on after the milestone.
    fn checkin<'a>(&'a self, dept: &'a str, milestone: &'a str, summaries: &'a [String]) -> BoxFut<'a, bool>;
}

pub struct AutoApprove;
impl Approver for AutoApprove {
    fn plan<'a>(&'a self, _: &'a str, _: &'a Plan) -> BoxFut<'a, PlanDecision> {
        Box::pin(async { PlanDecision::Approve })
    }
    fn checkin<'a>(&'a self, _: &'a str, _: &'a str, _: &'a [String]) -> BoxFut<'a, bool> {
        Box::pin(async { true })
    }
}

#[derive(Clone, Debug, Default)]
pub struct RunCtx {
    pub project: Option<String>,
    /// 0 for a department called by the assistant or the project leader; more for sub-departments.
    pub depth: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepState {
    Done,
    Failed,
    Parked,
    Stopped,
    /// Not run because a step it waited for did not finish.
    Blocked,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StepSummary {
    pub id: String,
    pub title: String,
    pub worker: String,
    pub state: StepState,
    pub summary: String,
    /// What the worker handed back, in full. The lead's closing answer is written from this.
    pub result: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DeptOutcome {
    Done { result: String, steps: Vec<StepSummary>, parked: Vec<String> },
    /// Something could not be done and the department could not fix it. It goes up to the caller.
    Failed { report: String, steps: Vec<StepSummary> },
    /// The user did not approve the plan.
    Rejected(String),
    Stopped { steps: Vec<StepSummary> },
}

struct Inner<B> {
    backend: B,
    store: Arc<Store>,
    roles: Arc<RoleBook>,
    depts: Arc<DeptBook>,
    registry: Arc<Registry>,
    events: Events,
    approver: Arc<dyn Approver>,
}

pub struct Engine<B: Backend> {
    inner: Arc<Inner<B>>,
}

impl<B: Backend> Clone for Engine<B> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

enum Worker {
    Role(Box<Role>),
    Dept(String),
}

impl Worker {
    fn name(&self) -> String {
        match self {
            Worker::Role(r) => r.name.clone(),
            Worker::Dept(d) => format!("dept:{d}"),
        }
    }
}

enum StepEnd {
    Done { result: String, summary: String, worker: String },
    Failed { report: String, worker: String },
    Parked(String),
    Stopped,
}

enum Attempt {
    Done { result: String, summary: String },
    Failed(String),
    Stopped,
    Parked(String),
    Unavailable(String),
}

impl<B: Backend> Engine<B> {
    pub fn new(backend: B, store: Arc<Store>, roles: Arc<RoleBook>, depts: Arc<DeptBook>, registry: Arc<Registry>, events: Events, approver: Arc<dyn Approver>) -> Self {
        Self { inner: Arc::new(Inner { backend, store, roles, depts, registry, events, approver }) }
    }

    pub fn registry(&self) -> &Arc<Registry> {
        &self.inner.registry
    }

    fn emit(&self, e: TeamEvent) {
        (self.inner.events)(e);
    }

    fn log(&self, dept: &str, who: &str, detail: Value) {
        let _ = self.inner.store.log("team", dept, who, &detail);
    }

    /// Run a goal through a department.
    pub fn run_department(&self, name: String, goal: String, ctx: RunCtx) -> BoxFut<'static, DeptOutcome> {
        let me = self.clone();
        Box::pin(async move {
            let out = me.run_dept(&name, &goal, &ctx).await;
            me.emit(TeamEvent::Finished { dept: name.clone(), ok: matches!(out, DeptOutcome::Done { .. }) });
            out
        })
    }

    async fn resolve(&self, role: &Role, over: &Option<Target>) -> Result<Resolution, crate::Error> {
        let target = over.clone().unwrap_or_else(|| role.target.clone());
        let available = self.inner.backend.models().await;
        let policy = TeamSettings::new(&self.inner.store).unavailable()?;
        ModelTypes::new(&self.inner.store).resolve(&target, &available, policy, &|_| true)
    }

    async fn lead_ask(&self, lead: &Role, prompt: String) -> Result<String, String> {
        let r = self.resolve(lead, &None).await.map_err(|e| e.to_string())?;
        if let Some(note) = r.note {
            self.emit(TeamEvent::ModelNote { note });
        }
        self.inner.backend.ask(lead, &r.model, prompt).await
    }

    async fn run_dept(&self, name: &str, goal: &str, ctx: &RunCtx) -> DeptOutcome {
        let fail = |report: String| DeptOutcome::Failed { report, steps: vec![] };
        let (dept, lead) = match (self.inner.depts.get(name), ) {
            (Ok(d), ) => match self.inner.roles.get(&d.lead) {
                Ok(l) => (d, l),
                Err(e) => return fail(format!("the lead of {name} cannot be used: {e}")),
            },
            (Err(e), ) => return fail(format!("the department {name} cannot be used: {e}")),
        };
        let settings = TeamSettings::new(&self.inner.store);
        let (attempts, limit, autonomy, away) = match (settings.attempts(), settings.parallel(), settings.autonomy(), settings.away_since()) {
            (Ok(a), Ok(p), Ok(x), Ok(w)) => (dept.attempts.unwrap_or(a), dept.parallel.unwrap_or(p), x, w.is_some()),
            _ => return fail("the team settings cannot be read".into()),
        };

        // members the lead may give steps to
        let mut members: Vec<(String, String)> = vec![];
        for m in &dept.members {
            match m {
                Member::Role(r) => match self.inner.roles.get(r) {
                    Ok(role) => members.push((r.clone(), role.description)),
                    Err(e) => return fail(format!("the member {r} cannot be used: {e}")),
                },
                Member::Dept(d) => {
                    let desc = self.inner.depts.get(d).map(|x| x.description).unwrap_or_default();
                    members.push((format!("dept:{d}"), if desc.is_empty() { "a sub-department".to_string() } else { desc }));
                }
            }
        }
        let member_names: Vec<String> = members.iter().map(|m| m.0.clone()).collect();

        // 1. the plan, corrected until it is valid or the attempts are used
        self.emit(TeamEvent::Planning { dept: name.to_string() });
        let mut prompt = plan_prompt(goal, &dept, &members, &ctx.project.as_deref().map(|p| format!("The work happens in the project folder {p}.")).unwrap_or_default());
        let mut steps = None;
        let mut last_error = String::new();
        for _ in 0..attempts.max(1) {
            match self.lead_ask(&lead, prompt.clone()).await {
                Err(e) => return fail(format!("the lead {} could not plan: {e}", lead.name)),
                Ok(reply) => match parse_plan(&reply, &member_names) {
                    Ok(s) => {
                        steps = Some(s);
                        break;
                    }
                    Err(e) => {
                        prompt = format!("{prompt}\n\nYour last plan was refused: {e}. Answer again with a corrected plan.");
                        last_error = e;
                    }
                },
            }
        }
        let Some(mut steps) = steps else { return fail(format!("the lead could not make a valid plan: {last_error}")) };
        self.emit(TeamEvent::Plan { dept: name.to_string(), steps: steps.clone() });

        // 2. approval
        if ctx.depth == 0 && autonomy != Autonomy::FireAndForget {
            if away {
                self.emit(TeamEvent::PlanDecided { dept: name.to_string(), approved: true, note: "the user is away, so the plan was approved without asking".into() });
                self.log(name, "assistant", json!({"away_decision": "approved the plan", "steps": steps.len()}));
            } else {
                match self.inner.approver.plan(name, &Plan { goal: goal.to_string(), steps: steps.clone() }).await {
                    PlanDecision::Approve => self.emit(TeamEvent::PlanDecided { dept: name.to_string(), approved: true, note: String::new() }),
                    PlanDecision::Edit(edited) => match parse_plan(&json!({"steps": edited.iter().map(|s| json!({"id": s.id, "title": s.title, "assignee": s.assignee, "brief": s.brief, "depends_on": s.depends_on, "milestone": s.milestone})).collect::<Vec<_>>()}).to_string(), &member_names) {
                        Ok(s) => {
                            steps = s;
                            self.emit(TeamEvent::PlanDecided { dept: name.to_string(), approved: true, note: "approved with the user's changes".into() });
                        }
                        Err(e) => return DeptOutcome::Rejected(format!("the edited plan is not valid: {e}")),
                    },
                    PlanDecision::Reject(why) => {
                        self.emit(TeamEvent::PlanDecided { dept: name.to_string(), approved: false, note: why.clone() });
                        return DeptOutcome::Rejected(why);
                    }
                }
            }
        }

        // 3. run the steps
        let mut state: HashMap<String, Option<StepEnd>> = steps.iter().map(|s| (s.id.clone(), None)).collect();
        let mut started: HashSet<String> = HashSet::new();
        let mut blocked: HashSet<String> = HashSet::new();
        let mut milestones_told: HashSet<String> = HashSet::new();
        let mut join: JoinSet<(String, StepEnd)> = JoinSet::new();
        let mut last_affinity: Option<String> = None;
        let mut halted = false;
        let steps_arc = Arc::new(steps.clone());
        loop {
            // steps that can never run, because something they wait for did not finish
            loop {
                let newly: Vec<String> = steps_arc
                    .iter()
                    .filter(|s| !started.contains(&s.id) && !blocked.contains(&s.id))
                    .filter(|s| s.depends_on.iter().any(|d| blocked.contains(d) || matches!(state.get(d), Some(Some(StepEnd::Failed { .. } | StepEnd::Parked(_) | StepEnd::Stopped)))))
                    .map(|s| s.id.clone())
                    .collect();
                if newly.is_empty() {
                    break;
                }
                for id in newly {
                    let s = steps_arc.iter().find(|s| s.id == id).unwrap();
                    let because = s.depends_on.iter().find(|d| blocked.contains(*d) || matches!(state.get(*d), Some(Some(StepEnd::Failed { .. } | StepEnd::Parked(_) | StepEnd::Stopped)))).cloned().unwrap_or_default();
                    self.emit(TeamEvent::StepBlocked { step: id.clone(), because });
                    blocked.insert(id);
                }
            }
            // start what is ready, up to the limit, preferring steps that use the model already loaded
            while !halted && join.len() < limit {
                let mut ready: Vec<&PlanStep> = steps_arc
                    .iter()
                    .filter(|s| !started.contains(&s.id) && !blocked.contains(&s.id) && s.depends_on.iter().all(|d| matches!(state.get(d), Some(Some(StepEnd::Done { .. })))))
                    .collect();
                if ready.is_empty() {
                    break;
                }
                let aff = |s: &PlanStep| self.affinity(&s.assignee);
                ready.sort_by_key(|s| aff(s) != last_affinity);
                let step = ready[0].clone();
                last_affinity = aff(&step);
                started.insert(step.id.clone());
                let inputs: Vec<(String, String)> = step
                    .depends_on
                    .iter()
                    .filter_map(|d| match state.get(d) {
                        Some(Some(StepEnd::Done { result, .. })) => steps_arc.iter().find(|s| &s.id == d).map(|s| (s.title.clone(), excerpt(result, 1800))),
                        _ => None,
                    })
                    .collect();
                let (me, dept2, lead2, ctx2, goal2) = (self.clone(), dept.clone(), lead.clone(), ctx.clone(), goal.to_string());
                join.spawn(async move {
                    let end = me.run_step(&goal2, &dept2, &lead2, &step, inputs, attempts, &ctx2).await;
                    (step.id.clone(), end)
                });
            }
            let Some(res) = join.join_next().await else { break };
            let Ok((id, end)) = res else { continue };
            let step = steps_arc.iter().find(|s| s.id == id).unwrap();
            match &end {
                StepEnd::Done { summary, .. } => self.emit(TeamEvent::StepDone { step: id.clone(), summary: summary.clone() }),
                StepEnd::Failed { report, .. } => self.emit(TeamEvent::StepFailed { step: id.clone(), report: report.clone() }),
                StepEnd::Parked(why) => self.emit(TeamEvent::StepParked { step: id.clone(), why: why.clone() }),
                StepEnd::Stopped => self.emit(TeamEvent::StepStopped { step: id.clone() }),
            }
            state.insert(id.clone(), Some(end));
            // a milestone is reached when all of its steps are finished one way or the other
            if let Some(m) = &step.milestone {
                let all: Vec<&PlanStep> = steps_arc.iter().filter(|s| s.milestone.as_ref() == Some(m)).collect();
                let finished = all.iter().all(|s| matches!(state.get(&s.id), Some(Some(_))) || blocked.contains(&s.id));
                if finished && milestones_told.insert(m.clone()) {
                    self.emit(TeamEvent::Milestone { dept: name.to_string(), name: m.clone() });
                    let more = steps_arc.iter().any(|s| !started.contains(&s.id) && !blocked.contains(&s.id));
                    if more && ctx.depth == 0 && autonomy == Autonomy::CheckIns && !away {
                        let sums: Vec<String> = all.iter().filter_map(|s| match state.get(&s.id) { Some(Some(StepEnd::Done { summary, .. })) => Some(format!("{}: {summary}", s.title)), _ => None }).collect();
                        if !self.inner.approver.checkin(name, m, &sums).await {
                            halted = true;
                        }
                    }
                }
            }
        }

        // 4. the outcome
        let mut summaries: Vec<StepSummary> = vec![];
        let (mut parked, mut failures, mut stopped) = (vec![], vec![], false);
        for s in steps_arc.iter() {
            let (st, worker, summary) = match state.get(&s.id) {
                Some(Some(StepEnd::Done { summary, worker, .. })) => (StepState::Done, worker.clone(), summary.clone()),
                Some(Some(StepEnd::Failed { report, worker })) => {
                    failures.push(format!("{}: {report}", s.title));
                    (StepState::Failed, worker.clone(), report.clone())
                }
                Some(Some(StepEnd::Parked(why))) => {
                    parked.push(format!("{}: {why}", s.title));
                    (StepState::Parked, s.assignee.clone(), why.clone())
                }
                Some(Some(StepEnd::Stopped)) => {
                    stopped = true;
                    (StepState::Stopped, s.assignee.clone(), "stopped".to_string())
                }
                _ => (StepState::Blocked, s.assignee.clone(), if halted { "not run: the user stopped at a check-in".to_string() } else { "not run: a step it needed did not finish".to_string() }),
            };
            let result = match state.get(&s.id) {
                Some(Some(StepEnd::Done { result, .. })) => result.clone(),
                _ => String::new(),
            };
            summaries.push(StepSummary { id: s.id.clone(), title: s.title.clone(), worker, state: st, summary, result });
        }
        if halted || stopped {
            return DeptOutcome::Stopped { steps: summaries };
        }
        if !failures.is_empty() {
            let mut report = format!("In {name}, these steps could not be finished:\n{}", failures.iter().map(|f| format!("- {f}")).collect::<Vec<_>>().join("\n"));
            let blocked_titles: Vec<&str> = summaries.iter().filter(|s| s.state == StepState::Blocked).map(|s| s.title.as_str()).collect();
            if !blocked_titles.is_empty() {
                report.push_str(&format!("\nNot run because of that: {}.", blocked_titles.join(", ")));
            }
            return DeptOutcome::Failed { report, steps: summaries };
        }
        let done: Vec<(String, String)> = summaries.iter().filter(|s| s.state == StepState::Done).map(|s| (s.title.clone(), excerpt(&s.result, 800))).collect();
        let result = match self.lead_ask(&lead, final_prompt(goal, &done, &parked)).await {
            Ok(t) if !t.trim().is_empty() => t.trim().to_string(),
            _ => done.iter().map(|(t, s)| format!("{t}: {s}")).collect::<Vec<_>>().join("\n"), // the lead could not sum up; the facts still stand
        };
        DeptOutcome::Done { result, steps: summaries, parked }
    }

    /// Steps that use the same kind of model are run together where they can, to avoid reloading.
    fn affinity(&self, assignee: &str) -> Option<String> {
        if assignee.starts_with("dept:") {
            return None;
        }
        self.inner.roles.get(assignee).ok().map(|r| match r.target {
            Target::Type(t) => format!("type:{t}"),
            Target::Model(m) => format!("model:{m}"),
        })
    }

    #[allow(clippy::too_many_arguments)]
    async fn run_step(&self, goal: &str, dept: &Department, lead: &Role, step: &PlanStep, inputs: Vec<(String, String)>, attempts: u32, ctx: &RunCtx) -> StepEnd {
        let members: Vec<String> = dept.members.iter().map(|m| match m {
            Member::Role(r) => r.clone(),
            Member::Dept(d) => format!("dept:{d}"),
        }).collect();
        let types: Vec<String> = ModelTypes::new(&self.inner.store).list().map(|v| v.into_iter().map(|t| t.name).collect()).unwrap_or_default();
        let mut worker = match self.worker_for(&step.assignee) {
            Ok(w) => w,
            Err(e) => return StepEnd::Failed { report: e, worker: step.assignee.clone() },
        };
        let (mut attempt, mut over, mut fix) = (0u32, None::<Target>, None::<Fix>);
        let mut tried: Vec<String> = vec![];
        loop {
            attempt += 1;
            let wname = worker.name();
            let outcome = match &worker {
                Worker::Role(role) => self.attempt_role(dept, role, step, &inputs, attempt, &over, fix.take(), ctx).await,
                Worker::Dept(d) => {
                    let goal = format!("{}\n\n{}", step.brief, inputs.iter().map(|(t, r)| format!("From the step \"{t}\":\n{r}")).collect::<Vec<_>>().join("\n\n"));
                    match self.run_department(d.clone(), goal, RunCtx { project: ctx.project.clone(), depth: ctx.depth + 1 }).await {
                        DeptOutcome::Done { result, .. } => Attempt::Done { summary: excerpt(&result, 240), result },
                        DeptOutcome::Failed { report, .. } => Attempt::Failed(report),
                        DeptOutcome::Rejected(w) => Attempt::Failed(format!("the sub-department's plan was not approved: {w}")),
                        DeptOutcome::Stopped { .. } => Attempt::Stopped,
                    }
                }
            };
            let reason = match outcome {
                Attempt::Stopped => return StepEnd::Stopped,
                Attempt::Parked(w) => return StepEnd::Parked(w),
                Attempt::Unavailable(m) => {
                    self.emit(TeamEvent::ModelUnavailable { role: wname.clone(), message: m.clone() });
                    m
                }
                Attempt::Failed(r) => r,
                Attempt::Done { result, summary } => {
                    // the lead checks the result against the step (a lead does not check itself)
                    let is_lead_work = matches!(&worker, Worker::Role(r) if r.name == lead.name);
                    if is_lead_work {
                        return StepEnd::Done { result, summary, worker: wname };
                    }
                    let verdict = match self.lead_ask(lead, check_prompt(goal, step, &summary, &result)).await {
                        Ok(t) => parse_check(&t),
                        Err(_) => Check::Ok, // a lead that cannot be reached does not block finished work
                    };
                    match verdict {
                        Check::Ok => {
                            self.emit(TeamEvent::StepChecked { step: step.id.clone(), verdict: "ok".into() });
                            return StepEnd::Done { result, summary, worker: wname };
                        }
                        Check::Small(what) if attempt < attempts => {
                            self.emit(TeamEvent::StepChecked { step: step.id.clone(), verdict: format!("small: {what}") });
                            tried.push(format!("attempt {attempt} ({wname}): small mistake, fixed in place: {what}"));
                            fix = Some(Fix { previous: result, instruction: what });
                            continue;
                        }
                        Check::Small(what) => {
                            self.emit(TeamEvent::StepChecked { step: step.id.clone(), verdict: format!("still not right: {what}") });
                            format!("still not right after the fixes: {what}")
                        }
                        Check::Fail(what) => {
                            self.emit(TeamEvent::StepChecked { step: step.id.clone(), verdict: format!("fail: {what}") });
                            format!("the result misses the step: {what}")
                        }
                    }
                }
            };
            // the failure path: the lead decides, within the attempt cap
            tried.push(format!("attempt {attempt} ({wname}): {reason}"));
            let give_up = |tried: &[String]| StepEnd::Failed { report: format!("{} failed after {} attempt(s):\n{}", step.title, tried.len(), tried.join("\n")), worker: wname.clone() };
            if attempt >= attempts {
                return give_up(&tried);
            }
            let remedy = match self.lead_ask(lead, failure_prompt(step, &wname, &reason, attempt, attempts, &members)).await {
                Ok(t) => parse_remedy(&t, &members, &types),
                Err(_) => Remedy::Retry,
            };
            self.emit(TeamEvent::StepRetry { step: step.id.clone(), remedy: format!("{remedy:?}"), reason: reason.clone() });
            match remedy {
                Remedy::GiveUp => return give_up(&tried),
                Remedy::Retry => {}
                Remedy::ChangeModel(t) => over = Some(t),
                Remedy::DoItYourself => worker = Worker::Role(Box::new(lead.clone())),
                Remedy::Subagent => {
                    // a regular helper with the failed worker's tools and model, handed only the brief
                    let base = match &worker {
                        Worker::Role(r) => (**r).clone(),
                        Worker::Dept(_) => lead.clone(),
                    };
                    worker = Worker::Role(Box::new(Role { name: "subagent".into(), description: "A helper handed one step.".into(), instructions: "You are a helper handed one step. Do it carefully, then report plainly what you did.".into(), ..base }));
                }
                Remedy::HandOff(m) => match self.worker_for(&m) {
                    Ok(w) => worker = w,
                    Err(e) => tried.push(format!("could not hand off to {m}: {e}")),
                },
            }
        }
    }

    fn worker_for(&self, assignee: &str) -> Result<Worker, String> {
        match assignee.strip_prefix("dept:") {
            Some(d) => Ok(Worker::Dept(d.to_string())),
            None => self.inner.roles.get(assignee).map(|r| Worker::Role(Box::new(r))).map_err(|e| format!("the role {assignee} cannot be used: {e}")),
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn attempt_role(&self, dept: &Department, role: &Role, step: &PlanStep, inputs: &[(String, String)], attempt: u32, over: &Option<Target>, fix: Option<Fix>, ctx: &RunCtx) -> Attempt {
        let res = match self.resolve(role, over).await {
            Ok(r) => r,
            Err(e @ crate::Error::Unavailable(_)) => return Attempt::Unavailable(e.to_string()),
            Err(e) => return Attempt::Failed(e.to_string()),
        };
        if let Some(note) = res.note.clone() {
            self.emit(TeamEvent::ModelNote { note });
        }
        let (wid, ctl) = self.inner.registry.add(WorkerInfo { id: String::new(), dept: dept.name.clone(), role: role.name.clone(), model: res.model.clone(), step: step.title.clone(), status: Status::Running, attempt });
        self.emit(TeamEvent::StepStarted { dept: dept.name.clone(), step: step.id.clone(), title: step.title.clone(), worker: role.name.clone(), model: res.model.clone(), attempt });
        let job = StepJob { id: step.id.clone(), dept: dept.name.clone(), role: role.clone(), model: res.model.clone(), brief: step.brief.clone(), inputs: inputs.to_vec(), fix, attempt, project: ctx.project.clone() };
        let result = tokio::select! {
            r = self.inner.backend.run_step(job, ctl.clone()) => r,
            _ = ctl.stopped() => StepResult::Stopped,
        };
        let (status, out) = match result {
            StepResult::Done { result, summary } if !result.trim().is_empty() => (Status::Done, Attempt::Done { result, summary }),
            StepResult::Done { .. } => (Status::Failed, Attempt::Failed("the worker finished without a result".into())),
            StepResult::Failed(r) => (Status::Failed, Attempt::Failed(r)),
            StepResult::Stopped => (Status::Stopped, Attempt::Stopped),
            StepResult::Parked(w) => (Status::Parked, Attempt::Parked(w)),
        };
        self.inner.registry.set(&wid, status, None, None);
        self.log(&dept.name, &role.name, json!({"step": step.id, "attempt": attempt, "model": res.model, "outcome": status.as_str()}));
        out
    }
}
