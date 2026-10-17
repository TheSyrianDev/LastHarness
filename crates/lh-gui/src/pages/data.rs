//! Your data: export, import and keeping the app running in the background. Import shows what is in a folder
//! first and changes nothing until you say so.

use serde_json::{json, Value};

use super::s;

/// "Exported to /x. rules: 3, roles: 2" and the core's warning.
pub fn export_note(v: &Value) -> String {
    let counts = v["counts"].as_object().map(|m| m.iter().map(|(k, v)| format!("{k}: {v}")).collect::<Vec<_>>().join(", ")).unwrap_or_default();
    let mut out = format!("Saved to {}.", s(v, "dir"));
    if !counts.is_empty() {
        out.push_str(&format!(" {counts}."));
    }
    let w = s(v, "warning");
    if !w.is_empty() {
        out.push(' ');
        out.push_str(&w);
    }
    out
}

#[derive(Clone, Debug, PartialEq)]
pub struct Named {
    pub name: String,
    /// You already have one with this name.
    pub exists: bool,
}

/// What an import folder holds.
#[derive(Clone, Debug, PartialEq)]
pub struct Preview {
    pub rules_in_folder: u64,
    pub rules_new: u64,
    pub style: bool,
    pub memory: bool,
    pub roles: Vec<Named>,
    pub departments: Vec<Named>,
    pub problems: Vec<String>,
    /// What was done, when this was an apply.
    pub done: Option<Vec<(String, String)>>,
}

fn named(v: &Value) -> Vec<Named> {
    v.as_array().into_iter().flatten().map(|i| Named { name: s(i, "name"), exists: i["exists"] == true }).collect()
}

pub fn preview(v: &Value) -> Preview {
    Preview {
        rules_in_folder: v["rules"]["in_folder"].as_u64().unwrap_or(0),
        rules_new: v["rules"]["new"].as_u64().unwrap_or(0),
        style: v["style"] == true,
        memory: v["memory_files"] == true,
        roles: named(&v["roles"]),
        departments: named(&v["departments"]),
        problems: v["problems"].as_array().into_iter().flatten().filter_map(|p| p.as_str().map(String::from)).collect(),
        done: v["done"].as_object().map(|d| {
            d.iter().map(|(k, val)| (k.clone(), val.as_array().map(|a| a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join("; ")).unwrap_or_else(|| val.to_string()))).collect()
        }),
    }
}

fn list(items: &[Named]) -> String {
    if items.is_empty() {
        "none".into()
    } else {
        items.iter().map(|i| if i.exists { format!("{} (you have one)", i.name) } else { i.name.clone() }).collect::<Vec<_>>().join(", ")
    }
}

/// The summary lines shown before anything is brought in.
pub fn summary(p: &Preview) -> Vec<String> {
    let mut out = vec![
        format!("Rules: {} in the folder, {} of them new to you.", p.rules_in_folder, p.rules_new),
        format!("Writing style profile: {}.", if p.style { "yes" } else { "no" }),
        format!("Memory: {}.", if p.memory { "yes" } else { "no" }),
        format!("Roles: {}.", list(&p.roles)),
        format!("Departments: {}.", list(&p.departments)),
    ];
    out.extend(p.problems.iter().map(|x| format!("Cannot use {x}.")));
    out
}

/// Names that clash with what you already have: the ones you can choose to replace.
pub fn clashes(p: &Preview) -> Vec<String> {
    p.roles.iter().chain(p.departments.iter()).filter(|n| n.exists).map(|n| n.name.clone()).collect()
}

/// Is there anything in the folder to bring in?
pub fn has_content(p: &Preview) -> bool {
    p.rules_in_folder > 0 || p.style || p.memory || !p.roles.is_empty() || !p.departments.is_empty()
}

/// The request. `preview_only` asks what is in the folder and changes nothing.
pub fn import_body(dir: &str, apply: bool, replace: &[String], replace_style: bool) -> Value {
    json!({"dir": dir.trim(), "apply": apply, "overwrite": replace, "replace_style": replace_style})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Value {
        json!({"rules": {"in_folder": 4, "new": 3}, "style": true, "memory_files": false,
               "roles": [{"name": "writer", "exists": true}, {"name": "tester", "exists": false}],
               "departments": [], "problems": ["roles/bad.toml"]})
    }

    #[test]
    fn the_summary_says_what_is_in_the_folder() {
        let p = preview(&sample());
        let s = summary(&p);
        assert_eq!(s[0], "Rules: 4 in the folder, 3 of them new to you.");
        assert_eq!(s[1], "Writing style profile: yes.");
        assert_eq!(s[2], "Memory: no.");
        assert_eq!(s[3], "Roles: writer (you have one), tester.");
        assert_eq!(s[4], "Departments: none.");
        assert_eq!(s[5], "Cannot use roles/bad.toml.");
        assert!(p.done.is_none());
        assert!(has_content(&p));
    }

    #[test]
    fn only_names_you_already_have_can_be_replaced() {
        assert_eq!(clashes(&preview(&sample())), ["writer"]);
    }

    #[test]
    fn an_empty_folder_has_nothing_to_bring_in() {
        assert!(!has_content(&preview(&json!({"rules": {"in_folder": 0, "new": 0}}))));
    }

    #[test]
    fn what_was_done_is_read_after_apply() {
        let v = json!({"rules": {"in_folder": 1, "new": 1}, "done": {"rules": 1, "roles": ["writer", "tester"]}});
        let d = preview(&v).done.unwrap();
        assert!(d.contains(&("rules".to_string(), "1".to_string())));
        assert!(d.contains(&("roles".to_string(), "writer; tester".to_string())));
    }

    #[test]
    fn the_request_changes_nothing_until_apply() {
        let b = import_body(" /x ", false, &["writer".to_string()], true);
        assert_eq!(b, json!({"dir": "/x", "apply": false, "overwrite": ["writer"], "replace_style": true}));
        assert_eq!(import_body("/x", true, &[], false)["apply"], true);
    }

    #[test]
    fn the_export_note_names_the_folder_and_keeps_the_warning() {
        let n = export_note(&json!({"dir": "/out", "counts": {"rules": 3}, "warning": "Keep it safe."}));
        assert_eq!(n, "Saved to /out. rules: 3. Keep it safe.");
    }
}
