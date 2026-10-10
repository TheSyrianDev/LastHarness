//! Past conversations: the raw record, as it was said. Never edited (the database refuses updates),
//! searchable by keyword, and exportable to Markdown. Incognito sessions never reach this code.

use lh_store::now;

use crate::{text, Memory, Result};

#[derive(Clone, Debug, PartialEq)]
pub struct MessageHit {
    pub conv: String,
    pub role: String,
    pub content: String,
    pub at: i64,
}

impl Memory {
    pub fn start_conversation(&self, id: &str, project: Option<&str>) -> Result<()> {
        self.store.with(|c| c.execute("insert or ignore into conversations(id, project, started) values (?1, ?2, ?3)", (id, project, now())).map(|_| ()))?;
        Ok(())
    }

    pub fn end_conversation(&self, id: &str) -> Result<()> {
        self.store.with(|c| c.execute("update conversations set ended = ?2 where id = ?1", (id, now())).map(|_| ()))?;
        Ok(())
    }

    pub fn append_message(&self, conv: &str, role: &str, content: &str) -> Result<()> {
        let st = text::tokens(content).join(" ");
        self.store.with(|c| {
            let seq: i64 = c.query_row("select coalesce(max(seq), 0) + 1 from messages where conv = ?1", [conv], |r| r.get(0))?;
            c.execute("insert into messages(conv, seq, role, content, search_text, at) values (?1,?2,?3,?4,?5,?6)", (conv, seq, role, content, st, now())).map(|_| ())
        })?;
        Ok(())
    }

    pub fn messages(&self, conv: &str) -> Result<Vec<(String, String)>> {
        Ok(self.store.with(|c| {
            let mut q = c.prepare("select role, content from messages where conv = ?1 order by seq")?;
            let rows = q.query_map([conv], |r| Ok((r.get(0)?, r.get(1)?)))?;
            rows.collect()
        })?)
    }

    /// Keyword search over everything ever said.
    pub fn search_conversations(&self, query: &str, limit: usize) -> Result<Vec<MessageHit>> {
        let toks = text::tokens(query);
        if toks.is_empty() {
            return Ok(vec![]);
        }
        let fts = toks.iter().map(|t| format!("\"{}\"{}", t.replace('"', ""), if t.chars().count() >= 3 { "*" } else { "" })).collect::<Vec<_>>().join(" OR ");
        Ok(self.store.with(|c| {
            let mut q = c.prepare("select m.conv, m.role, m.content, m.at from messages_fts join messages m on m.id = messages_fts.rowid where messages_fts match ?1 order by bm25(messages_fts) limit ?2")?;
            let rows = q.query_map((fts, limit as i64), |r| Ok(MessageHit { conv: r.get(0)?, role: r.get(1)?, content: r.get(2)?, at: r.get(3)? }))?;
            rows.collect()
        })?)
    }
}
