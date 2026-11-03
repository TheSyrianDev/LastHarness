//! `lh team ...`: roles, model types, departments, team settings, away mode, running a goal through a
//! department, and the worker list. Plain text, no styling. Everything goes through the core.

use std::io::Write;

use serde_json::{json, Value};

use crate::client::{Client, Result};

pub const HELP: &str = "lh team <command>
  roles                                   the roles
  role show <name>                        a role as its file text
  role set <name> <file>                  save a role from a file (a new version)
  role history <name> | role rollback <name> <version> | role delete <name>
  types                                   model types and the model each one points to
  type add <name> [model]                 a new type
  type set <name> <model|->               point a type at a model (- clears it)
  type fallbacks <name> <model>...        models to try in order when the first is missing
  type delete <name> [role=type:<t>|role=model:<m>]...   delete; say where roles using it go
  depts                                   the departments
  dept show <name> | dept set <name> <file> | dept history <name> | dept rollback <name> <version> | dept delete <name>
  settings                                parallel workers, autonomy, attempts, missing-model policy
  set parallel|attempts|autonomy|unavailable <value>
  plugin preview <folder>                 what a plugin folder holds (nothing is installed)
  plugin install <folder> [--replace <name>]...   install it; your own roles and departments stay unless named
  away on|off                             away mode; with no argument, what was decided while you were away
  workers                                 who is working on what
  worker pause|resume|stop <id>
  run <department> [--project <folder>] -- <goal>     give a goal to a department and follow it
  runs                                    goals given so far";

fn need<'a>(a: &'a [String], i: usize, what: &str) -> std::result::Result<&'a str, String> {
    a.get(i).map(String::as_str).ok_or_else(|| format!("missing {what}. Run `lh team help`."))
}

fn arr<'a>(v: &'a Value, k: &str) -> &'a [Value] {
    v[k].as_array().map(Vec::as_slice).unwrap_or(&[])
}

fn read(path: &str) -> Result<String> {
    std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))
}

fn history(v: &Value) -> String {
    arr(v, "versions").iter().map(|h| format!("v{}  {}  by {}  {}", h["version"], h["op"].as_str().unwrap_or(""), h["by"].as_str().unwrap_or(""), h["note"].as_str().unwrap_or(""))).collect::<Vec<_>>().join("\n")
}

fn type_line(t: &Value) -> String {
    let fb = arr(t, "fallbacks").iter().filter_map(Value::as_str).collect::<Vec<_>>();
    format!("{:<12} {}{}{}", t["name"].as_str().unwrap_or(""), t["model"].as_str().unwrap_or("(no model yet)"), if fb.is_empty() { String::new() } else { format!("  then {}", fb.join(", ")) }, if t["stock"] == true { "  [stock]" } else { "" })
}

fn settings_text(v: &Value) -> String {
    format!(
        "parallel workers: {}\nautonomy: {}\nattempts per step: {}\nwhen a model is missing: {}\naway: {}",
        v["parallel"], v["autonomy"].as_str().unwrap_or(""), v["attempts"], v["unavailable"].as_str().unwrap_or(""), if v["away_since"].is_null() { "no".to_string() } else { "yes".to_string() }
    )
}

/// One line for a run event, or `None` for events that need no line.
pub fn describe_event(e: &Value) -> Option<String> {
    let s = |k: &str| e[k].as_str().unwrap_or("");
    Some(match s("type") {
        "planning" => format!("{} is planning...", s("dept")),
        "plan" => {
            let steps: Vec<String> = arr(e, "steps").iter().map(|x| format!("  {}. {} -> {}{}", x["id"].as_str().unwrap_or(""), x["title"].as_str().unwrap_or(""), x["assignee"].as_str().unwrap_or(""), if arr(x, "depends_on").is_empty() { String::new() } else { format!("  (after {})", arr(x, "depends_on").iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", ")) })).collect();
            format!("Plan:\n{}", steps.join("\n"))
        }
        "plan_decided" => if e["approved"] == true { format!("Plan approved. {}", s("note")).trim().to_string() } else { format!("Plan not approved: {}", s("note")) },
        "step_started" => format!("[{}] {} starts \"{}\" on {} (attempt {})", s("step"), s("worker"), s("title"), s("model"), e["attempt"]),
        "step_checked" => format!("[{}] check: {}", s("step"), s("verdict")),
        "step_retry" => format!("[{}] {} ({}), trying again", s("step"), s("reason"), s("remedy")),
        "step_done" => format!("[{}] done: {}", s("step"), s("summary")),
        "step_failed" => format!("[{}] failed: {}", s("step"), s("report")),
        "step_parked" => format!("[{}] set aside for you: {}", s("step"), s("why")),
        "step_stopped" => format!("[{}] stopped", s("step")),
        "step_blocked" => format!("[{}] not run, it needed [{}]", s("step"), s("because")),
        "model_note" => s("note").to_string(),
        "model_unavailable" => format!("{}: {}", s("role"), s("message")),
        "milestone" => format!("Milestone reached: {}", s("name")),
        "away_denied" => format!("Said no while you were away: {} ({}) asked to {}", s("worker"), s("tool"), s("what")),
        "finished" => return None,
        _ => return None,
    })
}

fn outcome_text(e: &Value) -> String {
    let s = |k: &str| e[k].as_str().unwrap_or("");
    match s("status") {
        "done" => {
            let mut out = format!("\n{}", s("result"));
            let parked: Vec<&str> = arr(e, "parked").iter().filter_map(Value::as_str).collect();
            if !parked.is_empty() {
                out.push_str(&format!("\n\nSet aside for you:\n- {}", parked.join("\n- ")));
            }
            out
        }
        "failed" => format!("\nThe department could not finish.\n{}", s("report")),
        "rejected" => format!("\nNothing ran: {}", s("note")),
        _ => "\nStopped.".to_string(),
    }
}

async fn ask_line(prompt: String) -> String {
    tokio::task::spawn_blocking(move || {
        print!("{prompt} ");
        let _ = std::io::stdout().flush();
        let mut l = String::new();
        let _ = std::io::stdin().read_line(&mut l);
        l.trim().to_string()
    })
    .await
    .unwrap_or_default()
}

async fn run_goal(c: &Client, dept: &str, goal: &str, project: Option<String>) -> Result<String> {
    let r = c.call("POST", "/v1/team/runs", Some(json!({"department": dept, "goal": goal, "project_dir": project}))).await?;
    let id = r["id"].as_str().ok_or("the core sent no run id")?.to_string();
    let mut stream = c.events(&format!("/v1/team/runs/{id}/events")).await?;
    let mut last = String::new();
    while let Some(e) = stream.next().await? {
        let answer = |body: Value, q: String| {
            let path = format!("/v1/team/runs/{id}/answers/{q}");
            async move { c.call("POST", &path, Some(body)).await }
        };
        let q = e["id"].as_str().unwrap_or("").to_string();
        match e["type"].as_str().unwrap_or("") {
            "plan_needed" => {
                println!("{}", describe_event(&json!({"type": "plan", "steps": e["steps"]})).unwrap_or_default());
                let a = ask_line("Run this plan? [y]es / [n]o:".into()).await;
                if a.starts_with('y') {
                    answer(json!({"decision": "approve"}), q).await?;
                } else {
                    answer(json!({"decision": "reject", "note": "not approved"}), q).await?;
                }
            }
            "checkin_needed" => {
                println!("Milestone {} done:\n  {}", e["milestone"].as_str().unwrap_or(""), arr(&e, "summaries").iter().filter_map(Value::as_str).collect::<Vec<_>>().join("\n  "));
                let a = ask_line("Go on? [y]es / [n]o:".into()).await;
                answer(json!({"decision": if a.starts_with('y') { "continue" } else { "stop" }}), q).await?;
            }
            "ask" => {
                let a = ask_line(format!("{} wants to use {}: {}\n  {}\nAllow? [y]es / [n]o:", e["worker"].as_str().unwrap_or(""), e["tool"].as_str().unwrap_or(""), e["what"].as_str().unwrap_or(""), e["reason"].as_str().unwrap_or(""))).await;
                answer(json!({"answer": if a.starts_with('y') { "yes" } else { "no" }}), q).await?;
            }
            "plan" => println!("{}", describe_event(&e).unwrap_or_default()),
            "outcome" => last = outcome_text(&e),
            _ => {
                if let Some(l) = describe_event(&e) {
                    println!("{l}");
                }
            }
        }
    }
    Ok(last)
}

pub async fn run(c: &Client, args: &[String]) -> Result<String> {
    let cmd = args.first().map(String::as_str).unwrap_or("help");
    let rest = if args.is_empty() { &[][..] } else { &args[1..] };
    match cmd {
        "help" | "--help" | "-h" => Ok(HELP.into()),
        "roles" => {
            let v = c.call("GET", "/v1/team/roles", None).await?;
            Ok(arr(&v, "roles").iter().map(|r| match r["error"].as_str() {
                Some(e) => format!("{}  BROKEN: {e}", r["name"].as_str().unwrap_or("")),
                None => format!("{:<12} {:<22} {}", r["name"].as_str().unwrap_or(""), format!("{}:{}", r["target"]["kind"].as_str().unwrap_or(""), r["target"]["value"].as_str().unwrap_or("")), r["description"].as_str().unwrap_or("")),
            }).collect::<Vec<_>>().join("\n"))
        }
        "role" => {
            let (sub, name) = (need(rest, 0, "a role command")?, need(rest, 1, "a role name")?);
            match sub {
                "show" => Ok(c.call("GET", &format!("/v1/team/roles/{name}"), None).await?["text"].as_str().unwrap_or("").trim_end().to_string()),
                "set" => {
                    let v = c.call("PUT", &format!("/v1/team/roles/{name}"), Some(json!({"text": read(need(rest, 2, "a file")?)?}))).await?;
                    Ok(format!("Saved {name} as version {}.", v["version"]))
                }
                "history" => Ok(history(&c.call("GET", &format!("/v1/team/roles/{name}/history"), None).await?)),
                "rollback" => {
                    let ver: i64 = need(rest, 2, "a version")?.parse().map_err(|_| "the version must be a number".to_string())?;
                    let v = c.call("POST", &format!("/v1/team/roles/{name}/rollback"), Some(json!({"version": ver}))).await?;
                    Ok(format!("Rolled back {name}; it is now version {}.", v["version"]))
                }
                "delete" => c.call("DELETE", &format!("/v1/team/roles/{name}"), None).await.map(|_| format!("Deleted {name}.")),
                other => Err(format!("unknown role command {other}. Run `lh team help`.")),
            }
        }
        "types" => Ok(arr(&c.call("GET", "/v1/team/types", None).await?, "types").iter().map(type_line).collect::<Vec<_>>().join("\n")),
        "type" => {
            let (sub, name) = (need(rest, 0, "a type command")?, need(rest, 1, "a type name")?);
            match sub {
                "add" => Ok(format!("Added: {}", type_line(&c.call("POST", "/v1/team/types", Some(json!({"name": name, "model": rest.get(2)}))).await?))),
                "set" => {
                    let m = need(rest, 2, "a model name, or -")?;
                    Ok(type_line(&c.call("PUT", &format!("/v1/team/types/{name}"), Some(json!({"model": if m == "-" { Value::Null } else { json!(m) }}))).await?))
                }
                "fallbacks" => Ok(type_line(&c.call("PUT", &format!("/v1/team/types/{name}"), Some(json!({"fallbacks": &rest[2..]}))).await?)),
                "delete" => {
                    let mut re = serde_json::Map::new();
                    for a in &rest[2..] {
                        let (role, to) = a.split_once('=').ok_or(format!("{a}: write it as role=type:<name> or role=model:<name>"))?;
                        let (kind, val) = to.split_once(':').ok_or(format!("{a}: write it as role=type:<name> or role=model:<name>"))?;
                        re.insert(role.into(), json!({ kind: val }));
                    }
                    c.call("DELETE", &format!("/v1/team/types/{name}"), Some(json!({"reassign": re}))).await.map(|_| format!("Deleted type {name}."))
                }
                other => Err(format!("unknown type command {other}. Run `lh team help`.")),
            }
        }
        "depts" => {
            let v = c.call("GET", "/v1/team/departments", None).await?;
            Ok(arr(&v, "departments").iter().map(|d| match d["error"].as_str() {
                Some(e) => format!("{}  BROKEN: {e}", d["name"].as_str().unwrap_or("")),
                None => format!("{:<14} lead {:<10} members {}   {}", d["name"].as_str().unwrap_or(""), d["lead"].as_str().unwrap_or(""), arr(d, "members").iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" "), d["description"].as_str().unwrap_or("")),
            }).collect::<Vec<_>>().join("\n"))
        }
        "dept" => {
            let (sub, name) = (need(rest, 0, "a department command")?, need(rest, 1, "a department name")?);
            match sub {
                "show" => Ok(c.call("GET", &format!("/v1/team/departments/{name}"), None).await?["text"].as_str().unwrap_or("").trim_end().to_string()),
                "set" => {
                    let v = c.call("PUT", &format!("/v1/team/departments/{name}"), Some(json!({"text": read(need(rest, 2, "a file")?)?}))).await?;
                    Ok(format!("Saved {name} as version {}.", v["version"]))
                }
                "history" => Ok(history(&c.call("GET", &format!("/v1/team/departments/{name}/history"), None).await?)),
                "rollback" => {
                    let ver: i64 = need(rest, 2, "a version")?.parse().map_err(|_| "the version must be a number".to_string())?;
                    let v = c.call("POST", &format!("/v1/team/departments/{name}/rollback"), Some(json!({"version": ver}))).await?;
                    Ok(format!("Rolled back {name}; it is now version {}.", v["version"]))
                }
                "delete" => c.call("DELETE", &format!("/v1/team/departments/{name}"), None).await.map(|_| format!("Deleted {name}.")),
                other => Err(format!("unknown department command {other}. Run `lh team help`.")),
            }
        }
        "settings" => Ok(settings_text(&c.call("GET", "/v1/team/settings", None).await?)),
        "set" => {
            let (k, v) = (need(rest, 0, "a setting")?, need(rest, 1, "a value")?);
            let val = match k {
                "parallel" | "attempts" => json!(v.parse::<u64>().map_err(|_| format!("{k} needs a number"))?),
                "autonomy" | "unavailable" => json!(v),
                other => return Err(format!("unknown setting {other}. Run `lh team help`.")),
            };
            Ok(settings_text(&c.call("PUT", "/v1/team/settings", Some(json!({ k: val }))).await?))
        }
        "plugin" => {
            let (sub, folder) = (need(rest, 0, "preview or install")?, need(rest, 1, "a folder")?);
            let folder = std::fs::canonicalize(folder).map_err(|e| format!("{folder}: {e}"))?.to_string_lossy().to_string();
            let list = |v: &Value, k: &str| arr(v, k).iter().map(|i| format!("  {}{}  {}", i["name"].as_str().unwrap_or(""), if i["exists"] == true { " (you already have one)" } else { "" }, i["description"].as_str().unwrap_or(""))).collect::<Vec<_>>().join("\n");
            match sub {
                "preview" => {
                    let v = c.call("POST", "/v1/team/plugins/preview", Some(json!({"path": folder}))).await?;
                    let mut out = format!("{}  {}\nroles:\n{}\ndepartments:\n{}\nthemes: {}", v["name"].as_str().unwrap_or(""), v["description"].as_str().unwrap_or(""), list(&v, "roles"), list(&v, "departments"), arr(&v, "themes").iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", "));
                    for p in arr(&v, "problems").iter().filter_map(Value::as_str) {
                        out.push_str(&format!("\ncannot use {p}"));
                    }
                    Ok(out)
                }
                "install" => {
                    let replace: Vec<&String> = rest.iter().enumerate().filter(|(i, a)| *i > 0 && rest[*i - 1] == "--replace" && !a.starts_with("--")).map(|(_, a)| a).collect();
                    let v = c.call("POST", "/v1/team/plugins/install", Some(json!({"path": folder, "accept": true, "overwrite": replace}))).await?;
                    let mut out: Vec<String> = arr(&v, "installed").iter().filter_map(Value::as_str).map(|s| format!("installed {s}")).collect();
                    out.extend(arr(&v, "skipped").iter().filter_map(Value::as_str).map(|s| format!("skipped {s}")));
                    Ok(out.join("\n"))
                }
                other => Err(format!("unknown plugin command {other}. Run `lh team help`.")),
            }
        }
        "away" => {
            let v = match rest.first().map(String::as_str) {
                Some("on") => c.call("POST", "/v1/team/away", Some(json!({"on": true}))).await?,
                Some("off") => c.call("POST", "/v1/team/away", Some(json!({"on": false}))).await?,
                None => c.call("GET", "/v1/team/away", None).await?,
                Some(o) => return Err(format!("away takes on or off, not {o}")),
            };
            let mut out = format!("Away mode is {}.", if v["away"] == true { "on" } else { "off" });
            for d in arr(&v, "decisions") {
                out.push_str(&format!("\n  decided alone in {}: {}", d["dept"].as_str().unwrap_or(""), d["decision"].as_str().unwrap_or("")));
            }
            for p in arr(&v, "parked").iter().filter_map(Value::as_str) {
                out.push_str(&format!("\n  set aside for you: {p}"));
            }
            Ok(out)
        }
        "workers" => {
            let v = c.call("GET", "/v1/team/workers", None).await?;
            let w = arr(&v, "workers");
            Ok(if w.is_empty() { "Nobody is working right now.".into() } else { w.iter().map(|x| format!("{:<4} {:<9} {:<10} {:<10} {}  [{}]", x["id"].as_str().unwrap_or(""), x["status"].as_str().unwrap_or(""), x["dept"].as_str().unwrap_or(""), x["role"].as_str().unwrap_or(""), x["step"].as_str().unwrap_or(""), x["model"].as_str().unwrap_or(""))).collect::<Vec<_>>().join("\n") })
        }
        "worker" => {
            let (action, id) = (need(rest, 0, "pause, resume or stop")?, need(rest, 1, "a worker id")?);
            c.call("POST", &format!("/v1/team/workers/{id}/{action}"), None).await.map(|_| format!("Worker {id}: {action} done."))
        }
        "run" => {
            let dept = need(rest, 0, "a department")?;
            let mut project = None;
            if let Some(i) = rest.iter().position(|a| a == "--project") {
                project = Some(need(rest, i + 1, "a folder after --project")?.to_string());
            }
            let goal = rest.iter().position(|a| a == "--").map(|i| rest[i + 1..].join(" ")).unwrap_or_default();
            if goal.is_empty() {
                return Err("give the goal after --, for example: lh team run web -- make a landing page".into());
            }
            run_goal(c, dept, &goal, project).await
        }
        "runs" => {
            let v = c.call("GET", "/v1/team/runs", None).await?;
            Ok(arr(&v, "runs").iter().map(|r| format!("{}  {:<12} {}  {}", r["id"].as_str().unwrap_or(""), r["department"].as_str().unwrap_or(""), if r["finished"] == true { "finished" } else { "running " }, r["goal"].as_str().unwrap_or(""))).collect::<Vec<_>>().join("\n"))
        }
        other => Err(format!("unknown command {other}. Run `lh team help`.")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_read_as_plain_lines() {
        let e = json!({"type":"step_done","step":"2","summary":"wrote it"});
        assert_eq!(describe_event(&e).unwrap(), "[2] done: wrote it");
        let plan = json!({"type":"plan","steps":[{"id":"1","title":"A","assignee":"coder","depends_on":[]},{"id":"2","title":"B","assignee":"writer","depends_on":["1"]}]});
        assert_eq!(describe_event(&plan).unwrap(), "Plan:\n  1. A -> coder\n  2. B -> writer  (after 1)");
        assert!(describe_event(&json!({"type":"finished"})).is_none());
        assert!(outcome_text(&json!({"status":"done","result":"All done.","parked":["X: nobody there"]})).contains("Set aside for you:\n- X: nobody there"));
    }
}
