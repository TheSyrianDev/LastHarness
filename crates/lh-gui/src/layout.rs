//! Which parts of the window are shown and how wide. Every part can be hidden or made larger; this is the
//! state behind that, kept apart from the drawing so it can be tested.

use serde::{Deserialize, Serialize};

pub const LEFT_MIN: f32 = 180.;
pub const LEFT_MAX: f32 = 420.;
pub const RIGHT_MIN: f32 = 280.;
pub const RIGHT_MAX: f32 = 900.;

/// What the right panel shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Panel {
    Memory,
    /// Things the agent opened: a page, a file, a table.
    Content,
    /// The branches of the chat that is open.
    Tree,
    /// Last on purpose: a saved panel this version does not know (Settings used to be one; it is a window of its own
    /// now) falls back to this one.
    #[default]
    #[serde(other)]
    Departments,
}

/// How wide the conversation column is. Long lines are tiring to read, so even the widest fixed choice stops at a
/// length a person can follow; Full uses all the room there is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ColumnWidth {
    /// About 75 characters a line.
    Comfortable,
    /// About 100 characters a line.
    #[default]
    Wide,
    /// The whole width of the middle of the window.
    Full,
}

impl ColumnWidth {
    pub const ALL: [ColumnWidth; 3] = [ColumnWidth::Comfortable, ColumnWidth::Wide, ColumnWidth::Full];

    /// The widest the column gets, in pixels; `None` for no limit.
    pub fn max_px(self) -> Option<f32> {
        match self {
            ColumnWidth::Comfortable => Some(700.),
            ColumnWidth::Wide => Some(860.),
            ColumnWidth::Full => None,
        }
    }

    pub fn label(self, rtl: bool) -> &'static str {
        match (self, rtl) {
            (ColumnWidth::Comfortable, false) => "Comfortable",
            (ColumnWidth::Wide, false) => "Wide",
            (ColumnWidth::Full, false) => "Full width",
            (ColumnWidth::Comfortable, true) => "مريح",
            (ColumnWidth::Wide, true) => "عريض",
            (ColumnWidth::Full, true) => "العرض الكامل",
        }
    }
}

/// What fills the middle.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    Chat,
    /// A department's own page, opened from the sidebar. The chat keeps running behind it.
    Department(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Layout {
    pub left_visible: bool,
    pub left_width: f32,
    pub right_visible: bool,
    pub right_width: f32,
    /// The right panel takes the whole width except the chat list's.
    pub right_expanded: bool,
    pub panel: Panel,
    /// How wide the conversation, its questions and the message box are.
    pub column: ColumnWidth,
    #[serde(skip)]
    pub page: Page,
}

impl Default for Layout {
    fn default() -> Self {
        Layout { left_visible: true, left_width: 260., right_visible: false, right_width: 380., right_expanded: false, panel: Panel::Departments, column: ColumnWidth::default(), page: Page::Chat }
    }
}

impl Layout {
    pub fn toggle_left(&mut self) {
        self.left_visible = !self.left_visible;
    }

    /// Show the right panel with `panel`; choosing the panel that is already shown hides it.
    pub fn toggle_panel(&mut self, panel: Panel) {
        if self.right_visible && self.panel == panel {
            self.right_visible = false;
            self.right_expanded = false;
        } else {
            self.right_visible = true;
            self.panel = panel;
        }
    }

    /// Show the right panel with `panel` and keep it shown (the agent opened something).
    pub fn show_panel(&mut self, panel: Panel) {
        self.right_visible = true;
        self.panel = panel;
    }

    pub fn hide_right(&mut self) {
        self.right_visible = false;
        self.right_expanded = false;
    }

    pub fn toggle_expand(&mut self) {
        if self.right_visible {
            self.right_expanded = !self.right_expanded;
        }
    }

    /// The wide size the side panel takes between its normal width and the whole window.
    pub const RIGHT_WIDE: f32 = 560.;

    /// The Widen button: normal, then wide, then as wide as the window allows, then back to normal.
    pub fn cycle_width(&mut self) {
        if !self.right_visible {
            return;
        }
        if self.right_expanded {
            self.right_expanded = false;
            self.right_width = 380.;
        } else if self.right_width < Self::RIGHT_WIDE - 1. {
            self.right_width = Self::RIGHT_WIDE;
        } else {
            self.right_expanded = true;
        }
    }

    pub fn resize_left(&mut self, w: f32) {
        self.left_width = w.clamp(LEFT_MIN, LEFT_MAX);
    }

    pub fn resize_right(&mut self, w: f32) {
        self.right_width = w.clamp(RIGHT_MIN, RIGHT_MAX);
        self.right_expanded = false;
    }

    pub fn open_department(&mut self, name: &str) {
        self.page = Page::Department(name.into());
    }

    pub fn back_to_chat(&mut self) {
        self.page = Page::Chat;
    }

    /// Widths the two side parts take in a window `total` wide, leaving the middle at least `min_middle`.
    /// When space is short the right panel gives way first, then the left.
    pub fn widths(&self, total: f32, min_middle: f32) -> (f32, f32) {
        let mut left = if self.left_visible { self.left_width } else { 0. };
        let mut right = if !self.right_visible {
            0.
        } else if self.right_expanded {
            (total - left).max(RIGHT_MIN)
        } else {
            self.right_width
        };
        if left + right + min_middle > total && !self.right_expanded {
            right = (total - left - min_middle).max(if self.right_visible { RIGHT_MIN.min(self.right_width) } else { 0. });
        }
        if left + right + min_middle > total && !self.right_expanded {
            left = (total - right - min_middle).max(if self.left_visible { LEFT_MIN.min(self.left_width) } else { 0. });
        }
        (left, right)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_starts_with_the_chat_list_and_the_chat_only() {
        let l = Layout::default();
        assert!(l.left_visible && !l.right_visible);
        assert_eq!(l.page, Page::Chat);
    }

    #[test]
    fn choosing_the_shown_panel_again_hides_it() {
        let mut l = Layout::default();
        l.toggle_panel(Panel::Memory);
        assert!(l.right_visible && l.panel == Panel::Memory);
        l.toggle_panel(Panel::Tree);
        assert!(l.right_visible && l.panel == Panel::Tree, "another panel switches, not hides");
        l.toggle_expand();
        assert!(l.right_expanded);
        l.toggle_panel(Panel::Tree);
        assert!(!l.right_visible && !l.right_expanded, "hiding also ends the expanded state");
    }

    #[test]
    fn something_the_assistant_opens_brings_the_side_panel_back_whatever_was_hidden() {
        let mut l = Layout::default();
        l.toggle_left();
        l.toggle_panel(Panel::Memory);
        l.hide_right();
        assert!(!l.left_visible && !l.right_visible, "everything hidden");
        l.show_panel(Panel::Content);
        assert!(l.right_visible && l.panel == Panel::Content);
        let (left, right) = l.widths(1200., 420.);
        assert_eq!(left, 0., "the chat list stays hidden; the person has the Chats button for it");
        assert!(right > 0., "the panel has room to be seen");
        // and in a narrow window the panel still shows
        let (_, right) = l.widths(700., 420.);
        assert!(right > 0.);
        l.toggle_left();
        assert!(l.left_visible, "the Chats button brings the list back");
    }

    #[test]
    fn expanding_needs_a_visible_panel_and_dragging_ends_it() {
        let mut l = Layout::default();
        l.toggle_expand();
        assert!(!l.right_expanded);
        l.show_panel(Panel::Content);
        l.toggle_expand();
        assert!(l.right_expanded);
        l.resize_right(500.);
        assert!(!l.right_expanded);
        assert_eq!(l.right_width, 500.);
    }

    #[test]
    fn the_chat_column_starts_clearly_wider_than_before_and_has_three_sizes() {
        let l = Layout::default();
        assert_eq!(l.column, ColumnWidth::Wide);
        assert!(l.column.max_px().unwrap() >= 800., "clearly wider than the old 640");
        let px: Vec<Option<f32>> = ColumnWidth::ALL.iter().map(|c| c.max_px()).collect();
        assert!(px[0].unwrap() > 640. && px[0].unwrap() < px[1].unwrap(), "each choice is wider than the one before");
        assert_eq!(px[2], None, "full has no limit");
        for rtl in [false, true] {
            let mut names: Vec<&str> = ColumnWidth::ALL.iter().map(|c| c.label(rtl)).collect();
            names.sort();
            names.dedup();
            assert_eq!(names.len(), 3);
        }
    }

    #[test]
    fn the_column_choice_is_saved_and_an_old_file_without_it_gets_the_default() {
        let l = Layout { column: ColumnWidth::Full, ..Layout::default() };
        let s = toml::to_string(&l).unwrap();
        assert!(s.contains("column = \"full\""), "{s}");
        assert_eq!(toml::from_str::<Layout>(&s).unwrap().column, ColumnWidth::Full);
        assert_eq!(toml::from_str::<Layout>("left_visible = false\n").unwrap().column, ColumnWidth::Wide);
        assert!(toml::from_str::<Layout>("column = \"nonsense\"\n").is_err(), "a bad word is an error, which Prefs turns into the defaults");
    }

    #[test]
    fn widen_goes_normal_wide_whole_window_and_back() {
        let mut l = Layout::default();
        l.cycle_width();
        assert_eq!(l.right_width, 380., "nothing shown, nothing changes");
        l.show_panel(Panel::Content);
        l.cycle_width();
        assert!(!l.right_expanded && l.right_width == Layout::RIGHT_WIDE);
        l.cycle_width();
        assert!(l.right_expanded);
        l.cycle_width();
        assert!(!l.right_expanded && l.right_width == 380.);
    }

    #[test]
    fn sizes_stay_within_their_limits() {
        let mut l = Layout::default();
        l.resize_left(10.);
        assert_eq!(l.left_width, LEFT_MIN);
        l.resize_left(9000.);
        assert_eq!(l.left_width, LEFT_MAX);
        l.resize_right(10.);
        assert_eq!(l.right_width, RIGHT_MIN);
        l.resize_right(9000.);
        assert_eq!(l.right_width, RIGHT_MAX);
    }

    #[test]
    fn a_department_page_replaces_the_chat_without_ending_it() {
        let mut l = Layout::default();
        l.open_department("research");
        assert_eq!(l.page, Page::Department("research".into()));
        l.back_to_chat();
        assert_eq!(l.page, Page::Chat);
    }

    #[test]
    fn in_a_narrow_window_the_right_panel_gives_way_first_and_the_middle_keeps_its_minimum() {
        let mut l = Layout::default();
        l.show_panel(Panel::Memory);
        l.right_width = 600.;
        let (left, right) = l.widths(1500., 420.);
        assert_eq!((left, right), (260., 600.));
        let (left, right) = l.widths(1000., 420.);
        assert_eq!(left, 260.);
        assert_eq!(right, 320., "1000 - 260 - 420");
        let (left, right) = l.widths(700., 420.);
        assert!(left + right + 420. <= 700. + 1e-3 || (left, right) == (LEFT_MIN.min(260.), RIGHT_MIN.min(600.)), "{left} {right}");
        l.hide_right();
        assert_eq!(l.widths(700., 420.).1, 0.);
        l.left_visible = false;
        assert_eq!(l.widths(700., 420.), (0., 0.));
    }

    #[test]
    fn an_expanded_panel_takes_everything_but_the_chat_list() {
        let mut l = Layout::default();
        l.show_panel(Panel::Content);
        l.toggle_expand();
        assert_eq!(l.widths(1400., 420.), (260., 1140.));
        l.toggle_left();
        assert_eq!(l.widths(1400., 420.), (0., 1400.));
    }

    #[test]
    fn it_saves_and_reads_back_but_does_not_save_the_current_page() {
        let mut l = Layout::default();
        l.show_panel(Panel::Memory);
        l.open_department("x");
        let s = toml::to_string(&l).unwrap();
        assert!(!s.contains("page"));
        let back: Layout = toml::from_str(&s).unwrap();
        assert_eq!(back.panel, Panel::Memory);
        assert_eq!(back.page, Page::Chat);
        let old: Layout = toml::from_str("left_visible = false\n").unwrap();
        assert!(!old.left_visible && old.left_width == 260., "missing keys take the defaults");
    }

    #[test]
    fn a_file_saved_when_settings_was_a_side_panel_tab_still_reads() {
        let old: Layout = toml::from_str("right_visible = true\npanel = \"settings\"\n").unwrap();
        assert_eq!(old.panel, Panel::Departments, "settings is a window now, so the panel falls back");
        assert!(old.right_visible);
    }
}
