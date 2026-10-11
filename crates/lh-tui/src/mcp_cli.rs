//! `lh mcp ...` and `lh trust ...`: MCP servers and which folders may bring their own.

use serde_json::{json, Value};

use crate::client::{Client, Result};

pub const HELP: &str = "lh mcp <command>
  list                                    servers, whether you approved them, and their tools
  add <name> --command <program> [args...]   a local server (the exact command is shown before you approve)
  add <name> --url <link> [--bearer-env <VAR>]   a remote server; the key is read from the environment variable
  approve <name>                          let it run (this starts it and lists its tools)
  remove <name>
lh trust <folder> [trust|own|forget]      what a project folder may bring: trust it, or use only your own servers
                                          with no decision given, shows what the folder asks for";

fn need<'a>(a: &'a [String], i: usize, what: &str) -> std::result::Result<&'a str, String> {
    a.get(i).map(String::as_str).ok_or_else(|| format!("missing {what}. Run `lh mcp help`."))
}

pub async fn run(c: &Client, args: &[String]) -> Result<String> {
    let rest = if args.is_empty() { &[][..] } else { &args[1..] };
    match args.first().map(String::as_str).unwrap_or("help") {
        "help" | "--help" | "-h" => Ok(HELP.into()),
        "list" => {
            let v = c.call("GET", "/v1/mcp/servers", None).await?;
            let s = v["servers"].as_array().cloned().unwrap_or_default();
            if s.is_empty() {
                return Ok("No MCP servers.".into());
            }
            Ok(s.iter().map(|e| {
                let mut l = format!("{}  [{}, {}]  {}", e["name"].as_str().unwrap_or(""), e["kind"].as_str().unwrap_or(""), if e["approved"] == true { "approved" } else { "waiting for your approval" }, e["runs"].as_str().unwrap_or(""));
                if let Some(err) = e["error"].as_str() {
                    l.push_str(&format!("\n    cannot start: {err}"));
                }
                for t in e["tools"].as_array().into_iter().flatten() {
                    l.push_str(&format!("\n    {}  {}", t["name"].as_str().unwrap_or(""), t["description"].as_str().unwrap_or("")));
                }
                l
            }).collect::<Vec<_>>().join("\n"))
        }
        "add" => {
            let name = need(rest, 0, "a name")?;
            let transport = match rest.get(1).map(String::as_str) {
                Some("--command") => json!({"kind": "stdio", "command": need(rest, 2, "a program")?, "args": &rest[3..]}),
                Some("--url") => json!({"kind": "http", "url": need(rest, 2, "a link")?, "bearer_env": rest.iter().position(|a| a == "--bearer-env").and_then(|i| rest.get(i + 1))}),
                _ => return Err("give --command <program> or --url <link>. Run `lh mcp help`.".into()),
            };
            let v = c.call("POST", "/v1/mcp/servers", Some(json!({"name": name, "transport": transport}))).await?;
            Ok(format!("Added {name}. It will run:\n  {}\nNothing runs until you approve it: lh mcp approve {name}", v["runs"].as_str().unwrap_or("")))
        }
        "approve" => {
            let name = need(rest, 0, "a name")?;
            let v = c.call("POST", &format!("/v1/mcp/servers/{name}/approve"), None).await?;
            Ok(format!("Approved {name}. Tools: {}", v["tools"].as_array().map(|t| t.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", ")).unwrap_or_default()))
        }
        "remove" => c.call("DELETE", &format!("/v1/mcp/servers/{}", need(rest, 0, "a name")?), None).await.map(|_| "Removed.".into()),
        other => Err(format!("unknown command {other}. Run `lh mcp help`.")),
    }
}

pub async fn trust(c: &Client, args: &[String]) -> Result<String> {
    let folder = need(args, 0, "a folder")?;
    match args.get(1).map(String::as_str) {
        None => {
            let v = c.call("GET", &format!("/v1/trust?folder={folder}"), None).await?;
            let mut out = format!("{}\ndecision: {}", v["folder"].as_str().unwrap_or(folder), v["decision"].as_str().unwrap_or("none yet"));
            for m in v["mcp"].as_array().into_iter().flatten() {
                out.push_str(&format!("\n  asks to run {}: {}", m["name"].as_str().unwrap_or(""), m["runs"].as_str().unwrap_or("")));
            }
            Ok(out)
        }
        Some(d @ ("trust" | "own" | "forget")) => {
            let v = c.call("POST", "/v1/trust", Some(json!({"folder": folder, "decision": d}))).await?;
            Ok(format!("{}: {}", v["folder"].as_str().unwrap_or(folder), v["decision"].as_str().unwrap_or("forgotten")))
        }
        Some(o) => Err(format!("{o}: say trust, own or forget")),
    }
}
