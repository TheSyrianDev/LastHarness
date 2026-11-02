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
    /// Fetches a web address, or sends a search query. Leaves the machine.
    Fetch(String),
    /// Reads (`write: false`) or changes (`write: true`) the user's memory. `what` is shown in the prompt.
    Memory { write: bool, what: String },
    /// A tool defined outside this crate.
    Extra { name: String, what: String, risk: crate::Risk },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    /// Default: reads in the project are automatic, writes ask once per session, the rest always asks.
    Tiered,
    AskAll,
    /// `--dangerously-skip-permissions`: no prompts. `sudo` stays denied if it is off in settings.
    Bypass,
}

/// When a file write shows its diff (`permissions.show_diff`). Trusted means the chat said
/// "always allow writes", so its writes no longer ask.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ShowDiff {
    #[default]
    Always,
    /// Only in chats that still ask before writing.
    Untrusted,
    /// Only in chats set to always allow writes.
    Trusted,
    Never,
}

impl ShowDiff {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "always" => Self::Always,
            "untrusted" => Self::Untrusted,
            "trusted" => Self::Trusted,
            "never" => Self::Never,
            _ => return None,
        })
    }
    /// Should a write in a chat that is (or is not) trusted show its diff?
    pub fn shows(self, trusted: bool) -> bool {
        match self {
            Self::Always => true,
            Self::Untrusted => !trusted,
            Self::Trusted => trusted,
            Self::Never => false,
        }
    }
    fn code(self) -> u8 {
        self as u8
    }
    fn from_code(c: u8) -> Self {
        [Self::Always, Self::Untrusted, Self::Trusted, Self::Never].get(c as usize).copied().unwrap_or_default()
    }
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
    /// More to look at before answering, such as the diff of a file write. Shown, never logged. Often empty.
    pub detail: String,
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
    /// The mode this session asked for. With `live` set, the settings can tighten it or lift the bypass.
    pub mode: Mode,
    /// Permission settings the core can change while it runs. Read at every decision, so a change
    /// applies to the next tool call, also in a chat that is already open.
    pub live: Option<std::sync::Arc<LivePerms>>,
    /// Does the live ask-for-everything setting apply here? Chats yes; department workers keep their own mode.
    pub live_ask_all: bool,
    /// `sudo` is off for agents unless the user turned it on.
    pub sudo_enabled: bool,
    /// Command prefixes the user approved for good, such as `omarchy-pkg-add` or `git status`.
    pub allow_rules: Vec<String>,
    /// A project worker is locked to its folder: allow rules no longer cover commands that reach outside
    /// it, so those requests go up the chain instead of being waved through.
    pub locked: bool,
    /// When write diffs are shown. With `live` set, the live setting wins.
    pub show_diff: ShowDiff,
    root: PathBuf,
    session_writes: AtomicBool,
}

/// The permission settings that may change while the core runs. Only the core writes them.
#[derive(Debug, Default)]
pub struct LivePerms {
    bypass: AtomicBool,
    ask_all: AtomicBool,
    sudo: AtomicBool,
    show_diff: std::sync::atomic::AtomicU8,
}

impl LivePerms {
    pub fn new(bypass: bool, ask_all: bool, sudo: bool) -> Self {
        Self { bypass: AtomicBool::new(bypass), ask_all: AtomicBool::new(ask_all), sudo: AtomicBool::new(sudo), show_diff: Default::default() }
    }
    pub fn set_show_diff(&self, v: ShowDiff) {
        self.show_diff.store(v.code(), Ordering::SeqCst);
    }
    pub fn show_diff(&self) -> ShowDiff {
        ShowDiff::from_code(self.show_diff.load(Ordering::SeqCst))
    }
    pub fn set(&self, bypass: bool, ask_all: bool, sudo: bool) {
        self.bypass.store(bypass, Ordering::SeqCst);
        self.ask_all.store(ask_all, Ordering::SeqCst);
        self.sudo.store(sudo, Ordering::SeqCst);
    }
    pub fn get(&self) -> (bool, bool, bool) {
        (self.bypass.load(Ordering::SeqCst), self.ask_all.load(Ordering::SeqCst), self.sudo.load(Ordering::SeqCst))
    }
}

impl Permissions {
    /// The mode in force right now.
    pub fn effective_mode(&self) -> Mode {
        let Some(l) = &self.live else { return self.mode };
        if self.mode == Mode::Bypass || l.bypass.load(Ordering::SeqCst) {
            Mode::Bypass
        } else if self.mode == Mode::AskAll || (self.live_ask_all && l.ask_all.load(Ordering::SeqCst)) {
            Mode::AskAll
        } else {
            Mode::Tiered
        }
    }

    /// Is sudo allowed right now?
    pub fn sudo_allowed(&self) -> bool {
        self.live.as_ref().map_or(self.sudo_enabled, |l| l.sudo.load(Ordering::SeqCst))
    }

    pub fn new(project_root: &Path) -> std::io::Result<Self> {
        Ok(Self {
            mode: Mode::Tiered,
            live: None,
            live_ask_all: false,
            sudo_enabled: false,
            allow_rules: vec![],
            locked: false,
            show_diff: ShowDiff::Always,
            root: project_root.canonicalize()?,
            session_writes: AtomicBool::new(false),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The diff setting in force right now.
    pub fn show_diff(&self) -> ShowDiff {
        self.live.as_ref().map_or(self.show_diff, |l| l.show_diff())
    }

    /// Has this chat said "always allow writes"?
    pub fn writes_trusted(&self) -> bool {
        self.session_writes.load(Ordering::SeqCst)
    }

    /// Called after the user says yes to a prompt that offered a session grant.
    pub fn grant_session_writes(&self) {
        self.session_writes.store(true, Ordering::SeqCst);
    }

    /// Is this path inside the project? Follows symlinks and `..`; for a file that does not exist
    /// yet, the nearest existing parent is checked. Places that hold keys, passwords, browser profiles or
    /// LastHarness's own data never count as inside, even when the project is the whole home folder.
    pub fn inside(&self, path: &Path) -> bool {
        self.inside_raw(path) && !self.in_secret_place(path)
    }

    fn in_secret_place(&self, path: &Path) -> bool {
        const SINGLE: &[&str] = &[".ssh", ".gnupg", ".aws", ".azure", ".kube", ".password-store", ".netrc", ".mozilla", "keyrings", ".pki"];
        const PAIRS: &[(&str, &str)] = &[(".config", "gcloud"), (".config", "lastharness"), (".config", "BraveSoftware"), (".config", "chromium"), (".config", "google-chrome"), (".config", "Signal"), (".config", "rclone"), (".config", "gh"), (".docker", "config.json"), (".local", "lastharness")];
        let abs = if path.is_absolute() { path.to_path_buf() } else { self.root.join(path) };
        // judge the real place when it exists, so a link into one of these folders is caught too
        let real = abs.ancestors().find_map(|a| a.canonicalize().ok().map(|r| r.join(abs.strip_prefix(a).unwrap_or(Path::new(""))))).unwrap_or(abs);
        // only the part below the project root: a project that lives somewhere odd is the user's choice
        let below = real.strip_prefix(&self.root).unwrap_or(&real);
        let parts: Vec<&str> = below.components().filter_map(|c| c.as_os_str().to_str()).collect();
        parts.iter().any(|p| SINGLE.contains(p)) || parts.windows(2).any(|w| PAIRS.contains(&(w[0], w[1]))) || parts.windows(3).any(|w| w == [".local", "share", "lastharness"])
    }

    fn inside_raw(&self, path: &Path) -> bool {
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
        let mode = self.effective_mode();
        // sudo is a setting, not a prompt: off means off, even in bypass.
        if let Action::Run(cmd) = action {
            if !self.sudo_allowed() && segments(cmd).iter().any(|s| classify_segment(s) == Class::Sudo) {
                return Decision::Deny("sudo is turned off for agents. It can be enabled in settings.".into());
            }
        }
        if mode == Mode::Bypass {
            return Decision::Allow;
        }
        if let Action::Run(cmd) = action {
            if self.allowed_by_rule(cmd) && !(self.locked && self.command_leaves(cmd)) {
                return Decision::Allow;
            }
        }
        let ask = |reason: &str| Decision::Ask { reason: reason.into(), session_grant: false };
        let write_once = |this: &Self, reason: &str| {
            if mode == Mode::Tiered && this.session_writes.load(Ordering::SeqCst) {
                Decision::Allow
            } else {
                Decision::Ask { reason: reason.into(), session_grant: mode == Mode::Tiered }
            }
        };
        if mode == Mode::AskAll {
            return ask("ask-for-everything mode is on");
        }
        match action {
            Action::Read(p) if self.inside(p) => Decision::Allow,
            Action::Read(_) => ask("reads outside the project folder"),
            Action::Write(p) if self.inside(p) => write_once(self, "changes files in the project"),
            Action::Write(_) => ask("writes outside the project folder"),
            Action::Delete(_) => ask("deletes a file"),
            Action::Fetch(_) => ask("uses the network"),
            Action::Memory { write: false, .. } => Decision::Allow,
            Action::Memory { write: true, .. } => ask("changes what is remembered about you"),
            Action::Extra { risk: crate::Risk::ReadOnly, .. } => Decision::Allow,
            Action::Extra { risk: crate::Risk::Writes, .. } => write_once(self, "this tool changes things"),
            Action::Extra { risk: crate::Risk::Asks, name, .. } => ask(&format!("{name} reaches outside the project")),
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
        let leaves = self.command_leaves(cmd);
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

    /// Does the command name a path outside the project folder? Every word is looked at, not only the
    /// ones that start with `/`: relative paths that climb out (`a/../../x`) or run through a symlink,
    /// `--option=/path`, `VAR=/path`, input redirects (`</etc/passwd`), `~user` and anything built from a
    /// variable all count. What cannot be judged counts as leaving.
    fn command_leaves(&self, cmd: &str) -> bool {
        cmd.split_whitespace().any(|raw| {
            let w = raw.trim_matches(|c| c == '"' || c == '\'');
            // drop a redirect operator in front: `2>file`, `<file`, `>>file`, `&>file`
            let w = match w.find(['<', '>']) {
                Some(i) if w[..i].chars().all(|c| c.is_ascii_digit() || c == '&') => w[i..].trim_start_matches(['<', '>', '&']),
                _ => w,
            };
            let mut candidates = vec![w];
            if let Some((_, v)) = w.split_once('=') {
                candidates.push(v.trim_matches(|c| c == '"' || c == '\''));
            }
            candidates.into_iter().any(|s| {
                if s.is_empty() || s.starts_with('-') {
                    return false;
                }
                if s.starts_with('$') || s.starts_with('`') || s.starts_with('~') || s.contains("$(") || s.contains("${") {
                    return true;
                }
                if matches!(s, "/dev/null" | "/dev/stdout" | "/dev/stderr") {
                    return false;
                }
                // brace expansion: `{a,/etc/passwd}` is two paths. One group is expanded and each result judged;
                // anything more tangled counts as leaving.
                if let (Some(open), Some(close)) = (s.find('{'), s.rfind('}')) {
                    let inner = &s[open + 1..close];
                    if open < close && (inner.contains('{') || s[close + 1..].contains('{') || s[..open].contains('}')) {
                        return true;
                    }
                    if open < close && inner.contains(',') {
                        return inner.split(',').any(|alt| {
                            let word = format!("{}{}{}", &s[..open], alt, &s[close + 1..]);
                            word.starts_with('$') || word.starts_with('~') || !self.inside(Path::new(&word))
                        });
                    }
                    if open < close && inner.contains("..") {
                        return true; // a range such as {1..9}: not worth judging
                    }
                }
                !self.inside(Path::new(s))
            })
        })
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
    fn keys_browser_profiles_and_the_cores_own_data_are_never_inside_even_when_the_project_is_home() {
        let d = tempfile::tempdir().unwrap();
        for dir in [".ssh", ".gnupg", ".aws", ".config/lastharness", ".local/share/lastharness/memory/facts", ".config/BraveSoftware/Brave-Browser", "Documents"] {
            std::fs::create_dir_all(d.path().join(dir)).unwrap();
        }
        std::fs::write(d.path().join(".ssh/id_rsa"), "key").unwrap();
        std::fs::write(d.path().join(".local/share/lastharness/memory/facts/health.md"), "sensitive note").unwrap();
        std::fs::write(d.path().join("Documents/a.txt"), "ok").unwrap();
        std::os::unix::fs::symlink(d.path().join(".ssh"), d.path().join("Documents/shortcut")).unwrap();
        let p = Permissions::new(d.path()).unwrap();
        for f in [".ssh/id_rsa", ".ssh/new_key", ".local/share/lastharness/memory/facts/health.md", ".config/lastharness/config.toml", ".config/BraveSoftware/Brave-Browser/Cookies", ".gnupg/x", ".aws/credentials", "Documents/shortcut/id_rsa"] {
            assert!(is_ask(&p.decide(&Action::Read(d.path().join(f)))), "{f} must ask");
            assert!(is_ask(&p.decide(&Action::Write(d.path().join(f)))), "{f} must ask to write");
        }
        assert_eq!(p.decide(&Action::Read(d.path().join("Documents/a.txt"))), Decision::Allow);
        assert!(is_ask(&run(&p, "cat .ssh/id_rsa")));
        assert!(is_ask(&run(&p, "cat .local/share/lastharness/memory/facts/health.md")));
        assert!(is_ask(&run(&p, "grep -r note .local")) || run(&p, "grep -r note .local") == Decision::Allow, "a bare folder name is not a path into the place");
        assert_eq!(run(&p, "ls Documents"), Decision::Allow);
    }

    #[test]
    fn commands_cannot_reach_outside_by_less_obvious_paths() {
        let (d, p) = perms();
        std::fs::create_dir(d.path().join("sub")).unwrap();
        std::os::unix::fs::symlink("/etc/hostname", d.path().join("hn")).unwrap();
        std::os::unix::fs::symlink("/etc", d.path().join("etc-link")).unwrap();
        for c in [
            "cat sub/../../secret",
            "cat $HOME/.ssh/id_rsa",
            "cat ${HOME}/x",
            "cat </etc/passwd",
            "cat 0</etc/passwd",
            "grep --file=/etc/passwd x",
            "ls --ignore=../x",
            "cat ~root/.bashrc",
            "cat hn",
            "cat etc-link/passwd",
            "cat \"/etc/passwd\"",
            "cat {a,/etc/passwd}",
            "cat /{etc,tmp}/x",
            "cat x{a,{b,/etc/passwd}}",
            "ls {1..9}/../../x",
            "wc --files0-from=/etc/x",
            "cp src /tmp/out",
            "cat x > /etc/out",
            "echo hi 2>/tmp/err",
            "echo a >>../log",
            "FOO=/etc/x ls",
        ] {
            assert!(is_ask(&run(&p, c)), "{c} should ask, got {:?}", run(&p, c));
        }
        for c in ["cat sub/a.txt", "ls -la", "cat a.txt 2>/dev/null", "grep -rn 'fn main' src", "echo 2>&1", "sed -n 1,5p src/a.rs"] {
            assert_eq!(run(&p, c), Decision::Allow, "{c} stays inside");
        }
    }

    #[test]
    fn substitution_and_odd_environment_never_ride_on_an_allow_rule() {
        let (_d, mut p) = perms();
        p.allow_rules = vec!["git status".into(), "cat".into(), "ls".into()];
        for c in ["git status <(rm x)", "cat >(sh)", "ls $(rm x)", "ls `rm x`", "PATH=./bin ls", "LD_PRELOAD=./x.so ls"] {
            // an allow rule matches the start of a segment; none of these may be waved through
            assert!(is_ask(&run(&p, c)), "{c} -> {:?}", run(&p, c));
        }
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
    fn a_locked_worker_is_not_waved_through_by_an_allow_rule_when_it_reaches_outside() {
        let (_d, mut p) = perms();
        p.allow_rules = vec!["cat".into()];
        assert_eq!(run(&p, "cat src/a.txt"), Decision::Allow);
        assert_eq!(run(&p, "cat /etc/hostname"), Decision::Allow, "an unlocked worker may use the rule anywhere");
        p.locked = true;
        assert_eq!(run(&p, "cat src/a.txt"), Decision::Allow, "inside the folder the rule still counts");
        assert!(is_ask(&run(&p, "cat /etc/hostname")), "outside it, the request goes up");
        assert!(is_ask(&run(&p, "cat ../secret")));
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

    #[test]
    fn live_settings_change_the_next_decision() {
        let d = tempfile::tempdir().unwrap();
        let live = std::sync::Arc::new(LivePerms::new(false, false, false));
        let mut p = Permissions::new(d.path()).unwrap();
        p.live = Some(live.clone());
        p.live_ask_all = true;
        let sudo = Action::Run("sudo ls".into());
        let write = Action::Write(d.path().join("a"));
        assert!(matches!(p.decide(&sudo), Decision::Deny(_)));
        live.set(false, false, true);
        assert!(!matches!(p.decide(&sudo), Decision::Deny(_)), "sudo allowed at once");
        live.set(true, false, true);
        assert_eq!(p.decide(&write), Decision::Allow, "bypass at once");
        live.set(false, true, false);
        assert!(matches!(p.decide(&write), Decision::Ask { .. }));
        assert!(matches!(p.decide(&sudo), Decision::Deny(_)), "sudo off again, also during bypass");
        assert_eq!(p.effective_mode(), Mode::AskAll);
    }
}
