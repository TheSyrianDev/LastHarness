//! The chat tree as a front end shows it: one line per message, the active branch marked, branches indented, parts
//! folded away, tool results hidden on request. The terminal and the desktop app share this, so they list the same
//! lines in the same order. Nothing here talks to the core except the typed calls at the bottom.

use std::collections::{HashMap, HashSet};

use serde_json::Value;

use crate::{Client, Result};

#[derive(Clone, Debug, PartialEq)]
pub struct TreeNode {
    pub id: i64,
    pub parent: Option<i64>,
    /// `user`, `assistant`, `tool` or `summary`.
    pub role: String,
    pub preview: String,
    pub chars: usize,
    pub label: Option<String>,
}

/// One line of the tree view.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// 0 is the start of the chat, a place a person can go back to.
    pub id: i64,
    /// How far the line is indented: only where a message has more than one reply does the next level start.
    pub depth: usize,
    /// On the branch the chat is on now.
    pub active: bool,
    /// The message the chat is at (the next one will answer it).
    pub current: bool,
    pub role: String,
    pub text: String,
    pub label: Option<String>,
    /// Has replies that can be folded away.
    pub foldable: bool,
    pub folded: bool,
    /// How many messages are hidden under a folded line.
    pub hidden_below: usize,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ChatTree {
    pub nodes: Vec<TreeNode>,
    pub leaf: i64,
    pub active: Vec<i64>,
    /// What the core does with the branch you leave: `ask`, `always` or `never`.
    pub summarize: String,
    pub folded: HashSet<i64>,
    pub hide_tools: bool,
}

impl ChatTree {
    pub fn from_json(v: &Value) -> Result<ChatTree> {
        let nodes = v["nodes"]
            .as_array()
            .ok_or("the core sent no tree")?
            .iter()
            .map(|n| TreeNode {
                id: n["id"].as_i64().unwrap_or(0),
                parent: n["parent"].as_i64(),
                role: n["role"].as_str().unwrap_or("").to_string(),
                preview: n["preview"].as_str().unwrap_or("").to_string(),
                chars: n["chars"].as_u64().unwrap_or(0) as usize,
                label: n["label"].as_str().map(str::to_string),
            })
            .collect();
        Ok(ChatTree { nodes, leaf: v["leaf"].as_i64().unwrap_or(0), active: v["active"].as_array().map(|a| a.iter().filter_map(Value::as_i64).collect()).unwrap_or_default(), summarize: v["summarize"].as_str().unwrap_or("ask").to_string(), ..Default::default() })
    }

    pub fn node(&self, id: i64) -> Option<&TreeNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// How many branches the chat has (the ends of the tree) and which one the chat is on, counting from 1.
    /// `None` when there is only one line of talk. The position is `None` when the chat is parked part way
    /// along a branch rather than at its end.
    pub fn branches(&self) -> Option<(usize, Option<usize>)> {
        let has_reply: HashSet<i64> = self.nodes.iter().filter_map(|n| n.parent).collect();
        let mut ends: Vec<i64> = self.nodes.iter().filter(|n| !has_reply.contains(&n.id)).map(|n| n.id).collect();
        ends.sort_unstable();
        (ends.len() > 1).then(|| (ends.len(), ends.iter().position(|&e| e == self.leaf).map(|i| i + 1)))
    }

    /// Fold a line's replies away, or bring them back. Lines without replies cannot be folded.
    pub fn toggle_fold(&mut self, id: i64) {
        if id != 0 && !self.nodes.iter().any(|n| n.parent == Some(id)) {
            return;
        }
        if !self.folded.remove(&id) {
            self.folded.insert(id);
        }
    }

    fn hidden_role(&self, n: &TreeNode) -> bool {
        self.hide_tools && n.role == "tool"
    }

    /// The nearest ancestor that is shown, so hiding tool results keeps the tree connected.
    fn shown_parent(&self, n: &TreeNode, by_id: &HashMap<i64, &TreeNode>) -> i64 {
        let mut p = n.parent;
        while let Some(id) = p {
            match by_id.get(&id) {
                Some(a) if self.hidden_role(a) => p = a.parent,
                Some(_) => return id,
                None => return 0,
            }
        }
        0
    }

    /// The lines to draw, in order: the start of the chat, then every message depth first.
    pub fn rows(&self) -> Vec<Row> {
        let by_id: HashMap<i64, &TreeNode> = self.nodes.iter().map(|n| (n.id, n)).collect();
        let mut kids: HashMap<i64, Vec<&TreeNode>> = HashMap::new();
        for n in self.nodes.iter().filter(|n| !self.hidden_role(n)) {
            kids.entry(self.shown_parent(n, &by_id)).or_default().push(n);
        }
        let active: HashSet<i64> = self.active.iter().copied().collect();
        let mut rows = vec![Row { id: 0, depth: 0, active: true, current: self.leaf == 0, role: "start".into(), text: "start of the chat".into(), label: None, foldable: false, folded: false, hidden_below: 0 }];
        let count = |from: i64, kids: &HashMap<i64, Vec<&TreeNode>>| -> usize {
            let (mut n, mut stack) = (0, vec![from]);
            while let Some(i) = stack.pop() {
                for k in kids.get(&i).into_iter().flatten() {
                    n += 1;
                    stack.push(k.id);
                }
            }
            n
        };
        // depth first; the active branch is listed last at each fork, so the line you are on stays at the bottom
        let mut order: Vec<(&TreeNode, usize)> = vec![];
        fn walk<'a>(at: i64, depth: usize, kids: &HashMap<i64, Vec<&'a TreeNode>>, active: &HashSet<i64>, folded: &HashSet<i64>, out: &mut Vec<(&'a TreeNode, usize)>) {
            let Some(list) = kids.get(&at) else { return };
            let mut list: Vec<&TreeNode> = list.clone();
            list.sort_by_key(|n| (active.contains(&n.id), n.id));
            let branching = list.len() > 1;
            for n in list {
                let d = if branching { depth + 1 } else { depth };
                out.push((n, d));
                if !folded.contains(&n.id) {
                    walk(n.id, d, kids, active, folded, out);
                }
            }
        }
        walk(0, 0, &kids, &active, &self.folded, &mut order);
        for (n, depth) in order {
            let foldable = kids.contains_key(&n.id);
            let folded = self.folded.contains(&n.id) && foldable;
            rows.push(Row {
                id: n.id,
                depth,
                active: active.contains(&n.id),
                current: n.id == self.leaf,
                role: n.role.clone(),
                text: n.preview.clone(),
                label: n.label.clone(),
                foldable,
                folded,
                hidden_below: if folded { count(n.id, &kids) } else { 0 },
            });
        }
        rows
    }
}

/// What the core answered to a move.
#[derive(Clone, Debug, PartialEq)]
pub enum Moved {
    /// The branch being left has messages and the person decides whether to keep a summary of it.
    NeedsChoice { leaving_messages: usize },
    Done { input: Option<String>, summary: Option<String>, left_messages: usize },
}

impl Moved {
    pub fn from_json(v: &Value) -> Moved {
        if v["needs_choice"].as_bool() == Some(true) {
            return Moved::NeedsChoice { leaving_messages: v["leaving_messages"].as_u64().unwrap_or(0) as usize };
        }
        Moved::Done { input: v["input"].as_str().map(str::to_string), summary: v["summary"].as_str().map(str::to_string), left_messages: v["left_messages"].as_u64().unwrap_or(0) as usize }
    }
}

impl Client {
    pub async fn chat_tree(&self, chat: &str) -> Result<ChatTree> {
        ChatTree::from_json(&self.call("GET", &format!("/v1/chats/{chat}/tree"), None).await?)
    }

    /// Go to a message (0 is the start). `summarize` answers the question of whether to keep a summary of the branch left;
    /// leave it out to let the core ask.
    pub async fn move_in_tree(&self, chat: &str, node: i64, summarize: Option<bool>) -> Result<Moved> {
        let mut body = serde_json::json!({"node": node});
        if let Some(s) = summarize {
            body["summarize"] = s.into();
        }
        Ok(Moved::from_json(&self.call("POST", &format!("/v1/chats/{chat}/tree/move"), Some(body)).await?))
    }

    pub async fn label_node(&self, chat: &str, node: i64, label: &str) -> Result<()> {
        self.call("POST", &format!("/v1/chats/{chat}/tree/label"), Some(serde_json::json!({"node": node, "label": label}))).await.map(|_| ())
    }

    /// A new chat from an earlier point. Returns its id and the words to put back in the input box, if any.
    pub async fn fork_chat(&self, chat: &str, node: i64) -> Result<(String, Option<String>)> {
        let v = self.call("POST", &format!("/v1/chats/{chat}/fork"), Some(serde_json::json!({"node": node}))).await?;
        Ok((v["id"].as_str().ok_or("the core sent no chat id")?.to_string(), v["input"].as_str().map(str::to_string)))
    }

    /// A new chat with a copy of the active branch. Returns its id.
    pub async fn clone_chat(&self, chat: &str) -> Result<String> {
        let v = self.call("POST", &format!("/v1/chats/{chat}/clone"), Some(serde_json::json!({}))).await?;
        Ok(v["id"].as_str().ok_or("the core sent no chat id")?.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn n(id: i64, parent: Option<i64>, role: &str, preview: &str) -> Value {
        json!({"id": id, "parent": parent, "role": role, "preview": preview, "chars": preview.len(), "label": null})
    }

    /// q1 - a1 - q2 - a2          (the first path)
    ///          \\- q2b - t - a2b   (the second path, the one the chat is on)
    fn sample() -> ChatTree {
        let v = json!({"leaf": 7, "active": [1, 2, 5, 6, 7], "summarize": "ask", "nodes": [
            n(1, None, "user", "q1"), n(2, Some(1), "assistant", "a1"), n(3, Some(2), "user", "q2"), n(4, Some(3), "assistant", "a2"),
            n(5, Some(2), "user", "q2b"), n(6, Some(5), "tool", "show x"), n(7, Some(6), "assistant", "a2b")]});
        ChatTree::from_json(&v).unwrap()
    }

    fn texts(t: &ChatTree) -> Vec<String> {
        t.rows().iter().map(|r| format!("{}{}", " ".repeat(r.depth), r.text)).collect()
    }

    #[test]
    fn the_tree_lists_every_message_and_only_indents_where_it_forks() {
        let t = sample();
        assert_eq!(texts(&t), vec!["start of the chat", "q1", "a1", " q2", " a2", " q2b", " show x", " a2b"], "the active path is listed last at the fork");
        let rows = t.rows();
        assert!(rows.iter().find(|r| r.text == "a2b").unwrap().current, "the chat is at a2b");
        assert!(rows.iter().find(|r| r.text == "q2b").unwrap().active && !rows.iter().find(|r| r.text == "q2").unwrap().active, "the active branch is marked");
        assert!(rows[0].active && !rows[0].current, "the start is always on the way");
    }

    #[test]
    fn branches_count_the_ends_and_say_which_one_the_chat_is_on() {
        let mut t = sample();
        assert_eq!(t.branches(), Some((2, Some(2))), "a2 (id 4) and a2b (id 7); the chat is at the second");
        t.leaf = 4;
        assert_eq!(t.branches(), Some((2, Some(1))));
        t.leaf = 5;
        assert_eq!(t.branches(), Some((2, None)), "part way along a branch");
        let straight = ChatTree::from_json(&json!({"leaf": 2, "active": [1, 2], "nodes": [n(1, None, "user", "q"), n(2, Some(1), "assistant", "a")]})).unwrap();
        assert_eq!(straight.branches(), None, "one line of talk has no branches to show");
        assert_eq!(ChatTree::default().branches(), None);
    }

    #[test]
    fn a_fork_with_nothing_chosen_yet_is_just_a_line() {
        let t = ChatTree::from_json(&json!({"leaf": 0, "active": [], "nodes": []})).unwrap();
        assert_eq!(t.rows().len(), 1);
        assert!(t.rows()[0].current, "an empty chat is at its start");
    }

    #[test]
    fn folding_hides_everything_below_and_says_how_much() {
        let mut t = sample();
        t.toggle_fold(3);
        assert_eq!(texts(&t), vec!["start of the chat", "q1", "a1", " q2", " q2b", " show x", " a2b"]);
        let r = t.rows().into_iter().find(|r| r.id == 3).unwrap();
        assert!(r.foldable && r.folded && r.hidden_below == 1);
        t.toggle_fold(3);
        assert!(t.rows().iter().any(|r| r.text == "a2"));
        t.toggle_fold(4);
        assert!(t.folded.is_empty(), "a line without replies cannot be folded");
    }

    #[test]
    fn hiding_tool_results_keeps_the_rest_connected() {
        let mut t = sample();
        t.hide_tools = true;
        assert_eq!(texts(&t), vec!["start of the chat", "q1", "a1", " q2", " a2", " q2b", " a2b"], "a2b still hangs under q2b");
        t.hide_tools = false;
        assert!(texts(&t).contains(&" show x".to_string()));
    }

    #[test]
    fn labels_and_the_move_answers_are_read() {
        let mut v = sample();
        v.nodes[1].label = Some("good start".into());
        assert_eq!(v.rows().iter().find(|r| r.id == 2).unwrap().label.as_deref(), Some("good start"));
        assert_eq!(Moved::from_json(&json!({"needs_choice": true, "leaving_messages": 3})), Moved::NeedsChoice { leaving_messages: 3 });
        assert_eq!(Moved::from_json(&json!({"leaf": 2, "input": "q2", "summary": null, "left_messages": 2})), Moved::Done { input: Some("q2".into()), summary: None, left_messages: 2 });
        assert!(ChatTree::from_json(&json!({})).is_err());
    }
}
