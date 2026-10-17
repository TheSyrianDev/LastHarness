//! Learning over the local API: rules, the style profile, skills, suggestions, and the learn pass that
//! runs at the end of a session. Suggestions wait for the user unless silent learning is on.

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use lh_learn::{habit_prompt, parse_drafts, parse_skill_change, parse_skill_draft, session_prompt, skill_improve_prompt, skill_prompt, Applied, Kind, Learn, NewRule, Proposal, Rule, RuleEdit, SignalKind, Status};
use lh_memory::{Privacy, Trust};
use lh_provider::{ChatRequest, Message, Role};
use lh_tools::{Meter, MeterVerdict, Model};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::session::Session;
use crate::{ApiError, AppState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LearnMode {
    /// Suggestions wait for the user. The default.
    Propose,
    /// Suggestions are applied at once and logged; every one can still be rolled back.
    Silent,
}

/// Learning as the core uses it.
pub struct LearnRuntime {
    pub learn: Arc<Learn>,
    /// How far the model endpoint is trusted; sensitive rules and signals stay away from clouds.
    pub trust: Trust,
    pub mode: LearnMode,
    /// A wish seen in this many different sessions is a habit.
    pub threshold: usize,
    /// A session with at least this many tool calls is considered for a skill suggestion.
    pub skill_min_calls: usize,
}

pub(crate) fn routes<M: Model + 'static>() -> Router<Arc<AppState<M>>> {
    Router::new()
        .route("/v1/learn/rules", get(list_rules::<M>).post(add_rule::<M>))
        .route("/v1/learn/rules/{id}", get(get_rule::<M>).patch(edit_rule::<M>).delete(delete_rule::<M>))
        .route("/v1/learn/rules/{id}/status", post(rule_status::<M>))
        .route("/v1/learn/rules/{id}/rollback", post(rule_rollback::<M>))
        .route("/v1/learn/style", get(get_style::<M>).put(put_style::<M>))
        .route("/v1/learn/style/rollback", post(style_rollback::<M>))
        .route("/v1/learn/skills", get(list_skills::<M>).post(add_skill::<M>))
        .route("/v1/learn/skills/{name}", get(get_skill::<M>).delete(delete_skill::<M>))
        .route("/v1/learn/skills/{name}/rollback", post(skill_rollback::<M>))
        .route("/v1/learn/proposals", get(proposals::<M>))
        .route("/v1/learn/proposals/{id}/accept", post(accept::<M>))
        .route("/v1/learn/proposals/{id}/reject", post(reject::<M>))
        .route("/v1/learn/signals", get(signals::<M>))
        .route("/v1/learn/habits", get(habits::<M>))
        .route("/v1/sessions/{id}/learn", post(learn_pass::<M>))
        .route("/v1/sessions/{id}/feedback", post(feedback::<M>))
}

fn rt<M>(st: &AppState<M>) -> Result<&LearnRuntime, ApiError> {
    st.settings.learn.as_ref().ok_or(ApiError(StatusCode::NOT_FOUND, "learning is switched off".into()))
}

fn err(e: lh_learn::Error) -> ApiError {
    let code = match e {
        lh_learn::Error::NotFound(_) => StatusCode::NOT_FOUND,
        lh_learn::Error::Invalid(_) => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    ApiError(code, e.to_string())
}

fn rule_json(r: &Rule) -> Value {
    json!({"id": r.id, "topic": r.topic, "text": r.text, "project": r.project, "role": r.role, "privacy": r.privacy.as_str(), "active": r.status == Status::Active,
        "evidence": r.evidence, "source": r.source, "version": r.version, "created": r.created, "updated": r.updated})
}

fn proposal_json(p: &Proposal) -> Value {
    json!({"id": p.id, "kind": p.kind.as_str(), "payload": p.payload, "evidence": p.evidence, "source": p.source, "status": p.status, "session": p.session})
}

fn history_json(h: &[lh_learn::HistoryEntry]) -> Value {
    json!(h.iter().map(|x| json!({"version": x.version, "op": x.op, "by": x.by, "at": x.at, "content": x.content, "note": x.note})).collect::<Vec<_>>())
}

// ---- rules ----

#[derive(Deserialize)]
struct RulesQ {
    all: Option<bool>,
}

async fn list_rules<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Query(q): Query<RulesQ>) -> Result<Json<Value>, ApiError> {
    let rules = rt(&st)?.learn.list_rules(q.all.unwrap_or(false)).map_err(err)?;
    Ok(Json(json!({"rules": rules.iter().map(rule_json).collect::<Vec<_>>()})))
}

#[derive(Deserialize)]
struct NewRuleBody {
    text: String,
    topic: Option<String>,
    project: Option<String>,
    role: Option<String>,
}

async fn add_rule<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Json(b): Json<NewRuleBody>) -> Result<Json<Value>, ApiError> {
    let r = rt(&st)?;
    let class = lh_memory::detect(&b.text);
    let rule = r.learn.add_rule(NewRule { topic: b.topic.unwrap_or_default(), text: b.text, project: b.project, role: b.role, privacy: class.privacy, evidence: vec![], source: "user".into() }, "user").map_err(err)?;
    Ok(Json(rule_json(&rule)))
}

async fn get_rule<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    let r = rt(&st)?;
    Ok(Json(json!({"rule": rule_json(&r.learn.get_rule(&id).map_err(err)?), "history": history_json(&r.learn.history("rule", &id).map_err(err)?)})))
}

#[derive(Deserialize)]
struct EditRuleBody {
    text: Option<String>,
    topic: Option<String>,
    privacy: Option<String>,
}

async fn edit_rule<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<String>, Json(b): Json<EditRuleBody>) -> Result<Json<Value>, ApiError> {
    let privacy = match b.privacy.as_deref() {
        Some(p) => Some(Privacy::parse(p).ok_or(ApiError(StatusCode::BAD_REQUEST, "privacy must be normal or sensitive".into()))?),
        None => None,
    };
    Ok(Json(rule_json(&rt(&st)?.learn.edit_rule(&id, RuleEdit { text: b.text, topic: b.topic, privacy, ..Default::default() }, "user").map_err(err)?)))
}

#[derive(Deserialize)]
struct StatusBody {
    active: bool,
}
async fn rule_status<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<String>, Json(b): Json<StatusBody>) -> Result<Json<Value>, ApiError> {
    Ok(Json(rule_json(&rt(&st)?.learn.set_rule_status(&id, if b.active { Status::Active } else { Status::Disabled }, "user").map_err(err)?)))
}

#[derive(Deserialize)]
struct VersionBody {
    version: i64,
}
async fn rule_rollback<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<String>, Json(b): Json<VersionBody>) -> Result<Json<Value>, ApiError> {
    Ok(Json(rule_json(&rt(&st)?.learn.rollback_rule(&id, b.version, "user").map_err(err)?)))
}

async fn delete_rule<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<String>) -> Result<StatusCode, ApiError> {
    rt(&st)?.learn.delete_rule(&id, "user").map_err(err)?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- style ----

async fn get_style<M: Model + 'static>(State(st): State<Arc<AppState<M>>>) -> Result<Json<Value>, ApiError> {
    let r = rt(&st)?;
    let s = r.learn.style().map_err(err)?;
    Ok(Json(json!({"style": s.as_ref().map(|s| json!({"text": s.text, "version": s.version, "updated": s.updated})), "history": history_json(&r.learn.history("style", "style").map_err(err)?)})))
}

#[derive(Deserialize)]
struct StyleBody {
    text: String,
}
async fn put_style<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Json(b): Json<StyleBody>) -> Result<Json<Value>, ApiError> {
    let s = rt(&st)?.learn.set_style(&b.text, "user").map_err(err)?;
    Ok(Json(json!({"text": s.text, "version": s.version})))
}
async fn style_rollback<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Json(b): Json<VersionBody>) -> Result<Json<Value>, ApiError> {
    let s = rt(&st)?.learn.rollback_style(b.version, "user").map_err(err)?;
    Ok(Json(json!({"text": s.text, "version": s.version})))
}

// ---- skills ----

async fn list_skills<M: Model + 'static>(State(st): State<Arc<AppState<M>>>) -> Result<Json<Value>, ApiError> {
    let l = rt(&st)?.learn.list_skills().map_err(err)?;
    Ok(Json(json!({"skills": l.iter().map(|s| json!({"name": s.name, "description": s.description, "error": s.error})).collect::<Vec<_>>()})))
}

async fn get_skill<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(name): Path<String>) -> Result<Json<Value>, ApiError> {
    let r = rt(&st)?;
    let s = r.learn.load_skill(&name).map_err(err)?;
    Ok(Json(json!({"name": s.meta.name, "description": s.meta.description, "license": s.meta.license, "compatibility": s.meta.compatibility, "body": s.body, "files": s.files,
        "history": history_json(&r.learn.history("skill", &name).map_err(err)?)})))
}

#[derive(Deserialize)]
struct SkillBody {
    /// Relative path to text; must hold `SKILL.md`.
    files: std::collections::BTreeMap<String, String>,
}
async fn add_skill<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Json(b): Json<SkillBody>) -> Result<Json<Value>, ApiError> {
    let l = &rt(&st)?.learn;
    let md = b.files.get("SKILL.md").ok_or(ApiError(StatusCode::BAD_REQUEST, "files must hold SKILL.md".into()))?;
    let (meta, _) = lh_learn::parse_skill_md(md, None).map_err(|e| ApiError(StatusCode::BAD_REQUEST, e))?;
    let op = if l.load_skill(&meta.name).is_ok() { "edit" } else { "create" };
    let name = l.install_skill(&b.files, op, "user", "").map_err(err)?;
    Ok(Json(json!({"name": name})))
}
async fn skill_rollback<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(name): Path<String>, Json(b): Json<VersionBody>) -> Result<Json<Value>, ApiError> {
    Ok(Json(json!({"name": rt(&st)?.learn.rollback_skill(&name, b.version, "user").map_err(err)?})))
}
async fn delete_skill<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(name): Path<String>) -> Result<StatusCode, ApiError> {
    rt(&st)?.learn.delete_skill(&name, "user").map_err(err)?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- suggestions ----

#[derive(Deserialize)]
struct ProposalQ {
    status: Option<String>,
}
async fn proposals<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Query(q): Query<ProposalQ>) -> Result<Json<Value>, ApiError> {
    let l = rt(&st)?.learn.list_proposals(Some(q.status.as_deref().unwrap_or("pending"))).map_err(err)?;
    Ok(Json(json!({"proposals": l.iter().map(proposal_json).collect::<Vec<_>>()})))
}

fn applied_json(a: &Applied) -> Value {
    match a {
        Applied::Rule(r) => json!({"rule": rule_json(r)}),
        Applied::Style(s) => json!({"style": {"text": s.text, "version": s.version}}),
        Applied::Skill(n) => json!({"skill": n}),
    }
}

#[derive(Deserialize, Default)]
struct AcceptBody {
    /// Fields the user changed before accepting, such as `text`, `topic`, `body`.
    edit: Option<Value>,
}
async fn accept<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<i64>, body: Option<Json<AcceptBody>>) -> Result<Json<Value>, ApiError> {
    let b = body.map(|j| j.0).unwrap_or_default();
    let r = rt(&st)?;
    let session = r.learn.get_proposal(id).ok().and_then(|p| p.session).unwrap_or_else(|| "-".into());
    let a = r.learn.accept_proposal(id, b.edit.as_ref(), "user").map_err(err)?;
    crate::rewards::reward(&st.settings.store, &session, "proposal", 1.0, false);
    Ok(Json(applied_json(&a)))
}
async fn reject<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<i64>) -> Result<StatusCode, ApiError> {
    let r = rt(&st)?;
    let session = r.learn.get_proposal(id).ok().and_then(|p| p.session).unwrap_or_else(|| "-".into());
    r.learn.reject_proposal(id).map_err(err)?;
    crate::rewards::reward(&st.settings.store, &session, "proposal", -1.0, false);
    Ok(StatusCode::NO_CONTENT)
}

async fn signals<M: Model + 'static>(State(st): State<Arc<AppState<M>>>) -> Result<Json<Value>, ApiError> {
    let l = rt(&st)?.learn.signals(None).map_err(err)?;
    Ok(Json(json!({"signals": l.iter().map(|s| json!({"id": s.id, "session": s.session, "kind": s.kind.as_str(), "text": s.text, "privacy": s.privacy.as_str()})).collect::<Vec<_>>()})))
}

#[derive(Deserialize)]
struct HabitsQ {
    min: Option<usize>,
}
async fn habits<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Query(q): Query<HabitsQ>) -> Result<Json<Value>, ApiError> {
    let r = rt(&st)?;
    let h = r.learn.habits(q.min.unwrap_or(r.threshold)).map_err(err)?;
    Ok(Json(json!({"habits": h.iter().map(|c| json!({"sessions": c.sessions, "examples": c.signals.iter().map(|s| s.text.clone()).collect::<Vec<_>>()})).collect::<Vec<_>>()})))
}

#[derive(Deserialize)]
struct FeedbackBody {
    /// "up" or "down"
    kind: String,
    note: Option<String>,
}
async fn feedback<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<String>, Json(b): Json<FeedbackBody>) -> Result<StatusCode, ApiError> {
    let r = rt(&st)?;
    let s = st.session(&id)?;
    if s.incognito {
        return Ok(StatusCode::NO_CONTENT); // an incognito session learns nothing
    }
    let note = b.note.unwrap_or_default();
    // a thumbs down with words is a correction; the words are what can become a rule
    let kind = match (b.kind.as_str(), note.trim().is_empty()) {
        ("up", _) => SignalKind::ThumbsUp,
        ("down", false) => SignalKind::Correction,
        ("down", true) => SignalKind::ThumbsDown,
        _ => return Err(ApiError(StatusCode::BAD_REQUEST, "kind must be \"up\" or \"down\"".into())),
    };
    r.learn.record_signal(&id, kind, &note, s.project.as_deref()).map_err(err)?;
    crate::rewards::reward(&st.settings.store, &id, "thumbs", if b.kind == "up" { 1.0 } else { -1.0 }, false);
    Ok(StatusCode::NO_CONTENT)
}

// ---- the learn pass ----

struct Asker2<'a, M> {
    st: &'a AppState<M>,
    meter: lh_spend::SpendMeter,
    model: String,
}

impl<M: Model + 'static> Asker2<'_, M> {
    /// One model call, checked against the spending rules. `None` if refused or failed.
    async fn ask(&self, mut req: ChatRequest) -> Option<String> {
        req.model.clone_from(&self.model);
        if self.meter.before(&req).await != MeterVerdict::Allow {
            return None;
        }
        let resp = self.st.model.complete(&req, &mut |_| {}).await.ok()?;
        self.meter.after(&req, resp.usage).await;
        Some(resp.text)
    }
}

fn plain(prompt: String) -> ChatRequest {
    let mut r = ChatRequest { messages: vec![Message::user(prompt)], max_tokens: Some(3000), ..Default::default() };
    r.sampling.extra.insert("chat_template_kwargs".into(), json!({"enable_thinking": false}));
    r
}

fn excerpt(t: &str, n: usize) -> String {
    let flat = t.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= n { flat } else { format!("{}…", flat.chars().take(n).collect::<String>()) }
}

async fn learn_pass<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    let r = rt(&st)?;
    let s = st.session(&id)?;
    if s.incognito {
        return Err(ApiError(StatusCode::BAD_REQUEST, "an incognito session learns nothing".into()));
    }
    if !s.try_start() {
        return Err(ApiError(StatusCode::CONFLICT, "a turn is running in this session".into()));
    }
    let out = learn_inner(&st, r, &s, &id).await;
    s.finish_turn();
    out
}

async fn learn_inner<M: Model + 'static>(st: &Arc<AppState<M>>, r: &LearnRuntime, s: &Arc<Session>, id: &str) -> Result<Json<Value>, ApiError> {
    let work = s.req.lock().await.clone();
    if !work.messages.iter().any(|m| m.role == Role::User) {
        return Err(ApiError(StatusCode::BAD_REQUEST, "nothing was said yet".into()));
    }
    let ask = Asker2 { st, meter: st.settings.spend.for_session(id), model: work.model.clone() };
    let local = r.trust == Trust::Local;
    let rules = r.learn.list_rules(false).map_err(err)?.into_iter().filter(|x| x.privacy.allowed_at(r.trust)).collect::<Vec<_>>();
    let style = r.learn.style().map_err(err)?.map(|x| x.text).filter(|t| local || lh_memory::detect(t).privacy == Privacy::Normal);
    let mut new_ids: Vec<i64> = vec![];

    // 1. what the user told us in this session
    let mine: Vec<_> = r.learn.signals(Some(id)).map_err(err)?.into_iter().filter(|x| matches!(x.kind, SignalKind::Stated | SignalKind::Correction) && (local || x.privacy == Privacy::Normal) && x.asked == 0).collect();
    if !mine.is_empty() {
        let texts: Vec<String> = mine.iter().map(|x| x.text.clone()).collect();
        // the last turn of the conversation as it stands; the same tools stay listed so a caching server reuses its cache
        let mut req = work.clone();
        req.messages.push(Message::user(session_prompt(&texts, &rules, style.as_deref())));
        req.max_tokens = Some(3000);
        req.sampling.extra.insert("chat_template_kwargs".into(), json!({"enable_thinking": false}));
        if let Some(reply) = ask.ask(req).await {
            new_ids.extend(r.learn.propose_from_drafts(parse_drafts(&reply), "stated", Some(id), s.project.as_deref(), &texts).map_err(err)?);
        }
        r.learn.mark_asked(&mine.iter().map(|x| x.id).collect::<Vec<_>>(), 1).map_err(err)?;
    }

    // 2. wishes seen in several sessions: only the matching messages are sent, nothing else
    for c in r.learn.new_habits(r.threshold).map_err(err)? {
        let sigs: Vec<_> = c.signals.iter().filter(|x| local || x.privacy == Privacy::Normal).collect();
        if sigs.len() < r.threshold {
            continue;
        }
        let texts: Vec<String> = sigs.iter().take(8).map(|x| x.text.clone()).collect();
        if let Some(reply) = ask.ask(plain(habit_prompt(&texts, c.sessions, &rules, style.as_deref()))).await {
            new_ids.extend(r.learn.propose_from_drafts(parse_drafts(&reply), "repeat", Some(id), s.project.as_deref(), &texts).map_err(err)?);
        }
        r.learn.mark_asked(&c.signals.iter().map(|x| x.id).collect::<Vec<_>>(), 2).map_err(err)?;
    }

    // 3. skills: a long task may be worth keeping; a skill that was used may need a fix
    if r.learn.list_skills().is_ok() && s.tool_calls.load(std::sync::atomic::Ordering::SeqCst) >= r.skill_min_calls && local {
        let record = work.messages.iter().filter(|m| m.role == Role::Assistant).flat_map(|m| m.tool_calls.iter().map(|c| format!("- {} {}", c.name, excerpt(&c.arguments, 140)))).take(40).collect::<Vec<_>>().join("\n");
        let existing: Vec<String> = r.learn.list_skills().map_err(err)?.into_iter().map(|e| e.name).collect();
        let first = work.messages.iter().find(|m| m.role == Role::User).and_then(|m| m.content.clone()).unwrap_or_default();
        let task = format!("The user asked: {}\nSteps taken:\n{record}", excerpt(&first, 400));
        if let Some(reply) = ask.ask(plain(skill_prompt(&task, &existing))).await {
            if let Some(d) = parse_skill_draft(&reply) {
                if !existing.contains(&d.name) {
                    let ev = vec![excerpt(&first, 120)];
                    new_ids.extend(r.learn.add_proposal(Kind::SkillNew, json!({"name": d.name, "description": d.description, "body": d.body}), ev, "skill", Some(id)).map_err(err)?);
                }
            }
        }
    }
    let used: Vec<String> = s.skills_used.lock().unwrap().iter().cloned().collect();
    for name in used {
        let Ok(skill) = r.learn.load_skill(&name) else { continue };
        if !local {
            continue;
        }
        let happened = work.messages.iter().rev().take(14).rev().filter_map(|m| m.content.as_deref().map(|c| format!("{:?}: {}", m.role, excerpt(c, 300)))).collect::<Vec<_>>().join("\n");
        if let Some(reply) = ask.ask(plain(skill_improve_prompt(&name, &skill.body, &happened))).await {
            if let Some((body, note)) = parse_skill_change(&reply) {
                new_ids.extend(r.learn.add_proposal(Kind::SkillUpdate, json!({"name": name, "body": body, "note": note}), vec![note.clone()], "skill", Some(id)).map_err(err)?);
            }
        }
    }

    // 4. silent learning applies them at once (and logs it); otherwise they wait
    let mut applied = vec![];
    if r.mode == LearnMode::Silent {
        for pid in &new_ids {
            if let Ok(a) = r.learn.accept_proposal(*pid, None, "silent learning") {
                applied.push(applied_json(&a));
            }
        }
    }
    let _ = st.settings.store.log("learn_pass", id, "assistant", &json!({"suggestions": new_ids.len(), "applied_silently": applied.len()}));
    let pending: Vec<Value> = r.learn.list_proposals(Some("pending")).map_err(err)?.iter().filter(|p| new_ids.contains(&p.id)).map(proposal_json).collect();
    Ok(Json(json!({"proposals": pending, "applied": applied})))
}
