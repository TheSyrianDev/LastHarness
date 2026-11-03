use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use lh_store::rusqlite::{self, params, OptionalExtension, Row};
use lh_store::{content_hash, now, Store};

use crate::page::*;
use crate::text;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Store(#[from] lh_store::Error),
    #[error("file problem: {0}")]
    Io(#[from] std::io::Error),
    #[error("no such page: {0}")]
    NotFound(String),
    #[error("{0}")]
    Invalid(String),
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Live,
    Archived,
    Any,
}

#[derive(Clone, Debug)]
pub struct Hit {
    pub page: Page,
    /// Higher is better.
    pub score: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HistoryRow {
    pub seq: i64,
    pub version: i64,
    pub op: String,
    pub by: String,
    pub at: i64,
    pub title: String,
    pub body: String,
    pub note: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScanReport {
    pub created: Vec<String>,
    pub edited: Vec<String>,
    pub archived: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImportReport {
    pub imported: usize,
    pub already_present: usize,
}

const COLS: &str = "p.id, p.layer, p.slug, p.title, p.body, p.gist, p.privacy, p.privacy_reason, p.pinned, p.project, p.source, p.version, p.archived_at, p.created, p.updated";

fn page_from(r: &Row) -> rusqlite::Result<Page> {
    let bad = |what: &str| rusqlite::Error::InvalidColumnType(0, what.into(), rusqlite::types::Type::Text);
    Ok(Page {
        id: r.get(0)?,
        layer: Layer::parse(&r.get::<_, String>(1)?).ok_or_else(|| bad("layer"))?,
        slug: r.get(2)?,
        title: r.get(3)?,
        body: r.get(4)?,
        gist: r.get(5)?,
        privacy: Privacy::parse(&r.get::<_, String>(6)?).ok_or_else(|| bad("privacy"))?,
        privacy_reason: r.get(7)?,
        pinned: r.get::<_, i64>(8)? != 0,
        project: r.get(9)?,
        source: r.get(10)?,
        version: r.get(11)?,
        archived_at: r.get(12)?,
        created: r.get(13)?,
        updated: r.get(14)?,
    })
}

pub struct Memory {
    pub(crate) store: Arc<Store>,
    root: PathBuf,
    pub(crate) cache: std::sync::Mutex<Option<Arc<crate::vectors::VectorCache>>>,
    /// What the page files looked like at the last scan (names, sizes, times), so an unchanged folder is not read.
    last_look: std::sync::Mutex<Option<u64>>,
}

fn new_id() -> String {
    let mut b = [0u8; 4];
    getrandom::fill(&mut b).expect("the OS random source is unavailable");
    format!("p_{}", b.iter().map(|x| format!("{x:02x}")).collect::<String>())
}

fn write_atomic(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    let tmp = path.with_extension("md.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)
}

impl Memory {
    /// `root` is the memory folder: one sub-folder per layer, plus `_archive`.
    pub fn open(store: Arc<Store>, root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        for l in Layer::ALL {
            std::fs::create_dir_all(root.join(l.as_str()))?;
        }
        Ok(Self { store, root, cache: Default::default(), last_look: Default::default() })
    }

    #[doc(hidden)]
    pub fn store_for_tests(&self) -> &Store {
        &self.store
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
    fn dir(&self, l: Layer) -> PathBuf {
        self.root.join(l.as_str())
    }
    fn archive_dir(&self, l: Layer) -> PathBuf {
        self.root.join("_archive").join(l.as_str())
    }
    fn live_path(&self, p: &Page) -> PathBuf {
        self.dir(p.layer).join(format!("{}.md", p.slug))
    }

    // ---- reading ----

    pub fn get(&self, id: &str) -> Result<Page> {
        self.store
            .with(|c| c.query_row(&format!("select {COLS} from pages p where p.id = ?1"), [id], page_from).optional())?
            .ok_or_else(|| Error::NotFound(id.into()))
    }

    pub fn list(&self, layer: Option<Layer>, state: State) -> Result<Vec<Page>> {
        let l = layer.map(|l| l.as_str());
        Ok(self.store.with(|c| {
            let mut q = c.prepare(&format!(
                "select {COLS} from pages p where (?1 is null or p.layer = ?1) and (?2 = 'any' or (?2 = 'live' and p.archived_at is null) or (?2 = 'archived' and p.archived_at is not null)) order by p.layer, p.title"
            ))?;
            let s = match state {
                State::Live => "live",
                State::Archived => "archived",
                State::Any => "any",
            };
            let rows = q.query_map(params![l, s], page_from)?;
            rows.collect()
        })?)
    }

    pub fn history(&self, id: &str) -> Result<Vec<HistoryRow>> {
        Ok(self.store.with(|c| {
            let mut q = c.prepare("select seq, version, op, by, at, title, body, note from page_history where page_id = ?1 order by seq")?;
            let rows = q.query_map([id], |r| Ok(HistoryRow { seq: r.get(0)?, version: r.get(1)?, op: r.get(2)?, by: r.get(3)?, at: r.get(4)?, title: r.get(5)?, body: r.get(6)?, note: r.get(7)? }))?;
            rows.collect()
        })?)
    }

    /// Keyword search (always available). Archived pages are searchable on request.
    pub fn search(&self, query: &str, state: State, limit: usize) -> Result<Vec<Hit>> {
        let toks = text::tokens(query);
        if toks.is_empty() {
            return Ok(vec![]);
        }
        let fts = toks.iter().map(|t| format!("\"{}\"{}", t.replace('"', ""), if t.chars().count() >= 3 { "*" } else { "" })).collect::<Vec<_>>().join(" OR ");
        let s = match state {
            State::Live => "live",
            State::Archived => "archived",
            State::Any => "any",
        };
        Ok(self.store.with(|c| {
            let mut q = c.prepare(&format!(
                "select {COLS}, bm25(pages_fts) from pages_fts join pages p on p.rowid = pages_fts.rowid where pages_fts match ?1 and (?2 = 'any' or (?2 = 'live' and p.archived_at is null) or (?2 = 'archived' and p.archived_at is not null)) order by bm25(pages_fts) limit ?3"
            ))?;
            let rows = q.query_map(params![fts, s, limit as i64], |r| Ok(Hit { page: page_from(r)?, score: -r.get::<_, f64>(15)? }))?;
            rows.collect()
        })?)
    }

    // ---- writing ----

    fn unique_slug(&self, layer: Layer, title: &str) -> Result<String> {
        let base = text::slugify(title);
        for n in 1.. {
            let slug = if n == 1 { base.clone() } else { format!("{base}-{n}") };
            let taken: bool = self.store.with(|c| c.query_row("select exists(select 1 from pages where layer = ?1 and slug = ?2 and archived_at is null)", params![layer.as_str(), slug], |r| r.get(0)))?;
            if !taken && !self.dir(layer).join(format!("{slug}.md")).exists() {
                return Ok(slug);
            }
        }
        unreachable!()
    }

    fn insert(&self, p: &Page, hash: i64) -> Result<()> {
        self.store.with(|c| {
            c.execute(
                "insert into pages(id, layer, slug, title, body, search_text, gist, privacy, privacy_reason, pinned, project, source, version, archived_at, file_hash, created, updated) values (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)",
                params![p.id, p.layer.as_str(), p.slug, p.title, p.body, text::search_text(&p.title, &p.body), p.gist, p.privacy.as_str(), p.privacy_reason, p.pinned as i64, p.project, p.source, p.version, p.archived_at, hash, p.created, p.updated],
            )
            .map(|_| ())
        })?;
        Ok(())
    }

    fn save(&self, p: &Page, hash: i64) -> Result<()> {
        // any change to a page can change which vectors are valid
        *self.cache.lock().unwrap() = None;
        self.store.with(|c| {
            c.execute(
                "update pages set slug=?2, title=?3, body=?4, search_text=?5, gist=?6, privacy=?7, privacy_reason=?8, pinned=?9, project=?10, source=?11, version=?12, archived_at=?13, file_hash=?14, updated=?15 where id=?1",
                params![p.id, p.slug, p.title, p.body, text::search_text(&p.title, &p.body), p.gist, p.privacy.as_str(), p.privacy_reason, p.pinned as i64, p.project, p.source, p.version, p.archived_at, hash, p.updated],
            )
            .map(|_| ())
        })?;
        Ok(())
    }

    fn note_history(&self, p: &Page, op: &str, by: By, note: &str) -> Result<()> {
        self.store.with(|c| {
            c.execute(
                "insert into page_history(page_id, version, op, by, at, title, body, privacy, note) values (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                params![p.id, p.version, op, by.as_str(), now(), p.title, p.body, p.privacy.as_str(), note],
            )
            .map(|_| ())
        })?;
        // The log records that a change happened, never its text: titles can be private too.
        let _ = self.store.log("memory", "-", by.as_str(), &serde_json::json!({"op": op, "page": p.id, "version": p.version}));
        Ok(())
    }

    /// Keep the old version readable: `_archive/<layer>/<slug>-v<N>.md`.
    fn stash(&self, old: &Page) -> Result<()> {
        write_atomic(&self.archive_dir(old.layer).join(format!("{}-v{}.md", old.slug, old.version)), &old.file_text())?;
        Ok(())
    }

    fn write_live(&self, p: &Page) -> Result<i64> {
        let t = p.file_text();
        write_atomic(&self.live_path(p), &t)?;
        Ok(content_hash(t.as_bytes()))
    }

    pub fn create(&self, new: NewPage, by: By) -> Result<Page> {
        self.create_with(new, None, None, by)
    }

    fn create_with(&self, new: NewPage, slug: Option<String>, id: Option<String>, by: By) -> Result<Page> {
        if new.title.trim().is_empty() || new.body.trim().is_empty() {
            return Err(Error::Invalid("a page needs a title and some text".into()));
        }
        let t = now();
        let p = Page {
            id: id.unwrap_or_else(new_id),
            layer: new.layer,
            slug: match slug {
                Some(s) => s,
                None => self.unique_slug(new.layer, &new.title)?,
            },
            title: new.title.trim().into(),
            body: new.body.trim().into(),
            gist: new.gist.replace('\n', " ").trim().into(),
            privacy: new.privacy,
            privacy_reason: new.privacy_reason,
            pinned: new.pinned,
            project: new.project,
            source: new.source,
            version: 1,
            archived_at: None,
            created: t,
            updated: t,
        };
        let hash = self.write_live(&p)?;
        self.insert(&p, hash)?;
        self.note_history(&p, "create", by, "")?;
        self.write_indexes()?;
        Ok(p)
    }

    pub fn update(&self, id: &str, edit: Edit, by: By) -> Result<Page> {
        let old = self.get(id)?;
        if old.archived_at.is_some() {
            return Err(Error::Invalid("this page is archived; restore it first".into()));
        }
        let mut p = old.clone();
        if let Some(t) = edit.title {
            p.title = t.trim().into();
        }
        if let Some(b) = edit.body {
            p.body = b.trim().into();
        }
        if let Some(g) = edit.gist {
            p.gist = g.replace('\n', " ").trim().into();
        }
        if p.title.is_empty() || p.body.is_empty() {
            return Err(Error::Invalid("a page needs a title and some text".into()));
        }
        if (p.title.as_str(), p.body.as_str(), p.gist.as_str()) == (old.title.as_str(), old.body.as_str(), old.gist.as_str()) {
            return Ok(old);
        }
        self.stash(&old)?;
        p.version += 1;
        p.updated = now();
        let hash = self.write_live(&p)?;
        self.save(&p, hash)?;
        self.note_history(&p, "edit", by, "")?;
        self.write_indexes()?;
        Ok(p)
    }

    pub fn archive(&self, id: &str, by: By, note: &str) -> Result<Page> {
        let mut p = self.get(id)?;
        if p.archived_at.is_some() {
            return Ok(p);
        }
        let live = self.live_path(&p);
        p.archived_at = Some(now());
        p.updated = now();
        write_atomic(&self.archive_dir(p.layer).join(format!("{}-archived.md", p.slug)), &p.file_text())?;
        let _ = std::fs::remove_file(&live);
        self.save(&p, 0)?;
        self.note_history(&p, "archive", by, note)?;
        self.write_indexes()?;
        Ok(p)
    }

    pub fn restore(&self, id: &str, by: By) -> Result<Page> {
        let mut p = self.get(id)?;
        if p.archived_at.is_none() {
            return Ok(p);
        }
        p.slug = self.unique_slug(p.layer, &p.title)?;
        p.archived_at = None;
        p.updated = now();
        let hash = self.write_live(&p)?;
        self.save(&p, hash)?;
        self.note_history(&p, "restore", by, "")?;
        self.write_indexes()?;
        Ok(p)
    }

    /// Remove a page for good: its files, its vectors, and the text in its history.
    pub fn delete(&self, id: &str, by: By) -> Result<()> {
        let p = self.get(id)?;
        let _ = std::fs::remove_file(self.live_path(&p));
        if let Ok(rd) = std::fs::read_dir(self.archive_dir(p.layer)) {
            for e in rd.flatten() {
                if std::fs::read_to_string(e.path()).is_ok_and(|t| text::parse_file(&t).get("id") == Some(id)) {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
        *self.cache.lock().unwrap() = None;
        self.store.with(|c| {
            c.execute("update page_history set body = '', title = '[deleted]' where page_id = ?1", [id])?;
            c.execute("delete from embeddings where page_id = ?1", [id])?;
            c.execute("delete from pages where id = ?1", [id]).map(|_| ())
        })?;
        let gone = Page { title: "[deleted]".into(), body: String::new(), ..p };
        self.note_history(&gone, "delete", by, "")?;
        self.write_indexes()
    }

    pub fn set_privacy(&self, id: &str, privacy: Privacy, reason: &str, by: By) -> Result<Page> {
        let mut p = self.get(id)?;
        if p.privacy == privacy {
            return Ok(p);
        }
        p.privacy = privacy;
        p.privacy_reason = reason.into();
        p.version += 1;
        p.updated = now();
        let hash = if p.archived_at.is_none() { self.write_live(&p)? } else { 0 };
        self.save(&p, hash)?;
        self.note_history(&p, "reclassify", by, reason)?;
        Ok(p)
    }

    pub fn set_pinned(&self, id: &str, pinned: bool, by: By) -> Result<Page> {
        let mut p = self.get(id)?;
        if p.pinned == pinned {
            return Ok(p);
        }
        p.pinned = pinned;
        p.version += 1;
        p.updated = now();
        let hash = if p.archived_at.is_none() { self.write_live(&p)? } else { 0 };
        self.save(&p, hash)?;
        self.note_history(&p, "pin", by, "")?;
        Ok(p)
    }

    // ---- files ----

    /// One `index.md` per layer: a plain list of pages with their gists. Generated; edits to it are not kept.
    pub fn write_indexes(&self) -> Result<()> {
        for l in Layer::ALL {
            let mut s = format!("# {}\n\n", match l {
                Layer::Preferences => "Preferences",
                Layer::Facts => "Facts about you",
                Layer::Projects => "Projects and decisions",
            });
            for p in self.list(Some(l), State::Live)? {
                s.push_str(&format!("- [[{}]] {}{}\n", p.slug, p.title, if p.gist.is_empty() { String::new() } else { format!(" - {}", p.gist) }));
            }
            write_atomic(&self.dir(l).join("index.md"), &s)?;
        }
        Ok(())
    }

    /// Pick up changes made to the files by hand: new files, edited files, removed files.
    pub fn scan(&self) -> Result<ScanReport> {
        self.scan_as(By::HandEdit)
    }

    /// Like `scan`, but only when a page file was added, removed or changed since the last look. Costs one
    /// directory listing and one file-information call per page, without reading any file.
    pub fn scan_if_changed(&self) -> Result<ScanReport> {
        let look = self.look();
        if *self.last_look.lock().unwrap() == Some(look) {
            return Ok(ScanReport::default());
        }
        let rep = self.scan()?;
        // the scan may rewrite files (a new file gets its header), so look again afterwards
        *self.last_look.lock().unwrap() = Some(self.look());
        Ok(rep)
    }

    fn look(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        for layer in Layer::ALL {
            let mut entries: Vec<(String, u64, Option<std::time::SystemTime>)> = std::fs::read_dir(self.dir(layer))
                .map(|rd| rd.flatten().filter_map(|e| e.metadata().ok().map(|m| (e.file_name().to_string_lossy().to_string(), m.len(), m.modified().ok()))).collect())
                .unwrap_or_default();
            entries.sort();
            entries.hash(&mut h);
        }
        h.finish()
    }

    fn scan_as(&self, by: By) -> Result<ScanReport> {
        let mut rep = ScanReport::default();
        let mut seen: HashSet<String> = HashSet::new();
        for layer in Layer::ALL {
            let mut files: Vec<PathBuf> = std::fs::read_dir(self.dir(layer))?.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "md") && p.file_stem().is_some_and(|s| s != "index")).collect();
            files.sort();
            for path in files {
                let raw = std::fs::read_to_string(&path)?;
                let parts = text::parse_file(&raw);
                let stem = path.file_stem().unwrap().to_string_lossy().to_string();
                let existing = match parts.get("id") {
                    Some(id) => Some(self.get(id).ok()).flatten(),
                    None => None,
                };
                let header_privacy = parts.get("privacy").and_then(Privacy::parse);
                let title = if parts.title.is_empty() { stem.clone() } else { parts.title.clone() };
                match existing {
                    Some(old) if old.archived_at.is_none() => {
                        seen.insert(old.id.clone());
                        let hash = content_hash(raw.as_bytes());
                        let row_hash: i64 = self.store.with(|c| c.query_row("select file_hash from pages where id = ?1", [&old.id], |r| r.get(0)))?;
                        if hash == row_hash {
                            continue;
                        }
                        let mut p = old.clone();
                        p.title = title;
                        p.body = parts.body.clone();
                        p.gist = parts.get("gist").unwrap_or(&old.gist).to_string();
                        // a hand edit can make a page more private, never less: the header line could have been changed by
                        // anything that writes files, and lowering it would let the page go to cloud models
                        p.privacy = if old.privacy == Privacy::Sensitive { Privacy::Sensitive } else { header_privacy.unwrap_or(old.privacy) };
                        let found = crate::privacy::detect(&format!("{}\n{}", p.title, p.body));
                        if found.privacy == Privacy::Sensitive && p.privacy == Privacy::Normal {
                            p.privacy = Privacy::Sensitive;
                            p.privacy_reason = found.reason;
                        }
                        p.pinned = parts.get("pinned").map_or(old.pinned, |v| v == "true");
                        p.project = parts.get("project").map(str::to_string).filter(|s| !s.is_empty()).or(old.project.clone());
                        p.slug = stem;
                        let changed = (&p.title, &p.body, &p.gist, p.privacy, p.pinned, &p.project) != (&old.title, &old.body, &old.gist, old.privacy, old.pinned, &old.project);
                        if changed {
                            if p.body.trim().is_empty() {
                                continue; // an emptied file is not a decision to forget; leave it for the user
                            }
                            self.stash(&old)?;
                            p.version += 1;
                            p.updated = now();
                        }
                        // the file says what is true: put the privacy kept or found back into its header
                        let hash = if header_privacy.is_some_and(|h| h != p.privacy) { self.write_live(&p)? } else { hash };
                        self.save(&p, hash)?;
                        if changed {
                            let note = if header_privacy == Some(Privacy::Normal) && old.privacy == Privacy::Sensitive { "privacy stays sensitive; a hand edit cannot lower it" } else { "" };
                            self.note_history(&p, "hand_edit", by, note)?;
                            rep.edited.push(p.id.clone());
                        }
                    }
                    Some(_) => {} // an archived page whose id shows up in a live folder: leave it alone
                    None => {
                        if parts.body.trim().is_empty() {
                            continue;
                        }
                        let mut np = NewPage::new(layer, title.clone(), parts.body.clone());
                        np.gist = parts.get("gist").unwrap_or("").to_string();
                        np.privacy = header_privacy.unwrap_or(Privacy::Normal);
                        let found = crate::privacy::detect(&format!("{title}\n{}", parts.body));
                        if found.privacy == Privacy::Sensitive && np.privacy == Privacy::Normal {
                            np.privacy = Privacy::Sensitive;
                            np.privacy_reason = found.reason;
                        }
                        np.pinned = parts.get("pinned") == Some("true");
                        np.project = parts.get("project").map(str::to_string).filter(|s| !s.is_empty());
                        np.source = parts.get("source").unwrap_or("hand").to_string();
                        let id = parts.get("id").map(str::to_string);
                        let slug = if self.slug_free(layer, &stem) { stem.clone() } else { self.unique_slug(layer, &stem)? };
                        // create_with rewrites the file with a full header, keeping the user's text
                        let _ = std::fs::remove_file(&path);
                        let p = self.create_with(np, Some(slug), id, by)?;
                        seen.insert(p.id.clone());
                        rep.created.push(p.id);
                    }
                }
            }
        }
        // live pages whose file disappeared are archived, not deleted
        for p in self.list(None, State::Live)? {
            if !seen.contains(&p.id) && !self.live_path(&p).exists() {
                self.archive(&p.id, by, "its file was removed")?;
                rep.archived.push(p.id);
            }
        }
        if !rep.created.is_empty() || !rep.edited.is_empty() || !rep.archived.is_empty() {
            self.write_indexes()?;
        }
        Ok(rep)
    }

    fn slug_free(&self, layer: Layer, slug: &str) -> bool {
        !self.store.with(|c| c.query_row::<bool, _, _>("select exists(select 1 from pages where layer = ?1 and slug = ?2 and archived_at is null)", params![layer.as_str(), slug], |r| r.get(0))).unwrap_or(true)
    }

    // ---- export and import ----

    /// Write the memory folders, the archive and the conversations as Markdown into `dest`.
    pub fn export(&self, dest: &Path) -> Result<()> {
        copy_dir(&self.root, dest)?;
        let convs = dest.join("conversations");
        std::fs::create_dir_all(&convs)?;
        let rows: Vec<(String, Option<String>, i64)> = self.store.with(|c| {
            let mut q = c.prepare("select id, project, started from conversations order by started")?;
            let rows = q.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
            rows.collect()
        })?;
        for (id, project, started) in rows {
            let msgs: Vec<(String, String)> = self.store.with(|c| {
                let mut q = c.prepare("select role, content from messages where conv = ?1 order by seq")?;
                let rows = q.query_map([&id], |r| Ok((r.get(0)?, r.get(1)?)))?;
                rows.collect()
            })?;
            let mut s = format!("---\nid: {id}\nstarted: {started}\nproject: {}\n---\n# Conversation {id}\n\n", project.unwrap_or_default());
            for (role, content) in msgs {
                s.push_str(&format!("**{role}:**\n\n{content}\n\n"));
            }
            write_atomic(&convs.join(format!("{id}.md")), &s)?;
        }
        Ok(())
    }

    /// Read pages from an exported folder. Pages already here (same id) are left alone.
    pub fn import(&self, src: &Path) -> Result<ImportReport> {
        let mut rep = ImportReport::default();
        for layer in Layer::ALL {
            let Ok(rd) = std::fs::read_dir(src.join(layer.as_str())) else { continue };
            for e in rd.flatten() {
                let path = e.path();
                if path.extension().is_none_or(|x| x != "md") || path.file_stem().is_some_and(|s| s == "index") {
                    continue;
                }
                let raw = std::fs::read_to_string(&path)?;
                if let Some(id) = text::parse_file(&raw).get("id") {
                    if self.get(id).is_ok() {
                        rep.already_present += 1;
                        continue;
                    }
                }
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                let mut dest = self.dir(layer).join(&name);
                let mut n = 1;
                while dest.exists() {
                    n += 1;
                    dest = self.dir(layer).join(format!("{}-{n}.md", path.file_stem().unwrap().to_string_lossy()));
                }
                std::fs::write(&dest, raw)?;
                rep.imported += 1;
            }
        }
        self.scan_as(By::Import)?;
        Ok(rep)
    }
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)?.flatten() {
        let (src, dst) = (e.path(), to.join(e.file_name()));
        if e.file_type()?.is_dir() {
            copy_dir(&src, &dst)?;
        } else if src.extension().is_some_and(|x| x == "md") {
            std::fs::copy(&src, &dst)?;
        }
    }
    Ok(())
}
