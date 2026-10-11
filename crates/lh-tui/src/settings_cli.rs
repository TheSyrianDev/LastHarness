//! `lh settings`: the single place to see and change every setting. It talks to the running core, and
//! falls back to the settings file when the core is not running.

use lh_config::{ConfigFile, Kind};
use serde_json::{json, Value};

use crate::client::{Client, Result};

pub const HELP: &str = "lh settings                  all settings with their values
lh settings <group>          only one group (provider, memory, learning, team, media, web, spending, permissions, core, reports, updates)
lh settings set <key> <value>   change one (many keys: lh settings set a=1 b=2)
lh settings reset <key>      back to the default
lh settings help";

fn kind_text(k: &Value) -> String {
    match k["type"].as_str().unwrap_or("") {
        "bool" => "on/off".into(),
        "int" => format!("{} to {}", k["min"], k["max"]),
        "choice" => k["options"].as_array().map(|o| o.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" | ")).unwrap_or_default(),
        "auto-int" => "auto or a number".into(),
        "number" => "a number, or empty".into(),
        _ => "text".into(),
    }
}

fn render(items: &[Value], only: Option<&str>) -> String {
    let mut out = String::new();
    let mut group = "";
    for i in items.iter().filter(|i| only.is_none_or(|g| i["group"] == g)) {
        if i["group"].as_str().unwrap_or("") != group {
            group = i["group"].as_str().unwrap_or("");
            out.push_str(&format!("\n[{group}]\n"));
        }
        let v = i["value"].as_str().unwrap_or("");
        out.push_str(&format!("  {:<30} {:<24} {}{}\n      {}\n", i["key"].as_str().unwrap_or(""), if v.is_empty() { "(empty)" } else { v }, if i["is_set"] == true { "" } else { "(default)  " }, if i["live"] == true { "[now]" } else { "[next start]" }, i["help"].as_str().unwrap_or("")));
        let _ = kind_text(&i["kind"]);
    }
    out.trim_start().trim_end().to_string()
}

fn offline_items(cfg: &ConfigFile) -> Vec<Value> {
    cfg.all().into_iter().map(|(d, value, set)| {
        let kind = match d.kind {
            Kind::Bool => json!({"type": "bool"}),
            Kind::Int { min, max } => json!({"type": "int", "min": min, "max": max}),
            Kind::Choice(o) => json!({"type": "choice", "options": o}),
            Kind::AutoInt => json!({"type": "auto-int"}),
            Kind::OptFloat => json!({"type": "number"}),
            Kind::Text => json!({"type": "text"}),
        };
        json!({"key": d.key, "group": d.group(), "value": value, "default": d.default, "is_set": set, "kind": kind, "live": d.live, "help": d.help})
    }).collect()
}

pub async fn run(c: &Client, args: &[String], file: &std::path::Path) -> Result<String> {
    match args.first().map(String::as_str) {
        Some("help" | "--help" | "-h") => Ok(HELP.into()),
        Some("set") => {
            let mut body = serde_json::Map::new();
            let rest = &args[1..];
            if rest.len() == 2 && !rest[0].contains('=') {
                body.insert(rest[0].clone(), json!(rest[1]));
            } else {
                for kv in rest {
                    let (k, v) = kv.split_once('=').ok_or("write it as: lh settings set <key> <value>, or key=value pairs")?;
                    body.insert(k.into(), json!(v));
                }
            }
            if body.is_empty() {
                return Err("nothing to set. Run `lh settings help`.".into());
            }
            apply(c, body, file).await
        }
        Some("reset") => {
            let key = args.get(1).ok_or("which setting? Run `lh settings help`.")?;
            let d = lh_config::def(key).ok_or(format!("there is no setting called {key}"))?;
            let mut body = serde_json::Map::new();
            body.insert(key.clone(), json!(d.default));
            apply(c, body, file).await
        }
        group => {
            let items = match c.call("GET", "/v1/settings", None).await {
                Ok(v) => v["settings"].as_array().cloned().unwrap_or_default(),
                Err(_) => offline_items(&ConfigFile::load(file).map_err(|e| e.to_string())?),
            };
            Ok(render(&items, group))
        }
    }
}

async fn apply(c: &Client, body: serde_json::Map<String, Value>, file: &std::path::Path) -> Result<String> {
    match c.call("PUT", "/v1/settings", Some(Value::Object(body.clone()))).await {
        Ok(v) => {
            let mut out = format!("Changed: {}.", v["changed"].as_array().map(|a| a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", ")).unwrap_or_default());
            let r: Vec<&str> = v["restart_needed"].as_array().map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
            if !r.is_empty() {
                out.push_str(&format!("\nTakes effect when the core next starts: {}.", r.join(", ")));
            }
            Ok(out)
        }
        // the core is not running: change the file directly
        Err(e) if e.starts_with("Cannot reach the core") => {
            let mut cfg = ConfigFile::load(file).map_err(|e| e.to_string())?;
            for (k, v) in &body {
                cfg.set(k, v.as_str().unwrap_or("")).map_err(|e| e.to_string())?;
            }
            cfg.save().map_err(|e| e.to_string())?;
            Ok(format!("The core is not running; saved to {}. It applies at the next start.", file.display()))
        }
        Err(e) => Err(e),
    }
}
