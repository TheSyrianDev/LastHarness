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
    /// What is being done for this chat: its tool steps and department runs.
    Work,
    /// Last on purpose: a saved panel this version does not know (Settings used to be one; it is a window of its own
    /// now) falls back to this one.
    #[default]
    #[serde(other)]
    Departments,
}

/// The narrowest the conversation column goes, in pixels. Narrower than this and a code block or a table is cut off.
pub const COLUMN_MIN: f32 = 480.;
/// The width a fresh install starts with: clearly wide, because a narrow column wastes the screen.
pub const COLUMN_DEFAULT: f32 = 860.;
/// One arrow-key press, and one with Shift held, in pixels. Dragging and typing snap to the small step too.
pub const COLUMN_STEP: f32 = 10.;
pub const COLUMN_BIG_STEP: f32 = 50.;
/// Most presets a person can keep, and the longest name one can have.
pub const MAX_PRESETS: usize = 12;
pub const MAX_PRESET_NAME: usize = 30;

/// How wide the conversation column is: an exact number of pixels, or as wide as the middle of the window allows.
/// A number larger than the room the window has acts as Full, because the column never grows past the window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColumnWidth {
    Px(f32),
    /// The whole width of the middle of the window.
    Full,
}

impl Default for ColumnWidth {
    fn default() -> Self {
        ColumnWidth::Px(COLUMN_DEFAULT)
    }
}

/// The largest value the slider can reach for `room` pixels in the middle of the window: a whole number of steps,
/// and always a little above the minimum so the slider has a range.
pub fn slider_max(room: f32) -> f32 {
    let room = if room.is_finite() { room } else { COLUMN_MIN };
    ((room / COLUMN_STEP).floor() * COLUMN_STEP).max(COLUMN_MIN + COLUMN_STEP)
}

/// The slider value for the mirrored direction: in right-to-left the narrow end is on the right, and the slider
/// only knows left to right, so it is given the value reflected inside its range. Reflecting twice gives the
/// value back.
pub fn mirror(value: f32, room: f32) -> f32 {
    COLUMN_MIN + slider_max(room) - value
}

impl ColumnWidth {
    /// A number of pixels, rounded to the step and not below the minimum. Not-a-number gives the default.
    pub fn px(n: f32) -> ColumnWidth {
        if !n.is_finite() {
            return ColumnWidth::default();
        }
        ColumnWidth::Px(((n / COLUMN_STEP).round() * COLUMN_STEP).max(COLUMN_MIN))
    }

    /// The widest the column gets, in pixels; `None` for no limit.
    pub fn max_px(self) -> Option<f32> {
        match self {
            ColumnWidth::Px(n) => Some(n),
            ColumnWidth::Full => None,
        }
    }

    /// Where the slider thumb sits for `room` pixels in the middle of the window.
    pub fn position(self, room: f32) -> f32 {
        let max = slider_max(room);
        match self {
            ColumnWidth::Px(n) => n.clamp(COLUMN_MIN, max),
            ColumnWidth::Full => max,
        }
    }

    /// What a slider value means: the far end is Full, anything else is that many pixels.
    pub fn from_slider(value: f32, room: f32) -> ColumnWidth {
        let max = slider_max(room);
        if !value.is_finite() {
            return ColumnWidth::default();
        }
        if value >= max {
            ColumnWidth::Full
        } else {
            ColumnWidth::px(value)
        }
    }

    /// One arrow key: a step left or right (`down` is the narrower way), a bigger step with Shift.
    pub fn nudge(self, narrower: bool, big: bool, room: f32) -> ColumnWidth {
        let step = if big { COLUMN_BIG_STEP } else { COLUMN_STEP };
        let at = self.position(room);
        ColumnWidth::from_slider(if narrower { at - step } else { at + step }, room)
    }

    /// What a key on the slider does: arrows step (Shift for a bigger step), Page keys step by the big step, Home is
    /// the narrowest and End is Full. Left and right follow the direction of the text, so in right-to-left Left
    /// widens. `None` for any other key.
    pub fn after_key(self, key: &str, shift: bool, rtl: bool, room: f32) -> Option<ColumnWidth> {
        Some(match key {
            "right" => self.nudge(rtl, shift, room),
            "left" => self.nudge(!rtl, shift, room),
            "up" => self.nudge(false, shift, room),
            "down" => self.nudge(true, shift, room),
            "pageup" => self.nudge(false, true, room),
            "pagedown" => self.nudge(true, true, room),
            "home" => ColumnWidth::Px(COLUMN_MIN),
            "end" => ColumnWidth::Full,
            _ => return None,
        })
    }

    /// Words typed into the number box: "900", "900 px" or "full". Nothing else is a width.
    pub fn parse(text: &str, room: f32) -> Option<ColumnWidth> {
        let t = text.trim().to_lowercase();
        if t == "full" || t == "الكامل" {
            return Some(ColumnWidth::Full);
        }
        let digits = t.strip_suffix("px").unwrap_or(&t).trim();
        let n: f32 = digits.parse().ok()?;
        n.is_finite().then(|| ColumnWidth::from_slider(n.max(COLUMN_MIN), room))
    }

    /// The number a person sees: "860 px", or "Full".
    pub fn label(self, rtl: bool) -> String {
        match (self, rtl) {
            (ColumnWidth::Px(n), _) => format!("{} px", n as i64),
            (ColumnWidth::Full, false) => "Full".to_string(),
            (ColumnWidth::Full, true) => "العرض الكامل".to_string(),
        }
    }

    /// What goes in the number box: the pixels now in use (Full shows the room there is).
    pub fn input_text(self, room: f32) -> String {
        format!("{}", self.position(room) as i64)
    }
}

/// How `ColumnWidth` is written in `gui.toml`: a whole number of pixels, or the word `full`. The two words an older
/// version wrote for its fixed choices are still read: `comfortable` is 700 and `wide` is 860.
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum Written {
    Int(i64),
    Float(f64),
    Word(String),
}

impl From<ColumnWidth> for Written {
    fn from(w: ColumnWidth) -> Written {
        match w {
            ColumnWidth::Px(n) => Written::Int(n as i64),
            ColumnWidth::Full => Written::Word("full".into()),
        }
    }
}

impl TryFrom<Written> for ColumnWidth {
    type Error = String;
    fn try_from(w: Written) -> Result<ColumnWidth, String> {
        match w {
            Written::Int(n) => Ok(ColumnWidth::px(n as f32)),
            Written::Float(n) => Ok(ColumnWidth::px(n as f32)),
            Written::Word(s) => match s.trim().to_lowercase().as_str() {
                "full" => Ok(ColumnWidth::Full),
                "comfortable" => Ok(ColumnWidth::Px(700.)),
                "wide" => Ok(ColumnWidth::Px(860.)),
                other => Err(format!("{other:?} is not a chat width")),
            },
        }
    }
}

impl Serialize for ColumnWidth {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        Written::from(*self).serialize(s)
    }
}

impl<'de> Deserialize<'de> for ColumnWidth {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<ColumnWidth, D::Error> {
        ColumnWidth::try_from(Written::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}

/// A chat width a person named and kept.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub width: ColumnWidth,
}

/// Widths that are always in the list, labelled by their number.
pub const BUILT_IN_WIDTHS: [ColumnWidth; 4] = [ColumnWidth::Px(700.), ColumnWidth::Px(900.), ColumnWidth::Px(1100.), ColumnWidth::Full];

/// Why a preset could not be saved or renamed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresetError {
    EmptyName,
    NameTaken,
    TooMany,
    NoSuchPreset,
}

impl PresetError {
    pub fn message(self, rtl: bool) -> &'static str {
        match (self, rtl) {
            (PresetError::EmptyName, false) => "Give the preset a name first.",
            (PresetError::NameTaken, false) => "Another preset already has that name.",
            (PresetError::TooMany, false) => "That is the most presets you can keep. Delete one first.",
            (PresetError::NoSuchPreset, false) => "Pick a preset first.",
            (PresetError::EmptyName, true) => "اكتب اسمًا للإعداد أولًا.",
            (PresetError::NameTaken, true) => "إعداد آخر يحمل هذا الاسم.",
            (PresetError::TooMany, true) => "هذا أقصى عدد من الإعدادات. احذف واحدًا أولًا.",
            (PresetError::NoSuchPreset, true) => "اختر إعدادًا أولًا.",
        }
    }
}

/// A name as it is kept: trimmed, one line, not longer than the limit.
fn clean_name(name: &str) -> String {
    name.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(MAX_PRESET_NAME).collect()
}

/// What fills the middle.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    Chat,
    /// A department's own page, opened from the sidebar. The chat keeps running behind it.
    Department(String),
    /// Every department, opened from the sidebar.
    Departments,
    /// Everything remembered, opened from the sidebar.
    Memory,
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
    /// Chat widths the person named, in the order they made them.
    pub presets: Vec<Preset>,
    #[serde(skip)]
    pub page: Page,
}

impl Default for Layout {
    fn default() -> Self {
        Layout { left_visible: true, left_width: 260., right_visible: false, right_width: 380., right_expanded: false, panel: Panel::Departments, column: ColumnWidth::default(), presets: Vec::new(), page: Page::Chat }
    }
}

impl Layout {
    /// Drop presets that cannot be shown (no name, too many) after reading a file someone may have edited by hand.
    pub fn tidy(&mut self) {
        for p in &mut self.presets {
            p.name = clean_name(&p.name);
        }
        self.presets.retain(|p| !p.name.is_empty());
        let mut seen: Vec<String> = Vec::new();
        self.presets.retain(|p| {
            let key = p.name.to_lowercase();
            let fresh = !seen.contains(&key);
            seen.push(key);
            fresh
        });
        self.presets.truncate(MAX_PRESETS);
    }

    fn name_taken(&self, name: &str, except: Option<usize>) -> bool {
        let key = name.to_lowercase();
        self.presets.iter().enumerate().any(|(i, p)| Some(i) != except && p.name.to_lowercase() == key)
    }

    /// Keep the width now in use under `name`. A name that already exists gets the new width. Returns the place of
    /// the preset in the list.
    pub fn save_preset(&mut self, name: &str) -> Result<usize, PresetError> {
        let name = clean_name(name);
        if name.is_empty() {
            return Err(PresetError::EmptyName);
        }
        if let Some(i) = self.presets.iter().position(|p| p.name.to_lowercase() == name.to_lowercase()) {
            self.presets[i].width = self.column;
            return Ok(i);
        }
        if self.presets.len() >= MAX_PRESETS {
            return Err(PresetError::TooMany);
        }
        self.presets.push(Preset { name, width: self.column });
        Ok(self.presets.len() - 1)
    }

    pub fn rename_preset(&mut self, at: usize, name: &str) -> Result<(), PresetError> {
        let name = clean_name(name);
        if at >= self.presets.len() {
            return Err(PresetError::NoSuchPreset);
        }
        if name.is_empty() {
            return Err(PresetError::EmptyName);
        }
        if self.name_taken(&name, Some(at)) {
            return Err(PresetError::NameTaken);
        }
        self.presets[at].name = name;
        Ok(())
    }

    pub fn delete_preset(&mut self, at: usize) -> Result<(), PresetError> {
        if at >= self.presets.len() {
            return Err(PresetError::NoSuchPreset);
        }
        self.presets.remove(at);
        Ok(())
    }

    /// Use a saved preset's width.
    pub fn apply_preset(&mut self, at: usize) -> Result<(), PresetError> {
        let p = self.presets.get(at).ok_or(PresetError::NoSuchPreset)?;
        self.column = p.width;
        Ok(())
    }

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

    pub fn open_page(&mut self, page: Page) {
        self.page = page;
    }

    pub fn back_to_chat(&mut self) {
        self.page = Page::Chat;
    }

    /// The room the conversation column has in the middle, once the chat list and the side panel have their share.
    /// `total` is what `widths` is given. The 24 is the padding round the column.
    pub fn middle_room(&self, total: f32, min_middle: f32) -> f32 {
        let (left, right) = self.widths(total, min_middle);
        (total - left - right - 24.).max(0.)
    }

    /// The side panel does not fit beside a chat at least `min_middle` wide, so it is laid over the chat instead.
    pub fn overlays(&self, total: f32, min_middle: f32) -> bool {
        let left = if self.left_visible { self.left_width } else { 0. };
        self.right_visible && !self.right_expanded && total - left - self.right_width < min_middle
    }

    /// The width of the side panel laid over the chat: its own width, but leaving a strip of the chat in view.
    pub fn overlay_width(&self, total: f32) -> f32 {
        let left = if self.left_visible { self.left_width } else { 0. };
        self.right_width.min(total - left - 48.).max(RIGHT_MIN.min(total))
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
    fn a_narrow_window_lays_the_side_panel_over_the_chat() {
        let mut l = Layout::default();
        l.show_panel(Panel::Work);
        assert!(!l.overlays(1400., 420.), "room for all three");
        assert!(l.overlays(900., 420.));
        assert_eq!(l.overlay_width(900.), 380.);
        assert_eq!(l.overlay_width(600.), 292., "a strip of the chat stays in view");
        l.hide_right();
        assert!(!l.overlays(600., 420.));
    }

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

    const ROOM: f32 = 1200.;

    #[test]
    fn the_chat_column_starts_clearly_wide() {
        let l = Layout::default();
        assert_eq!(l.column, ColumnWidth::Px(860.));
        assert!(l.column.max_px().unwrap() >= 860., "at least 860, well past the old 640");
        assert!(l.presets.is_empty());
    }

    #[test]
    fn widths_are_clamped_snapped_and_the_far_end_is_full() {
        assert_eq!(ColumnWidth::px(100.), ColumnWidth::Px(480.), "never below the minimum");
        assert_eq!(ColumnWidth::px(-5.), ColumnWidth::Px(480.));
        assert_eq!(ColumnWidth::px(903.), ColumnWidth::Px(900.), "snaps to the step");
        assert_eq!(ColumnWidth::px(f32::NAN), ColumnWidth::default());
        assert_eq!(ColumnWidth::from_slider(900., ROOM), ColumnWidth::Px(900.));
        assert_eq!(ColumnWidth::from_slider(1200., ROOM), ColumnWidth::Full, "the far end is Full");
        assert_eq!(ColumnWidth::from_slider(5000., ROOM), ColumnWidth::Full, "past the end is Full");
        assert_eq!(ColumnWidth::from_slider(10., ROOM), ColumnWidth::Px(480.));
        assert_eq!(ColumnWidth::from_slider(f32::NAN, ROOM), ColumnWidth::default());
        // the thumb sits where the width is, and Full sits at the end; a width the window cannot show sits at the end
        assert_eq!(ColumnWidth::Px(900.).position(ROOM), 900.);
        assert_eq!(ColumnWidth::Full.position(ROOM), 1200.);
        assert_eq!(ColumnWidth::Px(1500.).position(ROOM), 1200.);
        assert_eq!(ColumnWidth::Px(480.).position(100.), 480., "a tiny window still has a range");
        assert!(slider_max(100.) > COLUMN_MIN);
        assert_eq!(slider_max(1234.), 1230., "a whole number of steps");
        assert_eq!(slider_max(f32::NAN), COLUMN_MIN + COLUMN_STEP);
    }

    #[test]
    fn mirroring_reflects_inside_the_range_and_undoes_itself() {
        assert_eq!(mirror(480., ROOM), 1200.);
        assert_eq!(mirror(1200., ROOM), 480.);
        for v in [480., 500., 860., 1190., 1200.] {
            assert_eq!(mirror(mirror(v, ROOM), ROOM), v);
        }
        // a bigger value is nearer the left after mirroring, so dragging right makes the column narrower
        assert!(mirror(900., ROOM) < mirror(800., ROOM));
    }

    #[test]
    fn the_room_in_the_middle_is_what_the_side_parts_leave() {
        let mut l = Layout::default();
        assert_eq!(l.middle_room(1500., 420.), 1500. - 260. - 24.);
        l.toggle_left();
        assert_eq!(l.middle_room(1500., 420.), 1500. - 24.);
        l.show_panel(Panel::Content);
        assert_eq!(l.middle_room(1500., 420.), 1500. - 380. - 24.);
        assert_eq!(l.middle_room(10., 420.), 0., "never negative");
    }

    #[test]
    fn arrows_step_ten_and_shift_arrows_step_fifty_and_stop_at_both_ends() {
        let w = ColumnWidth::Px(860.);
        assert_eq!(w.nudge(false, false, ROOM), ColumnWidth::Px(870.));
        assert_eq!(w.nudge(true, false, ROOM), ColumnWidth::Px(850.));
        assert_eq!(w.nudge(false, true, ROOM), ColumnWidth::Px(910.));
        assert_eq!(w.nudge(true, true, ROOM), ColumnWidth::Px(810.));
        assert_eq!(ColumnWidth::Px(480.).nudge(true, true, ROOM), ColumnWidth::Px(480.), "stops at the minimum");
        assert_eq!(ColumnWidth::Px(1190.).nudge(false, false, ROOM), ColumnWidth::Full, "the last step reaches Full");
        assert_eq!(ColumnWidth::Full.nudge(false, true, ROOM), ColumnWidth::Full, "stops at Full");
        assert_eq!(ColumnWidth::Full.nudge(true, false, ROOM), ColumnWidth::Px(1190.), "and comes back from it");
    }

    #[test]
    fn keys_follow_the_direction_of_the_text() {
        let w = ColumnWidth::Px(860.);
        let k = |key: &str, shift: bool, rtl: bool| w.after_key(key, shift, rtl, ROOM);
        assert_eq!(k("right", false, false), Some(ColumnWidth::Px(870.)));
        assert_eq!(k("left", false, false), Some(ColumnWidth::Px(850.)));
        assert_eq!(k("right", false, true), Some(ColumnWidth::Px(850.)), "mirrored: right is narrower");
        assert_eq!(k("left", false, true), Some(ColumnWidth::Px(870.)), "mirrored: left is wider");
        assert_eq!(k("left", true, true), Some(ColumnWidth::Px(910.)), "shift steps fifty");
        assert_eq!(k("up", false, true), Some(ColumnWidth::Px(870.)), "up is wider in both directions");
        assert_eq!(k("down", true, false), Some(ColumnWidth::Px(810.)));
        assert_eq!(k("pageup", false, false), Some(ColumnWidth::Px(910.)));
        assert_eq!(k("pagedown", false, true), Some(ColumnWidth::Px(810.)));
        assert_eq!(k("home", false, false), Some(ColumnWidth::Px(480.)));
        assert_eq!(k("end", false, true), Some(ColumnWidth::Full));
        for other in ["a", "enter", "tab", "escape", "space"] {
            assert_eq!(k(other, false, false), None, "{other} is not for the slider");
        }
    }

    #[test]
    fn typed_widths_are_read_clamped_or_refused() {
        assert_eq!(ColumnWidth::parse("900", ROOM), Some(ColumnWidth::Px(900.)));
        assert_eq!(ColumnWidth::parse("  900 px ", ROOM), Some(ColumnWidth::Px(900.)));
        assert_eq!(ColumnWidth::parse("905", ROOM), Some(ColumnWidth::Px(910.)));
        assert_eq!(ColumnWidth::parse("200", ROOM), Some(ColumnWidth::Px(480.)), "too small is the minimum");
        assert_eq!(ColumnWidth::parse("99999", ROOM), Some(ColumnWidth::Full), "too large is Full");
        assert_eq!(ColumnWidth::parse("Full", ROOM), Some(ColumnWidth::Full));
        for bad in ["", "wide", "12x", "px", "NaN", "inf", "-"] {
            assert_eq!(ColumnWidth::parse(bad, ROOM), None, "{bad:?}");
        }
        assert_eq!(ColumnWidth::Px(860.).input_text(ROOM), "860");
        assert_eq!(ColumnWidth::Full.input_text(ROOM), "1200");
    }

    #[test]
    fn labels_are_numbers_not_vague_words() {
        assert_eq!(ColumnWidth::Px(860.).label(false), "860 px");
        assert_eq!(ColumnWidth::Px(860.).label(true), "860 px");
        assert_eq!(ColumnWidth::Full.label(false), "Full");
        for w in BUILT_IN_WIDTHS {
            let l = w.label(false);
            assert!(l == "Full" || l.ends_with(" px"), "{l}");
        }
        assert_eq!(BUILT_IN_WIDTHS.iter().filter(|w| **w == ColumnWidth::Full).count(), 1);
    }

    #[test]
    fn the_column_is_saved_as_a_number_and_an_old_file_without_it_gets_the_default() {
        let l = Layout { column: ColumnWidth::Px(930.), ..Layout::default() };
        let s = toml::to_string(&l).unwrap();
        assert!(s.contains("column = 930"), "{s}");
        assert_eq!(toml::from_str::<Layout>(&s).unwrap().column, ColumnWidth::Px(930.));
        let f = toml::to_string(&Layout { column: ColumnWidth::Full, ..Layout::default() }).unwrap();
        assert!(f.contains("column = \"full\""), "{f}");
        assert_eq!(toml::from_str::<Layout>(&f).unwrap().column, ColumnWidth::Full);
        assert_eq!(toml::from_str::<Layout>("left_visible = false\n").unwrap().column, ColumnWidth::Px(860.));
        assert!(toml::from_str::<Layout>("column = \"nonsense\"\n").is_err(), "a bad word is an error, which Prefs turns into the defaults");
        // a hand-edited number is clamped, not trusted
        assert_eq!(toml::from_str::<Layout>("column = 12\n").unwrap().column, ColumnWidth::Px(480.));
        assert_eq!(toml::from_str::<Layout>("column = 905.5\n").unwrap().column, ColumnWidth::Px(910.));
    }

    #[test]
    fn the_three_named_choices_of_the_last_version_become_numbers() {
        let read = |w: &str| toml::from_str::<Layout>(&format!("column = \"{w}\"\n")).unwrap().column;
        assert_eq!(read("comfortable"), ColumnWidth::Px(700.));
        assert_eq!(read("wide"), ColumnWidth::Px(860.));
        assert_eq!(read("full"), ColumnWidth::Full);
        // and once saved again the old words are gone
        let l = Layout { column: read("comfortable"), ..Layout::default() };
        let s = toml::to_string(&l).unwrap();
        assert!(s.contains("column = 700") && !s.contains("comfortable"), "{s}");
    }

    #[test]
    fn presets_are_saved_applied_renamed_and_deleted() {
        let mut l = Layout { column: ColumnWidth::Px(1000.), ..Layout::default() };
        assert_eq!(l.save_preset("  Reading  "), Ok(0));
        l.column = ColumnWidth::Full;
        assert_eq!(l.save_preset("Wide screen"), Ok(1));
        assert_eq!(l.presets[0], Preset { name: "Reading".into(), width: ColumnWidth::Px(1000.) });
        // applying moves the column
        assert!(l.apply_preset(0).is_ok());
        assert_eq!(l.column, ColumnWidth::Px(1000.));
        // saving under a name that exists (any case) updates it instead of adding a second
        l.column = ColumnWidth::Px(600.);
        assert_eq!(l.save_preset("reading"), Ok(0));
        assert_eq!((l.presets.len(), l.presets[0].width), (2, ColumnWidth::Px(600.)));
        // rename
        assert!(l.rename_preset(1, "Big").is_ok());
        assert_eq!(l.presets[1].name, "Big");
        assert_eq!(l.rename_preset(1, "READING"), Err(PresetError::NameTaken));
        assert!(l.rename_preset(1, "big").is_ok(), "renaming to its own name in another case is fine");
        assert_eq!(l.rename_preset(1, "  "), Err(PresetError::EmptyName));
        assert_eq!(l.rename_preset(9, "x"), Err(PresetError::NoSuchPreset));
        // delete
        assert!(l.delete_preset(0).is_ok());
        assert_eq!(l.presets.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["big"]);
        assert_eq!(l.delete_preset(5), Err(PresetError::NoSuchPreset));
        assert_eq!(l.apply_preset(5), Err(PresetError::NoSuchPreset));
        assert_eq!(l.save_preset(""), Err(PresetError::EmptyName));
        // the cap and the name length
        let mut m = Layout::default();
        for i in 0..MAX_PRESETS {
            assert!(m.save_preset(&format!("p{i}")).is_ok());
        }
        assert_eq!(m.save_preset("one more"), Err(PresetError::TooMany));
        assert!(m.save_preset("P3").is_ok(), "an existing name can still be updated when the list is full");
        let long = "x".repeat(100);
        let mut n = Layout::default();
        n.save_preset(&long).unwrap();
        assert_eq!(n.presets[0].name.chars().count(), MAX_PRESET_NAME);
        n.save_preset("two\nlines").unwrap();
        assert_eq!(n.presets[1].name, "two lines");
    }

    #[test]
    fn presets_are_written_to_the_file_and_read_back() {
        let mut l = Layout { column: ColumnWidth::Px(1010.), ..Layout::default() };
        l.save_preset("Reading").unwrap();
        l.column = ColumnWidth::Full;
        l.save_preset("Whole screen").unwrap();
        let s = toml::to_string(&l).unwrap();
        let back = toml::from_str::<Layout>(&s).unwrap();
        assert_eq!(back.presets, l.presets, "{s}");
        // a hand-edited file with a nameless or repeated preset is tidied, not trusted
        let mut odd = toml::from_str::<Layout>("[[presets]]\nname = \" \"\nwidth = 900\n[[presets]]\nname = \"A\"\nwidth = 5\n[[presets]]\nname = \"a\"\nwidth = 800\n").unwrap();
        odd.tidy();
        assert_eq!(odd.presets, vec![Preset { name: "A".into(), width: ColumnWidth::Px(480.) }]);
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
