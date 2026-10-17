use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use lh_memory::Destination;
use lh_provider::ChatRequest;
use lh_tools::{Answer, AskRequest, Asker, Permissions, ToolSet};
use serde_json::{json, Value};
use tokio::sync::{mpsc, oneshot};

/// How long a question waits for a human. No answer counts as no.
pub const ASK_TIMEOUT: Duration = Duration::from_secs(600);

/// What the system message is made of. It is assembled on the first turn, when the model is loaded and
/// its name and size can be read, and again only if the user switches model.
pub struct SystemParts {
    pub kind: lh_tools::SessionKind,
    pub folder: Option<std::path::PathBuf>,
    pub guidelines: Vec<String>,
    pub client_block: String,
    pub learned_block: String,
    pub about: String,
}

pub struct Session {
    pub parts: SystemParts,
    /// True until the system message has been built for the current model.
    pub identity_stale: AtomicBool,
    /// The permission mode and sudo the system prompt last described; a change rebuilds the prompt.
    pub shown_perms: Mutex<Option<(lh_tools::Mode, bool)>>,
    /// The newest activity number when the last turn started: what "undo the last turn" goes back to.
    pub turn_start: std::sync::atomic::AtomicI64,
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
    /// The project folder, if the session has one.
    pub project: Option<String>,
    /// Tool calls made so far; a long task may be worth a skill.
    pub tool_calls: AtomicUsize,
    /// Skills the assistant has read in this session, which may then be improved.
    pub skills_used: Mutex<std::collections::HashSet<String>>,
    /// Page versions already in this conversation's history, so notes are not added twice.
    pub injected: crate::memory_api::Injected,
    pending: Mutex<HashMap<String, oneshot::Sender<Answer>>>,
    next_ask: AtomicU64,
    /// Where the running turn sends its events; department tools use it to ask the user.
    pub turn_events: Mutex<Option<mpsc::WeakUnboundedSender<(String, Value)>>>,
    /// Held while the model's KV cache is being saved or restored, so a turn never starts in the middle of it.
    pub kv_gate: Arc<tokio::sync::Mutex<()>>,
    /// The chat was opened again from storage: put its saved KV cache back before the first turn.
    pub kv_restore: AtomicBool,
}

impl Session {
    /// Has the permission mode or sudo changed since the system prompt last described them?
    pub fn perms_changed(&self) -> bool {
        *self.shown_perms.lock().unwrap() != Some((self.perms.effective_mode(), self.perms.sudo_allowed()))
    }

    pub fn new(req: ChatRequest, perms: Permissions, tools: ToolSet, parts: SystemParts) -> Self {
        Self { parts, identity_stale: AtomicBool::new(true), shown_perms: Mutex::new(None), turn_start: Default::default(), req: tokio::sync::Mutex::new(req), perms, tools, busy: AtomicBool::new(false), known_tokens: AtomicU64::new(0), incognito: false, dest: None, conv: None, project: None, tool_calls: AtomicUsize::new(0), skills_used: Default::default(), injected: Default::default(), pending: Mutex::new(HashMap::new()), next_ask: AtomicU64::new(1), turn_events: Mutex::new(None), kv_gate: Arc::new(tokio::sync::Mutex::new(())), kv_restore: AtomicBool::new(false) }
    }

    pub fn try_start(&self) -> bool {
        self.busy.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok()
    }

    /// Called when a turn ends or is cancelled: unanswered questions fall through as "no".
    pub fn finish_turn(&self) {
        *self.turn_events.lock().unwrap() = None;
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

impl Session {
    /// Ask the user through the running turn. `No` when no turn is running.
    pub async fn ask_user(self: &Arc<Self>, tool: &str, what: String, reason: String, worker: &str, detail: String) -> Answer {
        let events = self.turn_events.lock().unwrap().as_ref().and_then(|w| w.upgrade());
        match events {
            Some(events) => ApiAsker { session: self.clone(), events }.ask(&AskRequest { tool, what, reason, worker, detail }).await,
            None => Answer::No,
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
        let sent = self.events.send(("ask".into(), json!({"type":"ask","id":id,"tool":r.tool,"what":r.what,"reason":r.reason,"worker":r.worker,"detail":r.detail})));
        if sent.is_err() {
            return Answer::No;
        }
        match tokio::time::timeout(ASK_TIMEOUT, rx).await {
            Ok(Ok(a)) => a,
            _ => Answer::No,
        }
    }
}
