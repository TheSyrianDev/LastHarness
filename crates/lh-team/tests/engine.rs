//! The engine against a scripted backend: no model, exact control of what every worker and lead says.
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use lh_learn::Learn;
use lh_store::Store;
use lh_team::*;
use lh_tools::BoxFut;

#[derive(Default)]
struct Script {
    asks: Mutex<HashMap<&'static str, VecDeque<String>>>,
    steps: Mutex<HashMap<String, VecDeque<StepResult>>>,
    jobs: Mutex<Vec<StepJob>>,
    prompts: Mutex<Vec<(String, String)>>,
    running: AtomicUsize,
    max_running: AtomicUsize,
    delay_ms: u64,
    available: Vec<String>,
    /// roles whose worker waits at checkpoints until stopped (for pause and stop tests)
    hang: Vec<String>,
}
impl Script {
    fn say(&self, tag: &'static str, reply: &str) {
        self.asks.lock().unwrap().entry(tag).or_default().push_back(reply.to_string());
    }
    fn step(&self, role: &str, r: StepResult) {
        self.steps.lock().unwrap().entry(role.to_string()).or_default().push_back(r);
    }
    fn order(&self) -> Vec<String> {
        self.jobs.lock().unwrap().iter().map(|j| format!("{}:{}", j.id, j.role.name)).collect()
    }
}
fn done(r: &str) -> StepResult {
    StepResult::Done { result: r.into(), summary: format!("did {r}") }
}
struct Fake(Arc<Script>);
impl Backend for Fake {
    async fn run_step(&self, job: StepJob, ctl: StepControl) -> StepResult {
        self.0.jobs.lock().unwrap().push(job.clone());
        let now = self.0.running.fetch_add(1, Ordering::SeqCst) + 1;
        self.0.max_running.fetch_max(now, Ordering::SeqCst);
        if self.0.hang.contains(&job.role.name) {
            while ctl.checkpoint().await {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            self.0.running.fetch_sub(1, Ordering::SeqCst);
            return StepResult::Stopped;
        }
        if self.0.delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(self.0.delay_ms)).await;
        }
        self.0.running.fetch_sub(1, Ordering::SeqCst);
        self.0.steps.lock().unwrap().get_mut(&job.role.name).and_then(|q| q.pop_front()).unwrap_or_else(|| done(&format!("{}-result", job.id)))
    }
    async fn ask(&self, role: &Role, model: &str, prompt: String) -> std::result::Result<String, String> {
        let tag = ["PLAN", "CHECK", "FAILURE", "FINAL"].into_iter().find(|t| prompt.starts_with(&format!("[{t}]"))).unwrap_or("OTHER");
        self.0.prompts.lock().unwrap().push((format!("{tag}:{}:{model}", role.name), prompt));
        let next = self.0.asks.lock().unwrap().get_mut(tag).and_then(|q| q.pop_front());
        Ok(next.unwrap_or_else(|| match tag {
            "CHECK" => "OK".into(),
            "FINAL" => "All done.".into(),
            "FAILURE" => "RETRY".into(),
            _ => String::new(),
        }))
    }
    async fn models(&self) -> Vec<String> {
        self.0.available.clone()
    }
}

struct Judge {
    plans: Mutex<VecDeque<PlanDecision>>,
    asked_plan: AtomicUsize,
    checkins: Mutex<Vec<String>>,
    go_on: bool,
}
impl Approver for Judge {
    fn plan<'a>(&'a self, _: &'a str, _: &'a Plan) -> BoxFut<'a, PlanDecision> {
        self.asked_plan.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move { self.plans.lock().unwrap().pop_front().unwrap_or(PlanDecision::Approve) })
    }
    fn checkin<'a>(&'a self, _: &'a str, m: &'a str, _: &'a [String]) -> BoxFut<'a, bool> {
        self.checkins.lock().unwrap().push(m.to_string());
        Box::pin(async move { self.go_on })
    }
}
fn judge(go_on: bool) -> Arc<Judge> {
    Arc::new(Judge { plans: Default::default(), asked_plan: AtomicUsize::new(0), checkins: Default::default(), go_on })
}

struct Fx {
    _d: tempfile::TempDir,
    script: Arc<Script>,
    engine: Engine<Fake>,
    events: Arc<Mutex<Vec<TeamEvent>>>,
    store: Arc<Store>,
    roles: Arc<RoleBook>,
    depts: Arc<DeptBook>,
}
fn plan_json(steps: &[(&str, &str, &str, &[&str])]) -> String {
    let v: Vec<serde_json::Value> = steps.iter().map(|(id, who, brief, deps)| serde_json::json!({"id": id, "title": format!("Step {id}"), "assignee": who, "brief": brief, "depends_on": deps})).collect();
    serde_json::json!({"steps": v}).to_string()
}

fn build(script: Script, judge: Arc<Judge>, setup: impl FnOnce(&TeamSettings)) -> Fx {
    let d = tempfile::tempdir().unwrap();
    let store = Arc::new(Store::open_memory().unwrap());
    let learn = Arc::new(Learn::new(store.clone()));
    let roles = Arc::new(RoleBook::open(d.path().join("roles"), learn.clone()).unwrap());
    roles.install_starters().unwrap();
    let depts = Arc::new(DeptBook::open(d.path().join("departments"), learn).unwrap());
    let types = ModelTypes::new(&store);
    types.set_model("frontier", Some("front-model")).unwrap();
    types.set_model("mid-tier", Some("mid-model")).unwrap();
    types.set_model("fastest", Some("fast-model")).unwrap();
    let m = |r: &str| Member::Role(r.into());
    let team = Department { name: "web".into(), description: "Builds websites.".into(), lead: "planner".into(), members: vec![m("researcher"), m("writer"), m("coder"), Member::Dept("design".into())], parallel: None, attempts: None, instructions: "Work together.".into() };
    let design = Department { name: "design".into(), description: "Designs.".into(), lead: "designer".into(), members: vec![m("designer"), m("writer")], parallel: None, attempts: None, instructions: "Design it.".into() };
    depts.save(&design, &roles, "t").unwrap();
    depts.save(&team, &roles, "t").unwrap();
    setup(&TeamSettings::new(&store));
    let script = Arc::new(script);
    let events: Arc<Mutex<Vec<TeamEvent>>> = Default::default();
    let ev = events.clone();
    let engine = Engine::new(Fake(script.clone()), store.clone(), roles.clone(), depts.clone(), Arc::new(Registry::default()), Arc::new(move |e| ev.lock().unwrap().push(e)), judge.clone());
    Fx { _d: d, script, engine, events, store, roles, depts }
}
fn quick(setup: impl FnOnce(&TeamSettings)) -> Fx {
    build(Script::default(), judge(true), |s| {
        s.set_autonomy(Autonomy::FireAndForget).unwrap();
        setup(s);
    })
}
async fn run(f: &Fx, goal: &str) -> DeptOutcome {
    f.engine.run_department("web".into(), goal.into(), RunCtx::default()).await
}
fn kinds(f: &Fx) -> Vec<String> {
    f.events.lock().unwrap().iter().map(|e| e.to_json()["type"].as_str().unwrap().to_string()).collect()
}

#[tokio::test]
async fn a_goal_is_planned_run_in_order_checked_and_summed_up() {
    let f = quick(|_| {});
    f.script.say("PLAN", &plan_json(&[("1", "researcher", "find facts", &[]), ("2", "writer", "write it", &["1"]), ("3", "coder", "build it", &["2"])]));
    f.script.step("researcher", done("the facts"));
    f.script.say("FINAL", "The site is built.");
    let out = run(&f, "make a site").await;
    let DeptOutcome::Done { result, steps, parked } = out else { panic!("{out:?}") };
    assert_eq!((result.as_str(), parked.len(), steps.len()), ("The site is built.", 0, 3));
    assert!(steps.iter().all(|s| s.state == StepState::Done));
    assert_eq!(f.script.order(), ["1:researcher", "2:writer", "3:coder"]);
    // the writer's brief comes with what the researcher handed back
    let jobs = f.script.jobs.lock().unwrap().clone();
    assert_eq!(jobs[1].inputs, [("Step 1".to_string(), "the facts".to_string())]);
    assert_eq!(jobs[0].model, "mid-model");
    assert!(jobs[0].inputs.is_empty() && jobs[2].dept == "web");
    // the lead (planner, a frontier role) planned and summed up on the frontier model
    let prompts = f.script.prompts.lock().unwrap().clone();
    assert_eq!(prompts[0].0, "PLAN:planner:front-model");
    assert!(prompts[0].1.contains("make a site") && prompts[0].1.contains("- researcher:") && prompts[0].1.contains("- dept:design: Designs."));
    assert!(prompts.iter().any(|(k, p)| k.starts_with("FINAL:") && p.contains("the facts")));
    let k = kinds(&f);
    assert_eq!(k.first().map(String::as_str), Some("planning"));
    assert_eq!(k.last().map(String::as_str), Some("finished"));
    assert_eq!(k.iter().filter(|x| *x == "step_done").count(), 3);
    assert_eq!(f.store.recent(20, Some("team")).unwrap().len(), 3, "each attempt is logged, without any text");
}

#[tokio::test]
async fn independent_steps_run_together_up_to_the_limit_and_dependents_wait() {
    for (limit, want_max) in [(1usize, 1usize), (2, 2), (4, 3)] {
        let s = Script { delay_ms: 40, ..Default::default() };
        let f = build(s, judge(true), |t| {
            t.set_autonomy(Autonomy::FireAndForget).unwrap();
            t.set_parallel(limit).unwrap();
        });
        f.script.say("PLAN", &plan_json(&[("a", "researcher", "x", &[]), ("b", "writer", "y", &[]), ("c", "coder", "z", &[]), ("d", "writer", "after all", &["a", "b", "c"])]));
        let out = run(&f, "go").await;
        assert!(matches!(out, DeptOutcome::Done { .. }), "{out:?}");
        assert_eq!(f.script.max_running.load(Ordering::SeqCst), want_max, "limit {limit}");
        assert_eq!(f.script.order().last().unwrap(), "d:writer", "the dependent step starts last");
    }
}

#[tokio::test]
async fn steps_on_the_same_kind_of_model_run_together_to_avoid_reloading() {
    let f = quick(|_| {});
    // coder and researcher use mid-tier; the planner-led "frontier" role does not take part in this plan
    f.roles.save(&{ let mut r = f.roles.get("writer").unwrap(); r.name = "fast-writer".into(); r.target = Target::Type("fastest".into()); r }, "t").unwrap();
    let mut web = f.depts.get("web").unwrap();
    web.members.push(Member::Role("fast-writer".into()));
    f.depts.save(&web, &f.roles, "t").unwrap();
    f.script.say("PLAN", &plan_json(&[("1", "researcher", "a", &[]), ("2", "fast-writer", "b", &[]), ("3", "coder", "c", &[]), ("4", "fast-writer", "d", &[])]));
    run(&f, "go").await;
    let order = f.script.order();
    assert_eq!(order, ["1:researcher", "3:coder", "2:fast-writer", "4:fast-writer"], "mid-tier steps first, then the fastest ones: one model switch instead of three");
}

#[tokio::test]
async fn a_plan_with_mistakes_is_sent_back_to_the_lead_and_a_hopeless_one_fails() {
    let f = quick(|_| {});
    f.script.say("PLAN", &plan_json(&[("1", "ghost", "x", &[])]));
    f.script.say("PLAN", &plan_json(&[("1", "writer", "x", &[])]));
    assert!(matches!(run(&f, "go").await, DeptOutcome::Done { .. }));
    let prompts = f.script.prompts.lock().unwrap().clone();
    assert!(prompts[1].1.contains("Your last plan was refused: step 1 is given to \"ghost\", who is not a member"), "{}", prompts[1].1);

    let g = quick(|_| {});
    for _ in 0..3 {
        g.script.say("PLAN", "no plan, sorry");
    }
    let DeptOutcome::Failed { report, .. } = run(&g, "go").await else { panic!() };
    assert!(report.contains("could not make a valid plan") && report.contains("no JSON plan"), "{report}");
    assert!(g.script.jobs.lock().unwrap().is_empty(), "nothing ran");
}

#[tokio::test]
async fn the_user_approves_edits_or_rejects_the_plan_unless_autonomy_or_away_says_otherwise() {
    let reject = judge(true);
    reject.plans.lock().unwrap().push_back(PlanDecision::Reject("not now".into()));
    let f = build(Script::default(), reject.clone(), |_| {});
    f.script.say("PLAN", &plan_json(&[("1", "writer", "x", &[])]));
    assert_eq!(run(&f, "go").await, DeptOutcome::Rejected("not now".into()));
    assert!(f.script.jobs.lock().unwrap().is_empty(), "nothing runs before the plan is approved");
    assert_eq!(kinds(&f).iter().filter(|k| *k == "plan_decided").count(), 1);

    let edit = judge(true);
    let edited = vec![PlanStep { id: "9".into(), title: "Mine".into(), assignee: "coder".into(), brief: "my own step".into(), depends_on: vec![], milestone: None }];
    edit.plans.lock().unwrap().push_back(PlanDecision::Edit(edited));
    let f = build(Script::default(), edit, |_| {});
    f.script.say("PLAN", &plan_json(&[("1", "writer", "x", &[])]));
    run(&f, "go").await;
    assert_eq!(f.script.order(), ["9:coder"], "the user's version ran");

    // fire-and-forget never asks
    let j = judge(true);
    let f = build(Script::default(), j.clone(), |s| s.set_autonomy(Autonomy::FireAndForget).unwrap());
    f.script.say("PLAN", &plan_json(&[("1", "writer", "x", &[])]));
    run(&f, "go").await;
    assert_eq!(j.asked_plan.load(Ordering::SeqCst), 0);

    // away: nobody is asked, the decision is logged and said in the event
    let j = judge(false);
    let f = build(Script::default(), j.clone(), |s| s.set_away(true).unwrap());
    f.script.say("PLAN", &plan_json(&[("1", "writer", "x", &[])]));
    assert!(matches!(run(&f, "go").await, DeptOutcome::Done { .. }));
    assert_eq!(j.asked_plan.load(Ordering::SeqCst), 0);
    assert!(f.events.lock().unwrap().iter().any(|e| matches!(e, TeamEvent::PlanDecided { approved: true, note, .. } if note.contains("user is away"))));
    assert!(f.store.recent(20, Some("team")).unwrap().iter().any(|e| e.detail["away_decision"] == "approved the plan"));
}

#[tokio::test]
async fn small_mistakes_are_fixed_in_place_by_a_fresh_worker_and_larger_misses_are_failures() {
    let f = quick(|_| {});
    f.script.say("PLAN", &plan_json(&[("1", "writer", "write the intro", &[])]));
    f.script.step("writer", done("intro without title"));
    f.script.step("writer", done("intro with title"));
    f.script.say("CHECK", "SMALL: add the missing title");
    f.script.say("CHECK", "OK");
    let DeptOutcome::Done { steps, .. } = run(&f, "go").await else { panic!() };
    assert_eq!(steps[0].summary, "did intro with title");
    let jobs = f.script.jobs.lock().unwrap().clone();
    assert_eq!(jobs.len(), 2);
    let fix = jobs[1].fix.as_ref().expect("the second run is a fix");
    assert_eq!((fix.previous.as_str(), fix.instruction.as_str(), jobs[1].attempt), ("intro without title", "add the missing title", 2));
    assert!(jobs[0].fix.is_none());

    // a larger miss goes down the failure path: the lead is asked what to do
    let g = quick(|_| {});
    g.script.say("PLAN", &plan_json(&[("1", "writer", "write the intro", &[])]));
    g.script.step("writer", done("answers a different question"));
    g.script.step("writer", done("the right intro"));
    g.script.say("CHECK", "FAIL: it answers a different question");
    g.script.say("FAILURE", "RETRY");
    let DeptOutcome::Done { steps, .. } = run(&g, "go").await else { panic!() };
    assert_eq!(steps[0].summary, "did the right intro");
    assert!(g.script.prompts.lock().unwrap().iter().any(|(k, p)| k.starts_with("FAILURE:") && p.contains("the result misses the step: it answers a different question")));
}

#[tokio::test]
async fn every_remedy_the_lead_can_choose_does_what_it_says() {
    // retry, change model, do it yourself, subagent, hand off: each on a failing first attempt
    type Assertion = Box<dyn Fn(&[StepJob])>;
    let cases: [(&str, Assertion); 5] = [
        ("RETRY", Box::new(|j| assert_eq!((j[1].role.name.as_str(), j[1].model.as_str(), j[1].attempt), ("coder", "mid-model", 2)))),
        ("CHANGE_MODEL fastest", Box::new(|j| assert_eq!((j[1].role.name.as_str(), j[1].model.as_str()), ("coder", "fast-model")))),
        ("DO_IT_YOURSELF", Box::new(|j| assert_eq!((j[1].role.name.as_str(), j[1].model.as_str()), ("planner", "front-model")))),
        ("SUBAGENT", Box::new(|j| {
            assert_eq!(j[1].role.name, "subagent");
            assert_eq!(j[1].role.tools, j[0].role.tools, "same tools as the worker that failed");
            assert!(j[1].role.instructions.len() < 120, "a regular subagent gets a very short role line");
        })),
        ("HAND_OFF writer", Box::new(|j| assert_eq!(j[1].role.name, "writer"))),
    ];
    for (answer, check) in cases {
        let f = quick(|_| {});
        f.script.say("PLAN", &plan_json(&[("1", "coder", "build it", &[])]));
        f.script.step("coder", StepResult::Failed("it crashed".into()));
        f.script.say("FAILURE", answer);
        let out = run(&f, "go").await;
        assert!(matches!(out, DeptOutcome::Done { .. }), "{answer}: {out:?}");
        check(&f.script.jobs.lock().unwrap());
        assert!(f.events.lock().unwrap().iter().any(|e| matches!(e, TeamEvent::StepRetry { reason, .. } if reason == "it crashed")));
    }
}

#[tokio::test]
async fn the_attempt_cap_stops_a_stuck_step_and_the_failure_goes_up_with_what_was_tried() {
    let f = quick(|s| s.set_attempts(3).unwrap());
    f.script.say("PLAN", &plan_json(&[("1", "coder", "build it", &[]), ("2", "writer", "after", &["1"]), ("3", "researcher", "independent", &[])]));
    for _ in 0..5 {
        f.script.step("coder", StepResult::Failed("the build fails".into()));
    }
    let DeptOutcome::Failed { report, steps } = run(&f, "go").await else { panic!() };
    assert_eq!(f.script.jobs.lock().unwrap().iter().filter(|j| j.id == "1").count(), 3, "exactly the attempt cap");
    assert!(report.contains("could not be finished") && report.contains("Step 1") && report.contains("attempt 3") && report.contains("the build fails") && report.contains("Not run because of that: Step 2"), "{report}");
    let state = |id: &str| steps.iter().find(|s| s.id == id).unwrap().state;
    assert_eq!((state("1"), state("2"), state("3")), (StepState::Failed, StepState::Blocked, StepState::Done), "the independent step still finished");
    assert!(kinds(&f).contains(&"step_blocked".to_string()));
    // a department can set its own cap, and the lead can give up early
    let g = quick(|_| {});
    g.script.say("PLAN", &plan_json(&[("1", "coder", "x", &[])]));
    for _ in 0..5 {
        g.script.step("coder", StepResult::Failed("no".into()));
    }
    g.script.say("FAILURE", "GIVE_UP");
    assert!(matches!(run(&g, "go").await, DeptOutcome::Failed { .. }));
    assert_eq!(g.script.jobs.lock().unwrap().len(), 1, "given up after the first failure");
    let mut web = g.depts.get("web").unwrap();
    web.attempts = Some(5);
    g.depts.save(&web, &g.roles, "t").unwrap();
    g.script.say("PLAN", &plan_json(&[("1", "coder", "x", &[])]));
    for _ in 0..9 {
        g.script.step("coder", StepResult::Failed("no".into()));
    }
    let before = g.script.jobs.lock().unwrap().len();
    run(&g, "go").await;
    assert_eq!(g.script.jobs.lock().unwrap().len() - before, 5);
}

#[tokio::test]
async fn a_sub_department_runs_its_own_plan_and_its_failure_comes_back_as_a_failed_step() {
    let f = quick(|_| {});
    f.script.say("PLAN", &plan_json(&[("1", "dept:design", "design the page", &[])])); // the web lead's plan
    f.script.say("PLAN", &plan_json(&[("a", "designer", "layout", &[]), ("b", "writer", "copy", &["a"])])); // the design lead's plan
    f.script.say("FINAL", "The page is designed."); // design lead sums up
    f.script.say("FINAL", "Web is done."); // web lead sums up
    let DeptOutcome::Done { result, steps, .. } = run(&f, "make the page").await else { panic!() };
    assert_eq!(result, "Web is done.");
    assert_eq!(steps[0].summary, "The page is designed.", "the sub-department's answer is the step's result");
    let prompts = f.script.prompts.lock().unwrap().clone();
    assert!(prompts.iter().any(|(k, _)| k == "PLAN:designer:mid-model" || k.starts_with("PLAN:designer:")), "the design lead planned: {:?}", prompts.iter().map(|p| &p.0).collect::<Vec<_>>());
    assert_eq!(f.script.order(), ["a:designer", "b:writer"]);
    assert_eq!(kinds(&f).iter().filter(|k| *k == "finished").count(), 2);

    // a sub-department that fails is a failed step for the parent, which handles it like any other
    let g = quick(|s| s.set_attempts(2).unwrap());
    g.script.say("PLAN", &plan_json(&[("1", "dept:design", "design the page", &[])]));
    g.script.say("PLAN", &plan_json(&[("a", "designer", "layout", &[])]));
    g.script.say("PLAN", &plan_json(&[("a", "designer", "layout", &[])])); // the design lead plans again when the parent retries
    for _ in 0..6 {
        g.script.step("designer", StepResult::Failed("no inspiration".into()));
    }
    let DeptOutcome::Failed { report, .. } = run(&g, "make the page").await else { panic!() };
    assert!(report.contains("Step 1") && report.contains("dept:design"), "{report}");
}

#[tokio::test]
async fn a_stopped_worker_is_not_retried_and_a_paused_one_waits() {
    let s = Script { hang: vec!["coder".into()], ..Default::default() };
    let f = build(s, judge(true), |t| t.set_autonomy(Autonomy::FireAndForget).unwrap());
    f.script.say("PLAN", &plan_json(&[("1", "coder", "x", &[]), ("2", "writer", "y", &[])]));
    let (engine, handle) = (f.engine.clone(), tokio::spawn({
        let e = f.engine.clone();
        async move { e.run_department("web".into(), "go".into(), RunCtx::default()).await }
    }));
    let _ = engine;
    // wait for the coder to be running, then pause, check it is held, resume, then stop it
    let id = loop {
        if let Some(w) = f.engine.registry().list().into_iter().find(|w| w.role == "coder" && w.status == Status::Running) {
            break w.id;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    };
    assert!(f.engine.registry().pause(&id));
    assert_eq!(f.engine.registry().list().iter().find(|w| w.id == id).unwrap().status, Status::Paused);
    assert!(f.engine.registry().resume(&id));
    assert!(f.engine.registry().stop(&id));
    let out = tokio::time::timeout(Duration::from_secs(5), handle).await.expect("the run ends after the stop").unwrap();
    assert!(matches!(out, DeptOutcome::Stopped { .. }), "{out:?}");
    assert_eq!(f.script.jobs.lock().unwrap().iter().filter(|j| j.id == "1").count(), 1, "a stop is final: no retry");
    assert!(f.script.jobs.lock().unwrap().iter().any(|j| j.id == "2"), "the other step ran");
    assert!(!f.engine.registry().stop("w999") && !f.engine.registry().pause("nonsense"));
}

#[tokio::test]
async fn a_model_that_is_not_there_stops_with_a_message_or_falls_back_down_the_list() {
    // stop (the default): the step fails with the message, the lead may change the model
    let s = Script { available: vec!["front-model".into(), "fast-model".into()], ..Default::default() }; // mid-model is missing
    let f = build(s, judge(true), |t| t.set_autonomy(Autonomy::FireAndForget).unwrap());
    f.script.say("PLAN", &plan_json(&[("1", "writer", "x", &[])]));
    f.script.say("FAILURE", "CHANGE_MODEL fastest");
    let out = run(&f, "go").await;
    assert!(matches!(out, DeptOutcome::Done { .. }), "{out:?}");
    assert!(f.events.lock().unwrap().iter().any(|e| matches!(e, TeamEvent::ModelUnavailable { message, .. } if message.contains("mid-model") && message.contains("Pick another model now"))));
    assert_eq!(f.script.jobs.lock().unwrap()[0].model, "fast-model");

    // fall back: the next model in the type's list, said out loud
    let s = Script { available: vec!["front-model".into(), "backup-model".into()], ..Default::default() };
    let g = build(s, judge(true), |t| {
        t.set_autonomy(Autonomy::FireAndForget).unwrap();
        t.set_unavailable(UnavailablePolicy::Fallback).unwrap();
    });
    ModelTypes::new(&g.store).set_fallbacks("mid-tier", &["backup-model".to_string()]).unwrap();
    g.script.say("PLAN", &plan_json(&[("1", "writer", "x", &[])]));
    assert!(matches!(run(&g, "go").await, DeptOutcome::Done { .. }));
    assert_eq!(g.script.jobs.lock().unwrap()[0].model, "backup-model");
    assert!(g.events.lock().unwrap().iter().any(|e| matches!(e, TeamEvent::ModelNote { note } if note.contains("mid-model is not available") && note.contains("backup-model"))));

    // a type without a model says so
    let h = build(Script::default(), judge(true), |t| t.set_autonomy(Autonomy::FireAndForget).unwrap());
    ModelTypes::new(&h.store).set_model("mid-tier", None).unwrap();
    h.script.say("PLAN", &plan_json(&[("1", "writer", "x", &[])]));
    let DeptOutcome::Failed { report, .. } = run(&h, "go").await else { panic!() };
    assert!(report.contains("has no model yet"), "{report}");
}

#[tokio::test]
async fn check_ins_pause_at_each_milestone_and_the_user_can_stop_there() {
    let plan = |f: &Fx| {
        let steps = serde_json::json!({"steps":[
            {"id":"1","title":"A","assignee":"researcher","brief":"a","milestone":"research"},
            {"id":"2","title":"B","assignee":"writer","brief":"b","depends_on":["1"],"milestone":"draft"},
            {"id":"3","title":"C","assignee":"coder","brief":"c","depends_on":["2"],"milestone":"build"}]});
        f.script.say("PLAN", &steps.to_string());
    };
    let j = judge(true);
    let f = build(Script::default(), j.clone(), |s| s.set_autonomy(Autonomy::CheckIns).unwrap());
    plan(&f);
    assert!(matches!(run(&f, "go").await, DeptOutcome::Done { .. }));
    assert_eq!(*j.checkins.lock().unwrap(), ["research", "draft"], "no check-in after the last milestone: there is nothing left to hold");

    let j = judge(false);
    let f = build(Script::default(), j.clone(), |s| s.set_autonomy(Autonomy::CheckIns).unwrap());
    plan(&f);
    let out = run(&f, "go").await;
    let DeptOutcome::Stopped { steps } = out else { panic!("{out:?}") };
    assert_eq!(steps.iter().map(|s| s.state).collect::<Vec<_>>(), [StepState::Done, StepState::Blocked, StepState::Blocked]);
    assert_eq!(f.script.order(), ["1:researcher"], "nothing after the milestone the user stopped at");
    assert_eq!(*j.checkins.lock().unwrap(), ["research"]);
}

#[tokio::test]
async fn a_step_set_aside_for_the_user_does_not_stop_the_rest() {
    let f = quick(|_| {});
    f.script.say("PLAN", &plan_json(&[("1", "coder", "install a driver", &[]), ("2", "writer", "write docs", &[]), ("3", "researcher", "after the driver", &["1"])]));
    f.script.step("coder", StepResult::Parked("needed to install software and nobody was there to say yes".into()));
    let DeptOutcome::Done { parked, steps, .. } = run(&f, "go").await else { panic!() };
    assert_eq!(parked.len(), 1);
    assert!(parked[0].contains("Step 1") && parked[0].contains("nobody was there"));
    let state = |id: &str| steps.iter().find(|s| s.id == id).unwrap().state;
    assert_eq!((state("1"), state("2"), state("3")), (StepState::Parked, StepState::Done, StepState::Blocked));
    assert!(f.script.prompts.lock().unwrap().iter().any(|(k, p)| k.starts_with("FINAL:") && p.contains("set aside for the user")));
    assert_eq!(f.script.jobs.lock().unwrap().iter().filter(|j| j.id == "1").count(), 1, "a parked step is not retried");
}
