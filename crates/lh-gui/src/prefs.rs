//! What the app remembers between runs: the look, the layout, the language direction and the model last used.
//! A small TOML file in the config folder, written whenever one of them changes.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::appearance::{split_legacy, ModeChoice};
use crate::layout::Layout;
use crate::rtl::Direction;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    /// The family of looks, such as "Calm". Empty means the first one.
    pub theme: String,
    /// Light, dark, or follow the desktop.
    pub mode: ModeChoice,
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
        let Some(text) = std::fs::read_to_string(p).ok() else { return Prefs::default() };
        let Ok(mut prefs) = toml::from_str::<Prefs>(&text) else { return Prefs::default() };
        // before light and dark became one switch, the look was saved as one word such as "Calm dark"
        let has_mode = text.parse::<toml::Table>().is_ok_and(|t| t.contains_key("mode"));
        if let (false, Some((family, mode))) = (has_mode, split_legacy(&prefs.theme)) {
            prefs.theme = family;
            prefs.mode = mode;
        }
        prefs
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
        let mut a = Prefs { theme: "Glass".into(), mode: ModeChoice::Dark, direction: Direction::Rtl, model: "Qwen".into(), ..Default::default() };
        a.layout.show_panel(Panel::Memory);
        a.layout.resize_left(300.);
        a.save(&p).unwrap();
        assert_eq!(Prefs::load(&p), a);
        std::fs::write(&p, "this is [not toml").unwrap();
        assert_eq!(Prefs::load(&p), Prefs::default());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_look_saved_the_old_way_becomes_a_family_and_a_mode() {
        let dir = std::env::temp_dir().join(format!("lh-gui-prefs-old-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("gui.toml");
        std::fs::write(&p, "theme = \"Frutiger Aero dark\"\nmodel = \"Q\"\n").unwrap();
        let a = Prefs::load(&p);
        assert_eq!((a.theme.as_str(), a.mode, a.model.as_str()), ("Frutiger Aero", ModeChoice::Dark, "Q"));
        // a file that already has a mode is left alone, even if the family name happens to end in a mode word
        std::fs::write(&p, "theme = \"Odd dark\"\nmode = \"light\"\n").unwrap();
        let b = Prefs::load(&p);
        assert_eq!((b.theme.as_str(), b.mode), ("Odd dark", ModeChoice::Light));
        // saved again, it is in the new form
        a.save(&p).unwrap();
        let c = Prefs::load(&p);
        assert_eq!((c.theme.as_str(), c.mode), ("Frutiger Aero", ModeChoice::Dark));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
