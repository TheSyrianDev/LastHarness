//! The one list of settings. Every behaviour choice has a key, a kind of value, a default and a plain
//! description here, and the settings screens, the first-run questions and the core all read this list.
//! Values live in `config.toml`; a key that is not there has its default.
//!
//! Some settings take effect at once (`live`; a few of those only in chats started afterwards, and their help
//! says so), the rest when the core next starts.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Invalid(String),
    #[error("cannot read or write {0}: {1}")]
    Io(String, String),
    #[error("{0} is not valid TOML: {1}")]
    Parse(String, String),
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Bool,
    Int { min: i64, max: i64 },
    /// A number of zero or more. An empty value means "no limit".
    OptFloat,
    /// `auto` or a whole number of zero or more.
    AutoInt,
    Choice(&'static [&'static str]),
    Text,
}

#[derive(Clone, Copy, Debug)]
pub struct Def {
    pub key: &'static str,
    pub kind: Kind,
    pub default: &'static str,
    /// Takes effect without restarting the core.
    pub live: bool,
    pub help: &'static str,
}

impl Def {
    pub fn group(&self) -> &'static str {
        self.key.split('.').next().unwrap_or("")
    }
}

const fn d(key: &'static str, kind: Kind, default: &'static str, live: bool, help: &'static str) -> Def {
    Def { key, kind, default, live, help }
}

pub const SCHEMA: &[Def] = &[
    d("provider.url", Kind::Text, "", false, "Where the models are served, for example http://localhost:9292/v1. Needs a restart: the connection is made once at start"),
    d("provider.name", Kind::Text, "", false, "Name of the service for the first line of the prompt. Leave empty to use what the server reports. Needs a restart: it is read once at start"),
    d("provider.key", Kind::Text, "", false, "Reference to the stored secret that holds the API key, like secret:NAME (the key itself is never written here). Needs a restart: the key is read once at start"),
    d("memory.enabled", Kind::Bool, "true", false, "Remember things between sessions. Needs a restart: the memory store is opened once at start"),
    d("memory.trust", Kind::Choice(&["auto", "local", "own", "any"]), "auto", false, "How far the model endpoint is trusted with memory. auto: local if the endpoint is on this machine, otherwise any cloud. Needs a restart: memory and learning are set up with it at start"),
    d("memory.recall_items", Kind::Int { min: 1, max: 50 }, "6", true, "Most notes added to one message from memory"),
    d("memory.check_model", Kind::Text, "", true, "A model on this machine that looks for notes that disagree while the app is idle, and suggests archiving the older one. Empty: off. Only runs when memory is kept local"),
    d("memory.recall_chars", Kind::Int { min: 200, max: 100_000 }, "3000", true, "Most characters of memory added to one message, all notes together"),
    d("learning.mode", Kind::Choice(&["propose", "silent", "off"]), "propose", false, "propose: suggestions wait for you. silent: applied at once and logged. off: no learning. Needs a restart: learning is set up once at start"),
    d("learning.threshold", Kind::Int { min: 2, max: 20 }, "3", false, "Sessions a suggestion must repeat in before it counts as a habit. Needs a restart: learning is set up once at start"),
    d("team.autonomy", Kind::Choice(&["plan-first", "check-ins", "fire-and-forget"]), "plan-first", true, "How much you are asked on a big goal"),
    d("team.parallel", Kind::Int { min: 1, max: 16 }, "1", true, "Workers running at once"),
    d("team.attempts", Kind::Int { min: 1, max: 10 }, "3", true, "Attempts per step before a failure goes up"),
    d("team.unavailable", Kind::Choice(&["stop", "fallback"]), "stop", true, "When a model is missing: stop and tell you, or try the type's fallback list"),
    d("media.image_model", Kind::Text, "", true, "Model for making images. Empty switches image making off. Applies to new chats"),
    d("web.listen", Kind::Text, "", false, "Address for the browser UI over HTTPS, for example 127.0.0.1:7843. Empty switches it off. Use this machine's network address to reach it from a phone; every device must be paired first. Needs a restart: the port is opened once at start"),
    d("web.tailscale", Kind::Bool, "false", false, "Also open the browser UI on this machine's Tailscale address (found with `tailscale ip -4`), so your phone can reach it over your tailnet. Uses the port of web.listen, or 7843. A phone signs in once with the link or QR code under Settings or `lh devices phone`. Never listens on every network. Needs a restart"),
    d("tools.sandbox", Kind::Bool, "true", true, "Run the model's commands in a sandbox: they can change files only in the chat's folder and a private temporary folder, and read the rest. Needs bubblewrap (bwrap). Applies to new chats"),
    d("tools.sandbox_network", Kind::Choice(&["ask", "allowlist", "on", "auto"]), "ask", true, "Network access for commands in the sandbox. ask: you are asked for each run. allowlist: only when every site the command names is on tools.sandbox_network_allow. on: always. auto: tools.sandbox_network_model decides each run, and you are asked when it cannot. Applies to new chats"),
    d("tools.sandbox_network_allow", Kind::Text, "", true, "Sites sandboxed commands may reach in allowlist mode, separated by commas, for example github.com, crates.io. A site covers its subdomains. Applies to new chats"),
    d("tools.sandbox_network_model", Kind::Text, "", true, "A small model that decides network access in auto mode. Empty: you are asked. Applies to new chats"),
    d("tools.codemode", Kind::Bool, "false", true, "Offer the model a tool that runs a short JavaScript script calling the other tools, so long results are filtered before the model reads them. Each call the script makes is checked like any other. Off: one tool less in every request. Applies to new chats"),
    d("web.use_proxy", Kind::Bool, "false", true, "Let web fetch go through the proxy named in the environment. Off by default, because a proxy looks names up itself and the check against private addresses cannot see where it goes. Applies to new chats"),
    d("web.searxng", Kind::Text, "", true, "Address of a SearXNG server for web search. Empty switches search off. Applies to new chats"),
    d("spending.mode", Kind::Choice(&["none", "caps", "estimate"]), "estimate", true, "none: only record. caps: refuse past a limit. estimate: ask before an expensive call"),
    d("spending.approve_over_usd", Kind::OptFloat, "0.25", true, "In estimate mode, ask when one call is expected to cost more than this (US dollars)"),
    d("spending.per_request_cap_usd", Kind::OptFloat, "", true, "Refuse any single call past this (US dollars). Empty for no limit"),
    d("spending.daily_cap_usd", Kind::OptFloat, "", true, "Refuse past this per day (US dollars). Empty for no limit"),
    d("spending.monthly_cap_usd", Kind::OptFloat, "", true, "Refuse past this per month (US dollars). Empty for no limit"),
    d("permissions.mode", Kind::Choice(&["tiered", "ask-all"]), "tiered", true, "tiered: reads are free, writes ask once, risky things always ask. ask-all: ask for everything"),
    d("permissions.bypass", Kind::Bool, "false", true, "Skip every permission question. You accept the risk; the project is not responsible for the outcome"),
    d("permissions.show_diff", Kind::Choice(&["always", "untrusted", "trusted", "never"]), "always", true, "When a file change shows what it changes. always: every time. untrusted: only in chats that still ask before writing. trusted: only in chats set to always allow writes. never: no diff"),
    d("permissions.sudo", Kind::Bool, "false", true, "Let agents use sudo (it still asks like any other command)"),
    d("tree.summarize", Kind::Choice(&["ask", "always", "never"]), "ask", true, "When you leave a branch of a chat for another, write a short summary of it where you come back. ask: you are asked each time"),
    d("kvcache.enabled", Kind::Bool, "true", true, "Save the model's work on a local chat, so a stored chat goes on quickly instead of being read again. Needs kvcache.dir; never for private chats or cloud models"),
    d("kvcache.dir", Kind::Text, "", true, "The folder the local server saves slots into (its --slot-save-path; with llama-swap, the folder that holds the per-model ones). Empty switches saving off"),
    d("kvcache.max_gb", Kind::Int { min: 1, max: 500 }, "5", true, "Most disk the saved caches may use in total; the oldest are removed first"),
    d("chats.title_model", Kind::Text, "", true, "A small fast model that names a chat after its first reply, in a few words. Empty names a chat from the first words of your first message. A cloud model never sees a chat that looks sensitive"),
    d("core.idle_after", Kind::Int { min: 0, max: 86_400 }, "600", true, "Seconds without activity before the core goes idle. 0 means never"),
    d("core.compact_at", Kind::AutoInt, "auto", true, "Shorten a conversation at about this many tokens. auto: three quarters of the model's context. 0: never"),
    d("reports.crash", Kind::Choice(&["off", "ask", "auto"]), "off", true, "Anonymous crash reports. off: never. ask: show exactly what would be sent and ask. auto: send"),
    d("updates.enabled", Kind::Bool, "true", true, "Check for signed updates"),
];

pub fn def(key: &str) -> Option<&'static Def> {
    SCHEMA.iter().find(|d| d.key == key)
}

/// Check a value for a key and return it in its stored form. Errors say what is allowed.
pub fn normalize(key: &str, raw: &str) -> Result<String> {
    let d = def(key).ok_or_else(|| Error::Invalid(format!("there is no setting called {key}. Run `lh settings` to see them")))?;
    let raw = raw.trim();
    let bad = |what: &str| Error::Invalid(format!("{key} {what}"));
    Ok(match d.kind {
        Kind::Bool => match raw.to_lowercase().as_str() {
            "true" | "on" | "yes" | "1" => "true".into(),
            "false" | "off" | "no" | "0" => "false".into(),
            _ => return Err(bad("must be on or off")),
        },
        Kind::Int { min, max } => {
            let n: i64 = raw.parse().map_err(|_| bad(&format!("must be a whole number from {min} to {max}")))?;
            if !(min..=max).contains(&n) {
                return Err(bad(&format!("must be a whole number from {min} to {max}")));
            }
            n.to_string()
        }
        Kind::OptFloat => {
            if raw.is_empty() || raw == "none" {
                String::new()
            } else {
                let f: f64 = raw.parse().map_err(|_| bad("must be a number of zero or more, or empty for no limit"))?;
                if !f.is_finite() || f < 0.0 {
                    return Err(bad("must be a number of zero or more, or empty for no limit"));
                }
                raw.to_string()
            }
        }
        Kind::AutoInt => {
            if raw.eq_ignore_ascii_case("auto") {
                "auto".into()
            } else {
                raw.parse::<u64>().map_err(|_| bad("must be auto or a whole number of tokens (0 means never)"))?.to_string()
            }
        }
        Kind::Choice(opts) => {
            let r = raw.to_lowercase();
            if !opts.contains(&r.as_str()) {
                return Err(bad(&format!("must be one of: {}", opts.join(", "))));
            }
            r
        }
        Kind::Text => raw.to_string(),
    })
}

/// Windows has no XDG folders and no `HOME`. Point those variables at the per-user folders once, at start,
/// so every path built from them lands in `%APPDATA%` (settings) and `%LOCALAPPDATA%` (data, state, token),
/// never in a relative folder. A variable already set is kept. Elsewhere this does nothing.
pub fn platform_dirs() {
    #[cfg(windows)]
    {
        let get = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty());
        let local = get("LOCALAPPDATA").or_else(|| get("USERPROFILE").map(|h| PathBuf::from(h).join("AppData").join("Local").into_os_string()));
        let roaming = get("APPDATA").or_else(|| local.clone());
        for (k, v) in [("HOME", get("USERPROFILE")), ("XDG_CONFIG_HOME", roaming), ("XDG_DATA_HOME", local.clone()), ("XDG_STATE_HOME", local.clone()), ("XDG_RUNTIME_DIR", local)] {
            if let (None, Some(v)) = (get(k), v) {
                std::env::set_var(k, v);
            }
        }
    }
}

/// The settings file. Unknown keys in it are kept as they are and ignored.
#[derive(Clone, Debug)]
pub struct ConfigFile {
    path: PathBuf,
    values: BTreeMap<String, String>,
}

impl ConfigFile {
    pub fn default_path() -> PathBuf {
        let base = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config"));
        base.join("lastharness").join("config.toml")
    }

    /// Read the file. A file that does not exist is an empty one (every setting at its default).
    pub fn load(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let mut values = BTreeMap::new();
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                let t: toml::Table = text.parse().map_err(|e: toml::de::Error| Error::Parse(path.display().to_string(), e.to_string()))?;
                for (group, v) in t {
                    if let toml::Value::Table(inner) = v {
                        for (k, v) in inner {
                            let s = match v {
                                toml::Value::String(s) => s,
                                toml::Value::Boolean(b) => b.to_string(),
                                toml::Value::Integer(i) => i.to_string(),
                                toml::Value::Float(f) => f.to_string(),
                                _ => continue,
                            };
                            values.insert(format!("{group}.{k}"), s);
                        }
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(Error::Io(path.display().to_string(), e.to_string())),
        }
        Ok(Self { path, values })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// The value in the file, or the default.
    pub fn get(&self, key: &str) -> String {
        self.values.get(key).cloned().or_else(|| def(key).map(|d| d.default.to_string())).unwrap_or_default()
    }

    pub fn is_set(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    pub fn get_bool(&self, key: &str) -> bool {
        self.get(key) == "true"
    }

    /// `None` when empty.
    pub fn get_opt(&self, key: &str) -> Option<String> {
        Some(self.get(key)).filter(|v| !v.is_empty())
    }

    pub fn get_num<T: std::str::FromStr>(&self, key: &str) -> Option<T> {
        self.get(key).parse().ok()
    }

    pub fn set(&mut self, key: &str, raw: &str) -> Result<String> {
        let v = normalize(key, raw)?;
        self.values.insert(key.to_string(), v.clone());
        Ok(v)
    }

    pub fn unset(&mut self, key: &str) {
        self.values.remove(key);
    }

    pub fn save(&self) -> Result<()> {
        let mut root = toml::Table::new();
        for (k, v) in &self.values {
            let Some((group, name)) = k.split_once('.') else { continue };
            // typed in the file so it reads naturally: true/false and numbers are not quoted
            let typed = match def(k).map(|d| d.kind) {
                Some(Kind::Bool) => toml::Value::Boolean(v == "true"),
                Some(Kind::Int { .. }) => v.parse().map(toml::Value::Integer).unwrap_or_else(|_| toml::Value::String(v.clone())),
                Some(Kind::OptFloat) => v.parse().map(toml::Value::Float).unwrap_or_else(|_| toml::Value::String(v.clone())),
                _ => toml::Value::String(v.clone()),
            };
            if let toml::Value::Table(t) = root.entry(group).or_insert_with(|| toml::Value::Table(toml::Table::new())) {
                t.insert(name.to_string(), typed);
            }
        }
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| Error::Io(dir.display().to_string(), e.to_string()))?;
        }
        let tmp = self.path.with_extension("toml.tmp");
        std::fs::write(&tmp, toml::to_string_pretty(&root).unwrap_or_default()).and_then(|_| std::fs::rename(&tmp, &self.path)).map_err(|e| Error::Io(self.path.display().to_string(), e.to_string()))
    }

    /// Every setting with its current value, in the order of the list.
    pub fn all(&self) -> Vec<(&'static Def, String, bool)> {
        SCHEMA.iter().map(|d| (d, self.get(d.key), self.is_set(d.key))).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_are_checked_and_stored_in_a_readable_file() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("lh/config.toml");
        let mut c = ConfigFile::load(&p).unwrap();
        assert!(!c.exists());
        assert_eq!((c.get("team.parallel"), c.get("memory.enabled"), c.get("learning.mode")), ("1".into(), "true".into(), "propose".into()));
        assert_eq!(c.set("memory.enabled", "off").unwrap(), "false");
        assert_eq!(c.set("team.autonomy", "Check-Ins").unwrap(), "check-ins");
        assert_eq!(c.set("spending.daily_cap_usd", "5.5").unwrap(), "5.5");
        assert_eq!(c.set("spending.daily_cap_usd", "none").unwrap(), "");
        c.set("provider.url", " http://localhost:9292/v1 ").unwrap();
        c.set("core.compact_at", "AUTO").unwrap();
        c.save().unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.contains("[memory]") && text.contains("enabled = false") && text.contains("url = \"http://localhost:9292/v1\""), "{text}");
        let again = ConfigFile::load(&p).unwrap();
        assert_eq!((again.get("memory.enabled"), again.get("team.autonomy"), again.get_opt("spending.daily_cap_usd")), ("false".into(), "check-ins".into(), None));
        assert!(again.is_set("provider.url") && !again.is_set("learning.mode"));
    }

    #[test]
    fn wrong_values_say_what_is_allowed() {
        for (k, v, expect) in [
            ("team.parallel", "99", "from 1 to 16"),
            ("team.parallel", "many", "whole number"),
            ("learning.mode", "loud", "one of: propose, silent, off"),
            ("memory.enabled", "maybe", "on or off"),
            ("spending.mode", "", "one of"),
            ("core.compact_at", "-5", "auto or a whole number"),
            ("spending.daily_cap_usd", "-1", "zero or more"),
            ("nothing.here", "x", "no setting called nothing.here"),
        ] {
            let e = normalize(k, v).unwrap_err().to_string();
            assert!(e.contains(expect), "{k}={v}: {e}");
        }
    }

    #[test]
    fn every_default_is_itself_valid_and_every_key_is_unique() {
        let mut seen = std::collections::HashSet::new();
        for d in SCHEMA {
            assert!(seen.insert(d.key), "{} twice", d.key);
            assert_eq!(normalize(d.key, d.default).unwrap(), d.default, "{}", d.key);
            assert!(!d.help.is_empty());
        }
    }

    #[test]
    fn a_broken_file_is_reported_not_replaced() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("config.toml");
        std::fs::write(&p, "this is = = not toml").unwrap();
        assert!(ConfigFile::load(&p).unwrap_err().to_string().contains("not valid TOML"));
    }
}
