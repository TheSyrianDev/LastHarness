//! Where the core is and how to prove who we are to it. Flags win over the environment; both have defaults.

use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub base: String,
    pub token_file: PathBuf,
    /// Where the app keeps its own files (preferences); overridable so tests and trials stay apart.
    pub prefs: PathBuf,
    pub themes: PathBuf,
}

/// Where `lhd` writes its token by default: the runtime folder if there is one, else the state folder.
pub fn default_token_path() -> PathBuf {
    if let Some(d) = std::env::var_os("XDG_RUNTIME_DIR") {
        return PathBuf::from(d).join("lastharness").join("token");
    }
    PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state/lastharness/token")
}

/// Has the person asked their desktop for less movement? `LASTHARNESS_REDUCE_MOTION=1` says so directly; else the
/// desktop's own animation setting is read once (GNOME-family settings, which most desktops mirror).
pub fn reduce_motion() -> bool {
    static CACHE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *CACHE.get_or_init(|| {
        let forced = std::env::var("LASTHARNESS_REDUCE_MOTION").ok();
        let gtk = std::env::var("GTK_ENABLE_ANIMATIONS").ok();
        let setting = std::process::Command::new("gsettings").args(["get", "org.gnome.desktop.interface", "enable-animations"]).output().map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default();
        reduce_from(forced.as_deref(), gtk.as_deref(), &setting)
    })
}

/// The decision itself: an explicit `LASTHARNESS_REDUCE_MOTION` wins, then `GTK_ENABLE_ANIMATIONS=0`, then the
/// desktop's `enable-animations` setting saying `false`.
pub fn reduce_from(forced: Option<&str>, gtk: Option<&str>, setting: &str) -> bool {
    match forced {
        Some("1") | Some("true") => return true,
        Some("0") | Some("false") => return false,
        _ => {}
    }
    gtk == Some("0") || setting.trim() == "false"
}

impl Config {
    pub fn from_args(args: impl IntoIterator<Item = String>) -> Result<Config, String> {
        let mut c = Config { base: std::env::var("LH_URL").unwrap_or_else(|_| "http://127.0.0.1:7842".into()), token_file: default_token_path(), prefs: crate::prefs::path(), themes: crate::theme::user_dir() };
        let mut a = args.into_iter();
        while let Some(x) = a.next() {
            let mut val = |name: &str| a.next().ok_or_else(|| format!("{name} needs a value"));
            match x.as_str() {
                "--base" | "--url" => c.base = val(&x)?,
                "--token-file" => c.token_file = PathBuf::from(val(&x)?),
                "--prefs" => c.prefs = PathBuf::from(val(&x)?),
                "--themes" => c.themes = PathBuf::from(val(&x)?),
                other => return Err(format!("unknown option {other}. Options: --base URL, --token-file PATH, --prefs FILE, --themes FOLDER")),
            }
        }
        c.base = c.base.trim_end_matches('/').to_string();
        Ok(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn reduced_motion_follows_the_override_then_the_desktop_setting() {
        assert!(!reduce_from(None, None, "true\n"), "animations are on");
        assert!(reduce_from(None, None, "false\n"), "the desktop turned animations off");
        assert!(reduce_from(None, Some("0"), ""));
        assert!(reduce_from(Some("1"), None, "true"), "the override can force it on");
        assert!(!reduce_from(Some("0"), None, "false"), "and force it off");
        assert!(!reduce_from(None, None, ""), "no setting at all means normal motion");
    }

    #[test]
    fn flags_override_defaults_and_mistakes_are_explained() {
        let c = Config::from_args(args(&["--base", "http://127.0.0.1:9000/", "--token-file", "/t/token", "--prefs", "/p/gui.toml", "--themes", "/th"])).unwrap();
        assert_eq!(c.base, "http://127.0.0.1:9000");
        assert_eq!(c.token_file, PathBuf::from("/t/token"));
        assert_eq!(c.prefs, PathBuf::from("/p/gui.toml"));
        assert_eq!(c.themes, PathBuf::from("/th"));
        assert!(Config::from_args(args(&["--base"])).unwrap_err().contains("needs a value"));
        assert!(Config::from_args(args(&["--nope"])).unwrap_err().contains("unknown option"));
        assert!(Config::from_args(args(&[])).unwrap().token_file.ends_with("lastharness/token"));
    }
}
