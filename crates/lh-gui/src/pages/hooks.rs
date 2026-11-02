//! Hooks: commands that run before or after a tool call. They live in `hooks.toml` beside the settings file, which
//! this page reads and writes directly; the core reads the file again for each new chat.

use std::path::{Path, PathBuf};

pub use lh_tools::hooks::Hook;
pub use lh_tools::Hooks;

/// `hooks.toml` beside the settings file.
pub fn path() -> PathBuf {
    lh_config::ConfigFile::default_path().with_file_name("hooks.toml")
}

pub fn load(p: &Path) -> Result<Hooks, String> {
    Hooks::load(p)
}

/// The file text for these hooks, in the same form the user would write by hand.
pub fn to_toml(h: &Hooks) -> String {
    let mut out = String::from("# Commands run before or after a tool call. Edited in Settings, Hooks.\n");
    for (name, list) in [("before", &h.before), ("after", &h.after)] {
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
pub fn save(p: &Path, h: &Hooks) -> Result<(), String> {
    let text = to_toml(h);
    if Hooks::parse(&text)? != *h {
        return Err("the hooks could not be written the way they read back".into());
    }
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    std::fs::write(p, text).map_err(|e| format!("{}: {e}", p.display()))
}

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
    Ok(Hook { tools, run: run.to_string(), timeout: std::time::Duration::from_secs(secs) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hooks_survive_a_write_and_a_read() {
        let h = Hooks { before: vec![from_form("run_command", "echo \"hi\"; exit 0", "").unwrap()], after: vec![from_form("write_file, edit_file", "cargo fmt", "5").unwrap()] };
        let back = Hooks::parse(&to_toml(&h)).unwrap();
        assert_eq!(back, h);
        let dir = std::env::temp_dir().join(format!("lh-gui-hooks-{}", std::process::id()));
        let p = dir.join("hooks.toml");
        save(&p, &h).unwrap();
        assert_eq!(load(&p).unwrap(), h);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_form_says_what_is_wrong() {
        assert!(from_form("", " ", "").is_err());
        assert!(from_form("", "x", "soon").is_err());
        assert!(from_form("", "x", "0").is_err());
        assert!(from_form("", "x", "").unwrap().tools.is_empty(), "no tools means every tool");
    }
}
