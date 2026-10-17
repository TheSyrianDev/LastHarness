//! The assistant's memory tools through the real turn loop.
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use lh_memory::embed::AnyEmbedder;
use lh_memory::*;
use lh_provider::{ChatRequest, Event, FinishReason, Message, Response, Role, ToolCall};
use lh_store::Store;
use lh_tools::*;

struct Script(Mutex<Vec<Response>>);
impl Model for Script {
    async fn complete(&self, _r: &ChatRequest, _on: &mut (dyn FnMut(Event) + Send)) -> std::result::Result<Response, lh_provider::Error> {
        Ok(self.0.lock().unwrap().pop().expect("unscripted call"))
    }
}
fn script(mut r: Vec<Response>) -> Script {
    r.reverse();
    Script(Mutex::new(r))
}
struct User(Answer, AtomicUsize);
impl Asker for User {
    async fn ask(&self, r: &AskRequest<'_>) -> Answer {
        assert!(r.tool == "remember" || r.tool == "forget", "only changes ask");
        self.1.fetch_add(1, Ordering::SeqCst);
        self.0
    }
}
fn call(name: &str, args: &str) -> Response {
    Response { tool_calls: vec![ToolCall { id: "c1".into(), name: name.into(), arguments: args.into(), repaired: false }], finish: Some(FinishReason::ToolCalls), ..Default::default() }
}
fn done() -> Response {
    Response { text: "ok".into(), finish: Some(FinishReason::Stop), ..Default::default() }
}

struct Setup {
    _d: tempfile::TempDir,
    mem: Arc<Memory>,
    tools: ToolSet,
    perms: Permissions,
}
fn setup(trust: Trust) -> Setup {
    let d = tempfile::tempdir().unwrap();
    let mem = Arc::new(Memory::open(Arc::new(Store::open_memory().unwrap()), d.path().join("m")).unwrap());
    let h = Arc::new(MemoryHandle { memory: mem.clone(), embedder: Arc::new(AnyEmbedder::Off), dest: Destination { trust, project: None }, cfg: RecallConfig::default(), layers: None, judge: None });
    let proj = d.path().join("proj");
    std::fs::create_dir(&proj).unwrap();
    Setup { tools: default_tools().with_memory(h), perms: Permissions::new(&proj).unwrap(), mem, _d: d }
}
async fn turn(s: &Setup, replies: Vec<Response>, user: &User) -> Vec<String> {
    let mut req = ChatRequest { model: "m".into(), messages: vec![Message::user("x")], ..Default::default() };
    run_turn(&script(replies), &NoMeter, &mut req, &s.tools, &s.perms, user, &TurnOptions::default(), |_| {}).await.unwrap();
    req.messages.iter().filter(|m| m.role == Role::Tool).map(|m| m.content.clone().unwrap()).collect()
}
fn page(m: &Memory, title: &str, body: &str, p: Privacy) -> Page {
    let mut n = NewPage::new(Layer::Facts, title, body);
    n.privacy = p;
    m.create(n, By::User).unwrap()
}

#[tokio::test]
async fn recall_reads_memory_without_asking_and_a_cloud_model_sees_no_sensitive_notes() {
    let cloud = setup(Trust::AnyCloud);
    let open = page(&cloud.mem, "Trip", "Flight to Berlin in June.", Privacy::Normal);
    page(&cloud.mem, "Medication", "Takes metformin for the Berlin clinic.", Privacy::Sensitive);
    let u = User(Answer::No, AtomicUsize::new(0));
    let r = turn(&cloud, vec![call("recall", r#"{"query":"Berlin"}"#), done()], &u).await;
    assert!(r[0].contains(&open.id) && r[0].contains("Flight to Berlin"));
    assert!(!r[0].contains("metformin") && !r[0].contains("Medication"), "{}", r[0]);
    assert_eq!(u.1.load(Ordering::SeqCst), 0, "reading memory does not ask");
    // the list is short; a whole note is read on request, with the same gate
    let long = page(&cloud.mem, "Essay", &format!("Berlin {}", "words ".repeat(100)), Privacy::Normal);
    let r = turn(&cloud, vec![call("recall", r#"{"query":"essay"}"#), call("open_note", &format!(r#"{{"id":"{}"}}"#, long.id)), done()], &u).await;
    assert!(r[0].contains('…') && r[0].contains("open_note"), "{}", r[0]);
    assert!(r[1].contains(&"words ".repeat(99)), "the whole note");
    let hidden = page(&cloud.mem, "Hidden", "metformin dose", Privacy::Sensitive);
    let r = turn(&cloud, vec![call("open_note", &format!(r#"{{"id":"{}"}}"#, hidden.id)), done()], &u).await;
    assert!(r[0].contains("there is no note") && !r[0].contains("metformin"));
    let none = turn(&cloud, vec![call("recall", r#"{"query":"metformin"}"#), done()], &u).await;
    assert_eq!(none[0], "Nothing in memory matches.", "no hint that a hidden note exists");

    let local = setup(Trust::Local);
    page(&local.mem, "Medication", "Takes metformin.", Privacy::Sensitive);
    assert!(turn(&local, vec![call("recall", r#"{"query":"metformin"}"#), done()], &u).await[0].contains("metformin"));
}

#[tokio::test]
async fn remember_asks_first_marks_sensitive_text_and_can_be_refused() {
    let s = setup(Trust::AnyCloud);
    let yes = User(Answer::Yes, AtomicUsize::new(0));
    let r = turn(&s, vec![call("remember", r#"{"text":"The user prefers answers in plain words.","layer":"preferences"}"#), done()], &yes).await;
    assert!(r[0].starts_with("Remembered as"), "{}", r[0]);
    assert_eq!(yes.1.load(Ordering::SeqCst), 1);
    let pages = s.mem.list(Some(Layer::Preferences), State::Live).unwrap();
    assert_eq!((pages.len(), pages[0].source.as_str(), pages[0].privacy), (1, "assistant", Privacy::Normal));
    assert_eq!(s.mem.history(&pages[0].id).unwrap()[0].by, "assistant");

    let r = turn(&s, vec![call("remember", r#"{"text":"The user's bank password is hunter2."}"#), done()], &yes).await;
    assert!(r[0].contains("marked sensitive"));
    let secret = s.mem.search("hunter2", State::Live, 3).unwrap();
    assert_eq!(secret[0].page.privacy, Privacy::Sensitive);

    let no = User(Answer::No, AtomicUsize::new(0));
    let before = s.mem.list(None, State::Live).unwrap().len();
    let r = turn(&s, vec![call("remember", r#"{"text":"Something else."}"#), done()], &no).await;
    assert!(r[0].starts_with("The user said no"));
    assert_eq!(s.mem.list(None, State::Live).unwrap().len(), before);
}

#[tokio::test]
async fn forget_removes_one_note_after_a_yes_and_cannot_touch_what_the_model_may_not_see() {
    let s = setup(Trust::AnyCloud);
    let a = page(&s.mem, "Old address", "Lived on Palm Street.", Privacy::Normal);
    let hidden = page(&s.mem, "Insulin", "Takes insulin.", Privacy::Sensitive);
    let yes = User(Answer::Yes, AtomicUsize::new(0));
    let r = turn(&s, vec![call("forget", &format!(r#"{{"id":"{}"}}"#, a.id)), done()], &yes).await;
    assert!(r[0].contains("Removed \"Old address\""));
    assert!(s.mem.get(&a.id).is_err());
    assert!(s.mem.search("Palm", State::Any, 3).unwrap().is_empty());

    let r = turn(&s, vec![call("forget", &format!(r#"{{"id":"{}"}}"#, hidden.id)), done()], &yes).await;
    assert!(r[0].contains("there is no note"), "{}", r[0]);
    assert!(s.mem.get(&hidden.id).is_ok(), "still there");
    let r = turn(&s, vec![call("forget", r#"{"id":"p_nope"}"#), done()], &yes).await;
    assert!(r[0].contains("there is no note"));
}

#[tokio::test]
async fn without_a_memory_handle_the_tools_do_not_exist() {
    let tools = default_tools();
    for n in ["recall", "remember", "forget"] {
        assert!(tools.get(n).is_none(), "{n}");
    }
    assert!(!tools.defs().iter().any(|d| d.name == "remember"));
}
