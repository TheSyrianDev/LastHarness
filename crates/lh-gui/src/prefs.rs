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
    /// The page of the Settings window that was open last, by `Category::id`. Empty or unknown means the first.
    pub settings_page: String,
    /// Web pages are shown as a card with a button for the browser, not drawn inside the app. Off by default.
    pub pages_off: bool,
    /// The font for Latin text. Empty means the default of the window.
    pub latin_font: String,
    /// The font for Arabic text. Empty means Noto Naskh Arabic, or Noto Sans Arabic if that is missing.
    pub arabic_font: String,
    /// The name the new-chat screen greets. Empty means the real name from the system.
    pub name: String,
    /// Chats pinned to the top of the sidebar, by session id.
    pub pinned: Vec<String>,
    /// Snoozed chats by session id, with the Unix second they come back.
    pub snoozed: std::collections::BTreeMap<String, i64>,
    /// Days without a message before a chat settles into Earlier. 0 means the default, 3.
    pub settle_days: u32,
    /// The model on each stop of the model slider, fastest first; empty means filled for you.
    pub tiers: Vec<String>,
    /// How many stops the model slider has, 2 to 7. 0 means the default, 5 (an older file has no count and gets 5).
    pub stop_count: usize,
    /// The title of each stop of the model slider. Empty, or a blank entry, means the default title for that stop.
    pub stop_titles: Vec<String>,
    /// Up to three colours ("#rrggbb") the Glass look is tinted around; empty leaves it as built.
    pub glass_colours: Vec<String>,
    /// How strongly the Glass colours show, 0 to 1. None means 0.5.
    pub glass_opacity: Option<f32>,
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
        prefs.layout.tidy();
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

    #[test]
    fn the_width_and_presets_survive_a_restart_and_a_file_from_the_last_version_migrates() {
        use crate::layout::{ColumnWidth, Preset};
        let dir = std::env::temp_dir().join(format!("lh-gui-prefs-width-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("gui.toml");
        // the last version wrote a word under [layout]
        for (word, want) in [("comfortable", ColumnWidth::Px(700.)), ("wide", ColumnWidth::Px(860.)), ("full", ColumnWidth::Full)] {
            std::fs::write(&p, format!("[layout]\ncolumn = \"{word}\"\n")).unwrap();
            assert_eq!(Prefs::load(&p).layout.column, want, "{word}");
        }
        // a number and two presets are written and read back unchanged
        let mut a = Prefs::default();
        a.layout.column = ColumnWidth::Px(980.);
        a.layout.save_preset("Reading").unwrap();
        a.layout.column = ColumnWidth::Full;
        a.layout.save_preset("All").unwrap();
        a.save(&p).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.contains("[[layout.presets]]"), "{text}");
        let b = Prefs::load(&p);
        assert_eq!(b, a);
        assert_eq!(b.layout.presets, vec![Preset { name: "Reading".into(), width: ColumnWidth::Px(980.) }, Preset { name: "All".into(), width: ColumnWidth::Full }]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn odd_or_old_files_never_stop_the_app_from_starting() {
        use crate::layout::ColumnWidth;
        let dir = std::env::temp_dir().join(format!("lh-gui-prefs-odd-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("gui.toml");
        let missing = dir.join("nothing-here.toml");
        assert_eq!(Prefs::load(&missing), Prefs::default());
        let cases: [(&str, Option<ColumnWidth>); 14] = [
            ("", Some(ColumnWidth::Px(860.))),
            ("[layout]\ncolumn = \"wide\"\n", Some(ColumnWidth::Px(860.))),
            ("[layout]\ncolumn = \"comfortable\"\n", Some(ColumnWidth::Px(700.))),
            ("[layout]\ncolumn = \"nonsense\"\n", None),
            ("[layout]\ncolumn = -5\n", Some(ColumnWidth::Px(480.))),
            ("[layout]\ncolumn = 0\n", Some(ColumnWidth::Px(480.))),
            ("[layout]\ncolumn = 999999999999\n", Some(ColumnWidth::Px(999999995904.))),
            ("[layout]\ncolumn = 1e300\n", Some(ColumnWidth::Px(f32::MAX))),
            ("[layout]\ncolumn = nan\n", Some(ColumnWidth::Px(860.))),
            ("[layout]\ncolumn = true\n", None),
            ("[[layout.presets]]\nname = \"x\"\n", None),
            ("[[layout.presets]]\nname = \"x\"\nwidth = \"wide\"\n", Some(ColumnWidth::Px(860.))),
            ("[layout]\npresets = 5\n", None),
            ("\u{0}\u{1}binary", None),
        ];
        for (text, want) in cases {
            std::fs::write(&p, text).unwrap();
            let got = Prefs::load(&p);
            match want {
                Some(w) => match (got.layout.column, w) {
                    (ColumnWidth::Px(a), ColumnWidth::Px(b)) if b == f32::MAX => assert!(a.is_finite() && a >= 480., "{text:?}"),
                    (a, b) => assert_eq!(a, b, "{text:?}"),
                },
                None => assert_eq!(got, Prefs::default(), "{text:?} gives the defaults"),
            }
            // whatever was read, the controls can be built from it
            let room = 1200.;
            let pos = got.layout.column.position(room);
            assert!((480. ..=crate::layout::slider_max(room)).contains(&pos));
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_last_settings_page_is_remembered() {
        use crate::setting_names::Category;
        let dir = std::env::temp_dir().join(format!("lh-gui-prefs-page-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("gui.toml");
        assert_eq!(Category::from_id(&Prefs::load(&p).settings_page), Category::Appearance, "no file, first page");
        let a = Prefs { settings_page: Category::Spending.id().into(), ..Default::default() };
        a.save(&p).unwrap();
        assert_eq!(Category::from_id(&Prefs::load(&p).settings_page), Category::Spending);
        std::fs::write(&p, "settings_page = \"a page from the future\"\n").unwrap();
        assert_eq!(Category::from_id(&Prefs::load(&p).settings_page), Category::Appearance, "unknown falls back");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn showing_web_pages_inside_the_app_is_on_unless_switched_off_and_the_choice_is_kept() {
        assert!(!Prefs::default().pages_off, "on by default");
        let dir = std::env::temp_dir().join(format!("lh-gui-prefs-pages-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("gui.toml");
        assert!(!Prefs::load(&p).pages_off, "no file, on");
        std::fs::write(&p, "theme = \"Calm\"\n").unwrap();
        assert!(!Prefs::load(&p).pages_off, "an older file without the key, on");
        let a = Prefs { pages_off: true, ..Default::default() };
        a.save(&p).unwrap();
        assert!(Prefs::load(&p).pages_off, "off is kept");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The Windows 7 glass look was tried and taken out on 2026-10-13. A saved choice of it must not crash anything:
    /// the app uses the first look in the mode that was saved, and says so once.
    #[test]
    fn a_saved_choice_of_the_removed_look_falls_back_to_calm_in_the_same_mode_with_one_notice() {
        let dir = std::env::temp_dir().join(format!("lh-gui-prefs-removed-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("gui.toml");
        let set = crate::theme::ThemeSet::load(std::path::Path::new("/no/such/folder"));
        let families = set.families();
        for (saved, mode_line, dark) in [("theme = \"Frutiger Aero dark\"\n", "", true), ("theme = \"Frutiger Aero\"\nmode = \"dark\"\n", "", true), ("theme = \"Frutiger Aero light\"\n", "", false), ("theme = \"Frutiger Aero\"\nmode = \"system\"\n", "", false)] {
            std::fs::write(&p, format!("{saved}{mode_line}model = \"Q\"\n")).unwrap();
            let prefs = Prefs::load(&p);
            assert_eq!(prefs.theme, "Frutiger Aero", "the saved word is read as it is, no crash: {saved:?}");
            // what the app does at start: no such look, so the first one, in the saved mode, and one line of notice
            let notice = crate::theme::missing_look_notice(&prefs.theme, &families).expect("a notice");
            assert!(notice.contains("Frutiger Aero") && notice.contains("Calm") && notice.lines().count() == 1, "{notice}");
            let i = set.resolve("", dark);
            assert_eq!(set.themes[i].label(), if dark { "Calm dark" } else { "Calm light" }, "{saved:?}");
            assert_eq!(set.themes[set.resolve(&prefs.theme, dark)].label(), set.themes[i].label(), "resolving the saved word gives the same look");
        }
        // nothing saved, or a look that is there, gives no notice
        assert_eq!(crate::theme::missing_look_notice("", &families), None);
        assert_eq!(crate::theme::missing_look_notice("Matte", &families), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_two_fonts_default_to_empty_and_are_kept() {
        let dir = std::env::temp_dir().join(format!("lh-gui-prefs-fonts-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("gui.toml");
        let d = Prefs::default();
        assert!((d.latin_font.as_str(), d.arabic_font.as_str()) == ("", ""));
        std::fs::write(&p, "model = \"Q\"\n").unwrap();
        assert_eq!(Prefs::load(&p).latin_font, "", "an older file without the keys");
        let a = Prefs { latin_font: "Inter".into(), arabic_font: "Noto Kufi Arabic".into(), ..Default::default() };
        a.save(&p).unwrap();
        assert_eq!(Prefs::load(&p), a);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_slider_count_and_titles_survive_and_an_old_file_keeps_five_stops() {
        let dir = std::env::temp_dir().join(format!("lh-gui-stops-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("gui.toml");
        // an old file: five fixed stops, no count, no titles
        std::fs::write(&p, "tiers = [\"a\", \"\", \"b\", \"\", \"c\"]\n").unwrap();
        let old = Prefs::load(&p);
        assert_eq!(old.stop_count, 0, "no count in the file means the default");
        assert_eq!(crate::tiers::clamp_count(old.stop_count), 5);
        assert_eq!(old.tiers.len(), 5, "the old five stops load unchanged");
        assert!(old.stop_titles.is_empty());
        let a = Prefs { stop_count: 3, stop_titles: vec!["Slow".into(), String::new(), "Fast".into()], ..Default::default() };
        a.save(&p).unwrap();
        assert_eq!(Prefs::load(&p), a);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
