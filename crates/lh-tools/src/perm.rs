use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::command::{classify_segment, segments, Class};

/// What a tool wants to do, in terms the permission rules understand.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Read(PathBuf),
    Write(PathBuf),
    Delete(PathBuf),
    Run(String),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    /// Default: reads in the project are automatic, writes ask once per session, the rest always asks.
    Tiered,
    AskAll,
    /// `--dangerously-skip-permissions`: no prompts. `sudo` stays denied if it is off in settings.
    Bypass,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Decision {
    Allow,
    /// Ask the user. `session_grant` is true when one yes covers the rest of the session.
    Ask { reason: String, session_grant: bool },
    Deny(String),
}

pub struct AskRequest<'a> {
    pub tool: &'a str,
    pub what: String,
    pub reason: String,
    /// Which worker is asking (shown in the prompt).
    pub worker: &'a str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Answer {
    Yes,
    No,
}

/// Whoever answers prompts: the terminal, the browser, a caller up the chain, or away mode.
pub trait Asker: Send + Sync {
    fn ask(&self, req: &AskRequest<'_>) -> impl std::future::Future<Output = Answer> + Send;
}

pub struct Permissions {
    pub mode: Mode,
    /// `sudo` is off for agents unless the user turned it on.
    pub sudo_enabled: bool,
    /// Command prefixes the user approved for good, such as `omarchy-pkg-add` or `git status`.
    pub allow_rules: Vec<String>,
    root: PathBuf,
    session_writes: AtomicBool,
}

impl Permissions {
    pub fn new(project_root: &Path) -> std::io::Result<Self> {
        Ok(Self {
            mode: Mode::Tiered,
            sudo_enabled: false,
            allow_rules: vec![],
            root: project_root.canonicalize()?,
            session_writes: AtomicBool::new(false),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Called after the user says yes to a prompt that offered a session grant.
    pub fn grant_session_writes(&self) {
        self.session_writes.store(true, Ordering::SeqCst);
    }

    /// Is this path inside the project? Follows symlinks and `..`; for a file that does not exist
    /// yet, the nearest existing parent is checked.
    pub fn inside(&self, path: &Path) -> bool {
        let abs = if path.is_absolute() { path.to_path_buf() } else { self.root.join(path) };
        let mut probe = abs.as_path();
        let mut tail = PathBuf::new();
        loop {
            if let Ok(real) = probe.canonicalize() {
                return real.join(&tail).components().collect::<PathBuf>().starts_with(&self.root) && !tail.components().any(|c| c.as_os_str() == "..");
            }
            match (probe.parent(), probe.file_name()) {
                (Some(p), Some(n)) => {
                    tail = Path::new(n).join(&tail);
                    probe = p;
                }
                _ => return false,
            }
        }
    }

    pub fn decide(&self, action: &Action) -> Decision {
        // sudo is a setting, not a prompt: off means off, even in bypass.
        if let Action::Run(cmd) = action {
            if !self.sudo_enabled && segments(cmd).iter().any(|s| classify_segment(s) == Class::Sudo) {
                return Decision::Deny("sudo is turned off for agents. It can be enabled in settings.".into());
            }
        }
        if self.mode == Mode::Bypass {
            return Decision::Allow;
        }
        if let Action::Run(cmd) = action {
            if self.allowed_by_rule(cmd) {
                return Decision::Allow;
            }
        }
        let ask = |reason: &str| Decision::Ask { reason: reason.into(), session_grant: false };
        let write_once = |this: &Self, reason: &str| {
            if this.mode == Mode::Tiered && this.session_writes.load(Ordering::SeqCst) {
                Decision::Allow
            } else {
                Decision::Ask { reason: reason.into(), session_grant: this.mode == Mode::Tiered }
            }
        };
        if self.mode == Mode::AskAll {
            return ask("ask-for-everything mode is on");
        }
        match action {
            Action::Read(p) if self.inside(p) => Decision::Allow,
            Action::Read(_) => ask("reads outside the project folder"),
            Action::Write(p) if self.inside(p) => write_once(self, "changes files in the project"),
            Action::Write(_) => ask("writes outside the project folder"),
            Action::Delete(_) => ask("deletes a file"),
            Action::Run(cmd) => self.decide_command(cmd, &write_once),
        }
    }

    fn decide_command(&self, cmd: &str, write_once: &dyn Fn(&Self, &str) -> Decision) -> Decision {
        let ask = |reason: &str| Decision::Ask { reason: reason.into(), session_grant: false };
        let mut worst = Class::ReadOnly;
        for s in segments(cmd) {
            worst = worst.max(classify_segment(&s));
        }
        // paths in the command that point out of the project turn a read into an ask
        let leaves = cmd.split_whitespace().any(|w| {
            let w = w.trim_matches(|c| c == '"' || c == '\'');
            (w.starts_with('/') || w.starts_with("~/") || w == "~" || w.starts_with("../") || w == "..") && !self.inside(Path::new(&w.replacen('~', "/home-dir-placeholder", 1)))
        });
        match worst {
            Class::Sudo => ask("runs a command as root"),
            Class::Delete => ask("deletes files"),
            Class::Install => ask("installs software"),
            Class::Network => ask("uses the network"),
            Class::Unknown => ask("runs a command this app cannot classify"),
            _ if leaves => ask("touches paths outside the project folder"),
            Class::Write => write_once(self, "changes files in the project"),
            Class::ReadOnly => Decision::Allow,
        }
    }

    fn allowed_by_rule(&self, cmd: &str) -> bool {
        let segs = segments(cmd);
        !segs.is_empty()
            && segs.iter().all(|s| {
                s != "$("
                    && !s.contains('>')
                    && self.allow_rules.iter().any(|r| s == r || (s.starts_with(r.as_str()) && s[r.len()..].starts_with(char::is_whitespace)))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn perms() -> (tempfile::TempDir, Permissions) {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir(d.path().join("src")).unwrap();
        let p = Permissions::new(d.path()).unwrap();
        (d, p)
    }
    fn run(p: &Permissions, c: &str) -> Decision {
        p.decide(&Action::Run(c.into()))
    }
    fn is_ask(d: &Decision) -> bool {
        matches!(d, Decision::Ask { .. })
    }

    #[test]
    fn tiered_defaults() {
        let (d, p) = perms();
        assert_eq!(p.decide(&Action::Read(d.path().join("src"))), Decision::Allow);
        assert!(is_ask(&p.decide(&Action::Read("/etc/passwd".into()))));
        assert!(is_ask(&p.decide(&Action::Write(d.path().join("src/new.rs")))));
        assert!(is_ask(&p.decide(&Action::Write("/tmp/x".into()))));
        assert!(is_ask(&p.decide(&Action::Delete(d.path().join("src")))));
        assert_eq!(run(&p, "ls -la"), Decision::Allow);
        assert!(is_ask(&run(&p, "cat /etc/passwd")));
        assert!(is_ask(&run(&p, "rm x")));
        assert!(is_ask(&run(&p, "curl example.com")));
        assert!(is_ask(&run(&p, "pacman -S x")));
        assert!(is_ask(&run(&p, "python3 x.py")));
    }

    #[test]
    fn write_asks_once_per_session() {
        let (d, p) = perms();
        let w = Action::Write(d.path().join("a.txt"));
        assert_eq!(p.decide(&w), Decision::Ask { reason: "changes files in the project".into(), session_grant: true });
        p.grant_session_writes();
        assert_eq!(p.decide(&w), Decision::Allow);
        assert_eq!(run(&p, "touch b.txt"), Decision::Allow);
        // the grant does not stretch to outside the project, deletes, network
        assert!(is_ask(&p.decide(&Action::Write("/tmp/x".into()))));
        assert!(is_ask(&run(&p, "rm b.txt")));
    }

    #[test]
    fn paths_that_escape_are_outside() {
        let (d, p) = perms();
        assert!(!p.inside(&d.path().join("../other")));
        assert!(!p.inside(&d.path().join("src/../../other")));
        assert!(p.inside(&d.path().join("src/not/yet/created.rs")));
        std::os::unix::fs::symlink("/etc", d.path().join("link")).unwrap();
        assert!(!p.inside(&d.path().join("link/passwd")));
        assert!(is_ask(&run(&p, "cat ../secret")));
    }

    #[test]
    fn sudo_is_off_even_in_bypass() {
        let (_d, mut p) = perms();
        assert!(matches!(run(&p, "sudo ls"), Decision::Deny(_)));
        p.mode = Mode::Bypass;
        assert!(matches!(run(&p, "ls; sudo reboot"), Decision::Deny(_)));
        assert_eq!(run(&p, "rm -rf build"), Decision::Allow);
        p.sudo_enabled = true;
        assert_eq!(run(&p, "sudo ls"), Decision::Allow);
        p.mode = Mode::Tiered;
        assert!(is_ask(&run(&p, "sudo ls")));
    }

    #[test]
    fn allow_rules_and_ask_all() {
        let (_d, mut p) = perms();
        p.allow_rules = vec!["omarchy-pkg-add".into(), "git status".into()];
        assert_eq!(run(&p, "omarchy-pkg-add htop"), Decision::Allow);
        assert_eq!(run(&p, "git status -s"), Decision::Allow);
        assert!(is_ask(&run(&p, "omarchy-pkg-addx htop")));
        assert!(is_ask(&run(&p, "omarchy-pkg-add htop && rm -rf x")));
        assert!(is_ask(&run(&p, "git status > out.txt")));
        assert!(is_ask(&run(&p, "git status $(rm x)")));
        // a rule never lifts the sudo ban
        p.allow_rules.push("sudo".into());
        assert!(matches!(run(&p, "sudo ls"), Decision::Deny(_)));
        p.mode = Mode::AskAll;
        assert!(is_ask(&p.decide(&Action::Read("src".into()))));
        assert_eq!(run(&p, "git status"), Decision::Allow);
    }
}
