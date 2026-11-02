//! Activity: the append-only record of what the assistant did, newest first, and what was spent.

use serde_json::Value;

use super::{s, stamp};

/// The kinds that can be picked in the filter: the stored word and the words shown.
pub const KINDS: &[(&str, &str)] = &[
    ("", "Everything"),
    ("tool_call", "Tools used"),
    ("permission", "Permission answers"),
    ("request_sent", "Sent to a model service"),
    ("setting_changed", "Settings changed"),
    ("undo", "Undone"),
    ("mcp_server", "MCP servers"),
    ("folder_trust", "Folder trust"),
    ("device", "Devices"),
    ("channel", "Channels"),
];

pub fn kind_label(kind: &str) -> String {
    KINDS.iter().find(|(k, _)| *k == kind).map(|(_, l)| (*l).to_string()).unwrap_or_else(|| {
        let w = kind.replace('_', " ");
        let mut c = w.chars();
        c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub seq: i64,
    pub when: String,
    pub kind: String,
    pub actor: String,
    pub summary: String,
    pub undoable: bool,
}

fn short(text: &str, max: usize) -> String {
    let one: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if one.chars().count() <= max {
        one
    } else {
        format!("{}…", one.chars().take(max).collect::<String>().trim_end())
    }
}

/// One line about an entry, in words. Nothing secret is here: the core already took keys out of the log.
pub fn summary(kind: &str, d: &Value) -> String {
    let pick = |keys: &[&str]| keys.iter().find_map(|k| d[*k].as_str().filter(|x| !x.is_empty()).map(String::from));
    let line = match kind {
        "tool_call" => match (pick(&["tool"]), d["incognito"] == true) {
            (Some(t), true) => format!("Used {t} (private chat, details not kept)"),
            (Some(t), false) => {
                let r = d["result"].as_str().map(|r| format!(": {}", short(r, 80))).unwrap_or_default();
                format!("Used {t}{r}")
            }
            _ => "Used a tool".into(),
        },
        "permission" => format!("{} {}", pick(&["tool"]).unwrap_or_else(|| "A tool".into()), pick(&["outcome"]).unwrap_or_default().replace('_', " ")),
        "request_sent" => {
            let cost = d["cost_micro_usd"].as_i64().map(|c| format!(", cost {}", dollars(c))).unwrap_or_default();
            format!("Sent a request to {} ({}){cost}", pick(&["provider"]).unwrap_or_default(), pick(&["model"]).unwrap_or_default())
        }
        "setting_changed" => match pick(&["key", "setting"]) {
            Some(k) => {
                // the settings screen's own name for it; team settings are logged without their "team." part
                let full = if crate::setting_names::info(&k).is_none() && crate::setting_names::info(&format!("team.{k}")).is_some() { format!("team.{k}") } else { k.clone() };
                let name = crate::setting_names::info(&full).map_or(k.clone(), |i| format!("\"{}\"", i.title));
                match pick(&["value"]).filter(|v| !v.is_empty()) {
                    Some(v) => format!("Changed {name} to {}", short(&crate::setting_names::value_text(&full, &v), 60)),
                    None => format!("Changed {name}"),
                }
            }
            None => "Changed a setting".into(),
        },
        "undo" => format!("Undid the change to {}", pick(&["path"]).unwrap_or_else(|| "a file".into())),
        "mcp_server" => format!("MCP server: {}", pick(&["op"]).unwrap_or_default()),
        "folder_trust" => format!("Folder decision: {}", pick(&["decision"]).unwrap_or_default()),
        "device" => format!("Device: {}", pick(&["op"]).unwrap_or_default()),
        "startup" => "The core started".into(),
        "schedule_ran" => format!("Scheduled task \"{}\" ran: {}", pick(&["name"]).unwrap_or_default(), pick(&["result"]).unwrap_or_default()),
        "schedule_added" => format!("Added the scheduled task \"{}\"", pick(&["name"]).unwrap_or_default()),
        "schedule_removed" => "Removed a scheduled task".into(),
        "schedule_changed_files" => {
            let files: Vec<&str> = d["files"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
            format!("Scheduled task \"{}\" changed {}: {}", pick(&["name"]).unwrap_or_default(), if files.len() == 1 { "1 file".to_string() } else { format!("{} files", files.len()) }, short(&files.join(", "), 160))
        }
        "hooks_problem" => format!("hooks.toml could not be read: {}", pick(&["error"]).unwrap_or_default()),
        // something learned or changed in the team or memory: "Created the role planner", not the raw record
        _ if d["op"].is_string() && d["item"].is_string() => {
            let op = pick(&["op"]).unwrap_or_default();
            let done = match op.as_str() {
                "create" => "Created".to_string(),
                "update" => "Changed".to_string(),
                "delete" => "Removed".to_string(),
                "revert" => "Reverted".to_string(),
                o => kind_label(o),
            };
            let what = pick(&["kind"]).map(|k| format!("the {} ", k.replace('_', " "))).unwrap_or_default();
            format!("{done} {what}\"{}\"", pick(&["item"]).unwrap_or_default())
        }
        _ => match d {
            Value::Null => String::new(),
            Value::Object(m) if m.is_empty() => String::new(),
            other => other.to_string(),
        },
    };
    short(line.trim(), 160)
}

fn dollars(micro: i64) -> String {
    let d = micro as f64 / 1_000_000.0;
    if d.abs() < 0.01 && micro != 0 {
        format!("${d:.4}")
    } else {
        format!("${d:.2}")
    }
}

pub fn entries(v: &Value) -> Vec<Entry> {
    v["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|e| Entry {
            seq: e["seq"].as_i64().unwrap_or(0),
            when: stamp(e["at"].as_i64().unwrap_or(0)),
            kind: s(e, "kind"),
            actor: s(e, "actor"),
            summary: summary(&s(e, "kind"), &e["detail"]),
            undoable: e["undoable"] == true,
        })
        .collect()
}

/// The path to read the log: newest first, optionally one kind.
pub fn path(kind: &str, limit: u32) -> String {
    if kind.is_empty() {
        format!("/v1/activity?limit={limit}")
    } else {
        format!("/v1/activity?limit={limit}&kind={kind}")
    }
}

/// The answer to an undo, as a sentence.
pub fn undo_note(v: &Value) -> String {
    match v["result"].as_str() {
        Some("refused") => format!("Not undone. {}", s(v, "message")),
        _ => s(v, "message"),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Spend {
    pub today: String,
    pub today_calls: u64,
    pub month: String,
    pub month_calls: u64,
    pub unpriced: u64,
    /// This month: provider, amount, calls.
    pub providers: Vec<(String, String, u64)>,
}

pub fn spend(v: &Value) -> Spend {
    Spend {
        today: s(&v["today"], "text"),
        today_calls: v["today"]["calls"].as_u64().unwrap_or(0),
        month: s(&v["this_month"], "text"),
        month_calls: v["this_month"]["calls"].as_u64().unwrap_or(0),
        unpriced: v["unpriced_calls"].as_u64().unwrap_or(0),
        providers: v["providers"].as_array().into_iter().flatten().map(|p| (s(p, "provider"), s(p, "text"), p["calls"].as_u64().unwrap_or(0))).collect(),
    }
}

pub fn calls_words(n: u64) -> String {
    if n == 1 { "1 call".into() } else { format!("{n} calls") }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn entries_read_newest_first_as_the_core_sends_them() {
        let v = json!({"entries": [
            {"seq": 9, "at": 1_791_590_400, "kind": "tool_call", "actor": "assistant", "detail": {"tool": "write_file", "result": "wrote\n12 bytes"}, "undoable": true},
            {"seq": 8, "at": 0, "kind": "permission", "actor": "user", "detail": {"tool": "write_file", "outcome": "allowed_by_user"}}
        ]});
        let e = entries(&v);
        assert_eq!(e[0].seq, 9);
        assert_eq!(e[0].when, "2026-10-10 00:00");
        assert_eq!(e[0].summary, "Used write_file: wrote 12 bytes");
        assert!(e[0].undoable && !e[1].undoable);
        assert_eq!(e[1].summary, "write_file allowed by user");
    }

    #[test]
    fn each_kind_reads_in_words() {
        assert_eq!(summary("request_sent", &json!({"provider": "openai", "model": "gpt-4o", "cost_micro_usd": 12_500})), "Sent a request to openai (gpt-4o), cost $0.01");
        assert_eq!(summary("setting_changed", &json!({"key": "web.listen"})), "Changed \"Address for the browser and phone\"");
        assert_eq!(summary("setting_changed", &json!({"key": "parallel", "value": "2"})), "Changed \"Workers at the same time\" to 2 workers");
        assert_eq!(summary("setting_changed", &json!({"key": "unavailable", "value": "stop"})), "Changed \"When a model is missing\" to Stop and tell me");
        assert_eq!(summary("startup", &json!({"provider": "x"})), "The core started");
        assert_eq!(summary("schedule_changed_files", &json!({"id": 1, "name": "tidy", "files": ["a.txt", "b.txt"]})), "Scheduled task \"tidy\" changed 2 files: a.txt, b.txt");
        assert_eq!(summary("schedule_ran", &json!({"name": "news", "result": "ok"})), "Scheduled task \"news\" ran: ok");
        assert_eq!(summary("setting_changed", &json!({"key": "made.up"})), "Changed made.up");
        assert_eq!(summary("tool_call", &json!({"tool": "bash", "incognito": true})), "Used bash (private chat, details not kept)");
        assert_eq!(summary("undo", &json!({"path": "/a/b.txt"})), "Undid the change to /a/b.txt");
        assert_eq!(summary("mystery", &Value::Null), "");
        assert_eq!(summary("learned", &json!({"kind": "role", "item": "planner", "op": "create", "version": 1})), "Created the role \"planner\"");
        assert_eq!(summary("mystery", &json!({"a": 1})), "{\"a\":1}");
    }

    #[test]
    fn a_long_line_is_cut() {
        let long = "x".repeat(400);
        let s = summary("mystery", &json!({"t": long}));
        assert!(s.chars().count() <= 161 && s.ends_with('…'));
    }

    #[test]
    fn the_filter_asks_the_core_for_one_kind() {
        assert_eq!(path("", 100), "/v1/activity?limit=100");
        assert_eq!(path("tool_call", 50), "/v1/activity?limit=50&kind=tool_call");
        assert_eq!(kind_label("tool_call"), "Tools used");
        assert_eq!(kind_label("chat_forked"), "Chat forked");
    }

    #[test]
    fn undo_answers_read_as_sentences() {
        assert_eq!(undo_note(&json!({"result": "restored", "message": "The file is back to how it was."})), "The file is back to how it was.");
        assert_eq!(undo_note(&json!({"result": "refused", "message": "this was already undone"})), "Not undone. this was already undone");
    }

    #[test]
    fn spending_is_read_for_today_and_the_month() {
        let v = json!({"today": {"text": "$0.10", "calls": 1}, "this_month": {"text": "$2.00", "calls": 1}, "unpriced_calls": 3,
                       "providers": [{"provider": "openai", "text": "$2.00", "calls": 1}]});
        let s = spend(&v);
        assert_eq!((s.today.as_str(), s.month.as_str(), s.unpriced), ("$0.10", "$2.00", 3));
        assert_eq!(s.providers, vec![("openai".to_string(), "$2.00".to_string(), 1)]);
        assert_eq!(calls_words(1), "1 call");
        assert_eq!(calls_words(2), "2 calls");
    }
}
