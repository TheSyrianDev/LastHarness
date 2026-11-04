//! Stored chats over the local API: the list a chat sidebar shows, deleting a chat, and opening a chat
//! again, with another model if the user wants. Opening an old chat means the model reads the whole chat
//! again, so the user is told first: for a local model that the first reply is slow, for a cloud model that
//! it spends credits. A chat that repeats a sensitive note is never sent to a cloud model.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use lh_memory::{leaks, ConversationInfo, Trust};
use lh_provider::Message;
use lh_tools::Model;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{new_session, sensitive_texts, ApiError, AppState, NewSession};

pub(crate) fn routes<M: Model + 'static>() -> Router<Arc<AppState<M>>> {
    Router::new()
        .route("/v1/chats", get(list::<M>))
        .route("/v1/chats/{id}", get(one::<M>))
        .route("/v1/chats/{id}", delete(remove::<M>))
        .route("/v1/chats/{id}/resume", post(resume::<M>))
}

fn memory<M>(st: &AppState<M>) -> Result<&crate::memory_api::MemoryRuntime, ApiError> {
    st.settings.memory.as_ref().ok_or(ApiError(StatusCode::NOT_FOUND, "chats are not stored while memory is switched off".into()))
}

fn internal(e: lh_memory::Error) -> ApiError {
    ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

/// About 3.5 characters to a token, plus the lines the core adds before the chat (the base prompt and tool lines).
fn estimate_tokens(chars: usize) -> u64 {
    (chars as f64 / 3.5).ceil() as u64 + 250
}

fn info_json<M>(st: &AppState<M>, c: &ConversationInfo) -> Value {
    let live = st.sessions.lock().unwrap().get(&c.id).map(|s| s.known_tokens.load(Ordering::SeqCst));
    json!({
        "id": c.id, "title": c.title, "project": c.project, "mode": c.mode, "model": c.model,
        "started": c.started, "last": c.last, "messages": c.messages,
        "context_tokens": estimate_tokens(c.chars), "estimated": true,
        "live": live.is_some(),
    })
}

#[derive(Deserialize)]
struct ListQ {
    limit: Option<usize>,
}

async fn list<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Query(q): Query<ListQ>) -> Result<Json<Value>, ApiError> {
    let rows = memory(&st)?.memory.list_conversations(q.limit.unwrap_or(200).min(2000)).map_err(internal)?;
    Ok(Json(json!({"chats": rows.iter().map(|c| info_json(&st, c)).collect::<Vec<_>>()})))
}

async fn one<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    let rt = memory(&st)?;
    let c = rt.memory.conversation(&id).map_err(internal)?.ok_or(ApiError(StatusCode::NOT_FOUND, "no such chat".into()))?;
    let messages: Vec<Value> = rt.memory.messages(&id).map_err(internal)?.into_iter().map(|(role, content)| json!({"role": role, "content": content})).collect();
    let mut v = info_json(&st, &c);
    v["history"] = Value::Array(messages);
    Ok(Json(v))
}

async fn remove<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<String>) -> Result<StatusCode, ApiError> {
    let rt = memory(&st)?;
    let live = st.sessions.lock().unwrap().remove(&id).is_some();
    let stored = rt.memory.delete_conversation(&id).map_err(internal)?;
    // the model's saved work on this chat is private data too, so it goes with the chat
    if let Some(kv) = st.kv() {
        kv.forget(&id);
    }
    if !live && !stored {
        return Err(ApiError(StatusCode::NOT_FOUND, "no such chat".into()));
    }
    let _ = st.settings.store.log("chat_deleted", &id, "user", &json!({"live": live, "stored": stored}));
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct Resume {
    /// The model to continue with; the chat's own last model when left out.
    model: Option<String>,
    /// The user has read the warning.
    #[serde(default)]
    confirm: bool,
}

/// `price` is micro-USD per input token (that is, dollars per million tokens) when the user listed one for the model.
fn warning(cloud: bool, same_model: bool, tokens: u64, price: Option<f64>, cache_saved: bool) -> Value {
    let cost = price.filter(|_| cloud).map(|p| (tokens as f64 * p).round() as i64);
    let message = if cloud {
        match (cost, price) {
            (Some(c), Some(p)) => format!("This sends the whole chat again to the cloud model, about {tokens} tokens of context, and that spends credits: about {} at your listed price of ${p} per million input tokens.", lh_spend::dollars(c)),
            _ => format!("This sends the whole chat again to the cloud model, about {tokens} tokens of context, and that spends credits. No price is listed for this model, so the cost is not estimated."),
        }
    } else if cache_saved {
        format!("The model's work on this chat was saved, so it should carry on quickly. The chat is about {tokens} tokens of context.")
    } else {
        format!("The first reply will be slow: the model has to read the whole chat again, about {tokens} tokens of context.")
    };
    json!({"kind": if cloud { "cloud" } else { "local" }, "same_model": same_model, "context_tokens": tokens, "estimated": true, "estimated_cost_micro_usd": cost, "cache_saved": cache_saved, "price_per_million_usd": price.filter(|_| cloud), "message": message})
}

/// Is there a saved KV cache for this chat and model that a local endpoint can use?
fn saved_cache<M: lh_tools::Model + 'static>(st: &AppState<M>, chat: &str, model: &str, cloud: bool) -> bool {
    !cloud && st.kv().is_some_and(|kv| !kv.is_off() && kv.find(&crate::kv_cache::KvCache::file_name(chat, model)).is_some())
}

async fn resume<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<String>, Json(b): Json<Resume>) -> Result<Json<Value>, ApiError> {
    let rt = memory(&st)?;
    let c = rt.memory.conversation(&id).map_err(internal)?.ok_or(ApiError(StatusCode::NOT_FOUND, "no such chat".into()))?;
    let model = b.model.clone().or(c.model.clone()).ok_or(ApiError(StatusCode::BAD_REQUEST, "this chat has no recorded model; say which model to use".into()))?;
    let same_model = c.model.as_deref() == Some(model.as_str());
    let history = rt.memory.messages(&id).map_err(internal)?;
    let cloud = rt.trust != Trust::Local;
    let live = st.sessions.lock().unwrap().get(&id).cloned();

    // The same model in a session that is still loaded: nothing is read again.
    if let (Some(s), true) = (&live, same_model) {
        let tokens = s.known_tokens.load(Ordering::SeqCst).max(estimate_tokens(c.chars));
        return Ok(Json(json!({"session": id, "context_tokens": tokens, "resumed_messages": 0, "model": model, "live": true})));
    }
    let tokens = estimate_tokens(c.chars);
    if !b.confirm {
        return Ok(Json(json!({"needs_confirmation": true, "warning": warning(cloud, same_model, tokens, st.settings.spend.input_price(&model), saved_cache(&st, &id, &model, cloud)), "context_tokens": tokens, "model": model})));
    }
    // The privacy guard: what leaves the machine is checked the same way as any outbound text.
    if cloud {
        let sensitive = sensitive_texts(&st.settings.store);
        let hits = history.iter().filter(|(_, text)| leaks(text, &sensitive).is_some()).count();
        if hits > 0 {
            let _ = st.settings.store.log("chat_resume_refused", &id, "user", &json!({"model": model, "messages_with_sensitive_text": hits}));
            return Err(ApiError(StatusCode::CONFLICT, format!("{hits} message(s) in this chat repeat a sensitive note, so it cannot be sent to a cloud model. Continue it with a local model instead.")));
        }
    }
    let messages: Vec<Message> = crate::tree_api::history_to_messages(&history);
    if let Some(s) = live {
        // A loaded session moves to the new model: the first line of its prompt changes and the server reads it all again.
        let mut req = s.req.lock().await;
        req.model = model.clone();
        drop(req);
        s.known_tokens.store(0, Ordering::SeqCst);
        s.identity_stale.store(true, Ordering::SeqCst);
        let _ = rt.memory.set_conversation_model(&id, &model);
        let _ = st.settings.store.log("model_switched", &id, "user", &json!({"from": c.model, "to": model, "via": "resume"}));
        return Ok(Json(json!({"session": id, "context_tokens": tokens, "resumed_messages": 0, "model": model, "live": true})));
    }
    let project = c.project.clone().map(std::path::PathBuf::from).filter(|p| p.is_dir());
    let mode = if project.is_some() { None } else if c.mode == "assistant" { Some("assistant".to_string()) } else { Some("chat".to_string()) };
    if c.mode == "project" && project.is_none() {
        return Err(ApiError(StatusCode::CONFLICT, "this chat belongs to a project folder that no longer exists".into()));
    }
    let nb = NewSession { model: model.clone(), project_dir: project, mode, system: None, ask_all: false, no_sandbox: false, sampling: None, max_tokens: None, incognito: false, channel: false };
    let _ = new_session(&st, nb, Some(id.clone())).await?;
    let session = st.sessions.lock().unwrap().get(&id).cloned().ok_or(ApiError(StatusCode::INTERNAL_SERVER_ERROR, "the session was not created".into()))?;
    let n = messages.len();
    session.req.lock().await.messages.extend(messages);
    // the saved KV cache is put back before the first turn (it is skipped if it is for another model)
    if st.kv_for(&session).is_some() {
        session.kv_restore.store(true, Ordering::SeqCst);
    }
    let _ = rt.memory.set_conversation_model(&id, &model);
    let _ = st.settings.store.log("chat_resumed", &id, "user", &json!({"model": model, "messages": n, "cloud": cloud}));
    Ok(Json(json!({"session": id, "context_tokens": tokens, "resumed_messages": n, "model": model, "live": false})))
}
