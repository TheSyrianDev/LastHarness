//! A chat is a tree. Each stored message says which message it answers, and the chat is "at" one message (its
//! leaf): the next message answers that one. Going back to an earlier message and writing again starts a new
//! branch; nothing is lost. The model reads only the active branch, the path from the first message to the leaf.
//! Nothing here runs for incognito chats, because they store nothing.

use lh_store::rusqlite::{self, params, Connection, OptionalExtension};

use crate::{Error, Memory, Result};

/// One message in the tree. `role` is `user`, `assistant`, `tool` (what a tool did, for the tree view only; it is
/// never sent to a model again) or `summary` (a branch the person left, written down where they came back).
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub id: i64,
    pub parent: Option<i64>,
    pub role: String,
    pub content: String,
    pub at: i64,
    pub label: Option<String>,
}

/// The whole tree of a chat and where it is now.
#[derive(Clone, Debug, PartialEq)]
pub struct Tree {
    pub nodes: Vec<Node>,
    /// The message the next one will answer; 0 means before the first message.
    pub leaf: i64,
}

impl Tree {
    /// The ids on the active branch, first message first.
    pub fn active_path(&self) -> Vec<i64> {
        path_in(&self.nodes, self.leaf).iter().map(|n| n.id).collect()
    }
}

fn path_in(nodes: &[Node], leaf: i64) -> Vec<&Node> {
    let mut out = vec![];
    let mut at = Some(leaf).filter(|l| *l != 0);
    while let Some(id) = at {
        let Some(n) = nodes.iter().find(|n| n.id == id) else { break };
        out.push(n);
        at = n.parent;
    }
    out.reverse();
    out
}

/// What moving to a node would do. Nothing has changed yet.
#[derive(Clone, Debug, PartialEq)]
pub struct MovePlan {
    pub new_leaf: i64,
    /// The words to put back in the input box when the node is something the person said.
    pub input: Option<String>,
    /// Messages on the branch being left that are not on the branch being entered (tool results left out).
    pub leaving: Vec<Node>,
}

fn leaf_of(c: &Connection, conv: &str) -> rusqlite::Result<i64> {
    if let Some(l) = c.query_row("select leaf from chat_branch where conv = ?1", [conv], |r| r.get::<_, i64>(0)).optional()? {
        return Ok(l);
    }
    c.query_row("select coalesce(max(id), 0) from messages where conv = ?1", [conv], |r| r.get(0))
}

fn nodes_of(c: &Connection, conv: &str) -> rusqlite::Result<Vec<Node>> {
    let mut q = c.prepare("select m.id, m.parent, m.role, m.content, m.at, l.label from messages m left join chat_labels l on l.msg = m.id where m.conv = ?1 order by m.seq")?;
    let rows = q.query_map([conv], |r| Ok(Node { id: r.get(0)?, parent: r.get(1)?, role: r.get(2)?, content: r.get(3)?, at: r.get(4)?, label: r.get(5)? }))?;
    rows.collect()
}

impl Memory {
    /// Store a message under the node the chat is at, and move the chat to it. Returns the new node's id.
    pub fn append_message(&self, conv: &str, role: &str, content: &str) -> Result<i64> {
        let st = crate::text::tokens(content).join(" ");
        Ok(self.store.with(|c| {
            let leaf = leaf_of(c, conv)?;
            let seq: i64 = c.query_row("select coalesce(max(seq), 0) + 1 from messages where conv = ?1", [conv], |r| r.get(0))?;
            let parent = Some(leaf).filter(|l| *l != 0);
            c.execute("insert into messages(conv, seq, role, content, search_text, at, parent) values (?1,?2,?3,?4,?5,?6,?7)", params![conv, seq, role, content, st, lh_store::now(), parent])?;
            let id = c.last_insert_rowid();
            c.execute("insert into chat_branch(conv, leaf) values (?1, ?2) on conflict(conv) do update set leaf = excluded.leaf", params![conv, id])?;
            Ok(id)
        })?)
    }

    /// The whole tree of a chat and where it is now.
    pub fn tree(&self, conv: &str) -> Result<Tree> {
        Ok(self.store.with(|c| Ok(Tree { nodes: nodes_of(c, conv)?, leaf: leaf_of(c, conv)? }))?)
    }

    /// The active branch, first message first, every kind of node.
    pub fn branch(&self, conv: &str) -> Result<Vec<Node>> {
        let t = self.tree(conv)?;
        Ok(path_in(&t.nodes, t.leaf).into_iter().cloned().collect())
    }

    /// What moving to `node` would do: where the chat would be, what goes in the input box, and which messages
    /// of the current branch would be left behind. `node` 0 is the very start.
    pub fn plan_move(&self, conv: &str, node: i64) -> Result<MovePlan> {
        let t = self.tree(conv)?;
        let (new_leaf, input) = if node == 0 {
            (0, None)
        } else {
            let n = t.nodes.iter().find(|n| n.id == node).ok_or_else(|| Error::NotFound(format!("message {node} is not in this chat")))?;
            // choosing something the person said puts it back in the input box and goes to what came before it
            if n.role == "user" { (n.parent.unwrap_or(0), Some(n.content.clone())) } else { (n.id, None) }
        };
        let keep: Vec<i64> = path_in(&t.nodes, new_leaf).iter().map(|n| n.id).collect();
        let leaving = path_in(&t.nodes, t.leaf).into_iter().filter(|n| !keep.contains(&n.id) && n.role != "tool").cloned().collect();
        Ok(MovePlan { new_leaf, input, leaving })
    }

    /// Put the chat at `leaf` (0 for the start). The node must belong to this chat.
    pub fn set_leaf(&self, conv: &str, leaf: i64) -> Result<()> {
        if leaf != 0 && !self.tree(conv)?.nodes.iter().any(|n| n.id == leaf) {
            return Err(Error::NotFound(format!("message {leaf} is not in this chat")));
        }
        self.store.with(|c| c.execute("insert into chat_branch(conv, leaf) values (?1, ?2) on conflict(conv) do update set leaf = excluded.leaf", params![conv, leaf]).map(|_| ()))?;
        Ok(())
    }

    /// Name a message, or take its name away with an empty label.
    pub fn set_label(&self, conv: &str, node: i64, label: &str) -> Result<()> {
        if !self.tree(conv)?.nodes.iter().any(|n| n.id == node) {
            return Err(Error::NotFound(format!("message {node} is not in this chat")));
        }
        let label: String = label.trim().chars().take(80).collect();
        self.store.with(|c| {
            if label.is_empty() {
                c.execute("delete from chat_labels where msg = ?1", [node]).map(|_| ())
            } else {
                c.execute("insert into chat_labels(msg, label) values (?1, ?2) on conflict(msg) do update set label = excluded.label", params![node, label]).map(|_| ())
            }
        })?;
        Ok(())
    }

    /// Copy a path of messages into a new chat `new_id` (same project, mode and model; a title of its own). The copy
    /// is a straight branch and sits at its last message. Returns how many messages were copied.
    fn copy_path(&self, from: &str, path: &[Node], new_id: &str, title_suffix: &str) -> Result<usize> {
        let info = self.conversation(from)?.ok_or_else(|| Error::NotFound(format!("no chat {from}")))?;
        self.start_conversation(new_id, info.project.as_deref())?;
        self.set_conversation_meta(new_id, info.model.as_deref().unwrap_or(""), &info.mode)?;
        let title = format!("{}{}", info.title, title_suffix);
        self.store.with(|c| c.execute("update conversations set title = ?2 where id = ?1", params![new_id, title]).map(|_| ()))?;
        for n in path {
            let id = self.append_message(new_id, &n.role, &n.content)?;
            if let Some(l) = &n.label {
                self.store.with(|c| c.execute("insert into chat_labels(msg, label) values (?1, ?2)", params![id, l]).map(|_| ()))?;
            }
        }
        Ok(path.len())
    }

    /// A new chat from an earlier point: everything up to `node`, and when `node` is something the person said, up to
    /// just before it, with that text returned for the input box. Needs at least one message to copy or text to return.
    pub fn fork(&self, conv: &str, node: i64, new_id: &str) -> Result<Option<String>> {
        let plan = self.plan_move(conv, node)?;
        let t = self.tree(conv)?;
        let path: Vec<Node> = path_in(&t.nodes, plan.new_leaf).into_iter().cloned().collect();
        self.copy_path(conv, &path, new_id, " (fork)")?;
        Ok(plan.input)
    }

    /// A new chat with a copy of the active branch.
    pub fn clone_branch(&self, conv: &str, new_id: &str) -> Result<usize> {
        let path = self.branch(conv)?;
        self.copy_path(conv, &path, new_id, " (copy)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn mem() -> Memory {
        let d = tempfile::tempdir().unwrap();
        let store = Arc::new(lh_store::Store::open_memory().unwrap());
        let m = Memory::open(store, d.path()).unwrap();
        std::mem::forget(d);
        m
    }

    fn say(m: &Memory, conv: &str, role: &str, text: &str) -> i64 {
        m.append_message(conv, role, text).unwrap()
    }

    #[test]
    fn a_straight_chat_is_one_branch_and_new_messages_follow_the_last() {
        let m = mem();
        m.start_conversation("s1", None).unwrap();
        let (a, b, c) = (say(&m, "s1", "user", "hi"), say(&m, "s1", "assistant", "hello"), say(&m, "s1", "user", "more"));
        let t = m.tree("s1").unwrap();
        assert_eq!(t.leaf, c);
        assert_eq!(t.active_path(), vec![a, b, c]);
        assert_eq!(t.nodes[0].parent, None);
        assert_eq!(t.nodes[1].parent, Some(a));
        assert_eq!(m.messages("s1").unwrap().len(), 3);
    }

    #[test]
    fn choosing_something_said_puts_it_back_and_writing_again_starts_a_branch() {
        let m = mem();
        m.start_conversation("s1", None).unwrap();
        let q1 = say(&m, "s1", "user", "first question");
        let a1 = say(&m, "s1", "assistant", "first answer");
        let q2 = say(&m, "s1", "user", "second question");
        let a2 = say(&m, "s1", "assistant", "second answer");
        let plan = m.plan_move("s1", q2).unwrap();
        assert_eq!((plan.new_leaf, plan.input.as_deref()), (a1, Some("second question")), "the words come back; the chat is at the answer before them");
        assert_eq!(plan.leaving.iter().map(|n| n.id).collect::<Vec<_>>(), vec![q2, a2]);
        m.set_leaf("s1", plan.new_leaf).unwrap();
        let q2b = say(&m, "s1", "user", "second question, reworded");
        let t = m.tree("s1").unwrap();
        assert_eq!(t.active_path(), vec![q1, a1, q2b], "the new branch");
        assert_eq!(t.nodes.len(), 5, "nothing was lost: the old branch is still in the tree");
        assert_eq!(t.nodes.iter().find(|n| n.id == q2b).unwrap().parent, Some(a1));
        assert_eq!(t.nodes.iter().find(|n| n.id == q2).unwrap().parent, Some(a1), "both questions answer the same message");
        // the model's view of the chat: only the active branch
        let said: Vec<String> = m.messages("s1").unwrap().into_iter().map(|(_, t)| t).collect();
        assert_eq!(said, vec!["first question", "first answer", "second question, reworded"]);
    }

    #[test]
    fn choosing_an_answer_continues_after_it_with_an_empty_input_and_the_start_is_a_place() {
        let m = mem();
        m.start_conversation("s1", None).unwrap();
        let q1 = say(&m, "s1", "user", "q");
        let a1 = say(&m, "s1", "assistant", "a");
        say(&m, "s1", "user", "q2");
        let plan = m.plan_move("s1", a1).unwrap();
        assert_eq!((plan.new_leaf, plan.input), (a1, None));
        let start = m.plan_move("s1", q1).unwrap();
        assert_eq!((start.new_leaf, start.input.as_deref()), (0, Some("q")), "the first message goes back to the very start");
        m.set_leaf("s1", 0).unwrap();
        assert!(m.messages("s1").unwrap().is_empty());
        let again = say(&m, "s1", "user", "a different start");
        assert_eq!(m.tree("s1").unwrap().nodes.iter().find(|n| n.id == again).unwrap().parent, None);
        assert!(m.plan_move("s1", 9999).is_err(), "a message that is not in this chat");
        assert!(m.set_leaf("s1", 9999).is_err());
    }

    #[test]
    fn tool_results_are_in_the_tree_but_never_in_what_the_model_reads_again() {
        let m = mem();
        m.start_conversation("s1", None).unwrap();
        say(&m, "s1", "user", "open it");
        let t = say(&m, "s1", "tool", "show {\"path\":\"x\"}");
        say(&m, "s1", "assistant", "opened");
        assert_eq!(m.tree("s1").unwrap().nodes.len(), 3);
        let roles: Vec<String> = m.messages("s1").unwrap().into_iter().map(|(r, _)| r).collect();
        assert_eq!(roles, vec!["user", "assistant"]);
        let plan = m.plan_move("s1", t).unwrap();
        assert!(plan.leaving.iter().all(|n| n.role != "tool"), "leaving a branch does not count tool results");
    }

    #[test]
    fn labels_can_be_set_changed_and_removed() {
        let m = mem();
        m.start_conversation("s1", None).unwrap();
        let a = say(&m, "s1", "user", "x");
        m.set_label("s1", a, "  the good start ").unwrap();
        assert_eq!(m.tree("s1").unwrap().nodes[0].label.as_deref(), Some("the good start"));
        m.set_label("s1", a, "renamed").unwrap();
        assert_eq!(m.tree("s1").unwrap().nodes[0].label.as_deref(), Some("renamed"));
        m.set_label("s1", a, "").unwrap();
        assert_eq!(m.tree("s1").unwrap().nodes[0].label, None);
        assert!(m.set_label("s1", 777, "x").is_err());
    }

    #[test]
    fn a_fork_copies_up_to_the_point_and_a_clone_copies_the_active_branch() {
        let m = mem();
        m.start_conversation("s1", Some("/proj")).unwrap();
        m.set_conversation_meta("s1", "model-a", "project").unwrap();
        m.set_title_if_empty("s1", "Plan the trip").unwrap();
        say(&m, "s1", "user", "q1");
        let a1 = say(&m, "s1", "assistant", "a1");
        let q2 = say(&m, "s1", "user", "q2");
        say(&m, "s1", "assistant", "a2");
        m.set_label("s1", a1, "good").unwrap();
        // fork at a user message: everything before it, and its text for the input box
        let input = m.fork("s1", q2, "s2").unwrap();
        assert_eq!(input.as_deref(), Some("q2"));
        let said: Vec<String> = m.messages("s2").unwrap().into_iter().map(|(_, t)| t).collect();
        assert_eq!(said, vec!["q1", "a1"]);
        let info = m.conversation("s2").unwrap().unwrap();
        assert_eq!((info.project.as_deref(), info.mode.as_str(), info.model.as_deref(), info.title.as_str()), (Some("/proj"), "project", Some("model-a"), "Plan the trip (fork)"));
        assert_eq!(m.tree("s2").unwrap().nodes.iter().find(|n| n.content == "a1").unwrap().label.as_deref(), Some("good"), "labels come along");
        // fork at an answer: up to and including it, no input
        assert_eq!(m.fork("s1", a1, "s3").unwrap(), None);
        assert_eq!(m.messages("s3").unwrap().len(), 2);
        // the original is untouched
        assert_eq!(m.messages("s1").unwrap().len(), 4);
        // clone copies the active branch
        m.set_leaf("s1", a1).unwrap();
        assert_eq!(m.clone_branch("s1", "s4").unwrap(), 2);
        assert_eq!(m.conversation("s4").unwrap().unwrap().title, "Plan the trip (copy)");
    }

    #[test]
    fn the_size_a_chat_reports_counts_only_the_active_branch() {
        let m = mem();
        m.start_conversation("s1", None).unwrap();
        say(&m, "s1", "user", "aaaa");
        let a = say(&m, "s1", "assistant", "bbbb");
        say(&m, "s1", "user", &"c".repeat(1000));
        assert_eq!(m.conversation("s1").unwrap().unwrap().chars, 1008);
        m.set_leaf("s1", a).unwrap();
        let info = m.conversation("s1").unwrap().unwrap();
        assert_eq!((info.messages, info.chars), (2, 8), "the long message is on another branch now");
    }

    #[test]
    fn deleting_a_chat_removes_its_tree_state_and_labels() {
        let m = mem();
        m.start_conversation("s1", None).unwrap();
        let a = say(&m, "s1", "user", "x");
        m.set_label("s1", a, "keep").unwrap();
        assert!(m.delete_conversation("s1").unwrap());
        let (b, l): (i64, i64) = m.store.with(|c| Ok((c.query_row("select count(*) from chat_branch", [], |r| r.get(0))?, c.query_row("select count(*) from chat_labels", [], |r| r.get(0))?))).unwrap();
        assert_eq!((b, l), (0, 0));
    }
}
