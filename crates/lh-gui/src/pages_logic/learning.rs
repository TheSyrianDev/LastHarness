//! Learning: rules, the style text, skills, suggestions and habits as the page shows them.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{json, Value};

use super::{arr, text};

#[derive(Clone, Debug, PartialEq)]
pub struct RuleRow {
    pub id: String,
    pub text: String,
    pub active: bool,
    pub sensitive: bool,
    /// "project /x", "role coder": where the rule applies, when it is not everywhere.
    pub scope: Vec<String>,
    pub source: String,
}

impl RuleRow {
    pub fn from_json(r: &Value) -> RuleRow {
        let mut scope = vec![];
        if let Some(p) = r["project"].as_str() {
            scope.push(format!("project {p}"));
        }
        if let Some(p) = r["role"].as_str() {
            scope.push(format!("role {p}"));
        }
        RuleRow { id: text(r, "id"), text: text(r, "text"), active: r["active"].as_bool().unwrap_or(true), sensitive: r["privacy"] == "sensitive", scope, source: text(r, "source") }
    }

    /// Where the rule came from, in words.
    pub fn source_words(&self) -> &'static str {
        match self.source.as_str() {
            "user" => "You wrote it",
            "proposal" => "You accepted a suggestion",
            "" => "",
            _ => "The assistant added it",
        }
    }
}

pub fn rules(v: &Value) -> Vec<RuleRow> {
    arr(v, "rules").iter().map(RuleRow::from_json).collect()
}

/// A suggestion waiting for an answer.
#[derive(Clone, Debug, PartialEq)]
pub struct ProposalRow {
    pub id: i64,
    pub kind: String,
    /// What it would do, in one sentence.
    pub headline: String,
    /// The words it would apply; shown in an editable box. Empty for kinds that cannot be edited.
    pub editable: String,
    pub because: String,
    pub sensitive: bool,
}

pub fn proposal(p: &Value) -> ProposalRow {
    let pl = &p["payload"];
    let s = |k: &str| pl[k].as_str().unwrap_or("").to_string();
    let kind = text(p, "kind");
    let (headline, editable) = match kind.as_str() {
        "rule" => ("New rule".to_string(), s("text")),
        "rule_update" => (format!("Change a rule that now says: {}", pl["was"].as_str().unwrap_or("?")), s("text")),
        "style" => (format!("Add to how I talk to you: {}", s("added")), s("text")),
        "skill_new" => (format!("New skill {}: {}", s("name"), s("description")), String::new()),
        "skill_update" => (format!("Change the skill {}: {}", s("name"), s("note")), String::new()),
        other => (other.to_string(), String::new()),
    };
    ProposalRow { id: p["id"].as_i64().unwrap_or(0), kind, headline, editable, because: p["evidence"].as_array().and_then(|e| e.first()).and_then(Value::as_str).unwrap_or("").to_string(), sensitive: pl["privacy"] == "sensitive" }
}

pub fn proposals(v: &Value) -> Vec<ProposalRow> {
    arr(v, "proposals").iter().map(proposal).collect()
}

/// The body for accepting: your own wording when it differs from the suggestion.
pub fn accept_body(row: &ProposalRow, edited: &str) -> Value {
    let e = edited.trim();
    if row.editable.is_empty() || e.is_empty() || e == row.editable.trim() { json!({}) } else { json!({"edit": {"text": e}}) }
}

/// A wish noticed in several chats.
#[derive(Clone, Debug, PartialEq)]
pub struct HabitRow {
    pub sessions: u64,
    pub example: String,
}

pub fn habits(v: &Value) -> Vec<HabitRow> {
    arr(v, "habits").iter().map(|h| HabitRow { sessions: h["sessions"].as_u64().unwrap_or(0), example: h["examples"].as_array().and_then(|e| e.first()).and_then(Value::as_str).unwrap_or("").to_string() }).collect()
}

#[derive(Clone, Debug, PartialEq)]
pub struct SkillRow {
    pub name: String,
    pub description: String,
    pub broken: Option<String>,
}

pub fn skills(v: &Value) -> Vec<SkillRow> {
    arr(v, "skills").iter().map(|s| SkillRow { name: text(s, "name"), description: text(s, "description"), broken: s["error"].as_str().map(str::to_string) }).collect()
}

/// What a skill folder holds, shown before anything is installed.
#[derive(Clone, Debug, PartialEq)]
pub struct SkillPreview {
    pub name: String,
    pub description: String,
    pub files: Vec<String>,
    pub bytes: usize,
    pub problem: Option<String>,
}

/// Read a skill folder into the files map the core takes. Hidden files are left out; binary files too.
pub fn read_skill_folder(dir: &Path) -> Result<BTreeMap<String, String>, String> {
    fn walk(root: &Path, d: &Path, out: &mut BTreeMap<String, String>) -> Result<(), String> {
        for e in std::fs::read_dir(d).map_err(|e| format!("Cannot read {}: {e}", d.display()))?.flatten() {
            let p = e.path();
            if e.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            if p.is_dir() {
                walk(root, &p, out)?;
            } else if let Ok(t) = std::fs::read_to_string(&p) {
                out.insert(p.strip_prefix(root).unwrap_or(&p).to_string_lossy().replace('\\', "/"), t);
            }
        }
        Ok(())
    }
    let mut m = BTreeMap::new();
    walk(dir, dir, &mut m)?;
    if !m.contains_key("SKILL.md") {
        return Err(format!("{} has no SKILL.md, so it is not a skill folder.", dir.display()));
    }
    Ok(m)
}

/// The name and description in SKILL.md's header, and the files; a problem in plain words when the header
/// cannot be read. The core checks again when the skill is installed.
pub fn preview(files: &BTreeMap<String, String>) -> SkillPreview {
    let md = files.get("SKILL.md").map(String::as_str).unwrap_or("");
    let header = md.replace("\r\n", "\n");
    let header = header.strip_prefix("---\n").and_then(|r| r.split("\n---").next()).unwrap_or("");
    let field = |k: &str| {
        header.lines().find_map(|l| l.strip_prefix(&format!("{k}:"))).map(|v| v.trim().trim_matches('"').trim_matches('\'').to_string()).unwrap_or_default()
    };
    let (name, description) = (field("name"), field("description"));
    let problem = if header.is_empty() {
        Some("SKILL.md must start with a header between --- lines.".to_string())
    } else if name.is_empty() {
        Some("The header of SKILL.md needs a name.".to_string())
    } else if description.is_empty() {
        Some("The header of SKILL.md needs a description.".to_string())
    } else {
        None
    };
    SkillPreview { name, description, files: files.keys().cloned().collect(), bytes: files.values().map(String::len).sum(), problem }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_read_with_their_scope_and_source() {
        let r = RuleRow::from_json(&json!({"id": "r1", "text": "Be brief.", "active": false, "privacy": "sensitive", "project": "/p", "role": "coder", "source": "proposal"}));
        assert_eq!((r.active, r.sensitive, r.scope.clone(), r.source_words()), (false, true, vec!["project /p".to_string(), "role coder".to_string()], "You accepted a suggestion"));
        let plain = RuleRow::from_json(&json!({"id": "r2", "text": "x", "source": "user"}));
        assert!(plain.active && plain.scope.is_empty());
        assert_eq!(plain.source_words(), "You wrote it");
    }

    #[test]
    fn each_kind_of_suggestion_has_a_sentence_and_the_editable_words() {
        let p = proposal(&json!({"id": 4, "kind": "rule_update", "payload": {"was": "Old.", "text": "New."}, "evidence": ["I said so"]}));
        assert_eq!((p.id, p.editable.as_str(), p.because.as_str()), (4, "New.", "I said so"));
        assert!(p.headline.contains("Old."));
        let s = proposal(&json!({"id": 5, "kind": "style", "payload": {"added": "terse", "text": "Terse. Plain."}}));
        assert_eq!((s.headline.as_str(), s.editable.as_str()), ("Add to how I talk to you: terse", "Terse. Plain."));
        let k = proposal(&json!({"id": 6, "kind": "skill_new", "payload": {"name": "notes", "description": "d", "privacy": "sensitive"}}));
        assert!(k.editable.is_empty() && k.sensitive);
    }

    #[test]
    fn accepting_sends_your_wording_only_when_it_changed() {
        let p = proposal(&json!({"id": 1, "kind": "rule", "payload": {"text": "Use links."}}));
        assert_eq!(accept_body(&p, "Use links. "), json!({}));
        assert_eq!(accept_body(&p, ""), json!({}));
        assert_eq!(accept_body(&p, "Always use links."), json!({"edit": {"text": "Always use links."}}));
        let k = proposal(&json!({"id": 2, "kind": "skill_new", "payload": {"name": "n"}}));
        assert_eq!(accept_body(&k, "anything"), json!({}), "a skill cannot be reworded here");
    }

    #[test]
    fn a_skill_folder_is_read_and_previewed_before_install() {
        let d = tempfile::tempdir().unwrap();
        assert!(read_skill_folder(d.path()).unwrap_err().contains("no SKILL.md"));
        std::fs::write(d.path().join("SKILL.md"), "---\nname: release-notes\ndescription: \"Write release notes.\"\n---\n\n1. Read the log.\n").unwrap();
        std::fs::create_dir(d.path().join("refs")).unwrap();
        std::fs::write(d.path().join("refs/style.md"), "short").unwrap();
        std::fs::write(d.path().join(".hidden"), "x").unwrap();
        std::fs::write(d.path().join("logo.bin"), [0xff, 0xfe, 0x00]).unwrap();
        let files = read_skill_folder(d.path()).unwrap();
        assert_eq!(files.keys().cloned().collect::<Vec<_>>(), ["SKILL.md", "refs/style.md"]);
        let p = preview(&files);
        assert_eq!((p.name.as_str(), p.description.as_str(), p.problem.clone()), ("release-notes", "Write release notes.", None));
        assert_eq!(p.files.len(), 2);
    }

    #[test]
    fn a_bad_skill_header_is_explained() {
        let mut f = BTreeMap::new();
        f.insert("SKILL.md".to_string(), "no header".to_string());
        assert!(preview(&f).problem.unwrap().contains("between ---"));
        f.insert("SKILL.md".to_string(), "---\ndescription: d\n---\n".to_string());
        assert!(preview(&f).problem.unwrap().contains("needs a name"));
        f.insert("SKILL.md".to_string(), "---\nname: n\n---\n".to_string());
        assert!(preview(&f).problem.unwrap().contains("needs a description"));
    }

    #[test]
    fn habits_and_skills_read_from_the_core() {
        let h = habits(&json!({"habits": [{"sessions": 3, "examples": ["add links", "x"]}]}));
        assert_eq!((h[0].sessions, h[0].example.as_str()), (3, "add links"));
        let s = skills(&json!({"skills": [{"name": "a", "description": "d", "error": null}, {"name": "b", "description": "", "error": "bad"}]}));
        assert_eq!((s[0].broken.clone(), s[1].broken.clone()), (None, Some("bad".to_string())));
    }
}
