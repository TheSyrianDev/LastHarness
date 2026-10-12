//! Light or dark. A look is one family (Calm, Matte, Glass, Frutiger Aero) with a light and a dark variant, and one
//! switch picks the variant for every family: Light, Dark, or follow the desktop. No window code here.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModeChoice {
    #[default]
    Light,
    Dark,
    /// Whatever the desktop says; light when it says nothing.
    System,
}

impl ModeChoice {
    pub const ALL: [ModeChoice; 3] = [ModeChoice::Light, ModeChoice::Dark, ModeChoice::System];

    pub fn label(self, rtl: bool) -> &'static str {
        match (self, rtl) {
            (ModeChoice::Light, false) => "Light",
            (ModeChoice::Dark, false) => "Dark",
            (ModeChoice::System, false) => "Follow the system",
            (ModeChoice::Light, true) => "فاتح",
            (ModeChoice::Dark, true) => "داكن",
            (ModeChoice::System, true) => "اتباع النظام",
        }
    }
}

/// What the desktop's `color-scheme` setting says: `Some(true)` for dark, `Some(false)` for light, `None` when it
/// says nothing usable (no such setting, or an unknown word).
pub fn parse_color_scheme(setting: &str) -> Option<bool> {
    match setting.trim().trim_matches('\'') {
        "prefer-dark" => Some(true),
        "prefer-light" | "default" => Some(false),
        _ => None,
    }
}

/// Ask the desktop whether it prefers dark. Read with `gsettings`, which GNOME-family desktops and most others
/// (Omarchy, Hyprland with the portal) keep up to date. `None` when the desktop does not say.
pub fn system_dark() -> Option<bool> {
    let o = std::process::Command::new("gsettings").args(["get", "org.gnome.desktop.interface", "color-scheme"]).output().ok()?;
    o.status.success().then(|| parse_color_scheme(&String::from_utf8_lossy(&o.stdout))).flatten()
}

/// Is the dark variant in use? `system` is what the desktop said, if it said anything.
pub fn is_dark(choice: ModeChoice, system: Option<bool>) -> bool {
    match choice {
        ModeChoice::Light => false,
        ModeChoice::Dark => true,
        ModeChoice::System => system.unwrap_or(false),
    }
}

/// The light/dark key: swap to the other variant, as an explicit choice (so it stops following the desktop).
pub fn toggled(choice: ModeChoice, system: Option<bool>) -> ModeChoice {
    if is_dark(choice, system) { ModeChoice::Light } else { ModeChoice::Dark }
}

/// An old saved look such as "Calm dark" was a family and a mode in one word. Split it.
pub fn split_legacy(label: &str) -> Option<(String, ModeChoice)> {
    if let Some(f) = label.strip_suffix(" dark") {
        Some((f.to_string(), ModeChoice::Dark))
    } else {
        label.strip_suffix(" light").map(|f| (f.to_string(), ModeChoice::Light))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_desktop_setting_is_read_and_unknown_words_say_nothing() {
        assert_eq!(parse_color_scheme("'prefer-dark'\n"), Some(true));
        assert_eq!(parse_color_scheme("'prefer-light'\n"), Some(false));
        assert_eq!(parse_color_scheme("'default'"), Some(false), "no preference reads as light");
        assert_eq!(parse_color_scheme(""), None);
        assert_eq!(parse_color_scheme("No such key"), None);
    }

    #[test]
    fn the_mode_in_use_follows_the_choice_and_only_system_listens_to_the_desktop() {
        assert!(!is_dark(ModeChoice::Light, Some(true)));
        assert!(is_dark(ModeChoice::Dark, Some(false)));
        assert!(is_dark(ModeChoice::System, Some(true)));
        assert!(!is_dark(ModeChoice::System, Some(false)));
        assert!(!is_dark(ModeChoice::System, None), "a desktop that says nothing means light");
    }

    #[test]
    fn the_key_flips_what_is_showing_and_becomes_an_explicit_choice() {
        assert_eq!(toggled(ModeChoice::Light, None), ModeChoice::Dark);
        assert_eq!(toggled(ModeChoice::Dark, None), ModeChoice::Light);
        assert_eq!(toggled(ModeChoice::System, Some(true)), ModeChoice::Light, "following a dark desktop, the key goes light");
        assert_eq!(toggled(ModeChoice::System, Some(false)), ModeChoice::Dark);
    }

    #[test]
    fn old_saved_looks_split_into_family_and_mode() {
        assert_eq!(split_legacy("Calm dark"), Some(("Calm".into(), ModeChoice::Dark)));
        assert_eq!(split_legacy("Frutiger Aero light"), Some(("Frutiger Aero".into(), ModeChoice::Light)));
        assert_eq!(split_legacy("Calm"), None);
    }

    #[test]
    fn the_choice_is_saved_as_a_word() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct W {
            mode: ModeChoice,
        }
        for (m, word) in [(ModeChoice::Light, "light"), (ModeChoice::Dark, "dark"), (ModeChoice::System, "system")] {
            let s = toml::to_string(&W { mode: m }).unwrap();
            assert!(s.contains(&format!("\"{word}\"")), "{s}");
            assert_eq!(toml::from_str::<W>(&s).unwrap().mode, m);
        }
    }
}
