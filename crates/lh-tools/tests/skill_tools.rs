//! The assistant reading skills, through the real turn loop.
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use lh_learn::{render_skill_md, Learn, SkillMeta};
use lh_provider::{ChatRequest, Event, FinishReason, Message, Response, Role, ToolCall};
use lh_store::Store;
use lh_tools::*;

struct Script(Mutex<Vec<Response>>);
impl Model for Script {
    async fn complete(&self, _r: &ChatRequest, _on: &mut (dyn FnMut(Event) + Send)) -> Result<Response, lh_provider::Error> {
        Ok(self.0.lock().unwrap().pop().expect("unscripted call"))
    }
}
struct Nobody;
impl Asker for Nobody {
    async fn ask(&self, r: &AskRequest<'_>) -> Answer {
        panic!("reading skills must not ask: {}", r.tool)
    }
}
fn call(name: &str, args: &str) -> Response {
    Response { tool_calls: vec![ToolCall { id: "c".into(), name: name.into(), arguments: args.into(), repaired: false }], finish: Some(FinishReason::ToolCalls), ..Default::default() }
}

#[tokio::test]
async fn use_skill_gives_the_steps_and_read_skill_file_gives_the_rest() {
    let d = tempfile::tempdir().unwrap();
    let learn = Arc::new(Learn::new(Arc::new(Store::open_memory().unwrap())).with_skills_dir(d.path().join("skills")));
    let meta = SkillMeta { name: "release-notes".into(), description: "Write release notes. Use before a release.".into(), license: None, compatibility: None, allowed_tools: vec![], metadata: Default::default() };
    let files = BTreeMap::from([("SKILL.md".to_string(), render_skill_md(&meta, "1. Read git log.\n2. Group by type.")), ("references/style.md".to_string(), "Keep a changelog.".to_string())]);
    learn.install_skill(&files, "create", "user", "").unwrap();

    let tools = default_tools().with_skills(learn);
    assert!(tools.get("use_skill").is_some() && tools.get("read_skill_file").is_some());
    let proj = d.path().join("proj");
    std::fs::create_dir(&proj).unwrap();
    let perms = Permissions::new(&proj).unwrap();
    let replies = vec![
        call("use_skill", r#"{"name":"release-notes"}"#),
        call("read_skill_file", r#"{"name":"release-notes","path":"references/style.md"}"#),
        call("read_skill_file", r#"{"name":"release-notes","path":"../../etc/passwd"}"#),
        call("use_skill", r#"{"name":"missing"}"#),
        Response { text: "done".into(), finish: Some(FinishReason::Stop), ..Default::default() },
    ];
    // one call per model turn, in order
    let model = Script(Mutex::new(replies.into_iter().rev().collect()));
    let mut req = ChatRequest { model: "m".into(), messages: vec![Message::user("write notes")], ..Default::default() };
    run_turn(&model, &NoMeter, &mut req, &tools, &perms, &Nobody, &TurnOptions::default(), |_| {}).await.unwrap();
    let r: Vec<String> = req.messages.iter().filter(|m| m.role == Role::Tool).map(|m| m.content.clone().unwrap()).collect();
    assert!(r[0].starts_with("Skill: release-notes\nFolder: ") && r[0].contains("references/style.md") && r[0].contains("1. Read git log."), "{}", r[0]);
    assert_eq!(r[1], "Keep a changelog.");
    assert!(r[2].starts_with("Error:") && !r[2].contains("root:"), "{}", r[2]);
    assert!(r[3].contains("there is no skill called missing"));
    assert!(default_tools().get("use_skill").is_none(), "without skills set up the tools do not exist");
}
