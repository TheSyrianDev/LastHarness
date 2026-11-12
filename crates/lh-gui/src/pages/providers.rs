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
    /// Found running on this computer by the core, not saved. Read-only in the app.
    pub detected: bool,
    /// "openai", "anthropic", or empty when the core did not say.
    pub api: String,
    pub models: Vec<String>,
    /// Off in the settings: its models are not routed.
    pub enabled: bool,
    /// Models the person turned off. Any other listed model is on.
    pub disabled_models: Vec<String>,
    pub default_model: Option<String>,
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
        .map(|p| Provider { id: s(p, "id"), name: s(p, "name"), url: s(p, "url"), default: p["default"] == true, key_saved: p["key_saved"] == true, local: p["local"] == true, detected: p["detected"] == true, api: s(p, "api"), models: strings(&p["models"]), enabled: p["enabled"] != false, disabled_models: strings(&p["disabled_models"]), default_model: p["default_model"].as_str().map(str::to_string) })
        .collect()
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array().into_iter().flatten().filter_map(|m| m.as_str().map(str::to_string)).collect()
}

impl Provider {
    /// A listed model is on unless the person turned it off.
    pub fn model_on(&self, model: &str) -> bool {
        !self.disabled_models.iter().any(|m| m == model)
    }
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

/// The light beside a provider: ready, needs a look (missing key or failed test), or not tested yet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Light {
    Ready,
    Attention,
    Off,
}

impl Light {
    pub fn glyph(self) -> &'static str {
        match self {
            Light::Ready => "●",
            Light::Attention => "◐",
            Light::Off => "○",
        }
    }
}

/// The light and its words. `tested` is the last test result this session: `None` when no test ran.
pub fn light(p: &Provider, tested: Option<bool>) -> (Light, &'static str) {
    if !p.enabled {
        (Light::Off, "Off")
    } else if tested == Some(false) {
        (Light::Attention, "Last test failed")
    } else if !p.local && !p.key_saved {
        (Light::Attention, "Key missing")
    } else if tested == Some(true) {
        (Light::Ready, "Ready")
    } else {
        (Light::Off, "Not tested yet")
    }
}

/// The body that turns one model on or off.
pub fn toggle_body(model: &str, on: bool) -> Value {
    json!({"model": model, "enabled": on})
}

/// The body that sets the default model; `None` clears it.
pub fn default_model_body(model: Option<&str>) -> Value {
    json!({"model": model})
}

/// The API style as a person reads it.
pub fn api_label(api: &str) -> &'static str {
    match api {
        "anthropic" => "Anthropic",
        _ => "OpenAI style",
    }
}

/// The key as shown: never the key itself.
pub fn key_words(p: &Provider) -> &'static str {
    if p.key_saved {
        "••••••••  saved"
    } else if p.local {
        "Not needed"
    } else {
        "None saved"
    }
}

/// One line under a provider's name.
pub fn state_words(p: &Provider) -> String {
    let key = if p.local && !p.key_saved { "no key needed" } else if p.key_saved { "key saved" } else { "no key saved" };
    format!("{}  ·  {}", p.url, key)
}

/// The model count under a provider's name, or a hint to refresh.
pub fn models_words(p: &Provider) -> String {
    match p.models.len() {
        0 => "No models listed yet. Use Refresh models.".to_string(),
        1 => "1 model listed.".to_string(),
        n => format!("{n} models listed."),
    }
}

/// The body for editing: only the fields that were typed or picked. An address is checked before it is sent.
/// `api` is "openai", "anthropic", or empty to keep the saved style.
pub fn edit_body(name: &str, url: &str, key: &str, api: &str) -> Result<Value, String> {
    let (name, url, key, api) = (name.trim(), url.trim(), key.trim(), api.trim());
    let mut b = serde_json::Map::new();
    if !url.is_empty() {
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return Err("Type an address that starts with http:// or https://.".into());
        }
        b.insert("url".into(), json!(url));
    }
    if !name.is_empty() {
        b.insert("name".into(), json!(name));
    }
    if !key.is_empty() {
        b.insert("key".into(), json!(key));
    }
    if !api.is_empty() {
        if api != "openai" && api != "anthropic" {
            return Err("Pick OpenAI style or Anthropic.".into());
        }
        b.insert("api".into(), json!(api));
    }
    Ok(Value::Object(b))
}

/// The body for testing an address before saving: the address, the typed key and the style when given.
/// The key and the style are left out when empty, so the core uses the saved ones.
pub fn test_body(url: &str, key: &str, api: Option<&str>) -> Value {
    let mut b = serde_json::Map::new();
    b.insert("url".into(), json!(url.trim()));
    let key = key.trim();
    if !key.is_empty() {
        b.insert("key".into(), json!(key));
    }
    if let Some(a) = api.map(str::trim).filter(|a| !a.is_empty()) {
        b.insert("api".into(), json!(a));
    }
    Value::Object(b)
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

/// What the add flow's test says: "Found N models." on success, else the core's message.
pub fn found_words(ok: bool, models: &[String], message: &str) -> String {
    if !ok {
        return message.to_string();
    }
    match models.len() {
        0 => "Reached the server, but it lists no models yet.".to_string(),
        1 => "Found 1 model.".to_string(),
        n => format!("Found {n} models."),
    }
}

/// The models the default-model picker may offer: enabled providers that passed a test, with the model on.
/// Each item is (provider id, provider name, model).
pub fn pickable(list: &[Provider], tested: &std::collections::HashMap<String, bool>) -> Vec<(String, String, String)> {
    list.iter()
        .filter(|p| p.enabled && tested.get(&p.id) == Some(&true))
        .flat_map(|p| {
            let name = if p.name.is_empty() { p.id.clone() } else { p.name.clone() };
            p.models.iter().filter(|m| p.model_on(m)).map(move |m| (p.id.clone(), name.clone(), m.clone())).collect::<Vec<_>>()
        })
        .collect()
}

/// The default model as a person reads it: "model (Provider)". `None` when no default model is set.
pub fn default_words(list: &[Provider]) -> Option<String> {
    let p = list.iter().find(|p| p.default && p.default_model.is_some()).or_else(|| list.iter().find(|p| p.default_model.is_some()))?;
    let name = if p.name.is_empty() { p.id.clone() } else { p.name.clone() };
    Some(format!("{} ({name})", p.default_model.as_deref().unwrap_or("")))
}

/// The key box as sent: a `$NAME` (or a bare environment name, which this box accepts) is read from the
/// environment now, at test or save time. Any other text is the key itself. An error names the variable only,
/// never its value, and the value is not kept anywhere in the app.
pub fn resolve_key(typed: &str, env: impl Fn(&str) -> Option<String>) -> Result<String, String> {
    let t = typed.trim();
    let is_env_name = |n: &str| n.len() >= 2 && n.starts_with(|c: char| c.is_ascii_uppercase() || c == '_') && n.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
    let name = match t.strip_prefix('$') {
        Some(rest) if is_env_name(rest) => rest,
        Some(_) => return Err("Type the variable name after the $, for example $OPENAI_API_KEY.".into()),
        None if is_env_name(t) => t,
        None => return Ok(t.to_string()),
    };
    match env(name) {
        Some(v) if !v.trim().is_empty() => Ok(v.trim().to_string()),
        _ => Err(format!("{name} is not set in the environment. Set it, or type the key itself.")),
    }
}

/// The model count under a provider's name, short: "3 models" or "1 model".
pub fn short_models(p: &Provider) -> String {
    match p.models.len() {
        1 => "1 model".to_string(),
        n => format!("{n} models"),
    }
}

/// The approve body for a typed key box: the key is resolved first, the same way as on add and edit,
/// so `$NAME` or a bare environment name is read from the environment and the raw text is never echoed.
pub fn approve_key(typed: &str, env: impl Fn(&str) -> Option<String>) -> Result<Value, String> {
    let key = resolve_key(typed, env)?;
    Ok(approve_body(&key))
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
        assert_eq!(models_words(&l[0]), "No models listed yet. Use Refresh models.");
        let m = providers(&json!({"providers": [{"id": "c", "models": ["a", "b"]}]}));
        assert_eq!(models_words(&m[0]), "2 models listed.");
        assert!(state_words(&l[1]).contains("no key needed"));
        let q = proposals(&json!({"proposals": [{"id": "p", "name": "N", "url": "u", "preset": "openai", "reason": "r"}]}));
        assert_eq!((q[0].id.as_str(), q[0].reason.as_str()), ("p", "r"));
        assert!(key_required("openai", &ps()) && !key_required("ollama", &ps()) && !key_required("zzz", &ps()));
    }

    #[test]
    fn light_and_words() {
        let l = providers(&json!({"providers": [
            {"id": "a", "key_saved": true},
            {"id": "b"},
            {"id": "c", "local": true},
            {"id": "d", "key_saved": true, "api": "anthropic"}
        ]}));
        assert_eq!(light(&l[0], None), (Light::Off, "Not tested yet"));
        assert_eq!(light(&l[0], Some(true)), (Light::Ready, "Ready"));
        assert_eq!(light(&l[0], Some(false)), (Light::Attention, "Last test failed"));
        assert_eq!(light(&l[1], Some(true)), (Light::Attention, "Key missing"));
        assert_eq!(light(&l[2], None).0, Light::Off);
        assert_eq!(Light::Ready.glyph(), "●");
        assert_eq!(key_words(&l[0]), "••••••••  saved");
        assert_eq!(key_words(&l[1]), "None saved");
        assert_eq!(key_words(&l[2]), "Not needed");
        assert_eq!(l[3].api, "anthropic");
        assert_eq!(api_label(&l[3].api), "Anthropic");
        assert_eq!(api_label(""), "OpenAI style");
    }

    fn prov(enabled: bool, disabled: &[&str], default_model: Option<&str>) -> Provider {
        Provider { id: "x".into(), name: "X".into(), url: "u".into(), default: false, key_saved: true, local: false, detected: false, api: "openai".into(), models: vec!["a".into(), "b".into()], enabled, disabled_models: disabled.iter().map(|s| s.to_string()).collect(), default_model: default_model.map(str::to_string) }
    }

    #[test]
    fn enabled_flag_and_model_state_read() {
        let p = providers(&json!({"providers": [
            {"id": "old", "name": "Old", "url": "u", "models": ["a"]},
            {"id": "n", "name": "N", "url": "u", "models": ["a", "b"], "enabled": false, "disabled_models": ["b"], "default_model": "a"}
        ]}));
        assert!(p[0].enabled, "missing flag loads as on");
        assert!(p[0].model_on("a") && p[0].default_model.is_none());
        assert!(!p[1].enabled && !p[1].model_on("b") && p[1].model_on("a"));
        assert_eq!(p[1].default_model.as_deref(), Some("a"));
        assert_eq!(light(&p[1], Some(true)), (Light::Off, "Off"));
        assert_eq!(light(&prov(true, &[], None), Some(true)).0, Light::Ready);
    }

    #[test]
    fn key_box_resolves_env_names_and_never_echoes() {
        let env = |n: &str| match n {
            "OPENAI_API_KEY" => Some("sk-secret-123".to_string()),
            "EMPTY_ONE" => Some("  ".to_string()),
            _ => None,
        };
        // a literal key passes through unchanged
        assert_eq!(resolve_key(" sk-typed-1 ", env).unwrap(), "sk-typed-1");
        assert_eq!(resolve_key("", env).unwrap(), "");
        // $NAME and a bare env name both read the environment
        assert_eq!(resolve_key("$OPENAI_API_KEY", env).unwrap(), "sk-secret-123");
        assert_eq!(resolve_key(" OPENAI_API_KEY ", env).unwrap(), "sk-secret-123");
        // unset or blank variables are an error that names the variable, not a value
        let e = resolve_key("$MISSING_KEY", env).unwrap_err();
        assert!(e.contains("MISSING_KEY") && !e.contains("sk-"));
        assert!(resolve_key("EMPTY_ONE", env).is_err());
        assert!(resolve_key("$lower", env).is_err());
        assert_eq!(resolve_key("sk-x", |_| panic!("not an env name")).unwrap(), "sk-x");
    }

    #[test]
    fn short_model_count() {
        let l = providers(&json!({"providers": [{"id": "a", "models": ["x"]}, {"id": "b", "models": ["x", "y", "z"]}, {"id": "c"}]}));
        assert_eq!((short_models(&l[0]).as_str(), short_models(&l[1]).as_str(), short_models(&l[2]).as_str()), ("1 model", "3 models", "0 models"));
    }

    #[test]
    fn model_toggle_and_default_bodies() {
        assert_eq!(toggle_body("b", false), json!({"model": "b", "enabled": false}));
        assert_eq!(default_model_body(Some("a")), json!({"model": "a"}));
        assert_eq!(default_model_body(None), json!({"model": null}));
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
        assert_eq!(edit_body(" New ", "", "", "").unwrap(), json!({"name": "New"}));
        assert_eq!(edit_body("", "https://h/v1", " k ", "").unwrap(), json!({"url": "https://h/v1", "key": "k"}));
        assert_eq!(edit_body("", "", "", "").unwrap(), json!({}));
        assert_eq!(edit_body("", "", "", " anthropic ").unwrap(), json!({"api": "anthropic"}));
        assert_eq!(edit_body("N", "", "", "openai").unwrap(), json!({"name": "N", "api": "openai"}));
        assert!(edit_body("", "", "", "gemini").is_err());
        assert!(edit_body("", "ftp://x", "", "").is_err());
        assert_eq!(test_body(" http://h:1/v1 ", " k ", Some("anthropic")), json!({"url": "http://h:1/v1", "key": "k", "api": "anthropic"}));
        assert_eq!(test_body("http://h:1/v1", "  ", None), json!({"url": "http://h:1/v1"}));
        assert_eq!(test_body("http://h:1/v1", "", Some(" ")), json!({"url": "http://h:1/v1"}));
        assert_eq!(approve_body("  "), json!({}));
        assert_eq!(approve_body("k"), json!({"key": "k"}));
    }

    #[test]
    fn approve_resolves_env_names_and_never_echoes_a_missing_one() {
        let env = |n: &str| (n == "OPENAI_API_KEY").then(|| " sk-env-9 ".to_string());
        assert_eq!(approve_key("$OPENAI_API_KEY", env).unwrap(), json!({"key": "sk-env-9"}));
        assert_eq!(approve_key(" OPENAI_API_KEY ", env).unwrap(), json!({"key": "sk-env-9"}));
        assert_eq!(approve_key("sk-typed", env).unwrap(), json!({"key": "sk-typed"}));
        assert_eq!(approve_key("  ", env).unwrap(), json!({}));
        let e = approve_key("$MISSING_KEY", env).unwrap_err();
        assert!(e.contains("MISSING_KEY") && !e.contains("sk-"));
    }

    #[test]
    fn add_flow_words_and_picker() {
        assert_eq!(found_words(true, &["a".into(), "b".into()], "x"), "Found 2 models.");
        assert_eq!(found_words(true, &["a".into()], "x"), "Found 1 model.");
        assert_eq!(found_words(false, &[], "Could not reach it."), "Could not reach it.");
        let l = providers(&json!({"providers": [
            {"id": "loc", "name": "Local", "enabled": true, "models": ["m1", "m2"], "disabled_models": ["m2"], "default": true, "default_model": "m1"},
            {"id": "any", "name": "Any", "enabled": true, "models": ["x"]},
            {"id": "off", "name": "Off", "enabled": false, "models": ["y"]}
        ]}));
        let tested = std::collections::HashMap::from([("loc".to_string(), true), ("off".to_string(), true)]);
        assert_eq!(pickable(&l, &tested), vec![("loc".to_string(), "Local".to_string(), "m1".to_string())]);
        assert_eq!(default_words(&l).as_deref(), Some("m1 (Local)"));
        assert_eq!(default_words(&providers(&json!({"providers": [{"id": "a"}]}))), None);
    }
}
