//! Hooks: commands the user sets up to run before and after a tool call, such as a formatter after every file
//! write or a check that blocks commands in some folder.
//!
//! They live in `hooks.toml` beside the settings file:
//!
//! ```toml
//! [[before]]
//! tools = ["bash"]                 # leave out for every tool
//! run = "my-check"                 # run by `sh -c` in the chat's folder
//!
//! [[after]]
//! tools = ["write"]
//! run = "cargo fmt"
//! timeout_secs = 30                # default 30
//! ```
//!
//! A hook gets one line of JSON on standard input: `{"event", "tool", "arguments", "folder"}` and, after a tool,
//! `"result"`. A `before` hook that exits with an error stops the tool, and what it printed goes to the model as
//! the reason. An `after` hook's output is added to the tool's result. Hooks run only for tool calls the
//! permission rules let through, with the user's own rights, like any command the user starts.

use std::path::Path;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::io::AsyncWriteExt;

/// A hook from what was typed: tools as a comma list (empty for every tool), the command, and seconds.
pub fn from_form(tools: &str, run: &str, secs: &str) -> Result<Hook, String> {
    let run = run.trim();
    if run.is_empty() {
        return Err("Write the command to run.".into());
    }
    let secs: u64 = if secs.trim().is_empty() { 30 } else { secs.trim().parse().map_err(|_| "The time limit is a whole number of seconds.".to_string())? };
    if secs == 0 || secs > 600 {
        return Err("The time limit is 1 to 600 seconds.".into());
    }
    let tools = tools.split(',').map(str::trim).filter(|t| !t.is_empty()).map(String::from).collect();
    Ok(Hook { tools, run: run.to_string(), timeout: Duration::from_secs(secs) })
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Hooks {
    pub before: Vec<Hook>,
    pub after: Vec<Hook>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Hook {
    /// Tool names this hook is for; empty means every tool.
    pub tools: Vec<String>,
    pub run: String,
    pub timeout: Duration,
}

/// What a hook said.
#[derive(Clone, Debug, PartialEq)]
pub struct HookOutput {
    pub run: String,
    pub ok: bool,
    /// Standard output and error together, trimmed and cut to a few thousand characters.
    pub text: String,
}

const MAX_OUTPUT: usize = 4000;

impl Hooks {
    /// Read `hooks.toml`. A missing file means no hooks. A file that cannot be read is an error to show the user,
    /// so a broken file is never silently ignored.
    pub fn load(path: &Path) -> Result<Hooks, String> {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Hooks::default()),
            Err(e) => return Err(format!("{}: {e}", path.display())),
        };
        Self::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn parse(text: &str) -> Result<Hooks, String> {
        let v: toml::Table = text.parse().map_err(|e: toml::de::Error| e.message().to_string())?;
        let list = |key: &str| -> Result<Vec<Hook>, String> {
            let Some(items) = v.get(key) else { return Ok(vec![]) };
            let items = items.as_array().ok_or(format!("`{key}` must be a list of [[{key}]] tables"))?;
            items
                .iter()
                .enumerate()
                .map(|(i, h)| {
                    let t = h.as_table().ok_or(format!("{key} {}: not a table", i + 1))?;
                    let run = t.get("run").and_then(|r| r.as_str()).filter(|r| !r.trim().is_empty()).ok_or(format!("{key} {}: `run` is missing", i + 1))?;
                    let tools = match t.get("tools") {
                        None => vec![],
                        Some(l) => l.as_array().ok_or(format!("{key} {}: `tools` must be a list of names", i + 1))?.iter().filter_map(|x| x.as_str().map(str::to_string)).collect(),
                    };
                    let secs = t.get("timeout_secs").and_then(|s| s.as_integer()).unwrap_or(30).clamp(1, 600) as u64;
                    Ok(Hook { tools, run: run.to_string(), timeout: Duration::from_secs(secs) })
                })
                .collect()
        };
        Ok(Hooks { before: list("before")?, after: list("after")? })
    }

    pub fn is_empty(&self) -> bool {
        self.before.is_empty() && self.after.is_empty()
    }

    /// The file text for these hooks, in the same form the user would write by hand.
    pub fn to_toml(&self) -> String {
        let mut out = String::from("# Commands run before or after a tool call. Edited in Settings, Hooks.\n");
        for (name, list) in [("before", &self.before), ("after", &self.after)] {
            for k in list {
                out.push_str(&format!("\n[[{name}]]\n"));
                if !k.tools.is_empty() {
                    out.push_str(&format!("tools = {}\n", toml::Value::Array(k.tools.iter().map(|t| toml::Value::String(t.clone())).collect())));
                }
                out.push_str(&format!("run = {}\n", toml::Value::String(k.run.clone())));
                out.push_str(&format!("timeout_secs = {}\n", k.timeout.as_secs()));
            }
        }
        out
    }

    /// Check the new file reads back the same, then write it.
    pub fn save(&self, p: &Path) -> Result<(), String> {
        let text = self.to_toml();
        if Hooks::parse(&text)? != *self {
            return Err("the hooks could not be written the way they read back".into());
        }
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
        }
        std::fs::write(p, text).map_err(|e| format!("{}: {e}", p.display()))
    }

    /// Run the `before` hooks for this tool. `Err` holds the words for the model when one of them said no.
    pub async fn before(&self, tool: &str, args: &Value, folder: &Path) -> Result<(), String> {
        for h in self.before.iter().filter(|h| h.applies(tool)) {
            let out = h.call(&json!({"event": "before", "tool": tool, "arguments": args, "folder": folder})).await;
            if !out.ok {
                let why = if out.text.is_empty() { String::new() } else { format!(": {}", out.text) };
                return Err(format!("Blocked by the hook `{}`{why}", out.run));
            }
        }
        Ok(())
    }

    /// Run the `after` hooks for this tool and return the result with anything they printed added to it.
    pub async fn after(&self, tool: &str, args: &Value, folder: &Path, result: String) -> String {
        let mut result = result;
        for h in self.after.iter().filter(|h| h.applies(tool)) {
            let out = h.call(&json!({"event": "after", "tool": tool, "arguments": args, "folder": folder, "result": result})).await;
            if !out.text.is_empty() || !out.ok {
                let state = if out.ok { "" } else { " (it failed)" };
                result.push_str(&format!("\n\n[hook `{}`{state}]\n{}", out.run, out.text));
            }
        }
        result
    }
}

impl Hook {
    fn applies(&self, tool: &str) -> bool {
        self.tools.is_empty() || self.tools.iter().any(|t| crate::canonical_name(t) == crate::canonical_name(tool))
    }

    async fn call(&self, input: &Value) -> HookOutput {
        let folder = input["folder"].as_str().map(std::path::PathBuf::from);
        let mut cmd = tokio::process::Command::new("sh");
        cmd.arg("-c").arg(&self.run).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).kill_on_drop(true);
        if let Some(f) = folder.filter(|f| f.is_dir()) {
            cmd.current_dir(f);
        }
        let done = |ok: bool, text: String| HookOutput { run: self.run.clone(), ok, text };
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => return done(false, format!("could not start: {e}")),
        };
        if let Some(mut stdin) = child.stdin.take() {
            let line = format!("{input}\n");
            // a hook that does not read its input is fine
            let _ = stdin.write_all(line.as_bytes()).await;
        }
        match tokio::time::timeout(self.timeout, child.wait_with_output()).await {
            Ok(Ok(o)) => {
                let mut text = String::from_utf8_lossy(&o.stdout).trim().to_string();
                let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
                if !err.is_empty() {
                    if !text.is_empty() {
                        text.push('\n');
                    }
                    text.push_str(&err);
                }
                if text.chars().count() > MAX_OUTPUT {
                    text = format!("{}…", text.chars().take(MAX_OUTPUT).collect::<String>());
                }
                done(o.status.success(), text)
            }
            Ok(Err(e)) => done(false, e.to_string()),
            Err(_) => done(false, format!("stopped after {} seconds", self.timeout.as_secs())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_is_read_and_mistakes_are_named() {
        let h = Hooks::parse("[[before]]\nrun = \"true\"\n\n[[after]]\ntools = [\"write_file\"]\nrun = \"fmt\"\ntimeout_secs = 5\n").unwrap();
        assert_eq!(h.before[0].tools, Vec::<String>::new());
        assert_eq!((h.after[0].tools.clone(), h.after[0].timeout), (vec!["write_file".to_string()], Duration::from_secs(5)));
        assert!(Hooks::parse("[[before]]\ntools = []\n").unwrap_err().contains("`run` is missing"));
        assert!(Hooks::parse("before = 3").unwrap_err().contains("must be a list"));
        assert!(Hooks::parse("").unwrap().is_empty());
        assert_eq!(Hooks::load(Path::new("/nonexistent/hooks.toml")).unwrap(), Hooks::default());
    }

    #[tokio::test]
    async fn a_failing_before_hook_blocks_and_says_why() {
        let d = tempfile::tempdir().unwrap();
        let h = Hooks::parse("[[before]]\ntools = [\"run_command\"]\nrun = \"echo not here >&2; exit 1\"\n").unwrap();
        let e = h.before("run_command", &json!({"command": "ls"}), d.path()).await.unwrap_err();
        assert_eq!(e, "Blocked by the hook `echo not here >&2; exit 1`: not here");
        assert!(h.before("read_file", &json!({}), d.path()).await.is_ok(), "other tools are not touched");
    }

    #[tokio::test]
    async fn after_hooks_see_the_call_and_add_their_output() {
        let d = tempfile::tempdir().unwrap();
        // the hook reads its input and prints the tool's name from it, in the chat's folder
        let h = Hooks::parse("[[after]]\nrun = \"read l; echo \\\"$l\\\" | grep -o write_file; pwd\"\n").unwrap();
        let r = h.after("write_file", &json!({"path": "a"}), d.path(), "wrote a".into()).await;
        assert!(r.starts_with("wrote a\n\n[hook `"), "{r}");
        assert!(r.contains("write_file") && r.contains(&d.path().canonicalize().unwrap().display().to_string()), "{r}");
        let quiet = Hooks::parse("[[after]]\nrun = \"true\"\n").unwrap();
        assert_eq!(quiet.after("write_file", &json!({}), d.path(), "wrote a".into()).await, "wrote a", "a quiet hook adds nothing");
    }

    #[tokio::test]
    async fn a_slow_hook_is_stopped() {
        let d = tempfile::tempdir().unwrap();
        let h = Hooks::parse("[[before]]\nrun = \"sleep 5\"\ntimeout_secs = 1\n").unwrap();
        let e = h.before("x", &json!({}), d.path()).await.unwrap_err();
        assert!(e.contains("stopped after 1 seconds"), "{e}");
    }
}
