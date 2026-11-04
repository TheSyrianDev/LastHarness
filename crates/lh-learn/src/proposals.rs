//! Suggestions: a rule, a change to a rule, a change to the style profile, or a skill. They wait for
//! the user, who accepts, edits or drops each one. Nothing here talks to a model; the prompts and the
//! readers of the model's answers are here so they can be tested, and the caller runs the model.

use lh_memory::{detect, Privacy};
use lh_store::now;
use serde_json::{json, Value};

use crate::{norm_words, Error, Learn, NewRule, Result, Rule, RuleEdit, Style, STYLE_MAX_CHARS};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Rule,
    RuleUpdate,
    Style,
    SkillNew,
    SkillUpdate,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Rule => "rule",
            Kind::RuleUpdate => "rule_update",
            Kind::Style => "style",
            Kind::SkillNew => "skill_new",
            Kind::SkillUpdate => "skill_update",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        [Kind::Rule, Kind::RuleUpdate, Kind::Style, Kind::SkillNew, Kind::SkillUpdate].into_iter().find(|k| k.as_str() == s)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Proposal {
    pub id: i64,
    pub kind: Kind,
    pub payload: Value,
    pub evidence: Vec<String>,
    /// stated | repeat | correction | skill
    pub source: String,
    pub status: String,
    pub session: Option<String>,
}

/// What accepting a suggestion changed.
#[derive(Debug)]
pub enum Applied {
    Rule(Rule),
    Style(Style),
    Skill(String),
}

impl Learn {
    /// Store a suggestion. A suggestion that says the same as one already waiting or accepted is dropped.
    pub fn add_proposal(&self, kind: Kind, payload: Value, evidence: Vec<String>, source: &str, session: Option<&str>) -> Result<Option<i64>> {
        let key = payload_key(kind, &payload);
        let all = self.list_proposals(None)?;
        if all.iter().any(|p| p.kind == kind && p.status != "rejected" && payload_key(p.kind, &p.payload) == key) {
            return Ok(None);
        }
        // The same suggestion worded differently: no second suggestion. If the first is still waiting, it
        // gains the new quotes, and "seen again in other sessions" makes it a repeat.
        let wish = wish_words(kind, &payload);
        if !wish.is_empty() {
            if let Some(old) = all.iter().find(|p| same_family(p.kind, kind) && p.status != "rejected" && close(&wish_words(p.kind, &p.payload), &wish)) {
                if old.status == "pending" {
                    let mut ev = old.evidence.clone();
                    for e in evidence {
                        if !ev.contains(&e) && ev.len() < 5 {
                            ev.push(e);
                        }
                    }
                    let src = if source == "repeat" { "repeat" } else { old.source.as_str() };
                    self.store.with(|c| c.execute("update learn_proposals set evidence = ?2, source = ?3 where id = ?1", (old.id, serde_json::to_string(&ev).unwrap_or_default(), src)).map(|_| ()))?;
                }
                return Ok(None);
            }
        }
        Ok(Some(self.store.with(|c| {
            c.execute(
                "insert into learn_proposals(kind, payload, evidence, source, session, created) values (?1,?2,?3,?4,?5,?6)",
                (kind.as_str(), payload.to_string(), serde_json::to_string(&evidence).unwrap_or_default(), source, session, now()),
            )?;
            Ok(c.last_insert_rowid())
        })?))
    }

    pub fn list_proposals(&self, status: Option<&str>) -> Result<Vec<Proposal>> {
        Ok(self.store.with(|c| {
            let mut q = c.prepare("select id, kind, payload, evidence, source, status, session from learn_proposals where (?1 is null or status = ?1) order by id")?;
            let rows = q.query_map([status], |r| {
                Ok(Proposal {
                    id: r.get(0)?,
                    kind: Kind::parse(&r.get::<_, String>(1)?).unwrap_or(Kind::Rule),
                    payload: serde_json::from_str(&r.get::<_, String>(2)?).unwrap_or(Value::Null),
                    evidence: serde_json::from_str(&r.get::<_, String>(3)?).unwrap_or_default(),
                    source: r.get(4)?,
                    status: r.get(5)?,
                    session: r.get(6)?,
                })
            })?;
            rows.collect()
        })?)
    }

    pub fn get_proposal(&self, id: i64) -> Result<Proposal> {
        self.list_proposals(None)?.into_iter().find(|p| p.id == id).ok_or_else(|| Error::NotFound(format!("suggestion {id}")))
    }

    pub fn reject_proposal(&self, id: i64) -> Result<()> {
        let p = self.get_proposal(id)?;
        if p.status != "pending" {
            return Err(Error::Invalid(format!("suggestion {id} was already {}", p.status)));
        }
        self.store.with(|c| c.execute("update learn_proposals set status = 'rejected', decided = ?2 where id = ?1", (id, now())).map(|_| ()))?;
        Ok(())
    }

    /// Apply a suggestion. `edit` holds fields the user changed; they replace the same fields of the payload.
    pub fn accept_proposal(&self, id: i64, edit: Option<&Value>, by: &str) -> Result<Applied> {
        let p = self.get_proposal(id)?;
        if p.status != "pending" {
            return Err(Error::Invalid(format!("suggestion {id} was already {}", p.status)));
        }
        let mut pl = p.payload.clone();
        if let (Some(e), Some(o)) = (edit.and_then(Value::as_object), pl.as_object_mut()) {
            for (k, v) in e {
                o.insert(k.clone(), v.clone());
            }
        }
        let s = |k: &str| pl[k].as_str().map(str::to_string);
        let applied = match p.kind {
            Kind::Rule => {
                let text = s("text").unwrap_or_default();
                let class = detect(&format!("{text} {}", p.evidence.join(" ")));
                let privacy = match s("privacy").as_deref().and_then(Privacy::parse) {
                    Some(Privacy::Sensitive) => Privacy::Sensitive,
                    _ => class.privacy,
                };
                Applied::Rule(self.add_rule(NewRule { topic: s("topic").unwrap_or_default(), text, project: s("project"), role: s("role"), privacy, evidence: p.evidence.clone(), source: "proposal".into() }, by)?)
            }
            Kind::RuleUpdate => {
                let target = s("target").ok_or_else(|| Error::Invalid("this change names no rule".into()))?;
                let mut e = RuleEdit { text: s("text"), topic: s("topic"), ..Default::default() };
                if let Some(pr) = s("privacy").as_deref().and_then(Privacy::parse) {
                    e.privacy = Some(pr);
                }
                Applied::Rule(self.edit_rule(&target, e, by)?)
            }
            Kind::Style => Applied::Style(self.set_style(&s("text").unwrap_or_default(), by)?),
            Kind::SkillNew | Kind::SkillUpdate => Applied::Skill(self.apply_skill_proposal(p.kind, &pl, by)?),
        };
        self.store.with(|c| c.execute("update learn_proposals set status = 'accepted', decided = ?2 where id = ?1", (id, now())).map(|_| ()))?;
        Ok(applied)
    }
}

/// Rules and style notes can say the same thing; skills are compared by name only.
fn same_family(a: Kind, b: Kind) -> bool {
    let rule_like = |k: Kind| matches!(k, Kind::Rule | Kind::RuleUpdate | Kind::Style);
    (rule_like(a) && rule_like(b)) || a == b
}

fn wish_words(kind: Kind, pl: &Value) -> std::collections::HashSet<String> {
    let field = match kind {
        Kind::Style => "added",
        Kind::Rule | Kind::RuleUpdate => "text",
        Kind::SkillNew | Kind::SkillUpdate => return Default::default(),
    };
    norm_words(pl[field].as_str().unwrap_or("")).into_iter().collect()
}

fn close(a: &std::collections::HashSet<String>, b: &std::collections::HashSet<String>) -> bool {
    let common = a.intersection(b).count();
    common >= 2 && common as f64 / a.len().min(b.len()).max(1) as f64 >= 0.7
}

fn payload_key(kind: Kind, pl: &Value) -> String {
    let t = |k: &str| pl[k].as_str().unwrap_or("");
    match kind {
        Kind::Rule | Kind::RuleUpdate => format!("{}|{}", t("target"), norm_words(t("text")).join(" ")),
        Kind::Style => norm_words(t("added")).join(" ") + "|" + &norm_words(t("text")).join(" "),
        Kind::SkillNew | Kind::SkillUpdate => format!("{}|{}", t("name"), norm_words(t("body")).join(" ")),
    }
}

// ---- asking a model ----

#[derive(Clone, Debug, PartialEq)]
pub enum DraftKind {
    Rule,
    Style,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Draft {
    pub kind: DraftKind,
    pub topic: String,
    pub text: String,
    pub project_only: bool,
    pub evidence: String,
}

fn existing_block(rules: &[Rule], style: Option<&str>) -> String {
    let mut s = String::from("Existing rules:\n");
    if rules.is_empty() {
        s.push_str("(none)\n");
    }
    for r in rules {
        s.push_str(&format!("- [{}] {}\n", if r.topic.is_empty() { "-" } else { &r.topic }, r.text));
    }
    s.push_str(&format!("Existing style profile: {}\n", style.filter(|x| !x.trim().is_empty()).unwrap_or("(none)")));
    s
}

const SHAPE: &str = "Answer with ONLY a JSON array, at most 5 items, each like \
{\"kind\":\"rule\",\"topic\":\"two or three words\",\"text\":\"one short sentence addressed to an assistant\",\"scope\":\"global\",\"evidence\":\"a short quote\"}. \
Use kind \"style\" for tone and manner (terse or chatty, how much to explain, whether to ask first) and \"rule\" for a habit about a kind of task. \
Keep the strength of the user's words: if they said never, write never, not avoid. Use scope \"project\" only if it clearly applies to this one project. Skip anything an existing rule already covers. Answer [] if nothing is a lasting rule.";

/// The last user turn at the end of a conversation: which of the things the user said are lasting rules?
pub fn session_prompt(candidates: &[String], rules: &[Rule], style: Option<&str>) -> String {
    let mut s = String::from("The conversation is ending. Below are messages where the user told you how they want things done, or corrected you. Decide which are lasting rules or habits and not just about this one task. ");
    s.push_str(SHAPE);
    s.push_str("\n\nMessages:\n");
    for c in candidates {
        s.push_str(&format!("- \"{}\"\n", c.replace('\n', " ")));
    }
    s.push('\n');
    s.push_str(&existing_block(rules, style));
    s
}

/// For a habit seen in several conversations: only the matching messages are sent, nothing else.
pub fn habit_prompt(messages: &[String], sessions: usize, rules: &[Rule], style: Option<&str>) -> String {
    let mut s = format!("A user said things like these in {sessions} different conversations. Is there one lasting rule behind them? ");
    s.push_str(SHAPE);
    s.push_str("\n\nMessages:\n");
    for c in messages {
        s.push_str(&format!("- \"{}\"\n", c.replace('\n', " ")));
    }
    s.push('\n');
    s.push_str(&existing_block(rules, style));
    s
}

/// Reads the model's answer. Tolerant of prose and code fences; bad items are skipped.
pub fn parse_drafts(reply: &str) -> Vec<Draft> {
    let (Some(a), Some(b)) = (reply.find('['), reply.rfind(']')) else { return vec![] };
    if b <= a {
        return vec![];
    }
    let Ok(Value::Array(items)) = serde_json::from_str::<Value>(&reply[a..=b]) else { return vec![] };
    items
        .iter()
        .filter_map(|v| {
            let s = |k: &str| v[k].as_str().map(|x| x.split_whitespace().collect::<Vec<_>>().join(" ")).filter(|x| !x.is_empty());
            let kind = match s("kind")?.to_lowercase().as_str() {
                "rule" => DraftKind::Rule,
                "style" => DraftKind::Style,
                _ => return None,
            };
            let text = s("text")?;
            if text.chars().count() > 400 {
                return None;
            }
            Some(Draft { kind, topic: s("topic").unwrap_or_default().to_lowercase(), text, project_only: s("scope").is_some_and(|x| x.eq_ignore_ascii_case("project")), evidence: s("evidence").unwrap_or_default() })
        })
        .take(5)
        .collect()
}

impl Learn {
    /// Turn the model's drafts into suggestions. Returns the ids of the ones that were new.
    pub fn propose_from_drafts(&self, drafts: Vec<Draft>, source: &str, session: Option<&str>, project: Option<&str>, evidence: &[String]) -> Result<Vec<i64>> {
        let rules = self.list_rules(false)?;
        let style = self.style()?;
        let mut ids = vec![];
        for d in drafts {
            let ev: Vec<String> = if d.evidence.is_empty() { evidence.iter().take(3).cloned().collect() } else { vec![d.evidence.clone()] };
            let class = detect(&format!("{} {}", d.text, ev.join(" ")));
            let privacy = class.privacy.as_str();
            let id = match d.kind {
                DraftKind::Rule => {
                    let proj = d.project_only.then(|| project.map(str::to_string)).flatten();
                    let same_topic = (!d.topic.is_empty()).then(|| rules.iter().find(|r| r.topic == d.topic && r.project == proj)).flatten();
                    match same_topic {
                        Some(r) if norm_words(&r.text) == norm_words(&d.text) => None, // already known
                        Some(r) => self.add_proposal(Kind::RuleUpdate, json!({"target": r.id, "topic": d.topic, "text": d.text, "privacy": privacy, "was": r.text}), ev, source, session)?,
                        None if rules.iter().any(|r| norm_words(&r.text) == norm_words(&d.text)) => None,
                        None => self.add_proposal(Kind::Rule, json!({"topic": d.topic, "text": d.text, "project": proj, "privacy": privacy}), ev, source, session)?,
                    }
                }
                DraftKind::Style => {
                    let cur = style.as_ref().map(|s| s.text.clone()).unwrap_or_default();
                    if norm_words(&cur).join(" ").contains(&norm_words(&d.text).join(" ")) {
                        None
                    } else {
                        let merged = if cur.is_empty() { d.text.clone() } else { format!("{cur} {}", d.text) };
                        if merged.chars().count() > STYLE_MAX_CHARS {
                            None // the profile is full; the user must trim it first
                        } else {
                            self.add_proposal(Kind::Style, json!({"text": merged, "added": d.text, "privacy": privacy}), ev, source, session)?
                        }
                    }
                }
            };
            ids.extend(id);
        }
        Ok(ids)
    }
}

// ---- skills ----

#[derive(Clone, Debug, PartialEq)]
pub struct SkillDraft {
    pub name: String,
    pub description: String,
    pub body: String,
}

/// After a task that took many steps: is there a procedure worth keeping?
pub fn skill_prompt(task_summary: &str, existing: &[String]) -> String {
    format!(
        "The task is finished. Below is a short record of what was done. If it is a procedure the user is likely to ask for again, write it as a reusable skill. \
Answer with ONLY a JSON object: {{\"worth\":true,\"name\":\"lower-case-with-hyphens\",\"description\":\"what it does and when to use it, one or two sentences\",\"body\":\"Markdown instructions: numbered steps, what to check, what to avoid\"}}. \
Answer {{\"worth\":false}} if it was a one-off, if it is trivial, or if an existing skill already covers it.\n\nExisting skills: {}\n\nRecord:\n{task_summary}",
        if existing.is_empty() { "(none)".to_string() } else { existing.join(", ") }
    )
}

/// A skill was used in this conversation. Should its instructions change?
pub fn skill_improve_prompt(name: &str, current_body: &str, what_happened: &str) -> String {
    format!(
        "The skill \"{name}\" was used in this conversation. Below are its current instructions and what happened (including any corrections from the user). \
If the instructions should change so that next time goes better, answer with ONLY a JSON object {{\"change\":true,\"body\":\"the full new instructions\",\"note\":\"one line: what changed and why\"}}. \
Answer {{\"change\":false}} if they were fine.\n\nCurrent instructions:\n{current_body}\n\nWhat happened:\n{what_happened}"
    )
}

fn json_object(reply: &str) -> Option<Value> {
    let (a, b) = (reply.find('{')?, reply.rfind('}')?);
    (b > a).then(|| serde_json::from_str::<Value>(&reply[a..=b]).ok()).flatten().filter(Value::is_object)
}

pub fn parse_skill_draft(reply: &str) -> Option<SkillDraft> {
    let v = json_object(reply)?;
    if v["worth"].as_bool() != Some(true) {
        return None;
    }
    let s = |k: &str| v[k].as_str().map(|x| x.trim().to_string()).filter(|x| !x.is_empty());
    let (name, description, body) = (s("name")?, s("description")?, s("body")?);
    crate::valid_name(&name).then_some(SkillDraft { name, description, body })
}

/// `(new instructions, note)`.
pub fn parse_skill_change(reply: &str) -> Option<(String, String)> {
    let v = json_object(reply)?;
    if v["change"].as_bool() != Some(true) {
        return None;
    }
    let body = v["body"].as_str()?.trim().to_string();
    (!body.is_empty()).then(|| (body, v["note"].as_str().unwrap_or("").trim().to_string()))
}
