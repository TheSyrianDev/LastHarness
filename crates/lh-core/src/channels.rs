//! Messaging channels in the core: which channels are set up, who may use them, and the loop that
//! carries messages between a chat app and the same local API every other front end uses.
//!
//! Rules that never bend: a channel is off until the user has read its warning and switched it on; only
//! people on its allow-list are answered, and a message from anyone else is dropped without waking the
//! core, answering, or logging its text; channel conversations run at the lowest trust level, so
//! sensitive notes and rules are never used; a reply that looks sensitive is held back.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use lh_channels::{build, required_settings, split_message, warning, Channel, ChannelConfig, Error as ChError, Message, KINDS};
use lh_provider::SseParser;
use lh_store::Store;
use lh_tools::Model;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{ApiError, AppState};

const KEY: &str = "channels";
const REPLY_MAX: usize = 3500;
const INPUT_MAX: usize = 4000;

fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn get_setting(store: &Store, key: &str) -> Option<String> {
    use lh_store::rusqlite::OptionalExtension;
    store.with(|c| c.query_row("select value from settings where key = ?1", [key], |r| r.get(0)).optional()).ok().flatten()
}

fn put_setting(store: &Store, key: &str, value: &str) {
    let _ = store.with(|c| c.execute("insert into settings(key, value, updated) values (?1, ?2, ?3) on conflict(key) do update set value = excluded.value, updated = excluded.updated", (key, value, now())));
}

pub fn load(store: &Store) -> Vec<ChannelConfig> {
    get_setting(store, KEY).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn save(store: &Store, list: &[ChannelConfig]) {
    put_setting(store, KEY, &serde_json::to_string(list).unwrap_or_default());
}

fn status(store: &Store, name: &str) -> Value {
    get_setting(store, &format!("channel_status:{name}")).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(json!({}))
}

// ---- the API for setting channels up (the machine's own terminal only) ----

pub(crate) fn routes<M: Model + 'static>() -> Router<Arc<AppState<M>>> {
    Router::new()
        .route("/v1/channels", get(list::<M>).post(add::<M>))
        .route("/v1/channels/{name}", axum::routing::put(edit::<M>).delete(remove::<M>))
        .route("/v1/channels/{name}/enable", post(enable::<M>))
        .route("/v1/channels/{name}/disable", post(disable::<M>))
}

fn bad(m: impl Into<String>) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, m.into())
}

fn shown(c: &ChannelConfig, store: &Store) -> Value {
    json!({"name": c.name, "kind": c.kind, "enabled": c.enabled, "allow": c.allow, "mode": c.mode, "model": c.model, "settings": c.settings, "status": status(store, &c.name)})
}

async fn list<M: Model + 'static>(State(st): State<Arc<AppState<M>>>) -> Json<Value> {
    let kinds: Vec<Value> = KINDS.iter().map(|k| {
        let (need, opt) = required_settings(k).unwrap_or((&[], &[]));
        json!({"kind": k, "required": need.iter().map(|(n, d)| json!({"name": n, "help": d})).collect::<Vec<_>>(), "optional": opt.iter().map(|(n, d)| json!({"name": n, "help": d})).collect::<Vec<_>>(), "warning": warning(k)})
    }).collect();
    Json(json!({"channels": load(&st.settings.store).iter().map(|c| shown(c, &st.settings.store)).collect::<Vec<_>>(), "kinds": kinds}))
}

fn check_fields(c: &ChannelConfig) -> Result<(), ApiError> {
    if c.name.is_empty() || !c.name.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_') {
        return Err(bad("the name may use letters, digits, - and _"));
    }
    if required_settings(&c.kind).is_none() {
        return Err(bad(format!("unknown channel kind {}; the kinds are {}", c.kind, KINDS.join(", "))));
    }
    if !["chat", "assistant"].contains(&c.mode.as_str()) {
        return Err(bad("mode must be chat (no tools) or assistant (tools, with questions asked in the chat)"));
    }
    for (k, v) in &c.settings {
        let secretish = k.contains("token");
        if secretish && !v.starts_with("env:") {
            return Err(bad(format!("{k} must be written as env:NAME (the name of an environment variable that holds it), never the secret itself")));
        }
    }
    Ok(())
}

async fn add<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Json(mut c): Json<ChannelConfig>) -> Result<(StatusCode, Json<Value>), ApiError> {
    c.enabled = false; // always off until enabled on purpose
    if c.mode.is_empty() {
        c.mode = "chat".into();
    }
    check_fields(&c)?;
    let mut all = load(&st.settings.store);
    if all.iter().any(|x| x.name == c.name) {
        return Err(ApiError(StatusCode::CONFLICT, format!("there is already a channel called {}", c.name)));
    }
    all.push(c.clone());
    save(&st.settings.store, &all);
    let _ = st.settings.store.log("channel", &c.name, "user", &json!({"op": "added (off)", "kind": c.kind, "allow": c.allow.len()}));
    Ok((StatusCode::CREATED, Json(json!({"channel": shown(&c, &st.settings.store), "warning": warning(&c.kind), "note": "it is off; read the warning, then enable it"}))))
}

#[derive(Deserialize)]
struct Edit {
    allow: Option<Vec<String>>,
    mode: Option<String>,
    model: Option<String>,
    settings: Option<std::collections::BTreeMap<String, String>>,
}

async fn edit<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(name): Path<String>, Json(b): Json<Edit>) -> Result<Json<Value>, ApiError> {
    let mut all = load(&st.settings.store);
    let c = all.iter_mut().find(|c| c.name == name).ok_or(ApiError(StatusCode::NOT_FOUND, "no such channel".into()))?;
    if let Some(a) = b.allow {
        c.allow = a;
    }
    if let Some(m) = b.mode {
        c.mode = m;
    }
    if let Some(m) = b.model {
        c.model = m;
    }
    if let Some(s) = b.settings {
        c.settings = s;
    }
    check_fields(c)?;
    let out = shown(c, &st.settings.store);
    save(&st.settings.store, &all);
    let _ = st.settings.store.log("channel", &name, "user", &json!({"op": "edited"}));
    Ok(Json(out))
}

#[derive(Deserialize, Default)]
struct Enable {
    /// The user has read the warning for this kind of channel.
    #[serde(default)]
    acknowledge: bool,
}

async fn enable<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(name): Path<String>, body: Option<Json<Enable>>) -> Result<Json<Value>, ApiError> {
    let mut all = load(&st.settings.store);
    let c = all.iter_mut().find(|c| c.name == name).ok_or(ApiError(StatusCode::NOT_FOUND, "no such channel".into()))?;
    if !body.is_some_and(|b| b.0.acknowledge) {
        return Err(bad(format!("read this first, then confirm: {}", warning(&c.kind))));
    }
    if c.allow.is_empty() {
        return Err(bad("nobody is on the allow-list; add the people who may talk to the assistant first"));
    }
    if c.model.is_empty() {
        return Err(bad("choose a model for this channel first"));
    }
    // fail now, with a reason, rather than quietly later
    build(c).map_err(|e| bad(e.to_string()))?;
    c.enabled = true;
    let out = shown(c, &st.settings.store);
    save(&st.settings.store, &all);
    let _ = st.settings.store.log("channel", &name, "user", &json!({"op": "enabled", "acknowledged_warning": true}));
    Ok(Json(out))
}

async fn disable<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(name): Path<String>) -> Result<Json<Value>, ApiError> {
    let mut all = load(&st.settings.store);
    let c = all.iter_mut().find(|c| c.name == name).ok_or(ApiError(StatusCode::NOT_FOUND, "no such channel".into()))?;
    c.enabled = false;
    let out = shown(c, &st.settings.store);
    save(&st.settings.store, &all);
    let _ = st.settings.store.log("channel", &name, "user", &json!({"op": "disabled"}));
    Ok(Json(out))
}

async fn remove<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(name): Path<String>) -> Result<StatusCode, ApiError> {
    let mut all = load(&st.settings.store);
    let n = all.len();
    all.retain(|c| c.name != name);
    if all.len() == n {
        return Err(ApiError(StatusCode::NOT_FOUND, "no such channel".into()));
    }
    save(&st.settings.store, &all);
    let _ = st.settings.store.log("channel", &name, "user", &json!({"op": "removed"}));
    Ok(StatusCode::NO_CONTENT)
}

// ---- carrying messages ----

pub(crate) struct Api {
    pub(crate) base: String,
    pub(crate) token: String,
    pub(crate) http: reqwest::Client,
}

impl Api {
    pub(crate) async fn call(&self, method: reqwest::Method, path: &str, body: Option<Value>) -> Result<Value, String> {
        let mut rb = self.http.request(method, format!("{}{path}", self.base)).bearer_auth(&self.token);
        if let Some(b) = body {
            rb = rb.json(&b);
        }
        let r = rb.send().await.map_err(|e| format!("the core is not reachable: {e}"))?;
        let ok = r.status().is_success();
        let v: Value = r.json().await.unwrap_or(Value::Null);
        if ok { Ok(v) } else { Err(v["error"].as_str().unwrap_or("the core refused").to_string()) }
    }

    /// Follow a server-sent stream, calling `on` for each event until the stream ends.
    pub(crate) async fn stream(&self, method: reqwest::Method, path: &str, body: Option<Value>, mut on: impl FnMut(Value)) -> Result<(), String> {
        let mut rb = self.http.request(method, format!("{}{path}", self.base)).bearer_auth(&self.token);
        if let Some(b) = body {
            rb = rb.json(&b);
        }
        let mut r = rb.send().await.map_err(|e| format!("the core is not reachable: {e}"))?;
        if !r.status().is_success() {
            let v: Value = r.json().await.unwrap_or(Value::Null);
            return Err(v["error"].as_str().unwrap_or("the core refused").to_string());
        }
        let mut p = SseParser::new();
        while let Some(chunk) = r.chunk().await.map_err(|e| format!("the connection to the core broke: {e}"))? {
            for d in p.push(&chunk) {
                if let Ok(v) = serde_json::from_str::<Value>(&d) {
                    on(v);
                }
            }
        }
        Ok(())
    }
}

/// A question waiting for a yes or no from the phone.
struct Pending {
    path: String,
    yes: Value,
    no: Value,
}

#[derive(Default)]
struct Chat {
    session: Option<String>,
    busy: bool,
    pending: VecDeque<Pending>,
    run: Option<String>,
}

struct Ctx {
    cfg: ChannelConfig,
    ch: Box<dyn Channel>,
    api: Api,
    store: Arc<Store>,
    chats: Mutex<HashMap<String, Chat>>,
}

/// Does the text look like it holds something sensitive? Words with a slash (file paths, links) are left out:
/// a long path is not a secret, and the check would otherwise hold back a plain permission question.
fn looks_sensitive(text: &str) -> bool {
    let kept: Vec<&str> = text.split_whitespace().filter(|w| !w.contains('/')).collect();
    lh_memory::detect(&kept.join(" ")).privacy == lh_memory::Privacy::Sensitive
}

fn set_status(store: &Store, name: &str, f: impl FnOnce(&mut Value)) {
    let mut v = status(store, name);
    if !v.is_object() {
        v = json!({});
    }
    f(&mut v);
    v["updated"] = json!(now());
    put_setting(store, &format!("channel_status:{name}"), &v.to_string());
}

fn bump(store: &Store, name: &str, key: &str) {
    set_status(store, name, |v| v[key] = json!(v[key].as_u64().unwrap_or(0) + 1));
}

impl Ctx {
    /// Send text to a chat. A reply that looks sensitive is held back; replies are logged by size only.
    async fn say(&self, chat: &str, text: &str) {
        let leaked = lh_memory::leaks(text, &crate::sensitive_texts(&self.store)).is_some();
        let (out, withheld) = if leaked || looks_sensitive(text) {
            ("I held back this reply because it looks like it contains sensitive information. Chat apps are not a safe place for that. You can see it in the terminal or the browser.".to_string(), true)
        } else {
            (text.to_string(), false)
        };
        let _ = self.store.log("channel_out", &self.cfg.name, "assistant", &json!({"kind": self.cfg.kind, "chars": out.chars().count(), "withheld": withheld}));
        for part in split_message(&out, REPLY_MAX) {
            if let Err(e) = self.ch.send(chat, &part).await {
                set_status(&self.store, &self.cfg.name, |v| v["last_error"] = json!(e.to_string()));
                return;
            }
        }
    }

    async fn ensure_session(&self, chat: &str) -> Result<String, String> {
        if let Some(s) = self.chats.lock().unwrap().get(chat).and_then(|c| c.session.clone()) {
            return Ok(s);
        }
        let v = self.api.call(reqwest::Method::POST, "/v1/sessions", Some(json!({"model": self.cfg.model, "mode": self.cfg.mode, "channel": true}))).await?;
        let id = v["id"].as_str().ok_or("the core sent no session id")?.to_string();
        self.chats.lock().unwrap().entry(chat.to_string()).or_default().session = Some(id.clone());
        Ok(id)
    }

    fn push_pending(&self, chat: &str, p: Pending) {
        self.chats.lock().unwrap().entry(chat.to_string()).or_default().pending.push_back(p);
    }

    async fn handle(self: Arc<Self>, m: Message) {
        // Only people on the list are answered. Anyone else is dropped: no reply, no wake, no text kept.
        if !self.cfg.allow.iter().any(|a| a.eq_ignore_ascii_case(&m.sender)) {
            bump(&self.store, &self.cfg.name, "ignored");
            return;
        }
        bump(&self.store, &self.cfg.name, "handled");
        let text: String = m.text.trim().chars().take(INPUT_MAX).collect();
        let _ = self.store.log("channel_in", &self.cfg.name, "user", &json!({"kind": self.cfg.kind, "chars": text.chars().count()}));
        let lower = text.to_lowercase();

        // an answer to a question the assistant asked
        let word = lower.trim_matches(|c: char| !c.is_alphanumeric());
        let yes = ["yes", "y", "ok", "okay", "allow", "approve", "go"].contains(&word);
        let no = ["no", "n", "deny", "stop", "reject"].contains(&word);
        if yes || no {
            let p = self.chats.lock().unwrap().get_mut(&m.chat).and_then(|c| c.pending.pop_front());
            if let Some(p) = p {
                let r = self.api.call(reqwest::Method::POST, &p.path, Some(if yes { p.yes } else { p.no })).await;
                self.say(&m.chat, if r.is_ok() { if yes { "Done: allowed." } else { "Done: not allowed." } } else { "That question has already ended." }).await;
                return;
            }
        }
        match lower.as_str() {
            "/help" | "help" => return self.say(&m.chat, "Write to me as usual. I ask yes or no when I need your permission. Commands: /new starts a fresh conversation, /status says what I am doing, /stop stops the current work, /run <department> <goal> gives a goal to a department.").await,
            "/new" => {
                self.chats.lock().unwrap().remove(&m.chat);
                return self.say(&m.chat, "Started a fresh conversation.").await;
            }
            "/status" => {
                let busy = self.chats.lock().unwrap().get(&m.chat).is_some_and(|c| c.busy);
                return self.say(&m.chat, if busy { "Working on your last message." } else { "Idle. Ready for your message." }).await;
            }
            "/stop" => {
                let (sid, run) = self.chats.lock().unwrap().get(&m.chat).map(|c| (c.session.clone(), c.run.clone())).unwrap_or((None, None));
                if let Some(s) = sid {
                    let _ = self.api.call(reqwest::Method::POST, &format!("/v1/sessions/{s}/cancel"), None).await;
                }
                if let Some(r) = run {
                    let _ = self.api.call(reqwest::Method::POST, &format!("/v1/team/runs/{r}/stop"), None).await;
                }
                if let Some(c) = self.chats.lock().unwrap().get_mut(&m.chat) {
                    c.pending.clear();
                }
                return self.say(&m.chat, "Stopped.").await;
            }
            _ => {}
        }
        if self.chats.lock().unwrap().get(&m.chat).is_some_and(|c| c.busy) {
            return self.say(&m.chat, "Still working on your last message. Answer yes or no if I asked something, or send /stop.").await;
        }
        self.chats.lock().unwrap().entry(m.chat.clone()).or_default().busy = true;
        if lower == "/run" {
            self.say(&m.chat, "Write it as: /run <department> <goal>").await;
        } else if let Some(rest) = text.strip_prefix("/run ") {
            self.clone().run_department(&m.chat, rest).await;
        } else {
            self.clone().chat_turn(&m.chat, &text).await;
        }
        if let Some(c) = self.chats.lock().unwrap().get_mut(&m.chat) {
            c.busy = false;
            c.pending.clear();
            c.run = None;
        }
    }

    async fn chat_turn(self: Arc<Self>, chat: &str, text: &str) {
        let sid = match self.ensure_session(chat).await {
            Ok(s) => s,
            Err(e) => return self.say(chat, &format!("I could not start a conversation: {e}")).await,
        };
        let (mut reply, mut problem) = (String::new(), None::<String>);
        // Questions must reach the phone while the turn waits for the answer, so they are sent from a
        // second task as they arrive instead of after the turn.
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let (me, chat_id) = (self.clone(), chat.to_string());
        let announcer = tokio::spawn(async move {
            while let Some(line) = rx.recv().await {
                me.say(&chat_id, &line).await;
            }
        });
        let r = self
            .api
            .stream(reqwest::Method::POST, &format!("/v1/sessions/{sid}/turns"), Some(json!({"text": text})), |ev| match ev["type"].as_str() {
                Some("text") => reply.push_str(ev["text"].as_str().unwrap_or("")),
                Some("ask") => {
                    self.push_pending(chat, Pending { path: format!("/v1/sessions/{sid}/answers/{}", ev["id"].as_str().unwrap_or("")), yes: json!({"answer": "yes"}), no: json!({"answer": "no"}) });
                    let who = ev["worker"].as_str().filter(|w| !w.is_empty()).unwrap_or("The assistant");
                    let _ = tx.send(format!("{who} wants to use {}: {}. Allow? Reply yes or no.", ev["tool"].as_str().unwrap_or(""), ev["what"].as_str().unwrap_or("")));
                }
                Some("error") | Some("refused") => problem = Some(ev["message"].as_str().unwrap_or("it stopped").to_string()),
                Some("step_limit") => problem = Some("it used all its steps without finishing".into()),
                _ => {}
            })
            .await;
        drop(tx);
        let _ = announcer.await;
        match (r, problem) {
            (Err(e), _) => {
                self.chats.lock().unwrap().remove(chat); // the session may be gone; start fresh next time
                self.say(chat, &format!("Something went wrong: {e}")).await
            }
            (_, Some(p)) if reply.trim().is_empty() => self.say(chat, &format!("It stopped: {p}")).await,
            (_, Some(p)) => self.say(chat, &format!("{}\n\n(It stopped: {p})", reply.trim())).await,
            _ if reply.trim().is_empty() => self.say(chat, "(No answer.)").await,
            _ => self.say(chat, reply.trim()).await,
        }
    }

    async fn run_department(self: Arc<Self>, chat: &str, rest: &str) {
        let Some((dept, goal)) = rest.trim().split_once(' ') else {
            return self.say(chat, "Write it as: /run <department> <goal>").await;
        };
        let run = match self.api.call(reqwest::Method::POST, "/v1/team/runs", Some(json!({"department": dept, "goal": goal.trim()}))).await {
            Ok(v) => v["id"].as_str().unwrap_or("").to_string(),
            Err(e) => return self.say(chat, &format!("I could not start that: {e}")).await,
        };
        if let Some(c) = self.chats.lock().unwrap().get_mut(chat) {
            c.run = Some(run.clone());
        }
        let (me, chat_id) = (self.clone(), chat.to_string());
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let out = tokio::spawn(async move {
            while let Some(line) = rx.recv().await {
                me.say(&chat_id, &line).await;
            }
        });
        let api_path = format!("/v1/team/runs/{run}/answers");
        let r = self
            .api
            .stream(reqwest::Method::GET, &format!("/v1/team/runs/{run}/events"), None, |ev| {
                let s = |k: &str| ev[k].as_str().unwrap_or("").to_string();
                let q = s("id");
                match ev["type"].as_str() {
                    Some("plan_needed") => {
                        let steps = ev["steps"].as_array().map(|a| a.iter().map(|x| format!("- {} ({})", x["title"].as_str().unwrap_or(""), x["assignee"].as_str().unwrap_or(""))).collect::<Vec<_>>().join("\n")).unwrap_or_default();
                        self.push_pending(chat, Pending { path: format!("{api_path}/{q}"), yes: json!({"decision": "approve"}), no: json!({"decision": "reject", "note": "not approved from the phone"}) });
                        let _ = tx.send(format!("Plan for {}:\n{steps}\nRun it? Reply yes or no.", s("dept")));
                    }
                    Some("checkin_needed") => {
                        self.push_pending(chat, Pending { path: format!("{api_path}/{q}"), yes: json!({"decision": "continue"}), no: json!({"decision": "stop"}) });
                        let _ = tx.send(format!("Milestone {} is done. Go on? Reply yes or no.", s("milestone")));
                    }
                    Some("ask") => {
                        self.push_pending(chat, Pending { path: format!("{api_path}/{q}"), yes: json!({"answer": "yes"}), no: json!({"answer": "no"}) });
                        let _ = tx.send(format!("{} wants to use {}: {}. Allow? Reply yes or no.", s("worker"), s("tool"), s("what")));
                    }
                    Some("step_failed") => {
                        let _ = tx.send(format!("A step failed: {}", s("report")));
                    }
                    Some("outcome") => {
                        let _ = tx.send(match ev["status"].as_str() {
                            Some("done") => ev["result"].as_str().unwrap_or("Done.").to_string(),
                            Some("rejected") => "Nothing ran: the plan was not approved.".into(),
                            Some("failed") => format!("It could not finish. {}", s("report")),
                            _ => "Stopped.".into(),
                        });
                    }
                    _ => {}
                }
            })
            .await;
        drop(tx);
        let _ = out.await;
        if let Err(e) = r {
            self.say(chat, &format!("Something went wrong: {e}")).await;
        }
    }
}

/// Keep the enabled channels running. Looks at the settings every few seconds, so enabling or disabling
/// takes effect without restarting the core.
pub fn spawn_runner(store: Arc<Store>, api_base: String, token: String) -> tokio::task::JoinHandle<()> {
    spawn_runner_every(store, api_base, token, Duration::from_secs(2))
}

/// The same, looking at the settings at the given interval.
pub fn spawn_runner_every(store: Arc<Store>, api_base: String, token: String, every: Duration) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut running: HashMap<String, (ChannelConfig, tokio::task::JoinHandle<()>)> = HashMap::new();
        loop {
            let want: HashMap<String, ChannelConfig> = load(&store).into_iter().filter(|c| c.enabled).map(|c| (c.name.clone(), c)).collect();
            running.retain(|name, (cfg, h)| {
                let keep = want.get(name) == Some(cfg) && !h.is_finished();
                if !keep {
                    h.abort();
                    set_status(&store, name, |v| v["running"] = json!(false));
                }
                keep
            });
            for (name, cfg) in want {
                if running.contains_key(&name) {
                    continue;
                }
                let (store2, base, tok) = (store.clone(), api_base.clone(), token.clone());
                let cfg2 = cfg.clone();
                let h = tokio::spawn(async move { run_channel(cfg2, store2, base, tok).await });
                running.insert(name, (cfg, h));
            }
            tokio::time::sleep(every).await;
        }
    })
}

async fn run_channel(cfg: ChannelConfig, store: Arc<Store>, base: String, token: String) {
    let name = cfg.name.clone();
    let ch = match build(&cfg) {
        Ok(c) => c,
        Err(e) => {
            set_status(&store, &name, |v| {
                v["running"] = json!(false);
                v["last_error"] = json!(e.to_string());
            });
            return;
        }
    };
    let ctx = Arc::new(Ctx { cfg, ch, api: Api { base, token, http: reqwest::Client::new() }, store: store.clone(), chats: Mutex::new(HashMap::new()) });
    set_status(&store, &name, |v| {
        v["running"] = json!(true);
        v["last_error"] = Value::Null;
    });
    let mut backoff = Duration::from_secs(1);
    loop {
        match ctx.ch.next().await {
            Ok(msgs) => {
                backoff = Duration::from_secs(1);
                set_status(&store, &name, |v| {
                    v["running"] = json!(true);
                    v["last_error"] = Value::Null;
                });
                for m in msgs {
                    tokio::spawn(ctx.clone().handle(m));
                }
            }
            Err(e) => {
                set_status(&store, &name, |v| v["last_error"] = json!(e.to_string()));
                tokio::time::sleep(if matches!(e, ChError::Auth(_)) { Duration::from_secs(60) } else { backoff }).await;
                backoff = (backoff * 2).min(Duration::from_secs(60));
            }
        }
    }
}
