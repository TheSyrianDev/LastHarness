//! Settings as the panel edits them. The core lists every setting with its kind; the panel offers the
//! kinds it can edit safely (on/off, a choice, a number in a range) and shows the rest.

use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Bool,
    Int { min: i64, max: i64 },
    Choice(Vec<String>),
    /// Free text, a decimal number or "auto": shown, changed with `lh settings`.
    Other,
}

impl Kind {
    pub fn from_json(v: &Value) -> Kind {
        match v["type"].as_str().unwrap_or("") {
            "bool" => Kind::Bool,
            "int" => Kind::Int { min: v["min"].as_i64().unwrap_or(0), max: v["max"].as_i64().unwrap_or(i64::MAX) },
            "choice" => Kind::Choice(v["options"].as_array().map(|a| a.iter().filter_map(|o| o.as_str().map(String::from)).collect()).unwrap_or_default()),
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
        assert_eq!(Kind::from_json(&json!({"type": "text"})), Kind::Other);
        assert_eq!(Kind::from_json(&json!({"type": "auto-int"})), Kind::Other);
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
    fn groups_keep_the_cores_order() {
        let v = ["memory", "provider", "memory", "learn"];
        let g = groups(&v, |s| s);
        assert_eq!(g, vec![("memory".to_string(), vec![0, 2]), ("provider".to_string(), vec![1]), ("learn".to_string(), vec![3])]);
    }
}
