//! Departments: roles and departments as forms, model types and the question a deleted type asks, team
//! settings and away mode.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use super::{arr, text};

/// A role's instructions go with every step it does, so the core keeps them short.
pub const INSTRUCTIONS_MAX: usize = 600;

/// The tools a role may list (the core's own list; a test keeps the two the same).
pub const KNOWN_TOOLS: [&str; 13] = ["read_file", "write_file", "list_dir", "run_command", "web_fetch", "web_search", "generate_image", "recall", "open_note", "remember", "forget", "use_skill", "read_skill_file"];

/// Names are lower case letters, digits and single hyphens.
pub fn valid_name(n: &str) -> bool {
    !n.is_empty() && n.len() <= 40 && !n.starts_with('-') && !n.ends_with('-') && !n.contains("--") && n.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"))
}

/// A role as the form edits it. Numbers stay text until the form is checked.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RoleForm {
    pub name: String,
    pub description: String,
    /// `type` (a kind of model) or `model` (one exact model).
    pub kind: String,
    pub target: String,
    pub tools: Vec<String>,
    pub memory: String,
    pub temperature: String,
    pub top_p: String,
    pub top_k: String,
    pub min_p: String,
    pub repeat_penalty: String,
    pub max_steps: String,
    /// "", "on" or "off".
    pub thinking: String,
    pub instructions: String,
}

impl RoleForm {
    /// A new role with safe starting values.
    pub fn blank() -> RoleForm {
        RoleForm { kind: "type".into(), target: "mid-tier".into(), memory: "none".into(), max_steps: "12".into(), ..Default::default() }
    }

    pub fn from_json(r: &Value) -> RoleForm {
        let num = |v: &Value| if v.is_null() { String::new() } else { v.to_string() };
        let sm = &r["sampling"];
        RoleForm {
            name: text(r, "name"),
            description: text(r, "description"),
            kind: text(&r["target"], "kind"),
            target: text(&r["target"], "value"),
            tools: arr(r, "tools").iter().filter_map(Value::as_str).map(str::to_string).collect(),
            memory: text(r, "memory"),
            temperature: num(&sm["temperature"]),
            top_p: num(&sm["top_p"]),
            top_k: num(&sm["top_k"]),
            min_p: num(&sm["min_p"]),
            repeat_penalty: num(&sm["repeat_penalty"]),
            max_steps: r["max_steps"].as_u64().map(|n| n.to_string()).unwrap_or_default(),
            thinking: match r["thinking"].as_bool() {
                Some(true) => "on".into(),
                Some(false) => "off".into(),
                None => String::new(),
            },
            instructions: text(r, "instructions"),
        }
    }

    /// The role file text the core takes, or the first thing wrong, in plain words.
    pub fn compose(&self) -> Result<String, String> {
        if !valid_name(&self.name) {
            return Err("The name can use lower case letters, digits and single hyphens, like web-designer.".into());
        }
        if self.description.trim().is_empty() {
            return Err("Add a one-line description. The lead reads it when it decides who gets a step.".into());
        }
        if self.target.trim().is_empty() {
            return Err(if self.kind == "model" { "Say which model this role uses." } else { "Pick a model type for this role." }.into());
        }
        let ins = self.instructions.trim();
        if ins.is_empty() {
            return Err("Write the instructions for this role.".into());
        }
        if ins.chars().count() > INSTRUCTIONS_MAX {
            return Err(format!("The instructions are {} characters; the most is {INSTRUCTIONS_MAX}. They go with every step the role does, so keep them to a short role line.", ins.chars().count()));
        }
        let float = |label: &str, v: &str| -> Result<Option<String>, String> {
            let v = v.trim();
            if v.is_empty() {
                return Ok(None);
            }
            v.parse::<f32>().map(|n| Some(n.to_string())).map_err(|_| format!("{label} must be a number, like 0.7, or empty."))
        };
        let steps = self.max_steps.trim().parse::<u32>().ok().filter(|n| (1..=100).contains(n)).ok_or("Steps per task must be a whole number from 1 to 100.")?;
        let top_k = match self.top_k.trim() {
            "" => None,
            k => Some(k.parse::<u32>().map_err(|_| "Top K must be a whole number, or empty.".to_string())?.to_string()),
        };
        let mut h = format!("---\nname: {}\ndescription: {}\n", self.name, quote(self.description.trim()));
        if self.kind == "model" {
            h.push_str(&format!("model: {}\n", quote(self.target.trim())));
        } else {
            h.push_str(&format!("type: {}\n", self.target.trim().to_lowercase()));
        }
        if !self.tools.is_empty() {
            h.push_str(&format!("tools: {}\n", self.tools.join(" ")));
        }
        h.push_str(&format!("memory: {}\n", if self.memory.trim().is_empty() { "none" } else { self.memory.trim() }));
        for (k, v) in [("temperature", float("Temperature", &self.temperature)?), ("top_p", float("Top P", &self.top_p)?), ("top_k", top_k), ("min_p", float("Min P", &self.min_p)?), ("repeat_penalty", float("Repeat penalty", &self.repeat_penalty)?)] {
            if let Some(v) = v {
                h.push_str(&format!("{k}: {v}\n"));
            }
        }
        h.push_str(&format!("max_steps: {steps}\n"));
        match self.thinking.as_str() {
            "on" | "off" => h.push_str(&format!("thinking: {}\n", self.thinking)),
            _ => {}
        }
        Ok(format!("{h}---\n\n{ins}\n"))
    }
}

/// A department as the form edits it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DeptForm {
    pub name: String,
    pub description: String,
    pub lead: String,
    /// Role names; a department inside is written `dept:name`.
    pub members: Vec<String>,
    pub parallel: String,
    pub attempts: String,
    pub instructions: String,
}

impl DeptForm {
    pub fn from_json(d: &Value) -> DeptForm {
        DeptForm {
            name: text(d, "name"),
            description: text(d, "description"),
            lead: text(d, "lead"),
            members: arr(d, "members").iter().filter_map(Value::as_str).map(str::to_string).collect(),
            parallel: d["parallel"].as_u64().map(|n| n.to_string()).unwrap_or_default(),
            attempts: d["attempts"].as_u64().map(|n| n.to_string()).unwrap_or_default(),
            instructions: text(d, "instructions"),
        }
    }

    pub fn compose(&self) -> Result<String, String> {
        if !valid_name(&self.name) {
            return Err("The name can use lower case letters, digits and single hyphens, like web.".into());
        }
        if self.lead.trim().is_empty() {
            return Err("Pick a lead. The lead plans the work and checks it.".into());
        }
        if self.members.is_empty() {
            return Err("Pick at least one member.".into());
        }
        let opt = |label: &str, v: &str, max: u32| -> Result<Option<u32>, String> {
            match v.trim() {
                "" => Ok(None),
                t => t.parse::<u32>().ok().filter(|n| (1..=max).contains(n)).map(Some).ok_or(format!("{label} must be a whole number from 1 to {max}, or empty.")),
            }
        };
        let mut h = format!("---\nname: {}\n", self.name);
        if !self.description.trim().is_empty() {
            h.push_str(&format!("description: {}\n", quote(self.description.trim())));
        }
        h.push_str(&format!("lead: {}\nmembers: {}\n", self.lead, self.members.join(" ")));
        if let Some(p) = opt("Workers at once", &self.parallel, 16)? {
            h.push_str(&format!("parallel: {p}\n"));
        }
        if let Some(a) = opt("Attempts per step", &self.attempts, 10)? {
            h.push_str(&format!("attempts: {a}\n"));
        }
        Ok(format!("{h}---\n\n{}\n", self.instructions.trim()))
    }
}

/// A model type and the model it points to.
#[derive(Clone, Debug, PartialEq)]
pub struct TypeRow {
    pub name: String,
    pub model: Option<String>,
    pub fallbacks: Vec<String>,
    pub stock: bool,
}

pub fn types(v: &Value) -> Vec<TypeRow> {
    arr(v, "types").iter().map(|t| TypeRow { name: text(t, "name"), model: t["model"].as_str().map(str::to_string), fallbacks: arr(t, "fallbacks").iter().filter_map(Value::as_str).map(str::to_string).collect(), stock: t["stock"].as_bool().unwrap_or(false) }).collect()
}

/// Where a role points, for the reassign question.
#[derive(Clone, Debug, PartialEq)]
pub struct RoleTarget {
    pub name: String,
    pub kind: String,
    pub value: String,
}

pub fn role_targets(v: &Value) -> Vec<RoleTarget> {
    arr(v, "roles").iter().filter(|r| r["error"].is_null()).map(|r| RoleTarget { name: text(r, "name"), kind: text(&r["target"], "kind"), value: text(&r["target"], "value") }).collect()
}

/// The roles that use a type and so need somewhere else to go before it is deleted.
pub fn roles_using_type(roles: &[RoleTarget], ty: &str) -> Vec<String> {
    roles.iter().filter(|r| r.kind == "type" && r.value == ty).map(|r| r.name.clone()).collect()
}

/// The new home chosen for a role: another type, or one exact model.
#[derive(Clone, Debug, PartialEq)]
pub enum Home {
    Type(String),
    Model(String),
}

/// A starting choice for each affected role: the first other type, so "Delete" works with one click.
pub fn default_homes(affected: &[String], types: &[TypeRow], deleting: &str) -> BTreeMap<String, Home> {
    let other = types.iter().find(|t| t.name != deleting).map(|t| Home::Type(t.name.clone()));
    affected.iter().filter_map(|r| other.clone().map(|h| (r.clone(), h))).collect()
}

/// The body of the delete call, or what is missing.
pub fn reassign_body(affected: &[String], homes: &BTreeMap<String, Home>) -> Result<Value, String> {
    let mut m = serde_json::Map::new();
    for r in affected {
        match homes.get(r) {
            Some(Home::Type(t)) if !t.trim().is_empty() => m.insert(r.clone(), json!({"type": t})),
            Some(Home::Model(x)) if !x.trim().is_empty() => m.insert(r.clone(), json!({"model": x.trim()})),
            _ => return Err(format!("Choose where {r} goes first.")),
        };
    }
    Ok(json!({"reassign": m}))
}

/// Team settings as the core reports them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TeamSettingsView {
    pub parallel: u64,
    pub attempts: u64,
    pub autonomy: String,
    pub unavailable: String,
    pub away: bool,
    pub parallel_max: u64,
    pub attempts_max: u64,
}

impl TeamSettingsView {
    pub fn from_json(v: &Value) -> TeamSettingsView {
        TeamSettingsView {
            parallel: v["parallel"].as_u64().unwrap_or(1),
            attempts: v["attempts"].as_u64().unwrap_or(3),
            autonomy: text(v, "autonomy"),
            unavailable: text(v, "unavailable"),
            away: !v["away_since"].is_null(),
            parallel_max: v["limits"]["parallel_max"].as_u64().unwrap_or(16),
            attempts_max: v["limits"]["attempts_max"].as_u64().unwrap_or(10),
        }
    }
}

pub const AUTONOMY: [(&str, &str); 3] = [("plan-first", "Ask me to approve the plan first"), ("check-ins", "Also pause at milestones"), ("fire-and-forget", "Do not ask me anything")];
pub const UNAVAILABLE: [(&str, &str); 2] = [("stop", "Stop and tell me"), ("fallback", "Use another model")];

/// The words for a stored choice.
pub fn words(table: &[(&str, &str)], stored: &str) -> String {
    table.iter().find(|(k, _)| *k == stored).map(|(_, w)| w.to_string()).unwrap_or_else(|| stored.to_string())
}

/// What happened while you were away.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AwayView {
    pub away: bool,
    pub decisions: Vec<String>,
    pub parked: Vec<String>,
}

impl AwayView {
    pub fn from_json(v: &Value) -> AwayView {
        AwayView {
            away: v["away"].as_bool().unwrap_or(false),
            decisions: arr(v, "decisions").iter().map(|d| format!("{}: {}", d["dept"].as_str().unwrap_or(""), d["decision"].as_str().unwrap_or(""))).collect(),
            parked: arr(v, "parked").iter().filter_map(Value::as_str).map(str::to_string).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lh_team::{Department, Role, Target};

    fn good() -> RoleForm {
        RoleForm { name: "web-designer".into(), description: "Designs pages.".into(), tools: vec!["read_file".into(), "write_file".into()], temperature: "0.7".into(), top_k: "40".into(), instructions: "You design pages.\nKeep it simple.".into(), thinking: "off".into(), ..RoleForm::blank() }
    }

    #[test]
    fn a_role_form_makes_text_the_core_reads_back_the_same() {
        let text = good().compose().unwrap();
        let r = Role::parse(&text, Some("web-designer")).unwrap();
        assert_eq!((r.target.clone(), r.tools.clone(), r.max_steps, r.thinking, r.sampling.top_k), (Target::Type("mid-tier".into()), vec!["read_file".to_string(), "write_file".to_string()], 12, Some(false), Some(40)));
        assert_eq!((r.sampling.temperature, r.instructions.as_str(), r.description.as_str()), (Some(0.7), "You design pages.\nKeep it simple.", "Designs pages."));
        // an exact model, quotes in the description
        let mut f = good();
        f.kind = "model".into();
        f.target = "Qwen \"big\"".into();
        f.description = "Says \"hi\"".into();
        let r = Role::parse(&f.compose().unwrap(), Some("web-designer")).unwrap();
        assert_eq!((r.target, r.description.as_str()), (Target::Model("Qwen \"big\"".into()), "Says \"hi\""));
    }

    #[test]
    fn a_role_form_explains_what_is_wrong() {
        let bad = |f: RoleForm| f.compose().unwrap_err();
        assert!(bad(RoleForm { name: "Web Designer".into(), ..good() }).contains("lower case"));
        assert!(bad(RoleForm { description: " ".into(), ..good() }).contains("description"));
        assert!(bad(RoleForm { instructions: "x".repeat(601), ..good() }).contains("601"));
        assert!(bad(RoleForm { instructions: "".into(), ..good() }).contains("instructions"));
        assert!(bad(RoleForm { temperature: "warm".into(), ..good() }).contains("Temperature must be a number"));
        assert!(bad(RoleForm { top_k: "2.5".into(), ..good() }).contains("Top K"));
        assert!(bad(RoleForm { max_steps: "0".into(), ..good() }).contains("1 to 100"));
        assert!(bad(RoleForm { target: "".into(), ..good() }).contains("model type"));
    }

    #[test]
    fn the_core_role_fills_the_form_and_back_again() {
        let j = json!({"name": "coder", "description": "Codes.", "target": {"kind": "type", "value": "mid-tier"}, "tools": ["read_file"], "memory": "preferences projects", "sampling": {"temperature": 0.2, "top_p": null, "top_k": null, "min_p": null, "repeat_penalty": null}, "max_steps": 12, "thinking": null, "instructions": "Write code."});
        let f = RoleForm::from_json(&j);
        assert_eq!((f.temperature.as_str(), f.top_p.as_str(), f.thinking.as_str(), f.memory.as_str()), ("0.2", "", "", "preferences projects"));
        let r = Role::parse(&f.compose().unwrap(), Some("coder")).unwrap();
        assert_eq!(r.sampling.temperature, Some(0.2));
    }

    #[test]
    fn a_department_form_makes_text_the_core_reads_back() {
        let f = DeptForm { name: "web".into(), description: "Makes sites.".into(), lead: "planner".into(), members: vec!["coder".into(), "dept:docs".into()], parallel: "2".into(), attempts: "".into(), instructions: "Keep it small.".into() };
        let d = Department::parse(&f.compose().unwrap(), Some("web")).unwrap();
        assert_eq!((d.lead.as_str(), d.members.len(), d.parallel, d.attempts, d.instructions.as_str()), ("planner", 2, Some(2), None, "Keep it small."));
        assert!(DeptForm { members: vec![], ..f.clone() }.compose().unwrap_err().contains("at least one"));
        assert!(DeptForm { lead: "".into(), ..f.clone() }.compose().unwrap_err().contains("lead"));
        assert!(DeptForm { parallel: "40".into(), ..f.clone() }.compose().unwrap_err().contains("1 to 16"));
        assert!(DeptForm { attempts: "x".into(), ..f }.compose().unwrap_err().contains("Attempts"));
    }

    #[test]
    fn deleting_a_type_asks_where_each_role_that_uses_it_goes() {
        let roles = role_targets(&json!({"roles": [
            {"name": "coder", "target": {"kind": "type", "value": "mid-tier"}},
            {"name": "writer", "target": {"kind": "type", "value": "mid-tier"}},
            {"name": "planner", "target": {"kind": "type", "value": "frontier"}},
            {"name": "odd", "target": {"kind": "model", "value": "mid-tier"}},
            {"name": "broken", "error": "bad"}]}));
        let affected = roles_using_type(&roles, "mid-tier");
        assert_eq!(affected, ["coder", "writer"]);
        let ts = types(&json!({"types": [{"name": "mid-tier", "model": "q", "fallbacks": ["s"], "stock": true}, {"name": "small", "model": null, "fallbacks": [], "stock": false}]}));
        assert_eq!((ts[0].fallbacks.clone(), ts[1].model.clone()), (vec!["s".to_string()], None));
        let mut homes = default_homes(&affected, &ts, "mid-tier");
        assert_eq!(homes["coder"], Home::Type("small".into()));
        homes.insert("writer".into(), Home::Model("Qwen".into()));
        assert_eq!(reassign_body(&affected, &homes).unwrap(), json!({"reassign": {"coder": {"type": "small"}, "writer": {"model": "Qwen"}}}));
        homes.remove("writer");
        assert!(reassign_body(&affected, &homes).unwrap_err().contains("writer"));
        assert_eq!(reassign_body(&[], &BTreeMap::new()).unwrap(), json!({"reassign": {}}));
        // no other type to move to: nothing is chosen for the person
        assert!(default_homes(&affected, &ts[..1], "mid-tier").is_empty());
    }

    #[test]
    fn team_settings_and_away_read_in_words() {
        let t = TeamSettingsView::from_json(&json!({"parallel": 2, "attempts": 3, "autonomy": "check-ins", "unavailable": "stop", "away_since": 12345, "limits": {"parallel_max": 16, "attempts_max": 10}}));
        assert!(t.away && t.parallel == 2);
        assert_eq!(words(&AUTONOMY, &t.autonomy), "Also pause at milestones");
        assert_eq!(words(&UNAVAILABLE, "odd"), "odd");
        assert!(!TeamSettingsView::from_json(&json!({"away_since": null})).away);
        let a = AwayView::from_json(&json!({"away": true, "decisions": [{"dept": "web", "decision": "said no to rm"}], "parked": ["step 2"]}));
        assert_eq!((a.decisions, a.parked), (vec!["web: said no to rm".to_string()], vec!["step 2".to_string()]));
    }

    #[test]
    fn the_tool_list_is_the_cores() {
        assert_eq!(KNOWN_TOOLS.as_slice(), lh_team::KNOWN_TOOLS);
    }

    #[test]
    fn names_follow_the_core_rule() {
        for ok in ["web", "web-designer", "a1"] {
            assert!(valid_name(ok), "{ok}");
        }
        for bad in ["", "Web", "-a", "a-", "a--b", "a b", "a_b"] {
            assert!(!valid_name(bad), "{bad}");
        }
    }
}
