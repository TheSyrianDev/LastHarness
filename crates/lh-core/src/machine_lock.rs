//! The machine lock (`~/AI/.machine-busy`), taken and released only through `machine-lock`
//! (`~/AI/Rig/bin/machine-lock`), which creates the file atomically and refuses while it is held.
//! Nothing in this crate writes the lock file itself.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Owner field used on every lock line we write.
pub const OWNER: &str = "claude-agent lastai";

/// The script: `MACHINE_LOCK_CMD` when set (tests), else `$HOME/AI/Rig/bin/machine-lock`.
fn script() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("MACHINE_LOCK_CMD") {
        return Some(PathBuf::from(p));
    }
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join("AI/Rig/bin/machine-lock"))
}

/// Take the lock at `lock` for `job`. Errors with the holder's line when it is busy.
pub fn take(lock: &Path, job: &str) -> Result<(), String> {
    let cmd = script().ok_or("no home folder is set")?;
    let out = Command::new(&cmd)
        .env("MACHINE_LOCK", lock)
        .args(["take", OWNER, job])
        .output()
        .map_err(|e| format!("cannot run machine-lock: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        let why = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(format!("the machine is busy: {why}"))
    }
}

/// Release only our own line. Quietly does nothing when the lock is someone else's.
pub fn release(lock: &Path) {
    if let Some(cmd) = script() {
        let _ = Command::new(&cmd).env("MACHINE_LOCK", lock).args(["release", OWNER]).output();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("lh-mlock-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d.join(".machine-busy")
    }

    #[test]
    fn take_then_release_leaves_no_file_and_second_take_is_refused() {
        let lock = temp("cycle");
        take(&lock, "test one").unwrap();
        assert!(take(&lock, "test two").unwrap_err().contains("busy"));
        release(&lock);
        assert!(!lock.exists());
    }

    #[test]
    fn release_does_not_touch_someone_elses_line() {
        let lock = temp("other");
        std::fs::write(&lock, "lastengine-74 | bench | 2026-11-12\n").unwrap();
        release(&lock);
        assert!(std::fs::read_to_string(&lock).unwrap().starts_with("lastengine-74"));
    }
}
