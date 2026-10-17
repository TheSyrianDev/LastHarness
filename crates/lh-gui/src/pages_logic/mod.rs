//! What the Models and keys, Learning and Departments pages of Settings decide, apart from drawing: how the
//! core's answers read for a person, and how the forms turn into what the core takes.

pub mod learning;
pub mod models_keys;
pub mod team;

use serde_json::Value;

/// The items of a list in a core answer, or none.
pub fn arr<'a>(v: &'a Value, k: &str) -> &'a [Value] {
    v[k].as_array().map(Vec::as_slice).unwrap_or(&[])
}

/// A text field of a JSON object, empty when missing.
pub fn text(v: &Value, k: &str) -> String {
    v[k].as_str().unwrap_or("").to_string()
}

/// One version in a history list, as a person reads it.
#[derive(Clone, Debug, PartialEq)]
pub struct Version {
    pub number: i64,
    pub line: String,
}

/// "edited by you: the words". The core's "user" reads as "you".
pub fn version_line(op: &str, by: &str, what: &str) -> String {
    let op = match op {
        "create" => "created",
        "edit" => "edited",
        "rollback" => "rolled back",
        "disable" => "switched off",
        "enable" => "switched on",
        other => other,
    };
    let by = match by {
        "user" => "you",
        "assistant" => "the assistant",
        other => other,
    };
    let what = what.trim();
    if what.is_empty() { format!("{op} by {by}") } else { format!("{op} by {by}: {what}") }
}

/// A version list from the core: `[{version, op, by, ...}]` with the words each version held, newest first.
pub fn versions(list: &[Value], words: impl Fn(&Value) -> String) -> Vec<Version> {
    let mut out: Vec<Version> = list.iter().map(|h| Version { number: h["version"].as_i64().unwrap_or(0), line: version_line(h["op"].as_str().unwrap_or(""), h["by"].as_str().unwrap_or(""), &words(h)) }).collect();
    out.sort_by_key(|v| std::cmp::Reverse(v.number));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_version_reads_in_plain_words() {
        assert_eq!(version_line("edit", "user", " Be brief. "), "edited by you: Be brief.");
        assert_eq!(version_line("create", "assistant", ""), "created by the assistant");
        assert_eq!(version_line("odd", "x", ""), "odd by x");
    }

    #[test]
    fn versions_come_newest_first() {
        let l = [json!({"version": 1, "op": "create", "by": "user", "note": "a"}), json!({"version": 2, "op": "edit", "by": "user", "note": "b"})];
        let v = versions(&l, |h| text(h, "note"));
        assert_eq!((v[0].number, v[0].line.as_str(), v[1].number), (2, "edited by you: b", 1));
    }
}
