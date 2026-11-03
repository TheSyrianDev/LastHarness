//! Saving the model's KV cache (the work it did reading a chat) per local chat, so a stored chat can go on
//! without the whole chat being read again. A 32k-token chat on a large local model takes about six minutes
//! to read and under a second to restore.
//!
//! How it works: after a turn the core asks the local server (llama-server, behind llama-swap or alone) to save
//! its slot to a file named for the chat and the model; before the first turn of a chat that was opened again it
//! asks for the file back. The server writes the file into its own save folder, so the user tells the core where
//! that folder is (`kvcache.dir`); the core then fixes the file's mode to 0600, keeps the folder under a size cap
//! (oldest first), and deletes a chat's files when the chat is deleted. A file whose folder the core cannot find
//! is a file it cannot protect, so the feature switches itself off then.
//!
//! Only for local endpoints, never for private chats (they have no stored conversation), nothing for cloud.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde_json::{json, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Style {
    /// llama-swap: `/upstream/<model>/slots/0`.
    Swap,
    /// llama-server on its own: `/slots/0`.
    Direct,
}

pub struct KvCache {
    root: String,
    dir: PathBuf,
    max_bytes: u64,
    key: Option<String>,
    http: reqwest::Client,
    /// How a model's slot calls were answered last time: the style, and the name used in the address.
    styles: Mutex<HashMap<String, (Style, String)>>,
    off: AtomicBool,
    why_off: Mutex<Option<String>>,
}

#[derive(Debug, PartialEq)]
pub struct Saved {
    pub bytes: u64,
    pub tokens: u64,
    pub path: PathBuf,
}

fn fnv(s: &str) -> u64 {
    s.bytes().fold(0xcbf29ce484222325, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3))
}

fn clean(id: &str) -> String {
    id.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect()
}

fn enc(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-._~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

/// `http://localhost:9292/v1` -> `http://localhost:9292`.
fn root_of(provider: &str) -> String {
    let p = provider.trim_end_matches('/');
    p.strip_suffix("/v1").unwrap_or(p).to_string()
}

impl KvCache {
    pub fn new(provider_url: &str, key: Option<String>, dir: PathBuf, max_gb: u64) -> KvCache {
        KvCache { root: root_of(provider_url), dir, max_bytes: max_gb.max(1).saturating_mul(1_000_000_000), key, http: reqwest::Client::builder().timeout(Duration::from_secs(120)).build().unwrap_or_default(), styles: Mutex::new(HashMap::new()), off: AtomicBool::new(false), why_off: Mutex::new(None) }
    }

    /// For tests: a cap in bytes.
    pub fn with_max_bytes(mut self, n: u64) -> KvCache {
        self.max_bytes = n;
        self
    }

    pub fn is_off(&self) -> bool {
        self.off.load(Ordering::SeqCst)
    }

    pub fn why_off(&self) -> Option<String> {
        self.why_off.lock().unwrap().clone()
    }

    fn switch_off(&self, why: &str) {
        self.off.store(true, Ordering::SeqCst);
        *self.why_off.lock().unwrap() = Some(why.to_string());
    }

    /// The file name for a chat and a model: one file per chat, and a different model never matches it.
    pub fn file_name(chat: &str, model: &str) -> String {
        format!("lh-{}-{:08x}.bin", clean(chat), fnv(model) as u32)
    }

    fn url(&self, style: Style, model: &str, action: &str) -> String {
        match style {
            Style::Swap => format!("{}/upstream/{}/slots/0?action={action}", self.root, enc(model)),
            Style::Direct => format!("{}/slots/0?action={action}", self.root),
        }
    }

    /// What to try for `model`: what worked before, else llama-swap with the full id, llama-swap without a `:mode`
    /// suffix (the router's aliases such as `model:instruct` are not models), then a bare llama-server.
    fn candidates(&self, model: &str) -> Vec<(Style, String)> {
        let mut v: Vec<(Style, String)> = vec![];
        if let Some(k) = self.styles.lock().unwrap().get(model) {
            v.push(k.clone());
        }
        for c in [(Style::Swap, model.to_string()), (Style::Swap, model.split(':').next().unwrap_or(model).to_string()), (Style::Direct, model.to_string())] {
            if !v.contains(&c) {
                v.push(c);
            }
        }
        v
    }

    async fn post(&self, style: Style, model: &str, action: &str, filename: &str) -> Result<Value, (u16, String)> {
        let mut rb = self.http.post(self.url(style, model, action)).json(&json!({"filename": filename}));
        if let Some(k) = &self.key {
            rb = rb.bearer_auth(k);
        }
        let r = rb.send().await.map_err(|e| (0, e.to_string()))?;
        let code = r.status().as_u16();
        let text = r.text().await.unwrap_or_default();
        if (200..300).contains(&code) {
            Ok(serde_json::from_str(&text).unwrap_or(Value::Null))
        } else {
            Err((code, text.chars().take(200).collect()))
        }
    }

    /// Find a file we named, wherever the server put it below `dir` (llama-swap setups often use one folder per model).
    pub fn find(&self, name: &str) -> Option<PathBuf> {
        fn walk(d: &Path, name: &str, depth: u8) -> Option<PathBuf> {
            for e in std::fs::read_dir(d).ok()?.flatten() {
                let p = e.path();
                if p.is_file() && p.file_name().is_some_and(|n| n == name) {
                    return Some(p);
                }
                if depth > 0 && p.is_dir() {
                    if let Some(f) = walk(&p, name, depth - 1) {
                        return Some(f);
                    }
                }
            }
            None
        }
        walk(&self.dir, name, 3)
    }

    /// Every cache file the core made, with size and age.
    fn files(&self) -> Vec<(PathBuf, u64, std::time::SystemTime)> {
        fn walk(d: &Path, depth: u8, out: &mut Vec<(PathBuf, u64, std::time::SystemTime)>) {
            let Ok(rd) = std::fs::read_dir(d) else { return };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() && depth > 0 {
                    walk(&p, depth - 1, out);
                } else if p.is_file() && p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("lh-") && n.ends_with(".bin")) {
                    if let Ok(m) = e.metadata() {
                        out.push((p, m.len(), m.modified().unwrap_or(std::time::UNIX_EPOCH)));
                    }
                }
            }
        }
        let mut v = vec![];
        walk(&self.dir, 3, &mut v);
        v
    }

    /// Save the slot for `chat` and `model`. Errors are for the activity log; a turn never fails because of them.
    pub async fn save(&self, chat: &str, model: &str) -> Result<Saved, String> {
        if self.is_off() {
            return Err(self.why_off().unwrap_or_else(|| "switched off".into()));
        }
        let name = Self::file_name(chat, model);
        let mut last = (0u16, String::new());
        let mut done = None;
        for (style, path_model) in self.candidates(model) {
            match self.post(style, &path_model, "save", &name).await {
                Ok(v) => {
                    self.styles.lock().unwrap().insert(model.to_string(), (style, path_model));
                    done = Some(v);
                    break;
                }
                Err(e) => last = e,
            }
        }
        let Some(v) = done else {
            // the server cannot save slots (cloud, LM Studio, Ollama, or no save folder): stay off for this run
            self.switch_off(&format!("the server cannot save a slot ({} {})", last.0, last.1));
            return Err(self.why_off().unwrap());
        };
        let Some(path) = self.find(&name) else {
            self.switch_off(&format!("the server saved {name}, but it is not under {}: the file cannot be protected or removed, so saving is off. Point kvcache.dir at the server's slot save folder", self.dir.display()));
            return Err(self.why_off().unwrap());
        };
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }
        let bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        self.evict(&path);
        Ok(Saved { bytes, tokens: v["n_saved"].as_u64().unwrap_or(0), path })
    }

    /// Put the saved cache back. `Ok(tokens)`; an error means nothing was restored and the chat is read again as usual.
    pub async fn restore(&self, chat: &str, model: &str) -> Result<u64, String> {
        if self.is_off() {
            return Err("switched off".into());
        }
        let name = Self::file_name(chat, model);
        if self.find(&name).is_none() {
            return Err("nothing saved for this chat with this model".into());
        }
        let mut last = (0u16, String::new());
        for (style, path_model) in self.candidates(model) {
            match self.post(style, &path_model, "restore", &name).await {
                Ok(v) => {
                    self.styles.lock().unwrap().insert(model.to_string(), (style, path_model));
                    return Ok(v["n_restored"].as_u64().unwrap_or(0));
                }
                Err(e) => last = e,
            }
        }
        Err(format!("the server would not restore it ({} {})", last.0, last.1))
    }

    /// Remove every cache file of a chat (all models). Returns how many.
    pub fn forget(&self, chat: &str) -> usize {
        let prefix = format!("lh-{}-", clean(chat));
        self.files().into_iter().filter(|(p, _, _)| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(&prefix))).filter(|(p, _, _)| std::fs::remove_file(p).is_ok()).count()
    }

    /// Keep the total under the cap by removing the oldest files, never the one just saved.
    pub fn evict(&self, keep: &Path) -> usize {
        let mut files = self.files();
        let mut total: u64 = files.iter().map(|f| f.1).sum();
        files.sort_by_key(|f| f.2);
        let mut removed = 0;
        for (p, size, _) in files {
            if total <= self.max_bytes {
                break;
            }
            if p == keep {
                continue;
            }
            if std::fs::remove_file(&p).is_ok() {
                total = total.saturating_sub(size);
                removed += 1;
            }
        }
        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_per_chat_and_per_model_and_safe() {
        let a = KvCache::file_name("s12", "Qwen3.6-balanced");
        assert!(a.starts_with("lh-s12-") && a.ends_with(".bin"));
        assert_ne!(a, KvCache::file_name("s12", "Qwen3.6-max"), "another model never matches");
        assert_ne!(a, KvCache::file_name("s13", "Qwen3.6-balanced"));
        assert_eq!(a, KvCache::file_name("s12", "Qwen3.6-balanced"), "the same every time");
        assert!(!KvCache::file_name("../../etc/x", "m").contains('/'));
    }

    #[test]
    fn a_router_alias_is_tried_as_its_model_and_what_worked_is_tried_first() {
        let k = KvCache::new("http://h:1/v1", None, PathBuf::from("/x"), 5);
        let c = k.candidates("Qwen3.6:instruct");
        assert_eq!(c[0], (Style::Swap, "Qwen3.6:instruct".to_string()));
        assert_eq!(c[1], (Style::Swap, "Qwen3.6".to_string()));
        assert_eq!(c[2].0, Style::Direct);
        assert_eq!(k.candidates("plain").len(), 2, "no suffix, no extra try");
        k.styles.lock().unwrap().insert("Qwen3.6:instruct".into(), (Style::Swap, "Qwen3.6".into()));
        assert_eq!(k.candidates("Qwen3.6:instruct")[0], (Style::Swap, "Qwen3.6".to_string()));
    }

    #[test]
    fn addresses_follow_the_two_server_styles() {
        let k = KvCache::new("http://localhost:9292/v1/", None, PathBuf::from("/x"), 5);
        assert_eq!(k.url(Style::Swap, "Qwen 3:x", "save"), "http://localhost:9292/upstream/Qwen%203%3Ax/slots/0?action=save");
        assert_eq!(k.url(Style::Direct, "m", "restore"), "http://localhost:9292/slots/0?action=restore");
        assert_eq!(root_of("http://h:1/v1"), "http://h:1");
        assert_eq!(root_of("http://h:1"), "http://h:1");
    }
}
