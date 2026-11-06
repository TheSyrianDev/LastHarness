//! `lh providers ...`: the registered model providers and the proposals the model has made.
//! Keys are read with hidden input, sent once to the core and never printed or logged.

use serde_json::{json, Value};

use crate::client::{Client, Result};

pub const HELP: &str = "lh providers                         the registered providers (default marked)
lh provider add <preset|url> [--name <name>]   add one; asks for a key (hidden) when it needs one
lh provider default <id>             make it the default
lh provider remove <id>
lh providers presets                 the known providers you can add by name
lh proposals                         providers the model suggested, waiting for you
lh approve <id>                      accept a suggestion (asks for a key if it needs one)
lh reject <id>
(a leading / is accepted: /providers, /provider add openrouter, /proposals, /approve p1)";

/// Where a typed key comes from. The terminal hides typing; tests script it.
pub trait Keys {
    /// Ask for a key. An empty answer means none.
    fn key(&mut self, question: &str) -> String;
}

/// Hidden input: raw mode, nothing is echoed.
pub struct Hidden;
impl Keys for Hidden {
    fn key(&mut self, question: &str) -> String {
        use crossterm::event::{read, Event, KeyCode, KeyEventKind, KeyModifiers};
        use std::io::Write;
        print!("{question} (hidden, Enter for none) ");
        let _ = std::io::stdout().flush();
        if crossterm::terminal::enable_raw_mode().is_err() {
            return String::new();
        }
        let mut s = String::new();
        while let Ok(Event::Key(k)) = read() {
            if k.kind == KeyEventKind::Release {
                continue;
            }
            match k.code {
                KeyCode::Enter => break,
                KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                    s.clear();
                    break;
                }
                KeyCode::Char(c) => s.push(c),
                KeyCode::Backspace => {
                    s.pop();
                }
                KeyCode::Esc => {
                    s.clear();
                    break;
                }
                _ => {}
            }
        }
        let _ = crossterm::terminal::disable_raw_mode();
        println!();
        s.trim().to_string()
    }
}

fn need<'a>(a: &'a [String], i: usize, what: &str) -> std::result::Result<&'a str, String> {
    a.get(i).map(String::as_str).ok_or_else(|| format!("missing {what}. Run `lh providers help`."))
}

fn line(p: &Value, default: &str) -> String {
    format!(
        "{}{}  {}  {}  [{}]{}",
        if p["id"] == default { "* " } else { "  " },
        p["id"].as_str().unwrap_or(""),
        p["name"].as_str().unwrap_or(""),
        p["url"].as_str().unwrap_or(""),
        p["api"].as_str().unwrap_or(""),
        if p["key_saved"] == true { ", key saved" } else { "" }
    )
}

async fn key_for(c: &Client, preset: Option<&str>, keys: &mut dyn Keys) -> Result<Option<String>> {
    let required = match preset {
        Some(id) => c.call("GET", "/v1/providers/presets", None).await?["presets"].as_array().into_iter().flatten().find(|p| p["id"] == id).map(|p| p["key_required"] == true),
        None => None,
    };
    if required == Some(false) {
        return Ok(None);
    }
    let k = keys.key("API key:");
    Ok((!k.is_empty()).then_some(k))
}

pub async fn run(c: &Client, args: &[String], keys: &mut dyn Keys) -> Result<String> {
    let mut a: Vec<String> = args.to_vec();
    if let Some(f) = a.first_mut() {
        *f = f.trim_start_matches('/').to_string();
    }
    // "lh provider add x" and "lh providers add x" both work; "proposals/approve/reject" stand alone
    let (verb, rest): (String, Vec<String>) = match a.first().map(String::as_str) {
        Some("provider" | "providers") => (a.get(1).cloned().unwrap_or_else(|| "list".into()), a.iter().skip(2).cloned().collect()),
        Some(v) => (v.to_string(), a.iter().skip(1).cloned().collect()),
        None => ("list".into(), vec![]),
    };
    match verb.as_str() {
        "help" | "--help" | "-h" => Ok(HELP.into()),
        "list" => {
            let v = c.call("GET", "/v1/providers", None).await?;
            let d = v["default"].as_str().unwrap_or("").to_string();
            let l = v["providers"].as_array().cloned().unwrap_or_default();
            if l.is_empty() {
                return Ok("No providers yet. Add one: lh provider add <preset|url>".into());
            }
            Ok(l.iter().map(|p| line(p, &d)).collect::<Vec<_>>().join("\n"))
        }
        "presets" => {
            let v = c.call("GET", "/v1/providers/presets", None).await?;
            Ok(v["presets"].as_array().into_iter().flatten().filter(|p| p["supported"] == true).map(|p| format!("{}  {}  {}{}", p["id"].as_str().unwrap_or(""), p["name"].as_str().unwrap_or(""), p["url"].as_str().unwrap_or(""), if p["key_required"] == true { "  (needs a key)" } else { "" })).collect::<Vec<_>>().join("\n"))
        }
        "add" => {
            let what = need(&rest, 0, "a preset or an address")?;
            let name = rest.iter().position(|x| x == "--name").and_then(|i| rest.get(i + 1)).cloned();
            let is_url = what.starts_with("http://") || what.starts_with("https://");
            let mut body = if is_url { json!({"url": what}) } else { json!({"preset": what}) };
            if let Some(n) = name {
                body["name"] = json!(n);
            }
            if let Some(k) = key_for(c, (!is_url).then_some(what), keys).await? {
                body["key"] = json!(k);
            }
            let v = c.call("POST", "/v1/providers", Some(body)).await?;
            Ok(format!("Added {}.", v["added"]["id"].as_str().unwrap_or(what)))
        }
        "default" => {
            let id = need(&rest, 0, "a provider id")?;
            c.call("POST", &format!("/v1/providers/{id}/default"), None).await.map(|_| format!("{id} is now the default."))
        }
        "remove" => {
            let id = need(&rest, 0, "a provider id")?;
            c.call("DELETE", &format!("/v1/providers/{id}"), None).await.map(|_| format!("Removed {id}."))
        }
        "proposals" => {
            let v = c.call("GET", "/v1/providers/proposals", None).await?;
            let l = v["proposals"].as_array().cloned().unwrap_or_default();
            if l.is_empty() {
                return Ok("No proposals.".into());
            }
            Ok(l.iter().map(|p| format!("{}  {}  {}\n    why: {}", p["id"].as_str().unwrap_or(""), p["name"].as_str().unwrap_or(""), p["url"].as_str().unwrap_or(""), p["reason"].as_str().unwrap_or(""))).collect::<Vec<_>>().join("\n"))
        }
        "approve" => {
            let id = need(&rest, 0, "a proposal id")?;
            let all = c.call("GET", "/v1/providers/proposals", None).await?;
            let p = all["proposals"].as_array().into_iter().flatten().find(|p| p["id"] == id).cloned().ok_or_else(|| format!("No proposal called {id}."))?;
            let preset = p["preset"].as_str().map(str::to_string);
            let body = match key_for(c, preset.as_deref(), keys).await? {
                Some(k) => json!({"key": k}),
                None => json!({}),
            };
            let v = c.call("POST", &format!("/v1/providers/proposals/{id}/approve"), Some(body)).await?;
            Ok(format!("Approved. Added {}.", v["added"]["id"].as_str().unwrap_or(id)))
        }
        "reject" => {
            let id = need(&rest, 0, "a proposal id")?;
            c.call("POST", &format!("/v1/providers/proposals/{id}/reject"), Some(json!({}))).await.map(|_| format!("Rejected {id}."))
        }
        other => Err(format!("unknown command {other}. Run `lh providers help`.")),
    }
}
