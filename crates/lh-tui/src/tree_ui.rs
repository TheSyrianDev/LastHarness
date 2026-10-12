//! The chat tree in the terminal (`/tree`): a list with one line per message, the branch you are on marked, branches
//! indented, parts foldable, tool results hideable. Keys move around, fold, name a line, make a new chat from it
//! (fork) and go there. This holds the state and turns keys into actions; the event loop talks to the core.

use crossterm::event::{KeyCode, KeyModifiers};
use lh_client::tree::{ChatTree, Row};

/// A question or an input the screen is waiting on.
#[derive(Clone, Debug, PartialEq)]
pub enum Prompt {
    None,
    /// Typing a name for a line.
    Label { node: i64, text: String },
    /// Leaving a branch with `leaving` messages: keep a summary of it?
    Summary { node: i64, leaving: usize },
}

/// What the event loop must do after a key.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Nothing,
    Close,
    Go(i64),
    GoWithSummary(i64, bool),
    Label(i64, String),
    Fork(i64),
}

pub struct TreeScreen {
    pub tree: ChatTree,
    /// The line the cursor is on, by message id (0 is the start of the chat).
    pub at: i64,
    pub prompt: Prompt,
    /// A line of feedback, such as what a fork made.
    pub note: String,
}

impl TreeScreen {
    pub fn new(tree: ChatTree) -> Self {
        let at = tree.leaf;
        Self { tree, at, prompt: Prompt::None, note: String::new() }
    }

    pub fn rows(&self) -> Vec<Row> {
        self.tree.rows()
    }

    /// Show a fresher tree (after naming a line, say) without losing folds, the filter or the cursor.
    pub fn replace_tree(&mut self, mut fresh: ChatTree) {
        fresh.folded = std::mem::take(&mut self.tree.folded);
        fresh.hide_tools = self.tree.hide_tools;
        self.tree = fresh;
        if !self.rows().iter().any(|r| r.id == self.at) {
            self.at = self.tree.leaf;
        }
    }

    fn index(&self) -> usize {
        self.rows().iter().position(|r| r.id == self.at).unwrap_or(0)
    }

    fn step(&mut self, by: isize) {
        let rows = self.rows();
        let i = (self.index() as isize + by).clamp(0, rows.len() as isize - 1) as usize;
        self.at = rows[i].id;
    }

    pub fn key(&mut self, code: KeyCode, mods: KeyModifiers) -> Action {
        match self.prompt.clone() {
            Prompt::Label { node, mut text } => {
                match code {
                    KeyCode::Esc => self.prompt = Prompt::None,
                    KeyCode::Enter => {
                        self.prompt = Prompt::None;
                        return Action::Label(node, text);
                    }
                    KeyCode::Backspace => {
                        text.pop();
                        self.prompt = Prompt::Label { node, text };
                    }
                    KeyCode::Char(c) if !mods.contains(KeyModifiers::CONTROL) => {
                        text.push(c);
                        self.prompt = Prompt::Label { node, text };
                    }
                    _ => {}
                }
                return Action::Nothing;
            }
            Prompt::Summary { node, .. } => {
                return match code {
                    KeyCode::Char('y' | 'Y') => {
                        self.prompt = Prompt::None;
                        Action::GoWithSummary(node, true)
                    }
                    KeyCode::Char('n' | 'N') => {
                        self.prompt = Prompt::None;
                        Action::GoWithSummary(node, false)
                    }
                    KeyCode::Esc => {
                        self.prompt = Prompt::None;
                        Action::Nothing
                    }
                    _ => Action::Nothing,
                };
            }
            Prompt::None => {}
        }
        self.note.clear();
        match code {
            KeyCode::Esc | KeyCode::Char('q') => return Action::Close,
            KeyCode::Up | KeyCode::Char('k') => self.step(-1),
            KeyCode::Down | KeyCode::Char('j') => self.step(1),
            KeyCode::PageUp => self.step(-10),
            KeyCode::PageDown => self.step(10),
            KeyCode::Home => self.at = 0,
            KeyCode::End => self.at = self.tree.leaf,
            KeyCode::Left | KeyCode::Right | KeyCode::Char(' ') => {
                // Left folds an open line, Right opens a folded one, Space flips it
                let folded = self.tree.folded.contains(&self.at);
                match code {
                    KeyCode::Left if !folded => self.tree.toggle_fold(self.at),
                    KeyCode::Right if folded => self.tree.toggle_fold(self.at),
                    KeyCode::Char(' ') => self.tree.toggle_fold(self.at),
                    _ => {}
                }
            }
            KeyCode::Char('t') => {
                self.tree.hide_tools = !self.tree.hide_tools;
                if !self.rows().iter().any(|r| r.id == self.at) {
                    self.at = self.tree.leaf;
                }
            }
            KeyCode::Char('l') if self.at != 0 => {
                let text = self.tree.node(self.at).and_then(|n| n.label.clone()).unwrap_or_default();
                self.prompt = Prompt::Label { node: self.at, text };
            }
            KeyCode::Char('f') => return Action::Fork(self.at),
            KeyCode::Enter => return Action::Go(self.at),
            _ => {}
        }
        Action::Nothing
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn n(id: i64, parent: Option<i64>, role: &str, preview: &str) -> serde_json::Value {
        json!({"id": id, "parent": parent, "role": role, "preview": preview, "chars": 1, "label": null})
    }

    fn screen() -> TreeScreen {
        let v = json!({"leaf": 4, "active": [1, 2, 4], "summarize": "ask", "nodes": [
            n(1, None, "user", "q1"), n(2, Some(1), "assistant", "a1"), n(3, Some(2), "user", "q2"), n(4, Some(2), "user", "q2b")]});
        TreeScreen::new(ChatTree::from_json(&v).unwrap())
    }

    fn press(s: &mut TreeScreen, c: KeyCode) -> Action {
        s.key(c, KeyModifiers::NONE)
    }

    #[test]
    fn it_opens_on_the_line_the_chat_is_at_and_the_cursor_moves_within_the_list() {
        let mut s = screen();
        assert_eq!(s.at, 4);
        assert_eq!(press(&mut s, KeyCode::Up), Action::Nothing);
        // the list reads: start, q1, a1, q2, q2b; the line above q2b is q2, which sits beside it
        assert_eq!(s.at, 3);
        press(&mut s, KeyCode::Down);
        press(&mut s, KeyCode::Down);
        assert_eq!(s.at, 4, "the cursor stops at the last line");
    }

    #[test]
    fn enter_goes_to_the_line_and_the_start_is_a_place() {
        let mut s = screen();
        press(&mut s, KeyCode::Home);
        assert_eq!((s.at, press(&mut s, KeyCode::Enter)), (0, Action::Go(0)));
        press(&mut s, KeyCode::End);
        assert_eq!(s.at, 4);
        assert_eq!(press(&mut s, KeyCode::Esc), Action::Close);
        assert_eq!(press(&mut s, KeyCode::Char('q')), Action::Close);
    }

    #[test]
    fn folding_with_the_arrow_keys_and_space() {
        let mut s = screen();
        s.at = 2;
        press(&mut s, KeyCode::Left);
        assert!(s.tree.folded.contains(&2), "left folds an open line");
        press(&mut s, KeyCode::Left);
        assert!(s.tree.folded.contains(&2), "left on a folded line does nothing more");
        press(&mut s, KeyCode::Right);
        assert!(s.tree.folded.is_empty(), "right opens it");
        press(&mut s, KeyCode::Char(' '));
        assert!(s.tree.folded.contains(&2), "space flips");
        s.at = 4;
        press(&mut s, KeyCode::Char(' '));
        assert!(!s.tree.folded.contains(&4), "a line without replies has nothing to fold");
    }

    #[test]
    fn the_tool_filter_and_naming_a_line_with_the_keyboard() {
        let mut s = screen();
        press(&mut s, KeyCode::Char('t'));
        assert!(s.tree.hide_tools);
        s.at = 3;
        press(&mut s, KeyCode::Char('l'));
        assert_eq!(s.prompt, Prompt::Label { node: 3, text: String::new() });
        for c in "good".chars() {
            press(&mut s, KeyCode::Char(c));
        }
        press(&mut s, KeyCode::Backspace);
        press(&mut s, KeyCode::Char('d'));
        assert_eq!(press(&mut s, KeyCode::Enter), Action::Label(3, "good".into()));
        assert_eq!(s.prompt, Prompt::None);
        // Esc gives up the name without closing the screen
        press(&mut s, KeyCode::Char('l'));
        assert_eq!(press(&mut s, KeyCode::Esc), Action::Nothing);
        assert_eq!(s.prompt, Prompt::None);
        s.at = 0;
        press(&mut s, KeyCode::Char('l'));
        assert_eq!(s.prompt, Prompt::None, "the start of the chat cannot be named");
    }

    #[test]
    fn leaving_a_branch_asks_and_each_answer_is_its_own_action() {
        let mut s = screen();
        s.prompt = Prompt::Summary { node: 3, leaving: 2 };
        assert_eq!(press(&mut s, KeyCode::Char('x')), Action::Nothing);
        assert!(matches!(s.prompt, Prompt::Summary { .. }), "other keys do not decide it");
        assert_eq!(press(&mut s, KeyCode::Char('y')), Action::GoWithSummary(3, true));
        s.prompt = Prompt::Summary { node: 3, leaving: 2 };
        assert_eq!(press(&mut s, KeyCode::Char('n')), Action::GoWithSummary(3, false));
        s.prompt = Prompt::Summary { node: 3, leaving: 2 };
        assert_eq!(press(&mut s, KeyCode::Esc), Action::Nothing);
        assert_eq!(s.prompt, Prompt::None, "Esc stays where you were");
    }

    #[test]
    fn a_fresh_tree_keeps_the_folds_the_filter_and_the_cursor() {
        let mut s = screen();
        s.at = 2;
        press(&mut s, KeyCode::Char(' '));
        press(&mut s, KeyCode::Char('t'));
        let mut fresh = s.tree.clone();
        fresh.folded.clear();
        fresh.hide_tools = false;
        s.replace_tree(fresh);
        assert!(s.tree.folded.contains(&2) && s.tree.hide_tools);
    }
}
