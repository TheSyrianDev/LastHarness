//! The chat tree over the local API. A stored chat is a tree of messages; the chat is at one of them and the model
//! reads only the path from the first message to there. These routes show the tree, move within it, name nodes,
//! make a new chat from an earlier point (fork) and copy the active branch (clone). Leaving a branch can leave a
//! short summary of it behind, written by the model, where the person came back to. Private chats store nothing,
//! so they have no tree.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use lh_memory::{leaks, Node, Trust};
use lh_provider::{ChatRequest, Message};
use lh_tools::{Meter, MeterVerdict, Model};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{sensitive_texts, ApiError, AppState};

pub(crate) fn routes<M: Model + 'static>() -> Router<Arc<AppState<M>>> {
    Router::new()
        .route("/v1/chats/{id}/tree", get(show::<M>))
        .route("/v1/chats/{id}/tree/move", post(go::<M>))
        .route("/v1/chats/{id}/tree/label", post(label::<M>))
        .route("/v1/chats/{id}/fork", post(fork::<M>))
        .route("/v1/chats/{id}/clone", post(clone::<M>))
}

/// What the model reads again for a stored branch. A summary of another path goes in as something the person said.
pub(crate) fn history_to_messages(history: &[(String, String)]) -> Vec<Message> {
    history
        .iter()
        .filter_map(|(role, text)| match role.as_str() {
            "user" => Some(Message::user(text.clone())),
            "assistant" => Some(Message::assistant(text.clone(), vec![])),
            "summary" => Some(Message::user(format!("[Summary of another path this conversation took, for context: {text}]"))),
            _ => None,
        })
        .collect()
}

fn memory<M>(st: &AppState<M>) -> Result<&crate::memory_api::MemoryRuntime, ApiError> {
    st.settings.memory.as_ref().ok_or(ApiError(StatusCode::NOT_FOUND, "chats are not stored while memory is switched off, so there is no tree".into()))
}

fn internal(e: lh_memory::Error) -> ApiError {
    match e {
        lh_memory::Error::NotFound(m) => ApiError(StatusCode::NOT_FOUND, m),
        other => ApiError(StatusCode::INTERNAL_SERVER_ERROR, other.to_string()),
    }
}

/// The chat must be a stored one; a private chat has nothing stored, so no tree.
fn stored<M>(st: &AppState<M>, id: &str) -> Result<lh_memory::ConversationInfo, ApiError> {
    memory(st)?.memory.conversation(id).map_err(internal)?.ok_or(ApiError(StatusCode::NOT_FOUND, "this chat is not stored (a private chat has no tree)".into()))
}

fn one_line(text: &str, n: usize) -> String {
    let t: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.chars().count() > n { format!("{}…", t.chars().take(n).collect::<String>()) } else { t }
}

fn node_json(n: &Node) -> Value {
    json!({"id": n.id, "parent": n.parent, "role": n.role, "preview": one_line(&n.content, 160), "chars": n.content.chars().count(), "label": n.label, "at": n.at})
}

async fn show<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    stored(&st, &id)?;
    let t = memory(&st)?.memory.tree(&id).map_err(internal)?;
    Ok(Json(json!({"leaf": t.leaf, "active": t.active_path(), "nodes": t.nodes.iter().map(node_json).collect::<Vec<_>>(), "summarize": st.tree_summarize()})))
}

#[derive(Deserialize)]
struct Go {
    /// The node to go to; 0 is the very start.
    node: i64,
    /// Leave a summary of the branch being left. When absent, the `tree.summarize` setting decides (ask, always, never).
    summarize: Option<bool>,
}

const SUMMARY_ASK: &str = "Write a short summary, at most 150 words, of this part of a conversation, so the person can go on from another point and still have what was useful in it: decisions, facts, names, results and what was left open. Write only the summary.";
const MAX_TRANSCRIPT: usize = 24_000;

fn transcript(leaving: &[Node]) -> String {
    let mut t = String::new();
    for n in leaving {
        let who = match n.role.as_str() {
            "user" => "Person",
            "assistant" => "Assistant",
            _ => "Earlier summary",
        };
        t.push_str(&format!("{who}: {}\n\n", n.content));
    }
    // keep the end if it is long: that is where the branch ended up
    let n = t.chars().count();
    if n > MAX_TRANSCRIPT { t.chars().skip(n - MAX_TRANSCRIPT).collect() } else { t }
}

async fn go<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<String>, Json(b): Json<Go>) -> Result<Json<Value>, ApiError> {
    let info = stored(&st, &id)?;
    let rt = memory(&st)?;
    let live = st.sessions.lock().unwrap().get(&id).cloned();
    if live.as_ref().is_some_and(|s| s.busy.load(Ordering::SeqCst)) {
        return Err(ApiError(StatusCode::CONFLICT, "a turn is running in this chat; stop it before moving in the tree".into()));
    }
    let plan = rt.memory.plan_move(&id, b.node).map_err(internal)?;
    let leaving = plan.leaving.len();
    let want = match (b.summarize, st.tree_summarize().as_str()) {
        (_, _) if leaving == 0 => false,
        (Some(w), _) => w,
        (None, "always") => true,
        (None, "never") => false,
        (None, _) => {
            // ask each time: nothing has changed yet
            return Ok(Json(json!({"needs_choice": true, "leaving_messages": leaving, "leaving_chars": plan.leaving.iter().map(|n| n.content.chars().count()).sum::<usize>(), "node": b.node})));
        }
    };
    let mut summary = None;
    if want {
        let model = match &live {
            Some(s) => s.req.lock().await.model.clone(),
            None => info.model.clone().ok_or(ApiError(StatusCode::BAD_REQUEST, "this chat has no recorded model to write the summary with".into()))?,
        };
        let text = transcript(&plan.leaving);
        // a cloud model is only sent what the privacy guard lets through, the same as any other outbound text
        if rt.trust != Trust::Local && leaks(&text, &sensitive_texts(&st.settings.store)).is_some() {
            return Err(ApiError(StatusCode::CONFLICT, "the branch you are leaving repeats a sensitive note, so it cannot be sent to a cloud model to be summarised. Move without a summary.".into()));
        }
        let req = ChatRequest { model, messages: vec![Message::user(format!("{SUMMARY_ASK}\n\n{text}"))], max_tokens: Some(3000), ..Default::default() };
        let meter = st.settings.spend.for_session(&id);
        match meter.before(&req).await {
            MeterVerdict::Allow => {}
            MeterVerdict::Ask(w) | MeterVerdict::Refuse(w) => return Err(ApiError(StatusCode::PAYMENT_REQUIRED, format!("the summary was not written: {w}"))),
        }
        let resp = st.model.complete(&req, &mut |_| {}).await.map_err(|e| ApiError(StatusCode::BAD_GATEWAY, format!("the summary could not be written: {}", e.explain())))?;
        meter.after(&req, resp.usage).await;
        let s = resp.text.trim().to_string();
        if s.is_empty() {
            return Err(ApiError(StatusCode::BAD_GATEWAY, "the model wrote no summary; move without one, or try again".into()));
        }
        summary = Some(s);
    }
    rt.memory.set_leaf(&id, plan.new_leaf).map_err(internal)?;
    if let Some(s) = &summary {
        rt.memory.append_message(&id, "summary", s).map_err(internal)?;
    }
    if let Some(s) = &live {
        sync_session(&st, &id, s).await?;
    }
    let _ = st.settings.store.log("chat_branch_moved", &id, "user", &json!({"to": b.node, "left_messages": leaving, "summary": summary.is_some()}));
    let t = rt.memory.tree(&id).map_err(internal)?;
    Ok(Json(json!({"leaf": t.leaf, "active": t.active_path(), "input": plan.input, "summary": summary, "left_messages": leaving})))
}

/// A loaded session follows the branch: its history becomes the active branch, the notes it had been given are
/// forgotten (they are no longer in its history), and the saved KV cache is dropped, because it describes a prefix
/// that is no longer the chat.
pub(crate) async fn sync_session<M: Model + 'static>(st: &AppState<M>, id: &str, s: &crate::session::Session) -> Result<(), ApiError> {
    let history = memory(st)?.memory.messages(id).map_err(internal)?;
    s.req.lock().await.messages = history_to_messages(&history);
    s.known_tokens.store(0, Ordering::SeqCst);
    s.injected.lock().unwrap().clear();
    s.identity_stale.store(true, Ordering::SeqCst);
    s.kv_restore.store(false, Ordering::SeqCst);
    if let Some(kv) = st.kv() {
        kv.forget(id);
    }
    Ok(())
}

#[derive(Deserialize)]
struct Label {
    node: i64,
    label: String,
}

async fn label<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<String>, Json(b): Json<Label>) -> Result<StatusCode, ApiError> {
    stored(&st, &id)?;
    memory(&st)?.memory.set_label(&id, b.node, &b.label).map_err(internal)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct Fork {
    node: i64,
}

async fn fork<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<String>, Json(b): Json<Fork>) -> Result<Json<Value>, ApiError> {
    stored(&st, &id)?;
    let new_id = format!("s{}", st.next.fetch_add(1, Ordering::SeqCst));
    let input = memory(&st)?.memory.fork(&id, b.node, &new_id).map_err(internal)?;
    let _ = st.settings.store.log("chat_forked", &new_id, "user", &json!({"from": id, "node": b.node}));
    Ok(Json(json!({"id": new_id, "input": input})))
}

async fn clone<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    stored(&st, &id)?;
    let new_id = format!("s{}", st.next.fetch_add(1, Ordering::SeqCst));
    let n = memory(&st)?.memory.clone_branch(&id, &new_id).map_err(internal)?;
    let _ = st.settings.store.log("chat_cloned", &new_id, "user", &json!({"from": id, "messages": n}));
    Ok(Json(json!({"id": new_id, "messages": n})))
}
