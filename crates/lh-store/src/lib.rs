//! The local store: one SQLite file for spend records and the activity log (memory joins it in
//! Milestone 2). Write-ahead log mode, so readers never wait for a writer. Tables that must never
//! be edited (the activity log) are protected by triggers, not by trust.

use std::path::Path;
use std::sync::Mutex;

mod activity;

pub use activity::{content_hash, Entry, UndoResult};
pub use rusqlite::Connection;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("the local store failed: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("cannot create the store folder: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Each entry runs once, in order. `PRAGMA user_version` remembers how many have run.
const MIGRATIONS: &[&str] = &[r#"
    create table spend(
      id integer primary key,
      at integer not null,                -- unix seconds
      provider text not null,
      model text not null,
      session text not null,
      prompt_tokens integer not null,
      completion_tokens integer not null,
      cached_tokens integer not null,
      cost_micro_usd integer,             -- millionths of a dollar; null = the model has no known price
      owner text not null default 'me',
      scope text not null default 'personal'
    );
    create index spend_at on spend(at);

    create table activity(
      seq integer primary key,
      at integer not null,
      kind text not null,                 -- tool_call, request_sent, setting_changed, permission_asked, ...
      session text not null,
      actor text not null,
      detail text not null,               -- JSON
      owner text not null default 'me',
      scope text not null default 'personal'
    );
    create index activity_kind on activity(kind, seq);
    create trigger activity_no_update before update on activity begin select raise(abort, 'the activity log is append-only'); end;
    create trigger activity_no_delete before delete on activity begin select raise(abort, 'the activity log is append-only'); end;
"#, r#"
    -- how to put a file back after a write; the activity row itself stays untouched
    create table undo(
      seq integer primary key references activity(seq),
      path text not null,
      previous blob,                      -- null: the file did not exist before
      after_hash integer not null,        -- hash of what the tool wrote, to notice later edits
      undone_at integer
    );
"#];

pub struct Store {
    conn: Mutex<Connection>,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        Self::init(Connection::open(path)?)
    }

    pub fn open_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self> {
        conn.execute_batch("pragma journal_mode=wal; pragma synchronous=normal; pragma busy_timeout=5000;")?;
        let done: i64 = conn.query_row("pragma user_version", [], |r| r.get(0))?;
        for (i, m) in MIGRATIONS.iter().enumerate().skip(done as usize) {
            let tx = conn.unchecked_transaction()?;
            tx.execute_batch(m)?;
            tx.execute_batch(&format!("pragma user_version = {}", i + 1))?;
            tx.commit()?;
        }
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Give memory back and fold the write-ahead log into the main file. For idle mode.
    pub fn shrink(&self) {
        let _ = self.with(|c| c.execute_batch("pragma wal_checkpoint(truncate); pragma shrink_memory;"));
    }

    /// Run a closure with the connection. Calls are short, so one lock is enough.
    pub fn with<T>(&self, f: impl FnOnce(&Connection) -> rusqlite::Result<T>) -> Result<T> {
        Ok(f(&self.conn.lock().unwrap())?)
    }
}

pub fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_once_and_reopens() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("sub/lh.db");
        {
            let s = Store::open(&p).unwrap();
            s.with(|c| c.execute("insert into activity(at,kind,session,actor,detail) values (1,'k','s','a','{}')", [])).unwrap();
        }
        let s = Store::open(&p).unwrap();
        let n: i64 = s.with(|c| c.query_row("select count(*) from activity", [], |r| r.get(0))).unwrap();
        assert_eq!(n, 1);
        let v: i64 = s.with(|c| c.query_row("pragma user_version", [], |r| r.get(0))).unwrap();
        assert_eq!(v, MIGRATIONS.len() as i64);
    }

    #[test]
    fn the_activity_log_cannot_be_edited() {
        let s = Store::open_memory().unwrap();
        s.with(|c| c.execute("insert into activity(at,kind,session,actor,detail) values (1,'k','s','a','{}')", [])).unwrap();
        assert!(s.with(|c| c.execute("update activity set kind='x'", [])).is_err());
        assert!(s.with(|c| c.execute("delete from activity", [])).is_err());
    }
}
