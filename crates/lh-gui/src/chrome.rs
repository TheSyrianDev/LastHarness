//! The frame round the chat: the slim icon rail on the side and what the title bar says. Kept apart from the
//! drawing so the labels, the pressed states, the icon shapes and the wording can be tested without a window.

use crate::layout::{Layout, Panel};

/// Width of the icon rail in pixels.
pub const RAIL_WIDTH: f32 = 52.;

/// What the rail offers. Chats shows or hides the chat list; the others open the side panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RailItem {
    Chats,
    Departments,
    Memory,
    Settings,
}

impl RailItem {
    pub const ALL: [RailItem; 4] = [RailItem::Chats, RailItem::Departments, RailItem::Memory, RailItem::Settings];

    /// The name a screen reader says and a tooltip shows.
    pub fn label(self, rtl: bool) -> &'static str {
        match (self, rtl) {
            (RailItem::Chats, false) => "Chats",
            (RailItem::Chats, true) => "المحادثات",
            (RailItem::Departments, false) => "Departments",
            (RailItem::Departments, true) => "الأقسام",
            (RailItem::Memory, false) => "Memory",
            (RailItem::Memory, true) => "الذاكرة",
            (RailItem::Settings, false) => "Settings",
            (RailItem::Settings, true) => "الإعدادات",
        }
    }

    /// The key that does the same thing.
    pub fn shortcut(self) -> &'static str {
        match self {
            RailItem::Chats => "Ctrl+B",
            RailItem::Departments => "Ctrl+Shift+D",
            RailItem::Memory => "Ctrl+Shift+M",
            RailItem::Settings => "Ctrl+Shift+S",
        }
    }

    /// The label with its key, as a screen reader reads it.
    pub fn aria(self, rtl: bool) -> String {
        format!("{} ({})", self.label(rtl), self.shortcut())
    }

    /// The side panel this item opens; `None` for the chat list.
    pub fn panel(self) -> Option<Panel> {
        match self {
            RailItem::Chats => None,
            RailItem::Departments => Some(Panel::Departments),
            RailItem::Memory => Some(Panel::Memory),
            RailItem::Settings => Some(Panel::Settings),
        }
    }

    /// Is the thing this item opens showing now?
    pub fn pressed(self, l: &Layout) -> bool {
        match self.panel() {
            None => l.left_visible,
            Some(p) => l.right_visible && l.panel == p,
        }
    }

    /// The icon as lines on a 24 by 24 grid, each a list of points to join. Simple outlines, drawn by the app.
    pub fn icon(self) -> Vec<Vec<(f32, f32)>> {
        match self {
            // a speech bubble
            RailItem::Chats => vec![vec![(4., 5.), (20., 5.), (20., 16.), (9., 16.), (4., 20.), (4., 5.)]],
            // one box above two: a lead and its members
            RailItem::Departments => vec![circle(12., 5., 2.), circle(5., 19., 2.), circle(19., 19., 2.), vec![(12., 7.), (12., 12.), (6., 17.)], vec![(12., 12.), (18., 17.)]],
            // a page with two lines
            RailItem::Memory => vec![vec![(5., 4.), (16., 4.), (19., 7.), (19., 20.), (5., 20.), (5., 4.)], vec![(8., 11.), (16., 11.)], vec![(8., 15.), (16., 15.)]],
            // a ring with eight short spokes
            RailItem::Settings => {
                let mut v = vec![circle(12., 12., 3.)];
                for k in 0..8 {
                    let a = k as f32 * std::f32::consts::FRAC_PI_4;
                    let (s, c) = a.sin_cos();
                    v.push(vec![(12. + 6. * c, 12. + 6. * s), (12. + 9. * c, 12. + 9. * s)]);
                }
                v
            }
        }
    }
}

/// The tabs along the top of the side panel, in order. Settings has no tab: the rail opens it.
pub const DOCK_TABS: [Panel; 4] = [Panel::Content, Panel::Departments, Panel::Memory, Panel::Tree];

/// The name of a side panel, for its tab and for a screen reader.
pub fn panel_label(p: Panel, rtl: bool) -> &'static str {
    match (p, rtl) {
        (Panel::Content, false) => "Opened",
        (Panel::Content, true) => "المفتوح",
        (Panel::Departments, false) => "Departments",
        (Panel::Departments, true) => "الأقسام",
        (Panel::Memory, false) => "Memory",
        (Panel::Memory, true) => "الذاكرة",
        (Panel::Tree, false) => "Tree",
        (Panel::Tree, true) => "الشجرة",
        (Panel::Settings, false) => "Settings",
        (Panel::Settings, true) => "الإعدادات",
    }
}

/// A closed line round a circle.
fn circle(cx: f32, cy: f32, r: f32) -> Vec<(f32, f32)> {
    (0..=16).map(|k| {
        let a = k as f32 * std::f32::consts::TAU / 16.;
        (cx + r * a.cos(), cy + r * a.sin())
    }).collect()
}

/// A whole number with thousands separators: 4120 is "4,120".
pub fn thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// What the title bar says. Only the chat's own state: its name, which branch it is on, how much the model has
/// read, and which model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TitleInfo {
    pub title: String,
    pub branch: Option<String>,
    pub context: Option<String>,
    pub model: Option<String>,
}

impl TitleInfo {
    /// `branches` is `ChatTree::branches()`: how many ends the tree has and which one the chat is at.
    pub fn chat(title: &str, model: &str, tokens: Option<u64>, branches: Option<(usize, Option<usize>)>, rtl: bool) -> TitleInfo {
        let branch = branches.map(|(total, at)| match (at, rtl) {
            (Some(i), false) => format!("branch {i} of {total}"),
            (Some(i), true) => format!("الفرع {i} من {total}"),
            (None, false) => format!("{total} branches"),
            (None, true) => format!("{total} فروع"),
        });
        let context = tokens.map(|n| if rtl { format!("{} رمز", thousands(n)) } else { format!("{} tokens", thousands(n)) });
        let model = Some(if model.is_empty() { if rtl { "لا نموذج".to_string() } else { "no model".to_string() } } else { model.to_string() });
        TitleInfo { title: title.to_string(), branch, context, model }
    }

    /// The bar over a department's page: just its name, since the chat's model and branch do not apply.
    pub fn page(name: &str) -> TitleInfo {
        TitleInfo { title: name.to_string(), branch: None, context: None, model: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rail_has_four_items_each_with_a_name_a_key_and_a_picture() {
        assert_eq!(RailItem::ALL.len(), 4);
        for rtl in [false, true] {
            let names: Vec<&str> = RailItem::ALL.iter().map(|i| i.label(rtl)).collect();
            assert!(names.iter().all(|n| !n.is_empty()));
            let mut sorted = names.clone();
            sorted.sort();
            sorted.dedup();
            assert_eq!(sorted.len(), 4, "no two items share a name");
        }
        for i in RailItem::ALL {
            assert!(i.aria(false).contains(i.shortcut()), "a screen reader hears the key too");
            let icon = i.icon();
            assert!(!icon.is_empty() && icon.iter().all(|line| line.len() >= 2));
            for (x, y) in icon.iter().flatten() {
                assert!((0. ..=24.).contains(x) && (0. ..=24.).contains(y), "{i:?} stays on the grid: {x},{y}");
            }
        }
        assert_ne!(RailItem::Memory.icon(), RailItem::Chats.icon());
    }

    #[test]
    fn the_rail_shows_which_things_are_open() {
        let mut l = Layout::default();
        assert!(RailItem::Chats.pressed(&l) && !RailItem::Memory.pressed(&l), "the chat list starts open, the side panel closed");
        l.toggle_left();
        assert!(!RailItem::Chats.pressed(&l));
        l.toggle_panel(Panel::Memory);
        assert!(RailItem::Memory.pressed(&l) && !RailItem::Departments.pressed(&l) && !RailItem::Settings.pressed(&l));
        l.toggle_panel(Panel::Tree);
        assert!(RailItem::ALL.iter().all(|i| i.panel().is_none() || !i.pressed(&l)), "the branches tab is not on the rail, so no item looks pressed");
        l.hide_right();
        assert!(!RailItem::Memory.pressed(&l), "a hidden panel is not pressed");
        assert_eq!(RailItem::Chats.panel(), None);
        assert_eq!(RailItem::Settings.panel(), Some(Panel::Settings));
    }

    #[test]
    fn the_side_panel_has_a_tab_for_everything_the_rail_does_not_reach() {
        for rtl in [false, true] {
            let mut names: Vec<&str> = DOCK_TABS.iter().map(|p| panel_label(*p, rtl)).collect();
            names.sort();
            names.dedup();
            assert_eq!(names.len(), 4);
            // the rail and the tabs call the same panel the same thing
            for item in RailItem::ALL {
                if let Some(p) = item.panel() {
                    assert_eq!(item.label(rtl), panel_label(p, rtl));
                }
            }
        }
        for p in [Panel::Departments, Panel::Memory, Panel::Settings, Panel::Content, Panel::Tree] {
            assert!(DOCK_TABS.contains(&p) || p == Panel::Settings, "{p:?} can be reached");
        }
    }

    #[test]
    fn numbers_get_separators() {
        assert_eq!([0, 7, 999, 1000, 4120, 65432, 1234567].map(thousands), ["0", "7", "999", "1,000", "4,120", "65,432", "1,234,567"]);
    }

    #[test]
    fn the_title_bar_says_only_what_the_chat_knows() {
        let t = TitleInfo::chat("Plan the trip", "Qwen3.6 35B", Some(4120), Some((3, Some(2))), false);
        assert_eq!((t.branch.as_deref(), t.context.as_deref(), t.model.as_deref()), (Some("branch 2 of 3"), Some("4,120 tokens"), Some("Qwen3.6 35B")));
        let t = TitleInfo::chat("x", "", None, None, false);
        assert_eq!((t.branch, t.context, t.model.as_deref()), (None, None, Some("no model")), "one line of talk shows no branch, and nothing is invented for the context");
        assert_eq!(TitleInfo::chat("x", "m", None, Some((4, None)), false).branch.as_deref(), Some("4 branches"));
        let ar = TitleInfo::chat("رسالة", "m", Some(1500), Some((3, Some(1))), true);
        assert_eq!((ar.branch.as_deref(), ar.context.as_deref()), (Some("الفرع 1 من 3"), Some("1,500 رمز")));
        assert_eq!(TitleInfo::page("Weekly review"), TitleInfo { title: "Weekly review".into(), branch: None, context: None, model: None });
    }
}
