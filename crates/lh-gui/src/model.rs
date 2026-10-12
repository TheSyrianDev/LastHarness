//! What the chat screen shows, and how each thing the core says changes it. No window code here, so it is
//! tested without a screen.

use lh_client::ServerEvent;

#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    User(String),
    Assistant { text: String, reasoning: String },
    Tool { id: String, name: String, arguments: String, result: Option<String> },
    Notice(String),
    /// The assistant opened something beside the chat; the title lets the chat offer to open it again.
    Shown(String),
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
    turn_had_text: bool,
}

pub const NEW_TITLE: &str = "New chat";

impl Chat {
    pub fn new(key: u64, model: &str) -> Chat {
        Chat { key, session: None, title: NEW_TITLE.into(), model: model.into(), blocks: vec![], busy: false, asks: vec![], context_tokens: None, dormant: false, incognito: false, mode: Mode::Chat, project: String::new(), turn_had_text: false }
    }

    /// A chat that is only a name in the list until the user opens it.
    pub fn stored(key: u64, session: &str, title: &str, model: &str) -> Chat {
        let mut c = Chat::new(key, model);
        c.session = Some(session.into());
        c.title = if title.is_empty() { NEW_TITLE.into() } else { title.into() };
        c.dormant = true;
        c
    }

    pub fn push_user(&mut self, text: &str) {
        if self.title == NEW_TITLE {
            self.title = text.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(40).collect();
        }
        self.blocks.push(Block::User(text.into()));
        self.busy = true;
        self.turn_had_text = false;
    }

    /// Fill from the stored history of a chat.
    pub fn load_history(&mut self, messages: &[(String, String)]) {
        self.blocks = messages
            .iter()
            .filter_map(|(role, text)| match role.as_str() {
                "user" => Some(Block::User(text.clone())),
                "assistant" => Some(Block::Assistant { text: text.clone(), reasoning: String::new() }),
                "summary" => Some(Block::Notice(format!("Summary of another path: {text}"))),
                _ => None,
            })
            .collect();
    }

    fn assistant(&mut self) -> &mut Block {
        if !matches!(self.blocks.last(), Some(Block::Assistant { .. })) {
            self.blocks.push(Block::Assistant { text: String::new(), reasoning: String::new() });
        }
        self.blocks.last_mut().unwrap()
    }

    pub fn apply(&mut self, ev: &ServerEvent) {
        match ev {
            ServerEvent::Text(t) => {
                self.turn_had_text = true;
                if let Block::Assistant { text, .. } = self.assistant() {
                    text.push_str(t);
                }
            }
            ServerEvent::Reasoning(t) => {
                if let Block::Assistant { reasoning, .. } = self.assistant() {
                    reasoning.push_str(t);
                }
            }
            ServerEvent::ToolStart { id, name, arguments } => self.blocks.push(Block::Tool { id: id.clone(), name: name.clone(), arguments: arguments.clone(), result: None }),
            ServerEvent::ToolResult { id, text } => {
                if let Some(Block::Tool { result, .. }) = self.blocks.iter_mut().rev().find(|b| matches!(b, Block::Tool { id: i, .. } if i == id)) {
                    *result = Some(text.clone());
                }
            }
            ServerEvent::Ask { id, tool, what, reason, worker } => self.asks.push(Ask { id: id.clone(), tool: tool.clone(), what: what.clone(), reason: reason.clone(), worker: worker.clone() }),
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
            ServerEvent::Show { title, .. } => self.blocks.push(Block::Shown(title.clone())),
            ServerEvent::Finish(_) | ServerEvent::Other => {}
        }
    }

    /// The stream ended, however it ended: nothing is running and no question can still be answered.
    pub fn end_turn(&mut self) {
        self.busy = false;
        self.asks.clear();
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
        assert_eq!(c.blocks, vec![Block::User("  Hello   there,\nfriend  ".into()), Block::Assistant { text: "Hi there".into(), reasoning: "hmm".into() }]);
        assert!(!c.busy);
        c.push_user("again");
        assert_eq!(c.title, "Hello there, friend", "the title is set once");
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
    fn something_the_assistant_shows_leaves_a_line_in_the_chat() {
        let mut c = Chat::new(1, "m");
        c.push_user("x");
        c.apply(&ServerEvent::Show { title: "Plan".into(), kind: "markdown".into(), content: "# Plan".into(), path: None });
        assert_eq!(c.blocks.last(), Some(&Block::Shown("Plan".into())));
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
        c.apply(&ServerEvent::Ask { id: "a1".into(), tool: "run_command".into(), what: "ls".into(), reason: "reads the folder".into(), worker: String::new() });
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
            c.apply(&ServerEvent::Ask { id: "a".into(), tool: "t".into(), what: "w".into(), reason: String::new(), worker: String::new() });
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
        assert_eq!(cs.list.len(), 1, "an empty list starts a fresh chat");
        assert_eq!(cs.current().unwrap().title, NEW_TITLE);
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
}
