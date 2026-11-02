//! Settings as the panel edits them. The core lists every setting with its kind; the panel offers the
//! kinds it can edit safely (on/off, a choice, a number in a range) and shows the rest.

use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Bool,
    Int { min: i64, max: i64 },
    Choice(Vec<String>),
    /// Free text: an address, a folder, a model name.
    Text,
    /// A decimal number; may be left empty.
    Number,
    /// A whole number or the word "auto".
    AutoInt,
    /// A kind this app does not know: shown as it is.
    Other,
}

/// The control a setting gets in its card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    Switch,
    Dropdown,
    Stepper,
    Field,
    Folder,
    ModelPicker,
    ReadOnly,
}

impl Control {
    /// Controls that need the full width of the card, so they sit on their own row under the text.
    pub fn wide(self) -> bool {
        matches!(self, Control::Field | Control::Folder | Control::ModelPicker | Control::ReadOnly)
    }
}

/// Settings that name a model, filled from the core's model list.
pub fn is_model_key(key: &str) -> bool {
    key.ends_with("_model")
}

/// Settings that name a folder.
pub fn is_folder_key(key: &str) -> bool {
    key.ends_with(".dir") || key.ends_with("_dir")
}

/// Which control a setting gets.
pub fn control_for(key: &str, kind: &Kind) -> Control {
    match kind {
        Kind::Bool => Control::Switch,
        Kind::Choice(_) => Control::Dropdown,
        Kind::Int { .. } => Control::Stepper,
        Kind::Text if is_model_key(key) => Control::ModelPicker,
        Kind::Text if is_folder_key(key) => Control::Folder,
        Kind::Text | Kind::Number | Kind::AutoInt => Control::Field,
        Kind::Other => Control::ReadOnly,
    }
}

/// Is the value the default? Compared as trimmed text.
pub fn is_default(value: &str, default: &str) -> bool {
    value.trim() == default.trim()
}

/// Check what was typed before it is sent; the core checks again and has the last word.
pub fn check_typed(kind: &Kind, raw: &str) -> Result<(), String> {
    let raw = raw.trim();
    match kind {
        Kind::Number if !raw.is_empty() && raw.parse::<f64>().is_err() => Err("Type a number, like 0.25, or leave it empty.".into()),
        Kind::AutoInt if raw != "auto" && raw.parse::<i64>().is_err() => Err("Type a whole number, or the word auto.".into()),
        Kind::Int { min, max } => match raw.parse::<i64>() {
            Ok(n) if (*min..=*max).contains(&n) => Ok(()),
            _ => Err(format!("Type a whole number from {min} to {max}.")),
        },
        _ => Ok(()),
    }
}

/// The core's refusal as a sentence in the card: the setting's key at the start is dropped, because the card
/// already says which setting it is.
pub fn friendly_error(key: &str, err: &str) -> String {
    let e = err.trim();
    let e = e.strip_prefix(key).unwrap_or(e).trim();
    let mut c = e.chars();
    match c.next() {
        Some(f) => format!("{}{}", f.to_uppercase().collect::<String>(), c.as_str().trim_end_matches('.')),
        None => "The core did not accept this value".into(),
    }
}

impl Kind {
    pub fn from_json(v: &Value) -> Kind {
        match v["type"].as_str().unwrap_or("") {
            "bool" => Kind::Bool,
            "int" => Kind::Int { min: v["min"].as_i64().unwrap_or(0), max: v["max"].as_i64().unwrap_or(i64::MAX) },
            "choice" => Kind::Choice(v["options"].as_array().map(|a| a.iter().filter_map(|o| o.as_str().map(String::from)).collect()).unwrap_or_default()),
            "text" => Kind::Text,
            "number" => Kind::Number,
            "auto-int" => Kind::AutoInt,
            _ => Kind::Other,
        }
    }

    pub fn editable(&self) -> bool {
        !matches!(self, Kind::Other)
    }

    /// The value one click on the main control gives: a switch flips, a choice moves to the next option.
    pub fn next(&self, current: &str) -> Option<String> {
        match self {
            Kind::Bool => Some(if current == "true" { "false" } else { "true" }.to_string()),
            Kind::Choice(o) if !o.is_empty() => {
                let i = o.iter().position(|x| x == current).map(|i| (i + 1) % o.len()).unwrap_or(0);
                Some(o[i].clone())
            }
            _ => None,
        }
    }

    /// One step up or down for a number, kept inside its range.
    pub fn step(&self, current: &str, up: bool) -> Option<String> {
        let Kind::Int { min, max } = self else { return None };
        let n: i64 = current.trim().parse().ok()?;
        let m = if up { n.saturating_add(1) } else { n.saturating_sub(1) }.clamp(*min, *max);
        (m != n).then(|| m.to_string())
    }
}

/// Settings grouped by their first word, in the order the core lists them.
pub fn groups<T, F: Fn(&T) -> &str>(items: &[T], group: F) -> Vec<(String, Vec<usize>)> {
    let mut out: Vec<(String, Vec<usize>)> = vec![];
    for (i, it) in items.iter().enumerate() {
        let g = group(it);
        match out.iter_mut().find(|(n, _)| n == g) {
            Some((_, v)) => v.push(i),
            None => out.push((g.to_string(), vec![i])),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn kinds_are_read_from_what_the_core_sends() {
        assert_eq!(Kind::from_json(&json!({"type": "bool"})), Kind::Bool);
        assert_eq!(Kind::from_json(&json!({"type": "int", "min": 1, "max": 9})), Kind::Int { min: 1, max: 9 });
        assert_eq!(Kind::from_json(&json!({"type": "choice", "options": ["a", "b"]})), Kind::Choice(vec!["a".into(), "b".into()]));
        assert_eq!(Kind::from_json(&json!({"type": "text"})), Kind::Text);
        assert_eq!(Kind::from_json(&json!({"type": "auto-int"})), Kind::AutoInt);
        assert_eq!(Kind::from_json(&json!({"type": "number", "optional": true})), Kind::Number);
        assert_eq!(Kind::from_json(&json!({"type": "mystery"})), Kind::Other);
        assert_eq!(Kind::from_json(&Value::Null), Kind::Other);
        assert!(Kind::Bool.editable() && !Kind::Other.editable());
    }

    #[test]
    fn a_switch_flips_and_a_choice_goes_round() {
        assert_eq!(Kind::Bool.next("true").unwrap(), "false");
        assert_eq!(Kind::Bool.next("false").unwrap(), "true");
        assert_eq!(Kind::Bool.next("anything").unwrap(), "true");
        let c = Kind::Choice(vec!["off".into(), "ask".into(), "auto".into()]);
        assert_eq!(c.next("off").unwrap(), "ask");
        assert_eq!(c.next("auto").unwrap(), "off");
        assert_eq!(c.next("unknown").unwrap(), "off");
        assert_eq!(Kind::Other.next("x"), None);
        assert_eq!(Kind::Choice(vec![]).next("x"), None);
    }

    #[test]
    fn numbers_step_inside_their_range() {
        let k = Kind::Int { min: 0, max: 5 };
        assert_eq!(k.step("2", true).unwrap(), "3");
        assert_eq!(k.step("2", false).unwrap(), "1");
        assert_eq!(k.step("5", true), None, "already at the top");
        assert_eq!(k.step("0", false), None);
        assert_eq!(k.step("nope", true), None);
        assert_eq!(Kind::Bool.step("1", true), None);
    }

    #[test]
    fn every_setting_gets_a_control() {
        assert_eq!(control_for("memory.enabled", &Kind::Bool), Control::Switch);
        assert_eq!(control_for("tree.summarize", &Kind::Choice(vec![])), Control::Dropdown);
        assert_eq!(control_for("kvcache.max_gb", &Kind::Int { min: 1, max: 5 }), Control::Stepper);
        assert_eq!(control_for("kvcache.dir", &Kind::Text), Control::Folder);
        assert_eq!(control_for("chats.title_model", &Kind::Text), Control::ModelPicker);
        assert_eq!(control_for("media.image_model", &Kind::Text), Control::ModelPicker);
        assert_eq!(control_for("provider.url", &Kind::Text), Control::Field);
        assert_eq!(control_for("spending.daily_cap_usd", &Kind::Number), Control::Field);
        assert_eq!(control_for("core.compact_at", &Kind::AutoInt), Control::Field);
        assert_eq!(control_for("x", &Kind::Other), Control::ReadOnly);
        assert!(Control::Folder.wide() && !Control::Switch.wide() && !Control::Dropdown.wide());
    }

    #[test]
    fn default_is_found_by_value() {
        assert!(is_default("", ""));
        assert!(is_default(" 600 ", "600"));
        assert!(!is_default("false", "true"));
        assert!(!is_default("http://x", ""));
    }

    #[test]
    fn typed_values_are_checked_before_sending() {
        assert!(check_typed(&Kind::Number, "").is_ok());
        assert!(check_typed(&Kind::Number, "0.25").is_ok());
        assert!(check_typed(&Kind::Number, "abc").is_err());
        assert!(check_typed(&Kind::AutoInt, "auto").is_ok());
        assert!(check_typed(&Kind::AutoInt, "4000").is_ok());
        assert!(check_typed(&Kind::AutoInt, "lots").is_err());
        let k = Kind::Int { min: 1, max: 5 };
        assert!(check_typed(&k, "5").is_ok());
        assert!(check_typed(&k, "6").unwrap_err().contains("1 to 5"));
        assert!(check_typed(&Kind::Text, "anything").is_ok());
    }

    #[test]
    fn the_cores_refusal_reads_in_the_card() {
        assert_eq!(friendly_error("core.idle_after", "core.idle_after must be a whole number from 0 to 86400"), "Must be a whole number from 0 to 86400");
        assert_eq!(friendly_error("a.b", ""), "The core did not accept this value");
    }

    #[test]
    fn groups_keep_the_cores_order() {
        let v = ["memory", "provider", "memory", "learn"];
        let g = groups(&v, |s| s);
        assert_eq!(g, vec![("memory".to_string(), vec![0, 2]), ("provider".to_string(), vec![1]), ("learn".to_string(), vec![3])]);
    }
}
