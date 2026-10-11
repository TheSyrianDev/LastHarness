//! `lh service ...`: start the core at login as a systemd user service (Linux). It writes one file in
//! your own folder and asks systemd for your own user to start it; nothing needs root. The file is shown
//! before it is written.

use std::path::{Path, PathBuf};

pub const HELP: &str = "lh service preview          show the file that would be written and the commands that would run
lh service install [--yes]  write it, enable it and start the core now (asks first unless --yes)
lh service remove           stop the core, disable it and delete the file
lh service status           whether the file exists and what systemd says";

const NAME: &str = "lastharness.service";

pub fn unit_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config"));
    base.join("systemd/user").join(NAME)
}

/// Where the core program is: next to this program, or on the path.
pub fn find_lhd() -> Option<PathBuf> {
    let beside = std::env::current_exe().ok()?.with_file_name("lhd");
    if beside.is_file() {
        return Some(beside);
    }
    std::env::var_os("PATH").and_then(|p| std::env::split_paths(&p).map(|d| d.join("lhd")).find(|c| c.is_file()))
}

pub fn unit_text(lhd: &Path) -> String {
    format!(
        "[Unit]\nDescription=LastHarness core\nAfter=network.target\n\n[Service]\nExecStart={}\nRestart=on-failure\nRestartSec=5\n\n[Install]\nWantedBy=default.target\n",
        lhd.display()
    )
}

const INSTALL_STEPS: &[&[&str]] = &[&["daemon-reload"], &["enable", "--now", NAME]];
const REMOVE_STEPS: &[&[&str]] = &[&["disable", "--now", NAME]];

fn steps_text(steps: &[&[&str]]) -> String {
    steps.iter().map(|s| format!("  systemctl --user {}", s.join(" "))).collect::<Vec<_>>().join("\n")
}

pub fn preview(lhd: &Path) -> String {
    format!(
        "Would write {}:\n\n{}\nThen run, as your own user (no sudo):\n{}\n\nThe core reads its settings from the settings file (`lh setup` writes it). Nothing here needs root.",
        unit_path().display(),
        unit_text(lhd),
        steps_text(INSTALL_STEPS)
    )
}

/// `systemctl --user ARGS`, returning its output. Injected so tests change nothing on the machine.
pub type Systemctl<'a> = &'a mut dyn FnMut(&[&str]) -> Result<String, String>;

pub fn real_systemctl(args: &[&str]) -> Result<String, String> {
    let o = std::process::Command::new("systemctl").arg("--user").args(args).output().map_err(|e| format!("cannot run systemctl: {e}"))?;
    let text = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    if o.status.success() { Ok(text) } else { Err(text.trim().to_string()) }
}

pub fn run(args: &[String], lhd: Option<PathBuf>, confirm: &mut dyn FnMut(&str) -> bool, ctl: Systemctl) -> Result<String, String> {
    let need_lhd = || lhd.clone().ok_or_else(|| "cannot find the core program `lhd` next to this one or on the path. Build it with: cargo build --release".to_string());
    match args.first().map(String::as_str) {
        None | Some("help" | "--help" | "-h") => Ok(HELP.into()),
        Some("preview") => Ok(preview(&need_lhd()?)),
        Some("install") => {
            let lhd = need_lhd()?;
            if !args.iter().any(|a| a == "--yes") && !confirm(&format!("{}\nGo ahead? (yes/no)", preview(&lhd))) {
                return Ok("Nothing was written.".into());
            }
            let path = unit_path();
            std::fs::create_dir_all(path.parent().ok_or("no folder for the file")?).map_err(|e| format!("cannot create the folder: {e}"))?;
            std::fs::write(&path, unit_text(&lhd)).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
            for s in INSTALL_STEPS {
                ctl(s).map_err(|e| format!("systemctl --user {} failed: {e}\nThe file is at {}; `lh service remove` undoes it.", s.join(" "), path.display()))?;
            }
            Ok(format!("Installed {} and started the core. It will start at every login. Closing the terminal does not stop it.", path.display()))
        }
        Some("remove") => {
            let path = unit_path();
            if path.exists() {
                for s in REMOVE_STEPS {
                    let _ = ctl(s); // a service that is not running is fine to remove
                }
                std::fs::remove_file(&path).map_err(|e| format!("cannot delete {}: {e}", path.display()))?;
                let _ = ctl(&["daemon-reload"]);
                Ok(format!("Stopped the core and removed {}.", path.display()))
            } else {
                Ok("There is no service file to remove.".into())
            }
        }
        Some("status") => {
            let path = unit_path();
            if !path.exists() {
                return Ok(format!("Not installed ({} does not exist).", path.display()));
            }
            let st = ctl(&["is-active", NAME]).unwrap_or_else(|e| e);
            let en = ctl(&["is-enabled", NAME]).unwrap_or_else(|e| e);
            Ok(format!("Installed: {}\nActive: {}\nStarts at login: {}", path.display(), st.trim(), en.trim()))
        }
        Some(o) => Err(format!("unknown command {o}. Run `lh service help`.")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_writes_nothing_install_writes_and_enables_remove_undoes_it() {
        let d = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", d.path());
        let lhd = PathBuf::from("/opt/lh/lhd");
        let calls = std::cell::RefCell::new(Vec::<String>::new());
        let mut ctl = |a: &[&str]| -> Result<String, String> {
            calls.borrow_mut().push(a.join(" "));
            Ok("active\n".into())
        };
        let mut yes = |_: &str| true;
        let p = run(&["preview".into()], Some(lhd.clone()), &mut yes, &mut ctl).unwrap();
        assert!(p.contains("ExecStart=/opt/lh/lhd") && p.contains("no sudo") && p.contains("systemctl --user enable --now lastharness.service"), "{p}");
        assert!(!unit_path().exists());
        // saying no writes nothing
        let mut no = |_: &str| false;
        assert_eq!(run(&["install".into()], Some(lhd.clone()), &mut no, &mut ctl).unwrap(), "Nothing was written.");
        assert!(!unit_path().exists() && calls.borrow().is_empty());
        let out = run(&["install".into()], Some(lhd.clone()), &mut yes, &mut ctl).unwrap();
        assert!(out.contains("start at every login"), "{out}");
        assert_eq!(std::fs::read_to_string(unit_path()).unwrap(), unit_text(&lhd));
        assert_eq!(*calls.borrow(), ["daemon-reload", "enable --now lastharness.service"]);
        assert!(run(&["status".into()], None, &mut yes, &mut ctl).unwrap().contains("Active: active"));
        run(&["remove".into()], None, &mut yes, &mut ctl).unwrap();
        assert!(!unit_path().exists());
        assert!(run(&["remove".into()], None, &mut yes, &mut ctl).unwrap().contains("no service file"));
        assert!(run(&["install".into()], None, &mut yes, &mut ctl).unwrap_err().contains("cannot find the core program"));
    }

    #[test]
    fn the_generated_file_passes_systemd_s_own_check() {
        if std::process::Command::new("systemd-analyze").arg("--version").output().is_err() {
            return;
        }
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join(NAME);
        std::fs::write(&f, unit_text(Path::new("/bin/true"))).unwrap();
        let o = std::process::Command::new("systemd-analyze").args(["--user", "verify"]).arg(&f).output().unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    }
}
