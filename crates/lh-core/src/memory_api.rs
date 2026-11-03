//! Memory over the local API: the viewer (list, search, edit, archive, delete), suggestions from the
//! end-of-session review, the maintenance pass, import and export, and the record of what memory was
//! sent with requests.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use lh_memory::embed::AnyEmbedder;
use lh_memory::{contradiction_prompt, detect, excerpt, parse_contradiction, parse_judgement, parse_review, visible, Judgement, JUDGE_PROMPT, By, Classification, Destination, Edit, Layer, Memory, NewPage, NewProposal, Page, Privacy, Proposal, ProposalEdit, RecallConfig, Trust, REVIEW_PROMPT};
use lh_provider::{ChatRequest, Message};
use lh_tools::{Meter, MeterVerdict, Model};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{ApiError, AppState};

/// Memory as the core uses it.
pub struct MemoryRuntime {
    pub memory: Arc<Memory>,
    pub embedder: Arc<AnyEmbedder>,
    /// How far the session's model endpoint is trusted with memory.
    pub trust: Trust,
    pub cfg: RecallConfig,
}

impl MemoryRuntime {
    pub fn destination(&self, project: Option<String>) -> Destination {
        Destination { trust: self.trust, project }
    }
}

pub(crate) fn routes<M: Model + 'static>() -> Router<Arc<AppState<M>>> {
    Router::new()
        .route("/v1/memory/pages", get(list_pages::<M>).post(create_page::<M>))
        .route("/v1/memory/pages/{id}", get(get_page::<M>).patch(edit_page::<M>))
        .route("/v1/memory/pages/{id}", delete(delete_page::<M>))
        .route("/v1/memory/pages/{id}/archive", post(archive_page::<M>))
        .route("/v1/memory/pages/{id}/restore", post(restore_page::<M>))
        .route("/v1/memory/pages/{id}/pin", post(pin_page::<M>))
        .route("/v1/memory/pages/{id}/privacy", post(privacy_page::<M>))
        .route("/v1/memory/search", get(search::<M>))
        .route("/v1/memory/scan", post(scan::<M>))
        .route("/v1/memory/export", post(export::<M>))
        .route("/v1/memory/import", post(import::<M>))
        .route("/v1/memory/proposals", get(proposals::<M>))
        .route("/v1/memory/proposals/{id}/accept", post(accept::<M>))
        .route("/v1/memory/proposals/{id}/reject", post(reject::<M>))
        .route("/v1/memory/maintenance", post(maintenance::<M>))
        .route("/v1/memory/sent", get(sent::<M>))
        .route("/v1/memory/conversations", get(conversations::<M>))
        .route("/v1/sessions/{id}/review", post(review::<M>))
}

fn rt<M>(st: &AppState<M>) -> Result<&MemoryRuntime, ApiError> {
    st.settings.memory.as_ref().ok_or(ApiError(StatusCode::NOT_FOUND, "memory is switched off".into()))
}

fn err(e: lh_memory::Error) -> ApiError {
    let code = match e {
        lh_memory::Error::NotFound(_) => StatusCode::NOT_FOUND,
        lh_memory::Error::Invalid(_) => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    ApiError(code, e.to_string())
}

fn page_json(p: &Page) -> Value {
    json!({"id": p.id, "layer": p.layer.as_str(), "slug": p.slug, "title": p.title, "body": p.body, "gist": p.gist, "privacy": p.privacy.as_str(), "privacy_reason": p.privacy_reason,
        "pinned": p.pinned, "project": p.project, "source": p.source, "version": p.version, "archived": p.archived_at.is_some(), "created": p.created, "updated": p.updated})
}

fn proposal_json(p: &Proposal) -> Value {
    json!({"id": p.id, "kind": p.kind.as_str(), "layer": p.layer.as_str(), "title": p.title, "text": p.body, "gist": p.gist, "target": p.target_page, "privacy": p.privacy.as_str(), "privacy_reason": p.privacy_reason, "evidence": p.evidence, "conversation": p.conv, "status": p.status})
}

fn state_of(s: Option<&str>) -> lh_memory::State {
    match s {
        Some("archived") => lh_memory::State::Archived,
        Some("any") => lh_memory::State::Any,
        _ => lh_memory::State::Live,
    }
}

#[derive(Deserialize)]
struct ListQ {
    layer: Option<String>,
    state: Option<String>,
}

async fn list_pages<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Query(q): Query<ListQ>) -> Result<Json<Value>, ApiError> {
    let r = rt(&st)?;
    let pages = r.memory.list(q.layer.as_deref().and_then(Layer::parse), state_of(q.state.as_deref())).map_err(err)?;
    Ok(Json(json!({"pages": pages.iter().map(page_json).collect::<Vec<_>>()})))
}

async fn get_page<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    let r = rt(&st)?;
    let p = r.memory.get(&id).map_err(err)?;
    let h = r.memory.history(&id).map_err(err)?;
    Ok(Json(json!({"page": page_json(&p), "history": h.iter().map(|x| json!({"version": x.version, "op": x.op, "by": x.by, "at": x.at, "title": x.title, "body": x.body, "note": x.note})).collect::<Vec<_>>()})))
}

#[derive(Deserialize)]
struct NewPageBody {
    layer: String,
    title: String,
    text: String,
    gist: Option<String>,
    #[serde(default)]
    pinned: bool,
    privacy: Option<String>,
    project: Option<String>,
}

async fn create_page<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Json(b): Json<NewPageBody>) -> Result<Json<Value>, ApiError> {
    let r = rt(&st)?;
    let layer = Layer::parse(&b.layer).ok_or(ApiError(StatusCode::BAD_REQUEST, "layer must be preferences, facts or projects".into()))?;
    let class = detect(&format!("{} {}", b.title, b.text));
    let mut n = NewPage::new(layer, b.title, b.text);
    n.gist = b.gist.unwrap_or_default();
    n.pinned = b.pinned;
    n.project = b.project;
    (n.privacy, n.privacy_reason) = match b.privacy.as_deref().and_then(Privacy::parse) {
        Some(p) => (p, "set by the user".into()),
        None => (class.privacy, class.reason),
    };
    let p = r.memory.create(n, By::User).map_err(err)?;
    let (m, e) = (r.memory.clone(), r.embedder.clone());
    tokio::spawn(async move {
        let _ = m.index_pending(&*e).await;
    });
    Ok(Json(page_json(&p)))
}

#[derive(Deserialize)]
struct EditBody {
    title: Option<String>,
    text: Option<String>,
    gist: Option<String>,
}

async fn edit_page<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<String>, Json(b): Json<EditBody>) -> Result<Json<Value>, ApiError> {
    let r = rt(&st)?;
    let p = r.memory.update(&id, Edit { title: b.title, body: b.text, gist: b.gist }, By::User).map_err(err)?;
    let (m, e) = (r.memory.clone(), r.embedder.clone());
    tokio::spawn(async move {
        let _ = m.index_pending(&*e).await;
    });
    Ok(Json(page_json(&p)))
}

async fn archive_page<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    Ok(Json(page_json(&rt(&st)?.memory.archive(&id, By::User, "archived by the user").map_err(err)?)))
}
async fn restore_page<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    Ok(Json(page_json(&rt(&st)?.memory.restore(&id, By::User).map_err(err)?)))
}
async fn delete_page<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<String>) -> Result<StatusCode, ApiError> {
    rt(&st)?.memory.delete(&id, By::User).map_err(err)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct PinBody {
    pinned: bool,
}
async fn pin_page<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<String>, Json(b): Json<PinBody>) -> Result<Json<Value>, ApiError> {
    Ok(Json(page_json(&rt(&st)?.memory.set_pinned(&id, b.pinned, By::User).map_err(err)?)))
}

#[derive(Deserialize)]
struct PrivacyBody {
    privacy: String,
    reason: Option<String>,
}
async fn privacy_page<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<String>, Json(b): Json<PrivacyBody>) -> Result<Json<Value>, ApiError> {
    let p = Privacy::parse(&b.privacy).ok_or(ApiError(StatusCode::BAD_REQUEST, "privacy must be normal or sensitive".into()))?;
    Ok(Json(page_json(&rt(&st)?.memory.set_privacy(&id, p, b.reason.as_deref().unwrap_or("set by the user"), By::User).map_err(err)?)))
}

#[derive(Deserialize)]
struct SearchQ {
    q: String,
    state: Option<String>,
    limit: Option<usize>,
}

async fn search<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Query(q): Query<SearchQ>) -> Result<Json<Value>, ApiError> {
    let r = rt(&st)?;
    let limit = q.limit.unwrap_or(10).min(50);
    let state = state_of(q.state.as_deref());
    let mut found: Vec<(Page, f64)> = r.memory.search(&q.q, state, limit).map_err(err)?.into_iter().map(|h| (h.page, h.score)).collect();
    // the viewer is the owner at their own machine, so it also searches by meaning when that is set up
    if r.embedder.is_on() && state == lh_memory::State::Live {
        for (id, score) in r.memory.semantic_search(&*r.embedder, &q.q, limit).await.unwrap_or_default() {
            if !found.iter().any(|(p, _)| p.id == id) {
                if let Ok(p) = r.memory.get(&id) {
                    found.push((p, score));
                }
            }
        }
    }
    Ok(Json(json!({"results": found.iter().take(limit).map(|(p, s)| { let mut j = page_json(p); j["score"] = json!(s); j }).collect::<Vec<_>>()})))
}

async fn scan<M: Model + 'static>(State(st): State<Arc<AppState<M>>>) -> Result<Json<Value>, ApiError> {
    let r = rt(&st)?;
    let rep = r.memory.scan().map_err(err)?;
    Ok(Json(json!({"created": rep.created, "edited": rep.edited, "archived": rep.archived})))
}

#[derive(Deserialize)]
struct PathBody {
    path: PathBuf,
}

async fn export<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Json(b): Json<PathBody>) -> Result<StatusCode, ApiError> {
    if !b.path.is_absolute() {
        return Err(ApiError(StatusCode::BAD_REQUEST, "path must be absolute".into()));
    }
    rt(&st)?.memory.export(&b.path).map_err(err)?;
    let _ = st.settings.store.log("memory_export", "-", "user", &json!({"to": b.path}));
    Ok(StatusCode::NO_CONTENT)
}

async fn import<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Json(b): Json<PathBody>) -> Result<Json<Value>, ApiError> {
    if !b.path.is_absolute() {
        return Err(ApiError(StatusCode::BAD_REQUEST, "path must be absolute".into()));
    }
    let r = rt(&st)?.memory.import(&b.path).map_err(err)?;
    let _ = st.settings.store.log("memory_import", "-", "user", &json!({"from": b.path, "imported": r.imported}));
    Ok(Json(json!({"imported": r.imported, "already_present": r.already_present})))
}

#[derive(Deserialize)]
struct ProposalQ {
    status: Option<String>,
}

async fn proposals<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Query(q): Query<ProposalQ>) -> Result<Json<Value>, ApiError> {
    let list = rt(&st)?.memory.list_proposals(Some(q.status.as_deref().unwrap_or("pending"))).map_err(err)?;
    Ok(Json(json!({"proposals": list.iter().map(proposal_json).collect::<Vec<_>>()})))
}

#[derive(Deserialize, Default)]
struct AcceptBody {
    title: Option<String>,
    text: Option<String>,
    gist: Option<String>,
    privacy: Option<String>,
}

async fn accept<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<i64>, body: Option<Json<AcceptBody>>) -> Result<Json<Value>, ApiError> {
    let r = rt(&st)?;
    let b = body.map(|j| j.0).unwrap_or_default();
    let privacy = match b.privacy.as_deref() {
        Some(s) => Some(Privacy::parse(s).ok_or(ApiError(StatusCode::BAD_REQUEST, "privacy must be normal or sensitive".into()))?),
        None => None,
    };
    let page = r.memory.accept_proposal(id, ProposalEdit { title: b.title, body: b.text, gist: b.gist, privacy }, None).map_err(err)?;
    crate::rewards::reward(&st.settings.store, "-", "proposal", 1.0, false);
    let (m, e) = (r.memory.clone(), r.embedder.clone());
    tokio::spawn(async move {
        let _ = m.index_pending(&*e).await;
    });
    Ok(Json(page_json(&page)))
}

async fn reject<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<i64>) -> Result<StatusCode, ApiError> {
    rt(&st)?.memory.reject_proposal(id).map_err(err)?;
    crate::rewards::reward(&st.settings.store, "-", "proposal", -1.0, false);
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize, Default)]
struct MaintBody {
    /// Ask the model about pairs of notes that might disagree. Needs a model on this machine.
    #[serde(default)]
    contradictions: bool,
    model: Option<String>,
}

async fn maintenance<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, body: Option<Json<MaintBody>>) -> Result<Json<Value>, ApiError> {
    let r = rt(&st)?;
    let b = body.map(|j| j.0).unwrap_or_default();
    let mut findings: Vec<Value> = r
        .memory
        .maintenance()
        .map_err(err)?
        .iter()
        .map(|f| json!({"kind": format!("{:?}", f.kind).to_lowercase(), "pages": f.pages, "detail": f.detail}))
        .collect();
    if b.contradictions {
        let model = b.model.ok_or(ApiError(StatusCode::BAD_REQUEST, "give a model to ask".into()))?;
        if r.trust != Trust::Local {
            return Err(ApiError(StatusCode::BAD_REQUEST, "the contradiction check reads your notes, so it only runs on a model on this machine".into()));
        }
        for (a, b, why) in check_contradictions(&st, &model).await {
            findings.push(json!({"kind": "contradiction", "pages": [a, b], "detail": why}));
        }
    }
    let pending = r.memory.list_proposals(Some("pending")).map_err(err)?.len();
    Ok(Json(json!({"findings": findings, "pending_suggestions": pending})))
}

/// Ask `model` about pairs of notes that may disagree. For each pair it says disagree, the older page gets a
/// suggestion to archive it, which the user accepts or rejects; nothing is changed here. Pairs already asked about
/// (same versions) are not asked again, and a page whose archive suggestion was rejected is not suggested again.
/// Returns (older page, newer page, reason). Only for memory kept on this machine: the notes go to the model.
pub(crate) async fn check_contradictions<M: Model + 'static>(st: &AppState<M>, model: &str) -> Vec<(String, String, String)> {
    let Some(r) = &st.settings.memory else { return vec![] };
    if r.trust != Trust::Local {
        return vec![];
    }
    let Ok(pairs) = r.memory.contradiction_candidates(5) else { return vec![] };
    let meter = st.settings.spend.for_session("maintenance");
    let mut out = vec![];
    for (a, b) in pairs {
        let key = (a.id.clone(), a.version, b.id.clone(), b.version);
        if st.checked_pairs.lock().unwrap().contains(&key) {
            continue;
        }
        let req = ChatRequest { model: model.to_string(), messages: vec![Message::user(contradiction_prompt(&a, &b))], max_tokens: Some(1000), ..Default::default() };
        if meter.before(&req).await != MeterVerdict::Allow {
            break;
        }
        let Ok(resp) = st.model.complete(&req, &mut |_| {}).await else { break };
        meter.after(&req, resp.usage).await;
        st.checked_pairs.lock().unwrap().insert(key);
        let Some(why) = parse_contradiction(&resp.text) else { continue };
        let (older, newer) = if a.updated <= b.updated { (&a, &b) } else { (&b, &a) };
        let evidence = format!("disagrees with the newer note \"{}\"{}", newer.title, if why.is_empty() { String::new() } else { format!(": {why}") });
        let refused_before = r.memory.list_proposals(None).unwrap_or_default().iter().any(|p| p.kind == lh_memory::Kind::Archive && p.target_page.as_deref() == Some(older.id.as_str()) && p.status == "rejected");
        if !older.pinned && !refused_before {
            let _ = r.memory.add_proposal(NewProposal {
                kind: lh_memory::Kind::Archive,
                layer: older.layer,
                title: older.title.clone(),
                body: older.body.clone(),
                gist: older.gist.clone(),
                target_page: Some(older.id.clone()),
                privacy: older.privacy,
                privacy_reason: older.privacy_reason.clone(),
                evidence,
                conv: None,
                project: older.project.clone(),
            });
        }
        out.push((older.id.clone(), newer.id.clone(), why));
    }
    out
}

#[derive(Deserialize)]
struct SentQ {
    limit: Option<u32>,
}

/// Exactly which notes went into which requests, newest first.
async fn sent<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Query(q): Query<SentQ>) -> Result<Json<Value>, ApiError> {
    rt(&st)?;
    let rows = st.settings.store.recent(q.limit.unwrap_or(50).min(500), Some("memory_sent")).map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({"sent": rows.iter().map(|e| json!({"at": e.at, "session": e.session, "detail": e.detail})).collect::<Vec<_>>()})))
}

#[derive(Deserialize)]
struct ConvQ {
    q: String,
}

async fn conversations<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Query(q): Query<ConvQ>) -> Result<Json<Value>, ApiError> {
    let hits = rt(&st)?.memory.search_conversations(&q.q, 20).map_err(err)?;
    Ok(Json(json!({"messages": hits.iter().map(|h| json!({"conversation": h.conv, "role": h.role, "text": h.content, "at": h.at})).collect::<Vec<_>>()})))
}

/// The end-of-session review. The question is the last turn of the conversation as it stands, with
/// the same tools listed, so a server with prefix caching reads the long part from its cache.
async fn review<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    let r = rt(&st)?;
    let s = st.session(&id)?;
    if s.incognito {
        return Err(ApiError(StatusCode::BAD_REQUEST, "an incognito session stores and learns nothing, so there is nothing to review".into()));
    }
    if !s.try_start() {
        return Err(ApiError(StatusCode::CONFLICT, "a turn is running in this session".into()));
    }
    let result = review_inner(&st, r, &s, &id).await;
    s.finish_turn();
    result
}

async fn review_inner<M: Model + 'static>(st: &Arc<AppState<M>>, r: &MemoryRuntime, s: &Arc<crate::session::Session>, id: &str) -> Result<Json<Value>, ApiError> {
    let dest = s.dest.clone().unwrap_or_else(|| r.destination(None));
    let mut req = s.req.lock().await.clone();
    if !req.messages.iter().any(|m| m.role == lh_provider::Role::User) {
        return Err(ApiError(StatusCode::BAD_REQUEST, "nothing was said yet".into()));
    }
    // existing notes the model may see, so it can propose updates; sensitive ones stay out for cloud models
    let known: Vec<Page> = r.memory.list(None, lh_memory::State::Live).map_err(err)?.into_iter().filter(|p| visible(p, &dest)).collect();
    let mut prompt = REVIEW_PROMPT.to_string();
    prompt.push_str("\n\nNotes already known:\n");
    if known.is_empty() {
        prompt.push_str("(none)\n");
    }
    for p in &known {
        prompt.push_str(&format!("- [{}] {}: {}\n", p.id, p.title, if p.gist.is_empty() { excerpt(&p.body, 100) } else { p.gist.clone() }));
    }
    req.messages.push(Message::user(prompt));
    req.max_tokens = Some(req.max_tokens.unwrap_or(4096).max(4096)); // a thinking model may use much of it before it answers
    let meter = st.settings.spend.for_session(id);
    match meter.before(&req).await {
        MeterVerdict::Allow => {}
        MeterVerdict::Ask(w) | MeterVerdict::Refuse(w) => return Err(ApiError(StatusCode::PAYMENT_REQUIRED, format!("the review was not run: {w}"))),
    }
    let resp = st.model.complete(&req, &mut |_| {}).await.map_err(|e| ApiError(StatusCode::BAD_GATEWAY, e.explain()))?;
    meter.after(&req, resp.usage).await;

    let (mut added, mut skipped) = (vec![], 0);
    for p in parse_review(&resp.text) {
        let target = match &p.target {
            Some(t) => match known.iter().find(|k| &k.id == t) {
                Some(k) => Some(k.clone()),
                None => {
                    skipped += 1; // an id the model made up, or a note it may not see
                    continue;
                }
            },
            None => None,
        };
        let mut class = detect(&format!("{} {}", p.title, p.text));
        if let Some(t) = &target {
            if t.privacy == Privacy::Sensitive && class.privacy == Privacy::Normal {
                class = Classification { privacy: Privacy::Sensitive, reason: "the note it changes is sensitive".into() };
            }
        }
        // Patterns passed it. On a local model, ask that model too; unsure or unreachable counts as sensitive.
        if class.privacy == Privacy::Normal && r.trust == Trust::Local {
            let mut jreq = ChatRequest { model: req.model.clone(), messages: vec![Message::user(format!("{JUDGE_PROMPT}{}\n{}", p.title, p.text))], max_tokens: Some(1500), ..Default::default() };
            jreq.sampling.extra.insert("chat_template_kwargs".into(), json!({"enable_thinking": false}));
            class = match meter.before(&jreq).await {
                MeterVerdict::Allow => match st.model.complete(&jreq, &mut |_| {}).await {
                    Ok(resp) => {
                        meter.after(&jreq, resp.usage).await;
                        match parse_judgement(&resp.text) {
                            Judgement::Normal => class,
                            Judgement::Sensitive(why) => Classification { privacy: Privacy::Sensitive, reason: format!("judged sensitive: {why}") },
                            Judgement::Unsure => Classification { privacy: Privacy::Sensitive, reason: "the model was unsure".into() },
                        }
                    }
                    Err(_) => Classification { privacy: Privacy::Sensitive, reason: "could not be checked".into() },
                },
                _ => Classification { privacy: Privacy::Sensitive, reason: "could not be checked".into() },
            };
        }
        let gist = if p.gist.is_empty() { excerpt(&p.text, 120) } else { p.gist.clone() };
        let np = NewProposal { kind: p.kind, layer: p.layer, title: p.title, body: p.text, gist, target_page: p.target, privacy: class.privacy, privacy_reason: class.reason, evidence: p.evidence, conv: s.conv.clone(), project: None };
        if let Some(pid) = r.memory.add_proposal(np).map_err(err)? {
            added.push(pid);
        }
    }
    let _ = st.settings.store.log("memory_review", id, "assistant", &json!({"suggestions": added.len(), "skipped": skipped}));
    let list: Vec<Value> = r.memory.list_proposals(Some("pending")).map_err(err)?.iter().filter(|p| added.contains(&p.id)).map(proposal_json).collect();
    Ok(Json(json!({"proposals": list, "skipped": skipped})))
}

/// Used by the session code: notes already carried by a conversation.
pub(crate) type Injected = std::sync::Mutex<HashSet<(String, i64)>>;
