//! Learning through the real local API: what shapes the prompt, how a habit becomes a rule, and the
//! controls around it (accept, edit, roll back, silent mode).
use std::sync::{Arc, Mutex};
use std::time::Duration;

use lh_core::{auth, router, AppState, LearnMode, LearnRuntime, Settings};
use lh_learn::Learn;
use lh_memory::Trust;
use lh_provider::{ChatRequest, Event, FinishReason, Response, SseParser, ToolCall};
use lh_store::Store;
use lh_tools::Model;
use serde_json::{json, Value};

struct Script {
    replies: Mutex<Vec<Response>>,
    seen: Arc<Mutex<Vec<ChatRequest>>>,
}
impl Model for Script {
    async fn complete(&self, r: &ChatRequest, on: &mut (dyn FnMut(Event) + Send)) -> Result<Response, lh_provider::Error> {
        self.seen.lock().unwrap().push(r.clone());
        let resp = self.replies.lock().unwrap().pop().expect("unscripted model call");
        if !resp.text.is_empty() {
            on(Event::Text(resp.text.clone()));
        }
        Ok(resp)
    }
}
fn text(t: &str) -> Response {
    Response { text: t.into(), finish: Some(FinishReason::Stop), ..Default::default() }
}
fn tool(name: &str, args: Value) -> Response {
    Response { tool_calls: vec![ToolCall { id: "c".into(), name: name.into(), arguments: args.to_string(), repaired: false }], finish: Some(FinishReason::ToolCalls), ..Default::default() }
}

struct Server {
    base: String,
    token: String,
    http: reqwest::Client,
    seen: Arc<Mutex<Vec<ChatRequest>>>,
    learn: Arc<Learn>,
    dir: tempfile::TempDir,
}
impl Server {
    fn post(&self, path: &str, body: Value) -> reqwest::RequestBuilder {
        self.http.post(format!("{}{path}", self.base)).bearer_auth(&self.token).json(&body)
    }
    fn get(&self, path: &str) -> reqwest::RequestBuilder {
        self.http.get(format!("{}{path}", self.base)).bearer_auth(&self.token)
    }
    async fn json(&self, rb: reqwest::RequestBuilder) -> Value {
        rb.send().await.unwrap().json().await.unwrap()
    }
    async fn session(&self, extra: Value) -> String {
        let proj = self.dir.path().join("proj");
        std::fs::create_dir_all(&proj).unwrap();
        let mut body = json!({"model": "m", "project_dir": proj});
        for (k, v) in extra.as_object().unwrap() {
            body[k] = v.clone();
        }
        self.json(self.post("/v1/sessions", body)).await["id"].as_str().unwrap().to_string()
    }
    async fn turn(&self, id: &str, msg: &str) {
        let mut resp = self.post(&format!("/v1/sessions/{id}/turns"), json!({"text": msg})).send().await.unwrap();
        let mut p = SseParser::new();
        'outer: while let Some(chunk) = tokio::time::timeout(Duration::from_secs(10), resp.chunk()).await.expect("no event in 10 s").unwrap() {
            for d in p.push(&chunk) {
                let v: Value = serde_json::from_str(&d).unwrap();
                if v["type"] == "ask" {
                    self.post(&format!("/v1/sessions/{id}/answers/{}", v["id"].as_str().unwrap()), json!({"answer": "yes"})).send().await.unwrap();
                }
                if matches!(v["type"].as_str(), Some("done" | "error" | "refused")) {
                    break 'outer;
                }
            }
        }
    }
    fn systems(&self) -> Vec<String> {
        self.seen.lock().unwrap().iter().filter_map(|r| r.messages.first().filter(|m| m.role == lh_provider::Role::System).and_then(|m| m.content.clone())).collect()
    }
}

fn start(replies: Vec<Response>, trust: Trust, mode: LearnMode) -> Server {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(Store::open(&dir.path().join("lh.db")).unwrap());
    let learn = Arc::new(Learn::new(store.clone()).with_skills_dir(dir.path().join("skills")));
    let mut r = replies;
    r.reverse();
    let seen: Arc<Mutex<Vec<ChatRequest>>> = Default::default();
    let settings = Settings {
        provider_name: None,
        learn: Some(LearnRuntime { learn: learn.clone(), trust, mode, threshold: 3, skill_min_calls: 3 }),
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
        team: None,
        mcp: None,
        ask_all: false,
        config: None,
    };
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    l.set_nonblocking(true).unwrap();
    let port = l.local_addr().unwrap().port();
    let token = auth::new_token();
    let app = router(AppState::new(Script { replies: Mutex::new(r), seen: seen.clone() }, settings), token.clone(), port);
    tokio::spawn(async move { axum::serve(tokio::net::TcpListener::from_std(l).unwrap(), app).await.unwrap() });
    Server { base: format!("http://127.0.0.1:{port}"), token, http: reqwest::Client::new(), seen, learn, dir }
}

#[tokio::test]
async fn rules_style_and_skills_follow_the_users_own_prompt_and_cloud_models_get_only_the_safe_ones() {
    for (trust, cloud) in [(Trust::Local, false), (Trust::AnyCloud, true)] {
        let s = start(vec![text("ok"), text("ok")], trust, LearnMode::Propose);
        let r = s.json(s.post("/v1/learn/rules", json!({"text": "Always add links after research.", "topic": "links"}))).await;
        assert_eq!(r["privacy"], "normal");
        let secret = s.json(s.post("/v1/learn/rules", json!({"text": "My doctor is Dr. Amal; for health questions mention her diabetes plan."}))).await;
        assert_eq!(secret["privacy"], "sensitive", "classified when saved");
        s.json(s.http.put(format!("{}/v1/learn/style", s.base)).bearer_auth(&s.token).json(&json!({"text": "Terse. Plain words."}))).await;
        let md = "---\nname: release-notes\ndescription: \"Write release notes. Use before a release.\"\n---\n\n1. Read git log.\n";
        s.json(s.post("/v1/learn/skills", json!({"files": {"SKILL.md": md}}))).await;

        let id = s.session(json!({"system": "You are the user's assistant."})).await;
        s.turn(&id, "hello").await;
        let sys = s.systems()[0].clone();
        assert!(sys.starts_with("You are m. You work in LastHarness"), "the built-in base prompt is first: {sys}");
        let at = |t: &str| sys.find(t).unwrap_or_else(|| panic!("{t:?} missing in {sys}"));
        assert!(at("Sensitive notes never go to a cloud model.") < at("You are the user's assistant.") && at("You are the user's assistant.") < at("Rules the user has set"), "base, then the user's own words untouched, then what was learned: {sys}");
        assert!(sys.contains("- Always add links after research.") && sys.contains("How to talk to this user:\nTerse. Plain words.") && sys.contains("- release-notes: Write release notes."), "{sys}");
        assert_eq!(sys.contains("Dr. Amal"), !cloud, "a sensitive rule reaches only a local model ({trust:?})");
        let tools: Vec<String> = s.seen.lock().unwrap()[0].tools.iter().map(|t| t.name.clone()).collect();
        assert!(tools.contains(&"use_skill".to_string()) && tools.contains(&"read_skill_file".to_string()));
        let applied = s.json(s.get("/v1/activity?kind=learned_applied")).await;
        assert_eq!(applied["entries"].as_array().unwrap().len(), 1, "what shaped the prompt is logged");

        // incognito still follows the rules but is never learned from
        let inc = s.session(json!({"incognito": true})).await;
        s.turn(&inc, "Always answer in French from now on").await;
        assert!(s.systems()[1].contains("Always add links"));
        assert!(s.learn.signals(None).unwrap().is_empty());
        assert_eq!(s.post(&format!("/v1/sessions/{inc}/learn"), json!({})).send().await.unwrap().status(), 400);
    }
}

#[tokio::test]
async fn a_habit_across_three_sessions_becomes_a_suggestion_the_user_accepts_edits_and_can_roll_back() {
    let habit = r#"[{"kind":"rule","topic":"bullets","text":"Do not use bullet points; write short paragraphs.","scope":"global","evidence":"never use bullet points"}]"#;
    // per session: the turn, then the session's own learn pass; the third also asks about the habit
    let s = start(vec![text("ok"), text("[]"), text("ok"), text("[]"), text("ok"), text("[]"), text(habit), text("ok"), text("ok")], Trust::Local, LearnMode::Propose);
    for (i, said) in ["Please never use bullet points", "Don't give me bullet points, ever", "no bullet points please, I said that before"].iter().enumerate() {
        let id = s.session(json!({})).await;
        s.turn(&id, said).await;
        let r = s.json(s.post(&format!("/v1/sessions/{id}/learn"), json!({}))).await;
        if i < 2 {
            assert!(r["proposals"].as_array().unwrap().is_empty(), "two sessions are not a habit yet: {r}");
        } else {
            assert_eq!(r["proposals"].as_array().unwrap().len(), 1, "{r}");
        }
    }
    assert!(s.json(s.get("/v1/learn/rules")).await["rules"].as_array().unwrap().is_empty(), "nothing is a rule before the user accepts");
    let p = s.json(s.get("/v1/learn/proposals")).await["proposals"][0].clone();
    assert_eq!((p["kind"].as_str(), p["source"].as_str()), (Some("rule"), Some("repeat")));
    assert_eq!(p["evidence"][0], "never use bullet points");
    let habits = s.json(s.get("/v1/learn/habits")).await;
    assert_eq!(habits["habits"][0]["sessions"], 3);

    // the user edits the wording and accepts
    let a = s.json(s.post(&format!("/v1/learn/proposals/{}/accept", p["id"]), json!({"edit": {"text": "Never use bullet points. Write short paragraphs."}}))).await;
    let rule_id = a["rule"]["id"].as_str().unwrap().to_string();
    assert_eq!((a["rule"]["text"].as_str(), a["rule"]["source"].as_str()), (Some("Never use bullet points. Write short paragraphs."), Some("proposal")));

    // it now shapes the next session
    let id4 = s.session(json!({})).await;
    s.turn(&id4, "hello").await;
    assert!(s.systems().last().unwrap().contains("- Never use bullet points. Write short paragraphs."));

    // edit, then roll back to the first wording; every version is in the history
    s.json(s.http.patch(format!("{}/v1/learn/rules/{rule_id}", s.base)).bearer_auth(&s.token).json(&json!({"text": "Bullets are fine now."}))).await;
    let back = s.json(s.post(&format!("/v1/learn/rules/{rule_id}/rollback"), json!({"version": 1}))).await;
    assert_eq!((back["text"].as_str(), back["version"].as_i64()), (Some("Never use bullet points. Write short paragraphs."), Some(3)));
    let h = s.json(s.get(&format!("/v1/learn/rules/{rule_id}"))).await;
    assert_eq!(h["history"].as_array().unwrap().iter().map(|x| x["op"].as_str().unwrap()).collect::<Vec<_>>(), ["create", "edit", "rollback"]);

    // turn it off: the next session does not get it
    s.json(s.post(&format!("/v1/learn/rules/{rule_id}/status"), json!({"active": false}))).await;
    let id5 = s.session(json!({})).await;
    s.turn(&id5, "hello again").await;
    let last = s.seen.lock().unwrap().last().unwrap().clone();
    assert!(last.messages.first().is_none_or(|m| m.role != lh_provider::Role::System || !m.content.as_deref().unwrap_or("").contains("bullet")), "a disabled rule is not sent");
    // and the same habit is not put to the model a second time
    assert!(s.json(s.post(&format!("/v1/sessions/{id5}/learn"), json!({}))).await["proposals"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn silent_learning_applies_at_once_and_it_can_all_be_undone() {
    let reply = r#"[{"kind":"rule","topic":"links","text":"Always add source links after research.","evidence":"give me links"},{"kind":"style","text":"Be terse."}]"#;
    let s = start(vec![text("ok"), text(reply)], Trust::Local, LearnMode::Silent);
    let id = s.session(json!({})).await;
    s.turn(&id, "Always give me links after you research").await;
    let r = s.json(s.post(&format!("/v1/sessions/{id}/learn"), json!({}))).await;
    assert_eq!(r["applied"].as_array().unwrap().len(), 2, "{r}");
    let rules = s.json(s.get("/v1/learn/rules")).await["rules"].clone();
    assert_eq!(rules[0]["text"], "Always add source links after research.");
    assert_eq!(s.json(s.get("/v1/learn/style")).await["style"]["text"], "Be terse.");
    let h = s.json(s.get(&format!("/v1/learn/rules/{}", rules[0]["id"].as_str().unwrap()))).await;
    assert_eq!(h["history"][0]["by"], "silent learning", "silent changes are marked as such");
    let log = s.json(s.get("/v1/activity?kind=learned")).await;
    assert!(log["entries"].as_array().unwrap().len() >= 2);
    // undo: delete the rule, roll the style profile back to nothing is not possible, but it can be edited away
    assert_eq!(s.http.delete(format!("{}/v1/learn/rules/{}", s.base, rules[0]["id"].as_str().unwrap())).bearer_auth(&s.token).send().await.unwrap().status(), 204);
    assert!(s.json(s.get("/v1/learn/rules?all=true")).await["rules"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn a_long_task_can_become_a_skill_and_a_skill_in_use_can_be_improved() {
    let skill = r#"{"worth":true,"name":"count-files","description":"Count the files in a folder. Use when asked how many files there are.","body":"1. List the folder.\n2. Count the entries."}"#;
    let change = r#"{"change":true,"body":"1. List the folder.\n2. Count the entries, skipping hidden ones.","note":"hidden files should not count"}"#;
    let s = start(
        vec![
            tool("list_dir", json!({"path": "."})),
            tool("list_dir", json!({"path": "."})),
            tool("list_dir", json!({"path": "."})),
            text("There are 3 files."),
            text(skill),
            // second session: the skill is read, then used
            tool("use_skill", json!({"name": "count-files"})),
            text("Done."),
            text(change),
        ],
        Trust::Local,
        LearnMode::Propose,
    );
    let id = s.session(json!({})).await;
    s.turn(&id, "how many files are in this folder?").await;
    let r = s.json(s.post(&format!("/v1/sessions/{id}/learn"), json!({}))).await;
    let p = &r["proposals"][0];
    assert_eq!((p["kind"].as_str(), p["payload"]["name"].as_str(), p["source"].as_str()), (Some("skill_new"), Some("count-files"), Some("skill")), "{r}");
    assert!(s.json(s.get("/v1/learn/skills")).await["skills"].as_array().unwrap().is_empty(), "not a skill before it is accepted");
    s.json(s.post(&format!("/v1/learn/proposals/{}/accept", p["id"]), json!({}))).await;
    let skills = s.json(s.get("/v1/learn/skills")).await;
    assert_eq!(skills["skills"][0]["name"], "count-files");
    assert!(s.dir.path().join("skills/count-files/SKILL.md").exists());

    let id2 = s.session(json!({})).await;
    s.turn(&id2, "count the files again").await;
    assert!(s.systems().last().unwrap().contains("- count-files: Count the files in a folder."));
    let r = s.json(s.post(&format!("/v1/sessions/{id2}/learn"), json!({}))).await;
    let p = &r["proposals"][0];
    assert_eq!((p["kind"].as_str(), p["payload"]["note"].as_str()), (Some("skill_update"), Some("hidden files should not count")), "{r}");
    s.json(s.post(&format!("/v1/learn/proposals/{}/accept", p["id"]), json!({}))).await;
    let sk = s.json(s.get("/v1/learn/skills/count-files")).await;
    assert!(sk["body"].as_str().unwrap().contains("skipping hidden ones") && sk["description"].as_str().unwrap().starts_with("Count the files"));
    assert_eq!(sk["history"].as_array().unwrap().len(), 2);
    let back = s.json(s.post("/v1/learn/skills/count-files/rollback", json!({"version": 1}))).await;
    assert_eq!(back["name"], "count-files");
    assert!(!s.json(s.get("/v1/learn/skills/count-files")).await["body"].as_str().unwrap().contains("hidden"));
}

#[tokio::test]
async fn the_controls_style_feedback_errors_and_switching_off() {
    let s = start(vec![], Trust::Local, LearnMode::Propose);
    // style
    assert!(s.json(s.get("/v1/learn/style")).await["style"].is_null());
    s.json(s.http.put(format!("{}/v1/learn/style", s.base)).bearer_auth(&s.token).json(&json!({"text": "Terse."}))).await;
    s.json(s.http.put(format!("{}/v1/learn/style", s.base)).bearer_auth(&s.token).json(&json!({"text": "Chatty."}))).await;
    assert_eq!(s.json(s.post("/v1/learn/style/rollback", json!({"version": 1}))).await["text"], "Terse.");
    let long = "x".repeat(1600);
    assert_eq!(s.http.put(format!("{}/v1/learn/style", s.base)).bearer_auth(&s.token).json(&json!({"text": long})).send().await.unwrap().status(), 400);
    // rules: bad input and unknown ids
    assert_eq!(s.post("/v1/learn/rules", json!({"text": "  "})).send().await.unwrap().status(), 400);
    assert_eq!(s.get("/v1/learn/rules/r_nope").send().await.unwrap().status(), 404);
    assert_eq!(s.post("/v1/learn/proposals/99/accept", json!({})).send().await.unwrap().status(), 404);
    // a bad skill is refused with a reason
    let bad = s.post("/v1/learn/skills", json!({"files": {"SKILL.md": "no header"}})).send().await.unwrap();
    assert_eq!(bad.status(), 400);
    assert!(bad.text().await.unwrap().contains("header"));
    // feedback: a thumbs down with words is a correction that can become a rule; without words it is only counted
    let id = s.session(json!({})).await;
    assert_eq!(s.post(&format!("/v1/sessions/{id}/feedback"), json!({"kind": "down", "note": "too long, keep it to one line"})).send().await.unwrap().status(), 204);
    assert_eq!(s.post(&format!("/v1/sessions/{id}/feedback"), json!({"kind": "up"})).send().await.unwrap().status(), 204);
    assert_eq!(s.post(&format!("/v1/sessions/{id}/feedback"), json!({"kind": "sideways"})).send().await.unwrap().status(), 400);
    let kinds: Vec<String> = s.json(s.get("/v1/learn/signals")).await["signals"].as_array().unwrap().iter().map(|x| x["kind"].as_str().unwrap().to_string()).collect();
    assert_eq!(kinds, ["correction", "thumbs_up"]);
    assert!(s.json(s.get("/v1/learn/habits?min=1")).await["habits"].as_array().unwrap().len() == 1, "a bare thumbs up has no words, so it never forms a habit");
}
