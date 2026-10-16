//! Saving the model's KV cache per local chat, against a fake llama-swap: when files are made, how they are
//! protected and removed, the size cap, when nothing is done, and putting the cache back for a chat opened again.
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::{Path as UrlPath, Query, State};
use axum::routing::post;
use axum::{Json, Router};
use lh_core::kv_cache::KvCache;
use lh_core::{auth, router, AppState, MemoryRuntime, Settings};
use lh_memory::embed::AnyEmbedder;
use lh_memory::{Memory, RecallConfig, Trust};
use lh_provider::{ChatRequest, Event, FinishReason, Response, SseParser};
use lh_store::Store;
use lh_tools::Model;
use serde_json::{json, Value};

struct Script {
    replies: Mutex<Vec<Response>>,
}
impl Model for Script {
    async fn complete(&self, _r: &ChatRequest, on: &mut (dyn FnMut(Event) + Send)) -> Result<Response, lh_provider::Error> {
        let resp = self.replies.lock().unwrap().pop().expect("unscripted model call");
        on(Event::Text(resp.text.clone()));
        Ok(resp)
    }
}
fn text(t: &str) -> Response {
    Response { text: t.into(), finish: Some(FinishReason::Stop), ..Default::default() }
}

/// What the fake llama-swap does with slot calls.
#[derive(Clone)]
struct Fake {
    saved_into: PathBuf,
    size: usize,
    /// `false`: refuse every slot call, like a server without a save folder.
    works: bool,
    seen: Arc<Mutex<Vec<String>>>,
}

async fn slot(State(f): State<Fake>, UrlPath(model): UrlPath<String>, Query(q): Query<std::collections::HashMap<String, String>>, Json(b): Json<Value>) -> (axum::http::StatusCode, Json<Value>) {
    let action = q.get("action").cloned().unwrap_or_default();
    let name = b["filename"].as_str().unwrap_or("").to_string();
    f.seen.lock().unwrap().push(format!("{action} {name} {model}"));
    if !f.works {
        return (axum::http::StatusCode::NOT_IMPLEMENTED, Json(json!({"error": "slot save is disabled"})));
    }
    match action.as_str() {
        "save" => {
            let dir = f.saved_into.join("abc123"); // one folder per model, as the router does
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(&name), vec![7u8; f.size]).unwrap();
            (axum::http::StatusCode::OK, Json(json!({"n_saved": 321, "n_written": f.size})))
        }
        "restore" => {
            let p = f.saved_into.join("abc123").join(&name);
            if p.exists() { (axum::http::StatusCode::OK, Json(json!({"n_restored": 321}))) } else { (axum::http::StatusCode::BAD_REQUEST, Json(json!({"error": "no such file"}))) }
        }
        _ => (axum::http::StatusCode::OK, Json(json!({}))),
    }
}

async fn fake_router(f: Fake) -> String {
    let app = Router::new().route("/upstream/{model}/slots/0", post(slot)).with_state(f);
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    format!("http://127.0.0.1:{port}/v1")
}

struct Server {
    base: String,
    token: String,
    http: reqwest::Client,
    seen: Arc<Mutex<Vec<String>>>,
    store: Arc<Store>,
}
impl Server {
    fn post(&self, p: &str, b: Value) -> reqwest::RequestBuilder {
        self.http.post(format!("{}{p}", self.base)).bearer_auth(&self.token).json(&b)
    }
    fn get(&self, p: &str) -> reqwest::RequestBuilder {
        self.http.get(format!("{}{p}", self.base)).bearer_auth(&self.token)
    }
    async fn json(&self, rb: reqwest::RequestBuilder) -> Value {
        rb.send().await.unwrap().json().await.unwrap_or(Value::Null)
    }
    async fn session(&self, extra: Value) -> String {
        let mut body = json!({"model": "m", "mode": "chat"});
        for (k, v) in extra.as_object().unwrap() {
            body[k] = v.clone();
        }
        self.json(self.post("/v1/sessions", body)).await["id"].as_str().unwrap().to_string()
    }
    async fn turn(&self, id: &str, msg: &str) {
        let mut resp = self.post(&format!("/v1/sessions/{id}/turns"), json!({"text": msg})).send().await.unwrap();
        let mut p = SseParser::new();
        'o: while let Some(chunk) = tokio::time::timeout(Duration::from_secs(10), resp.chunk()).await.expect("no event").unwrap() {
            for d in p.push(&chunk) {
                let v: Value = serde_json::from_str(&d).unwrap();
                if matches!(v["type"].as_str(), Some("done" | "error")) {
                    break 'o;
                }
            }
        }
        // the save runs just after the turn ends
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}

struct Setup<'a> {
    data: &'a Path,
    slots: &'a Path,
    trust: Trust,
    fake: Fake,
    cap: Option<u64>,
    replies: Vec<Response>,
}

async fn start(s: Setup<'_>) -> (Server, Arc<KvCache>) {
    let provider = fake_router(s.fake.clone()).await;
    let store = Arc::new(Store::open(&s.data.join("lh.db")).unwrap());
    let mem = Arc::new(Memory::open(store.clone(), s.data.join("memory")).unwrap());
    let mut replies = s.replies;
    replies.reverse();
    let settings = Settings {
        team: None,
        mcp: None,
        ask_all: false,
        config: None,
        provider_name: None,
        learn: None,
        maintain_when_idle: false,
        memory: Some(MemoryRuntime { memory: mem, embedder: Arc::new(AnyEmbedder::Off), trust: s.trust, cfg: RecallConfig::default() }),
        image_model: None,
        web: Default::default(),
        idle_after: Duration::ZERO,
        compact_at: Some(0),
        store: store.clone(),
        spend: lh_spend::SpendMeter::new(store.clone(), lh_spend::Config::default(), "test", true),
        bypass: false,
        sudo_enabled: false,
        allow_rules: vec![],
    };
    let mut kv = KvCache::new(&provider, None, s.slots.to_path_buf(), 5);
    if let Some(c) = s.cap {
        kv = kv.with_max_bytes(c);
    }
    let kv = Arc::new(kv);
    let state = AppState::new(Script { replies: Mutex::new(replies) }, settings);
    state.set_kv(Some(kv.clone()));
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    l.set_nonblocking(true).unwrap();
    let port = l.local_addr().unwrap().port();
    let token = auth::new_token();
    let app = router(state, token.clone(), port);
    tokio::spawn(async move { axum::serve(tokio::net::TcpListener::from_std(l).unwrap(), app).await.unwrap() });
    (Server { base: format!("http://127.0.0.1:{port}"), token, http: reqwest::Client::new(), seen: s.fake.seen.clone(), store }, kv)
}

fn fake(slots: &Path, size: usize, works: bool) -> Fake {
    Fake { saved_into: slots.to_path_buf(), size, works, seen: Default::default() }
}

fn bins(slots: &Path) -> Vec<PathBuf> {
    let mut v = vec![];
    for d in std::fs::read_dir(slots).into_iter().flatten().flatten() {
        for f in std::fs::read_dir(d.path()).into_iter().flatten().flatten() {
            v.push(f.path());
        }
    }
    v
}

#[tokio::test]
async fn a_local_chat_is_saved_after_each_turn_in_one_private_file() {
    let (data, slots) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let (s, _kv) = start(Setup { data: data.path(), slots: slots.path(), trust: Trust::Local, fake: fake(slots.path(), 1000, true), cap: None, replies: vec![text("one"), text("two")] }).await;
    let id = s.session(json!({})).await;
    s.turn(&id, "first").await;
    let files = bins(slots.path());
    assert_eq!(files.len(), 1, "{files:?}");
    let name = files[0].file_name().unwrap().to_string_lossy().to_string();
    assert!(name.starts_with(&format!("lh-{id}-")) && name.ends_with(".bin"), "{name}");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(&files[0]).unwrap().permissions().mode() & 0o777, 0o600, "private to the user");
    }
    s.turn(&id, "second").await;
    assert_eq!(bins(slots.path()).len(), 1, "the file is replaced, not added to");
    let seen = s.seen.lock().unwrap().clone();
    assert_eq!(seen.iter().filter(|x| x.starts_with("save ")).count(), 2, "{seen:?}");
    assert!(seen[0].ends_with(" m"), "the model id is in the path: {seen:?}");
    // what was logged has sizes, not text
    let log = s.store.recent(20, None).unwrap_or_default();
    let saved: Vec<String> = log.iter().filter(|e| e.kind == "kv_cache_saved").map(|e| e.detail.to_string()).collect();
    assert_eq!(saved.len(), 2, "{log:?}");
    assert!(saved.iter().all(|d| d.contains("bytes") && !d.contains("first")));
}

#[tokio::test]
async fn private_chats_and_cloud_endpoints_leave_no_cache() {
    let (data, slots) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let (s, _) = start(Setup { data: data.path(), slots: slots.path(), trust: Trust::Local, fake: fake(slots.path(), 1000, true), cap: None, replies: vec![text("a")] }).await;
    let id = s.session(json!({"incognito": true})).await;
    s.turn(&id, "secret").await;
    assert!(bins(slots.path()).is_empty() && s.seen.lock().unwrap().is_empty(), "a private chat is never saved");
    let (data2, slots2) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let (s2, _) = start(Setup { data: data2.path(), slots: slots2.path(), trust: Trust::AnyCloud, fake: fake(slots2.path(), 1000, true), cap: None, replies: vec![text("a")] }).await;
    let id = s2.session(json!({})).await;
    s2.turn(&id, "hello").await;
    assert!(bins(slots2.path()).is_empty() && s2.seen.lock().unwrap().is_empty(), "nothing is saved for a cloud endpoint");
}

#[tokio::test]
async fn deleting_a_chat_deletes_its_cache_but_a_closed_session_keeps_it() {
    let (data, slots) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let (s, _) = start(Setup { data: data.path(), slots: slots.path(), trust: Trust::Local, fake: fake(slots.path(), 1000, true), cap: None, replies: vec![text("a"), text("b")] }).await;
    let (one, two) = (s.session(json!({})).await, s.session(json!({})).await);
    s.turn(&one, "one").await;
    s.turn(&two, "two").await;
    assert_eq!(bins(slots.path()).len(), 2);
    s.http.delete(format!("{}/v1/sessions/{one}", s.base)).bearer_auth(&s.token).send().await.unwrap();
    assert_eq!(bins(slots.path()).len(), 2, "closing a session does not delete the stored chat or its cache");
    let r = s.http.delete(format!("{}/v1/chats/{one}", s.base)).bearer_auth(&s.token).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 204);
    let left = bins(slots.path());
    assert_eq!(left.len(), 1);
    assert!(left[0].file_name().unwrap().to_string_lossy().starts_with(&format!("lh-{two}-")));
}

#[tokio::test]
async fn the_folder_is_kept_under_the_cap_by_removing_the_oldest_first() {
    let (data, slots) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let (s, _) = start(Setup { data: data.path(), slots: slots.path(), trust: Trust::Local, fake: fake(slots.path(), 1000, true), cap: Some(2500), replies: vec![text("a"), text("b"), text("c")] }).await;
    let mut ids = vec![];
    for t in ["one", "two", "three"] {
        let id = s.session(json!({})).await;
        s.turn(&id, t).await;
        tokio::time::sleep(Duration::from_millis(1100)).await; // file times differ by whole seconds on some systems
        ids.push(id);
    }
    let names: Vec<String> = bins(slots.path()).iter().map(|p| p.file_name().unwrap().to_string_lossy().to_string()).collect();
    assert_eq!(names.len(), 2, "3000 bytes written, 2500 allowed: {names:?}");
    assert!(!names.iter().any(|n| n.starts_with(&format!("lh-{}-", ids[0]))), "the oldest went: {names:?}");
    assert!(names.iter().any(|n| n.starts_with(&format!("lh-{}-", ids[2]))), "the newest stays");
}

#[tokio::test]
async fn a_server_that_cannot_save_switches_the_feature_off_without_hurting_the_chat() {
    let (data, slots) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let (s, kv) = start(Setup { data: data.path(), slots: slots.path(), trust: Trust::Local, fake: fake(slots.path(), 1000, false), cap: None, replies: vec![text("a"), text("b")] }).await;
    let id = s.session(json!({})).await;
    s.turn(&id, "one").await;
    assert!(kv.is_off(), "{:?}", kv.why_off());
    assert!(kv.why_off().unwrap().contains("cannot save a slot"));
    let before = s.seen.lock().unwrap().len();
    s.turn(&id, "two").await;
    assert_eq!(s.seen.lock().unwrap().len(), before, "no more calls once it is off");
    assert!(bins(slots.path()).is_empty());
}

#[tokio::test]
async fn a_file_saved_outside_the_configured_folder_is_a_file_nobody_can_protect_so_saving_stops() {
    let (data, slots, elsewhere) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let (s, kv) = start(Setup { data: data.path(), slots: slots.path(), trust: Trust::Local, fake: fake(elsewhere.path(), 1000, true), cap: None, replies: vec![text("a")] }).await;
    let id = s.session(json!({})).await;
    s.turn(&id, "one").await;
    assert!(kv.is_off());
    assert!(kv.why_off().unwrap().contains("kvcache.dir"), "{:?}", kv.why_off());
}

#[tokio::test]
async fn a_chat_opened_again_gets_its_cache_back_before_the_first_turn_unless_the_model_changed() {
    let (data, slots) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let fk = fake(slots.path(), 1000, true);
    let id = {
        let (s, _) = start(Setup { data: data.path(), slots: slots.path(), trust: Trust::Local, fake: fk.clone(), cap: None, replies: vec![text("a")] }).await;
        let id = s.session(json!({})).await;
        s.turn(&id, "first").await;
        id
    };
    fk.seen.lock().unwrap().clear();
    // a fresh core: same model, the cache is restored
    let (s, _) = start(Setup { data: data.path(), slots: slots.path(), trust: Trust::Local, fake: fk.clone(), cap: None, replies: vec![text("b")] }).await;
    let w = s.json(s.post(&format!("/v1/chats/{id}/resume"), json!({"model": "m"}))).await;
    assert_eq!(w["warning"]["cache_saved"], true);
    assert!(w["warning"]["message"].as_str().unwrap().contains("saved, so it should carry on quickly"), "{w}");
    let w = s.json(s.post(&format!("/v1/chats/{id}/resume"), json!({"model": "another-model"}))).await;
    assert_eq!(w["warning"]["cache_saved"], false, "a cache only helps the model that made it");
    assert!(w["warning"]["message"].as_str().unwrap().contains("slow"));
    s.json(s.post(&format!("/v1/chats/{id}/resume"), json!({"model": "m", "confirm": true}))).await;
    s.turn(&id, "second").await;
    let seen = fk.seen.lock().unwrap().clone();
    assert!(seen.iter().any(|x| x.starts_with("restore lh-") && x.contains(&id)), "{seen:?}");
    assert!(seen.iter().position(|x| x.starts_with("restore")).unwrap() < seen.iter().position(|x| x.starts_with("save")).unwrap(), "restored first, saved after the turn");
    let log = s.store.recent(30, None).unwrap_or_default();
    assert!(log.iter().any(|e| e.kind == "kv_cache_restored"), "{log:?}");
    // another core, another model: nothing to restore for it
    fk.seen.lock().unwrap().clear();
    let (s2, _) = start(Setup { data: data.path(), slots: slots.path(), trust: Trust::Local, fake: fk.clone(), cap: None, replies: vec![text("c")] }).await;
    s2.json(s2.post(&format!("/v1/chats/{id}/resume"), json!({"model": "another-model", "confirm": true}))).await;
    s2.turn(&id, "third").await;
    let seen = fk.seen.lock().unwrap().clone();
    assert!(!seen.iter().any(|x| x.starts_with("restore")), "a cache is only for the model that made it: {seen:?}");
    let log = s2.store.recent(30, None).unwrap_or_default();
    assert!(log.iter().any(|e| e.kind == "kv_cache_skipped"), "{log:?}");
}

#[tokio::test]
async fn moving_in_the_chat_tree_drops_the_saved_cache_because_it_no_longer_matches_the_chat() {
    let (data, slots) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let (s, _kv) = start(Setup { data: data.path(), slots: slots.path(), trust: Trust::Local, fake: fake(slots.path(), 1000, true), cap: None, replies: vec![text("one"), text("two"), text("three")] }).await;
    let id = s.session(json!({})).await;
    s.turn(&id, "first").await;
    s.turn(&id, "second").await;
    assert_eq!(bins(slots.path()).len(), 1, "the chat's work is saved");
    let t = s.json(s.get(&format!("/v1/chats/{id}/tree"))).await;
    let a1 = t["nodes"].as_array().unwrap().iter().find(|n| n["preview"] == "one").unwrap()["id"].as_i64().unwrap();
    s.json(s.post(&format!("/v1/chats/{id}/tree/move"), json!({"node": a1, "summarize": false}))).await;
    assert!(bins(slots.path()).is_empty(), "a saved cache of the longer branch is not valid for the shorter one, so it is gone");
    s.turn(&id, "third").await;
    assert_eq!(bins(slots.path()).len(), 1, "the next turn saves a cache that matches the new branch");
    let seen = s.seen.lock().unwrap().clone();
    assert!(!seen.iter().any(|x| x.starts_with("restore ")), "an old cache is never put back: {seen:?}");
}
