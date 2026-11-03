//! What the chat screen shows, and how each thing the core says changes it. No window code here, so it is
//! tested without a screen.

use std::time::Instant;

use lh_client::ServerEvent;

/// Starts the changes of an allowed file write, as added under the step's result.
pub const DIFF_MARK: &str = "Changes to ";

#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    User(String),
    /// `worked` is the seconds of thinking and tool use that came before this answer began.
    Assistant { text: String, reasoning: String, worked: Option<u64> },
    Tool { id: String, name: String, arguments: String, result: Option<String> },
    Notice(String),
    /// Something the assistant showed (a table, chart, document, picture or small app), drawn in the chat itself.
    Shown(crate::content::Content),
    Error(String),
}

/// A question the core is waiting on: may this tool run?
#[derive(Clone, Debug, PartialEq)]
pub struct Ask {
    pub id: String,
    pub tool: String,
    pub what: String,
    pub reason: String,
    pub worker: String,
    /// More to look at before answering, such as the diff of a file write. Often empty.
    pub detail: String,
}

/// How a chat works with the computer. A chat without tools only talks; an assistant works from the home
/// folder and a project works inside one folder. Both ask before anything that matters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Chat,
    Assistant,
    Project,
}

impl Mode {
    pub fn parse(s: &str) -> Mode {
        match s {
            "assistant" => Mode::Assistant,
            "project" => Mode::Project,
            _ => Mode::Chat,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Chat => "chat",
            Mode::Assistant => "assistant",
            Mode::Project => "project",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Chat {
    /// Local key, stable for the life of the app; the session id comes later and can change.
    pub key: u64,
    pub session: Option<String>,
    pub title: String,
    pub model: String,
    pub blocks: Vec<Block>,
    pub busy: bool,
    pub asks: Vec<Ask>,
    pub context_tokens: Option<u64>,
    /// Opened from the stored list and not yet loaded in the core.
    pub dormant: bool,
    pub incognito: bool,
    pub mode: Mode,
    /// The project folder, for a chat in project mode.
    pub project: String,
    /// Unix seconds of the last message, for the sidebar; 0 when unknown.
    pub last: i64,
    turn_had_text: bool,
    /// When the thinking and tool use before the next answer began.
    work_since: Option<Instant>,
    /// How long and how fast the model has been thinking since the last thing it wrote or did.
    pub think: crate::thinking::Thinking,
}

pub fn unix_now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

pub const NEW_TITLE: &str = "New chat";

/// A chat title on one line: any run of whitespace (newlines too) becomes one space, the ends are trimmed. A title
/// from a model can start with a line break, which would draw an empty first line in the sidebar.
pub fn one_line(title: &str) -> String {
    title.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A title cut to at most `max` characters with "…" at its logical end. GPUI puts its own ellipsis at the right
/// edge, which is the wrong end of a right-to-left title; a "…" in the text is placed by the bidi rules, so it
/// lands on the left of an Arabic title.
pub fn truncate_title(title: &str, max: usize) -> String {
    if title.chars().count() <= max {
        return title.to_string();
    }
    let cut: String = title.chars().take(max.saturating_sub(1)).collect();
    format!("{}…", cut.trim_end())
}

/// A tool step in plain words, like "Read garden/notes.md" or "Run cargo test", for the Work panel. The raw
/// arguments stay one click away; this is only the line a person reads.
pub fn step_words(name: &str, arguments: &str) -> String {
    let args: serde_json::Value = serde_json::from_str(arguments).unwrap_or(serde_json::Value::Null);
    let pick = |keys: &[&str]| keys.iter().find_map(|k| args.get(*k).and_then(|v| v.as_str()).map(|v| one_line(v)));
    let target = pick(&["path", "file", "command", "url", "query", "name", "text"]);
    let verb = match name {
        "read_file" => "Read",
        "write_file" => "Wrote",
        "edit_file" => "Edited",
        "list_dir" | "list_files" => "Listed",
        "run_command" | "bash" | "shell" => "Ran",
        "web_fetch" | "fetch" => "Opened",
        "web_search" | "search" => "Searched for",
        "recall" => "Looked up",
        "remember" => "Saved",
        _ => "",
    };
    let verb = if verb.is_empty() {
        let mut w = name.replace('_', " ");
        if let Some(f) = w.get(0..1).map(|f| f.to_uppercase()) { w.replace_range(0..1, &f); }
        w
    } else { verb.to_string() };
    match target {
        Some(t) => truncate_title(&format!("{verb} {t}"), 120),
        None => verb,
    }
}

impl Chat {
    pub fn new(key: u64, model: &str) -> Chat {
        Chat { key, session: None, title: NEW_TITLE.into(), model: model.into(), blocks: vec![], busy: false, asks: vec![], context_tokens: None, dormant: false, incognito: false, mode: Mode::Chat, project: String::new(), last: 0, turn_had_text: false, work_since: None, think: Default::default() }
    }

    /// A chat that is only a name in the list until the user opens it.
    pub fn stored(key: u64, session: &str, title: &str, model: &str) -> Chat {
        let mut c = Chat::new(key, model);
        c.session = Some(session.into());
        let t = one_line(title);
        c.title = if t.is_empty() { NEW_TITLE.into() } else { t };
        c.dormant = true;
        c
    }

    pub fn push_user(&mut self, text: &str) {
        if self.title == NEW_TITLE {
            self.title = one_line(text).chars().take(40).collect();
        }
        self.blocks.push(Block::User(text.into()));
        self.last = unix_now();
        self.busy = true;
        self.turn_had_text = false;
        self.work_since = Some(Instant::now());
        self.think.reset();
    }

    /// Fill from the stored history of a chat.
    pub fn load_history(&mut self, messages: &[(String, String)]) {
        self.blocks = messages
            .iter()
            .filter_map(|(role, text)| match role.as_str() {
                "user" => Some(Block::User(text.clone())),
                "assistant" => Some(Block::Assistant { text: text.clone(), reasoning: String::new(), worked: None }),
                "summary" => Some(Block::Notice(format!("Summary of another path: {text}"))),
                _ => None,
            })
            .collect();
    }

    fn assistant(&mut self) -> &mut Block {
        if !matches!(self.blocks.last(), Some(Block::Assistant { .. })) {
            self.blocks.push(Block::Assistant { text: String::new(), reasoning: String::new(), worked: None });
        }
        self.blocks.last_mut().unwrap()
    }

    pub fn apply(&mut self, ev: &ServerEvent) {
        self.apply_at(ev, Instant::now());
    }

    /// `apply` with the time the event arrived, so the seconds an answer waited for can be tested.
    pub fn apply_at(&mut self, ev: &ServerEvent, now: Instant) {
        // thinking and tool use start a stretch of work; the answer that follows says how long it was
        if matches!(ev, ServerEvent::Reasoning(_) | ServerEvent::ToolStart { .. }) && self.work_since.is_none() {
            self.work_since = Some(now);
        }
        match ev {
            ServerEvent::Text(t) => {
                self.turn_had_text = true;
                let since = self.work_since;
                let mut began = false;
                if let Block::Assistant { text, worked, .. } = self.assistant() {
                    if text.is_empty() && !t.is_empty() {
                        *worked = since.map(|s| now.saturating_duration_since(s).as_secs());
                        began = true;
                    }
                    text.push_str(t);
                }
                if began {
                    self.work_since = None;
                }
                self.think.reset();
            }
            ServerEvent::Reasoning(t) => {
                if let Block::Assistant { reasoning, .. } = self.assistant() {
                    reasoning.push_str(t);
                }
                if !t.is_empty() {
                    self.think.on_piece(now);
                }
            }
            ServerEvent::ToolStart { id, name, arguments } => {
                self.think.reset();
                self.blocks.push(Block::Tool { id: id.clone(), name: name.clone(), arguments: arguments.clone(), result: None })
            }
            ServerEvent::ToolResult { id, text } => {
                if let Some(Block::Tool { result, .. }) = self.blocks.iter_mut().rev().find(|b| matches!(b, Block::Tool { id: i, .. } if i == id)) {
                    // a diff that came first stays under the result
                    *result = Some(match result.take() {
                        Some(d) if d.starts_with(DIFF_MARK) => format!("{text}\n\n{d}"),
                        _ => text.clone(),
                    });
                }
            }
            ServerEvent::Diff { id, path, diff } => {
                // shown inside the step, which stays folded until opened
                if let Some(Block::Tool { result, .. }) = self.blocks.iter_mut().rev().find(|b| matches!(b, Block::Tool { id: i, .. } if i == id)) {
                    let d = format!("{DIFF_MARK}{path}\n{diff}");
                    *result = Some(match result.take() {
                        Some(r) => format!("{r}\n\n{d}"),
                        None => d,
                    });
                }
            }
            ServerEvent::Ask { id, tool, what, reason, worker, detail } => self.asks.push(Ask { id: id.clone(), tool: tool.clone(), what: what.clone(), reason: reason.clone(), worker: worker.clone(), detail: detail.clone() }),
            ServerEvent::Usage { prompt_tokens, completion_tokens, .. } => self.context_tokens = Some(prompt_tokens + completion_tokens),
            ServerEvent::Done { empty } => {
                self.end_turn();
                if *empty && !self.turn_had_text {
                    self.blocks.push(Block::Notice("The model sent no text. If it used up its limit while thinking, try again or choose a larger limit.".into()));
                }
            }
            ServerEvent::StepLimit => {
                self.end_turn();
                self.blocks.push(Block::Notice("Stopped: this took more steps than one turn allows.".into()));
            }
            ServerEvent::Refused(m) => {
                self.end_turn();
                self.blocks.push(Block::Error(m.clone()));
            }
            ServerEvent::Compacted { .. } => self.blocks.push(Block::Notice("Older messages were summarised to keep the chat within the model's limit.".into())),
            ServerEvent::Notice(m) => self.blocks.push(Block::Notice(m.clone())),
            ServerEvent::Error(m) => {
                self.end_turn();
                if m == "stopped" {
                    self.blocks.push(Block::Notice("Stopped.".into()));
                } else {
                    self.blocks.push(Block::Error(m.clone()));
                }
            }
            ServerEvent::Show { title, kind, content, path } => self.blocks.push(Block::Shown(crate::content::Content::from_show(title, kind, content, path.as_deref()))),
            ServerEvent::Finish(_) | ServerEvent::Other => {}
        }
    }

    /// The stream ended, however it ended: nothing is running and no question can still be answered.
    pub fn end_turn(&mut self) {
        self.busy = false;
        self.asks.clear();
        self.think.reset();
    }

    /// Did the last turn write a file? Then "undo the last turn" has something to put back.
    pub fn last_turn_wrote(&self) -> bool {
        !self.busy && self.session.is_some() && self.blocks.iter().rev().take_while(|b| !matches!(b, Block::User(_))).any(|b| matches!(b, Block::Tool { name, .. } if name == "write_file"))
    }

    /// A chat nobody has written in and the core has never stored. It is not listed.
    pub fn is_draft(&self) -> bool {
        self.session.is_none() && self.blocks.is_empty() && !self.dormant
    }

    pub fn answer(&mut self, ask: &str) {
        self.asks.retain(|a| a.id != ask);
    }
}

/// All chats on screen, newest first, and which one is shown.
#[derive(Default)]
pub struct Chats {
    pub list: Vec<Chat>,
    pub active: usize,
    next_key: u64,
}

impl Chats {
    /// A new chat goes to the top of the list and is shown.
    pub fn new_chat(&mut self, model: &str) -> u64 {
        self.next_key += 1;
        self.list.insert(0, Chat::new(self.next_key, model));
        self.active = 0;
        self.next_key
    }

    /// A chat the core has stored goes to the end (the core lists newest first), without changing what is shown.
    pub fn add_stored(&mut self, session: &str, title: &str, model: &str) -> u64 {
        self.next_key += 1;
        self.list.push(Chat::stored(self.next_key, session, title, model));
        self.next_key
    }

    /// Whether any chat has been written or stored; drafts do not count.
    pub fn has_history(&self) -> bool {
        self.list.iter().any(|c| !c.is_draft())
    }

    /// Show a blank chat: the one already waiting if there is one, else a new one.
    pub fn show_draft(&mut self, model: &str) {
        match self.list.iter().position(Chat::is_draft) {
            Some(i) => self.active = i,
            None => {
                self.new_chat(model);
            }
        }
    }

    pub fn current(&self) -> Option<&Chat> {
        self.list.get(self.active)
    }

    pub fn current_mut(&mut self) -> Option<&mut Chat> {
        self.list.get_mut(self.active)
    }

    pub fn by_key(&mut self, key: u64) -> Option<&mut Chat> {
        self.list.iter_mut().find(|c| c.key == key)
    }

    pub fn by_session(&mut self, session: &str) -> Option<&mut Chat> {
        self.list.iter_mut().find(|c| c.session.as_deref() == Some(session))
    }

    pub fn select(&mut self, key: u64) {
        if let Some(i) = self.list.iter().position(|c| c.key == key) {
            self.active = i;
        }
    }

    /// Remove a chat. If it was the one shown, the next one down is shown; if none is left, a new one starts.
    pub fn remove(&mut self, key: u64, model: &str) -> Option<Chat> {
        let i = self.list.iter().position(|c| c.key == key)?;
        let gone = self.list.remove(i);
        if self.list.is_empty() {
            self.new_chat(model);
        } else if i < self.active {
            self.active -= 1;
        } else if i == self.active {
            self.active = i.min(self.list.len() - 1);
        }
        Some(gone)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_tool_step_reads_as_plain_words() {
        use super::step_words;
        assert_eq!(step_words("read_file", r#"{"path":"~/garden/notes.md"}"#), "Read ~/garden/notes.md");
        assert_eq!(step_words("run_command", r#"{"command":"cargo test"}"#), "Ran cargo test");
        assert_eq!(step_words("make_chart", "{}"), "Make chart");
        assert_eq!(step_words("odd", "not json"), "Odd");
    }

    #[test]
    fn long_titles_end_with_the_ellipsis_in_the_text() {
        assert_eq!(truncate_title("short", 10), "short");
        assert_eq!(truncate_title("abcdefghij", 6), "abcde…");
        assert_eq!(truncate_title("abc def ghi", 5), "abc…");
        let t = truncate_title("اكتب فقرة عربية طويل جدا", 10);
        assert!(t.ends_with('…') && t.chars().count() <= 10);
        assert!(t.starts_with("اكتب"));
    }

    #[test]
    fn titles_are_one_line_and_arabic_survives() {
        assert_eq!(one_line("\n  كتابة \r\n الرسالة  "), "كتابة الرسالة");
        assert_eq!(Chat::stored(1, "s", "\nكتابة رسالة", "m").title, "كتابة رسالة");
        assert_eq!(Chat::stored(1, "s", " \n ", "m").title, NEW_TITLE);
    }

    use super::*;

    fn text(t: &str) -> ServerEvent {
        ServerEvent::Text(t.into())
    }

    #[test]
    fn streamed_text_builds_one_assistant_block_and_the_title_comes_from_the_first_message() {
        let mut c = Chat::new(1, "m");
        c.push_user("  Hello   there,\nfriend  ");
        assert_eq!(c.title, "Hello there, friend");
        assert!(c.busy);
        c.apply(&ServerEvent::Reasoning("hmm".into()));
        c.apply(&text("Hi"));
        c.apply(&text(" there"));
        c.apply(&ServerEvent::Done { empty: false });
        assert_eq!(c.blocks, vec![Block::User("  Hello   there,\nfriend  ".into()), Block::Assistant { text: "Hi there".into(), reasoning: "hmm".into(), worked: Some(0) }]);
        assert!(!c.busy);
        c.push_user("again");
        assert_eq!(c.title, "Hello there, friend", "the title is set once");
    }

    #[test]
    fn an_answer_remembers_how_long_the_work_before_it_took() {
        let t0 = Instant::now();
        let at = |s: u64| t0 + std::time::Duration::from_secs(s);
        let mut c = Chat::new(1, "m");
        c.push_user("find a hotel");
        c.work_since = Some(t0);
        c.apply_at(&ServerEvent::ToolStart { id: "t".into(), name: "web_search".into(), arguments: "{}".into() }, at(1));
        c.apply_at(&ServerEvent::ToolResult { id: "t".into(), text: "x".into() }, at(9));
        c.apply_at(&text("Found "), at(14));
        c.apply_at(&text("three."), at(15));
        assert!(matches!(c.blocks.last(), Some(Block::Assistant { worked: Some(14), .. })), "14 s from the start of the work to the first word, not to the last");
        // a second stretch of work in the same turn is timed on its own
        c.apply_at(&ServerEvent::ToolStart { id: "u".into(), name: "show".into(), arguments: "{}".into() }, at(20));
        c.apply_at(&text("Done."), at(23));
        assert!(matches!(c.blocks.last(), Some(Block::Assistant { worked: Some(3), .. })));
        // an answer with no work before it has no time to report
        let mut d = Chat::new(2, "m");
        d.push_user("hi");
        d.work_since = None;
        d.apply_at(&text("Hello"), at(5));
        assert!(matches!(d.blocks.last(), Some(Block::Assistant { worked: None, .. })));
    }

    #[test]
    fn modes_read_and_write_the_names_the_core_uses() {
        for m in [Mode::Chat, Mode::Assistant, Mode::Project] {
            assert_eq!(Mode::parse(m.as_str()), m);
        }
        assert_eq!(Mode::parse("something else"), Mode::Chat);
        assert_eq!(Chat::new(1, "m").mode, Mode::Chat);
    }

    #[test]
    fn something_the_assistant_shows_is_kept_in_the_chat() {
        let mut c = Chat::new(1, "m");
        c.push_user("x");
        c.apply(&ServerEvent::Show { title: "Plan".into(), kind: "markdown".into(), content: "# Plan".into(), path: None });
        assert!(matches!(c.blocks.last(), Some(Block::Shown(x)) if x.title == "Plan" && x.body == "# Plan"), "the whole thing is kept, to draw in the chat");
        assert!(c.busy, "showing something does not end the turn");
    }

    #[test]
    fn long_titles_are_cut() {
        let mut c = Chat::new(1, "m");
        c.push_user(&"word ".repeat(30));
        assert_eq!(c.title.chars().count(), 40);
    }

    #[test]
    fn tools_questions_and_usage_are_tracked() {
        let mut c = Chat::new(1, "m");
        c.push_user("list files");
        c.apply(&ServerEvent::ToolStart { id: "t1".into(), name: "run_command".into(), arguments: "{\"command\":\"ls\"}".into() });
        c.apply(&ServerEvent::Ask { id: "a1".into(), tool: "run_command".into(), what: "ls".into(), reason: "reads the folder".into(), worker: String::new(), detail: String::new() });
        assert_eq!(c.asks.len(), 1);
        c.answer("a1");
        assert!(c.asks.is_empty());
        c.apply(&ServerEvent::ToolResult { id: "t1".into(), text: "a.txt".into() });
        c.apply(&text("One file."));
        c.apply(&ServerEvent::Usage { prompt_tokens: 100, completion_tokens: 20, cached_tokens: 0 });
        c.apply(&ServerEvent::Done { empty: false });
        assert!(matches!(&c.blocks[1], Block::Tool { result: Some(r), .. } if r == "a.txt"));
        assert!(matches!(&c.blocks[2], Block::Assistant { text, .. } if text == "One file."), "text after a tool starts a new block");
        assert_eq!(c.context_tokens, Some(120));
    }

    #[test]
    fn every_way_a_turn_can_end_clears_busy_and_open_questions() {
        for ev in [ServerEvent::Done { empty: false }, ServerEvent::StepLimit, ServerEvent::Refused("limit".into()), ServerEvent::Error("boom".into()), ServerEvent::Error("stopped".into())] {
            let mut c = Chat::new(1, "m");
            c.push_user("x");
            c.apply(&ServerEvent::Ask { id: "a".into(), tool: "t".into(), what: "w".into(), reason: String::new(), worker: String::new(), detail: String::new() });
            c.apply(&ev);
            assert!(!c.busy, "{ev:?}");
            assert!(c.asks.is_empty(), "{ev:?}");
        }
    }

    #[test]
    fn stopping_and_empty_answers_are_said_plainly() {
        let mut c = Chat::new(1, "m");
        c.push_user("x");
        c.apply(&ServerEvent::Error("stopped".into()));
        assert_eq!(c.blocks.last(), Some(&Block::Notice("Stopped.".into())));
        let mut c = Chat::new(1, "m");
        c.push_user("x");
        c.apply(&ServerEvent::Done { empty: true });
        assert!(matches!(c.blocks.last(), Some(Block::Notice(m)) if m.contains("no text")));
        let mut c = Chat::new(1, "m");
        c.push_user("x");
        c.apply(&text("fine"));
        c.apply(&ServerEvent::Done { empty: true });
        assert!(matches!(c.blocks.last(), Some(Block::Assistant { .. })), "no notice when text did arrive");
    }

    #[test]
    fn stored_history_fills_a_dormant_chat() {
        let mut c = Chat::stored(7, "s3", "", "m");
        assert!(c.dormant);
        assert_eq!(c.title, NEW_TITLE);
        c.load_history(&[("user".into(), "q".into()), ("assistant".into(), "a".into()), ("system".into(), "ignored".into())]);
        assert_eq!(c.blocks.len(), 2);
    }

    #[test]
    fn new_chats_go_on_top_and_stored_ones_below_without_changing_the_view() {
        let mut cs = Chats::default();
        let a = cs.new_chat("m");
        let b = cs.new_chat("m");
        assert_eq!(cs.list[0].key, b);
        assert_eq!(cs.current().unwrap().key, b, "the new chat is shown");
        let s = cs.add_stored("s9", "old chat", "m");
        assert_eq!(cs.list.last().unwrap().key, s);
        assert_eq!(cs.current().unwrap().key, b, "adding stored chats does not move the view");
        assert_eq!(cs.by_session("s9").unwrap().title, "old chat");
        assert!(cs.by_session("nope").is_none());
        cs.select(a);
        assert_eq!(cs.active, 1);
    }

    #[test]
    fn removing_the_shown_chat_shows_the_next_one_down_and_an_empty_list_starts_a_new_chat() {
        let mut cs = Chats::default();
        let a = cs.new_chat("m");
        let b = cs.new_chat("m");
        let c = cs.new_chat("m");
        // list is [c, b, a]
        cs.select(b);
        cs.remove(b, "m");
        assert_eq!(cs.current().unwrap().key, a, "the one below");
        cs.remove(a, "m");
        assert_eq!(cs.current().unwrap().key, c, "last one removed: the one above");
        cs.remove(c, "m");
        assert_eq!(cs.list.len(), 1, "a hidden blank draft keeps the new-chat screen working");
        assert!(cs.current().unwrap().is_draft());
        assert!(!cs.has_history(), "deleting every chat leaves nothing to list");
        cs.show_draft("m");
        assert_eq!(cs.list.len(), 1, "the blank draft is reused");
        assert!(cs.remove(999, "m").is_none());
    }

    #[test]
    fn removing_a_chat_above_the_shown_one_keeps_the_shown_one() {
        let mut cs = Chats::default();
        let a = cs.new_chat("m");
        let _b = cs.new_chat("m");
        let c = cs.new_chat("m");
        cs.select(a);
        cs.remove(c, "m");
        assert_eq!(cs.current().unwrap().key, a);
    }

    #[test]
    fn thinking_is_counted_from_the_reasoning_pieces_and_stops_when_something_else_happens() {
        let mut c = Chat::new(1, "m");
        c.push_user("hi");
        let t0 = Instant::now();
        for i in 0..40u64 {
            c.apply_at(&ServerEvent::Reasoning("x".into()), t0 + std::time::Duration::from_millis(i * 100));
        }
        let s = c.think.snapshot(t0 + std::time::Duration::from_millis(3900)).unwrap();
        assert_eq!((s.tokens, s.secs), (40, 3));
        c.apply_at(&ServerEvent::Reasoning(String::new()), t0 + std::time::Duration::from_secs(5));
        assert_eq!(c.think.snapshot(t0 + std::time::Duration::from_secs(5)).unwrap().tokens, 40, "an empty piece is not a token");
        // the answer begins: thinking is over
        c.apply_at(&ServerEvent::Text("Hello".into()), t0 + std::time::Duration::from_secs(6));
        assert!(!c.think.active());
        // a tool starts: over; the next thinking starts counting from nothing
        c.apply_at(&ServerEvent::Reasoning("y".into()), t0 + std::time::Duration::from_secs(7));
        assert!(c.think.active());
        c.apply(&ServerEvent::ToolStart { id: "1".into(), name: "ls".into(), arguments: "{}".into() });
        assert!(!c.think.active());
        c.apply(&ServerEvent::Reasoning("z".into()));
        assert_eq!(c.think.snapshot(Instant::now()).unwrap().tokens, 1);
        // the turn ends, however it ends
        c.apply(&ServerEvent::Error("stopped".into()));
        assert!(!c.think.active());
        c.push_user("again");
        c.apply(&ServerEvent::Reasoning("w".into()));
        c.push_user("and again");
        assert!(!c.think.active(), "a new message starts clean");
    }
}

/// One line for the chat after "undo the last turn": what was put back and what was left alone, and why.
pub fn undo_summary(v: &serde_json::Value) -> String {
    let files = v["files"].as_array().cloned().unwrap_or_default();
    if files.is_empty() {
        return "The last turn changed no files that can still be put back.".into();
    }
    let name = |f: &serde_json::Value| f["path"].as_str().map(|p| p.rsplit('/').next().unwrap_or(p).to_string()).unwrap_or_default();
    let back: Vec<String> = files.iter().filter(|f| f["result"] != "refused").map(name).collect();
    let left: Vec<String> = files.iter().filter(|f| f["result"] == "refused").map(|f| format!("{} ({})", name(f), f["message"].as_str().unwrap_or(""))).collect();
    let mut s = String::new();
    if !back.is_empty() {
        s.push_str(&format!("Put back {}.", back.join(", ")));
    }
    if !left.is_empty() {
        s.push_str(&format!("{}Left alone: {}.", if s.is_empty() { "" } else { " " }, left.join(", ")));
    }
    s
}

/// One line after "Refine": how many suggestions wait, and where to find them.
pub fn refine_summary(v: &serde_json::Value) -> String {
    let waiting = v["proposals"].as_array().map_or(0, Vec::len);
    let applied = v["applied"].as_array().map_or(0, Vec::len);
    match (waiting, applied) {
        (0, 0) => "Nothing new worth keeping from this chat.".into(),
        (w, 0) => format!("{w} suggestion{} wait{} for you in Settings, Learning.", if w == 1 { "" } else { "s" }, if w == 1 { "s" } else { "" }),
        (0, a) => format!("Learned {a} thing{} from this chat at once (silent learning is on). See Settings, Learning.", if a == 1 { "" } else { "s" }),
        (w, a) => format!("Learned {a} at once; {w} more wait for you in Settings, Learning."),
    }
}

#[cfg(test)]
mod undo_tests {
    use serde_json::json;

    #[test]
    fn the_undo_line_says_what_came_back_and_what_did_not() {
        assert_eq!(super::undo_summary(&json!({"files": []})), "The last turn changed no files that can still be put back.");
        let v = json!({"files": [{"path": "/p/a.txt", "result": "restored"}, {"path": "/p/b.txt", "result": "removed"}, {"path": "/p/c.txt", "result": "refused", "message": "the file was changed after the harness wrote it, so it was left alone"}]});
        assert_eq!(super::refine_summary(&json!({"proposals": [1, 2], "applied": []})), "2 suggestions wait for you in Settings, Learning.");
        assert_eq!(super::refine_summary(&json!({"proposals": [], "applied": []})), "Nothing new worth keeping from this chat.");
        assert_eq!(super::undo_summary(&v), "Put back a.txt, b.txt. Left alone: c.txt (the file was changed after the harness wrote it, so it was left alone).");
    }
}
