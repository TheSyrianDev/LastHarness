//! `lh memory ...`: look at, search, edit and clean up what is remembered, without opening a chat.
//! Everything goes through the core's API; this file only turns the replies into readable text.

use serde_json::{json, Value};

use crate::client::{Client, Result};

pub const HELP: &str = "lh memory <command>
  list [preferences|facts|projects] [--archived|--all]   pages, one line each
  search <words> [--archived]                            by words and, if set up, by meaning
  show <id>                                              the whole page and its history
  add <layer> <title> -- <text>                          save a page yourself
  archive|restore|delete <id>                            archive keeps it searchable; delete removes it for good
  pin|unpin <id>                                         pinned pages come first in the always-on summary
  sensitive|normal <id>                                  where a page may be sent: sensitive stays on this machine
  proposals                                              suggestions waiting for a decision
  accept|reject <id>                                     decide a suggestion
  scan                                                   pick up pages you edited by hand in the memory folder
  maintenance [--contradictions MODEL]                   look for duplicates, broken links, stale pages
  sent                                                   which notes went into which requests
  conversations <words>                                  search past conversations
  export|import <absolute folder>                        copy memory out as folders, or in";

fn flags(p: &Value) -> String {
    let mut f = vec![];
    if p["privacy"] == "sensitive" {
        f.push("sensitive");
    }
    if p["pinned"] == true {
        f.push("pinned");
    }
    if p["archived"] == true {
        f.push("archived");
    }
    if f.is_empty() { String::new() } else { format!("  [{}]", f.join(", ")) }
}

fn line(p: &Value) -> String {
    format!("{}  {:<11} {}{}", p["id"].as_str().unwrap_or(""), p["layer"].as_str().unwrap_or(""), p["title"].as_str().unwrap_or(""), flags(p))
}

fn lines(v: &[Value], empty: &str) -> String {
    if v.is_empty() { empty.to_string() } else { v.iter().map(line).collect::<Vec<_>>().join("\n") }
}

fn arr<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v[key].as_array().map(Vec::as_slice).unwrap_or(&[])
}

fn need<'a>(args: &'a [String], i: usize, what: &str) -> std::result::Result<&'a str, String> {
    args.get(i).map(String::as_str).ok_or_else(|| format!("missing {what}. Run `lh memory help`."))
}

pub async fn run(c: &Client, args: &[String]) -> Result<String> {
    let cmd = args.first().map(String::as_str).unwrap_or("help");
    let rest = if args.is_empty() { &[][..] } else { &args[1..] };
    let id_call = |m: &'static str, suffix: &'static str, body: Option<Value>| {
        let id = need(rest, 0, "page id").map(str::to_string);
        async move { c.call(m, &format!("/v1/memory/pages/{}{suffix}", id?), body).await }
    };
    match cmd {
        "help" | "--help" | "-h" => Ok(HELP.into()),
        "list" => {
            let layer = rest.iter().find(|a| matches!(a.as_str(), "preferences" | "facts" | "projects"));
            let state = if rest.iter().any(|a| a == "--all") { "any" } else if rest.iter().any(|a| a == "--archived") { "archived" } else { "live" };
            let q = format!("/v1/memory/pages?state={state}{}", layer.map(|l| format!("&layer={l}")).unwrap_or_default());
            Ok(lines(arr(&c.call("GET", &q, None).await?, "pages"), "Nothing is remembered yet."))
        }
        "search" => {
            let words: Vec<&str> = rest.iter().filter(|a| !a.starts_with("--")).map(String::as_str).collect();
            if words.is_empty() {
                return Err("what should I search for?".into());
            }
            let state = if rest.iter().any(|a| a == "--archived") { "archived" } else { "live" };
            let q = format!("/v1/memory/search?state={state}&q={}", urlencode(&words.join(" ")));
            Ok(lines(arr(&c.call("GET", &q, None).await?, "results"), "No matches."))
        }
        "show" => {
            let v = id_call("GET", "", None).await?;
            let p = &v["page"];
            let mut out = format!("{}\n\n{}\n", line(p), p["body"].as_str().unwrap_or(""));
            if !p["gist"].as_str().unwrap_or("").is_empty() {
                out.push_str(&format!("\nSummary line: {}\n", p["gist"].as_str().unwrap_or("")));
            }
            out.push_str("\nHistory:\n");
            for h in arr(&v, "history") {
                out.push_str(&format!("  v{}  {}  by {}\n", h["version"], h["op"].as_str().unwrap_or(""), h["by"].as_str().unwrap_or("")));
            }
            Ok(out.trim_end().into())
        }
        "add" => {
            let layer = need(rest, 0, "layer")?;
            let title = need(rest, 1, "title")?;
            let text = rest.iter().position(|a| a == "--").map(|i| rest[i + 1..].join(" ")).unwrap_or_default();
            if text.trim().is_empty() {
                return Err("give the text after `--`: lh memory add facts \"Home\" -- Lives in Berlin.".into());
            }
            let v = c.call("POST", "/v1/memory/pages", Some(json!({"layer": layer, "title": title, "text": text}))).await?;
            Ok(format!("Saved: {}", line(&v)))
        }
        "archive" => Ok(format!("Archived: {}", line(&id_call("POST", "/archive", Some(json!({}))).await?))),
        "restore" => Ok(format!("Restored: {}", line(&id_call("POST", "/restore", Some(json!({}))).await?))),
        "pin" => Ok(format!("Pinned: {}", line(&id_call("POST", "/pin", Some(json!({"pinned": true}))).await?))),
        "unpin" => Ok(format!("Unpinned: {}", line(&id_call("POST", "/pin", Some(json!({"pinned": false}))).await?))),
        "sensitive" => Ok(format!("Marked sensitive (stays on this machine): {}", line(&id_call("POST", "/privacy", Some(json!({"privacy": "sensitive"}))).await?))),
        "normal" => Ok(format!("Marked normal (may be shared with cloud models): {}", line(&id_call("POST", "/privacy", Some(json!({"privacy": "normal"}))).await?))),
        "delete" => {
            id_call("DELETE", "", None).await?;
            Ok("Deleted for good, including its older versions.".into())
        }
        "proposals" => {
            let v = c.call("GET", "/v1/memory/proposals", None).await?;
            let list = arr(&v, "proposals");
            if list.is_empty() {
                return Ok("No suggestions waiting.".into());
            }
            Ok(list
                .iter()
                .map(|p| format!("{}  {} {}: {}{}", p["id"], p["kind"].as_str().unwrap_or(""), p["layer"].as_str().unwrap_or(""), p["title"].as_str().unwrap_or(""), if p["privacy"] == "sensitive" { "  [sensitive]" } else { "" }))
                .collect::<Vec<_>>()
                .join("\n"))
        }
        "accept" => {
            let id = need(rest, 0, "suggestion number")?;
            let v = c.call("POST", &format!("/v1/memory/proposals/{id}/accept"), Some(json!({}))).await?;
            Ok(format!("Remembered: {}", line(&v)))
        }
        "reject" => {
            c.call("POST", &format!("/v1/memory/proposals/{}/reject", need(rest, 0, "suggestion number")?), Some(json!({}))).await?;
            Ok("Dropped.".into())
        }
        "scan" => {
            let v = c.call("POST", "/v1/memory/scan", Some(json!({}))).await?;
            Ok(format!("New files: {}, edited: {}, removed (archived): {}", arr(&v, "created").len(), arr(&v, "edited").len(), arr(&v, "archived").len()))
        }
        "maintenance" => {
            let body = match rest.iter().position(|a| a == "--contradictions") {
                Some(i) => json!({"contradictions": true, "model": need(rest, i + 1, "model name")?}),
                None => json!({}),
            };
            let v = c.call("POST", "/v1/memory/maintenance", Some(body)).await?;
            let f = arr(&v, "findings");
            let mut out = if f.is_empty() { "Nothing to tidy.".to_string() } else { f.iter().map(|x| format!("{}: {} ({})", x["kind"].as_str().unwrap_or(""), x["detail"].as_str().unwrap_or(""), x["pages"].as_array().map(|p| p.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", ")).unwrap_or_default())).collect::<Vec<_>>().join("\n") };
            out.push_str(&format!("\n{} suggestion(s) waiting. Nothing was changed.", v["pending_suggestions"]));
            Ok(out)
        }
        "sent" => {
            let v = c.call("GET", "/v1/memory/sent", None).await?;
            let rows = arr(&v, "sent");
            if rows.is_empty() {
                return Ok("No memory has been sent with a request yet.".into());
            }
            Ok(rows
                .iter()
                .map(|r| {
                    let d = &r["detail"];
                    format!("session {}  {}  to a {} destination: {}", r["session"].as_str().unwrap_or(""), d["with"].as_str().unwrap_or(""), d["trust"].as_str().unwrap_or(""), d["pages"].as_array().map(|p| p.iter().map(|x| format!("{} v{}", x["page"].as_str().unwrap_or(""), x["version"])).collect::<Vec<_>>().join(", ")).unwrap_or_default())
                })
                .collect::<Vec<_>>()
                .join("\n"))
        }
        "conversations" => {
            let q = rest.join(" ");
            let v = c.call("GET", &format!("/v1/memory/conversations?q={}", urlencode(&q)), None).await?;
            let m = arr(&v, "messages");
            Ok(if m.is_empty() { "No matches.".into() } else { m.iter().map(|x| format!("{}  {}: {}", x["conversation"].as_str().unwrap_or(""), x["role"].as_str().unwrap_or(""), x["text"].as_str().unwrap_or("").chars().take(120).collect::<String>())).collect::<Vec<_>>().join("\n") })
        }
        "export" => {
            c.call("POST", "/v1/memory/export", Some(json!({"path": need(rest, 0, "folder")?}))).await?;
            Ok("Exported pages, archive and conversations as Markdown. The export includes sensitive pages; keep it safe.".into())
        }
        "import" => {
            let v = c.call("POST", "/v1/memory/import", Some(json!({"path": need(rest, 0, "folder")?}))).await?;
            Ok(format!("Imported {} page(s); {} were already here.", v["imported"], v["already_present"]))
        }
        other => Err(format!("unknown memory command `{other}`. Run `lh memory help`.")),
    }
}

fn urlencode(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queries_are_encoded_including_arabic() {
        assert_eq!(urlencode("a b&c"), "a%20b%26c");
        assert_eq!(urlencode("برلين"), "%D8%A8%D8%B1%D9%84%D9%8A%D9%86");
    }
}
