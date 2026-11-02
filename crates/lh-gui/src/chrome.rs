//! The frame round the chat: the slim icon rail on the side and what the title bar says. Kept apart from the
//! drawing so the labels, the pressed states, the icon shapes and the wording can be tested without a window.

use crate::layout::{Layout, Panel};

/// Width of the icon rail in pixels.
pub const RAIL_WIDTH: f32 = 52.;

/// How wide the Settings window is, at most, in pixels.
pub const SETTINGS_WIDTH: f32 = 800.;

/// How much the rest of the window is darkened while Settings is up. This is a plain dimming, not a blur: the toolkit
/// cannot blur what is behind a panel. Strong enough that the page behind stops competing, a little stronger in a
/// dark look where the page itself is dark.
pub fn scrim_alpha(dark: bool) -> f32 {
    if dark { 0.65 } else { 0.55 }
}

/// What Escape closes first: the thing drawn on top.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    Palette,
    Settings,
    ModelMenu,
    ResumeQuestion,
    DeleteQuestion,
    Nothing,
}

/// Which open thing is on top, in the order they cover each other.
pub fn topmost(palette: bool, settings: bool, model_menu: bool, resume: bool, delete: bool) -> Layer {
    if palette {
        Layer::Palette
    } else if settings {
        Layer::Settings
    } else if model_menu {
        Layer::ModelMenu
    } else if resume {
        Layer::ResumeQuestion
    } else if delete {
        Layer::DeleteQuestion
    } else {
        Layer::Nothing
    }
}

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

    /// The side panel this item opens; `None` for the chat list and for Settings, which is a window of its own.
    pub fn panel(self) -> Option<Panel> {
        match self {
            RailItem::Chats | RailItem::Settings => None,
            RailItem::Departments => Some(Panel::Departments),
            RailItem::Memory => Some(Panel::Memory),
        }
    }

    /// Is the thing this item opens showing now? `settings_open` is whether the Settings window is up.
    pub fn pressed(self, l: &Layout, settings_open: bool) -> bool {
        match self {
            RailItem::Chats => l.left_visible,
            RailItem::Settings => settings_open,
            _ => self.panel().is_some_and(|p| l.right_visible && l.panel == p),
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
            // a gear: eight teeth round a hub
            RailItem::Settings => vec![gear(12., 12.), circle(12., 12., 3.)],
        }
    }
}

/// The little arrow beside a folded line: pointing down when it is open, otherwise towards where the text
/// continues (right, or left in right-to-left).
pub fn chevron(open: bool, rtl: bool) -> Vec<Vec<(f32, f32)>> {
    vec![match (open, rtl) {
        (true, _) => vec![(6., 9.), (12., 15.), (18., 9.)],
        (false, false) => vec![(9., 6.), (15., 12.), (9., 18.)],
        (false, true) => vec![(15., 6.), (9., 12.), (15., 18.)],
    }]
}

/// The tabs along the top of the side panel, in order. Settings is not one: it opens as a window over everything.
/// The side panel is about the chat that is open. Departments and the full memory are pages of their own.
pub const DOCK_TABS: [Panel; 4] = [Panel::Content, Panel::Work, Panel::Memory, Panel::Tree];

/// The name of a side panel, for its tab and for a screen reader.
pub fn panel_label(p: Panel, rtl: bool) -> &'static str {
    match (p, rtl) {
        (Panel::Content, false) => "Opened",
        (Panel::Content, true) => "المفتوح",
        (Panel::Departments, false) => "Departments",
        (Panel::Departments, true) => "الأقسام",
        (Panel::Memory, false) => "Memory used",
        (Panel::Memory, true) => "الذاكرة المستخدمة",
        (Panel::Tree, false) => "Branches",
        (Panel::Tree, true) => "الفروع",
        (Panel::Work, false) => "Work",
        (Panel::Work, true) => "العمل",
    }
}

/// The outline of a gear with eight square teeth, as one closed line.
fn gear(cx: f32, cy: f32) -> Vec<(f32, f32)> {
    let (root, tip) = (6.8f32, 9.6f32);
    let at = |r: f32, deg: f32| {
        let a = deg.to_radians();
        (cx + r * a.cos(), cy + r * a.sin())
    };
    let mut v = vec![];
    for k in 0..8 {
        let mid = k as f32 * 45.;
        v.push(at(root, mid - 15.));
        v.push(at(tip, mid - 9.));
        v.push(at(tip, mid + 9.));
        v.push(at(root, mid + 15.));
    }
    v.push(v[0]);
    v
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
    #[test]
    fn the_sidebar_button_folds_when_wide_and_opens_when_folded_with_theme_icons() {
        let wide = super::SidebarToggle::new(false, false);
        let folded = super::SidebarToggle::new(true, false);
        assert_eq!((wide.icon, folded.icon), ("sidebar-collapse", "sidebar-expand"));
        assert!(wide.aria.starts_with("Hide") && folded.aria.starts_with("Show"));
        for t in [wide, folded, super::SidebarToggle::new(true, true), super::SidebarToggle::new(false, true)] {
            assert!(crate::icons::NAMES.contains(&t.icon));
            assert!(t.aria.contains("Ctrl+B"));
        }
    }

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
    fn settings_is_a_gear_with_eight_teeth_and_a_hub_and_nothing_like_a_sun() {
        let icon = RailItem::Settings.icon();
        assert_eq!(icon.len(), 2, "the outline and the hub");
        assert_eq!(icon[0].len(), 8 * 4 + 1, "four points a tooth, closed");
        assert_eq!(icon[0].first(), icon[0].last());
        // a sun is a ring with separate rays; a gear is one joined outline, so no line in it is a short loose stroke
        assert!(icon.iter().all(|line| line.len() > 2));
        // the tips are farther out than the roots, so it has teeth
        let dist = |p: &(f32, f32)| ((p.0 - 12.).powi(2) + (p.1 - 12.).powi(2)).sqrt();
        let (lo, hi) = icon[0].iter().fold((f32::MAX, 0f32), |(lo, hi), p| (lo.min(dist(p)), hi.max(dist(p))));
        assert!(hi - lo > 2.5, "teeth stand out: {lo} to {hi}");
        assert!(icon[1].iter().all(|p| dist(p) < 3.1), "the hub is small");
    }

    #[test]
    fn the_rail_shows_which_things_are_open() {
        let mut l = Layout::default();
        assert!(RailItem::Chats.pressed(&l, false) && !RailItem::Memory.pressed(&l, false), "the chat list starts open, the side panel closed");
        l.toggle_left();
        assert!(!RailItem::Chats.pressed(&l, false));
        l.toggle_panel(Panel::Memory);
        assert!(RailItem::Memory.pressed(&l, false) && !RailItem::Departments.pressed(&l, false) && !RailItem::Settings.pressed(&l, false));
        l.toggle_panel(Panel::Tree);
        assert!(RailItem::ALL.iter().all(|i| i.panel().is_none() || !i.pressed(&l, false)), "the tree tab is not on the rail, so no item looks pressed");
        l.hide_right();
        assert!(!RailItem::Memory.pressed(&l, false), "a hidden panel is not pressed");
        assert_eq!(RailItem::Chats.panel(), None);
        assert_eq!(RailItem::Settings.panel(), None, "settings is a window, not a side panel");
        assert!(RailItem::Settings.pressed(&l, true) && !RailItem::Settings.pressed(&l, false), "pressed while its window is up");
        assert!(!RailItem::Memory.pressed(&l, true), "the window being up presses only its own icon");
    }

    #[test]
    fn the_side_panel_has_a_tab_for_everything_the_rail_does_not_reach() {
        for rtl in [false, true] {
            let mut names: Vec<&str> = DOCK_TABS.iter().map(|p| panel_label(*p, rtl)).collect();
            names.sort();
            names.dedup();
            assert_eq!(names.len(), 4);
        }
        for p in [Panel::Work, Panel::Memory, Panel::Content, Panel::Tree] {
            assert!(DOCK_TABS.contains(&p), "{p:?} has a tab");
        }
    }

    #[test]
    fn the_arrow_points_down_when_open_and_the_way_the_text_runs_when_closed() {
        let (open, ltr, rtl) = (chevron(true, false), chevron(false, false), chevron(false, true));
        assert_eq!(open, chevron(true, true), "open looks the same either way");
        assert!(ltr[0][1].0 > ltr[0][0].0, "closed points right");
        assert!(rtl[0][1].0 < rtl[0][0].0, "closed points left in right-to-left");
        assert!(open[0][1].1 > open[0][0].1, "open points down");
    }

    #[test]
    fn the_dimming_is_strong_and_the_top_layer_closes_first() {
        assert!(scrim_alpha(false) >= 0.5 && scrim_alpha(true) > scrim_alpha(false) && scrim_alpha(true) < 0.9);
        assert_eq!(topmost(true, true, true, true, true), Layer::Palette, "the palette covers settings");
        assert_eq!(topmost(false, true, true, true, true), Layer::Settings);
        assert_eq!(topmost(false, false, true, true, true), Layer::ModelMenu);
        assert_eq!(topmost(false, false, false, true, true), Layer::ResumeQuestion);
        assert_eq!(topmost(false, false, false, false, true), Layer::DeleteQuestion);
        assert_eq!(topmost(false, false, false, false, false), Layer::Nothing);
        const { assert!(SETTINGS_WIDTH >= 560. && SETTINGS_WIDTH <= 800.) };
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

/// The button at the top of the left sidebar: it folds the sidebar when it is wide and opens it when it is folded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SidebarToggle {
    /// The theme icon it shows (the folded sidebar shows the mark until the pointer is over it).
    pub icon: &'static str,
    /// What a screen reader says.
    pub aria: &'static str,
}

impl SidebarToggle {
    pub fn new(compact: bool, rtl: bool) -> SidebarToggle {
        match (compact, rtl) {
            (true, false) => SidebarToggle { icon: "sidebar-expand", aria: "Show the sidebar (Ctrl+B)" },
            (true, true) => SidebarToggle { icon: "sidebar-expand", aria: "إظهار الشريط (Ctrl+B)" },
            (false, false) => SidebarToggle { icon: "sidebar-collapse", aria: "Hide the sidebar (Ctrl+B)" },
            (false, true) => SidebarToggle { icon: "sidebar-collapse", aria: "إخفاء الشريط (Ctrl+B)" },
        }
    }
}

/// The theme icon name of a rail item.
pub fn rail_icon_name(i: RailItem) -> &'static str {
    match i {
        RailItem::Chats => "chats",
        RailItem::Departments => "departments",
        RailItem::Memory => "memory",
        RailItem::Settings => "settings",
    }
}

/// The theme icon name of a page of Settings. Chats, Memory and Departments share the rail's icon and name.
pub fn category_icon_name(c: crate::setting_names::Category) -> &'static str {
    use crate::setting_names::Category as C;
    match c {
        C::Appearance => "appearance",
        C::Models => "models",
        C::Chats => "chats",
        C::Memory => "memory",
        C::Learning => "learning",
        C::Departments => "departments",
        C::Permissions => "permissions",
        C::Spending => "spending",
        C::Web => "web",
        C::Tools => "tools",
        C::Channels => "channels",
        C::Data => "data",
        C::Activity => "activity",
        C::Hooks => "hooks",
        C::Schedules => "schedules",
        C::Updates => "updates",
        C::Other => "other",
    }
}

/// An outline icon for each page of Settings, on the same 24 by 24 grid as the rail.
pub fn category_icon(c: crate::setting_names::Category) -> Vec<Vec<(f32, f32)>> {
    use crate::setting_names::Category as C;
    let ellipse = |cx: f32, cy: f32, rx: f32, ry: f32| -> Vec<(f32, f32)> {
        (0..=16).map(|k| { let a = k as f32 * std::f32::consts::TAU / 16.; (cx + rx * a.cos(), cy + ry * a.sin()) }).collect()
    };
    match c {
        // a circle split down the middle: light and dark
        C::Appearance => vec![circle(12., 12., 8.), vec![(12., 4.), (12., 20.)]],
        // a chip with pins
        C::Models => vec![vec![(7., 7.), (17., 7.), (17., 17.), (7., 17.), (7., 7.)], vec![(10., 4.), (10., 7.)], vec![(14., 4.), (14., 7.)], vec![(10., 17.), (10., 20.)], vec![(14., 17.), (14., 20.)], vec![(4., 10.), (7., 10.)], vec![(4., 14.), (7., 14.)], vec![(17., 10.), (20., 10.)], vec![(17., 14.), (20., 14.)]],
        C::Chats => RailItem::Chats.icon(),
        C::Memory => RailItem::Memory.icon(),
        // a light bulb
        C::Learning => vec![circle(12., 10., 6.), vec![(9.5, 16.), (9.5, 19.), (14.5, 19.), (14.5, 16.)], vec![(10.5, 21.), (13.5, 21.)]],
        C::Departments => RailItem::Departments.icon(),
        // a shield
        C::Permissions => vec![vec![(12., 3.5), (19., 6.), (19., 12.), (12., 20.5), (5., 12.), (5., 6.), (12., 3.5)]],
        // a coin
        C::Spending => vec![circle(12., 12., 8.), vec![(12., 7.), (12., 17.)], vec![(9.5, 9.5), (14.5, 9.5)], vec![(9.5, 14.5), (14.5, 14.5)]],
        // a globe
        C::Web => vec![circle(12., 12., 8.), ellipse(12., 12., 3.5, 8.), vec![(4., 12.), (20., 12.)]],
        // a plug
        C::Tools => vec![vec![(8., 10.), (16., 10.), (16., 14.), (12., 18.), (8., 14.), (8., 10.)], vec![(10., 5.), (10., 10.)], vec![(14., 5.), (14., 10.)], vec![(12., 18.), (12., 21.)]],
        // a paper plane
        C::Channels => vec![vec![(4., 11.), (20., 4.), (14., 20.), (11., 13.), (4., 11.)], vec![(11., 13.), (20., 4.)]],
        // a stack of disks
        C::Data => vec![ellipse(12., 6., 7., 2.5), vec![(5., 6.), (5., 18.)], vec![(19., 6.), (19., 18.)], (0..=8).map(|k| { let a = k as f32 * std::f32::consts::PI / 8.; (12. + 7. * a.cos(), 18. + 2.5 * a.sin()) }).collect(), (0..=8).map(|k| { let a = k as f32 * std::f32::consts::PI / 8.; (12. + 7. * a.cos(), 12. + 2.5 * a.sin()) }).collect()],
        // a pulse
        C::Activity => vec![vec![(3., 12.), (8., 12.), (10., 6.), (14., 18.), (16., 12.), (21., 12.)]],
        // an arrow into a tray
        C::Updates => vec![vec![(12., 4.), (12., 14.)], vec![(8., 10.), (12., 14.), (16., 10.)], vec![(4., 15.), (4., 19.), (20., 19.), (20., 15.)]],
        // a fishhook: an eye, the shank, the bend and a barb
        C::Hooks => vec![circle(14., 5., 2.), vec![(14., 7.), (14., 15.), (12.5, 18.5), (9.5, 19.5), (7., 17.5), (6.5, 13.5)], vec![(6.5, 13.5), (9.5, 15.5)]],
        // a clock
        C::Schedules => vec![circle(12., 12., 8.), vec![(12., 7.), (12., 12.), (15.5, 14.)]],
        C::Other => vec![circle(6., 12., 1.2), circle(12., 12., 1.2), circle(18., 12., 1.2)],
    }
}
