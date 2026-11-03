//! The command palette (Ctrl+K): one box that reaches every place and setting by typing a few letters.
//! This module builds the list of commands from what the app knows, ranks them against what was typed, and
//! keeps the chosen row. It draws nothing, so it is tested without a window.

use crate::chrome::{panel_label, DOCK_TABS};
use crate::appearance::ModeChoice;
use crate::layout::{ColumnWidth, Panel, Preset, BUILT_IN_WIDTHS};
use crate::rtl::Direction;

/// What choosing a row does.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    NewChat,
    Settings,
    ToggleChatList,
    OpenTab(Panel),
    SwitchChat(u64),
    Model(String),
    /// A family of looks, by its place in the list of families.
    Look(usize),
    Mode(ModeChoice),
    ToggleMode,
    Direction(Direction),
    /// A chat width, from the built-in numbers or a preset.
    Width(ColumnWidth),
    /// Put back the files the last turn of the open chat wrote.
    UndoTurn,
    /// Look back over the open chat and suggest rules, style and skill changes (`/refine`).
    Refine,
    /// Say the last answer was good, or not.
    Rate(bool),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub command: Command,
    /// The kind of thing, shown small beside the row: "Chat", "Model", "Look"…
    pub group: &'static str,
    pub title: String,
    /// The key that does the same, or "current".
    pub hint: String,
}

/// What the palette needs to know to list its commands.
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub rtl: bool,
    /// Key and title of each chat, in the order of the chat list.
    pub chats: Vec<(u64, String)>,
    pub current_chat: Option<u64>,
    /// Model names, and the one the chat uses.
    /// (choice sent to the core, name shown)
    pub models: Vec<(String, String)>,
    pub current_model: String,
    /// The families of looks (Calm, Matte…), and the one in use.
    pub looks: Vec<String>,
    pub current_look: usize,
    /// Light, dark or follow the system; whether dark is showing; whether the desktop says anything about it.
    pub mode: ModeChoice,
    pub dark: bool,
    pub system_available: bool,
    pub direction: Direction,
    /// The chat width in use, and the presets the person saved.
    pub column: ColumnWidth,
    pub presets: Vec<Preset>,
    /// The open chat's last turn wrote files.
    pub can_undo_turn: bool,
    /// The open chat has a session the core knows and has answered at least once.
    pub chat_has_answers: bool,
}

fn entry(command: Command, group: &'static str, title: impl Into<String>, hint: impl Into<String>) -> Entry {
    Entry { command, group, title: title.into(), hint: hint.into() }
}

/// Every command, in the order shown when nothing is typed: places first, then the chats, models, looks, directions.
pub fn entries(s: &Snapshot) -> Vec<Entry> {
    let ar = s.rtl;
    let (g_do, g_tab, g_chat, g_model, g_look, g_dir) = if ar { ("أمر", "لوحة", "محادثة", "نموذج", "مظهر", "اتجاه") } else { ("Do", "Tab", "Chat", "Model", "Look", "Direction") };
    let current = if ar { "الحالي" } else { "current" };
    let mut v = vec![
        entry(Command::NewChat, g_do, if ar { "محادثة جديدة" } else { "New chat" }, "Ctrl+N"),
        entry(Command::Settings, g_do, if ar { "الإعدادات" } else { "Settings" }, "Ctrl+Shift+S"),
        entry(Command::ToggleChatList, g_do, if ar { "إظهار قائمة المحادثات أو إخفاؤها" } else { "Show or hide the chat list" }, "Ctrl+B"),
    ];
    if s.chat_has_answers {
        v.push(entry(Command::Refine, g_do, if ar { "تحسين: التعلم من هذه المحادثة" } else { "Refine: learn from this chat" }, ""));
        v.push(entry(Command::Rate(true), g_do, if ar { "كان الرد الأخير جيدًا" } else { "The last answer was good" }, ""));
        v.push(entry(Command::Rate(false), g_do, if ar { "لم يكن الرد الأخير صحيحًا" } else { "The last answer was not right" }, ""));
    }
    if s.can_undo_turn {
        v.push(entry(Command::UndoTurn, g_do, if ar { "التراجع عن تغييرات الملفات في آخر دور" } else { "Undo the file changes of the last turn" }, ""));
    }
    for p in DOCK_TABS {
        let hint = match p {
            Panel::Content => "Ctrl+Shift+O",
            Panel::Work => "Ctrl+Shift+W",
            Panel::Tree => "Ctrl+Shift+T",
            _ => "",
        };
        v.push(entry(Command::OpenTab(p), g_tab, if ar { format!("افتح {}", panel_label(p, true)) } else { format!("Open {}", panel_label(p, false)) }, hint));
    }
    for (key, title) in &s.chats {
        v.push(entry(Command::SwitchChat(*key), g_chat, title.clone(), if s.current_chat == Some(*key) { current } else { "" }));
    }
    for (m, label) in &s.models {
        v.push(entry(Command::Model(m.clone()), g_model, label.clone(), if *m == s.current_model { current } else { "" }));
    }
    for (i, l) in s.looks.iter().enumerate() {
        v.push(entry(Command::Look(i), g_look, l.clone(), if i == s.current_look { current } else { "" }));
    }
    let g_mode = if ar { "نمط" } else { "Mode" };
    let (to_dark, to_light) = (if ar { "التبديل إلى الداكن" } else { "Switch to dark" }, if ar { "التبديل إلى الفاتح" } else { "Switch to light" });
    v.push(entry(Command::ToggleMode, g_mode, if s.dark { to_light } else { to_dark }, "Ctrl+T"));
    for m in ModeChoice::ALL {
        if m == ModeChoice::System && !s.system_available {
            continue;
        }
        v.push(entry(Command::Mode(m), g_mode, m.label(ar), if m == s.mode { current } else { "" }));
    }
    for d in [Direction::Auto, Direction::Ltr, Direction::Rtl] {
        let title = match (d, ar) {
            (Direction::Auto, false) => "Follow the desktop language",
            (Direction::Ltr, false) => "Left to right",
            (Direction::Rtl, false) => "Right to left",
            (Direction::Auto, true) => "اتباع لغة سطح المكتب",
            (Direction::Ltr, true) => "من اليسار إلى اليمين",
            (Direction::Rtl, true) => "من اليمين إلى اليسار",
        };
        v.push(entry(Command::Direction(d), g_dir, title, if d == s.direction { current } else { "Ctrl+Shift+R" }));
    }
    let g_width = if ar { "عرض" } else { "Width" };
    let width_title = |w: ColumnWidth| if ar { format!("عرض المحادثة: {}", w.label(true)) } else { format!("Chat width: {}", w.label(false)) };
    for w in BUILT_IN_WIDTHS {
        v.push(entry(Command::Width(w), g_width, width_title(w), if w == s.column { current } else { "" }));
    }
    for p in &s.presets {
        let title = if ar { format!("عرض المحادثة: {} ({})", p.name, p.width.label(true)) } else { format!("Chat width: {} ({})", p.name, p.width.label(false)) };
        v.push(entry(Command::Width(p.width), g_width, title, if p.width == s.column { current } else { "" }));
    }
    v
}

/// The rows that match what was typed, best first. Every word typed must appear somewhere in the group or the
/// title, in any case; a word at the start of the title counts most, then one at the start of a word, then
/// anywhere. Rows that tie keep their order. Nothing typed keeps every row.
pub fn filter(entries: &[Entry], query: &str) -> Vec<usize> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    if words.is_empty() {
        return (0..entries.len()).collect();
    }
    let mut scored: Vec<(u32, usize)> = entries
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            let title = e.title.to_lowercase();
            let group = e.group.to_lowercase();
            let mut score = 0;
            for w in &words {
                score += if title.starts_with(w.as_str()) {
                    0
                } else if title.split(|c: char| !c.is_alphanumeric()).any(|t| t.starts_with(w.as_str())) {
                    1
                } else if title.contains(w.as_str()) {
                    2
                } else if group.contains(w.as_str()) {
                    3
                } else {
                    return None;
                };
            }
            Some((score, i))
        })
        .collect();
    scored.sort();
    scored.into_iter().map(|(_, i)| i).collect()
}

/// What is typed and which row is chosen.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Palette {
    pub query: String,
    pub selected: usize,
}

impl Palette {
    /// New words typed: the first row is chosen again.
    pub fn set_query(&mut self, q: &str) {
        if self.query != q {
            self.query = q.to_string();
            self.selected = 0;
        }
    }

    /// Move the choice up or down, going round at the ends. `len` is how many rows match.
    pub fn step(&mut self, down: bool, len: usize) {
        if len == 0 {
            self.selected = 0;
        } else if down {
            self.selected = (self.selected + 1) % len;
        } else {
            self.selected = (self.selected + len - 1) % len;
        }
    }

    /// The rows to draw when only `rows` fit: a stretch of the matches that always includes the chosen one.
    pub fn visible(&self, len: usize, rows: usize) -> std::ops::Range<usize> {
        let selected = self.selected.min(len.saturating_sub(1));
        let start = selected.saturating_sub(rows.saturating_sub(1)).min(len.saturating_sub(rows));
        let start = if selected < start { selected } else { start };
        start..(start + rows).min(len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap() -> Snapshot {
        Snapshot {
            rtl: false,
            chats: vec![(1, "Plan the Berlin trip".into()), (2, "Fix the failing export test".into()), (3, "رسالة إلى المالك".into())],
            current_chat: Some(1),
            models: vec![("Qwen3.6 35B".into(), "Qwen3.6 35B".into()), ("Tiel Coder".into(), "Tiel Coder".into())],
            current_model: "Qwen3.6 35B".into(),
            looks: vec!["Calm".into(), "Matte".into(), "Glass".into()],
            current_look: 1,
            mode: ModeChoice::Light,
            dark: false,
            system_available: true,
            direction: Direction::Auto,
            column: ColumnWidth::Px(860.),
            presets: vec![Preset { name: "Reading".into(), width: ColumnWidth::Px(1010.) }, Preset { name: "Whole screen".into(), width: ColumnWidth::Full }],
            can_undo_turn: false,
            chat_has_answers: false,
        }
    }

    #[test]
    fn undo_turn_is_offered_only_after_a_turn_that_wrote() {
        let has = |s: &Snapshot| entries(s).iter().any(|e| e.command == Command::UndoTurn);
        let mut s = snap();
        assert!(!has(&s));
        s.can_undo_turn = true;
        assert!(has(&s));
    }

    fn titles(e: &[Entry], idx: &[usize]) -> Vec<String> {
        idx.iter().map(|&i| e[i].title.clone()).collect()
    }

    #[test]
    fn everything_the_mock_names_can_be_reached() {
        let e = entries(&snap());
        let has = |f: &dyn Fn(&Command) -> bool| e.iter().any(|x| f(&x.command));
        assert!(has(&|c| *c == Command::NewChat) && has(&|c| *c == Command::Settings));
        assert!(has(&|c| matches!(c, Command::SwitchChat(3))) && has(&|c| matches!(c, Command::Model(m) if m == "Tiel Coder")));
        assert!(has(&|c| matches!(c, Command::Look(2))));
        for d in [Direction::Auto, Direction::Ltr, Direction::Rtl] {
            assert!(has(&|c| *c == Command::Direction(d)));
        }
        for p in DOCK_TABS {
            assert!(has(&|c| *c == Command::OpenTab(p)), "{p:?}");
        }
    }

    #[test]
    fn the_chat_widths_are_listed_by_number_and_presets_by_name_with_their_number() {
        let e = entries(&snap());
        let ts = titles(&e, &filter(&e, "width"));
        assert_eq!(ts, ["Chat width: 700 px", "Chat width: 900 px", "Chat width: 1100 px", "Chat width: Full", "Chat width: Reading (1010 px)", "Chat width: Whole screen (Full)"]);
        // choosing a row hands over the exact width
        let pick = |t: &str| e.iter().find(|x| x.title == t).unwrap().command.clone();
        assert_eq!(pick("Chat width: Reading (1010 px)"), Command::Width(ColumnWidth::Px(1010.)));
        assert_eq!(pick("Chat width: Full"), Command::Width(ColumnWidth::Full));
        // a number typed finds the rows with that number
        assert_eq!(titles(&e, &filter(&e, "1010")), ["Chat width: Reading (1010 px)"]);
        assert_eq!(titles(&e, &filter(&e, "reading")), ["Chat width: Reading (1010 px)"]);
        // the width in use is marked, and no width row is marked when none matches
        let mut s = snap();
        s.column = ColumnWidth::Px(900.);
        let e = entries(&s);
        assert_eq!(e.iter().find(|x| x.title == "Chat width: 900 px").unwrap().hint, "current");
        assert_eq!(e.iter().filter(|x| x.group == "Width" && x.hint == "current").count(), 1);
        s.column = ColumnWidth::Px(777.);
        assert_eq!(entries(&s).iter().filter(|x| x.group == "Width" && x.hint == "current").count(), 0);
        // Arabic names the group and keeps the numbers
        s.rtl = true;
        let ar = entries(&s);
        assert!(ar.iter().any(|x| x.group == "عرض" && x.title == "عرض المحادثة: 900 px"));
    }

    #[test]
    fn the_current_thing_is_marked_and_keys_are_shown() {
        let e = entries(&snap());
        let hint = |c: Command| e.iter().find(|x| x.command == c).unwrap().hint.clone();
        assert_eq!(hint(Command::SwitchChat(1)), "current");
        assert_eq!(hint(Command::SwitchChat(2)), "");
        assert_eq!(hint(Command::Look(1)), "current");
        assert_eq!(hint(Command::Look(0)), "");
        assert_eq!(hint(Command::ToggleMode), "Ctrl+T");
        assert_eq!(hint(Command::Mode(ModeChoice::Light)), "current");
        assert_eq!(hint(Command::Model("Qwen3.6 35B".into())), "current");
        assert_eq!(hint(Command::Direction(Direction::Auto)), "current");
        assert_eq!(hint(Command::NewChat), "Ctrl+N");
    }

    #[test]
    fn typing_narrows_the_list_and_puts_the_best_match_first() {
        let e = entries(&snap());
        assert_eq!(filter(&e, "").len(), e.len(), "nothing typed shows everything, in order");
        assert_eq!(titles(&e, &filter(&e, "berlin")), ["Plan the Berlin trip"]);
        assert_eq!(titles(&e, &filter(&e, "BERLIN")), ["Plan the Berlin trip"], "case does not matter");
        let r = titles(&e, &filter(&e, "settings"));
        assert_eq!(r, ["Settings"]);
        let r = titles(&e, &filter(&e, "calm"));
        assert_eq!(r, ["Calm"], "one entry a family, not one a mode");
        // a word at the start of the title beats one in the middle
        let r = titles(&e, &filter(&e, "new"));
        assert_eq!(r[0], "New chat");
        // every word must match
        assert_eq!(titles(&e, &filter(&e, "switch dark")), ["Switch to dark"]);
        assert!(filter(&e, "zzzz").is_empty());
        // the group name is searchable: "model" lists the models
        assert_eq!(titles(&e, &filter(&e, "model")), ["Qwen3.6 35B", "Tiel Coder"]);
    }

    #[test]
    fn light_and_dark_are_one_switch_and_follow_the_system_only_when_the_desktop_says_something() {
        let mut s = snap();
        let e = entries(&s);
        let mode_titles = |e: &[Entry]| e.iter().filter(|x| matches!(x.command, Command::Mode(_) | Command::ToggleMode)).map(|x| x.title.clone()).collect::<Vec<_>>();
        assert_eq!(mode_titles(&e), ["Switch to dark", "Light", "Dark", "Follow the system"]);
        s.dark = true;
        s.mode = ModeChoice::Dark;
        let e = entries(&s);
        assert_eq!(mode_titles(&e)[0], "Switch to light", "the switch names where it goes");
        assert_eq!(e.iter().find(|x| x.command == Command::Mode(ModeChoice::Dark)).unwrap().hint, "current");
        s.system_available = false;
        assert_eq!(mode_titles(&entries(&s)), ["Switch to light", "Light", "Dark"], "no desktop setting, no follow entry");
        // no separate light and dark entries for each family any more
        assert!(entries(&s).iter().filter(|x| matches!(x.command, Command::Look(_))).all(|x| !x.title.ends_with(" dark") && !x.title.ends_with(" light")));
        s.rtl = true;
        assert!(entries(&s).iter().filter(|x| matches!(x.command, Command::Mode(_) | Command::ToggleMode)).all(|x| !x.title.is_ascii()));
    }

    #[test]
    fn arabic_titles_are_found_and_the_labels_follow_the_language() {
        let mut s = snap();
        s.rtl = true;
        let e = entries(&s);
        assert_eq!(titles(&e, &filter(&e, "المالك")), ["رسالة إلى المالك"]);
        assert!(e.iter().all(|x| !x.title.is_empty() && !x.group.is_empty()));
        assert_eq!(e[0].title, "محادثة جديدة");
    }

    #[test]
    fn the_choice_goes_round_and_resets_when_the_words_change() {
        let mut p = Palette::default();
        p.step(false, 5);
        assert_eq!(p.selected, 4, "up from the first row wraps to the last");
        p.step(true, 5);
        assert_eq!(p.selected, 0);
        p.step(true, 5);
        p.step(true, 5);
        assert_eq!(p.selected, 2);
        p.set_query("abc");
        assert_eq!(p.selected, 0);
        p.step(true, 0);
        assert_eq!(p.selected, 0, "an empty list has nothing to choose");
        p.selected = 3;
        p.set_query("abc");
        assert_eq!(p.selected, 3, "the same words keep the choice");
    }

    #[test]
    fn the_chosen_row_is_always_in_the_part_that_is_drawn() {
        let mut p = Palette::default();
        assert_eq!(p.visible(30, 8), 0..8);
        p.selected = 7;
        assert_eq!(p.visible(30, 8), 0..8);
        p.selected = 8;
        assert_eq!(p.visible(30, 8), 1..9);
        p.selected = 29;
        assert_eq!(p.visible(30, 8), 22..30);
        p.selected = 2;
        let v = p.visible(30, 8);
        assert!(v.contains(&2) && v.len() == 8);
        assert_eq!(p.visible(3, 8), 0..3, "a short list is shown whole");
        assert_eq!(p.visible(0, 8), 0..0);
    }
}
