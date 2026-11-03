use lh_store::now;
use serde_json::Value;

use crate::{Error, Learn, Result};

#[derive(Clone, Debug, PartialEq)]
pub struct HistoryEntry {
    pub seq: i64,
    pub version: i64,
    pub op: String,
    pub by: String,
    pub at: i64,
    pub content: Value,
    pub note: String,
}

impl Learn {
    /// Record a new version of something that is kept as a file or a row elsewhere (a role, say), so it
    /// has the same history and rollback as rules. `content` is whatever is needed to put it back.
    pub fn record_version(&self, item: (&str, &str), version: i64, op: &str, by: &str, content: &Value, note: &str) -> Result<()> {
        self.note(item, version, op, by, content, note)
    }

    pub(crate) fn note(&self, (kind, item): (&str, &str), version: i64, op: &str, by: &str, content: &Value, note: &str) -> Result<()> {
        self.store.with(|c| {
            c.execute("insert into learn_history(kind, item, version, op, by, at, content, note) values (?1,?2,?3,?4,?5,?6,?7,?8)", (kind, item, version, op, by, now(), content.to_string(), note)).map(|_| ())
        })?;
        // The activity log says that something was learned, not what: rules can mention private things.
        let _ = self.store.log("learned", "-", by, &serde_json::json!({"kind": kind, "item": item, "op": op, "version": version}));
        Ok(())
    }

    pub fn history(&self, kind: &str, item: &str) -> Result<Vec<HistoryEntry>> {
        Ok(self.store.with(|c| {
            let mut q = c.prepare("select seq, version, op, by, at, content, note from learn_history where kind = ?1 and item = ?2 order by seq")?;
            let rows = q.query_map((kind, item), |r| {
                Ok(HistoryEntry { seq: r.get(0)?, version: r.get(1)?, op: r.get(2)?, by: r.get(3)?, at: r.get(4)?, content: serde_json::from_str(&r.get::<_, String>(5)?).unwrap_or(Value::Null), note: r.get(6)? })
            })?;
            rows.collect()
        })?)
    }

    /// The content of one version, for rollback.
    pub fn version_content(&self, kind: &str, item: &str, version: i64) -> Result<Value> {
        let h = self.history(kind, item)?;
        h.into_iter().rev().find(|e| e.version == version && e.op != "delete").map(|e| e.content).filter(|c| c.as_object().is_some_and(|o| !o.is_empty())).ok_or_else(|| Error::NotFound(format!("version {version} of {item}")))
    }

    /// Blank what the history holds for an item the user deleted for good.
    pub fn scrub(&self, kind: &str, item: &str) -> Result<()> {
        self.store.with(|c| c.execute("update learn_history set content = '{}' where kind = ?1 and item = ?2", (kind, item)).map(|_| ()))?;
        Ok(())
    }
}
