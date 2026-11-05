//! The real backend against a scripted model: what a worker is actually sent.
use std::sync::{Arc, Mutex};

use lh_provider::{ChatRequest, Event, FinishReason, Response};
use lh_team::*;
use lh_tools::*;

struct Spy {
    seen: Mutex<Vec<ChatRequest>>,
    replies: Mutex<Vec<Response>>,
}
impl Model for Spy {
    async fn complete(&self, req: &ChatRequest, _on: &mut (dyn FnMut(Event) + Send)) -> std::result::Result<Response, lh_provider::Error> {
        self.seen.lock().unwrap().push(req.clone());
        Ok(self.replies.lock().unwrap().pop().unwrap_or(Response { text: "ok".into(), finish: Some(FinishReason::Stop), ..Default::default() }))
    }
}
struct No;
impl Asker for No {
    async fn ask(&self, _: &AskRequest<'_>) -> Answer {
        Answer::No
    }
}

fn role(tools: &[&str], memory: MemorySlice) -> Role {
    Role { name: "coder".into(), description: "Writes code.".into(), target: Target::Type("mid-tier".into()), tools: tools.iter().map(|s| s.to_string()).collect(), memory, sampling: RoleSampling { temperature: Some(0.2), ..Default::default() }, max_steps: 4, thinking: Some(false), instructions: "You write code. Keep it simple.".into() }
}
fn backend(spy: Arc<Spy>, home: &std::path::Path) -> ModelBackend<Spy, NoMeter, No> {
    ModelBackend {
        model: spy,
        meter: Arc::new(NoMeter),
        asker: Arc::new(No),
        env: WorkerEnv { home: home.to_path_buf(), mode: Mode::Tiered, sudo_enabled: false, allow_rules: vec![], live: None },
        toolbox: Arc::new(|r, _| tools_for_role(&default_tools(), r)),
        provider_name: Some("llama-swap".into()),
        max_tokens: 2000,
    }
}
fn job(r: Role, project: Option<String>) -> StepJob {
    StepJob { id: "1".into(), dept: "web".into(), role: r, model: "m-7b".into(), brief: "Write hello.py.".into(), inputs: vec![("Plan".into(), "print hello".into())], fix: None, attempt: 1, project }
}

#[tokio::test]
async fn a_worker_gets_the_base_prompt_a_short_role_line_and_only_the_brief() {
    let d = tempfile::tempdir().unwrap();
    let spy = Arc::new(Spy { seen: Default::default(), replies: Default::default() });
    let b = backend(spy.clone(), d.path());
    let ctl = Registry::default().add(WorkerInfo { id: String::new(), dept: "web".into(), role: "coder".into(), model: "m".into(), step: "s".into(), status: Status::Running, attempt: 1 }).1;
    let out = b.run_step(job(role(&["read_file", "list_dir"], MemorySlice::None), Some(d.path().display().to_string())), ctl).await;
    assert_eq!(out, StepResult::Done { result: "ok".into(), summary: "ok".into() });
    let req = spy.seen.lock().unwrap()[0].clone();
    assert_eq!(req.messages.len(), 2, "system and the brief, nothing else");
    let sys = req.messages[0].content.clone().unwrap();
    assert!(sys.starts_with("You are m-7b"), "{sys}");
    assert!(sys.contains("served by llama-swap, an endpoint, not the model's maker") && sys.ends_with("You write code. Keep it simple."));
    assert!(sys.contains("stay inside it"), "a project worker is told its folder");
    assert!(lh_tools::estimate_text_tokens(&sys) <= 300, "{} tokens", lh_tools::estimate_text_tokens(&sys));
    let user = req.messages[1].content.clone().unwrap();
    assert_eq!(user, "Write hello.py.\n\nFrom the step \"Plan\":\nprint hello");
    let names: Vec<String> = req.tools.iter().map(|t| t.name.clone()).collect();
    assert_eq!(names, ["read"], "legacy read aliases resolve to one canonical tool");
    assert_eq!((req.sampling.temperature, req.max_tokens), (Some(0.2), Some(2000)));
    assert_eq!(req.sampling.extra["chat_template_kwargs"], serde_json::json!({"enable_thinking": false}));
}

#[tokio::test]
async fn empty_answers_and_a_stop_are_reported_for_what_they_are() {
    let d = tempfile::tempdir().unwrap();
    let spy = Arc::new(Spy { seen: Default::default(), replies: Mutex::new(vec![Response { finish: Some(FinishReason::Length), ..Default::default() }]) });
    let b = backend(spy.clone(), d.path());
    let reg = Registry::default();
    let new = || reg.add(WorkerInfo { id: String::new(), dept: "web".into(), role: "coder".into(), model: "m".into(), step: "s".into(), status: Status::Running, attempt: 1 });
    let StepResult::Failed(why) = b.run_step(job(role(&[], MemorySlice::None), None), new().1).await else { panic!() };
    assert!(why.contains("ran out of tokens"), "{why}");
    let (id, ctl) = new();
    reg.stop(&id);
    assert_eq!(b.run_step(job(role(&[], MemorySlice::None), None), ctl).await, StepResult::Stopped);
    assert_eq!(spy.seen.lock().unwrap().len(), 1, "a stopped worker makes no model call");
    // the lead's question: base prompt plus the lead's line, the prompt as the one message
    assert_eq!(b.ask(&role(&[], MemorySlice::None), "m-7b", "[PLAN] x".into()).await, std::result::Result::<String,String>::Ok("ok".to_string()));
    let seen = spy.seen.lock().unwrap();
    assert_eq!((seen[1].messages.len(), seen[1].tools.len()), (2, 0));
}
