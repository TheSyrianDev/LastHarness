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
