//! `lh learn ...` against a real core.
use std::sync::Arc;
use std::time::Duration;

use lh_core::{auth, router, AppState, LearnMode, LearnRuntime, Settings};
use lh_learn::Learn;
use lh_memory::Trust;
use lh_provider::{ChatRequest, Event, Response};
use lh_store::Store;
use lh_tools::Model;
use lh_tui::client::Client;
use lh_tui::learn_cli::run;

struct Nothing;
impl Model for Nothing {
    async fn complete(&self, _: &ChatRequest, _: &mut (dyn FnMut(Event) + Send)) -> Result<Response, lh_provider::Error> {
        Ok(Response::default())
    }
}

async fn core() -> (tempfile::TempDir, Arc<Learn>, Client) {
    let d = tempfile::tempdir().unwrap();
    let store = Arc::new(Store::open(&d.path().join("lh.db")).unwrap());
    let learn = Arc::new(Learn::new(store.clone()).with_skills_dir(d.path().join("skills")));
    let settings = Settings {
        provider_name: None,
        learn: Some(LearnRuntime { learn: learn.clone(), trust: Trust::Local, mode: LearnMode::Propose, threshold: 3, skill_min_calls: 5 }),
        maintain_when_idle: false,
        memory: None,
        image_model: None,
        web: Default::default(),
        idle_after: Duration::ZERO,
        compact_at: Some(0),
        store: store.clone(),
        spend: lh_spend::SpendMeter::new(store, lh_spend::Config::default(), "t", true),
        bypass: false,
        sudo_enabled: false,
        allow_rules: vec![],
        team: None,
        mcp: None,
        ask_all: false,
        config: None,
    };
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    let token = auth::new_token();
    let app = router(AppState::new(Nothing, settings), token.clone(), port);
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    (d, learn, Client::new(format!("http://127.0.0.1:{port}"), token))
}
fn args(s: &str) -> Vec<String> {
    s.split_whitespace().map(str::to_string).collect()
}
async fn ok(c: &Client, s: &str) -> String {
    run(c, &args(s)).await.unwrap_or_else(|e| panic!("{s}: {e}"))
}

#[tokio::test]
async fn rules_style_and_suggestions_from_the_terminal() {
    let (_d, learn, c) = core().await;
    assert_eq!(ok(&c, "rules").await, "No rules yet.");
    assert!(ok(&c, "help").await.contains("lh learn <command>"));

    let added = ok(&c, "rule add Always add links after research --topic links").await;
    assert!(added.starts_with("Added: r_") && added.contains("Always add links after research"), "{added}");
    let id = added.split_whitespace().nth(1).unwrap().to_string();
    let secret = ok(&c, "rule add My doctor prescribed metformin for diabetes").await;
    assert!(secret.contains("[sensitive]"), "{secret}");

    assert_eq!(ok(&c, "rules").await.lines().count(), 2);
    assert!(ok(&c, &format!("rule edit {id} -- Always add source links after research")).await.contains("source links"));
    assert!(ok(&c, &format!("rule off {id}")).await.contains("[off]"));
    assert_eq!(ok(&c, "rules").await.lines().count(), 1);
    assert_eq!(ok(&c, "rules --all").await.lines().count(), 2);
    assert!(ok(&c, &format!("rule on {id}")).await.starts_with("On:"));
    let shown = ok(&c, &format!("rule show {id}")).await;
    assert!(shown.contains("from: user") && shown.contains("v1  create") && shown.contains("v2  edit") && shown.contains("v3  disable") && shown.contains("v4  enable"), "{shown}");
    assert!(ok(&c, &format!("rule rollback {id} 1")).await.contains("Always add links after research"));
    assert!(run(&c, &args(&format!("rule rollback {id} nine"))).await.is_err());

    assert_eq!(ok(&c, "style").await, "No style profile yet.");
    assert!(ok(&c, "style set -- Terse. Plain words.").await.contains("version 1"));
    assert!(ok(&c, "style").await.starts_with("Terse. Plain words."));
    ok(&c, "style set -- Chatty.").await;
    assert!(ok(&c, "style rollback 1").await.contains("version 3"));

    // a suggestion waits, then is accepted with the user's own wording
    let ids = learn.propose_from_drafts(lh_learn::parse_drafts(r#"[{"kind":"rule","topic":"bullets","text":"Do not use bullet points.","evidence":"never use bullet points"}]"#), "repeat", Some("s1"), None, &[]).unwrap();
    let list = ok(&c, "proposals").await;
    assert!(list.contains("new rule: Do not use bullet points.") && list.contains("because: \"never use bullet points\""), "{list}");
    let done = ok(&c, &format!("accept {} --text Never use bullet points", ids[0])).await;
    assert!(done.starts_with("Now a rule: ") && done.contains("Never use bullet points"), "{done}");
    assert_eq!(ok(&c, "proposals").await, "No suggestions waiting.");
    assert!(ok(&c, &format!("rule delete {id}")).await.contains("Deleted"));
    assert!(run(&c, &args("rule show r_nope")).await.is_err());
    assert!(run(&c, &args("bogus")).await.unwrap_err().contains("unknown learn command"));
    assert!(ok(&c, "habits").await.contains("No habit"));
    assert!(ok(&c, "signals").await.contains("Nothing noted"));
}

#[tokio::test]
async fn skills_install_from_a_folder_and_roll_back() {
    let (d, _l, c) = core().await;
    assert_eq!(ok(&c, "skills").await, "No skills yet.");
    let dir = d.path().join("mine/count-files");
    std::fs::create_dir_all(dir.join("references")).unwrap();
    std::fs::write(dir.join("SKILL.md"), "---\nname: count-files\ndescription: >\n  Count files in a folder.\n  Use when asked how many.\n---\n\n1. List the folder.\n").unwrap();
    std::fs::write(dir.join("references/notes.md"), "Skip hidden files.").unwrap();
    assert_eq!(ok(&c, &format!("skill add {}", dir.display())).await, "Installed skill count-files.");
    assert_eq!(ok(&c, "skills").await, "count-files  Count files in a folder. Use when asked how many.");
    let shown = ok(&c, "skill show count-files").await;
    assert!(shown.contains("other files: references/notes.md") && shown.contains("1. List the folder.") && shown.contains("versions: 1"), "{shown}");
    std::fs::write(dir.join("SKILL.md"), std::fs::read_to_string(dir.join("SKILL.md")).unwrap() + "2. Count them.\n").unwrap();
    ok(&c, &format!("skill add {}", dir.display())).await;
    assert!(ok(&c, "skill show count-files").await.contains("2. Count them."));
    assert!(ok(&c, "skill rollback count-files 1").await.contains("back to version 1"));
    assert!(!ok(&c, "skill show count-files").await.contains("2. Count them."));
    assert!(run(&c, &args(&format!("skill add {}", d.path().display()))).await.unwrap_err().contains("no SKILL.md"));
    assert!(ok(&c, "skill delete count-files").await.contains("Deleted"));
    assert_eq!(ok(&c, "skills").await, "No skills yet.");
}
