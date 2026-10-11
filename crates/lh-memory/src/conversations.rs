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

/// One stored chat, as the chat list shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct ConversationInfo {
    pub id: String,
    pub title: String,
    pub project: Option<String>,
    /// `chat`, `assistant` or `project`.
    pub mode: String,
    /// The model the chat last used, if known.
    pub model: Option<String>,
    pub started: i64,
    pub last: i64,
    pub messages: usize,
    /// Characters over all messages, for a token estimate.
    pub chars: usize,
}

impl Memory {
    /// The next number for a session id: one more than the largest `s<number>` already stored, so a restart
    /// of the core never reuses the id of a stored chat.
    pub fn next_session_number(&self) -> u64 {
        self.store
            .with(|c| c.query_row("select coalesce(max(cast(substr(id, 2) as integer)), 0) from conversations where id glob 's[0-9]*'", [], |r| r.get::<_, i64>(0)))
            .map(|n| n as u64 + 1)
            .unwrap_or(1)
    }

    pub fn set_conversation_meta(&self, id: &str, model: &str, mode: &str) -> Result<()> {
        self.store.with(|c| c.execute("update conversations set model = ?2, mode = ?3 where id = ?1", (id, model, mode)).map(|_| ()))?;
        Ok(())
    }

    pub fn set_conversation_model(&self, id: &str, model: &str) -> Result<()> {
        self.store.with(|c| c.execute("update conversations set model = ?2 where id = ?1", (id, model)).map(|_| ()))?;
        Ok(())
    }

    /// The title is the start of the first thing the user said; set once.
    pub fn set_title_if_empty(&self, id: &str, text: &str) -> Result<()> {
        let title: String = text.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(60).collect();
        self.store.with(|c| c.execute("update conversations set title = ?2 where id = ?1 and (title is null or title = '')", (id, title)).map(|_| ()))?;
        Ok(())
    }

    /// Stored chats that have at least one message, newest first.
    pub fn list_conversations(&self, limit: usize) -> Result<Vec<ConversationInfo>> {
        Ok(self.store.with(|c| {
            let mut q = c.prepare(
                "select c.id, coalesce(c.title, ''), c.project, coalesce(c.mode, 'chat'), c.model, c.started, coalesce(max(m.at), c.started), count(m.id), coalesce(sum(length(m.content)), 0) \
                 from conversations c join messages m on m.conv = c.id group by c.id order by 7 desc, c.started desc limit ?1",
            )?;
            let rows = q.query_map([limit as i64], |r| {
                Ok(ConversationInfo { id: r.get(0)?, title: r.get(1)?, project: r.get(2)?, mode: r.get(3)?, model: r.get(4)?, started: r.get(5)?, last: r.get(6)?, messages: r.get::<_, i64>(7)? as usize, chars: r.get::<_, i64>(8)? as usize })
            })?;
            rows.collect()
        })?)
    }

    pub fn conversation(&self, id: &str) -> Result<Option<ConversationInfo>> {
        Ok(self.list_conversations(100_000)?.into_iter().find(|c| c.id == id))
    }

    /// Remove a chat for good: its messages, their search entries and the chat itself.
    pub fn delete_conversation(&self, id: &str) -> Result<bool> {
        Ok(self.store.with(|c| {
            let tx = c.unchecked_transaction()?;
            {
                let mut q = tx.prepare("select id, search_text from messages where conv = ?1")?;
                let rows: Vec<(i64, String)> = q.query_map([id], |r| Ok((r.get(0)?, r.get(1)?)))?.flatten().collect();
                for (rowid, st) in rows {
                    tx.execute("insert into messages_fts(messages_fts, rowid, search_text) values ('delete', ?1, ?2)", (rowid, st))?;
                }
            }
            tx.execute("delete from messages where conv = ?1", [id])?;
            let n = tx.execute("delete from conversations where id = ?1", [id])?;
            tx.commit()?;
            Ok(n > 0)
        })?)
    }

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
