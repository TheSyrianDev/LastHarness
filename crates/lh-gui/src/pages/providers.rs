//! Providers: the registry of model services. List, add from a preset or an address, set the default, remove, and
//! approve or reject a provider the assistant proposed. A key is typed by the person only and is never shown again.

use serde_json::{json, Value};

use super::s;

#[derive(Clone, Debug, PartialEq)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub url: String,
    pub default: bool,
    pub key_saved: bool,
    pub local: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub url: String,
    pub key_required: bool,
    pub local: bool,
    pub supported: bool,
    pub note: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Proposal {
    pub id: String,
    pub name: String,
    pub url: String,
    pub preset: String,
    pub reason: String,
}

pub fn providers(v: &Value) -> Vec<Provider> {
    v["providers"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|p| Provider { id: s(p, "id"), name: s(p, "name"), url: s(p, "url"), default: p["default"] == true, key_saved: p["key_saved"] == true, local: p["local"] == true })
        .collect()
}

pub fn presets(v: &Value) -> Vec<Preset> {
    v["presets"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|p| Preset { id: s(p, "id"), name: s(p, "name"), url: s(p, "url"), key_required: p["key_required"] == true, local: p["local"] == true, supported: p["supported"] != false, note: s(p, "note") })
        .collect()
}

pub fn proposals(v: &Value) -> Vec<Proposal> {
    v["proposals"].as_array().into_iter().flatten().map(|p| Proposal { id: s(p, "id"), name: s(p, "name"), url: s(p, "url"), preset: s(p, "preset"), reason: s(p, "reason") }).collect()
}

/// One line under a provider's name.
pub fn state_words(p: &Provider) -> String {
    let key = if p.local && !p.key_saved { "no key needed" } else if p.key_saved { "key saved" } else { "no key saved" };
    format!("{}  ·  {}", p.url, key)
}

/// Does approving or adding this need a key? A known preset says; an unknown address might, so the box is offered.
pub fn key_required(preset: &str, presets: &[Preset]) -> bool {
    presets.iter().find(|p| p.id == preset).map(|p| p.key_required).unwrap_or(false)
}

/// The body for adding: a preset id, or an address. The key is left out when empty.
pub fn add_body(preset: Option<&str>, url: &str, name: &str, key: &str, presets: &[Preset]) -> Result<Value, String> {
    let (name, url, key) = (name.trim(), url.trim(), key.trim());
    let mut b = serde_json::Map::new();
    match preset {
        Some(id) => {
            let p = presets.iter().find(|p| p.id == id).ok_or("Choose a preset or type an address.")?;
            if !p.supported {
                return Err(format!("{} is not supported yet.", p.name));
            }
            if p.key_required && key.is_empty() {
                return Err(format!("{} needs a key.", p.name));
            }
            b.insert("preset".into(), json!(id));
        }
        None => {
            if !(url.starts_with("http://") || url.starts_with("https://")) {
                return Err("Type an address that starts with http:// or https://.".into());
            }
            b.insert("url".into(), json!(url));
        }
    }
    if !name.is_empty() {
        b.insert("name".into(), json!(name));
    }
    if !key.is_empty() {
        b.insert("key".into(), json!(key));
    }
    Ok(Value::Object(b))
}

/// The approve body: the key only when one was typed.
pub fn approve_body(key: &str) -> Value {
    let k = key.trim();
    if k.is_empty() {
        json!({})
    } else {
        json!({"key": k})
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ps() -> Vec<Preset> {
        presets(&json!({"presets": [
            {"id": "openai", "name": "OpenAI", "url": "https://api.openai.com/v1", "key_required": true, "supported": true},
            {"id": "ollama", "name": "Ollama", "url": "http://localhost:11434/v1", "local": true, "supported": true},
            {"id": "later", "name": "Later", "supported": false}
        ]}))
    }

    #[test]
    fn lists_are_read() {
        let l = providers(&json!({"providers": [{"id": "a", "name": "A", "url": "http://x", "default": true, "key_saved": true}, {"id": "b", "local": true}]}));
        assert!(l[0].default && l[0].key_saved && !l[1].default);
        assert_eq!(state_words(&l[0]), "http://x  ·  key saved");
        assert!(state_words(&l[1]).contains("no key needed"));
        let q = proposals(&json!({"proposals": [{"id": "p", "name": "N", "url": "u", "preset": "openai", "reason": "r"}]}));
        assert_eq!((q[0].id.as_str(), q[0].reason.as_str()), ("p", "r"));
        assert!(key_required("openai", &ps()) && !key_required("ollama", &ps()) && !key_required("zzz", &ps()));
    }

    #[test]
    fn adding_checks_before_sending() {
        let p = ps();
        assert_eq!(add_body(Some("openai"), "", "", " sk-1 ", &p).unwrap(), json!({"preset": "openai", "key": "sk-1"}));
        assert!(add_body(Some("openai"), "", "", "", &p).unwrap_err().contains("needs a key"));
        assert_eq!(add_body(Some("ollama"), "", "Mine", "", &p).unwrap(), json!({"preset": "ollama", "name": "Mine"}));
        assert!(add_body(Some("later"), "", "", "", &p).is_err());
        assert!(add_body(Some("nope"), "", "", "", &p).is_err());
        assert_eq!(add_body(None, " http://h:1/v1 ", "", "", &p).unwrap(), json!({"url": "http://h:1/v1"}));
        assert!(add_body(None, "ftp://x", "", "", &p).is_err());
        assert_eq!(approve_body("  "), json!({}));
        assert_eq!(approve_body("k"), json!({"key": "k"}));
    }
}
