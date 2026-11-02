//! The link to the core. Everything that talks to `lhd` runs on a background thread with its own runtime;
//! the window sends commands in and receives events out, and never waits on the network.

use std::collections::HashMap;
use std::sync::Arc;

use futures_util::future::AbortHandle;
use lh_client::{Client, ServerEvent};
use serde_json::{json, Value};
use tokio::sync::mpsc;

use crate::config::Config;

/// A chat the core has stored, as the list shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct StoredChat {
    pub id: String,
    pub title: String,
    pub model: String,
    pub mode: String,
    pub messages: u64,
    pub context_tokens: u64,
    pub live: bool,
    pub last: i64,
    pub project: String,
}

impl StoredChat {
    pub fn from_json(v: &Value) -> Option<StoredChat> {
        Some(StoredChat {
            id: v["id"].as_str()?.to_string(),
            title: v["title"].as_str().unwrap_or("").to_string(),
            model: v["model"].as_str().unwrap_or("").to_string(),
            mode: v["mode"].as_str().unwrap_or("chat").to_string(),
            messages: v["messages"].as_u64().unwrap_or(0),
            context_tokens: v["context_tokens"].as_u64().unwrap_or(0),
            live: v["live"].as_bool().unwrap_or(false),
            last: v["last"].as_i64().unwrap_or(0),
            project: v["project"].as_str().unwrap_or("").to_string(),
        })
    }
}

/// How a new session is made (ignored when the chat already has one).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SessionStart {
    /// `chat`, `assistant` or `project`.
    pub mode: String,
    pub project_dir: Option<String>,
    /// Nothing is remembered or stored.
    pub incognito: bool,
}

/// What the window asks the core to do. `chat` is the window's own key for a chat.
#[derive(Debug)]
pub enum Cmd {
    /// Ask for the model list and the stored chats.
    Refresh,
    Send { chat: u64, session: Option<String>, session_model: Option<String>, model: String, text: String, start: SessionStart },
    Stop { chat: u64, session: Option<String> },
    Answer { session: String, ask: String, yes: bool },
    Delete { chat: u64, id: Option<String>, stored: bool },
    LoadHistory { chat: u64, id: String },
    Resume { chat: u64, id: String, model: Option<String>, confirm: bool },
    /// Any other call, for the side panels. The reply comes back under the same `tag`.
    Call { tag: String, method: String, path: String, body: Option<Value> },
    /// Start a department on a goal and follow it.
    StartRun { department: String, goal: String, project_dir: Option<String> },
    /// Follow a run that is already going (the log is replayed from the start).
    FollowRun { run: String, department: String, goal: String },
    AnswerRun { run: String, question: String, body: Value },
    StopRun { run: String },
}

/// What the core (or the link to it) tells the window.
#[derive(Debug)]
pub enum Ev {
    Models(Vec<String>),
    /// The same list with modes and load state, when the core gives them.
    ModelDetails(Vec<crate::models::ModelInfo>),
    Chats(Vec<StoredChat>),
    Connected(bool),
    SessionReady { chat: u64, session: String, model: String },
    Server { chat: u64, event: ServerEvent },
    /// The stream for this chat is over, however it ended.
    StreamEnded { chat: u64 },
    Failed { chat: u64, message: String },
    Deleted { chat: u64 },
    History { chat: u64, messages: Vec<(String, String)>, context_tokens: u64 },
    ResumeWarning { chat: u64, warning: Value, model: String },
    Resumed { chat: u64, session: String, model: String, context_tokens: u64 },
    Reply { tag: String, result: Result<Value, String> },
    RunStarted { run: String, department: String, goal: String },
    RunEvent { run: String, event: Value },
    RunEnded { run: String },
    /// The core named a chat (with `chats.title_model`), after its turn had already ended.
    Title { session: String, title: String },
}

pub struct Backend {
    tx: mpsc::UnboundedSender<Cmd>,
}

impl Backend {
    /// Start the background thread. Events go to `events` and `wake` is called after each, so the window
    /// can redraw.
    pub fn start(cfg: Config, events: futures_channel::UnboundedSender<Ev>) -> Backend {
        let (tx, rx) = mpsc::unbounded_channel();
        std::thread::Builder::new()
            .name("lh-backend".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().expect("runtime");
                rt.block_on(run(cfg, rx, events));
            })
            .expect("backend thread");
        Backend { tx }
    }

    pub fn send(&self, c: Cmd) {
        let _ = self.tx.send(c);
    }
}

pub mod futures_channel {
    //! A tiny unbounded channel the window can await on, so the backend never needs the window's types.
    use std::collections::VecDeque;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll, Waker};

    struct Shared<T> {
        queue: VecDeque<T>,
        waker: Option<Waker>,
        closed: bool,
    }

    pub struct UnboundedSender<T>(Arc<Mutex<Shared<T>>>);
    pub struct UnboundedReceiver<T>(Arc<Mutex<Shared<T>>>);

    impl<T> Clone for UnboundedSender<T> {
        fn clone(&self) -> Self {
            UnboundedSender(self.0.clone())
        }
    }

    pub fn unbounded<T>() -> (UnboundedSender<T>, UnboundedReceiver<T>) {
        let s = Arc::new(Mutex::new(Shared { queue: VecDeque::new(), waker: None, closed: false }));
        (UnboundedSender(s.clone()), UnboundedReceiver(s))
    }

    impl<T> UnboundedSender<T> {
        pub fn send(&self, v: T) -> bool {
            let mut s = self.0.lock().unwrap();
            if s.closed {
                return false;
            }
            s.queue.push_back(v);
            if let Some(w) = s.waker.take() {
                w.wake();
            }
            true
        }
    }

    impl<T> Drop for UnboundedReceiver<T> {
        fn drop(&mut self) {
            self.0.lock().unwrap().closed = true;
        }
    }

    impl<T> UnboundedReceiver<T> {
        pub fn recv(&mut self) -> Next<'_, T> {
            Next(self)
        }
    }

    pub struct Next<'a, T>(&'a mut UnboundedReceiver<T>);
    impl<T> Future for Next<'_, T> {
        type Output = T;
        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
            let mut s = (self.0).0.lock().unwrap();
            match s.queue.pop_front() {
                Some(v) => Poll::Ready(v),
                None => {
                    s.waker = Some(cx.waker().clone());
                    Poll::Pending
                }
            }
        }
    }
}

type Out = futures_channel::UnboundedSender<Ev>;

async fn run(cfg: Config, mut rx: mpsc::UnboundedReceiver<Cmd>, out: Out) {
    // a missing token file is not fatal: the core may start later, and Refresh tries again
    let client = Arc::new(loop_client(&cfg));
    let mut turns: HashMap<u64, AbortHandle> = HashMap::new();
    tokio::spawn(app_events(cfg.clone(), out.clone()));
    while let Some(cmd) = rx.recv().await {
        turns.retain(|_, h| !h.is_aborted());
        match cmd {
            Cmd::Refresh => {
                let (c, o, cfg2) = (client.clone(), out.clone(), cfg.clone());
                tokio::spawn(async move { refresh(&cfg2, c, o).await });
            }
            Cmd::Send { chat, session, session_model, model, text, start } => {
                let (c, o) = (client.clone(), out.clone());
                let (task, handle) = futures_util::future::abortable(send(c, o.clone(), chat, session, session_model, model, text, start));
                turns.insert(chat, handle);
                tokio::spawn(async move {
                    if task.await.is_err() {
                        // aborted by Stop; the window has already been told
                    }
                });
            }
            Cmd::Stop { chat, session } => {
                if let Some(h) = turns.remove(&chat) {
                    h.abort(); // dropping the stream is what stops the turn on the core
                }
                if let Some(id) = session {
                    let c = client.clone();
                    tokio::spawn(async move {
                        let _ = c.call("POST", &format!("/v1/sessions/{id}/cancel"), None).await;
                    });
                }
                out.send(Ev::StreamEnded { chat });
            }
            Cmd::Answer { session, ask, yes } => {
                let c = client.clone();
                tokio::spawn(async move {
                    let _ = c.answer(&session, &ask, yes).await;
                });
            }
            Cmd::Delete { chat, id, stored } => {
                if let Some(h) = turns.remove(&chat) {
                    h.abort();
                }
                let (c, o) = (client.clone(), out.clone());
                tokio::spawn(async move {
                    let result = match (&id, stored) {
                        (Some(id), true) => c.call("DELETE", &format!("/v1/chats/{id}"), None).await.map(|_| ()),
                        (Some(id), false) => c.call("DELETE", &format!("/v1/sessions/{id}"), None).await.map(|_| ()),
                        (None, _) => Ok(()),
                    };
                    match result {
                        Ok(()) => o.send(Ev::Deleted { chat }),
                        Err(message) => o.send(Ev::Failed { chat, message }),
                    };
                });
            }
            Cmd::LoadHistory { chat, id } => {
                let (c, o) = (client.clone(), out.clone());
                tokio::spawn(async move {
                    match c.call("GET", &format!("/v1/chats/{id}"), None).await {
                        Ok(v) => {
                            let messages = v["history"].as_array().map(|a| a.iter().map(|m| (m["role"].as_str().unwrap_or("").to_string(), m["content"].as_str().unwrap_or("").to_string())).collect()).unwrap_or_default();
                            o.send(Ev::History { chat, messages, context_tokens: v["context_tokens"].as_u64().unwrap_or(0) });
                        }
                        Err(message) => {
                            o.send(Ev::Failed { chat, message });
                        }
                    }
                });
            }
            Cmd::Resume { chat, id, model, confirm } => {
                let (c, o) = (client.clone(), out.clone());
                tokio::spawn(async move {
                    let mut body = json!({"confirm": confirm});
                    if let Some(m) = &model {
                        body["model"] = json!(m);
                    }
                    match c.call("POST", &format!("/v1/chats/{id}/resume"), Some(body)).await {
                        Ok(v) if v["needs_confirmation"].as_bool() == Some(true) => {
                            o.send(Ev::ResumeWarning { chat, warning: v["warning"].clone(), model: v["model"].as_str().unwrap_or("").to_string() });
                        }
                        Ok(v) => {
                            o.send(Ev::Resumed { chat, session: v["session"].as_str().unwrap_or(&id).to_string(), model: v["model"].as_str().unwrap_or("").to_string(), context_tokens: v["context_tokens"].as_u64().unwrap_or(0) });
                        }
                        Err(message) => {
                            o.send(Ev::Failed { chat, message });
                        }
                    }
                });
            }
            Cmd::StartRun { department, goal, project_dir } => {
                let (c, o) = (client.clone(), out.clone());
                tokio::spawn(async move {
                    let mut body = json!({"department": department, "goal": goal});
                    if let Some(p) = project_dir {
                        body["project_dir"] = json!(p);
                    }
                    match c.call("POST", "/v1/team/runs", Some(body)).await {
                        Ok(v) => match v["id"].as_str() {
                            Some(run) => {
                                o.send(Ev::RunStarted { run: run.to_string(), department: department.clone(), goal: goal.clone() });
                                follow(c, o, run.to_string()).await;
                            }
                            None => {
                                o.send(Ev::Reply { tag: "run".into(), result: Err("the core sent no run id".into()) });
                            }
                        },
                        Err(e) => {
                            o.send(Ev::Reply { tag: "run".into(), result: Err(e) });
                        }
                    }
                });
            }
            Cmd::FollowRun { run, department, goal } => {
                let (c, o) = (client.clone(), out.clone());
                tokio::spawn(async move {
                    o.send(Ev::RunStarted { run: run.clone(), department, goal });
                    follow(c, o, run).await;
                });
            }
            Cmd::AnswerRun { run, question, body } => {
                let c = client.clone();
                tokio::spawn(async move {
                    let _ = c.call("POST", &format!("/v1/team/runs/{run}/answers/{question}"), Some(body)).await;
                });
            }
            Cmd::StopRun { run } => {
                let c = client.clone();
                tokio::spawn(async move {
                    let _ = c.call("POST", &format!("/v1/team/runs/{run}/stop"), None).await;
                });
            }
            Cmd::Call { tag, method, path, body } => {
                let (c, o) = (client.clone(), out.clone());
                tokio::spawn(async move {
                    let result = c.call(&method, &path, body).await;
                    o.send(Ev::Reply { tag, result });
                });
            }
        }
    }
}

/// Stream a run's events until it ends.
async fn follow(client: Arc<Client>, out: Out, run: String) {
    match client.events(&format!("/v1/team/runs/{run}/events")).await {
        Ok(mut s) => {
            while let Ok(Some(event)) = s.next().await {
                out.send(Ev::RunEvent { run: run.clone(), event });
            }
        }
        Err(e) => {
            out.send(Ev::Reply { tag: "run".into(), result: Err(e) });
        }
    }
    out.send(Ev::RunEnded { run });
}

/// Listen to the core's app-wide events for as long as the app runs, coming back after the core restarts.
async fn app_events(cfg: Config, out: Out) {
    loop {
        if let Ok(mut stream) = loop_client(&cfg).events("/v1/events").await {
            while let Ok(Some(v)) = stream.next().await {
                if v["type"] == "title" {
                    let (session, title) = (v["session"].as_str().unwrap_or(""), v["title"].as_str().unwrap_or(""));
                    if !session.is_empty() && !title.is_empty() {
                        out.send(Ev::Title { session: session.into(), title: title.into() });
                    }
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
}

fn loop_client(cfg: &Config) -> Client {
    Client::from_token_file(&cfg.base, &cfg.token_file).unwrap_or_else(|_| Client::new(&cfg.base, ""))
}

async fn refresh(cfg: &Config, client: Arc<Client>, out: Out) {
    // the token file may have appeared since the app started
    let client = if cfg.token_file.exists() { Arc::new(loop_client(cfg)) } else { client };
    match client.models().await {
        Ok(m) => {
            out.send(Ev::Connected(true));
            out.send(Ev::Models(m));
            if let Ok(v) = client.call("GET", "/v1/models", None).await {
                let d: Vec<crate::models::ModelInfo> = v["details"].as_array().map(|a| a.iter().filter_map(crate::models::ModelInfo::from_json).collect()).unwrap_or_default();
                if !d.is_empty() {
                    out.send(Ev::ModelDetails(d));
                }
            }
        }
        Err(message) => {
            out.send(Ev::Connected(false));
            out.send(Ev::Failed { chat: 0, message });
            return;
        }
    }
    // chats may be off (memory switched off); that is not an error
    if let Ok(v) = client.call("GET", "/v1/chats", None).await {
        let list = v["chats"].as_array().map(|a| a.iter().filter_map(StoredChat::from_json).collect()).unwrap_or_default();
        out.send(Ev::Chats(list));
    }
}

#[allow(clippy::too_many_arguments)]
async fn send(client: Arc<Client>, out: Out, chat: u64, session: Option<String>, session_model: Option<String>, model: String, text: String, start: SessionStart) {
    let fail = |message: String| {
        out.send(Ev::Failed { chat, message });
        out.send(Ev::StreamEnded { chat });
    };
    let id = match session {
        Some(id) => {
            if session_model.as_deref().is_some_and(|m| m != model) {
                if let Err(e) = client.call("POST", &format!("/v1/sessions/{id}/model"), Some(json!({"model": model}))).await {
                    return fail(e);
                }
                out.send(Ev::SessionReady { chat, session: id.clone(), model: model.clone() });
            }
            id
        }
        None => match client.create_session_for(&model, &start).await {
            Ok(id) => {
                out.send(Ev::SessionReady { chat, session: id.clone(), model: model.clone() });
                id
            }
            Err(e) => return fail(e),
        },
    };
    let mut stream = match client.turn(&id, &text).await {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    loop {
        match stream.next().await {
            Ok(Some(event)) => {
                out.send(Ev::Server { chat, event });
            }
            Ok(None) => break,
            Err(e) => {
                out.send(Ev::Failed { chat, message: e });
                break;
            }
        }
    }
    out.send(Ev::StreamEnded { chat });
}

/// Session creation with the mode the chat was started in.
trait StartSession {
    async fn create_session_for(&self, model: &str, start: &SessionStart) -> Result<String, String>;
}
impl StartSession for Client {
    async fn create_session_for(&self, model: &str, start: &SessionStart) -> Result<String, String> {
        let mut body = json!({"model": model, "mode": if start.mode.is_empty() { "chat" } else { start.mode.as_str() }, "incognito": start.incognito});
        if let Some(d) = &start.project_dir {
            body["project_dir"] = json!(d);
        }
        let v = self.call("POST", "/v1/sessions", Some(body)).await?;
        v["id"].as_str().map(str::to_string).ok_or_else(|| "the core sent no session id".into())
    }
}
