//! Every icon the app draws goes through one lookup: the theme's own SVG when it names one, else the app's built-in
//! line drawing. Kept apart from the drawing so the choice can be tested without a window.

use std::collections::BTreeMap;
use std::sync::Arc;

/// Lines on a 24 by 24 grid, each a list of points to join.
pub type Lines = Vec<Vec<(f32, f32)>>;

/// An icon by name, with the built-in drawing to fall back on. `mirror`: the theme's SVG is flipped left to right
/// (the right-to-left form of a sidebar or panel icon), since a theme draws each icon once.
#[derive(Clone, Debug, PartialEq)]
pub struct Icon {
    pub name: &'static str,
    pub lines: Lines,
    pub mirror: bool,
}

impl Icon {
    pub fn new(name: &'static str, lines: Lines) -> Icon {
        Icon { name, lines, mirror: false }
    }
    pub fn mirrored(mut self, mirror: bool) -> Icon {
        self.mirror = mirror;
        self
    }
}

/// The names a theme can give its own SVG for, one per icon the app draws.
pub const NAMES: &[&str] = &[
    // the sidebar and the bars
    "new-chat", "search", "send", "attach", "stop", "sidebar-collapse", "sidebar-expand", "lock", "panel", "close",
    "widen", "popout", "chevron-right", "chevron-left", "chevron-down",
    // the rail and the pages of Settings
    "chats", "departments", "memory", "settings", "appearance", "models", "learning", "permissions", "spending", "web",
    "tools", "channels", "data", "activity", "hooks", "schedules", "updates", "other",
];

/// The SVGs a theme brings, by icon name.
pub type IconSet = BTreeMap<String, Arc<[u8]>>;

/// What to draw for `icon`: the theme's SVG when it has one by that name, else `None` for the built-in lines.
pub fn resolve<'a>(icon: &Icon, set: &'a IconSet) -> Option<&'a Arc<[u8]>> {
    set.get(icon.name)
}

/// Read the `[icons]` table of a theme: `folder` (every `<name>.svg` in it) and `<name> = "file.svg"` entries, which
/// win over the folder. Paths are relative to `dir`, the theme file's folder. An unknown name, a missing file or a
/// file that is not an SVG is skipped with a note; that icon keeps the built-in drawing.
pub fn load(table: &BTreeMap<String, String>, dir: &std::path::Path) -> (IconSet, Vec<String>) {
    let mut set = IconSet::new();
    let mut problems = vec![];
    let mut read = |name: &str, path: &std::path::Path, problems: &mut Vec<String>, quiet_missing: bool| match std::fs::read(path) {
        Ok(b) if String::from_utf8_lossy(&b[..b.len().min(4096)]).contains("<svg") => {
            set.insert(name.to_string(), Arc::from(b));
        }
        Ok(_) => problems.push(format!("icons.{name}: {} is not an SVG file", path.display())),
        Err(_) if quiet_missing => {}
        Err(e) => problems.push(format!("icons.{name}: cannot read {}: {e}", path.display())),
    };
    if let Some(folder) = table.get("folder") {
        let folder = dir.join(folder);
        if !folder.is_dir() {
            problems.push(format!("icons.folder: {} is not a folder", folder.display()));
        }
        for name in NAMES {
            read(name, &folder.join(format!("{name}.svg")), &mut problems, true);
        }
    }
    for (name, file) in table.iter().filter(|(k, _)| *k != "folder") {
        match NAMES.iter().find(|n| **n == name.as_str()) {
            Some(n) => read(n, &dir.join(file), &mut problems, false),
            None => problems.push(format!("icons.{name}: no icon has that name")),
        }
    }
    (set, problems)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M4 12h16" stroke="black"/></svg>"#;

    fn table(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
    }

    #[test]
    fn an_icon_the_theme_leaves_out_keeps_the_built_in_drawing() {
        let icon = Icon::new("send", vec![vec![(0., 0.), (1., 1.)]]);
        assert!(resolve(&icon, &IconSet::new()).is_none());
    }

    #[test]
    fn a_theme_icon_replaces_the_built_in_one_by_name_and_entries_win_over_the_folder() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir(d.path().join("icons")).unwrap();
        std::fs::write(d.path().join("icons/send.svg"), SVG).unwrap();
        std::fs::write(d.path().join("icons/settings.svg"), SVG).unwrap();
        std::fs::write(d.path().join("gear.svg"), SVG.replace("M4", "M5")).unwrap();
        let (set, problems) = load(&table(&[("folder", "icons"), ("settings", "gear.svg")]), d.path());
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(set.keys().collect::<Vec<_>>(), ["send", "settings"]);
        assert!(String::from_utf8_lossy(&set["settings"]).contains("M5"), "the named entry wins");
        let send = Icon::new("send", vec![]);
        assert!(resolve(&send, &set).is_some());
        assert!(resolve(&Icon::new("search", vec![]), &set).is_none(), "the rest fall back");
    }

    #[test]
    fn a_missing_or_wrong_file_or_name_falls_back_with_a_note() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("not.svg"), "hello").unwrap();
        let (set, problems) = load(&table(&[("send", "gone.svg"), ("stop", "not.svg"), ("rocket", "x.svg")]), d.path());
        assert!(set.is_empty());
        assert_eq!(problems.len(), 3, "{problems:?}");
        assert!(problems.iter().any(|p| p.starts_with("icons.rocket")));
    }

    #[test]
    fn every_rail_and_settings_icon_has_a_name_a_theme_can_use() {
        use crate::chrome::{category_icon_name, rail_icon_name, RailItem};
        for c in crate::setting_names::Category::ALL {
            assert!(NAMES.contains(&category_icon_name(c)), "{c:?}");
        }
        for i in [RailItem::Chats, RailItem::Departments, RailItem::Memory, RailItem::Settings] {
            assert!(NAMES.contains(&rail_icon_name(i)));
        }
    }

    #[test]
    fn every_name_is_unique() {
        let mut v = NAMES.to_vec();
        v.sort();
        v.dedup();
        assert_eq!(v.len(), NAMES.len());
    }
}
