//! The activity log: one append-only record of what the harness did (tool calls, permission
//! answers, requests that left the machine, startup settings). Undo exists only for file writes.

use std::path::Path;

use serde_json::Value;

use crate::{now, Result, Store};

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub seq: i64,
    pub at: i64,
    pub kind: String,
    pub session: String,
    pub actor: String,
    pub detail: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub enum UndoResult {
    /// The file is back to its earlier content.
    Restored,
    /// The file did not exist before, so it was removed.
    Removed,
    /// Nothing was changed; the text says why.
    Refused(String),
}

/// FNV-1a. Only used to notice that a file changed after the harness wrote it; not a security hash.
pub fn content_hash(bytes: &[u8]) -> i64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h as i64
}

impl Store {
    pub fn log(&self, kind: &str, session: &str, actor: &str, detail: &Value) -> Result<i64> {
        self.with(|c| {
            c.execute("insert into activity(at, kind, session, actor, detail) values (?1,?2,?3,?4,?5)", (now(), kind, session, actor, detail.to_string()))?;
            Ok(c.last_insert_rowid())
        })
    }

    /// Remember how to undo a file write that was logged as `seq`.
    pub fn log_file_change(&self, seq: i64, path: &Path, previous: Option<&[u8]>, after_hash: i64) -> Result<()> {
        self.with(|c| c.execute("insert into undo(seq, path, previous, after_hash) values (?1,?2,?3,?4)", (seq, path.to_string_lossy(), previous, after_hash)).map(|_| ()))
    }

    /// Newest first.
    pub fn recent(&self, limit: u32, kind: Option<&str>) -> Result<Vec<Entry>> {
        self.with(|c| {
            let mut q = c.prepare("select seq, at, kind, session, actor, detail from activity where (?1 is null or kind = ?1) order by seq desc limit ?2")?;
            let rows = q.query_map((kind, limit), |r| {
                Ok(Entry { seq: r.get(0)?, at: r.get(1)?, kind: r.get(2)?, session: r.get(3)?, actor: r.get(4)?, detail: serde_json::from_str(&r.get::<_, String>(5)?).unwrap_or(Value::Null) })
            })?;
            rows.collect()
        })
    }

    /// Put the file written by activity `seq` back. Refuses if anything edited it since.
    pub fn undo(&self, seq: i64, session: &str) -> Result<UndoResult> {
        let row = self.with(|c| {
            c.query_row("select path, previous, after_hash, undone_at from undo where seq = ?1", [seq], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, Option<Vec<u8>>>(1)?, r.get::<_, i64>(2)?, r.get::<_, Option<i64>>(3)?))
            })
            .map(Some)
            .or_else(|e| if matches!(e, rusqlite::Error::QueryReturnedNoRows) { Ok(None) } else { Err(e) })
        })?;
        let Some((path, previous, after_hash, undone)) = row else { return Ok(UndoResult::Refused("this entry cannot be undone".into())) };
        if undone.is_some() {
            return Ok(UndoResult::Refused("this was already undone".into()));
        }
        let current = std::fs::read(&path).ok();
        match &current {
            Some(b) if content_hash(b) != after_hash => return Ok(UndoResult::Refused("the file was changed after the harness wrote it, so it was left alone".into())),
            None if previous.is_some() => return Ok(UndoResult::Refused("the file was removed after the harness wrote it, so it was left alone".into())),
            _ => {}
        }
        let result = match &previous {
            Some(bytes) => {
                if std::fs::write(&path, bytes).is_err() {
                    return Ok(UndoResult::Refused("the file could not be written".into()));
                }
                UndoResult::Restored
            }
            None => {
                if current.is_some() && std::fs::remove_file(&path).is_err() {
                    return Ok(UndoResult::Refused("the file could not be removed".into()));
                }
                UndoResult::Removed
            }
        };
        self.with(|c| c.execute("update undo set undone_at = ?2 where seq = ?1", (seq, now())).map(|_| ()))?;
        self.log("undo", session, "user", &serde_json::json!({"undid": seq, "path": path}))?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn logged_write(s: &Store, path: &Path, previous: Option<&[u8]>, content: &[u8]) -> i64 {
        std::fs::write(path, content).unwrap();
        let seq = s.log("tool_call", "s1", "assistant", &json!({"tool":"write_file"})).unwrap();
        s.log_file_change(seq, path, previous, content_hash(content)).unwrap();
        seq
    }

    #[test]
    fn recent_is_newest_first_and_filters_by_kind() {
        let s = Store::open_memory().unwrap();
        s.log("a", "s", "x", &json!({"n":1})).unwrap();
        s.log("b", "s", "x", &json!({"n":2})).unwrap();
        s.log("a", "s", "x", &json!({"n":3})).unwrap();
        let all = s.recent(10, None).unwrap();
        assert_eq!(all.iter().map(|e| e.detail["n"].as_i64().unwrap()).collect::<Vec<_>>(), [3, 2, 1]);
        assert_eq!(s.recent(10, Some("b")).unwrap().len(), 1);
        assert_eq!(s.recent(2, None).unwrap().len(), 2);
    }

    #[test]
    fn undo_restores_an_overwritten_file_once() {
        let (s, d) = (Store::open_memory().unwrap(), tempfile::tempdir().unwrap());
        let p = d.path().join("a.txt");
        let seq = logged_write(&s, &p, Some(b"old"), b"new");
        assert_eq!(s.undo(seq, "s1").unwrap(), UndoResult::Restored);
        assert_eq!(std::fs::read(&p).unwrap(), b"old");
        assert!(matches!(s.undo(seq, "s1").unwrap(), UndoResult::Refused(m) if m.contains("already")));
        assert_eq!(s.recent(1, Some("undo")).unwrap().len(), 1);
    }

    #[test]
    fn undo_removes_a_file_that_was_new() {
        let (s, d) = (Store::open_memory().unwrap(), tempfile::tempdir().unwrap());
        let p = d.path().join("new.txt");
        let seq = logged_write(&s, &p, None, b"hello");
        assert_eq!(s.undo(seq, "s1").unwrap(), UndoResult::Removed);
        assert!(!p.exists());
    }

    #[test]
    fn undo_never_overwrites_later_edits() {
        let (s, d) = (Store::open_memory().unwrap(), tempfile::tempdir().unwrap());
        let p = d.path().join("a.txt");
        let seq = logged_write(&s, &p, Some(b"old"), b"new");
        std::fs::write(&p, b"the user kept typing").unwrap();
        assert!(matches!(s.undo(seq, "s1").unwrap(), UndoResult::Refused(m) if m.contains("changed")));
        assert_eq!(std::fs::read(&p).unwrap(), b"the user kept typing");
        std::fs::remove_file(&p).unwrap();
        assert!(matches!(s.undo(seq, "s1").unwrap(), UndoResult::Refused(m) if m.contains("removed")));
        let plain = s.log("tool_call", "s", "a", &json!({})).unwrap();
        assert!(matches!(s.undo(plain, "s").unwrap(), UndoResult::Refused(_)));
    }
}
