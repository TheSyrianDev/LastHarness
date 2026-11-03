//! "Start with the computer": a systemd user service that starts the core at login (Linux). It writes one file in
//! the person's own folder and asks systemd for their own user; nothing needs root. This mirrors `lh service`
//! in the terminal app, and the file is shown before it is written.

use std::path::{Path, PathBuf};

const NAME: &str = "lastharness.service";
const INSTALL_STEPS: &[&[&str]] = &[&["daemon-reload"], &["enable", "--now", NAME]];
const REMOVE_STEPS: &[&[&str]] = &[&["disable", "--now", NAME]];

pub fn unit_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config"));
    base.join("systemd/user").join(NAME)
}

/// Where the core program is: next to this program, or on the path.
pub fn find_lhd() -> Option<PathBuf> {
    let beside = std::env::current_exe().ok()?.with_file_name(format!("lhd{}", std::env::consts::EXE_SUFFIX));
    if beside.is_file() {
        return Some(beside);
    }
    std::env::var_os("PATH").and_then(|p| std::env::split_paths(&p).map(|d| d.join(format!("lhd{}", std::env::consts::EXE_SUFFIX))).find(|c| c.is_file()))
}

pub fn unit_text(lhd: &Path) -> String {
    format!("[Unit]\nDescription=LastHarness core\nAfter=network.target\n\n[Service]\nExecStart={}\nRestart=on-failure\nRestartSec=5\n\n[Install]\nWantedBy=default.target\n", lhd.display())
}

fn steps_text(steps: &[&[&str]]) -> String {
    steps.iter().map(|s| format!("  systemctl --user {}", s.join(" "))).collect::<Vec<_>>().join("\n")
}

/// What would be written and run, shown before the person agrees.
pub fn preview(lhd: &Path, path: &Path) -> String {
    format!("This writes {}:\n\n{}\nThen it runs, as you (no sudo):\n{}", path.display(), unit_text(lhd), steps_text(INSTALL_STEPS))
}

/// `systemctl --user ARGS`, returning its output. Passed in so tests change nothing on the machine.
pub type Systemctl<'a> = &'a mut dyn FnMut(&[&str]) -> Result<String, String>;

pub fn real_systemctl(args: &[&str]) -> Result<String, String> {
    let o = std::process::Command::new("systemctl").arg("--user").args(args).output().map_err(|e| format!("cannot run systemctl: {e}"))?;
    let text = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    if o.status.success() {
        Ok(text)
    } else {
        Err(text.trim().to_string())
    }
}

pub fn install(lhd: &Path, path: &Path, ctl: Systemctl) -> Result<String, String> {
    std::fs::create_dir_all(path.parent().ok_or("There is no folder for the file.")?).map_err(|e| format!("Cannot create the folder: {e}"))?;
    std::fs::write(path, unit_text(lhd)).map_err(|e| format!("Cannot write {}: {e}", path.display()))?;
    for s in INSTALL_STEPS {
        ctl(s).map_err(|e| format!("systemctl --user {} failed: {e}. The file is at {}; Stop starting with the computer undoes it.", s.join(" "), path.display()))?;
    }
    Ok("The core now starts when you log in, and is running now.".into())
}

pub fn remove(path: &Path, ctl: Systemctl) -> Result<String, String> {
    if path.exists() {
        for s in REMOVE_STEPS {
            // not running or not enabled is fine: the goal is that it is gone
            let _ = ctl(s);
        }
        std::fs::remove_file(path).map_err(|e| format!("Cannot delete {}: {e}", path.display()))?;
        let _ = ctl(&["daemon-reload"]);
        Ok("The core no longer starts with the computer.".into())
    } else {
        Ok("It was not set to start with the computer.".into())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Status {
    pub installed: bool,
    pub active: Option<String>,
}

pub fn status(path: &Path, ctl: Systemctl) -> Status {
    let installed = path.exists();
    let active = if installed { ctl(&["is-active", NAME]).ok().map(|t| t.trim().to_string()).or_else(|| Some("not running".into())) } else { None };
    Status { installed, active }
}

pub fn status_words(s: &Status) -> String {
    match (s.installed, s.active.as_deref()) {
        (false, _) => "Not set up. The core starts only when you start it.".into(),
        (true, Some("active")) => "Set up. The core starts when you log in, and is running now.".into(),
        (true, _) => "Set up. The core starts when you log in, but is not running now.".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake(log: &std::cell::RefCell<Vec<String>>) -> impl FnMut(&[&str]) -> Result<String, String> + '_ {
        move |a: &[&str]| {
            log.borrow_mut().push(a.join(" "));
            if a[0] == "is-active" {
                Ok("active\n".into())
            } else {
                Ok(String::new())
            }
        }
    }

    #[test]
    fn install_writes_the_file_then_asks_systemd() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("systemd/user/lastharness.service");
        let log = std::cell::RefCell::new(vec![]);
        let msg = install(Path::new("/opt/lhd"), &path, &mut fake(&log)).unwrap();
        assert!(msg.contains("log in"));
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("ExecStart=/opt/lhd"));
        assert_eq!(*log.borrow(), ["daemon-reload", "enable --now lastharness.service"]);
        let st = status(&path, &mut fake(&log));
        assert_eq!(st, Status { installed: true, active: Some("active".into()) });
        assert!(status_words(&st).contains("running now"));
    }

    #[test]
    fn remove_stops_it_and_deletes_the_file() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("lastharness.service");
        std::fs::write(&path, "x").unwrap();
        let log = std::cell::RefCell::new(vec![]);
        assert!(remove(&path, &mut fake(&log)).unwrap().contains("no longer"));
        assert!(!path.exists());
        assert_eq!(log.borrow()[0], "disable --now lastharness.service");
        assert!(remove(&path, &mut fake(&log)).unwrap().contains("not set"));
    }

    #[test]
    fn a_failed_step_names_the_file_so_it_can_be_undone() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("u.service");
        let e = install(Path::new("/x/lhd"), &path, &mut |_: &[&str]| Err("no bus".to_string())).unwrap_err();
        assert!(e.contains("no bus") && e.contains("u.service"));
    }

    #[test]
    fn nothing_installed_means_no_systemd_question() {
        let d = tempfile::tempdir().unwrap();
        let st = status(&d.path().join("none"), &mut |_: &[&str]| panic!("must not be asked"));
        assert_eq!(st, Status { installed: false, active: None });
        assert!(status_words(&st).starts_with("Not set up"));
    }

    #[test]
    fn the_preview_shows_the_file_and_the_commands() {
        let p = preview(Path::new("/opt/lhd"), Path::new("/home/me/.config/systemd/user/lastharness.service"));
        assert!(p.contains("ExecStart=/opt/lhd") && p.contains("systemctl --user enable --now lastharness.service") && p.contains("no sudo"));
    }
}
