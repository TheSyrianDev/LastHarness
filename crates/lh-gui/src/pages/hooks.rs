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
    h.to_toml()
}

/// Check the new file reads back the same, then write it.
pub fn save(p: &Path, h: &Hooks) -> Result<(), String> {
    h.save(p)
}

pub use lh_tools::hooks::from_form;

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
