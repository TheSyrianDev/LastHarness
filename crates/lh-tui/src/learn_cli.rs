//! `lh learn ...`: see and control what the assistant has learned. Rules, the style profile, skills,
//! suggestions waiting for a decision, and the habits it has noticed. Everything goes through the core.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{json, Value};

use crate::client::{Client, Result};

pub const HELP: &str = "lh learn <command>
  rules [--all]                              the rules in force (--all includes switched-off ones)
  rule add <text> [--topic T] [--project P]  add a rule yourself
  rule edit <id> -- <new text>               change the wording (the old one stays in the history)
  rule on|off <id>                           switch a rule on or off
  rule show <id>                             the rule, where it came from, and every version
  rule rollback <id> <version>               go back to an earlier version
  rule delete <id>                           remove it for good, including its history text
  style                                      the style profile (how to talk to you)
  style set -- <text>                        replace it
  style rollback <version>
  skills                                     the skills, and any that are broken
  skill show <name>
  skill add <folder>                         install a skill folder (SKILL.md and its other files)
  skill rollback <name> <version>
  skill delete <name>
  proposals                                  suggestions waiting for you
  accept <id> [--text <your wording>]        apply a suggestion, optionally with your own wording (all words after --text)
  reject <id>
  habits [--min N]                           wishes you have repeated across sessions
  signals                                    the raw things you said that look like rules";

fn need<'a>(a: &'a [String], i: usize, what: &str) -> std::result::Result<&'a str, String> {
    a.get(i).map(String::as_str).ok_or_else(|| format!("missing {what}. Run `lh learn help`."))
}

fn arr<'a>(v: &'a Value, k: &str) -> &'a [Value] {
    v[k].as_array().map(Vec::as_slice).unwrap_or(&[])
}

fn rule_line(r: &Value) -> String {
    let mut f = vec![];
    if r["privacy"] == "sensitive" {
        f.push("sensitive".to_string());
    }
    if r["active"] == false {
        f.push("off".into());
    }
    if let Some(p) = r["project"].as_str() {
        f.push(format!("project {p}"));
    }
    if let Some(p) = r["role"].as_str() {
        f.push(format!("role {p}"));
    }
    format!("{}  {}{}", r["id"].as_str().unwrap_or(""), r["text"].as_str().unwrap_or(""), if f.is_empty() { String::new() } else { format!("  [{}]", f.join(", ")) })
}

fn proposal_line(p: &Value) -> String {
    let pl = &p["payload"];
    let what = match p["kind"].as_str().unwrap_or("") {
        "rule" => format!("new rule: {}", pl["text"].as_str().unwrap_or("")),
        "rule_update" => format!("change a rule: \"{}\" -> \"{}\"", pl["was"].as_str().unwrap_or("?"), pl["text"].as_str().unwrap_or("")),
        "style" => format!("add to your style profile: {}", pl["added"].as_str().unwrap_or("")),
        "skill_new" => format!("new skill {}: {}", pl["name"].as_str().unwrap_or(""), pl["description"].as_str().unwrap_or("")),
        "skill_update" => format!("change skill {}: {}", pl["name"].as_str().unwrap_or(""), pl["note"].as_str().unwrap_or("")),
        k => k.to_string(),
    };
    let ev = p["evidence"].as_array().and_then(|e| e.first()).and_then(Value::as_str).map(|e| format!("\n      because: \"{e}\"")).unwrap_or_default();
    format!("{}  {what}{}{ev}", p["id"], if pl["privacy"] == "sensitive" { "  [sensitive]" } else { "" })
}

fn read_folder(dir: &Path) -> std::result::Result<BTreeMap<String, String>, String> {
    fn walk(root: &Path, d: &Path, out: &mut BTreeMap<String, String>) -> std::result::Result<(), String> {
        for e in std::fs::read_dir(d).map_err(|e| format!("{}: {e}", d.display()))?.flatten() {
            let p = e.path();
            if e.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            if p.is_dir() {
                walk(root, &p, out)?;
            } else if let Ok(t) = std::fs::read_to_string(&p) {
                out.insert(p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"), t);
            }
        }
        Ok(())
    }
    let mut m = BTreeMap::new();
    walk(dir, dir, &mut m)?;
    if !m.contains_key("SKILL.md") {
        return Err(format!("{} has no SKILL.md", dir.display()));
    }
    Ok(m)
}

pub async fn run(c: &Client, args: &[String]) -> Result<String> {
    let cmd = args.first().map(String::as_str).unwrap_or("help");
    let rest = if args.is_empty() { &[][..] } else { &args[1..] };
    let after_dashes = |r: &[String]| r.iter().position(|a| a == "--").map(|i| r[i + 1..].join(" ")).unwrap_or_default();
    let opt = |r: &[String], name: &str| r.iter().position(|a| a == name).and_then(|i| r.get(i + 1)).cloned();
    match cmd {
        "help" | "--help" | "-h" => Ok(HELP.into()),
        "rules" => {
            let v = c.call("GET", &format!("/v1/learn/rules?all={}", rest.iter().any(|a| a == "--all")), None).await?;
            let r = arr(&v, "rules");
            Ok(if r.is_empty() { "No rules yet.".into() } else { r.iter().map(rule_line).collect::<Vec<_>>().join("\n") })
        }
        "rule" => {
            let sub = need(rest, 0, "a rule command")?;
            let tail = &rest[1..];
            let id = || need(tail, 0, "rule id").map(str::to_string);
            match sub {
                "add" => {
                    let (mut text, mut words) = (Vec::new(), tail.iter());
                    while let Some(a) = words.next() {
                        if a.starts_with("--") {
                            words.next(); // an option and its value
                        } else {
                            text.push(a.as_str());
                        }
                    }
                    let v = c.call("POST", "/v1/learn/rules", Some(json!({"text": text.join(" "), "topic": opt(tail, "--topic"), "project": opt(tail, "--project")}))).await?;
                    Ok(format!("Added: {}", rule_line(&v)))
                }
                "edit" => Ok(format!("Changed: {}", rule_line(&c.call("PATCH", &format!("/v1/learn/rules/{}", id()?), Some(json!({"text": after_dashes(tail)}))).await?))),
                "on" | "off" => Ok(format!("{}: {}", if sub == "on" { "On" } else { "Off" }, rule_line(&c.call("POST", &format!("/v1/learn/rules/{}/status", id()?), Some(json!({"active": sub == "on"}))).await?))),
                "show" => {
                    let v = c.call("GET", &format!("/v1/learn/rules/{}", id()?), None).await?;
                    let mut out = format!("{}\nfrom: {}\n", rule_line(&v["rule"]), v["rule"]["source"].as_str().unwrap_or(""));
                    for e in v["rule"]["evidence"].as_array().into_iter().flatten().filter_map(Value::as_str) {
                        out.push_str(&format!("because: \"{e}\"\n"));
                    }
                    out.push_str("versions:\n");
                    for h in arr(&v, "history") {
                        out.push_str(&format!("  v{}  {}  by {}  {}\n", h["version"], h["op"].as_str().unwrap_or(""), h["by"].as_str().unwrap_or(""), h["content"]["text"].as_str().unwrap_or("")));
                    }
                    Ok(out.trim_end().into())
                }
                "rollback" => {
                    let ver: i64 = need(tail, 1, "version number")?.parse().map_err(|_| "the version must be a number".to_string())?;
                    Ok(format!("Rolled back: {}", rule_line(&c.call("POST", &format!("/v1/learn/rules/{}/rollback", id()?), Some(json!({"version": ver}))).await?)))
                }
                "delete" => {
                    c.call("DELETE", &format!("/v1/learn/rules/{}", id()?), None).await?;
                    Ok("Deleted for good.".into())
                }
                other => Err(format!("unknown rule command `{other}`. Run `lh learn help`.")),
            }
        }
        "style" => match rest.first().map(String::as_str) {
            None => {
                let v = c.call("GET", "/v1/learn/style", None).await?;
                Ok(match v["style"]["text"].as_str() {
                    Some(t) => format!("{t}\n(version {})", v["style"]["version"]),
                    None => "No style profile yet.".into(),
                })
            }
            Some("set") => {
                let v = c.call("PUT", "/v1/learn/style", Some(json!({"text": after_dashes(rest)}))).await?;
                Ok(format!("Style profile is now version {}.", v["version"]))
            }
            Some("rollback") => {
                let ver: i64 = need(rest, 1, "version number")?.parse().map_err(|_| "the version must be a number".to_string())?;
                let v = c.call("POST", "/v1/learn/style/rollback", Some(json!({"version": ver}))).await?;
                Ok(format!("Style profile is back to the text of version {ver} (now version {}).", v["version"]))
            }
            Some(o) => Err(format!("unknown style command `{o}`. Run `lh learn help`.")),
        },
        "skills" => {
            let v = c.call("GET", "/v1/learn/skills", None).await?;
            let s = arr(&v, "skills");
            Ok(if s.is_empty() { "No skills yet.".into() } else { s.iter().map(|x| match x["error"].as_str() { Some(e) => format!("{}  BROKEN: {e}", x["name"].as_str().unwrap_or("")), None => format!("{}  {}", x["name"].as_str().unwrap_or(""), x["description"].as_str().unwrap_or("")) }).collect::<Vec<_>>().join("\n") })
        }
        "skill" => {
            let sub = need(rest, 0, "a skill command")?;
            let name = need(rest, 1, "skill name or folder")?;
            match sub {
                "show" => {
                    let v = c.call("GET", &format!("/v1/learn/skills/{name}"), None).await?;
                    let files = arr(&v, "files").iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", ");
                    Ok(format!("{}: {}\nother files: {}\n\n{}\n\nversions: {}", name, v["description"].as_str().unwrap_or(""), if files.is_empty() { "none" } else { &files }, v["body"].as_str().unwrap_or(""), arr(&v, "history").len()))
                }
                "add" => {
                    let v = c.call("POST", "/v1/learn/skills", Some(json!({"files": read_folder(Path::new(name))?}))).await?;
                    Ok(format!("Installed skill {}.", v["name"].as_str().unwrap_or("")))
                }
                "rollback" => {
                    let ver: i64 = need(rest, 2, "version number")?.parse().map_err(|_| "the version must be a number".to_string())?;
                    c.call("POST", &format!("/v1/learn/skills/{name}/rollback"), Some(json!({"version": ver}))).await?;
                    Ok(format!("Skill {name} is back to version {ver}."))
                }
                "delete" => {
                    c.call("DELETE", &format!("/v1/learn/skills/{name}"), None).await?;
                    Ok("Deleted for good.".into())
                }
                other => Err(format!("unknown skill command `{other}`. Run `lh learn help`.")),
            }
        }
        "proposals" => {
            let v = c.call("GET", "/v1/learn/proposals", None).await?;
            let p = arr(&v, "proposals");
            Ok(if p.is_empty() { "No suggestions waiting.".into() } else { p.iter().map(proposal_line).collect::<Vec<_>>().join("\n") })
        }
        "accept" => {
            let id = need(rest, 0, "suggestion number")?;
            let body = rest.iter().position(|a| a == "--text").map(|i| json!({"edit": {"text": rest[i + 1..].join(" ")}}));
            let v = c.call("POST", &format!("/v1/learn/proposals/{id}/accept"), Some(body.unwrap_or(json!({})))).await?;
            Ok(if v["rule"].is_object() { format!("Now a rule: {}", rule_line(&v["rule"])) } else if v["style"].is_object() { "Style profile updated.".to_string() } else { format!("Skill saved: {}", v["skill"].as_str().unwrap_or("")) })
        }
        "reject" => {
            c.call("POST", &format!("/v1/learn/proposals/{}/reject", need(rest, 0, "suggestion number")?), Some(json!({}))).await?;
            Ok("Dropped.".into())
        }
        "habits" => {
            let min = opt(rest, "--min").unwrap_or_else(|| "3".into());
            let v = c.call("GET", &format!("/v1/learn/habits?min={min}"), None).await?;
            let h = arr(&v, "habits");
            Ok(if h.is_empty() { "No habit noticed yet.".into() } else { h.iter().map(|x| format!("{} sessions: {}", x["sessions"], x["examples"].as_array().and_then(|e| e.first()).and_then(Value::as_str).unwrap_or(""))).collect::<Vec<_>>().join("\n") })
        }
        "signals" => {
            let v = c.call("GET", "/v1/learn/signals", None).await?;
            let s = arr(&v, "signals");
            Ok(if s.is_empty() { "Nothing noted yet.".into() } else { s.iter().map(|x| format!("{}  {}  {}", x["session"].as_str().unwrap_or(""), x["kind"].as_str().unwrap_or(""), x["text"].as_str().unwrap_or(""))).collect::<Vec<_>>().join("\n") })
        }
        other => Err(format!("unknown learn command `{other}`. Run `lh learn help`.")),
    }
}
