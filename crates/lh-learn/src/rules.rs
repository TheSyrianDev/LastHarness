use lh_memory::{Privacy, Trust};
use lh_store::now;
use serde_json::{json, Value};

use crate::{new_id, Error, Learn, Result};

/// A rule is one habit, in a sentence or two.
pub const RULE_MAX_CHARS: usize = 400;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Active,
    Disabled,
}

impl Status {
    fn as_str(self) -> &'static str {
        match self {
            Status::Active => "active",
            Status::Disabled => "disabled",
        }
    }
    fn parse(s: &str) -> Self {
        if s == "disabled" { Status::Disabled } else { Status::Active }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
    pub id: String,
    /// Rules about the same topic compete; the most specific wins. Empty means it competes with none.
    pub topic: String,
    pub text: String,
    pub project: Option<String>,
    pub role: Option<String>,
    pub privacy: Privacy,
    pub status: Status,
    /// Short quotes that led to the rule.
    pub evidence: Vec<String>,
    pub source: String,
    pub version: i64,
    pub created: i64,
    pub updated: i64,
}

impl Rule {
    fn snapshot(&self) -> Value {
        json!({"topic": self.topic, "text": self.text, "project": self.project, "role": self.role, "privacy": self.privacy.as_str(), "status": self.status.as_str(), "evidence": self.evidence, "source": self.source})
    }

    /// How narrow the rule is: a project rule beats a general one, a role rule beats one for all roles.
    pub fn specificity(&self) -> u8 {
        u8::from(self.project.is_some()) * 2 + u8::from(self.role.is_some())
    }
}

#[derive(Clone, Debug)]
pub struct NewRule {
    pub topic: String,
    pub text: String,
    pub project: Option<String>,
    pub role: Option<String>,
    pub privacy: Privacy,
    pub evidence: Vec<String>,
    pub source: String,
}

impl NewRule {
    pub fn new(text: impl Into<String>) -> Self {
        Self { topic: String::new(), text: text.into(), project: None, role: None, privacy: Privacy::Normal, evidence: vec![], source: "user".into() }
    }
}

#[derive(Clone, Debug, Default)]
pub struct RuleEdit {
    pub topic: Option<String>,
    pub text: Option<String>,
    /// `Some(None)` makes the rule apply everywhere again.
    pub project: Option<Option<String>>,
    pub role: Option<Option<String>>,
    pub privacy: Option<Privacy>,
}

/// Where a request is going, for choosing rules.
#[derive(Clone, Debug)]
pub struct RuleContext {
    pub project: Option<String>,
    pub role: Option<String>,
    pub trust: Trust,
}

const COLS: &str = "id, topic, text, project, role, privacy, status, evidence, source, version, created, updated";

fn from_row(r: &lh_store::rusqlite::Row) -> lh_store::rusqlite::Result<Rule> {
    Ok(Rule {
        id: r.get(0)?,
        topic: r.get(1)?,
        text: r.get(2)?,
        project: r.get(3)?,
        role: r.get(4)?,
        privacy: Privacy::parse(&r.get::<_, String>(5)?).unwrap_or(Privacy::Sensitive),
        status: Status::parse(&r.get::<_, String>(6)?),
        evidence: serde_json::from_str(&r.get::<_, String>(7)?).unwrap_or_default(),
        source: r.get(8)?,
        version: r.get(9)?,
        created: r.get(10)?,
        updated: r.get(11)?,
    })
}

fn check_text(text: &str) -> Result<String> {
    let t = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.is_empty() {
        return Err(Error::Invalid("a rule needs some text".into()));
    }
    if t.chars().count() > RULE_MAX_CHARS {
        return Err(Error::Invalid(format!("a rule is one habit in a sentence or two (at most {RULE_MAX_CHARS} characters). Split it into several rules.")));
    }
    Ok(t)
}

impl Learn {
    pub fn add_rule(&self, n: NewRule, by: &str) -> Result<Rule> {
        let text = check_text(&n.text)?;
        let t = now();
        let r = Rule {
            id: new_id("r"),
            topic: n.topic.trim().to_lowercase(),
            text,
            project: n.project,
            role: n.role,
            privacy: n.privacy,
            status: Status::Active,
            evidence: n.evidence,
            source: n.source,
            version: 1,
            created: t,
            updated: t,
        };
        self.store.with(|c| {
            c.execute(
                "insert into rules(id, topic, text, project, role, privacy, status, evidence, source, version, created, updated) values (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
                (&r.id, &r.topic, &r.text, &r.project, &r.role, r.privacy.as_str(), r.status.as_str(), serde_json::to_string(&r.evidence).unwrap_or_default(), &r.source, r.version, r.created, r.updated),
            )
            .map(|_| ())
        })?;
        self.note(("rule", &r.id), 1, "create", by, &r.snapshot(), "")?;
        Ok(r)
    }

    pub fn get_rule(&self, id: &str) -> Result<Rule> {
        use lh_store::rusqlite::OptionalExtension;
        self.store.with(|c| c.query_row(&format!("select {COLS} from rules where id = ?1"), [id], from_row).optional())?.ok_or_else(|| Error::NotFound(format!("rule {id}")))
    }

    pub fn list_rules(&self, include_disabled: bool) -> Result<Vec<Rule>> {
        Ok(self.store.with(|c| {
            let mut q = c.prepare(&format!("select {COLS} from rules where (?1 = 1 or status = 'active') order by created, rowid"))?;
            let rows = q.query_map([include_disabled as i64], from_row)?;
            rows.collect()
        })?)
    }

    fn save_rule(&self, r: &Rule) -> Result<()> {
        self.store.with(|c| {
            c.execute(
                "update rules set topic=?2, text=?3, project=?4, role=?5, privacy=?6, status=?7, evidence=?8, source=?9, version=?10, updated=?11 where id=?1",
                (&r.id, &r.topic, &r.text, &r.project, &r.role, r.privacy.as_str(), r.status.as_str(), serde_json::to_string(&r.evidence).unwrap_or_default(), &r.source, r.version, r.updated),
            )
            .map(|_| ())
        })?;
        Ok(())
    }

    pub fn edit_rule(&self, id: &str, e: RuleEdit, by: &str) -> Result<Rule> {
        let old = self.get_rule(id)?;
        let mut r = old.clone();
        if let Some(t) = e.text {
            r.text = check_text(&t)?;
        }
        if let Some(t) = e.topic {
            r.topic = t.trim().to_lowercase();
        }
        if let Some(p) = e.project {
            r.project = p;
        }
        if let Some(p) = e.role {
            r.role = p;
        }
        if let Some(p) = e.privacy {
            r.privacy = p;
        }
        if r == old {
            return Ok(old);
        }
        r.version += 1;
        r.updated = now();
        self.save_rule(&r)?;
        self.note(("rule", id), r.version, "edit", by, &r.snapshot(), "")?;
        Ok(r)
    }

    /// Turn a rule off or on without losing it.
    pub fn set_rule_status(&self, id: &str, status: Status, by: &str) -> Result<Rule> {
        let mut r = self.get_rule(id)?;
        if r.status == status {
            return Ok(r);
        }
        r.status = status;
        r.version += 1;
        r.updated = now();
        self.save_rule(&r)?;
        self.note(("rule", id), r.version, if status == Status::Active { "enable" } else { "disable" }, by, &r.snapshot(), "")?;
        Ok(r)
    }

    /// Put a rule back the way it was in an earlier version. This adds a new version; the history stays whole.
    pub fn rollback_rule(&self, id: &str, version: i64, by: &str) -> Result<Rule> {
        let old = self.get_rule(id)?;
        let snap = self.version_content("rule", id, version)?;
        let mut r = old.clone();
        let s = |k: &str| snap[k].as_str().map(str::to_string);
        r.topic = s("topic").unwrap_or_default();
        r.text = s("text").unwrap_or(old.text.clone());
        r.project = s("project");
        r.role = s("role");
        r.privacy = s("privacy").and_then(|p| Privacy::parse(&p)).unwrap_or(old.privacy);
        r.status = Status::parse(&s("status").unwrap_or_default());
        r.evidence = snap["evidence"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default();
        r.version += 1;
        r.updated = now();
        self.save_rule(&r)?;
        self.note(("rule", id), r.version, "rollback", by, &r.snapshot(), &format!("back to version {version}"))?;
        Ok(r)
    }

    /// Remove a rule for good, including its text in the history.
    pub fn delete_rule(&self, id: &str, by: &str) -> Result<()> {
        let r = self.get_rule(id)?;
        self.scrub("rule", id)?;
        self.store.with(|c| c.execute("delete from rules where id = ?1", [id]).map(|_| ()))?;
        self.note(("rule", id), r.version + 1, "delete", by, &json!({}), "")?;
        Ok(())
    }

    /// The rules that apply to a request: active, allowed at the destination's trust level, for this
    /// project and role, and for each topic only the most specific one (the newest if tied).
    pub fn applicable_rules(&self, ctx: &RuleContext) -> Result<Vec<Rule>> {
        let mut fits: Vec<Rule> = self
            .list_rules(false)?
            .into_iter()
            .filter(|r| r.privacy.allowed_at(ctx.trust))
            .filter(|r| r.project.is_none() || r.project == ctx.project)
            .filter(|r| r.role.is_none() || r.role == ctx.role)
            .collect();
        let best: std::collections::HashMap<String, (u8, i64, String)> = fits.iter().filter(|r| !r.topic.is_empty()).fold(Default::default(), |mut m, r| {
            let key = (r.specificity(), r.updated, r.id.clone());
            let e = m.entry(r.topic.clone()).or_insert_with(|| key.clone());
            if key > *e {
                *e = key;
            }
            m
        });
        fits.retain(|r| r.topic.is_empty() || best.get(&r.topic).is_some_and(|(_, _, id)| *id == r.id));
        Ok(fits)
    }
}
