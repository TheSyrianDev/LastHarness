//! Things the harness suggests remembering. Nothing becomes memory until the user accepts it,
//! optionally after editing it (spec §6.2).

use lh_store::now;
use serde_json::Value;

use crate::{text, By, Edit, Error, Layer, Memory, NewPage, Page, Privacy, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A new page.
    New,
    /// A changed fact: the old version is archived, the page gets the new text.
    Update,
    /// The fact no longer holds: archive the page.
    Archive,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::New => "new",
            Kind::Update => "update",
            Kind::Archive => "archive",
        }
    }
    fn parse(s: &str) -> Option<Self> {
        match s {
            "new" => Some(Kind::New),
            "update" => Some(Kind::Update),
            "archive" => Some(Kind::Archive),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Proposal {
    pub id: i64,
    pub kind: Kind,
    pub layer: Layer,
    pub title: String,
    pub body: String,
    pub gist: String,
    pub target_page: Option<String>,
    pub privacy: Privacy,
    pub privacy_reason: String,
    pub evidence: String,
    pub conv: Option<String>,
    pub status: String,
}

#[derive(Clone, Debug)]
pub struct NewProposal {
    pub kind: Kind,
    pub layer: Layer,
    pub title: String,
    pub body: String,
    pub gist: String,
    pub target_page: Option<String>,
    pub privacy: Privacy,
    pub privacy_reason: String,
    pub evidence: String,
    pub conv: Option<String>,
    pub project: Option<String>,
}

/// What the user changed before accepting.
#[derive(Clone, Debug, Default)]
pub struct ProposalEdit {
    pub title: Option<String>,
    pub body: Option<String>,
    pub gist: Option<String>,
    pub privacy: Option<Privacy>,
}

const COLS: &str = "id, kind, layer, title, body, gist, target_page, privacy, privacy_reason, evidence, conv, status";

fn from_row(r: &lh_store::rusqlite::Row) -> lh_store::rusqlite::Result<Proposal> {
    let bad = |what: &str| lh_store::rusqlite::Error::InvalidColumnType(0, what.into(), lh_store::rusqlite::types::Type::Text);
    Ok(Proposal {
        id: r.get(0)?,
        kind: Kind::parse(&r.get::<_, String>(1)?).ok_or_else(|| bad("kind"))?,
        layer: Layer::parse(&r.get::<_, String>(2)?).ok_or_else(|| bad("layer"))?,
        title: r.get(3)?,
        body: r.get(4)?,
        gist: r.get(5)?,
        target_page: r.get(6)?,
        privacy: Privacy::parse(&r.get::<_, String>(7)?).ok_or_else(|| bad("privacy"))?,
        privacy_reason: r.get(8)?,
        evidence: r.get(9)?,
        conv: r.get(10)?,
        status: r.get(11)?,
    })
}

impl Memory {
    /// Store a suggestion. A suggestion identical to one still waiting, or already accepted, is dropped.
    pub fn add_proposal(&self, n: NewProposal) -> Result<Option<i64>> {
        if n.kind != Kind::Archive && (n.title.trim().is_empty() || n.body.trim().is_empty()) {
            return Err(Error::Invalid("a suggestion needs a title and some text".into()));
        }
        let key = text::tokens(&format!("{} {}", n.title, n.body)).join(" ");
        let duplicate = self.list_proposals(None)?.iter().any(|p| p.status != "rejected" && p.kind == n.kind && p.target_page == n.target_page && text::tokens(&format!("{} {}", p.title, p.body)).join(" ") == key)
            || (n.kind == Kind::New && self.list(None, crate::State::Live)?.iter().any(|p| text::tokens(&format!("{} {}", p.title, p.body)).join(" ") == key));
        if duplicate {
            return Ok(None);
        }
        Ok(Some(self.store.with(|c| {
            c.execute(
                "insert into proposals(kind, layer, title, body, gist, target_page, privacy, privacy_reason, evidence, conv, created) values (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                (n.kind.as_str(), n.layer.as_str(), n.title.trim(), n.body.trim(), n.gist.replace('\n', " ").trim(), &n.target_page, n.privacy.as_str(), &n.privacy_reason, &n.evidence, &n.conv, now()),
            )?;
            Ok(c.last_insert_rowid())
        })?))
    }

    /// `status`: `Some("pending")`, `Some("accepted")`, ... or `None` for all.
    pub fn list_proposals(&self, status: Option<&str>) -> Result<Vec<Proposal>> {
        Ok(self.store.with(|c| {
            let mut q = c.prepare(&format!("select {COLS} from proposals where (?1 is null or status = ?1) order by id"))?;
            let rows = q.query_map([status], from_row)?;
            rows.collect()
        })?)
    }

    fn proposal(&self, id: i64) -> Result<Proposal> {
        self.list_proposals(None)?.into_iter().find(|p| p.id == id).ok_or_else(|| Error::NotFound(format!("suggestion {id}")))
    }

    /// Apply a suggestion, with the user's edits if any. Returns the page it created or changed.
    pub fn accept_proposal(&self, id: i64, edit: ProposalEdit, project: Option<String>) -> Result<Page> {
        let p = self.proposal(id)?;
        if p.status != "pending" {
            return Err(Error::Invalid(format!("suggestion {id} was already {}", p.status)));
        }
        let title = edit.title.unwrap_or(p.title.clone());
        let body = edit.body.unwrap_or(p.body.clone());
        let gist = edit.gist.unwrap_or(p.gist.clone());
        let privacy = edit.privacy.unwrap_or(p.privacy);
        let page = match p.kind {
            Kind::New => {
                let mut n = NewPage::new(p.layer, title, body);
                n.gist = gist;
                n.privacy = privacy;
                n.privacy_reason = p.privacy_reason.clone();
                n.project = project;
                n.source = p.conv.clone().unwrap_or_else(|| "review".into());
                self.create(n, By::User)?
            }
            Kind::Update => {
                let target = p.target_page.as_deref().ok_or_else(|| Error::Invalid("this update has no page to change".into()))?;
                let mut page = self.update(target, Edit { title: Some(title), body: Some(body), gist: Some(gist) }, By::User)?;
                if page.privacy != privacy {
                    page = self.set_privacy(target, privacy, &p.privacy_reason, By::User)?;
                }
                page
            }
            Kind::Archive => {
                let target = p.target_page.as_deref().ok_or_else(|| Error::Invalid("this has no page to archive".into()))?;
                self.archive(target, By::User, "no longer true (confirmed in review)")?
            }
        };
        self.store.with(|c| c.execute("update proposals set status = 'accepted', decided = ?2 where id = ?1", (id, now())).map(|_| ()))?;
        Ok(page)
    }

    pub fn reject_proposal(&self, id: i64) -> Result<()> {
        let p = self.proposal(id)?;
        if p.status != "pending" {
            return Err(Error::Invalid(format!("suggestion {id} was already {}", p.status)));
        }
        self.store.with(|c| c.execute("update proposals set status = 'rejected', decided = ?2 where id = ?1", (id, now())).map(|_| ()))?;
        Ok(())
    }
}

// ---- the end-of-session review turn ----

/// The last user turn of the review. The conversation before it is untouched, so a server with
/// prefix caching reads it from its cache.
pub const REVIEW_PROMPT: &str = "The conversation is ending. Look for things worth remembering about the user for future conversations: \
how they like things done (layer \"preferences\"), facts about them (\"facts\"), and decisions or project details (\"projects\"). \
Skip small talk, things already in the notes below, and anything only true for this one task. \
If a fact in the notes changed, use action \"update\" with the note's id; if one is no longer true, use \"archive\" with its id. \
Answer with ONLY a JSON array, at most 8 items, each like \
{\"action\":\"new\",\"layer\":\"facts\",\"title\":\"short title\",\"text\":\"one to three sentences\",\"gist\":\"one short line\",\"evidence\":\"a short quote from the conversation\"} \
(for update/archive add \"target\":\"<note id>\"). Answer [] if there is nothing.";

#[derive(Clone, Debug, PartialEq)]
pub struct Parsed {
    pub kind: Kind,
    pub layer: Layer,
    pub title: String,
    pub text: String,
    pub gist: String,
    pub target: Option<String>,
    pub evidence: String,
}

/// Reads the model's answer. Tolerant: the array may sit inside prose or a code fence, bad items
/// are skipped, and anything unreadable gives an empty list, never an error.
pub fn parse_review(reply: &str) -> Vec<Parsed> {
    let (Some(a), Some(b)) = (reply.find('['), reply.rfind(']')) else { return vec![] };
    if b <= a {
        return vec![];
    }
    let Ok(Value::Array(items)) = serde_json::from_str::<Value>(&reply[a..=b]) else { return vec![] };
    items
        .iter()
        .filter_map(|v| {
            let s = |k: &str| v[k].as_str().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
            let kind = Kind::parse(&s("action")?.to_lowercase())?;
            let layer = Layer::parse(&s("layer").unwrap_or_else(|| "facts".into()).to_lowercase()).unwrap_or(Layer::Facts);
            let target = s("target");
            if kind != Kind::New && target.is_none() {
                return None;
            }
            let (title, text) = (s("title").unwrap_or_default(), s("text").unwrap_or_default());
            if kind != Kind::Archive && (title.is_empty() || text.is_empty()) {
                return None;
            }
            Some(Parsed { kind, layer, title, text, gist: s("gist").unwrap_or_default(), target, evidence: s("evidence").unwrap_or_default() })
        })
        .take(8)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_replies_are_read_tolerantly() {
        let reply = "Sure! Here you go:\n```json\n[{\"action\":\"new\",\"layer\":\"Facts\",\"title\":\"Home\",\"text\":\"Lives in Berlin.\",\"gist\":\"Lives in Berlin\",\"evidence\":\"I moved to Berlin\"},\n{\"action\":\"update\",\"title\":\"x\",\"text\":\"y\",\"target\":\"p_1\"},\n{\"action\":\"archive\",\"target\":\"p_2\"},\n{\"action\":\"update\",\"title\":\"no target\",\"text\":\"y\"},\n{\"action\":\"new\",\"title\":\"\",\"text\":\"empty title\"},\n{\"action\":\"explode\"}]\n```";
        let p = parse_review(reply);
        assert_eq!(p.len(), 3);
        assert_eq!((p[0].kind, p[0].layer, p[0].title.as_str(), p[0].gist.as_str()), (Kind::New, Layer::Facts, "Home", "Lives in Berlin"));
        assert_eq!((p[1].kind, p[1].target.as_deref()), (Kind::Update, Some("p_1")));
        assert_eq!((p[2].kind, p[2].target.as_deref()), (Kind::Archive, Some("p_2")));
        for bad in ["", "nothing to remember", "[]", "[not json", "{\"action\":\"new\"}", "] ["] {
            assert!(parse_review(bad).is_empty(), "{bad}");
        }
        let many = format!("[{}]", (0..20).map(|i| format!("{{\"action\":\"new\",\"title\":\"t{i}\",\"text\":\"x\"}}")).collect::<Vec<_>>().join(","));
        assert_eq!(parse_review(&many).len(), 8);
    }
}
