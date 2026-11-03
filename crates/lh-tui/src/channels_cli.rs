//! `lh channels`: set up messaging channels (Telegram, Matrix, Signal, Slack). A channel is off until you
//! have read its warning and switched it on, and only the people you list are answered.

use serde_json::{json, Value};

use crate::client::{Client, Result};

pub const HELP: &str = "lh channels                              the channels, and whether each is running
lh channels kinds                        what each kind needs, and its warning
lh channels add <name> --kind telegram|matrix|slack|signal --allow <id>[,<id>...] --model <model>
        [--mode chat|assistant] --set <setting>=<value>...
                                         a new channel; it starts OFF. Secrets are written env:NAME
lh channels edit <name> [--allow ...] [--model ...] [--mode ...] [--set k=v]...
lh channels enable <name> [--yes]        shows the warning, then asks before switching it on
lh channels disable <name>
lh channels remove <name>";

fn opt(a: &[String], name: &str) -> Option<String> {
    a.iter().position(|x| x == name).and_then(|i| a.get(i + 1)).cloned()
}

fn sets(a: &[String]) -> serde_json::Map<String, Value> {
    let mut m = serde_json::Map::new();
    for (i, x) in a.iter().enumerate() {
        if x == "--set" {
            if let Some((k, v)) = a.get(i + 1).and_then(|kv| kv.split_once('=')) {
                m.insert(k.to_string(), json!(v));
            }
        }
    }
    m
}

fn line(c: &Value) -> String {
    let st = &c["status"];
    let state = if c["enabled"] != true {
        "off".to_string()
    } else if st["running"] == true {
        "running".to_string()
    } else {
        format!("on, not running{}", st["last_error"].as_str().map(|e| format!(" ({e})")).unwrap_or_default())
    };
    format!(
        "{:<12} {:<9} {:<9} mode {:<9} model {}   allowed: {}   answered {} ignored {}{}",
        c["name"].as_str().unwrap_or(""), c["kind"].as_str().unwrap_or(""), state, c["mode"].as_str().unwrap_or(""), c["model"].as_str().unwrap_or("(none)"),
        c["allow"].as_array().map(|a| a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(",")).unwrap_or_default(),
        st["handled"].as_u64().unwrap_or(0), st["ignored"].as_u64().unwrap_or(0),
        if state.starts_with("on,") || st["last_error"].is_null() { String::new() } else { format!("   last problem: {}", st["last_error"].as_str().unwrap_or("")) }
    )
}

pub async fn run(c: &Client, args: &[String], confirm: &mut dyn FnMut(&str) -> bool) -> Result<String> {
    let rest = if args.is_empty() { &[][..] } else { &args[1..] };
    match args.first().map(String::as_str) {
        Some("help" | "--help" | "-h") => Ok(HELP.into()),
        None => {
            let v = c.call("GET", "/v1/channels", None).await?;
            let list = v["channels"].as_array().cloned().unwrap_or_default();
            Ok(if list.is_empty() { "No channels. They are all off until you add one: lh channels help".into() } else { list.iter().map(line).collect::<Vec<_>>().join("\n") })
        }
        Some("kinds") => {
            let v = c.call("GET", "/v1/channels", None).await?;
            let nm = |x: &Value| format!("    {}: {}", x["name"].as_str().unwrap_or(""), x["help"].as_str().unwrap_or(""));
            Ok(v["kinds"].as_array().into_iter().flatten().map(|k| {
                format!("{}\n  needs:\n{}\n  may have:\n{}\n  warning: {}", k["kind"].as_str().unwrap_or(""), k["required"].as_array().into_iter().flatten().map(nm).collect::<Vec<_>>().join("\n"), k["optional"].as_array().into_iter().flatten().map(nm).collect::<Vec<_>>().join("\n"), k["warning"].as_str().unwrap_or(""))
            }).collect::<Vec<_>>().join("\n\n"))
        }
        Some("add") => {
            let name = rest.first().ok_or("give the channel a name: lh channels add <name> --kind ...")?;
            let kind = opt(rest, "--kind").ok_or("which kind? --kind telegram|matrix|slack|signal")?;
            let allow: Vec<String> = opt(rest, "--allow").map(|a| a.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()).unwrap_or_default();
            let v = c.call("POST", "/v1/channels", Some(json!({"name": name, "kind": kind, "allow": allow, "model": opt(rest, "--model").unwrap_or_default(), "mode": opt(rest, "--mode").unwrap_or_else(|| "chat".into()), "settings": sets(rest)}))).await?;
            Ok(format!("Added {name}. It is OFF.\n\n{}\n\nWhen you have read that: lh channels enable {name}", v["warning"].as_str().unwrap_or("")))
        }
        Some("edit") => {
            let name = rest.first().ok_or("which channel?")?;
            let mut body = serde_json::Map::new();
            if let Some(a) = opt(rest, "--allow") {
                body.insert("allow".into(), json!(a.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect::<Vec<_>>()));
            }
            for (flag, key) in [("--model", "model"), ("--mode", "mode")] {
                if let Some(v) = opt(rest, flag) {
                    body.insert(key.into(), json!(v));
                }
            }
            let s = sets(rest);
            if !s.is_empty() {
                let cur = c.call("GET", "/v1/channels", None).await?;
                let mut merged = cur["channels"].as_array().into_iter().flatten().find(|x| x["name"] == name.as_str()).map(|x| x["settings"].clone()).and_then(|v| v.as_object().cloned()).unwrap_or_default();
                merged.extend(s);
                body.insert("settings".into(), Value::Object(merged));
            }
            c.call("PUT", &format!("/v1/channels/{name}"), Some(Value::Object(body))).await.map(|v| line(&v))
        }
        Some("enable") => {
            let name = rest.first().ok_or("which channel?")?;
            let v = c.call("GET", "/v1/channels", None).await?;
            let kind = v["channels"].as_array().into_iter().flatten().find(|x| x["name"] == name.as_str()).and_then(|x| x["kind"].as_str()).ok_or("no such channel")?.to_string();
            let warning = v["kinds"].as_array().into_iter().flatten().find(|k| k["kind"] == kind.as_str()).and_then(|k| k["warning"].as_str()).unwrap_or("").to_string();
            if !rest.iter().any(|a| a == "--yes") && !confirm(&format!("{warning}\n\nSwitch on {name}? (yes/no)")) {
                return Ok("Left off.".into());
            }
            c.call("POST", &format!("/v1/channels/{name}/enable"), Some(json!({"acknowledge": true}))).await.map(|v| format!("Switched on.\n{}", line(&v)))
        }
        Some("disable") => c.call("POST", &format!("/v1/channels/{}/disable", rest.first().ok_or("which channel?")?), None).await.map(|v| format!("Switched off.\n{}", line(&v))),
        Some("remove") => c.call("DELETE", &format!("/v1/channels/{}", rest.first().ok_or("which channel?")?), None).await.map(|_| "Removed.".into()),
        Some(o) => Err(format!("unknown command {o}. Run `lh channels help`.")),
    }
}
