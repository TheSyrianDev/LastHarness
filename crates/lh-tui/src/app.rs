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
    /// The diff of a file write, or empty.
    pub detail: String,
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

/// Stored user text without the `[[lh-attach:..]]` marker; a note says how many files it carried.
pub fn user_text(stored: &str) -> String {
    let (t, n) = lh_client::attach::strip_marker(stored);
    if n == 0 { t } else { format!("{t}\n[{} attached]", lh_client::attach::count_label(n)) }
}

fn expand(p: &str) -> std::path::PathBuf {
    let p = p.trim().trim_matches(|c| c == '"' || c == '\'');
    match (p.strip_prefix("~/"), std::env::var_os("HOME")) {
        (Some(r), Some(h)) => std::path::Path::new(&h).join(r),
        _ => p.into(),
    }
}

/// Everything the screen shows. Pure data; the event loop feeds it and `ui::draw` shows it.
pub struct App {
    pub model: String,
    pub entries: Vec<Entry>,
    pub input: String,
    /// Files chosen with /attach, sent with the next message.
    pub stage: lh_client::attach::Stage,
    pub question: Option<Question>,
    pub busy: bool,
    pub show_thinking: bool,
    /// Lines scrolled up from the bottom; 0 follows the newest text.
    pub scroll: u16,
    pub usage: Option<(u64, u64, u64)>,
    /// Suggestions waiting for a decision; the first one is on screen.
    pub review: Vec<Suggestion>,
    pub incognito: bool,
    /// The chat tree, while it is open.
    pub tree: Option<crate::tree_ui::TreeScreen>,
    last_finish: Option<String>,
}

impl App {
    pub fn new(model: impl Into<String>) -> Self {
        Self { model: model.into(), entries: vec![], input: String::new(), stage: Default::default(), question: None, busy: false, show_thinking: false, scroll: 0, usage: None, review: vec![], incognito: false, tree: None, last_finish: None }
    }

    /// Show the active branch the core sent after a move in the tree: what was said, and a summary if one was kept.
    pub fn show_branch(&mut self, history: &[(String, String)], input: Option<String>) {
        self.entries = history
            .iter()
            .map(|(role, text)| match role.as_str() {
                "user" => Entry::User(user_text(text)),
                "assistant" => Entry::Assistant(text.clone()),
                _ => Entry::Notice(format!("Summary of another path: {text}")),
            })
            .collect();
        self.scroll = 0;
        if let Some(i) = input {
            self.input = i;
        }
    }

    /// Handle `/attach <path>`, `/detach <n>` and `/attach` alone. Returns false when `line` is not one of them.
    pub fn attach_command(&mut self, line: &str) -> bool {
        let line = line.trim();
        let note = if let Some(rest) = line.strip_prefix("/attach") {
            let rest = rest.trim();
            if rest.is_empty() {
                self.stage_summary().unwrap_or_else(|| "No files attached. Use /attach <path>.".into())
            } else {
                match self.stage.add(&expand(rest)) {
                    Ok(()) => self.stage_summary().unwrap_or_default(),
                    Err(e) => return self.notice(e),
                }
            }
        } else if let Some(rest) = line.strip_prefix("/detach") {
            match rest.trim().parse::<usize>() {
                Ok(n) if n >= 1 && n <= self.stage.files.len() => {
                    self.stage.remove(n - 1);
                    self.stage_summary().unwrap_or_else(|| "No files attached.".into())
                }
                _ => "Say which one: /detach <number>, as numbered above the input box.".into(),
            }
        } else {
            return false;
        };
        self.notice(note)
    }

    fn notice(&mut self, t: String) -> bool {
        self.entries.push(Entry::Notice(t));
        true
    }

    fn stage_summary(&self) -> Option<String> {
        if self.stage.is_empty() {
            return None;
        }
        Some(format!("Attached: {} ({}, {}).", self.stage.files.iter().map(|f| f.name.as_str()).collect::<Vec<_>>().join(", "), lh_client::attach::count_label(self.stage.files.len()), lh_client::attach::size_label(self.stage.total())))
    }

    /// The chips line above the input box: `[1 notes.pdf 2.0 KB] [2 pic.png 1.1 MB]`.
    pub fn chips(&self) -> String {
        self.stage.files.iter().enumerate().map(|(i, f)| format!("[{} {} {}]", i + 1, f.name, lh_client::attach::size_label(f.size))).collect::<Vec<_>>().join(" ")
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
            ServerEvent::Ask { id, tool, what, reason, worker, detail } => self.question = Some(Question { id, tool, what, reason, worker, detail }),
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
                ("table", _) => format!("[{title}]\n{}", table_text(&content)),
                ("chart", _) => format!("[{title}] chart\n{}", chart_text(&content)),
                ("app", _) => format!("[{title}] a small app; open this chat in the desktop app or browser to use it"),
                _ => format!("[{title}]\n{content}"),
            })),
            ServerEvent::Diff { path, diff, .. } => self.entries.push(Entry::Notice(format!("Changes to {path}:\n{diff}"))),
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

fn plain(v: &serde_json::Value) -> String {
    v.as_str().map(str::to_string).unwrap_or_else(|| if v.is_null() { String::new() } else { v.to_string() })
}

/// A table from the `show` tool as aligned text columns.
pub fn table_text(json: &str) -> String {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else { return json.to_string() };
    let mut rows: Vec<Vec<String>> = vec![v["columns"].as_array().map(|a| a.iter().map(plain).collect()).unwrap_or_default()];
    rows.extend(v["rows"].as_array().into_iter().flatten().map(|r| r.as_array().map(|a| a.iter().map(plain).collect()).unwrap_or_default()));
    let n = rows.iter().map(Vec::len).max().unwrap_or(0);
    let w: Vec<usize> = (0..n).map(|i| rows.iter().filter_map(|r| r.get(i)).map(|c| c.chars().count()).max().unwrap_or(0)).collect();
    rows.iter().map(|r| r.iter().enumerate().map(|(i, c)| format!("{c:<width$}", width = w[i])).collect::<Vec<_>>().join("  ").trim_end().to_string()).collect::<Vec<_>>().join("\n")
}

/// A chart from the `show` tool as one line per label with its values.
pub fn chart_text(json: &str) -> String {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else { return json.to_string() };
    let series = v["series"].as_array().cloned().unwrap_or_default();
    v["labels"].as_array().into_iter().flatten().enumerate().map(|(i, l)| {
        let vals: Vec<String> = series.iter().map(|s| { let x = plain(&s["values"][i]); match s["name"].as_str() { Some(n) => format!("{n} {x}"), None => x } }).collect();
        format!("{}: {}", plain(l), vals.join(", "))
    }).collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attach_commands_and_marker() {
        let d = std::env::temp_dir().join(format!("lh-tui-attach-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("a.txt");
        std::fs::write(&f, "hi").unwrap();
        let mut a = App::new("m");
        assert!(!a.attach_command("hello"));
        assert!(a.attach_command(&format!("/attach {}", f.display())));
        assert_eq!(a.stage.files.len(), 1);
        assert!(a.chips().starts_with("[1 a.txt 2 B]"));
        assert!(a.attach_command("/attach /no/such/file"));
        assert!(matches!(a.entries.last(), Some(Entry::Notice(t)) if t.contains("no file")));
        assert!(a.attach_command("/detach 1"));
        assert!(a.stage.is_empty());
        assert_eq!(user_text("see\n[[lh-attach:ab12]]"), "see\n[1 file attached]");
        a.show_branch(&[("user".into(), "x\n[[lh-attach:ab12,cd34]]".into())], None);
        assert_eq!(a.entries, [Entry::User("x\n[2 files attached]".into())]);
        std::fs::remove_dir_all(&d).ok();
    }

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
        a.apply(ServerEvent::Ask { id: "a1".into(), tool: "write_file".into(), what: "w".into(), reason: "r".into(), worker: "assistant".into(), detail: String::new() });
        assert_eq!(a.question.as_ref().unwrap().id, "a1");
        a.apply(ServerEvent::Error("boom".into()));
        assert!(a.question.is_none() && !a.busy);
    }

    #[test]
    fn a_move_in_the_tree_replaces_the_chat_with_the_branch_and_fills_the_input() {
        let mut a = App::new("m");
        a.start_turn("old".into());
        a.show_branch(&[("user".into(), "q1".into()), ("assistant".into(), "a1".into()), ("summary".into(), "found 42".into())], Some("q2".into()));
        assert_eq!(a.entries, vec![Entry::User("q1".into()), Entry::Assistant("a1".into()), Entry::Notice("Summary of another path: found 42".into())]);
        assert_eq!(a.input, "q2");
        a.show_branch(&[], None);
        assert_eq!(a.input, "q2", "no new words leaves what is typed alone");
        assert!(a.entries.is_empty());
    }
}

#[cfg(test)]
mod show_text_tests {
    #[test]
    fn tables_and_charts_print_as_text() {
        let t = super::table_text(r#"{"columns":["Name","Cores"],"rows":[["Laptop A",8],["B",16]]}"#);
        assert_eq!(t, "Name      Cores\nLaptop A  8\nB         16");
        let c = super::chart_text(r#"{"type":"bar","labels":["Jan","Feb"],"series":[{"name":"Sales","values":[3,5]}]}"#);
        assert_eq!(c, "Jan: Sales 3\nFeb: Sales 5");
    }
}
