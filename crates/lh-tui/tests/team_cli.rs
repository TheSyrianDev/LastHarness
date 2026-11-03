//! `lh team ...` against a real core with a scripted model.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use lh_core::{auth, router, AppState, McpRuntime, Settings, TeamRuntime};
use lh_provider::{ChatRequest, Event, FinishReason, Response};
use lh_store::Store;
use lh_tools::Model;
use lh_tui::client::Client;
use lh_tui::team_cli::run;

struct Script(Mutex<Vec<Response>>);
impl Model for Script {
    async fn complete(&self, _: &ChatRequest, on: &mut (dyn FnMut(Event) + Send)) -> Result<Response, lh_provider::Error> {
        let r = self.0.lock().unwrap().pop().expect("unscripted model call");
        on(Event::Text(r.text.clone()));
        Ok(r)
    }
}
fn text(t: &str) -> Response {
    Response { text: t.into(), finish: Some(FinishReason::Stop), ..Default::default() }
}

async fn core(mut replies: Vec<Response>) -> (tempfile::TempDir, Client) {
    replies.reverse();
    let d = tempfile::tempdir().unwrap();
    let store = Arc::new(Store::open(&d.path().join("lh.db")).unwrap());
    let team = TeamRuntime::open(&d.path().join("team"), Arc::new(lh_learn::Learn::new(store.clone())), 1000).unwrap();
    let settings = Settings {
        team: Some(team),
        mcp: Some(McpRuntime::open(d.path().join("mcp.json"))),
        ask_all: false,
        config: None,
        provider_name: None,
        learn: None,
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
    };
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    let token = auth::new_token();
    let app = router(AppState::new(Script(Mutex::new(replies)), settings), token.clone(), port);
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    (d, Client::new(format!("http://127.0.0.1:{port}"), token))
}
fn args(s: &str) -> Vec<String> {
    s.split_whitespace().map(str::to_string).collect()
}
async fn ok(c: &Client, s: &str) -> String {
    run(c, &args(s)).await.unwrap_or_else(|e| panic!("{s}: {e}"))
}

#[tokio::test]
async fn roles_types_departments_and_settings_from_the_terminal() {
    let (d, c) = core(vec![]).await;
    assert!(ok(&c, "help").await.contains("lh team <command>"));
    let roles = ok(&c, "roles").await;
    assert_eq!(roles.lines().count(), 5);
    assert!(roles.contains("coder") && roles.contains("type:mid-tier"), "{roles}");
    let text = ok(&c, "role show coder").await;
    let file = d.path().join("coder.md");
    std::fs::write(&file, text.replace("Writes, changes and tests code.", "Writes and tests code carefully.")).unwrap();
    assert!(ok(&c, &format!("role set coder {}", file.display())).await.starts_with("Saved coder as version"));
    assert!(ok(&c, "roles").await.contains("carefully"));
    assert!(ok(&c, "role history coder").await.lines().count() >= 2);

    assert_eq!(ok(&c, "types").await.lines().count(), 3);
    assert!(ok(&c, "type set mid-tier qwen").await.contains("qwen"));
    assert!(ok(&c, "type fallbacks mid-tier small other").await.contains("then small, other"));
    let err = run(&c, &args("type delete mid-tier")).await.unwrap_err();
    assert!(err.contains("coder"), "a used type cannot be deleted without a new home: {err}");
    assert!(ok(&c, "type add cheap").await.contains("(no model yet)"));

    let dept = d.path().join("pets.md");
    std::fs::write(&dept, "---\nname: pets\ndescription: Pets.\nlead: planner\nmembers: researcher writer\n---\nBe brief.\n").unwrap();
    assert!(ok(&c, &format!("dept set pets {}", dept.display())).await.starts_with("Saved pets"));
    let depts = ok(&c, "depts").await;
    assert!(depts.contains("pets") && depts.contains("researcher writer"), "{depts}");
    assert!(run(&c, &args("dept delete ghost")).await.is_err());

    assert!(ok(&c, "set parallel 3").await.contains("parallel workers: 3"));
    assert!(ok(&c, "set autonomy fire-and-forget").await.contains("autonomy: fire-and-forget"));
    assert!(run(&c, &args("set parallel 99")).await.is_err());
    assert!(ok(&c, "away on").await.contains("Away mode is on"));
    assert!(ok(&c, "away off").await.contains("Away mode is off"));
    assert_eq!(ok(&c, "workers").await, "Nobody is working right now.");
}

#[tokio::test]
async fn a_goal_is_followed_to_its_result() {
    let plan = r#"{"steps":[{"id":"1","title":"Find facts","assignee":"researcher","brief":"Find facts.","depends_on":[]}]}"#;
    let (d, c) = core(vec![text(plan), text("Cats purr."), text("OK"), text("Cats purr, and that is all.")]).await;
    ok(&c, "type set frontier big").await;
    ok(&c, "type set mid-tier mid").await;
    ok(&c, "set autonomy fire-and-forget").await;
    let dept = d.path().join("pets.md");
    std::fs::write(&dept, "---\nname: pets\nlead: planner\nmembers: researcher\n---\n").unwrap();
    ok(&c, &format!("dept set pets {}", dept.display())).await;
    let out = ok(&c, "run pets -- tell me about cats").await;
    assert!(out.contains("Cats purr, and that is all."), "{out}");
    assert!(ok(&c, "runs").await.contains("finished"));
    assert!(run(&c, &args("run pets")).await.unwrap_err().contains("goal after --"));
}

#[tokio::test]
async fn a_plugin_is_previewed_then_installed_from_the_terminal() {
    let (d, c) = core(vec![]).await;
    let p = d.path().join("kit");
    std::fs::create_dir_all(p.join("roles")).unwrap();
    std::fs::write(p.join("roles/proofreader.md"), "---\nname: proofreader\ndescription: Reads for typos.\ntype: fastest\ntools: read_file\n---\nProofread.\n").unwrap();
    std::fs::write(p.join("roles/writer.md"), "---\nname: writer\ndescription: Other writer.\ntype: fastest\n---\nWrite.\n").unwrap();
    let pv = ok(&c, &format!("plugin preview {}", p.display())).await;
    assert!(pv.contains("proofreader") && pv.contains("writer (you already have one)"), "{pv}");
    assert!(!ok(&c, "roles").await.contains("proofreader"), "a preview installs nothing");
    let out = ok(&c, &format!("plugin install {}", p.display())).await;
    assert!(out.contains("installed role proofreader") && out.contains("skipped role writer"), "{out}");
    assert!(ok(&c, "roles").await.contains("proofreader"));
    let out = ok(&c, &format!("plugin install {} --replace writer", p.display())).await;
    assert!(out.contains("installed role writer"), "{out}");
}

#[tokio::test]
async fn mcp_servers_are_added_shown_approved_and_folders_trusted_from_the_terminal() {
    use lh_tui::mcp_cli::{run as mcp, trust};
    let (d, c) = core(vec![]).await;
    assert_eq!(mcp(&c, &args("list")).await.unwrap(), "No MCP servers.");
    let script = format!("{}/../lh-mcp/tests/mock_server.py", env!("CARGO_MANIFEST_DIR"));
    let out = mcp(&c, &args(&format!("add mock --command python3 -I {script}"))).await.unwrap();
    assert!(out.contains("python3 -I") && out.contains("Nothing runs until you approve it"), "{out}");
    assert!(mcp(&c, &args("list")).await.unwrap().contains("waiting for your approval"));
    let out = mcp(&c, &args("approve mock")).await.unwrap();
    assert!(out.contains("echo") && out.contains("boom"), "{out}");
    let list = mcp(&c, &args("list")).await.unwrap();
    assert!(list.contains("[local, approved]") && list.contains("mcp__mock__echo  Repeats the text."), "{list}");
    assert!(mcp(&c, &args("add x")).await.is_err());
    assert_eq!(mcp(&c, &args("remove mock")).await.unwrap(), "Removed.");

    let proj = d.path().join("repo");
    std::fs::create_dir_all(proj.join(".lastharness")).unwrap();
    std::fs::write(proj.join(".lastharness/mcp.json"), format!(r#"{{"servers":[{{"name":"tools","transport":{{"kind":"stdio","command":"python3","args":["-I","{script}"]}}}}]}}"#)).unwrap();
    let shown = trust(&c, &[proj.display().to_string()]).await.unwrap();
    assert!(shown.contains("decision: none yet") && shown.contains("asks to run tools: python3 -I"), "{shown}");
    assert!(trust(&c, &[proj.display().to_string(), "trust".into()]).await.unwrap().ends_with(": trust"));
    assert!(trust(&c, &[proj.display().to_string(), "maybe".into()]).await.is_err());
}
