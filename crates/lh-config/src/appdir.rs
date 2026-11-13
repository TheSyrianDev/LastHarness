//! Folder and environment-variable names for the app. The app was called LastHarness; it is Last AI now.
//! New names are used first. The old names still work: an old folder is copied over once (first run), and an
//! old `LASTHARNESS_*` variable is read when its `LASTAI_*` twin is not set. Files are copied, never moved, so
//! the old folder stays as a backup.

use std::path::{Path, PathBuf};

/// The new folder name (`~/.config/lastai`, `~/.local/share/lastai`, ...).
pub const NEW_NAME: &str = "lastai";
/// The name used before the rename. Read only, for the fallback and the migration.
pub const OLD_NAME: &str = "lastharness";

/// Pick the folder to use. If `new` exists, use it. Else, if `old` exists, copy it to `new` (once, logged to
/// stderr) and use `new`. Else `new` is used and created later by the caller. Returns the folder to use.
pub fn resolve_dir(new: &Path, old: &Path) -> PathBuf {
    if new.exists() || !old.is_dir() {
        return new.to_path_buf();
    }
    match copy_dir(old, new) {
        Ok(n) => eprintln!("lastai: copied {n} file(s) from {} to {} (the old folder is kept)", old.display(), new.display()),
        Err(e) => eprintln!("lastai: cannot copy {} to {}: {e}; using the old folder", old.display(), new.display()),
    }
    if new.exists() { new.to_path_buf() } else { old.to_path_buf() }
}

/// Copy every file under `from` into `to`, keeping names. Existing files in `to` are not overwritten.
/// Returns how many files were copied.
pub fn copy_dir(from: &Path, to: &Path) -> std::io::Result<usize> {
    let mut n = 0;
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let (src, dst) = (entry.path(), to.join(entry.file_name()));
        if entry.file_type()?.is_dir() {
            n += copy_dir(&src, &dst)?;
        } else if !dst.exists() {
            std::fs::copy(&src, &dst)?;
            n += 1;
        }
    }
    Ok(n)
}

/// Read an environment variable under its new name, else its old name.
pub fn env_os(new: &str, old: &str) -> Option<std::ffi::OsString> {
    std::env::var_os(new).or_else(|| std::env::var_os(old))
}

/// `$XDG_CONFIG_HOME` or `~/.config`, then the app folder, with the old-name migration applied.
pub fn config_dir() -> PathBuf {
    let base = xdg_or_home("XDG_CONFIG_HOME", ".config");
    resolve_dir(&base.join(NEW_NAME), &base.join(OLD_NAME))
}

/// `$XDG_DATA_HOME` or `~/.local/share`, then the app folder, with the old-name migration applied.
pub fn data_dir() -> PathBuf {
    let base = xdg_or_home("XDG_DATA_HOME", ".local/share");
    resolve_dir(&base.join(NEW_NAME), &base.join(OLD_NAME))
}

/// `$XDG_RUNTIME_DIR` (if set), else `~/.local/state`, then the app folder, with the old-name migration applied.
pub fn state_or_runtime_dir() -> PathBuf {
    let base = match std::env::var_os("XDG_RUNTIME_DIR") {
        Some(d) => PathBuf::from(d),
        None => home().join(".local/state"),
    };
    resolve_dir(&base.join(NEW_NAME), &base.join(OLD_NAME))
}

fn home() -> PathBuf {
    PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
}

fn xdg_or_home(var: &str, fallback: &str) -> PathBuf {
    std::env::var_os(var).filter(|v| !v.is_empty()).map(PathBuf::from).unwrap_or_else(|| home().join(fallback))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn new_dir_is_used_when_it_exists() {
        let d = tmp();
        let (new, old) = (d.path().join("lastai"), d.path().join("lastharness"));
        std::fs::create_dir_all(&new).unwrap();
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join("config.toml"), "x").unwrap();
        assert_eq!(resolve_dir(&new, &old), new);
        assert!(!new.join("config.toml").exists(), "no copy when the new folder exists");
    }

    #[test]
    fn migration_copies_old_files_once() {
        let d = tmp();
        let (new, old) = (d.path().join("lastai"), d.path().join("lastharness"));
        std::fs::create_dir_all(old.join("themes")).unwrap();
        std::fs::write(old.join("config.toml"), "a = 1").unwrap();
        std::fs::write(old.join("themes/mine.toml"), "b = 2").unwrap();
        assert_eq!(resolve_dir(&new, &old), new);
        assert_eq!(std::fs::read_to_string(new.join("config.toml")).unwrap(), "a = 1");
        assert_eq!(std::fs::read_to_string(new.join("themes/mine.toml")).unwrap(), "b = 2");
        assert!(old.join("config.toml").exists(), "old folder is kept, files are copied");
        // second run: the new folder exists, nothing is copied again
        std::fs::write(new.join("config.toml"), "changed").unwrap();
        assert_eq!(resolve_dir(&new, &old), new);
        assert_eq!(std::fs::read_to_string(new.join("config.toml")).unwrap(), "changed");
    }

    #[test]
    fn no_old_dir_returns_new_without_creating() {
        let d = tmp();
        let (new, old) = (d.path().join("lastai"), d.path().join("lastharness"));
        // no old folder at all: the new name is returned, nothing is created by resolve
        assert_eq!(resolve_dir(&new, &old), new);
        assert!(!new.exists());
    }

    #[test]
    fn copy_never_overwrites_existing_new_file() {
        let d = tmp();
        let (from, to) = (d.path().join("a"), d.path().join("b"));
        std::fs::create_dir_all(&from).unwrap();
        std::fs::create_dir_all(&to).unwrap();
        std::fs::write(from.join("f"), "old").unwrap();
        std::fs::write(to.join("f"), "new").unwrap();
        assert_eq!(copy_dir(&from, &to).unwrap(), 0);
        assert_eq!(std::fs::read_to_string(to.join("f")).unwrap(), "new");
    }

    #[test]
    fn env_var_new_name_wins_then_old_name() {
        let key_new = "LASTAI_TEST_APPDIR_ENV";
        let key_old = "LASTHARNESS_TEST_APPDIR_ENV";
        std::env::remove_var(key_new);
        std::env::remove_var(key_old);
        assert_eq!(env_os(key_new, key_old), None);
        std::env::set_var(key_old, "old");
        assert_eq!(env_os(key_new, key_old).unwrap(), "old", "old name is the fallback");
        std::env::set_var(key_new, "new");
        assert_eq!(env_os(key_new, key_old).unwrap(), "new", "new name wins");
        std::env::remove_var(key_new);
        std::env::remove_var(key_old);
    }
}
