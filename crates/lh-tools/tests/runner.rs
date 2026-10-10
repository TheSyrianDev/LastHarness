//! The turn loop against a scripted model, real files in a temp folder, and a scripted user.
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use lh_provider::*;
use lh_tools::*;

struct Script(Mutex<Vec<Response>>);
impl Script {
    fn new(mut r: Vec<Response>) -> Self {
        r.reverse();
        Self(Mutex::new(r))
    }
}
impl Model for Script {
    async fn complete(&self, _req: &ChatRequest, _on: &mut (dyn FnMut(Event) + Send)) -> Result<Response, Error> {
        Ok(self.0.lock().unwrap().pop().expect("model asked more often than scripted"))
    }
}

struct User {
    answer: Answer,
    asked: AtomicUsize,
}
impl Asker for User {
    async fn ask(&self, _r: &AskRequest<'_>) -> Answer {
        self.asked.fetch_add(1, Ordering::SeqCst);
        self.answer
    }
}
fn user(a: Answer) -> User {
    User { answer: a, asked: AtomicUsize::new(0) }
}

fn call(id: &str, name: &str, args: &str) -> ToolCall {
    ToolCall { id: id.into(), name: name.into(), arguments: args.into(), repaired: false }
}
fn tools_reply(calls: Vec<ToolCall>) -> Response {
    Response { tool_calls: calls, finish: Some(FinishReason::ToolCalls), ..Default::default() }
}
fn text_reply(t: &str) -> Response {
    Response { text: t.into(), finish: Some(FinishReason::Stop), ..Default::default() }
}
fn req() -> ChatRequest {
    ChatRequest { model: "m".into(), messages: vec![Message::user("do it")], ..Default::default() }
}
fn tool_results(r: &ChatRequest) -> Vec<String> {
    r.messages.iter().filter(|m| m.role == Role::Tool).map(|m| m.content.clone().unwrap()).collect()
}

#[tokio::test]
async fn reads_then_answers() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("a.txt"), "hello file").unwrap();
    let perms = Permissions::new(d.path()).unwrap();
    let model = Script::new(vec![tools_reply(vec![call("1", "read_file", r#"{"path":"a.txt"}"#)]), text_reply("it says hello")]);
    let (u, mut r) = (user(Answer::No), req());
    let end = run_turn(&model, &mut r, &default_tools(), &perms, &u, &TurnOptions::default(), |_| {}).await.unwrap();
    assert!(matches!(end, TurnEnd::Done(ref x) if x.text == "it says hello"));
    assert_eq!(tool_results(&r), ["hello file"]);
    assert_eq!(u.asked.load(Ordering::SeqCst), 0, "reads in the project do not ask");
    assert_eq!(r.messages.last().unwrap().content.as_deref(), Some("it says hello"));
}

#[tokio::test]
async fn write_asks_once_then_not_again() {
    let d = tempfile::tempdir().unwrap();
    let perms = Permissions::new(d.path()).unwrap();
    let model = Script::new(vec![
        tools_reply(vec![call("1", "write_file", r#"{"path":"src/a.txt","content":"one"}"#)]),
        tools_reply(vec![call("2", "write_file", r#"{"path":"b.txt","content":"two"}"#)]),
        text_reply("done"),
    ]);
    let (u, mut r) = (user(Answer::Yes), req());
    run_turn(&model, &mut r, &default_tools(), &perms, &u, &TurnOptions::default(), |_| {}).await.unwrap();
    assert_eq!(std::fs::read_to_string(d.path().join("src/a.txt")).unwrap(), "one");
    assert_eq!(std::fs::read_to_string(d.path().join("b.txt")).unwrap(), "two");
    assert_eq!(u.asked.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn no_means_nothing_happens() {
    let d = tempfile::tempdir().unwrap();
    let perms = Permissions::new(d.path()).unwrap();
    let model = Script::new(vec![tools_reply(vec![call("1", "write_file", r#"{"path":"a.txt","content":"x"}"#)]), text_reply("ok, stopping")]);
    let (u, mut r) = (user(Answer::No), req());
    run_turn(&model, &mut r, &default_tools(), &perms, &u, &TurnOptions::default(), |_| {}).await.unwrap();
    assert!(!d.path().join("a.txt").exists());
    assert!(tool_results(&r)[0].starts_with("The user said no"));
}

#[tokio::test]
async fn escapes_sudo_and_unknown_tools_are_stopped() {
    let d = tempfile::tempdir().unwrap();
    let perms = Permissions::new(d.path()).unwrap();
    let model = Script::new(vec![
        tools_reply(vec![
            call("1", "run_command", r#"{"command":"sudo ls"}"#),
            call("2", "read_file", r#"{"path":"../../etc/passwd"}"#),
            call("3", "launch_missiles", "{}"),
            call("4", "read_file", "{not json"),
        ]),
        text_reply("ok"),
    ]);
    let (u, mut r) = (user(Answer::No), req());
    run_turn(&model, &mut r, &default_tools(), &perms, &u, &TurnOptions::default(), |_| {}).await.unwrap();
    let res = tool_results(&r);
    assert!(res[0].starts_with("Denied: sudo is turned off"), "{res:?}");
    assert!(res[1].starts_with("The user said no"), "outside the project asks, and the answer was no");
    assert!(res[2].contains("not available"));
    assert!(res[3].contains("not valid JSON"));
}

#[tokio::test]
async fn cut_off_calls_are_never_run() {
    let d = tempfile::tempdir().unwrap();
    let mut perms = Permissions::new(d.path()).unwrap();
    perms.mode = Mode::Bypass;
    let mut cut = call("1", "run_command", r#"{"command":"touch should-not-exist"}"#);
    cut.repaired = true;
    let model = Script::new(vec![tools_reply(vec![cut]), text_reply("ok")]);
    let (u, mut r) = (user(Answer::Yes), req());
    run_turn(&model, &mut r, &default_tools(), &perms, &u, &TurnOptions::default(), |_| {}).await.unwrap();
    assert!(!d.path().join("should-not-exist").exists());
    assert!(tool_results(&r)[0].contains("cut off"));
}

#[tokio::test]
async fn a_worker_only_gets_its_tools_and_steps_are_capped() {
    let d = tempfile::tempdir().unwrap();
    let perms = Permissions::new(d.path()).unwrap();
    let read_only = ToolSet::new([Tool::ReadFile, Tool::ListDir]);
    let looping: Vec<_> = (0..5).map(|i| tools_reply(vec![call(&i.to_string(), "write_file", r#"{"path":"a","content":"x"}"#)])).collect();
    let model = Script::new(looping);
    let (u, mut r) = (user(Answer::Yes), req());
    let end = run_turn(&model, &mut r, &read_only, &perms, &u, &TurnOptions { max_steps: 3, worker: "researcher".into() }, |_| {}).await.unwrap();
    assert!(matches!(end, TurnEnd::StepLimit));
    assert!(!d.path().join("a").exists());
    assert!(tool_results(&r).iter().all(|t| t.contains("not available")));
    assert_eq!(r.tools.len(), 2, "the model is only told about its own tools");
}

#[tokio::test]
async fn command_runs_in_project_with_exit_code_and_timeout() {
    let d = tempfile::tempdir().unwrap();
    let mut perms = Permissions::new(d.path()).unwrap();
    perms.allow_rules = vec!["sleep".into(), "pwd".into(), "ls".into()];
    let model = Script::new(vec![
        tools_reply(vec![
            call("1", "run_command", r#"{"command":"pwd"}"#),
            call("2", "run_command", r#"{"command":"ls nope"}"#),
            call("3", "run_command", r#"{"command":"sleep 5","timeout_secs":1}"#),
        ]),
        text_reply("ok"),
    ]);
    let (u, mut r) = (user(Answer::No), req());
    run_turn(&model, &mut r, &default_tools(), &perms, &u, &TurnOptions::default(), |_| {}).await.unwrap();
    let res = tool_results(&r);
    assert!(res[0].starts_with("exit code 0") && res[0].contains(&d.path().canonicalize().unwrap().display().to_string()), "{}", res[0]);
    assert!(res[1].starts_with("exit code 2") && res[1].contains("[stderr]"), "{}", res[1]);
    assert!(res[2].contains("ran longer than 1 s"), "{}", res[2]);
}
