use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use lh_memory::Destination;
use lh_provider::ChatRequest;
use lh_tools::{Answer, AskRequest, Asker, Permissions, ToolSet};
use serde_json::{json, Value};
use tokio::sync::{mpsc, oneshot};

/// How long a question waits for a human. No answer counts as no.
pub const ASK_TIMEOUT: Duration = Duration::from_secs(600);

pub struct Session {
    pub req: tokio::sync::Mutex<ChatRequest>,
    pub perms: Permissions,
    pub tools: ToolSet,
    pub busy: AtomicBool,
    /// Prompt plus reply size the server reported for the last model call; 0 if unknown.
    pub known_tokens: AtomicU64,
    /// A temporary session: nothing is remembered, stored or learned.
    pub incognito: bool,
    /// Where memory in this session may be sent. `None` when memory is off for the session.
    pub dest: Option<Destination>,
    /// The id of the stored conversation, if one is being kept.
    pub conv: Option<String>,
    /// Page versions already in this conversation's history, so notes are not added twice.
    pub injected: crate::memory_api::Injected,
    pending: Mutex<HashMap<String, oneshot::Sender<Answer>>>,
    next_ask: AtomicU64,
}

impl Session {
    pub fn new(req: ChatRequest, perms: Permissions, tools: ToolSet) -> Self {
        Self { req: tokio::sync::Mutex::new(req), perms, tools, busy: AtomicBool::new(false), known_tokens: AtomicU64::new(0), incognito: false, dest: None, conv: None, injected: Default::default(), pending: Mutex::new(HashMap::new()), next_ask: AtomicU64::new(1) }
    }

    pub fn try_start(&self) -> bool {
        self.busy.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok()
    }

    /// Called when a turn ends or is cancelled: unanswered questions fall through as "no".
    pub fn finish_turn(&self) {
        self.pending.lock().unwrap().clear();
        self.busy.store(false, Ordering::SeqCst);
    }

    pub fn answer(&self, ask_id: &str, a: Answer) -> bool {
        match self.pending.lock().unwrap().remove(ask_id) {
            Some(tx) => tx.send(a).is_ok(),
            None => false,
        }
    }
}

/// Sends a question to the client as an `ask` event and waits for the matching answer.
pub struct ApiAsker {
    pub session: Arc<Session>,
    pub events: mpsc::UnboundedSender<(String, Value)>,
}

impl Asker for ApiAsker {
    async fn ask(&self, r: &AskRequest<'_>) -> Answer {
        let id = format!("a{}", self.session.next_ask.fetch_add(1, Ordering::SeqCst));
        let (tx, rx) = oneshot::channel();
        self.session.pending.lock().unwrap().insert(id.clone(), tx);
        let sent = self.events.send(("ask".into(), json!({"type":"ask","id":id,"tool":r.tool,"what":r.what,"reason":r.reason,"worker":r.worker})));
        if sent.is_err() {
            return Answer::No;
        }
        match tokio::time::timeout(ASK_TIMEOUT, rx).await {
            Ok(Ok(a)) => a,
            _ => Answer::No,
        }
    }
}
