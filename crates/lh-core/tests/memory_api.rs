//! Memory through the real local API: what reaches the model, what stays out, and what survives a restart.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use lh_core::{auth, router, AppState, MemoryRuntime, Settings};
use lh_memory::embed::AnyEmbedder;
use lh_memory::{By, Edit, Layer, Memory, NewPage, Privacy, RecallConfig, State, Trust};
use lh_provider::{ChatRequest, Event, FinishReason, Response, SseParser};
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

struct Server {
    base: String,
    token: String,
    http: reqwest::Client,
    seen: Arc<Mutex<Vec<ChatRequest>>>,
    mem: Arc<Memory>,
    store: Arc<Store>,
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
        let mut body = json!({"model": "m", "mode": "chat"});
        for (k, v) in extra.as_object().unwrap() {
            body[k] = v.clone();
        }
        self.json(self.post("/v1/sessions", body)).await["id"].as_str().unwrap().to_string()
    }
    /// Run one turn and return the events.
    async fn turn(&self, id: &str, msg: &str) -> Vec<Value> {
        let mut resp = self.post(&format!("/v1/sessions/{id}/turns"), json!({"text": msg})).send().await.unwrap();
        let (mut p, mut seen) = (SseParser::new(), vec![]);
        'outer: while let Some(chunk) = tokio::time::timeout(Duration::from_secs(10), resp.chunk()).await.expect("no event in 10 s").unwrap() {
            for d in p.push(&chunk) {
                let v: Value = serde_json::from_str(&d).unwrap();
                let end = matches!(v["type"].as_str(), Some("done" | "error" | "refused"));
                seen.push(v);
                if end {
                    break 'outer;
                }
            }
        }
        seen
    }
    fn all_text_sent(&self) -> String {
        self.seen.lock().unwrap().iter().map(|r| format!("{:?} {:?}", r.messages, r.tools)).collect::<Vec<_>>().join("\n")
    }
}

fn start_at(dir: &std::path::Path, replies: Vec<Response>, trust: Trust, project_dir_free: bool) -> Server {
    let _ = project_dir_free;
    let store = Arc::new(Store::open(&dir.join("lh.db")).unwrap());
    let mem = Arc::new(Memory::open(store.clone(), dir.join("memory")).unwrap());
    let mut r = replies;
    r.reverse();
    let seen: Arc<Mutex<Vec<ChatRequest>>> = Default::default();
    let model = Script { replies: Mutex::new(r), seen: seen.clone() };
    let spend = lh_spend::SpendMeter::new(store.clone(), lh_spend::Config::default(), "test", true);
    let settings = Settings {
        provider_name: None,
        learn: None,
        maintain_when_idle: false,
        memory: Some(MemoryRuntime { memory: mem.clone(), embedder: Arc::new(AnyEmbedder::Off), trust, cfg: RecallConfig::default() }),
        image_model: None,
        web: Default::default(),
        idle_after: Duration::ZERO,
        compact_at: Some(0),
        store: store.clone(),
        spend,
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
    let app = router(AppState::new(model, settings), token.clone(), port);
    tokio::spawn(async move { axum::serve(tokio::net::TcpListener::from_std(l).unwrap(), app).await.unwrap() });
    Server { base: format!("http://127.0.0.1:{port}"), token, http: reqwest::Client::new(), seen, mem, store }
}
fn add(m: &Memory, layer: Layer, title: &str, body: &str, gist: &str, p: Privacy) {
    let mut n = NewPage::new(layer, title, body);
    n.gist = gist.into();
    n.privacy = p;
    m.create(n, By::User).unwrap();
}

#[tokio::test]
async fn about_you_and_matching_notes_reach_the_model_once_and_what_was_sent_is_recorded() {
    let d = tempfile::tempdir().unwrap();
    let s = start_at(d.path(), vec![text("Berlin is lovely."), text("Yes.")], Trust::Local, true);
    add(&s.mem, Layer::Preferences, "Style", "Short answers, plain words.", "Prefers plain words", Privacy::Normal);
    add(&s.mem, Layer::Facts, "Trip", "Flight to Berlin on 14 June.", "Flying to Berlin in June", Privacy::Normal);
    let id = s.session(json!({"system": "Be helpful."})).await;
    s.turn(&id, "tell me about Berlin").await;
    s.turn(&id, "Berlin again please").await;

    let seen = s.seen.lock().unwrap().clone();
    let sys = seen[0].messages[0].content.clone().unwrap();
    assert!(sys.starts_with("You are m. Last AI, made by Last, is the app you run in;") && sys.contains("Be helpful.") && sys.contains("What you know about the user") && sys.contains("- Prefers plain words"), "{sys}");
    let first_user = seen[0].messages.last().unwrap().content.clone().unwrap();
    assert!(first_user.contains("Flight to Berlin on 14 June") && first_user.ends_with("tell me about Berlin"), "{first_user}");
    let second_user = seen[1].messages.last().unwrap().content.clone().unwrap();
    assert_eq!(second_user, "Berlin again please", "a note already in the conversation is not added twice");
    assert_eq!(seen[1].messages[0], seen[0].messages[0], "the start of the prompt is byte-identical, so a server can keep its cache");

    // the conversation is stored as said, without the notes
    let msgs = s.mem.messages(&id).unwrap();
    assert_eq!(msgs[0], ("user".to_string(), "tell me about Berlin".to_string()));
    assert_eq!(msgs[1].1, "Berlin is lovely.");
    // and the record of what was sent names the pages
    let sent = s.json(s.get("/v1/memory/sent")).await["sent"].as_array().unwrap().clone();
    let withs: Vec<_> = sent.iter().map(|e| e["detail"]["with"].as_str().unwrap().to_string()).collect();
    assert_eq!(withs, ["recall", "about_you"], "newest first");
    assert_eq!(sent[0]["detail"]["pages"].as_array().unwrap().len(), 1);
    assert_eq!(sent[1]["detail"]["pages"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn a_request_to_a_cloud_model_provably_excludes_sensitive_memory() {
    for (trust, cloud) in [(Trust::AnyCloud, true), (Trust::OwnCloud, true), (Trust::Local, false)] {
        let d = tempfile::tempdir().unwrap();
        let s = start_at(d.path(), vec![text("ok"), text("ok")], trust, true);
        add(&s.mem, Layer::Facts, "Pay", "Salary is 4200 euros a month.", "Earns 4200 a month", Privacy::Sensitive);
        add(&s.mem, Layer::Facts, "Insulin", "Takes insulin with breakfast.", "Takes insulin", Privacy::Sensitive);
        add(&s.mem, Layer::Facts, "Hobby", "Plays chess and talks about salary of chess pros.", "Plays chess", Privacy::Normal);
        let id = s.session(json!({})).await;
        s.turn(&id, "what is my salary and what do I take for breakfast?").await;
        s.turn(&id, "and the chess hobby").await;
        let sent = s.all_text_sent();
        let leaked = ["4200", "Earns 4200", "insulin", "Insulin"].iter().any(|w| sent.contains(w));
        assert_eq!(leaked, !cloud, "trust {trust:?}: {sent}");
        assert!(sent.contains("Plays chess"), "ordinary memory is still used ({trust:?})");
        let log = serde_json::to_string(&s.json(s.get("/v1/memory/sent")).await).unwrap();
        let ids: Vec<String> = s.mem.list(None, State::Live).unwrap().into_iter().filter(|p| p.privacy == Privacy::Sensitive).map(|p| p.id).collect();
        assert_eq!(ids.iter().any(|i| log.contains(i)), !cloud, "the sent-record never names a sensitive page for a cloud destination");
    }
}

#[tokio::test]
async fn incognito_sessions_neither_read_nor_write_memory() {
    let d = tempfile::tempdir().unwrap();
    let s = start_at(d.path(), vec![text("hello")], Trust::Local, true);
    add(&s.mem, Layer::Facts, "Trip", "Flight to Berlin on 14 June.", "Flying to Berlin", Privacy::Normal);
    let id = s.session(json!({"incognito": true})).await;
    s.turn(&id, "tell me about Berlin").await;
    let seen = s.seen.lock().unwrap()[0].clone();
    let all = format!("{:?}", seen.messages);
    assert!(!all.contains("What you know") && !all.contains("Flight to Berlin"), "{all}");
    assert_eq!(seen.messages.len(), 2, "the base prompt and the user's message, nothing from memory");
    assert!(s.mem.search_conversations("Berlin", 5).unwrap().is_empty(), "nothing stored");
    assert!(s.mem.messages(&id).unwrap().is_empty());
    assert!(s.json(s.get("/v1/memory/sent")).await["sent"].as_array().unwrap().is_empty());
    let r = s.post(&format!("/v1/sessions/{id}/review"), json!({})).send().await.unwrap();
    assert_eq!(r.status(), 400, "nothing to learn from an incognito session");
}

#[tokio::test]
async fn the_review_suggests_you_accept_and_a_restart_still_remembers() {
    let d = tempfile::tempdir().unwrap();
    let review = r#"Here you go: [
      {"action":"new","layer":"facts","title":"Home city","text":"The user lives in Berlin.","gist":"Lives in Berlin","evidence":"I live in Berlin"},
      {"action":"new","layer":"facts","title":"Wifi","text":"The wifi password is hunter2.","gist":"Wifi password","evidence":"my wifi password"},
      {"action":"update","title":"x","text":"y","target":"p_not_a_real_note"}]"#;
    let s = start_at(d.path(), vec![text("Nice city."), text(review), text("NORMAL: just a city")], Trust::Local, true);
    let id = s.session(json!({})).await;
    s.turn(&id, "I live in Berlin and my wifi password is hunter2").await;
    let r = s.json(s.post(&format!("/v1/sessions/{id}/review"), json!({}))).await;
    let props = r["proposals"].as_array().unwrap();
    assert_eq!((props.len(), r["skipped"].as_i64()), (2, Some(1)), "{r}");
    assert_eq!(props[0]["privacy"], "normal");
    assert_eq!(props[1]["privacy"], "sensitive", "the password was caught: {}", props[1]["privacy_reason"]);
    assert!(s.mem.list(None, State::Live).unwrap().is_empty(), "nothing is remembered before the user accepts");

    let page = s.json(s.post(&format!("/v1/memory/proposals/{}/accept", props[0]["id"]), json!({"gist": "Lives in Berlin (edited)"}))).await;
    assert_eq!((page["title"].as_str(), page["gist"].as_str()), (Some("Home city"), Some("Lives in Berlin (edited)")));
    s.post(&format!("/v1/memory/proposals/{}/accept", props[1]["id"]), json!({})).send().await.unwrap();
    assert_eq!(s.json(s.get("/v1/memory/proposals")).await["proposals"].as_array().unwrap().len(), 0);
    drop(s);

    // restart: a new core on the same files
    let s2 = start_at(d.path(), vec![text("Hi again."), text("ok")], Trust::Local, true);
    let id2 = s2.session(json!({})).await;
    s2.turn(&id2, "hello").await;
    let sys = s2.seen.lock().unwrap()[0].messages[0].content.clone().unwrap();
    assert!(sys.contains("Lives in Berlin (edited)") && sys.contains("Wifi password"), "{sys}");
    drop(s2);

    // a cloud core on the same files gets the ordinary fact and not the password note
    let s3 = start_at(d.path(), vec![text("Hi.")], Trust::AnyCloud, true);
    let id3 = s3.session(json!({})).await;
    s3.turn(&id3, "hello").await;
    let sys = s3.seen.lock().unwrap()[0].messages[0].content.clone().unwrap();
    assert!(sys.contains("Lives in Berlin (edited)") && !sys.contains("Wifi") && !sys.contains("hunter2"), "{sys}");
    assert!(s3.store.recent(50, Some("memory_review")).unwrap().len() == 1);
}

#[tokio::test]
async fn the_viewer_endpoints_search_edit_archive_restore_delete_and_pick_up_hand_edits() {
    let d = tempfile::tempdir().unwrap();
    let s = start_at(d.path(), vec![], Trust::Local, true);
    let p = s.json(s.post("/v1/memory/pages", json!({"layer":"facts","title":"Trip","text":"Flight to Berlin on 14 June."}))).await;
    let id = p["id"].as_str().unwrap().to_string();
    assert_eq!(p["privacy"], "normal");
    let secret = s.json(s.post("/v1/memory/pages", json!({"layer":"facts","title":"Card","text":"Card number 4111 1111 1111 1111"}))).await;
    assert_eq!(secret["privacy"], "sensitive", "classified when saved");
    let found = s.json(s.get("/v1/memory/search?q=flights")).await;
    assert_eq!(found["results"][0]["id"], id.as_str());

    let e = s.json(s.http.patch(format!("{}/v1/memory/pages/{id}", s.base)).bearer_auth(&s.token).json(&json!({"text":"Flight to Paris on 3 July."}))).await;
    assert_eq!(e["version"], 2);
    let g = s.json(s.get(&format!("/v1/memory/pages/{id}"))).await;
    assert_eq!(g["history"].as_array().unwrap().len(), 2);
    assert_eq!(s.json(s.post(&format!("/v1/memory/pages/{id}/pin"), json!({"pinned": true}))).await["pinned"], true);
    assert_eq!(s.json(s.post(&format!("/v1/memory/pages/{id}/privacy"), json!({"privacy":"sensitive","reason":"mine"}))).await["privacy"], "sensitive");
    assert_eq!(s.json(s.post(&format!("/v1/memory/pages/{id}/archive"), json!({}))).await["archived"], true);
    assert!(s.json(s.get("/v1/memory/search?q=Paris")).await["results"].as_array().unwrap().is_empty());
    assert_eq!(s.json(s.get("/v1/memory/search?q=Paris&state=archived")).await["results"].as_array().unwrap().len(), 1);
    assert_eq!(s.json(s.post(&format!("/v1/memory/pages/{id}/restore"), json!({}))).await["archived"], false);

    // hand edit on disk, then scan
    let file = d.path().join("memory/facts/trip.md");
    std::fs::write(&file, std::fs::read_to_string(&file).unwrap().replace("Paris on 3 July", "Rome on 9 August")).unwrap();
    let scan = s.json(s.post("/v1/memory/scan", json!({}))).await;
    assert_eq!(scan["edited"].as_array().unwrap().len(), 1);
    assert!(s.json(s.get(&format!("/v1/memory/pages/{id}"))).await["page"]["body"].as_str().unwrap().contains("Rome"));

    // export, then import into a fresh install
    let out = d.path().join("export");
    assert_eq!(s.post("/v1/memory/export", json!({"path": out})).send().await.unwrap().status(), 204);
    let d2 = tempfile::tempdir().unwrap();
    let s2 = start_at(d2.path(), vec![], Trust::Local, true);
    let imp = s2.json(s2.post("/v1/memory/import", json!({"path": out}))).await;
    assert_eq!(imp["imported"], 2);
    assert_eq!(s2.json(s2.get("/v1/memory/search?q=Rome")).await["results"].as_array().unwrap().len(), 1);
    assert_eq!(s.post("/v1/memory/export", json!({"path": "relative/dir"})).send().await.unwrap().status(), 400);

    assert_eq!(s.http.delete(format!("{}/v1/memory/pages/{id}", s.base)).bearer_auth(&s.token).send().await.unwrap().status(), 204);
    assert_eq!(s.get(&format!("/v1/memory/pages/{id}")).send().await.unwrap().status(), 404);
    let m = s.json(s.post("/v1/memory/maintenance", json!({}))).await;
    assert!(m["findings"].is_array());
}

#[tokio::test]
async fn a_local_model_can_flag_what_patterns_miss_and_unsure_means_sensitive() {
    let review = r#"[{"action":"new","layer":"facts","title":"Mood","text":"Gets very anxious before flights.","gist":"Flight anxiety","evidence":"I hate flying"},
        {"action":"new","layer":"facts","title":"Pet","text":"Has a cat called Nour.","gist":"Cat called Nour","evidence":"my cat Nour"}]"#;
    let d = tempfile::tempdir().unwrap();
    let s = start_at(d.path(), vec![text("ok"), text(review), text("SENSITIVE: mental health"), text("maybe?")], Trust::Local, true);
    let id = s.session(json!({})).await;
    s.turn(&id, "I hate flying and my cat Nour hates it more").await;
    let r = s.json(s.post(&format!("/v1/sessions/{id}/review"), json!({}))).await;
    let props = r["proposals"].as_array().unwrap();
    assert_eq!((props[0]["privacy"].as_str(), props[0]["privacy_reason"].as_str()), (Some("sensitive"), Some("judged sensitive: mental health")));
    assert_eq!((props[1]["privacy"].as_str(), props[1]["privacy_reason"].as_str()), (Some("sensitive"), Some("the model was unsure")));
    // the judge saw only the note, on the local model
    let seen = s.seen.lock().unwrap();
    assert!(seen[2].messages[0].content.as_deref().unwrap().contains("Gets very anxious") && seen.len() == 4);
}

#[tokio::test]
async fn adding_a_note_through_the_api_saves_it_and_it_is_found_by_search() {
    let d = tempfile::tempdir().unwrap();
    let s = start_at(d.path(), vec![], Trust::Local, true);
    let before = s.json(s.get("/v1/memory/pages")).await["pages"].as_array().unwrap().len();
    let p = s.json(s.post("/v1/memory/pages", json!({"layer":"preferences","title":"Editor","text":"Uses Helix for all code."}))).await;
    assert_eq!(p["layer"], "preferences");
    assert_eq!(p["title"], "Editor");
    assert_eq!(p["privacy"], "normal");
    let after = s.json(s.get("/v1/memory/pages")).await["pages"].as_array().unwrap().len();
    assert_eq!(after, before + 1, "the note is in the list");
    let found = s.json(s.get("/v1/memory/search?q=Helix")).await;
    assert_eq!(found["results"][0]["id"], p["id"]);
    assert_eq!(s.post("/v1/memory/pages", json!({"layer":"nowhere","title":"x","text":"y"})).send().await.unwrap().status(), 400);
}

#[tokio::test]
async fn tidy_returns_a_rewrite_that_is_saved_only_when_the_user_accepts() {
    let reply = r#"Sure: {"title": "Cat", "text": "The cat sleeps. Key is sk-abcdefghijklmnop."}"#;
    let d = tempfile::tempdir().unwrap();
    let s = start_at(d.path(), vec![text(reply)], Trust::Local, true);
    let t = s.json(s.post("/v1/memory/tidy", json!({"text":"the cat sleep. key sk-abcdefghijklmnop","model":"m"}))).await;
    assert_eq!(t["title"], "Cat");
    assert_eq!(t["redacted"], true, "a secret in the rewrite is screened out");
    assert!(!t["text"].as_str().unwrap().contains("sk-abcdef"));
    // tidy saves nothing
    assert!(s.json(s.get("/v1/memory/pages")).await["pages"].as_array().unwrap().is_empty());
    // accepting saves the screened text as a new note
    let p = s.json(s.post("/v1/memory/pages", json!({"layer":"facts","title":t["title"],"text":t["text"]}))).await;
    assert_eq!(p["title"], "Cat");
    assert!(!p["body"].as_str().unwrap().contains("sk-abcdef"));
    assert_eq!(s.json(s.get("/v1/memory/pages")).await["pages"].as_array().unwrap().len(), 1);
    // empty note and missing model are refused before any model call
    assert_eq!(s.post("/v1/memory/tidy", json!({"text":"  ","model":"m"})).send().await.unwrap().status(), 400);
    assert_eq!(s.post("/v1/memory/tidy", json!({"text":"hi"})).send().await.unwrap().status(), 400);
}

/// The Memory list shows the assistant's note writes with their chat.
#[tokio::test]
async fn the_assistant_write_log_is_listed_with_its_chat() {
    let d = tempfile::tempdir().unwrap();
    let s = start_at(d.path(), vec![], Trust::Local, true);
    let rt = s.mem.clone();
    rt.log_write("add", "p1", "Tea", Some("chat-7")).unwrap();
    let r = s.json(s.get("/v1/memory/writes")).await;
    assert_eq!(r["writes"][0]["action"], "add");
    assert_eq!(r["writes"][0]["chat"], "chat-7");
    assert_eq!(r["writes"][0]["title"], "Tea");
}

/// Undo of an assistant write: an added note is archived, an updated note goes back to its earlier text.
#[tokio::test]
async fn undoing_an_assistant_write_archives_an_add_and_restores_the_earlier_text() {
    let d = tempfile::tempdir().unwrap();
    let s = start_at(d.path(), vec![], Trust::Local, true);
    let m = s.mem.clone();
    let mut n = NewPage::new(Layer::Facts, "Tea", "Likes green tea");
    n.gist = "tea".into();
    let p = m.create(n, By::User).unwrap();
    m.update(&p.id, Edit { title: None, body: Some("Likes black tea".into()), gist: None }, By::Assistant).unwrap();
    m.log_write("update", &p.id, "Tea", Some("chat-1")).unwrap();
    let upd = s.json(s.get("/v1/memory/writes")).await["writes"][0]["id"].as_i64().unwrap();
    let r = s.json(s.post(&format!("/v1/memory/writes/{upd}/undo"), json!({}))).await;
    assert_eq!(r["body"], "Likes green tea", "{r}");

    let mut n2 = NewPage::new(Layer::Facts, "Coffee", "Drinks one cup");
    n2.gist = "coffee".into();
    let c = m.create(n2, By::Assistant).unwrap();
    m.log_write("add", &c.id, "Coffee", Some("chat-2")).unwrap();
    let add = s.json(s.get("/v1/memory/writes")).await["writes"][0]["id"].as_i64().unwrap();
    let a = s.json(s.post(&format!("/v1/memory/writes/{add}/undo"), json!({}))).await;
    assert_eq!(a["archived"], true, "{a}");
}

/// A git repo in `dir` with one committed file, then `edit` applied to it (or not).
fn repo_with_edit(dir: &std::path::Path, edit: bool) {
    let git = |args: &[&str]| assert!(std::process::Command::new("git").arg("-C").arg(dir).args(args).status().unwrap().success());
    git(&["init", "-q"]);
    git(&["config", "user.email", "t@example.com"]);
    git(&["config", "user.name", "t"]);
    std::fs::write(dir.join("notes.txt"), "old line\n").unwrap();
    git(&["add", "notes.txt"]);
    git(&["commit", "-q", "-m", "init"]);
    if edit {
        std::fs::write(dir.join("notes.txt"), "new line about the release\n").unwrap();
    }
}

#[tokio::test]
async fn the_review_reads_the_git_diff_when_there_is_one_and_the_chat_otherwise() {
    let d = tempfile::tempdir().unwrap();
    let proj = d.path().join("proj");
    std::fs::create_dir(&proj).unwrap();
    repo_with_edit(&proj, true);
    let s = start_at(d.path(), vec![text("ok"), text("[]")], Trust::Local, true);
    let id = s.session(json!({"project_dir": proj.to_str().unwrap()})).await;
    s.turn(&id, "hello there").await;
    let r = s.json(s.post(&format!("/v1/sessions/{id}/review"), json!({}))).await;
    assert_eq!(r["source"], "diff", "{r}");
    let sent = format!("{:?}", s.seen.lock().unwrap().last().unwrap().messages);
    assert!(sent.contains("new line about the release") && sent.contains("old line"), "the diff was sent: {sent}");
}

#[tokio::test]
async fn the_review_falls_back_to_the_chat_when_the_diff_is_empty() {
    let d = tempfile::tempdir().unwrap();
    let proj = d.path().join("proj");
    std::fs::create_dir(&proj).unwrap();
    repo_with_edit(&proj, false);
    let s = start_at(d.path(), vec![text("ok"), text("[]")], Trust::Local, true);
    let id = s.session(json!({"project_dir": proj.to_str().unwrap()})).await;
    s.turn(&id, "hello there").await;
    let r = s.json(s.post(&format!("/v1/sessions/{id}/review"), json!({"source": "auto"}))).await;
    assert_eq!(r["source"], "transcript", "{r}");
    let sent = format!("{:?}", s.seen.lock().unwrap().last().unwrap().messages);
    assert!(sent.contains("hello there") && !sent.contains("```diff"), "{sent}");
}

#[tokio::test]
async fn the_review_transcript_source_is_forced_even_with_a_diff() {
    let d = tempfile::tempdir().unwrap();
    let proj = d.path().join("proj");
    std::fs::create_dir(&proj).unwrap();
    repo_with_edit(&proj, true);
    let s = start_at(d.path(), vec![text("ok"), text("[]")], Trust::Local, true);
    let id = s.session(json!({"project_dir": proj.to_str().unwrap()})).await;
    s.turn(&id, "hello there").await;
    let r = s.json(s.post(&format!("/v1/sessions/{id}/review"), json!({"source": "transcript"}))).await;
    assert_eq!(r["source"], "transcript", "{r}");
    let sent = format!("{:?}", s.seen.lock().unwrap().last().unwrap().messages);
    assert!(sent.contains("hello there") && !sent.contains("```diff"), "{sent}");
}
