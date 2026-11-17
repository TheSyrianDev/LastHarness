//! Reading the Omarchy theme in use, for the service that hands it to the
//! website. The files are the same ones the desktop app reads:
//! `~/.local/state/omarchy/current/theme/colors.toml` and the `theme.name` beside
//! it. Nothing is written, nothing needs a network.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// One theme, as the website needs it: the name, light or dark, and the colour
/// keys from `colors.toml` the site knows how to use.
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub name: String,
    pub mode: String,
    pub palette: Vec<(String, String)>,
}

impl Theme {
    /// The JSON the site reads at `GET /theme.json`.
    pub fn json(&self) -> String {
        let mut palette: Vec<String> = Vec::new();
        for (key, value) in &self.palette {
            palette.push(format!("\"{}\":\"{}\"", escape(key), escape(value)));
        }
        format!(
            "{{\"name\":\"{}\",\"mode\":\"{}\",\"palette\":{{{}}}}}",
            escape(&self.name),
            escape(&self.mode),
            palette.join(",")
        )
    }

    /// What a change looks like: the name, and when the files were last written.
    fn stamp(&self, files: &Files) -> Stamp {
        Stamp {
            name: self.name.clone(),
            written: files.written(),
        }
    }
}

/// Where the theme files are, so a change can be noticed without re-parsing.
pub struct Files {
    pub theme_dir: PathBuf,
    pub name_file: PathBuf,
}

impl Files {
    fn written(&self) -> Vec<(SystemTime, u64)> {
        let one = |path: &Path| {
            std::fs::metadata(path)
                .ok()
                .map(|m| (m.modified().unwrap_or(SystemTime::UNIX_EPOCH), m.len()))
        };
        [one(&self.theme_dir.join("colors.toml")), one(&self.name_file)]
            .into_iter()
            .flatten()
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stamp {
    pub name: String,
    pub written: Vec<(SystemTime, u64)>,
}

/// Where Omarchy keeps the current theme, newest layout first: `$XDG_STATE_HOME`
/// (or `~/.local/state`), then the older `$XDG_CONFIG_HOME` location. Tests pass
/// their own roots; the service passes the real ones.
pub fn current_dir(state_home: Option<PathBuf>, config_home: Option<PathBuf>) -> Option<Files> {
    let mut roots = Vec::new();
    if let Some(state) = state_home {
        roots.push(state.join("omarchy/current"));
    }
    if let Some(config) = config_home {
        roots.push(config.join("omarchy/current"));
    }
    roots
        .into_iter()
        .map(|root| Files {
            theme_dir: root.join("theme"),
            name_file: root.join("theme.name"),
        })
        .find(|files| files.theme_dir.join("colors.toml").is_file())
}

/// The current theme, or `None` when there is no Omarchy theme on this machine.
pub fn read(files: &Files) -> Option<Theme> {
    let source = std::fs::read_to_string(files.theme_dir.join("colors.toml")).ok()?;
    let palette = parse(&source);
    let background = value(&palette, "background")?;
    let foreground = value(&palette, "foreground")?;
    // Two colours the site cannot do without; a file with neither is not a theme.
    let _ = (background, foreground);

    let name = std::fs::read_to_string(&files.name_file)
        .ok()
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .or_else(|| {
            files
                .theme_dir
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "Omarchy".to_string());

    let mode = value(&palette, "mode").unwrap_or_else(|| "dark".to_string());

    Some(Theme {
        name,
        mode,
        palette,
    })
}

/// The theme plus the stamp of the files it was read from.
pub fn snapshot(files: &Files) -> Option<(Theme, Stamp)> {
    let theme = read(files)?;
    let stamp = theme.stamp(files);
    Some((theme, stamp))
}

/// `colors.toml` is flat: `key = "value"`, comments on their own lines. Unknown
/// keys are kept so a newer theme still reaches the site whole.
pub fn parse(source: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
            continue;
        }
        let Some((key, rest)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let rest = rest.trim();
        let value = match rest.strip_prefix('"') {
            Some(quoted) => quoted.split('"').next().unwrap_or(""),
            None => rest.split_whitespace().next().unwrap_or(""),
        };
        if !key.is_empty() && !value.is_empty() {
            out.push((key.to_string(), value.to_string()));
        }
    }
    out
}

fn value(palette: &[(String, String)], key: &str) -> Option<String> {
    palette
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.clone())
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' | '\r' | '\t' => out.push(' '),
            c if (c as u32) < 0x20 => {}
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const DARK: &str = r##"mode = "dark"

accent = "#89b4fa"
selection = "#45475a"
background = "#1e1e2e"
foreground = "#cdd6f4"
orange = "#f6b6ab"
green = "#a6e3a1"

# a comment, and a colour with a hash in it
blue = "#89b4fa"
"##;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("lastai-theme-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("omarchy/current/theme")).unwrap();
        dir
    }

    #[test]
    fn a_theme_file_becomes_name_mode_and_colours() {
        let palette = parse(DARK);
        assert_eq!(value(&palette, "background").as_deref(), Some("#1e1e2e"));
        assert_eq!(value(&palette, "orange").as_deref(), Some("#f6b6ab"));
        // the hash inside a value is kept, the whole-line comment is not
        assert!(palette.iter().all(|(k, _)| k != "#"));
        assert_eq!(palette.len(), 8);
    }

    #[test]
    fn the_newest_theme_folder_wins_and_the_name_comes_from_theme_name() {
        let root = scratch("resolve");
        let state = root.join("state");
        std::fs::create_dir_all(state.join("omarchy/current/theme")).unwrap();
        std::fs::write(state.join("omarchy/current/theme/colors.toml"), DARK).unwrap();
        std::fs::write(state.join("omarchy/current/theme.name"), "catppuccin\n").unwrap();

        let files = current_dir(Some(state.clone()), Some(root.join("config"))).expect("found");
        let theme = read(&files).expect("read");
        assert_eq!(theme.name, "catppuccin");
        assert_eq!(theme.mode, "dark");
        assert!(theme.json().contains("\"palette\":{"));
        assert!(theme.json().starts_with("{\"name\":\"catppuccin\""));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_file_without_a_background_is_not_a_theme() {
        let root = scratch("bad");
        let state = root.join("state");
        std::fs::create_dir_all(state.join("omarchy/current/theme")).unwrap();
        std::fs::write(
            state.join("omarchy/current/theme/colors.toml"),
            "accent = \"#ffffff\"\n",
        )
        .unwrap();
        let files = current_dir(Some(state), None).expect("found");
        assert!(read(&files).is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_name_with_a_quote_cannot_break_the_json() {
        let theme = Theme {
            name: "Bob's \"theme\"".into(),
            mode: "dark".into(),
            palette: vec![("background".into(), "#000000".into())],
        };
        let json = theme.json();
        assert!(json.contains(r#""name":"Bob's \"theme\"""#));
        assert!(json.ends_with("}"));
    }
}
