//! Models and keys: the provider as the core reports it, the connection test, and what is sent when it is saved.

use serde_json::{json, Value};

use super::{arr, text};

/// How the provider is doing, for the dot beside its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// No address yet.
    NotSet,
    /// Answers with a model list.
    Working,
    /// Set, but not answering.
    Down,
}

impl Status {
    pub fn words(self) -> &'static str {
        match self {
            Status::NotSet => "Not set up",
            Status::Working => "Working",
            Status::Down => "Not answering",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProviderInfo {
    pub url: String,
    pub name: String,
    pub key_saved: bool,
    pub local: bool,
    pub reachable: bool,
    pub models: usize,
    /// The address came from how the core was started (`--provider`), not from the settings file.
    pub from_start: bool,
}

impl ProviderInfo {
    pub fn from_json(v: &Value) -> ProviderInfo {
        ProviderInfo {
            url: text(v, "url"),
            name: text(v, "name"),
            key_saved: v["key_saved"].as_bool().unwrap_or(false),
            local: v["local"].as_bool().unwrap_or(false),
            reachable: v["reachable"].as_bool().unwrap_or(false),
            models: v["models"].as_u64().unwrap_or(0) as usize,
            from_start: v["from_start"].as_bool().unwrap_or(false),
        }
    }

    pub fn status(&self) -> Status {
        if self.url.trim().is_empty() {
            Status::NotSet
        } else if self.reachable {
            Status::Working
        } else {
            Status::Down
        }
    }

    /// The name to show: the one given, else the address.
    pub fn title(&self) -> String {
        if self.name.trim().is_empty() { self.url.clone() } else { self.name.clone() }
    }

    /// The window opens on this page by itself when nothing is set up.
    pub fn needs_first_run(&self) -> bool {
        self.status() == Status::NotSet
    }
}

/// Does the address look like an Ollama server? Then the test also looks for Ollama's own API.
pub fn looks_like_ollama(url: &str) -> bool {
    let u = url.to_lowercase();
    u.contains("ollama") || u.contains(":11434")
}

/// What an address needs to look like, said in plain words. `None` when it is fine. The core checks again.
pub fn check_url(url: &str) -> Option<String> {
    let u = url.trim();
    if u.is_empty() {
        return Some("Type the address of the model server first.".into());
    }
    let rest = u.strip_prefix("http://").or_else(|| u.strip_prefix("https://"));
    if rest.is_none_or(str::is_empty) || u.contains(' ') {
        return Some("The address should start with http:// or https://, for example http://localhost:9292/v1".into());
    }
    None
}

/// What the connection test found.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TestResult {
    pub ok: bool,
    pub message: String,
    pub models: Vec<String>,
    pub ollama: Option<String>,
}

impl TestResult {
    pub fn from_json(v: &Value) -> TestResult {
        TestResult { ok: v["ok"].as_bool().unwrap_or(false), message: text(v, "message"), models: arr(v, "models").iter().filter_map(Value::as_str).map(str::to_string).collect(), ollama: v["ollama"].as_str().map(str::to_string) }
    }

    /// A few model names, then "and 12 more".
    pub fn model_sample(&self, max: usize) -> String {
        let shown: Vec<&str> = self.models.iter().take(max).map(String::as_str).collect();
        let rest = self.models.len().saturating_sub(max);
        if rest > 0 { format!("{}, and {rest} more", shown.join(", ")) } else { shown.join(", ") }
    }
}

/// The body for saving: the address and name always, the key only when one was typed (an empty box leaves the
/// saved key alone; removing it is its own button).
pub fn save_body(url: &str, name: &str, typed_key: &str) -> Value {
    let mut b = json!({"url": url.trim(), "name": name.trim()});
    if !typed_key.trim().is_empty() {
        b["key"] = json!(typed_key.trim());
    }
    b
}

/// The body for testing an address with the key typed so far (the saved key is used when nothing is typed).
pub fn test_body(url: &str, typed_key: &str) -> Value {
    let mut b = json!({"url": url.trim()});
    if !typed_key.trim().is_empty() {
        b["key"] = json!(typed_key.trim());
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dot_follows_the_address_and_the_answer() {
        let mut p = ProviderInfo::from_json(&json!({"url": "", "reachable": false}));
        assert_eq!((p.status(), p.needs_first_run()), (Status::NotSet, true));
        p.url = "http://localhost:9292/v1".into();
        assert_eq!((p.status(), p.needs_first_run()), (Status::Down, false));
        p.reachable = true;
        assert_eq!(p.status(), Status::Working);
        assert_eq!(p.title(), "http://localhost:9292/v1");
        p.name = "Home".into();
        assert_eq!(p.title(), "Home");
    }

    #[test]
    fn addresses_are_checked_in_plain_words() {
        assert!(check_url(" ").unwrap().contains("Type the address"));
        assert!(check_url("localhost:9292").unwrap().contains("http://"));
        assert!(check_url("https://").is_some() && check_url("http://a b").is_some());
        assert!(check_url(" http://localhost:9292/v1 ").is_none());
    }

    #[test]
    fn ollama_is_guessed_from_the_address() {
        assert!(looks_like_ollama("http://localhost:11434"));
        assert!(looks_like_ollama("http://box/Ollama/v1"));
        assert!(!looks_like_ollama("http://localhost:9292/v1"));
    }

    #[test]
    fn a_key_is_sent_only_when_typed() {
        assert_eq!(save_body(" http://x/v1 ", " Home ", "  "), json!({"url": "http://x/v1", "name": "Home"}));
        assert_eq!(save_body("http://x/v1", "", " sk-1 ")["key"], "sk-1");
        assert!(test_body("http://x", "").get("key").is_none());
        assert_eq!(test_body("http://x", "k")["key"], "k");
    }

    #[test]
    fn the_test_result_reads_the_core_answer() {
        let r = TestResult::from_json(&json!({"ok": true, "message": "Reached.", "models": ["a", "b", "c", "d"], "ollama": "0.5.1"}));
        assert!(r.ok && r.ollama.as_deref() == Some("0.5.1"));
        assert_eq!(r.model_sample(2), "a, b, and 2 more");
        assert_eq!(r.model_sample(10), "a, b, c, d");
        assert!(!TestResult::from_json(&json!({})).ok);
    }
}
