//! Roles, model types, departments and runs through the real local API, with a scripted model.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use lh_core::{auth, router, AppState, Settings, TeamRuntime};
use lh_provider::{ChatRequest, Event, FinishReason, Response, SseParser};
use lh_store::Store;
use lh_tools::Model;
use serde_json::{json, Value};

struct Script {
    replies: Mutex<Vec<Response>>,
    seen: Arc<Mutex<Vec<ChatRequest>>>,
    models: Vec<String>,
}
impl Model for Script {
    async fn complete(&self, r: &ChatRequest, on: &mut (dyn FnMut(Event) + Send)) -> Result<Response, lh_provider::Error> {
        self.seen.lock().unwrap().push(r.clone());
        let resp = self.replies.lock().unwrap().pop().expect("unscripted model call");
        on(Event::Text(resp.text.clone()));
        Ok(resp)
    }
    async fn models(&self) -> Result<Vec<String>, lh_provider::Error> {
        Ok(self.models.clone())
    }
}
fn text(t: &str) -> Response {
    Response { text: t.into(), finish: Some(FinishReason::Stop), ..Default::default() }
}

struct Server {
    base: String,
    token: String,
    http: reqwest::Client,
    seen: Arc<Mutex<Vec<ChatRequest>>>,
    _dir: tempfile::TempDir,
}
impl Server {
    fn req(&self, m: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        self.http.request(m, format!("{}{path}", self.base)).bearer_auth(&self.token)
    }
    async fn json(&self, rb: reqwest::RequestBuilder) -> (u16, Value) {
        let r = rb.send().await.unwrap();
        let code = r.status().as_u16();
        (code, r.json().await.unwrap_or(Value::Null))
    }
    async fn get(&self, p: &str) -> Value {
        self.json(self.req(reqwest::Method::GET, p)).await.1
    }
    async fn post(&self, p: &str, b: Value) -> (u16, Value) {
        self.json(self.req(reqwest::Method::POST, p).json(&b)).await
    }
    async fn put(&self, p: &str, b: Value) -> (u16, Value) {
        self.json(self.req(reqwest::Method::PUT, p).json(&b)).await
    }
    /// Follow a run to its end, answering questions with `answer`. Returns every event seen.
    async fn follow(&self, run: &str, answer: impl Fn(&Value) -> Option<Value>) -> Vec<Value> {
        let mut resp = self.req(reqwest::Method::GET, &format!("/v1/team/runs/{run}/events")).send().await.unwrap();
        let mut p = SseParser::new();
        let mut out = vec![];
        while let Some(chunk) = tokio::time::timeout(Duration::from_secs(15), resp.chunk()).await.expect("no event in 15 s").unwrap() {
            for d in p.push(&chunk) {
                let v: Value = serde_json::from_str(&d).unwrap();
                if let Some(a) = answer(&v) {
                    self.post(&format!("/v1/team/runs/{run}/answers/{}", v["id"].as_str().unwrap()), a).await;
                }
                out.push(v);
            }
        }
        out
    }
}

fn start(mut replies: Vec<Response>, models: &[&str]) -> Server {
    replies.reverse();
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(Store::open(&dir.path().join("lh.db")).unwrap());
    let learn = Arc::new(lh_learn::Learn::new(store.clone()));
    let team = TeamRuntime::open(&dir.path().join("team"), learn, 1000).unwrap();
    let seen: Arc<Mutex<Vec<ChatRequest>>> = Default::default();
    let settings = Settings {
        team: Some(team),
        mcp: None,
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
        spend: lh_spend::SpendMeter::new(store, lh_spend::Config::default(), "test", true),
        bypass: false,
        sudo_enabled: false,
        allow_rules: vec![],
    };
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    l.set_nonblocking(true).unwrap();
    let port = l.local_addr().unwrap().port();
    let token = auth::new_token();
    let model = Script { replies: Mutex::new(replies), seen: seen.clone(), models: models.iter().map(|m| m.to_string()).collect() };
    let app = router(AppState::new(model, settings), token.clone(), port);
    tokio::spawn(async move { axum::serve(tokio::net::TcpListener::from_std(l).unwrap(), app).await.unwrap() });
    Server { base: format!("http://127.0.0.1:{port}"), token, http: reqwest::Client::new(), seen, _dir: dir }
}

/// The stock types start without a model; give the two that the starter roles use one.
async fn with_models(s: Server) -> Server {
    s.put("/v1/team/types/frontier", json!({"model": "big"})).await;
    s.put("/v1/team/types/mid-tier", json!({"model": "mid"})).await;
    s
}

const PLAN: &str = r#"{"steps":[{"id":"1","title":"Find facts","assignee":"researcher","brief":"Find three facts about cats.","depends_on":[]}]}"#;

#[tokio::test]
async fn starter_roles_types_and_settings_can_be_read_and_changed() {
    let s = start(vec![], &[]);
    let roles = s.get("/v1/team/roles").await;
    let names: Vec<&str> = roles["roles"].as_array().unwrap().iter().map(|r| r["name"].as_str().unwrap()).collect();
    assert_eq!(names.len(), 5);
    assert!(names.contains(&"planner") && names.contains(&"coder"));
    let coder = s.get("/v1/team/roles/coder").await;
    assert_eq!(coder["target"]["kind"], "type");
    // the form fields the settings screen needs come with the role
    assert!(coder["instructions"].as_str().unwrap().starts_with("You write and change code"));
    assert_eq!((coder["memory"].as_str(), coder["sampling"]["temperature"].as_f64().map(|t| (t * 10.).round() as i64), coder["sampling"]["top_k"].is_null()), (Some("preferences projects"), Some(2), true));
    // edit through the file text, then roll back
    let text = coder["text"].as_str().unwrap().replace("Writes, changes and tests code.", "Writes and tests code carefully.");
    let (code, saved) = s.put("/v1/team/roles/coder", json!({"text": text})).await;
    assert_eq!((code, saved["version"].as_i64().unwrap() > 1), (200, true), "{saved}");
    assert!(s.get("/v1/team/roles/coder").await["text"].as_str().unwrap().contains("carefully"));
    let hist = s.get("/v1/team/roles/coder/history").await;
    assert!(hist["versions"].as_array().unwrap().len() >= 2);
    let (code, _) = s.put("/v1/team/roles/coder", json!({"text": "not a role"})).await;
    assert_eq!(code, 400);
    let (code, _) = s.req(reqwest::Method::GET, "/v1/team/roles/ghost").send().await.map(|r| (r.status().as_u16(), ())).unwrap();
    assert_eq!(code, 404);

    // types: stock three, add one, give one a model, deleting a used one asks for a new home
    let types = s.get("/v1/team/types").await;
    assert_eq!(types["types"].as_array().unwrap().len(), 3);
    let (code, t) = s.put("/v1/team/types/mid-tier", json!({"model": "qwen", "fallbacks": ["small"]})).await;
    assert_eq!((code, t["model"].as_str(), t["fallbacks"][0].as_str()), (200, Some("qwen"), Some("small")));
    s.post("/v1/team/types", json!({"name": "cheap", "model": null})).await;
    let (code, body) = s.json(s.req(reqwest::Method::DELETE, "/v1/team/types/mid-tier")).await;
    assert_eq!(code, 409, "{body}");
    assert!(body["error"].as_str().unwrap().contains("coder"));

    // settings
    let (_, set) = s.put("/v1/team/settings", json!({"parallel": 3, "autonomy": "check-ins", "attempts": 2, "unavailable": "fallback"})).await;
    assert_eq!((set["parallel"].as_u64(), set["autonomy"].as_str(), set["attempts"].as_u64(), set["unavailable"].as_str()), (Some(3), Some("check-ins"), Some(2), Some("fallback")));
    assert_eq!(s.put("/v1/team/settings", json!({"parallel": 99})).await.0, 400);
}

#[tokio::test]
async fn a_department_is_made_planned_approved_run_and_its_events_replay() {
    // replies in order: plan, the researcher's work, the lead's check, the lead's summary
    let s = with_models(start(vec![text(PLAN), text("Cats purr. Cats sleep. Cats climb."), text("OK"), text("Three facts about cats were found.")], &[])).await;
    let dept = "---\nname: pets\ndescription: Answers pet questions.\nlead: planner\nmembers: researcher\n---\nBe brief.\n";
    let (code, d) = s.put("/v1/team/departments/pets", json!({"text": dept})).await;
    assert_eq!(code, 200, "{d}");
    assert_eq!(s.put("/v1/team/departments/bad", json!({"text": dept.replace("researcher", "ghost")})).await.0, 400, "a member that does not exist is refused");
    s.put("/v1/team/settings", json!({"autonomy": "plan-first"})).await;
    let (code, run) = s.post("/v1/team/runs", json!({"department": "pets", "goal": "tell me about cats"})).await;
    assert_eq!(code, 201);
    let id = run["id"].as_str().unwrap().to_string();
    let events = s.follow(&id, |e| (e["type"] == "plan_needed").then(|| json!({"decision": "approve"}))).await;
    let kinds: Vec<&str> = events.iter().map(|e| e["type"].as_str().unwrap()).collect();
    assert_eq!(kinds.first(), Some(&"planning"));
    for k in ["plan", "plan_needed", "plan_decided", "step_started", "step_checked", "step_done", "finished", "outcome"] {
        assert!(kinds.contains(&k), "{k} missing in {kinds:?}");
    }
    let out = events.last().unwrap();
    assert_eq!((out["status"].as_str(), out["result"].as_str()), (Some("done"), Some("Three facts about cats were found.")));
    // a late reader sees the same thing
    let again = s.get(&format!("/v1/team/runs/{id}")).await;
    assert_eq!(again["events"].as_array().unwrap().len(), events.len());
    assert_eq!(again["finished"], true);
    // each worker was sent a short prompt: base, role line, brief
    let seen = s.seen.lock().unwrap();
    let worker = seen.iter().find(|r| r.messages.iter().any(|m| m.content.as_deref() == Some("Find three facts about cats."))).expect("the researcher ran");
    assert_eq!(worker.messages.len(), 2);
    assert!(lh_tools::estimate_text_tokens(worker.messages[0].content.as_deref().unwrap()) <= 300);
}

#[tokio::test]
async fn the_user_can_reject_a_plan_and_away_mode_decides_for_them_and_says_so() {
    let s = with_models(start(vec![text(PLAN)], &[])).await;
    s.put("/v1/team/departments/pets", json!({"text": "---\nname: pets\ndescription: x\nlead: planner\nmembers: researcher\n---\n"})).await;
    let (_, run) = s.post("/v1/team/runs", json!({"department": "pets", "goal": "go"})).await;
    let events = s.follow(run["id"].as_str().unwrap(), |e| (e["type"] == "plan_needed").then(|| json!({"decision": "reject", "note": "too big"}))).await;
    assert_eq!(events.last().unwrap()["status"], "rejected");
    assert_eq!(events.last().unwrap()["note"], "too big");

    let s = with_models(start(vec![text(PLAN), text("facts"), text("OK"), text("done")], &[])).await;
    s.put("/v1/team/departments/pets", json!({"text": "---\nname: pets\ndescription: x\nlead: planner\nmembers: researcher\n---\n"})).await;
    let (_, away) = s.post("/v1/team/away", json!({"on": true})).await;
    assert_eq!(away["away"], true);
    let (_, run) = s.post("/v1/team/runs", json!({"department": "pets", "goal": "go"})).await;
    let events = s.follow(run["id"].as_str().unwrap(), |_| None).await;
    assert!(events.iter().all(|e| e["type"] != "plan_needed"), "nobody is asked in away mode");
    assert!(events.iter().any(|e| e["type"] == "plan_decided" && e["note"].as_str().unwrap().contains("away")));
    let summary = s.get("/v1/team/away").await;
    assert!(summary["decisions"].as_array().unwrap().iter().any(|d| d["decision"] == "approved the plan"), "{summary}");
    let (_, back) = s.post("/v1/team/away", json!({"on": false})).await;
    assert_eq!(back["away"], false);
}

#[tokio::test]
async fn a_run_started_for_a_chat_is_listed_with_that_chat_only() {
    let s = start(vec![text(PLAN)], &["only-this-one"]);
    s.put("/v1/team/departments/pets", json!({"text": "---\nname: pets\ndescription: x\nlead: planner\nmembers: researcher\n---\n"})).await;
    let (_, mine) = s.post("/v1/team/runs", json!({"department": "pets", "goal": "go", "session": "s7"})).await;
    let (_, other) = s.post("/v1/team/runs", json!({"department": "pets", "goal": "go"})).await;
    let all = s.get("/v1/team/runs").await;
    assert_eq!(all["runs"].as_array().unwrap().len(), 2);
    let ours = s.get("/v1/team/runs?session=s7").await;
    let ours = ours["runs"].as_array().unwrap();
    assert_eq!(ours.len(), 1);
    assert_eq!((&ours[0]["id"], &ours[0]["session"]), (&mine["id"], &json!("s7")));
    let one = s.get(&format!("/v1/team/runs/{}", other["id"].as_str().unwrap())).await;
    assert!(one["session"].is_null(), "a run started from the Departments page belongs to no chat");
}

#[tokio::test]
async fn a_model_that_is_not_served_stops_the_run_with_a_message() {
    let s = start(vec![text(PLAN)], &["only-this-one"]);
    s.put("/v1/team/types/frontier", json!({"model": "big-one"})).await;
    s.put("/v1/team/types/mid-tier", json!({"model": "mid-one"})).await;
    s.put("/v1/team/departments/pets", json!({"text": "---\nname: pets\ndescription: x\nlead: planner\nmembers: researcher\n---\n"})).await;
    let (_, run) = s.post("/v1/team/runs", json!({"department": "pets", "goal": "go"})).await;
    let events = s.follow(run["id"].as_str().unwrap(), |_| None).await;
    let out = events.last().unwrap();
    assert_eq!(out["status"], "failed");
    assert!(out["report"].as_str().unwrap().contains("big-one"), "{out}");
}

fn tool_call(name: &str, args: Value) -> Response {
    Response { tool_calls: vec![lh_provider::ToolCall { id: "c1".into(), name: name.into(), arguments: args.to_string(), repaired: false }], finish: Some(FinishReason::ToolCalls), ..Default::default() }
}

#[tokio::test]
async fn the_assistant_starts_a_project_in_a_named_folder_and_workers_are_locked_to_it() {
    let proj = tempfile::tempdir().unwrap();
    let place = proj.path().join("cats-site");
    let s = with_models(start(
        vec![
            tool_call("start_project", json!({"name": "Cats site", "goal": "tell me about cats", "department": "pets", "location": place})),
            text(PLAN),
            text("Cats purr."),
            text("OK"),
            text("The site notes are ready."),
            text("Your project is ready in cats-site."),
        ],
        &[],
    ))
    .await;
    s.put("/v1/team/departments/pets", json!({"text": "---\nname: pets\ndescription: x\nlead: planner\nmembers: researcher\n---\n"})).await;
    s.put("/v1/team/settings", json!({"autonomy": "fire-and-forget"})).await;
    let (_, sess) = s.post("/v1/sessions", json!({"model": "m", "mode": "assistant"})).await;
    let id = sess["id"].as_str().unwrap().to_string();
    let mut resp = s.req(reqwest::Method::POST, &format!("/v1/sessions/{id}/turns")).json(&json!({"text": "make me a cats site"})).send().await.unwrap();
    let mut p = SseParser::new();
    let (mut asked, mut last) = (vec![], Value::Null);
    'outer: while let Some(chunk) = tokio::time::timeout(Duration::from_secs(15), resp.chunk()).await.expect("no event in 15 s").unwrap() {
        for d in p.push(&chunk) {
            let v: Value = serde_json::from_str(&d).unwrap();
            if v["type"] == "ask" {
                asked.push(v["tool"].as_str().unwrap().to_string());
                s.post(&format!("/v1/sessions/{id}/answers/{}", v["id"].as_str().unwrap()), json!({"answer": "yes"})).await;
            }
            if matches!(v["type"].as_str(), Some("done" | "error" | "refused")) {
                last = v;
                break 'outer;
            }
        }
    }
    assert_eq!(last["type"], "done", "{last}");
    assert_eq!(asked, ["start_project"], "one question: may it make the project; the plan was approved ahead by the autonomy setting");
    assert!(place.is_dir(), "the folder was made where the request said");
    let seen = s.seen.lock().unwrap();
    let names = |r: &ChatRequest| r.tools.iter().map(|t| t.name.clone()).collect::<Vec<_>>();
    assert!(names(&seen[0]).contains(&"start_project".to_string()) && names(&seen[0]).contains(&"call_department".to_string()));
    let worker = seen.iter().find(|r| r.messages.iter().any(|m| m.content.as_deref() == Some("Find three facts about cats."))).unwrap();
    assert!(!names(worker).iter().any(|n| n == "start_project" || n == "call_department"), "a worker cannot start another project: {:?}", names(worker));
    assert!(worker.messages[0].content.as_deref().unwrap().contains(&format!("{}", place.canonicalize().unwrap().display())), "the worker is told its folder");
    let tool_result = seen.last().unwrap().messages.iter().rev().find(|m| m.role == lh_provider::Role::Tool).unwrap().content.clone().unwrap();
    assert!(tool_result.starts_with("Project Cats site created in") && tool_result.contains("The site notes are ready."), "{tool_result}");
}
