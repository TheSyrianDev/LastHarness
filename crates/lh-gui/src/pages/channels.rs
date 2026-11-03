//! Channels: Telegram, Matrix, Signal and Slack. Every channel starts off and is switched on only after its
//! warning is read.

use serde_json::{json, Value};

use super::s;

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub name: String,
    pub help: String,
    pub required: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Kind {
    pub kind: String,
    pub fields: Vec<Field>,
    pub warning: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Channel {
    pub name: String,
    pub kind: String,
    pub enabled: bool,
    pub running: bool,
    pub mode: String,
    pub model: String,
    pub allow: Vec<String>,
    pub settings: Vec<(String, String)>,
    pub handled: u64,
    pub ignored: u64,
    pub last_error: Option<String>,
}

pub fn kinds(v: &Value) -> Vec<Kind> {
    let fields = |k: &Value, key: &str, required: bool| -> Vec<Field> {
        k[key].as_array().into_iter().flatten().map(|f| Field { name: s(f, "name"), help: s(f, "help"), required }).collect()
    };
    v["kinds"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|k| Kind { kind: s(k, "kind"), fields: [fields(k, "required", true), fields(k, "optional", false)].concat(), warning: s(k, "warning") })
        .collect()
}

pub fn channels(v: &Value) -> Vec<Channel> {
    v["channels"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|c| Channel {
            name: s(c, "name"),
            kind: s(c, "kind"),
            enabled: c["enabled"] == true,
            running: c["status"]["running"] == true,
            mode: s(c, "mode"),
            model: s(c, "model"),
            allow: c["allow"].as_array().into_iter().flatten().filter_map(|a| a.as_str().map(String::from)).collect(),
            settings: c["settings"].as_object().map(|m| m.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect()).unwrap_or_default(),
            handled: c["status"]["handled"].as_u64().unwrap_or(0),
            ignored: c["status"]["ignored"].as_u64().unwrap_or(0),
            last_error: c["status"]["last_error"].as_str().filter(|e| !e.is_empty()).map(String::from),
        })
        .collect()
}

/// One line saying where a channel stands.
pub fn state_words(c: &Channel) -> String {
    if !c.enabled {
        "Off".into()
    } else if c.running {
        format!("On and running. Answered {}, ignored {}.", c.handled, c.ignored)
    } else {
        format!("On, but not running{}", c.last_error.as_deref().map(|e| format!(": {e}")).unwrap_or_default())
    }
}

/// A list typed as `a, b,c` as separate ids.
pub fn parse_allow(text: &str) -> Vec<String> {
    text.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()
}

/// Secrets are never typed here: a field whose name has "token" must point to where the secret lives.
pub fn check_secret(field: &str, value: &str) -> Result<(), String> {
    if field.contains("token") && !value.is_empty() && !(value.starts_with("env:") || value.starts_with("secret:")) {
        return Err(format!("{field} is written as env:NAME or secret:NAME, naming where the key is kept. Never the key itself."));
    }
    Ok(())
}

/// What the form holds, as the core wants it. `values` is each setting field's typed text.
pub fn form_body(name: &str, kind: &Kind, allow: &str, model: &str, assistant: bool, values: &[(String, String)]) -> Result<Value, String> {
    let name = name.trim();
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err("Give the channel a name with letters, digits, - or _.".into());
    }
    let mut settings = serde_json::Map::new();
    for f in &kind.fields {
        let v = values.iter().find(|(k, _)| *k == f.name).map(|(_, v)| v.trim().to_string()).unwrap_or_default();
        if f.required && v.is_empty() {
            return Err(format!("{} needs a value.", f.name));
        }
        check_secret(&f.name, &v)?;
        if !v.is_empty() {
            settings.insert(f.name.clone(), json!(v));
        }
    }
    Ok(json!({"name": name, "kind": kind.kind, "allow": parse_allow(allow), "model": model.trim(), "mode": if assistant { "assistant" } else { "chat" }, "settings": settings}))
}

/// Switching on needs a list of people, a model and the warning read. This says what is still missing.
pub fn missing_for_enable(c: &Channel) -> Option<&'static str> {
    if c.allow.is_empty() {
        Some("List at least one person who may talk to it.")
    } else if c.model.trim().is_empty() {
        Some("Choose the model that answers.")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tg() -> Kind {
        Kind {
            kind: "telegram".into(),
            fields: vec![Field { name: "token".into(), help: "".into(), required: true }, Field { name: "api_base".into(), help: "".into(), required: false }],
            warning: "w".into(),
        }
    }

    #[test]
    fn kinds_and_channels_are_read() {
        let v = json!({
            "kinds": [{"kind": "telegram", "required": [{"name": "token", "help": "h"}], "optional": [{"name": "api_base", "help": "o"}], "warning": "careful"}],
            "channels": [
                {"name": "t", "kind": "telegram", "enabled": true, "mode": "chat", "model": "m", "allow": ["1", "2"], "settings": {"token": "env:T"}, "status": {"running": true, "handled": 3, "ignored": 1}},
                {"name": "u", "kind": "slack", "enabled": true, "status": {"running": false, "last_error": "bad token"}},
                {"name": "v", "kind": "signal", "enabled": false}
            ]
        });
        let k = kinds(&v);
        assert_eq!(k[0].fields.len(), 2);
        assert!(k[0].fields[0].required && !k[0].fields[1].required);
        assert_eq!(k[0].warning, "careful");
        let c = channels(&v);
        assert_eq!(c[0].allow, ["1", "2"]);
        assert_eq!(c[0].settings, vec![("token".to_string(), "env:T".to_string())]);
        assert_eq!(state_words(&c[0]), "On and running. Answered 3, ignored 1.");
        assert_eq!(state_words(&c[1]), "On, but not running: bad token");
        assert_eq!(state_words(&c[2]), "Off");
    }

    #[test]
    fn the_form_builds_the_body_and_starts_with_what_the_core_needs() {
        let b = form_body(" home ", &tg(), "12, 34 ,", " llama ", true, &[("token".into(), "env:TG".into()), ("api_base".into(), "".into())]).unwrap();
        assert_eq!(b, json!({"name": "home", "kind": "telegram", "allow": ["12", "34"], "model": "llama", "mode": "assistant", "settings": {"token": "env:TG"}}));
        assert_eq!(form_body("a", &tg(), "", "", false, &[("token".into(), "secret:x".into())]).unwrap()["mode"], "chat");
    }

    #[test]
    fn a_key_typed_in_full_is_refused() {
        let e = form_body("a", &tg(), "1", "m", false, &[("token".into(), "123456:ABCDEF".into())]).unwrap_err();
        assert!(e.contains("env:NAME"));
        assert!(form_body("a", &tg(), "1", "m", false, &[]).unwrap_err().contains("token needs a value"));
        assert!(form_body("bad name", &tg(), "1", "m", false, &[]).is_err());
        assert!(check_secret("account", "+4912345").is_ok());
    }

    #[test]
    fn enabling_asks_for_people_and_a_model_first() {
        let mut c = channels(&json!({"channels": [{"name": "t", "kind": "telegram", "enabled": false, "allow": [], "model": ""}]})).remove(0);
        assert!(missing_for_enable(&c).unwrap().contains("person"));
        c.allow = vec!["1".into()];
        assert!(missing_for_enable(&c).unwrap().contains("model"));
        c.model = "m".into();
        assert_eq!(missing_for_enable(&c), None);
    }
}
