//! The link to the core, against a real `lhd` router with a scripted model: refresh, send and stream, stop,
//! delete, resume with a warning, and following a department run to its end.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use lh_core::{auth, router, AppState, MemoryRuntime, Settings, TeamRuntime};
use lh_gui::api::{futures_channel, Backend, Cmd, Ev};
use lh_gui::config::Config;
use lh_gui::runs::{Outcome, Run};
use lh_memory::embed::AnyEmbedder;
use lh_memory::{Memory, RecallConfig, Trust};
use lh_provider::{ChatRequest, Event, FinishReason, Response};
use lh_store::Store;
use lh_tools::Model;
use serde_json::{json, Value};

struct Script {
    replies: Mutex<Vec<Response>>,
    models: Vec<String>,
    /// Hold each reply this long after its first words, so a turn can be stopped in the middle.
    slow: Duration,
}
impl Model for Script {
    async fn complete(&self, _r: &ChatRequest, on: &mut (dyn FnMut(Event) + Send)) -> Result<Response, lh_provider::Error> {
        let resp = self.replies.lock().unwrap().pop().expect("unscripted model call");
        if !resp.text.is_empty() {
            let (a, b) = resp.text.split_at(resp.text.len() / 2);
            on(Event::Text(a.into()));
            if !self.slow.is_zero() {
                tokio::time::sleep(self.slow).await;
            }
            on(Event::Text(b.into()));
        }
        Ok(resp)
    }
    async fn models(&self) -> Result<Vec<String>, lh_provider::Error> {
        Ok(self.models.clone())
    }
}
fn text(t: &str) -> Response {
    Response { text: t.into(), finish: Some(FinishReason::Stop), ..Default::default() }
}

struct Core {
    cfg: Config,
    dir: std::path::PathBuf,
    _keep: Option<tempfile::TempDir>,
}

fn start(dir: &std::path::Path, replies: Vec<Response>, slow: Duration, team: bool) -> Core {
    let mut replies = replies;
    replies.reverse();
    let store = Arc::new(Store::open(&dir.join("lh.db")).unwrap());
    let mem = Arc::new(Memory::open(store.clone(), dir.join("memory")).unwrap());
    let learn = Arc::new(lh_learn::Learn::new(store.clone()));
    let team = team.then(|| TeamRuntime::open(&dir.join("team"), learn, 1000).unwrap());
    let settings = Settings {
        team,
        mcp: None,
        ask_all: false,
        config: None,
        provider_name: None,
        learn: None,
        maintain_when_idle: false,
        memory: Some(MemoryRuntime { memory: mem, embedder: Arc::new(AnyEmbedder::Off), trust: Trust::Local, cfg: RecallConfig::default() }),
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
    let model = Script { replies: Mutex::new(replies), models: ["alpha-balanced", "alpha-max", "beta", "big", "mid"].map(String::from).to_vec(), slow };
    let app = router(AppState::new(model, settings), token.clone(), port);
    tokio::spawn(async move { axum::serve(tokio::net::TcpListener::from_std(l).unwrap(), app).await.unwrap() });
    std::fs::write(dir.join("token"), &token).unwrap();
    let cfg = Config { base: format!("http://127.0.0.1:{port}"), token_file: dir.join("token"), prefs: dir.join("gui.toml"), themes: dir.join("themes") };
    Core { cfg, dir: dir.to_path_buf(), _keep: None }
}

fn fresh(replies: Vec<Response>, slow: Duration, team: bool) -> Core {
    let d = tempfile::tempdir().unwrap();
    let mut c = start(d.path(), replies, slow, team);
    c._keep = Some(d);
    c
}

struct Window {
    backend: Backend,
    rx: futures_channel::UnboundedReceiver<Ev>,
}
impl Window {
    fn open(cfg: &Config) -> Window {
        let (tx, rx) = futures_channel::unbounded();
        Window { backend: Backend::start(cfg.clone(), tx), rx }
    }
    async fn next(&mut self) -> Ev {
        tokio::time::timeout(Duration::from_secs(15), self.rx.recv()).await.expect("no event in 15 s")
    }
    /// Events until one matches; the others are returned too.
    async fn until(&mut self, mut hit: impl FnMut(&Ev) -> bool) -> Vec<Ev> {
        let mut seen = vec![];
        loop {
            let e = self.next().await;
            let done = hit(&e);
            seen.push(e);
            if done {
                return seen;
            }
        }
    }
}

fn joined_text(evs: &[Ev]) -> String {
    evs.iter().filter_map(|e| match e {
        Ev::Server { event: lh_client::ServerEvent::Text(t), .. } => Some(t.as_str()),
        _ => None,
    }).collect()
}

#[tokio::test]
async fn it_connects_lists_the_models_and_streams_an_answer() {
    let core = fresh(vec![text("Hello there")], Duration::ZERO, false);
    let mut w = Window::open(&core.cfg);
    w.backend.send(Cmd::Refresh);
    let evs = w.until(|e| matches!(e, Ev::Chats(_))).await;
    assert!(evs.iter().any(|e| matches!(e, Ev::Connected(true))));
    let models = evs.iter().find_map(|e| if let Ev::Models(m) = e { Some(m.clone()) } else { None }).unwrap();
    assert_eq!(models, ["alpha-balanced", "alpha-max", "beta", "big", "mid"]);
    assert!(matches!(evs.last(), Some(Ev::Chats(c)) if c.is_empty()));

    w.backend.send(Cmd::Send { chat: 7, session: None, session_model: None, model: "alpha-balanced".into(), text: "hi".into(), start: Default::default() });
    let evs = w.until(|e| matches!(e, Ev::StreamEnded { .. })).await;
    assert!(matches!(&evs[0], Ev::SessionReady { chat: 7, model, .. } if model == "alpha-balanced"));
    assert_eq!(joined_text(&evs), "Hello there");
    assert!(evs.iter().any(|e| matches!(e, Ev::Server { event: lh_client::ServerEvent::Done { .. }, .. })));

    // the chat is now in the stored list, with its title
    w.backend.send(Cmd::Refresh);
    let evs = w.until(|e| matches!(e, Ev::Chats(_))).await;
    let Some(Ev::Chats(list)) = evs.last() else { panic!() };
    assert_eq!(list.len(), 1);
    assert_eq!((list[0].title.as_str(), list[0].messages, list[0].live), ("hi", 2, true));
}

#[tokio::test]
async fn an_unreachable_core_is_reported_not_hidden() {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    drop(l);
    let d = tempfile::tempdir().unwrap();
    let cfg = Config { base: format!("http://127.0.0.1:{port}"), token_file: d.path().join("missing"), prefs: d.path().join("p"), themes: d.path().join("t") };
    let mut w = Window::open(&cfg);
    w.backend.send(Cmd::Refresh);
    let evs = w.until(|e| matches!(e, Ev::Failed { .. })).await;
    assert!(evs.iter().any(|e| matches!(e, Ev::Connected(false))));
    assert!(matches!(evs.last(), Some(Ev::Failed { chat: 0, message }) if message.contains("Cannot reach the core")), "{evs:?}");
}

#[tokio::test]
async fn stop_ends_the_turn_in_the_middle_and_the_chat_can_go_on() {
    let core = fresh(vec![text("First answer, a long one"), text("Second")], Duration::from_millis(1500), false);
    let mut w = Window::open(&core.cfg);
    w.backend.send(Cmd::Send { chat: 1, session: None, session_model: None, model: "beta".into(), text: "go".into(), start: Default::default() });
    let evs = w.until(|e| matches!(e, Ev::Server { event: lh_client::ServerEvent::Text(_), .. })).await;
    let session = evs.iter().find_map(|e| if let Ev::SessionReady { session, .. } = e { Some(session.clone()) } else { None }).unwrap();
    w.backend.send(Cmd::Stop { chat: 1, session: Some(session.clone()) });
    w.until(|e| matches!(e, Ev::StreamEnded { chat: 1 })).await;
    // the core frees the session once the connection drops; the next turn may be refused for a moment
    tokio::time::sleep(Duration::from_millis(300)).await;
    w.backend.send(Cmd::Send { chat: 1, session: Some(session), session_model: Some("beta".into()), model: "beta".into(), text: "again".into(), start: Default::default() });
    let evs = w.until(|e| matches!(e, Ev::StreamEnded { .. })).await;
    assert!(!evs.iter().any(|e| matches!(e, Ev::Failed { .. })), "{evs:?}");
    assert!(joined_text(&evs).starts_with("Sec"), "the second answer streamed: {evs:?}");
}

#[tokio::test]
async fn switching_the_model_inside_a_chat_tells_the_core() {
    let core = fresh(vec![text("one"), text("two")], Duration::ZERO, false);
    let mut w = Window::open(&core.cfg);
    w.backend.send(Cmd::Send { chat: 1, session: None, session_model: None, model: "alpha-balanced".into(), text: "a".into(), start: Default::default() });
    let evs = w.until(|e| matches!(e, Ev::StreamEnded { .. })).await;
    let session = evs.iter().find_map(|e| if let Ev::SessionReady { session, .. } = e { Some(session.clone()) } else { None }).unwrap();
    w.backend.send(Cmd::Send { chat: 1, session: Some(session.clone()), session_model: Some("alpha-balanced".into()), model: "beta".into(), text: "b".into(), start: Default::default() });
    let evs = w.until(|e| matches!(e, Ev::StreamEnded { .. })).await;
    assert!(matches!(&evs[0], Ev::SessionReady { model, .. } if model == "beta"), "{evs:?}");
    w.backend.send(Cmd::Refresh);
    let evs = w.until(|e| matches!(e, Ev::Chats(_))).await;
    let Some(Ev::Chats(list)) = evs.last() else { panic!() };
    assert_eq!(list[0].model, "beta", "the stored chat remembers the new model");
}

#[tokio::test]
async fn a_stored_chat_is_loaded_resumed_after_a_warning_and_deleted() {
    let d = tempfile::tempdir().unwrap();
    let first = start(d.path(), vec![text("It is blue.")], Duration::ZERO, false);
    let mut w = Window::open(&first.cfg);
    w.backend.send(Cmd::Send { chat: 1, session: None, session_model: None, model: "alpha-balanced".into(), text: "What colour is the sky?".into(), start: Default::default() });
    let evs = w.until(|e| matches!(e, Ev::StreamEnded { .. })).await;
    let id = evs.iter().find_map(|e| if let Ev::SessionReady { session, .. } = e { Some(session.clone()) } else { None }).unwrap();
    drop(w);
    // a new core on the same store: the chat is stored but not loaded
    let second = start(&first.dir, vec![text("Scattering.")], Duration::ZERO, false);
    let mut w = Window::open(&second.cfg);
    w.backend.send(Cmd::Refresh);
    let evs = w.until(|e| matches!(e, Ev::Chats(_))).await;
    let Some(Ev::Chats(list)) = evs.last() else { panic!() };
    assert_eq!((list[0].id.as_str(), list[0].live), (id.as_str(), false));
    w.backend.send(Cmd::LoadHistory { chat: 5, id: id.clone() });
    let evs = w.until(|e| matches!(e, Ev::History { .. })).await;
    let Some(Ev::History { messages, context_tokens, .. }) = evs.last() else { panic!() };
    assert_eq!(messages, &[("user".to_string(), "What colour is the sky?".to_string()), ("assistant".to_string(), "It is blue.".to_string())]);
    assert!(*context_tokens > 200);
    // asking first gives the warning, with the other model named
    w.backend.send(Cmd::Resume { chat: 5, id: id.clone(), model: Some("beta".into()), confirm: false });
    let evs = w.until(|e| matches!(e, Ev::ResumeWarning { .. })).await;
    let Some(Ev::ResumeWarning { warning, model, .. }) = evs.last() else { panic!() };
    assert_eq!(model, "beta");
    assert_eq!(warning["kind"], "local");
    assert!(warning["message"].as_str().unwrap().contains("slow"));
    w.backend.send(Cmd::Resume { chat: 5, id: id.clone(), model: Some("beta".into()), confirm: true });
    let evs = w.until(|e| matches!(e, Ev::Resumed { .. })).await;
    assert!(matches!(evs.last(), Some(Ev::Resumed { session, model, .. }) if *session == id && model == "beta"));
    w.backend.send(Cmd::Send { chat: 5, session: Some(id.clone()), session_model: None, model: "beta".into(), text: "why?".into(), start: Default::default() });
    let evs = w.until(|e| matches!(e, Ev::StreamEnded { .. })).await;
    assert_eq!(joined_text(&evs), "Scattering.");
    // delete it for good
    w.backend.send(Cmd::Delete { chat: 5, id: Some(id.clone()), stored: true });
    w.until(|e| matches!(e, Ev::Deleted { chat: 5 })).await;
    w.backend.send(Cmd::Refresh);
    let evs = w.until(|e| matches!(e, Ev::Chats(_))).await;
    assert!(matches!(evs.last(), Some(Ev::Chats(c)) if c.is_empty()));
}

#[tokio::test]
async fn a_department_run_is_followed_questions_are_answered_and_the_page_model_ends_done() {
    const PLAN: &str = r#"{"steps":[{"id":"1","title":"Find facts","assignee":"researcher","brief":"Find three facts about cats.","depends_on":[]}]}"#;
    let core = fresh(vec![text(PLAN), text("Cats purr. Cats sleep. Cats climb."), text("OK"), text("Three facts about cats were found.")], Duration::ZERO, true);
    let mut w = Window::open(&core.cfg);
    let http = reqwest::Client::new();
    let token = std::fs::read_to_string(&core.cfg.token_file).unwrap();
    let put = |path: &str, body: Value| http.put(format!("{}{path}", core.cfg.base)).bearer_auth(&token).json(&body).send();
    put("/v1/team/types/frontier", json!({"model": "big"})).await.unwrap();
    put("/v1/team/types/mid-tier", json!({"model": "mid"})).await.unwrap();
    put("/v1/team/departments/pets", json!({"text": "---\nname: pets\ndescription: Answers pet questions.\nlead: planner\nmembers: researcher\n---\nBe brief.\n"})).await.unwrap();
    put("/v1/team/settings", json!({"autonomy": "plan-first"})).await.unwrap();

    w.backend.send(Cmd::Call { tag: "departments".into(), method: "GET".into(), path: "/v1/team/departments".into(), body: None });
    let evs = w.until(|e| matches!(e, Ev::Reply { .. })).await;
    assert!(matches!(evs.last(), Some(Ev::Reply { result: Ok(v), .. }) if v["departments"][0]["name"] == "pets"));

    w.backend.send(Cmd::StartRun { department: "pets".into(), goal: "tell me about cats".into(), project_dir: None });
    let mut run: Option<Run> = None;
    loop {
        match w.next().await {
            Ev::RunStarted { run: id, department, goal } => run = Some(Run::new(&id, &department, &goal)),
            Ev::RunEvent { run: id, event } => {
                let r = run.as_mut().unwrap();
                r.apply(&event);
                if let Some(q) = r.questions.first().cloned() {
                    assert!(r.is_waiting());
                    // the page would show the plan and ask; the user approves
                    let body = json!({"decision": "approve"});
                    w.backend.send(Cmd::AnswerRun { run: id, question: q.id().to_string(), body });
                    r.answered(q.id());
                }
            }
            Ev::RunEnded { .. } => break,
            Ev::Reply { result: Err(e), .. } => panic!("{e}"),
            _ => {}
        }
    }
    let r = run.unwrap();
    assert!(r.finished);
    assert_eq!(r.outcome, Some(Outcome::Done { result: "Three facts about cats were found.".into(), parked: 0 }));
    assert_eq!(r.progress(), (1, 1));
    assert_eq!(r.steps[0].title, "Find facts");
}

#[tokio::test]
async fn a_private_chat_is_not_stored_and_a_project_chat_remembers_its_kind() {
    let core = fresh(vec![text("secret reply"), text("project reply")], Duration::ZERO, false);
    let mut w = Window::open(&core.cfg);
    let private = lh_gui::api::SessionStart { mode: "chat".into(), project_dir: None, incognito: true };
    w.backend.send(Cmd::Send { chat: 1, session: None, session_model: None, model: "beta".into(), text: "private words".into(), start: private });
    w.until(|e| matches!(e, Ev::StreamEnded { .. })).await;
    w.backend.send(Cmd::Refresh);
    let evs = w.until(|e| matches!(e, Ev::Chats(_))).await;
    assert!(matches!(evs.last(), Some(Ev::Chats(c)) if c.is_empty()), "an incognito chat leaves nothing to list");

    let folder = tempfile::tempdir().unwrap();
    let project = lh_gui::api::SessionStart { mode: "project".into(), project_dir: Some(folder.path().to_string_lossy().into()), incognito: false };
    w.backend.send(Cmd::Send { chat: 2, session: None, session_model: None, model: "beta".into(), text: "work here".into(), start: project });
    let evs = w.until(|e| matches!(e, Ev::StreamEnded { .. })).await;
    assert_eq!(joined_text(&evs), "project reply");
    w.backend.send(Cmd::Refresh);
    let evs = w.until(|e| matches!(e, Ev::Chats(_))).await;
    let Some(Ev::Chats(list)) = evs.last() else { panic!() };
    assert_eq!((list.len(), list[0].mode.as_str()), (1, "project"));
}

#[tokio::test]
async fn a_tool_that_needs_permission_asks_and_goes_ahead_when_the_user_says_yes() {
    let folder = tempfile::tempdir().unwrap();
    let write = Response {
        tool_calls: vec![lh_provider::ToolCall { id: "c1".into(), name: "write_file".into(), arguments: json!({"path": "note.txt", "content": "hello"}).to_string(), repaired: false }],
        finish: Some(FinishReason::ToolCalls),
        ..Default::default()
    };
    let core = fresh(vec![write, text("Saved the note.")], Duration::ZERO, false);
    let mut w = Window::open(&core.cfg);
    let start = lh_gui::api::SessionStart { mode: "project".into(), project_dir: Some(folder.path().to_string_lossy().into()), incognito: false };
    w.backend.send(Cmd::Send { chat: 1, session: None, session_model: None, model: "beta".into(), text: "save a note".into(), start });
    let mut chat = lh_gui::model::Chat::new(1, "beta");
    chat.push_user("save a note");
    let mut session = String::new();
    loop {
        match w.next().await {
            Ev::SessionReady { session: s, .. } => session = s,
            Ev::Server { event, .. } => {
                chat.apply(&event);
                if let Some(a) = chat.asks.first().cloned() {
                    assert_eq!(a.tool, "write_file");
                    assert!(!file_exists(folder.path()), "nothing is written before the answer");
                    w.backend.send(Cmd::Answer { session: session.clone(), ask: a.id.clone(), yes: true });
                    chat.answer(&a.id);
                }
            }
            Ev::StreamEnded { .. } => break,
            Ev::Failed { message, .. } => panic!("{message}"),
            _ => {}
        }
    }
    assert!(file_exists(folder.path()), "the file was written after yes");
    assert!(!chat.busy);
    assert!(chat.blocks.iter().any(|b| matches!(b, lh_gui::model::Block::Tool { name, result: Some(_), .. } if name == "write_file")));
    assert!(chat.blocks.iter().any(|b| matches!(b, lh_gui::model::Block::Assistant { text, .. } if text == "Saved the note.")));
}

fn file_exists(dir: &std::path::Path) -> bool {
    dir.join("note.txt").exists()
}

#[tokio::test]
async fn something_the_assistant_shows_arrives_as_content_for_the_side_panel() {
    let folder = tempfile::tempdir().unwrap();
    let show = Response {
        tool_calls: vec![lh_provider::ToolCall { id: "s1".into(), name: "show".into(), arguments: json!({"title": "Summary", "content": "# Summary\n- done", "kind": "markdown"}).to_string(), repaired: false }],
        finish: Some(FinishReason::ToolCalls),
        ..Default::default()
    };
    let core = fresh(vec![show, text("It is open beside the chat.")], Duration::ZERO, false);
    let mut w = Window::open(&core.cfg);
    let start = lh_gui::api::SessionStart { mode: "project".into(), project_dir: Some(folder.path().to_string_lossy().into()), incognito: false };
    w.backend.send(Cmd::Send { chat: 1, session: None, session_model: None, model: "beta".into(), text: "summarise".into(), start });
    let evs = w.until(|e| matches!(e, Ev::StreamEnded { .. })).await;
    let shown = evs.iter().find_map(|e| match e {
        Ev::Server { event: lh_client::ServerEvent::Show { title, kind, content, path }, .. } => Some(lh_gui::content::Content::from_show(title, kind, content, path.as_deref())),
        _ => None,
    });
    let c = shown.expect("a show event arrived");
    assert_eq!((c.title.as_str(), c.kind, c.body.as_str()), ("Summary", lh_gui::content::ContentKind::Markdown, "# Summary\n- done"));
}

#[tokio::test]
async fn the_tree_panel_loads_moves_and_shows_the_branch_through_the_backend() {
    use lh_client::tree::{ChatTree, Moved};
    use lh_gui::treeview::{Outcome, TreeState};
    let core = fresh(vec![text("a one"), text("a two")], Duration::ZERO, false);
    let mut w = Window::open(&core.cfg);
    w.backend.send(Cmd::Send { chat: 3, session: None, session_model: None, model: "alpha-balanced".into(), text: "q one".into(), start: Default::default() });
    let evs = w.until(|e| matches!(e, Ev::StreamEnded { .. })).await;
    let sid = evs.iter().find_map(|e| if let Ev::SessionReady { session, .. } = e { Some(session.clone()) } else { None }).unwrap();
    w.backend.send(Cmd::Send { chat: 3, session: Some(sid.clone()), session_model: None, model: "alpha-balanced".into(), text: "q two".into(), start: Default::default() });
    w.until(|e| matches!(e, Ev::StreamEnded { .. })).await;

    // load the tree the way the panel does
    w.backend.send(Cmd::Call { tag: "tree:3".into(), method: "GET".into(), path: format!("/v1/chats/{sid}/tree"), body: None });
    let evs = w.until(|e| matches!(e, Ev::Reply { tag, .. } if tag == "tree:3")).await;
    let Some(Ev::Reply { result: Ok(v), .. }) = evs.last() else { panic!("{evs:?}") };
    let mut state = TreeState::default();
    state.load(3, ChatTree::from_json(v).unwrap());
    let rows = state.tree.as_ref().unwrap().rows();
    assert_eq!(rows.iter().map(|r| r.text.as_str()).collect::<Vec<_>>(), ["start of the chat", "q one", "a one", "q two", "a two"]);
    assert!(rows.last().unwrap().current);

    // choose the second question: the core asks about a summary, and nothing moves until the answer
    let q2 = state.tree.as_ref().unwrap().nodes.iter().find(|n| n.preview == "q two").unwrap().id;
    w.backend.send(Cmd::Call { tag: format!("tree-move:3:{q2}"), method: "POST".into(), path: format!("/v1/chats/{sid}/tree/move"), body: Some(json!({"node": q2})) });
    let evs = w.until(|e| matches!(e, Ev::Reply { tag, .. } if tag.starts_with("tree-move"))).await;
    let Some(Ev::Reply { result: Ok(v), .. }) = evs.last() else { panic!() };
    assert_eq!(state.moved(q2, Moved::from_json(v)), Outcome::Asked);
    // no summary
    w.backend.send(Cmd::Call { tag: format!("tree-move:3:{q2}"), method: "POST".into(), path: format!("/v1/chats/{sid}/tree/move"), body: Some(json!({"node": q2, "summarize": false})) });
    let evs = w.until(|e| matches!(e, Ev::Reply { tag, .. } if tag.starts_with("tree-move"))).await;
    let Some(Ev::Reply { result: Ok(v), .. }) = evs.last() else { panic!() };
    assert_eq!(state.moved(q2, Moved::from_json(v)), Outcome::Moved { input: Some("q two".into()), summary: false });
    // the chat's own history is now the branch, and the list counts only it
    w.backend.send(Cmd::LoadHistory { chat: 3, id: sid.clone() });
    let evs = w.until(|e| matches!(e, Ev::History { .. })).await;
    let Some(Ev::History { messages, .. }) = evs.last() else { panic!() };
    assert_eq!(messages, &[("user".to_string(), "q one".to_string()), ("assistant".to_string(), "a one".to_string())]);
    w.backend.send(Cmd::Refresh);
    let evs = w.until(|e| matches!(e, Ev::Chats(_))).await;
    let Some(Ev::Chats(list)) = evs.last() else { panic!() };
    assert_eq!(list[0].messages, 2);
    // a private chat has no tree and says so
    w.backend.send(Cmd::Send { chat: 4, session: None, session_model: None, model: "alpha-balanced".into(), text: "hush".into(), start: lh_gui::api::SessionStart { incognito: true, mode: "chat".into(), ..Default::default() } });
    let evs = w.until(|e| matches!(e, Ev::StreamEnded { chat: 4 })).await;
    let psid = evs.iter().find_map(|e| if let Ev::SessionReady { session, .. } = e { Some(session.clone()) } else { None }).unwrap();
    w.backend.send(Cmd::Call { tag: "tree:4".into(), method: "GET".into(), path: format!("/v1/chats/{psid}/tree"), body: None });
    let evs = w.until(|e| matches!(e, Ev::Reply { tag, .. } if tag == "tree:4")).await;
    let Some(Ev::Reply { result: Err(e), .. }) = evs.last() else { panic!() };
    state.failed(4, e.clone());
    assert!(state.tree.is_none() && state.note.contains("private"));
}
