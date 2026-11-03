//! Tools and MCP: the servers the assistant may use, and the folders that may bring their own.

use serde_json::{json, Value};

use super::{s, split_command};

#[derive(Clone, Debug, PartialEq)]
pub struct Tool {
    pub name: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Server {
    pub name: String,
    /// "local" (a program on this computer) or "remote" (a link).
    pub kind: String,
    /// The exact command or link, as the core will run it.
    pub runs: String,
    pub approved: bool,
    pub error: Option<String>,
    pub tools: Vec<Tool>,
}

pub fn servers(v: &Value) -> Vec<Server> {
    v["servers"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|e| Server {
            name: s(e, "name"),
            kind: s(e, "kind"),
            runs: s(e, "runs"),
            approved: e["approved"] == true,
            error: e["error"].as_str().map(String::from),
            tools: e["tools"].as_array().into_iter().flatten().map(|t| Tool { name: s(t, "name"), description: s(t, "description") }).collect(),
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
pub struct Folder {
    pub folder: String,
    /// `trust`, `own` or `changed`.
    pub decision: String,
    pub exists: bool,
    /// What the folder asks to run: name and exact command.
    pub asks: Vec<(String, String)>,
}

pub fn folders(v: &Value) -> Vec<Folder> {
    v["folders"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|f| Folder {
            folder: s(f, "folder"),
            decision: s(f, "decision"),
            exists: f["exists"] != false,
            asks: f["mcp"].as_array().into_iter().flatten().map(|m| (s(m, "name"), s(m, "runs"))).collect(),
        })
        .collect()
}

/// What a folder decision means, in words.
pub fn decision_words(d: &str) -> &'static str {
    match d {
        "trust" => "Trusted: the servers it lists may run.",
        "own" => "Only your own servers are used here.",
        "changed" => "Its server list changed since you trusted it. You are asked again.",
        _ => "No decision yet.",
    }
}

/// The sentence after a server is added, with the exact command or link.
pub fn after_add(v: &Value, name: &str) -> String {
    format!("Added {name}. It will run: {}. Nothing runs until you approve it.", s(v, "runs"))
}

/// The body to add a server. `link` is true for a remote server. A local server takes the whole command as typed.
pub fn add_body(name: &str, link: bool, command_or_url: &str, bearer_env: &str) -> Result<Value, String> {
    let name = name.trim();
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err("Give it a name with letters, digits, - or _.".into());
    }
    let transport = if link {
        let url = command_or_url.trim();
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return Err("A link starts with http:// or https://.".into());
        }
        let env = bearer_env.trim();
        json!({"kind": "http", "url": url, "bearer_env": if env.is_empty() { Value::Null } else { json!(env) }})
    } else {
        let mut words = split_command(command_or_url)?;
        if words.is_empty() {
            return Err("Type the command that starts the server.".into());
        }
        let program = words.remove(0);
        json!({"kind": "stdio", "command": program, "args": words})
    };
    Ok(json!({"name": name, "transport": transport}))
}

/// The exact command a local server will run, written the way it was typed, for showing before it is added.
pub fn preview(link: bool, command_or_url: &str) -> String {
    if link {
        command_or_url.trim().to_string()
    } else {
        split_command(command_or_url).map(|w| w.iter().map(|x| if x.is_empty() || x.contains(char::is_whitespace) { format!("'{x}'") } else { x.clone() }).collect::<Vec<_>>().join(" ")).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn servers_and_their_tools_are_read() {
        let v = json!({"servers": [
            {"name": "a", "kind": "local", "runs": "npx x", "approved": true, "tools": [{"name": "mcp__a__echo", "description": "Echo"}]},
            {"name": "b", "kind": "remote", "runs": "https://x", "approved": true, "error": "cannot connect"},
            {"name": "c", "kind": "local", "runs": "python3 s.py", "approved": false}
        ]});
        let l = servers(&v);
        assert_eq!(l.len(), 3);
        assert_eq!(l[0].tools, vec![Tool { name: "mcp__a__echo".into(), description: "Echo".into() }]);
        assert_eq!(l[1].error.as_deref(), Some("cannot connect"));
        assert!(!l[2].approved && l[2].tools.is_empty());
        assert!(servers(&Value::Null).is_empty());
    }

    #[test]
    fn folders_are_read_with_what_they_ask_for() {
        let v = json!({"folders": [{"folder": "/p", "decision": "trust", "exists": false, "mcp": [{"name": "t", "runs": "python3 m.py"}]}]});
        let f = folders(&v);
        assert_eq!(f[0].asks, vec![("t".to_string(), "python3 m.py".to_string())]);
        assert!(!f[0].exists);
        assert!(decision_words("changed").contains("asked again"));
    }

    #[test]
    fn adding_by_command_sends_the_program_and_its_arguments() {
        let b = add_body("files", false, "npx -y 'my pkg' --root /tmp", "").unwrap();
        assert_eq!(b["transport"], json!({"kind": "stdio", "command": "npx", "args": ["-y", "my pkg", "--root", "/tmp"]}));
        assert_eq!(b["name"], "files");
    }

    #[test]
    fn adding_by_link_sends_the_link_and_the_key_variable() {
        let b = add_body("web", true, " https://mcp.example.org/x ", " MY_KEY ").unwrap();
        assert_eq!(b["transport"], json!({"kind": "http", "url": "https://mcp.example.org/x", "bearer_env": "MY_KEY"}));
        assert_eq!(add_body("web", true, "https://x", "").unwrap()["transport"]["bearer_env"], Value::Null);
    }

    #[test]
    fn bad_input_is_refused_with_a_sentence() {
        assert!(add_body("", false, "x", "").unwrap_err().contains("name"));
        assert!(add_body("a b", false, "x", "").is_err());
        assert!(add_body("a", false, "  ", "").unwrap_err().contains("command"));
        assert!(add_body("a", true, "ftp://x", "").unwrap_err().contains("http"));
        assert!(add_body("a", false, "x 'y", "").is_err());
    }

    #[test]
    fn the_preview_shows_the_command_with_quotes_where_needed() {
        assert_eq!(preview(false, "npx   -y 'my pkg'"), "npx -y 'my pkg'");
        assert_eq!(preview(true, " https://x "), "https://x");
    }

    #[test]
    fn the_add_note_names_the_exact_command() {
        assert!(after_add(&json!({"runs": "python3 s.py"}), "s").contains("python3 s.py"));
    }
}
