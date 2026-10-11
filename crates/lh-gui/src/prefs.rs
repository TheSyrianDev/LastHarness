//! What the app remembers between runs: the look, the layout, the language direction and the model last used.
//! A small TOML file in the config folder, written whenever one of them changes.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::layout::Layout;
use crate::rtl::Direction;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    /// A theme label such as "Calm light". Empty means the default.
    pub theme: String,
    pub direction: Direction,
    pub layout: Layout,
    pub model: String,
}

pub fn path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config"))).unwrap_or_default();
    base.join("lastharness").join("gui.toml")
}

impl Prefs {
    /// A missing or unreadable file gives the defaults; the app must always start.
    pub fn load(p: &Path) -> Prefs {
        std::fs::read_to_string(p).ok().and_then(|s| toml::from_str(&s).ok()).unwrap_or_default()
    }

    pub fn save(&self, p: &Path) -> std::io::Result<()> {
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::write(p, toml::to_string_pretty(self).unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::Panel;

    #[test]
    fn preferences_survive_a_restart_and_a_bad_file_gives_the_defaults() {
        let dir = std::env::temp_dir().join(format!("lh-gui-prefs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let p = dir.join("sub").join("gui.toml");
        assert_eq!(Prefs::load(&p), Prefs::default(), "no file yet");
        let mut a = Prefs { theme: "Glass dark".into(), direction: Direction::Rtl, model: "Qwen".into(), ..Default::default() };
        a.layout.show_panel(Panel::Memory);
        a.layout.resize_left(300.);
        a.save(&p).unwrap();
        assert_eq!(Prefs::load(&p), a);
        std::fs::write(&p, "this is [not toml").unwrap();
        assert_eq!(Prefs::load(&p), Prefs::default());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
