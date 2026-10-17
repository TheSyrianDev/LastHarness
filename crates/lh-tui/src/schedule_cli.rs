//! `lh schedule`: tasks the core runs by itself at set times.

use serde_json::json;

use crate::client::{Client, Result};

pub const HELP: &str = "lh schedule                                   the tasks, when they run next and how the last run went
lh schedule add \"<when>\" \"<what to do>\" [--model NAME] [--name NAME]
                                              runs in the current folder; <when> is \"every 30m\", \"every 2h\", \"every 1d\" or \"daily 08:00\"
lh schedule on <id> | off <id>                turn a task on or off
lh schedule remove <id>                       delete a task
Nobody watches a task, so permission questions are answered no. Each run is a stored chat you can open.";

fn when(t: Option<i64>) -> String {
    let Some(t) = t else { return "never".into() };
    let ago = t - std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let (n, past) = (ago.abs(), ago < 0);
    let span = if n < 90 { format!("{n} s") } else if n < 5400 { format!("{} min", n / 60) } else if n < 129_600 { format!("{} h", n / 3600) } else { format!("{} days", n / 86400) };
    if past { format!("{span} ago") } else { format!("in {span}") }
}

pub async fn run(c: &Client, args: &[String], model: Option<String>) -> Result<String> {
    match args.first().map(String::as_str) {
        None => {
            let v = c.call("GET", "/v1/schedules", None).await?;
            let list = v["schedules"].as_array().cloned().unwrap_or_default();
            if list.is_empty() {
                return Ok("No scheduled tasks. Add one with: lh schedule add \"daily 08:00\" \"what to do\" --model NAME".into());
            }
            Ok(list
                .iter()
                .map(|s| {
                    let state = if s["enabled"] == true { format!("next {}", when(s["next_run"].as_i64())) } else { "off".into() };
                    let last = s["last_result"].as_str().filter(|r| !r.is_empty()).map(|r| format!(", last run {}: {r}", when(s["last_run"].as_i64()))).unwrap_or_default();
                    format!("{:>3}  {}  ({}, {state}{last})\n     in {} with {}", s["id"], s["name"].as_str().unwrap_or(""), s["when"].as_str().unwrap_or(""), s["folder"].as_str().unwrap_or(""), s["model"].as_str().unwrap_or(""))
                })
                .collect::<Vec<_>>()
                .join("\n"))
        }
        Some("add") => {
            let (mut pos, mut name, mut model) = (vec![], String::new(), model);
            let mut it = args[1..].iter();
            while let Some(a) = it.next() {
                match a.as_str() {
                    "--model" => model = it.next().cloned(),
                    "--name" => name = it.next().cloned().unwrap_or_default(),
                    _ => pos.push(a.clone()),
                }
            }
            let [w, what] = pos.as_slice() else { return Err("write it as: lh schedule add \"<when>\" \"<what to do>\" --model NAME".into()) };
            let model = model.or_else(|| std::env::var("LH_MODEL").ok()).ok_or("choose a model with --model NAME")?;
            let folder = std::env::current_dir().map_err(|e| e.to_string())?;
            let v = c.call("POST", "/v1/schedules", Some(json!({"name": name, "prompt": what, "folder": folder, "model": model, "when": w}))).await?;
            Ok(format!("Added task {} \"{}\": {}, first run {}.", v["id"], v["name"].as_str().unwrap_or(""), v["when"].as_str().unwrap_or(""), when(v["next_run"].as_i64())))
        }
        Some(op @ ("on" | "off")) => {
            let id = args.get(1).ok_or("which task? Run `lh schedule` to see them.")?;
            let v = c.call("POST", &format!("/v1/schedules/{id}/enabled"), Some(json!({"enabled": op == "on"}))).await?;
            Ok(format!("Task {id} is {op}{}.", if op == "on" { format!(", next run {}", when(v["next_run"].as_i64())) } else { String::new() }))
        }
        Some("remove") => {
            let id = args.get(1).ok_or("which task? Run `lh schedule` to see them.")?;
            c.call("DELETE", &format!("/v1/schedules/{id}"), None).await.map(|_| format!("Removed task {id}."))
        }
        Some(o) => Err(format!("unknown command {o}. Run `lh schedule help`.")),
    }
}
