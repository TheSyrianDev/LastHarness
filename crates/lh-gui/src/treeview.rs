//! What the chat tree panel remembers: which chat's tree it shows, the line chosen, a question waiting for an
//! answer. The lines themselves come from `lh_client::tree::ChatTree`, shared with the terminal. No window code here.

use lh_client::tree::{ChatTree, Moved};

#[derive(Clone, Debug, PartialEq, Default)]
pub enum Prompt {
    #[default]
    None,
    /// Typing a name for the chosen line.
    Label,
    /// Leaving a branch with `leaving` messages: keep a summary of it where the person lands?
    Summary { node: i64, leaving: usize },
}

/// What a move led to, for the window to act on.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// Waiting for the person's answer about a summary.
    Asked,
    /// The chat is now somewhere else: show its branch again, and put these words in the input box.
    Moved { input: Option<String>, summary: bool },
}

#[derive(Clone, Debug, Default)]
pub struct TreeState {
    /// The window's key of the chat the tree belongs to.
    pub chat: Option<u64>,
    pub tree: Option<ChatTree>,
    /// The chosen line (0 is the start of the chat).
    pub at: i64,
    pub prompt: Prompt,
    /// Something to tell the person: a problem, or what a fork made.
    pub note: String,
}

/// The nearest line at or above `from` that is on screen (folds and the filter hide lines), else the start.
fn nearest_shown(t: &ChatTree, from: i64) -> i64 {
    let rows = t.rows();
    let mut at = Some(from);
    while let Some(id) = at {
        if id == 0 || rows.iter().any(|r| r.id == id) {
            return id;
        }
        at = t.node(id).and_then(|n| n.parent);
    }
    0
}

impl TreeState {
    /// A tree arrived. For the same chat the folds, the filter and the chosen line stay; for another chat they start fresh.
    pub fn load(&mut self, chat: u64, mut fresh: ChatTree) {
        if self.chat == Some(chat) {
            if let Some(old) = &self.tree {
                fresh.folded = old.folded.clone();
                fresh.hide_tools = old.hide_tools;
            }
            self.at = nearest_shown(&fresh, self.at);
        } else {
            self.at = nearest_shown(&fresh, fresh.leaf);
            self.prompt = Prompt::None;
            self.note.clear();
        }
        self.chat = Some(chat);
        self.tree = Some(fresh);
    }

    /// The tree could not be had (a private chat, a core that is away). Say so and show no stale tree.
    pub fn failed(&mut self, chat: u64, why: String) {
        self.chat = Some(chat);
        self.tree = None;
        self.prompt = Prompt::None;
        self.note = why;
    }

    pub fn select(&mut self, id: i64) {
        self.at = id;
        self.note.clear();
    }

    pub fn toggle_fold(&mut self, id: i64) {
        if let Some(t) = self.tree.as_mut() {
            t.toggle_fold(id);
        }
    }

    pub fn toggle_tools(&mut self) {
        if let Some(t) = self.tree.as_mut() {
            t.hide_tools = !t.hide_tools;
            self.at = nearest_shown(t, self.at);
        }
    }

    /// The core answered a move.
    pub fn moved(&mut self, node: i64, m: Moved) -> Outcome {
        match m {
            Moved::NeedsChoice { leaving_messages } => {
                self.prompt = Prompt::Summary { node, leaving: leaving_messages };
                Outcome::Asked
            }
            Moved::Done { input, summary, .. } => {
                self.prompt = Prompt::None;
                self.note = match &summary {
                    Some(_) => "A short summary of the other path was kept where you landed.".into(),
                    None => String::new(),
                };
                Outcome::Moved { input, summary: summary.is_some() }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tree(leaf: i64) -> ChatTree {
        let n = |id: i64, parent: Option<i64>, role: &str, p: &str| json!({"id": id, "parent": parent, "role": role, "preview": p, "chars": 1, "label": null});
        ChatTree::from_json(&json!({"leaf": leaf, "active": [1, 2], "summarize": "ask", "nodes": [n(1, None, "user", "q"), n(2, Some(1), "assistant", "a"), n(3, Some(2), "user", "q2")]})).unwrap()
    }

    #[test]
    fn a_tree_opens_on_the_line_the_chat_is_at_and_a_refresh_keeps_what_the_person_set() {
        let mut s = TreeState::default();
        s.load(1, tree(2));
        assert_eq!(s.at, 2);
        s.select(1);
        s.toggle_fold(2);
        s.toggle_tools();
        s.load(1, tree(3));
        assert_eq!(s.at, 1, "the chosen line stays");
        let t = s.tree.as_ref().unwrap();
        assert!(t.folded.contains(&2) && t.hide_tools, "folds and the filter stay");
        // another chat starts fresh
        s.note = "x".into();
        s.load(2, tree(3));
        assert_eq!((s.at, s.note.as_str()), (3, ""));
        assert!(s.tree.as_ref().unwrap().folded.is_empty());
    }

    #[test]
    fn a_chosen_line_that_disappears_falls_back_to_where_the_chat_is() {
        let mut s = TreeState::default();
        s.load(1, tree(2));
        s.select(3);
        s.toggle_fold(2);
        s.load(1, tree(2));
        assert_eq!(s.at, 2, "line 3 is folded away, so the cursor goes up to the line that holds it");
    }

    #[test]
    fn a_move_either_asks_about_a_summary_or_finishes() {
        let mut s = TreeState::default();
        s.load(1, tree(3));
        assert_eq!(s.moved(2, Moved::NeedsChoice { leaving_messages: 2 }), Outcome::Asked);
        assert_eq!(s.prompt, Prompt::Summary { node: 2, leaving: 2 });
        let o = s.moved(2, Moved::Done { input: Some("q2".into()), summary: Some("found it".into()), left_messages: 2 });
        assert_eq!(o, Outcome::Moved { input: Some("q2".into()), summary: true });
        assert_eq!(s.prompt, Prompt::None);
        assert!(s.note.contains("summary"));
        let o = s.moved(1, Moved::Done { input: None, summary: None, left_messages: 0 });
        assert_eq!(o, Outcome::Moved { input: None, summary: false });
        assert!(s.note.is_empty());
    }

    #[test]
    fn a_failure_shows_a_note_and_no_old_tree() {
        let mut s = TreeState::default();
        s.load(1, tree(2));
        s.failed(1, "this chat is not stored (a private chat has no tree)".into());
        assert!(s.tree.is_none() && s.note.contains("private"));
    }

    #[test]
    fn the_filter_moves_the_cursor_off_a_line_that_is_hidden() {
        let n = |id: i64, parent: Option<i64>, role: &str, p: &str| json!({"id": id, "parent": parent, "role": role, "preview": p, "chars": 1, "label": null});
        let t = ChatTree::from_json(&json!({"leaf": 2, "active": [1, 2], "nodes": [n(1, None, "user", "q"), n(2, Some(1), "tool", "t")]})).unwrap();
        let mut s = TreeState::default();
        s.load(1, t);
        assert_eq!(s.at, 2);
        s.toggle_tools();
        assert_eq!(s.at, 1, "the tool line is hidden, so the cursor goes to the message above it");
    }
}
