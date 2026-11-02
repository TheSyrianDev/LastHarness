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

const MASK: &str = "[secret removed]";

/// Take anything shaped like a key or token out of a piece of text. The log is for the user to read and
/// may be shown in a browser, so a secret that passed through a command or an error never stays in it.
pub fn redact(text: &str) -> String {
    fn secretish(w: &str) -> bool {
        let w = w.trim_matches(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | '+' | '=' | ':')));
        let after = |p: &str, n: usize| w.strip_prefix(p).is_some_and(|r| r.len() >= n && r.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_')));
        after("sk-", 16) || after("ghp_", 20) || after("gho_", 20) || after("ghu_", 20) || after("ghs_", 20) || after("ghr_", 20) || after("xoxb-", 10) || after("xoxp-", 10) || after("xoxa-", 10) || after("xoxs-", 10) || after("xapp-", 10)
            || (w.starts_with("AKIA") && w.len() == 20 && w.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()))
            || (w.starts_with("eyJ") && w.matches('.').count() == 2 && w.len() > 30)
            // a Telegram bot token: digits, a colon, then a long run
            || w.split_once(':').is_some_and(|(a, b)| a.len() >= 6 && a.chars().all(|c| c.is_ascii_digit()) && b.len() >= 30 && b.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_')))
    }
    let mut out = text.to_string();
    // key blocks
    while let (Some(a), Some(b)) = (out.find("-----BEGIN "), out.find("PRIVATE KEY-----")) {
        if b < a {
            break;
        }
        let end = out[b..].find("-----END ").and_then(|e| out[b + e..].find("KEY-----").map(|k| b + e + k + 8)).unwrap_or(out.len());
        out.replace_range(a..end, MASK);
    }
    // words, and the part of `Bearer <token>`, `key=<token>` and URLs with a password in them
    let mut res = String::with_capacity(out.len());
    let mut prev_bearer = false;
    for part in out.split_inclusive(char::is_whitespace) {
        let word = part.trim_end();
        let tail = &part[word.len()..];
        // user:password@host inside an address
        let word_owned: String = match word.find("://").filter(|_| word.contains('@')) {
            Some(i) => {
                let (scheme, rest) = word.split_at(i + 3);
                match rest.split_once('@') {
                    Some((cred, host)) if cred.contains(':') && !cred.contains('/') => format!("{scheme}{MASK}@{host}"),
                    _ => word.to_string(),
                }
            }
            None => word.to_string(),
        };
        let word_ref = word_owned.as_str();
        let masked = if prev_bearer && word_ref.len() >= 12 {
            MASK.to_string()
        } else if let Some((k, _)) = word_ref.split_once('=').filter(|(_, v)| secretish(v)) {
            format!("{k}={MASK}")
        } else if secretish(word_ref) {
            MASK.to_string()
        } else {
            word_owned.clone()
        };
        prev_bearer = word.eq_ignore_ascii_case("bearer") || word.eq_ignore_ascii_case("bearer:");
        res.push_str(&masked);
        res.push_str(tail);
    }
    res
}

fn redact_value(v: &Value) -> Value {
    match v {
        Value::String(s) => Value::String(redact(s)),
        Value::Array(a) => Value::Array(a.iter().map(redact_value).collect()),
        Value::Object(m) => Value::Object(m.iter().map(|(k, v)| (k.clone(), redact_value(v))).collect()),
        other => other.clone(),
    }
}

impl Store {
    pub fn log(&self, kind: &str, session: &str, actor: &str, detail: &Value) -> Result<i64> {
        let detail = redact_value(detail);
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

    /// Which of these entries still have an undo waiting (a file write not yet undone).
    pub fn undoable(&self, seqs: &[i64]) -> Result<Vec<i64>> {
        self.with(|c| {
            let mut q = c.prepare("select 1 from undo where seq = ?1 and undone_at is null")?;
            let mut out = vec![];
            for s in seqs {
                if q.exists([s])? {
                    out.push(*s);
                }
            }
            Ok(out)
        })
    }

    /// The newest activity number, or 0 when nothing is logged. A turn remembers it when it starts.
    pub fn last_seq(&self) -> Result<i64> {
        self.with(|c| c.query_row("select coalesce(max(seq), 0) from activity", [], |r| r.get(0)))
    }

    /// The files this session's tools wrote after activity `after`, oldest first, each once.
    pub fn files_written(&self, session: &str, after: i64) -> Result<Vec<String>> {
        self.with(|c| {
            let mut q = c.prepare("select u.path from undo u join activity a on a.seq = u.seq where a.session = ?1 and u.seq > ?2 group by u.path order by min(u.seq)")?;
            let rows = q.query_map((session, after), |r| r.get(0))?;
            rows.collect()
        })
    }

    /// Put back every file this session wrote after activity `after`, newest first: the undo of one turn. Each
    /// file is checked as `undo` checks it, so a file someone changed since is left alone and reported.
    pub fn undo_since(&self, session: &str, after: i64) -> Result<Vec<(i64, String, UndoResult)>> {
        let rows: Vec<(i64, String)> = self.with(|c| {
            let mut q = c.prepare("select u.seq, u.path from undo u join activity a on a.seq = u.seq where a.session = ?1 and u.seq > ?2 and u.undone_at is null order by u.seq desc")?;
            let rows = q.query_map((session, after), |r| Ok((r.get(0)?, r.get(1)?)))?;
            rows.collect()
        })?;
        let mut out = vec![];
        for (seq, path) in rows {
            let r = self.undo(seq, session)?;
            out.push((seq, path, r));
        }
        Ok(out)
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
    fn a_whole_turn_is_undone_newest_first() {
        let (s, d) = (Store::open_memory().unwrap(), tempfile::tempdir().unwrap());
        let (a, b) = (d.path().join("a.txt"), d.path().join("b.txt"));
        logged_write(&s, &a, None, b"first turn");
        let start = s.last_seq().unwrap();
        logged_write(&s, &a, Some(b"first turn"), b"second turn");
        logged_write(&s, &a, Some(b"second turn"), b"second turn, again");
        logged_write(&s, &b, None, b"new file");
        let other = s.log("tool_call", "s2", "assistant", &json!({})).unwrap();
        s.log_file_change(other, &d.path().join("c.txt"), None, 0).unwrap();
        let done = s.undo_since("s1", start).unwrap();
        assert_eq!(done.len(), 3, "only this session's writes after the start: {done:?}");
        assert!(done.iter().all(|(_, _, r)| matches!(r, UndoResult::Restored | UndoResult::Removed)), "{done:?}");
        assert_eq!(std::fs::read(&a).unwrap(), b"first turn");
        assert!(!b.exists(), "a file the turn made is removed");
        assert!(s.undo_since("s1", start).unwrap().is_empty(), "nothing is undone twice");
    }

    #[test]
    fn secrets_never_stay_in_the_log() {
        let s = Store::open_memory().unwrap();
        let cmd = "curl -H 'Authorization: Bearer abcdefghijklmnop1234' https://u:hunter2@host.example/x?key=sk-abcdefghijklmnopqrstuv -d token=ghp_abcdefghijklmnopqrstuvwx";
        s.log("tool_call", "s", "a", &json!({"arguments": cmd, "nested": {"list": ["xoxb-1234567890-abcdef", "123456789:AAEhBP0abcdefghijklmnopqrstuvwxyz012", "AKIAABCDEFGHIJKLMNOP"]}})).unwrap();
        s.log("x", "s", "a", &json!({"text": "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEA\n-----END OPENSSH PRIVATE KEY----- done"})).unwrap();
        s.log("y", "s", "a", &json!({"jwt": "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.abcdefghijklmnopqrstu"})).unwrap();
        let all = serde_json::to_string(&s.recent(10, None).unwrap().iter().map(|e| e.detail.clone()).collect::<Vec<_>>()).unwrap();
        for secret in ["abcdefghijklmnop1234", "hunter2", "sk-abcdefghijklmnopqrstuv", "ghp_abcdefghijklmnopqrstuvwx", "xoxb-1234567890", "AAEhBP0abc", "AKIAABCDEFGHIJKLMNOP", "b3BlbnNzaC1rZXk", "eyJhbGciOiJIUzI1NiJ9"] {
            assert!(!all.contains(secret), "{secret} is in the log: {all}");
        }
        assert!(all.contains("curl -H") && all.contains("host.example") && all.contains("done"), "the rest is kept: {all}");
        // ordinary text is left alone
        assert_eq!(redact("ls -la src/main.rs && echo sk-short"), "ls -la src/main.rs && echo sk-short");
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
        assert_eq!(s.undoable(&[seq, seq + 100]).unwrap(), vec![seq]);
        assert_eq!(s.undo(seq, "s1").unwrap(), UndoResult::Restored);
        assert!(s.undoable(&[seq]).unwrap().is_empty(), "once undone it is not offered again");
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
