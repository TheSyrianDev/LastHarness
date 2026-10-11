use crate::client::ServerEvent;

#[derive(Clone, Debug, PartialEq)]
pub enum Entry {
    User(String),
    Assistant(String),
    /// Thinking text, shown collapsed unless the user expands it.
    Thinking(String),
    Tool { name: String, arguments: String, result: Option<String>, id: String },
    Notice(String),
    Error(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Question {
    pub id: String,
    pub tool: String,
    pub what: String,
    pub reason: String,
    pub worker: String,
}

/// One suggestion from the end-of-session review, as the screen shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct Suggestion {
    pub id: i64,
    pub kind: String,
    pub layer: String,
    pub title: String,
    pub text: String,
    pub evidence: String,
    pub sensitive: bool,
    pub reason: String,
    /// From the learning side (rules, style, skills) rather than memory.
    pub learn: bool,
}

impl Suggestion {
    pub fn from_json(v: &serde_json::Value) -> Self {
        let s = |k: &str| v[k].as_str().unwrap_or("").to_string();
        Self { id: v["id"].as_i64().unwrap_or(0), kind: s("kind"), layer: s("layer"), title: s("title"), text: s("text"), evidence: s("evidence"), sensitive: v["privacy"] == "sensitive", reason: s("privacy_reason"), learn: false }
    }

    /// A suggestion from the learn pass.
    pub fn from_learn_json(v: &serde_json::Value) -> Self {
        let pl = &v["payload"];
        let p = |k: &str| pl[k].as_str().unwrap_or("").to_string();
        let kind = v["kind"].as_str().unwrap_or("");
        let (title, text) = match kind {
            "rule" => (p("topic"), p("text")),
            "rule_update" => (p("topic"), format!("{}\n(was: {})", p("text"), p("was"))),
            "style" => ("style profile".to_string(), p("added")),
            "skill_new" => (p("name"), format!("{}\n\n{}", p("description"), p("body"))),
            _ => (p("name"), format!("{}\n\n{}", p("note"), p("body"))),
        };
        Self {
            id: v["id"].as_i64().unwrap_or(0),
            kind: format!("learn_{kind}"),
            layer: v["source"].as_str().unwrap_or("").to_string(),
            title,
            text,
            evidence: v["evidence"].as_array().and_then(|e| e.first()).and_then(|e| e.as_str()).unwrap_or("").to_string(),
            sensitive: pl["privacy"] == "sensitive",
            reason: String::new(),
            learn: true,
        }
    }
}

/// Everything the screen shows. Pure data; the event loop feeds it and `ui::draw` shows it.
pub struct App {
    pub model: String,
    pub entries: Vec<Entry>,
    pub input: String,
    pub question: Option<Question>,
    pub busy: bool,
    pub show_thinking: bool,
    /// Lines scrolled up from the bottom; 0 follows the newest text.
    pub scroll: u16,
    pub usage: Option<(u64, u64, u64)>,
    /// Suggestions waiting for a decision; the first one is on screen.
    pub review: Vec<Suggestion>,
    pub incognito: bool,
    last_finish: Option<String>,
}

impl App {
    pub fn new(model: impl Into<String>) -> Self {
        Self { model: model.into(), entries: vec![], input: String::new(), question: None, busy: false, show_thinking: false, scroll: 0, usage: None, review: vec![], incognito: false, last_finish: None }
    }

    pub fn start_turn(&mut self, text: String) {
        self.entries.push(Entry::User(text));
        self.busy = true;
        self.scroll = 0;
        self.last_finish = None;
    }

    pub fn apply(&mut self, ev: ServerEvent) {
        match ev {
            ServerEvent::Text(t) => match self.entries.last_mut() {
                Some(Entry::Assistant(s)) => s.push_str(&t),
                _ => self.entries.push(Entry::Assistant(t)),
            },
            ServerEvent::Reasoning(t) => match self.entries.last_mut() {
                Some(Entry::Thinking(s)) => s.push_str(&t),
                _ => self.entries.push(Entry::Thinking(t)),
            },
            ServerEvent::ToolStart { id, name, arguments } => self.entries.push(Entry::Tool { name, arguments, result: None, id }),
            ServerEvent::ToolResult { id, text } => {
                if let Some(Entry::Tool { result, .. }) = self.entries.iter_mut().rev().find(|e| matches!(e, Entry::Tool { id: i, .. } if *i == id)) {
                    *result = Some(text);
                }
            }
            ServerEvent::Ask { id, tool, what, reason, worker } => self.question = Some(Question { id, tool, what, reason, worker }),
            ServerEvent::Usage { prompt_tokens, completion_tokens, cached_tokens } => self.usage = Some((prompt_tokens, completion_tokens, cached_tokens)),
            ServerEvent::Finish(r) => self.last_finish = Some(r),
            ServerEvent::Done { empty } => {
                self.busy = false;
                self.question = None;
                if empty && self.last_finish.as_deref() == Some("length") {
                    self.entries.push(Entry::Notice("The model ran out of tokens while thinking and gave no answer. Ask again, or use a model with a larger limit.".into()));
                } else if empty {
                    self.entries.push(Entry::Notice("The model gave an empty answer.".into()));
                }
            }
            ServerEvent::StepLimit => {
                self.busy = false;
                self.question = None;
                self.entries.push(Entry::Notice("Stopped: the model kept asking for tools and hit the step limit.".into()));
            }
            ServerEvent::Compacted { messages_summarized } => self.entries.push(Entry::Notice(format!("Earlier messages ({messages_summarized}) were replaced by a summary to make room."))),
            ServerEvent::Notice(m) => self.entries.push(Entry::Notice(m)),
            ServerEvent::Refused(m) => {
                self.busy = false;
                self.question = None;
                self.entries.push(Entry::Notice(m));
            }
            ServerEvent::Error(m) => {
                self.busy = false;
                self.question = None;
                self.entries.push(Entry::Error(m));
            }
            // the terminal has no side panel: what the assistant opened is printed here, an image as its path
            ServerEvent::Show { title, kind, content, path } => self.entries.push(Entry::Notice(match (kind.as_str(), path) {
                ("image", Some(p)) => format!("[{title}] image file: {p}"),
                _ => format!("[{title}]\n{content}"),
            })),
            ServerEvent::Other => {}
        }
        if matches!(self.entries.last(), Some(Entry::Assistant(_))) {
            self.scroll = 0;
        }
    }

    pub fn cancelled(&mut self) {
        self.busy = false;
        self.question = None;
        self.entries.push(Entry::Notice("Cancelled.".into()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_and_thinking_accumulate_in_their_own_entries() {
        let mut a = App::new("m");
        a.start_turn("hi".into());
        for t in ["a", "b"] {
            a.apply(ServerEvent::Reasoning(t.into()));
        }
        for t in ["Hel", "lo"] {
            a.apply(ServerEvent::Text(t.into()));
        }
        a.apply(ServerEvent::Done { empty: false });
        assert_eq!(a.entries, [Entry::User("hi".into()), Entry::Thinking("ab".into()), Entry::Assistant("Hello".into())]);
        assert!(!a.busy);
    }

    #[test]
    fn tool_result_finds_its_call() {
        let mut a = App::new("m");
        a.apply(ServerEvent::Show { title: "Plan".into(), kind: "markdown".into(), content: "# Plan".into(), path: None });
        a.apply(ServerEvent::Show { title: "Pic".into(), kind: "image".into(), content: String::new(), path: Some("/p/a.png".into()) });
        assert!(matches!(&a.entries[a.entries.len() - 2], Entry::Notice(n) if n == "[Plan]\n# Plan"));
        assert!(matches!(a.entries.last(), Some(Entry::Notice(n)) if n == "[Pic] image file: /p/a.png"));
        a.entries.clear();
        a.apply(ServerEvent::ToolStart { id: "1".into(), name: "read_file".into(), arguments: "{}".into() });
        a.apply(ServerEvent::ToolStart { id: "2".into(), name: "list_dir".into(), arguments: "{}".into() });
        a.apply(ServerEvent::ToolResult { id: "1".into(), text: "ok".into() });
        assert!(matches!(&a.entries[0], Entry::Tool { result: Some(r), .. } if r == "ok"));
        assert!(matches!(&a.entries[1], Entry::Tool { result: None, .. }));
    }

    #[test]
    fn running_out_of_tokens_is_explained() {
        let mut a = App::new("m");
        a.start_turn("x".into());
        a.apply(ServerEvent::Reasoning("thinking".into()));
        a.apply(ServerEvent::Finish("length".into()));
        a.apply(ServerEvent::Done { empty: true });
        assert!(matches!(a.entries.last(), Some(Entry::Notice(n)) if n.contains("ran out of tokens")));
    }

    #[test]
    fn a_question_blocks_until_the_turn_ends() {
        let mut a = App::new("m");
        a.apply(ServerEvent::Ask { id: "a1".into(), tool: "write_file".into(), what: "w".into(), reason: "r".into(), worker: "assistant".into() });
        assert_eq!(a.question.as_ref().unwrap().id, "a1");
        a.apply(ServerEvent::Error("boom".into()));
        assert!(a.question.is_none() && !a.busy);
    }
}
