//! The local store: one SQLite file for spend records and the activity log (memory joins it in
//! Milestone 2). Write-ahead log mode, so readers never wait for a writer. Tables that must never
//! be edited (the activity log) are protected by triggers, not by trust.

use std::path::Path;
use std::sync::Mutex;

mod activity;
pub mod schedules;

pub use activity::{content_hash, Entry, UndoResult};
pub use rusqlite;
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
"#, r#"
    -- Memory pages. The Markdown file is the readable source; this table is its index and the place
    -- where privacy class, version and archive state live.
    create table pages(
      id text primary key,
      layer text not null check (layer in ('preferences','facts','projects')),
      slug text not null,
      title text not null,
      body text not null,
      search_text text not null,          -- title and body normalised for keyword search
      gist text not null default '',
      privacy text not null default 'normal' check (privacy in ('normal','sensitive')),
      privacy_reason text not null default '',
      pinned integer not null default 0,
      project text,                       -- null = applies everywhere
      source text not null default 'user',
      version integer not null default 1,
      archived_at integer,                -- null = live; otherwise left out of normal recall
      file_hash integer not null default 0,
      created integer not null,
      updated integer not null,
      owner text not null default 'me',
      scope text not null default 'personal'
    );
    create unique index pages_live_slug on pages(layer, slug) where archived_at is null;
    create virtual table pages_fts using fts5(search_text, content='pages', tokenize='unicode61 remove_diacritics 2');
    create trigger pages_fts_ai after insert on pages begin insert into pages_fts(rowid, search_text) values (new.rowid, new.search_text); end;
    create trigger pages_fts_ad after delete on pages begin insert into pages_fts(pages_fts, rowid, search_text) values ('delete', old.rowid, old.search_text); end;
    create trigger pages_fts_au after update of search_text on pages begin
      insert into pages_fts(pages_fts, rowid, search_text) values ('delete', old.rowid, old.search_text);
      insert into pages_fts(rowid, search_text) values (new.rowid, new.search_text);
    end;

    -- Every change to memory, with the state it produced. Never edited; the only allowed update blanks
    -- the text of a page the user deleted for good.
    create table page_history(
      seq integer primary key,
      page_id text not null,
      version integer not null,
      op text not null,                   -- create, edit, hand_edit, archive, restore, reclassify, pin, delete
      by text not null,                   -- user, assistant, hand_edit, maintenance, import
      at integer not null,
      title text not null,
      body text not null,
      privacy text not null,
      note text not null default ''
    );
    create index page_history_page on page_history(page_id, seq);
    create trigger page_history_no_delete before delete on page_history begin select raise(abort, 'memory history is append-only'); end;
    create trigger page_history_only_scrub before update on page_history
      when not (new.body = '' and new.title = '[deleted]' and new.seq = old.seq and new.page_id = old.page_id and new.op = old.op and new.version = old.version)
      begin select raise(abort, 'memory history is append-only'); end;

    create table embeddings(
      page_id text not null,
      model text not null,
      version integer not null,           -- the page version this vector was made from
      vec blob not null,
      primary key (page_id, model)
    );
"#, r#"
    -- Raw conversations: kept as they were said, never edited, searchable. Incognito sessions never get a row.
    create table conversations(
      id text primary key,
      project text,
      started integer not null,
      ended integer,
      owner text not null default 'me',
      scope text not null default 'personal'
    );
    create table messages(
      id integer primary key,
      conv text not null references conversations(id),
      seq integer not null,
      role text not null,
      content text not null,
      search_text text not null,
      at integer not null
    );
    create index messages_conv on messages(conv, seq);
    create virtual table messages_fts using fts5(search_text, content='messages', content_rowid='id', tokenize='unicode61 remove_diacritics 2');
    create trigger messages_fts_ai after insert on messages begin insert into messages_fts(rowid, search_text) values (new.id, new.search_text); end;
    create trigger messages_no_update before update on messages begin select raise(abort, 'conversations are never edited'); end;

    -- Things the harness suggests remembering, waiting for the user to accept, edit or drop.
    create table proposals(
      id integer primary key,
      kind text not null check (kind in ('new','update','archive')),
      layer text not null,
      title text not null,
      body text not null,
      gist text not null default '',
      target_page text,                   -- for update and archive
      privacy text not null default 'normal',
      privacy_reason text not null default '',
      evidence text not null default '',  -- a short quote from the conversation
      conv text,
      status text not null default 'pending' check (status in ('pending','accepted','rejected')),
      created integer not null,
      decided integer,
      owner text not null default 'me',
      scope text not null default 'personal'
    );
"#, r#"
    -- Learning. Rules say how the user wants things done; the style profile says how to talk to them;
    -- signals are the raw material (corrections, stated rules, thumbs) that suggestions come from.
    create table rules(
      id text primary key,
      topic text not null default '',      -- rules about the same topic compete; the most specific one wins
      text text not null,
      project text,                        -- null = everywhere
      role text,                           -- null = every role
      privacy text not null default 'normal' check (privacy in ('normal','sensitive')),
      status text not null default 'active' check (status in ('active','disabled')),
      evidence text not null default '[]', -- JSON list of short quotes
      source text not null default 'user', -- proposal | stated | user
      version integer not null default 1,
      created integer not null,
      updated integer not null,
      owner text not null default 'me',
      scope text not null default 'personal'
    );
    create table style_profile(
      id text primary key,
      text text not null,
      version integer not null,
      updated integer not null,
      owner text not null default 'me',
      scope text not null default 'personal'
    );
    -- Every version of every rule, style profile and skill, as it was. Rollback adds a new version
    -- that copies an old one; nothing here is edited or removed. The one allowed change blanks the
    -- content of something the user deleted for good.
    create table learn_history(
      seq integer primary key,
      kind text not null,                  -- rule | style | skill
      item text not null,
      version integer not null,
      op text not null,                    -- create | edit | disable | enable | rollback | accept | delete
      by text not null,
      at integer not null,
      content text not null,               -- JSON snapshot
      note text not null default ''
    );
    create index learn_history_item on learn_history(kind, item, seq);
    create trigger learn_history_no_delete before delete on learn_history begin select raise(abort, 'learning history is append-only'); end;
    create trigger learn_history_only_scrub before update on learn_history
      when not (new.content = '{}' and new.seq = old.seq and new.kind = old.kind and new.item = old.item and new.version = old.version and new.op = old.op)
      begin select raise(abort, 'learning history is append-only'); end;

    create table signals(
      id integer primary key,
      at integer not null,
      session text not null,
      kind text not null,                  -- stated | correction | thumbs_up | thumbs_down
      text text not null,
      norm text not null,                  -- words used to compare signals with each other
      project text,
      privacy text not null default 'normal',
      owner text not null default 'me',
      scope text not null default 'personal'
    );
    create index signals_session on signals(session);

    create table learn_proposals(
      id integer primary key,
      kind text not null,                  -- rule | rule_update | style | skill_new | skill_update
      payload text not null,               -- JSON; shape depends on kind
      evidence text not null default '[]',
      source text not null,                -- stated | repeat | correction | skill
      status text not null default 'pending' check (status in ('pending','accepted','rejected')),
      session text,
      created integer not null,
      decided integer,
      owner text not null default 'me',
      scope text not null default 'personal'
    );
"#, r#"
    -- remembers that a group of similar signals was already put to the model, so it is not asked again
    alter table signals add column asked integer not null default 0;
"#, r#"
    -- Model types: a role asks for a kind of model (frontier, mid-tier, fastest, or one the user made)
    -- and the type says which model that means today. The three stock types start without a model.
    create table model_types(
      name text primary key,
      model text,                          -- null until the user (or onboarding) assigns one
      fallbacks text not null default '[]',-- JSON list of model names, tried in order when `model` is unavailable
      stock integer not null default 0,
      position integer not null default 0
    );
    insert into model_types(name, stock, position) values ('frontier', 1, 1), ('mid-tier', 1, 2), ('fastest', 1, 3);

    -- Plain settings the user can change: parallelism, autonomy level, what to do when a model is missing.
    create table settings(
      key text primary key,
      value text not null,
      updated integer not null default 0
    );
"#, r#"
    -- Paired devices (browsers, later phones). A device is known by its public key; the private key
    -- never leaves the device. Revoking keeps the row so the activity log still makes sense.
    create table devices(
      id text primary key,
      name text not null,
      public_key text not null,
      created integer not null,
      last_seen integer not null default 0,
      revoked integer not null default 0
    );
"#, r#"
    -- What a stored chat needs to be opened again: the model it last used, how it was started, a short title.
    alter table conversations add column model text;
    alter table conversations add column mode text;
    alter table conversations add column title text;
"#, r#"
    -- A chat is a tree: each message says which message it answers. A new message goes under the node the chat
    -- is at now, so going back to an earlier message and writing again starts a new branch and loses nothing.
    alter table messages add column parent integer;                 -- the message this one answers; null = the first
    -- Chats stored before this were one straight line: each message answers the one before it. The one allowed edit
    -- (setting that once) is made with the guard lifted, then the guard goes back exactly as it was.
    drop trigger messages_no_update;
    update messages set parent = (select m2.id from messages m2 where m2.conv = messages.conv and m2.seq < messages.seq order by m2.seq desc limit 1);
    create trigger messages_no_update before update on messages begin select raise(abort, 'conversations are never edited'); end;

    -- Where each chat is now: the message the next one will answer (0 = before the first). No row means the newest message.
    create table chat_branch(conv text primary key, leaf integer not null);
    -- A name the user gave a message, to find it again in the tree.
    create table chat_labels(msg integer primary key, label text not null);
"#, r#"
    -- Tasks the core runs by itself at set times (see schedules.rs).
    create table schedules(
      id integer primary key,
      name text not null,
      prompt text not null,
      folder text not null,
      model text not null,
      run_when text not null,             -- "every 2h" or "daily 08:00"
      enabled integer not null default 1,
      next_run integer not null,          -- unix seconds
      last_run integer,
      last_result text not null default '',
      created integer not null
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
    fn chats_stored_before_the_tree_become_one_straight_branch_and_stay_uneditable() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("pragma journal_mode=wal;").unwrap();
        // run every migration before the one that made chats a tree
        let before = MIGRATIONS.iter().position(|m| m.contains("create table chat_branch")).unwrap();
        for (i, m) in MIGRATIONS.iter().enumerate().take(before) {
            conn.execute_batch(m).unwrap();
            conn.execute_batch(&format!("pragma user_version = {}", i + 1)).unwrap();
        }
        conn.execute("insert into conversations(id, started) values ('s1', 1), ('s2', 1)", []).unwrap();
        for (conv, seq, role) in [("s1", 1, "user"), ("s1", 2, "assistant"), ("s1", 3, "user"), ("s2", 1, "user"), ("s2", 2, "assistant")] {
            conn.execute("insert into messages(conv, seq, role, content, search_text, at) values (?1,?2,?3,'x','x',1)", (conv, seq, role)).unwrap();
        }
        let s = Store::init(conn).unwrap();
        let parents: Vec<(i64, Option<i64>)> = s.with(|c| {
            let mut q = c.prepare("select seq, parent from messages where conv = 's1' order by seq")?;
            let rows = q.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
            rows.collect()
        }).unwrap();
        let ids: Vec<i64> = s.with(|c| {
            let mut q = c.prepare("select id from messages where conv = 's1' order by seq")?;
            let rows = q.query_map([], |r| r.get(0))?;
            rows.collect()
        }).unwrap();
        assert_eq!(parents, vec![(1, None), (2, Some(ids[0])), (3, Some(ids[1]))], "a straight line, first message has no parent");
        let other: Option<i64> = s.with(|c| c.query_row("select parent from messages where conv = 's2' and seq = 2", [], |r| r.get(0))).unwrap();
        assert!(other.is_some() && !ids.contains(&other.unwrap()), "another chat's messages never join this one");
        assert!(s.with(|c| c.execute("update messages set parent = null", [])).is_err(), "conversations are still never edited");
        assert!(s.with(|c| c.execute("update messages set content = 'y'", [])).is_err());
    }

    #[test]
    fn the_activity_log_cannot_be_edited() {
        let s = Store::open_memory().unwrap();
        s.with(|c| c.execute("insert into activity(at,kind,session,actor,detail) values (1,'k','s','a','{}')", [])).unwrap();
        assert!(s.with(|c| c.execute("update activity set kind='x'", [])).is_err());
        assert!(s.with(|c| c.execute("delete from activity", [])).is_err());
    }
}
