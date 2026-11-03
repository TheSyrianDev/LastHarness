use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::app::{App, Entry};

const MAX_TOOL_LINES: usize = 6;

fn clip(s: &str, n: usize) -> String {
    let one: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if one.chars().count() > n {
        format!("{}…", one.chars().take(n).collect::<String>())
    } else {
        one
    }
}

fn lines(app: &App) -> Vec<Line<'static>> {
    let dim = Style::new().add_modifier(Modifier::DIM);
    let mut out: Vec<Line> = vec![];
    for e in &app.entries {
        match e {
            Entry::User(t) => {
                out.push(Line::from(Span::styled("you", Style::new().bold().fg(Color::Cyan))));
                out.extend(t.lines().map(|l| Line::from(l.to_string())));
            }
            Entry::Assistant(t) => {
                out.push(Line::from(Span::styled("assistant", Style::new().bold().fg(Color::Green))));
                out.extend(t.lines().map(|l| Line::from(l.to_string())));
            }
            Entry::Thinking(t) if app.show_thinking => {
                out.extend(t.lines().map(|l| Line::from(Span::styled(format!("│ {l}"), dim))));
            }
            Entry::Thinking(t) => out.push(Line::from(Span::styled(format!("· thinking ({} characters, Ctrl-T to show)", t.chars().count()), dim))),
            Entry::Tool { name, arguments, result, .. } => {
                out.push(Line::from(vec![Span::styled("⚙ ", Style::new().fg(Color::Yellow)), Span::raw(format!("{name} {}", clip(arguments, 100)))]));
                if let Some(r) = result {
                    for l in r.lines().take(MAX_TOOL_LINES) {
                        out.push(Line::from(Span::styled(format!("  {}", clip(l, 120)), dim)));
                    }
                    if r.lines().count() > MAX_TOOL_LINES {
                        out.push(Line::from(Span::styled(format!("  … {} more lines", r.lines().count() - MAX_TOOL_LINES), dim)));
                    }
                }
            }
            Entry::Notice(t) => out.push(Line::from(Span::styled(t.clone(), Style::new().fg(Color::Yellow)))),
            Entry::Error(t) => out.push(Line::from(Span::styled(format!("error: {t}"), Style::new().fg(Color::Red).bold()))),
        }
        out.push(Line::raw(""));
    }
    out
}

fn draw_tree(f: &mut Frame, screen: &crate::tree_ui::TreeScreen) {
    use crate::tree_ui::Prompt;
    let area = f.area();
    f.render_widget(Clear, area);
    let rows = screen.rows();
    let sel = rows.iter().position(|r| r.id == screen.at).unwrap_or(0);
    let inner_h = area.height.saturating_sub(4) as usize;
    let first = if sel >= inner_h { sel + 1 - inner_h } else { 0 };
    let dim = Style::new().add_modifier(Modifier::DIM);
    let lines: Vec<Line> = rows
        .iter()
        .enumerate()
        .skip(first)
        .take(inner_h)
        .map(|(i, r)| {
            let marker = if r.current { "◆" } else if r.active { "●" } else { "○" };
            let who = match r.role.as_str() {
                "user" => "you",
                "assistant" => "assistant",
                "tool" => "tool",
                "summary" => "summary",
                _ => "",
            };
            let fold = if r.folded { format!(" ▸ +{}", r.hidden_below) } else if r.foldable { " ▾".to_string() } else { String::new() };
            let label = r.label.as_ref().map(|l| format!(" [{l}]")).unwrap_or_default();
            let text = format!("{}{marker} {}{}{label}{fold}", "  ".repeat(r.depth), if who.is_empty() { String::new() } else { format!("{who}: ") }, clip(&r.text, 80));
            let style = if i == sel { Style::new().add_modifier(Modifier::REVERSED) } else if r.active { Style::new().fg(Color::Green) } else { dim };
            Line::from(Span::styled(text, style))
        })
        .collect();
    let title = format!("Chat tree ({}{})", if screen.tree.hide_tools { "tool results hidden, " } else { "" }, "● the branch you are on, ◆ where the chat is");
    let outer = Rect { x: area.x, y: area.y, width: area.width, height: area.height.saturating_sub(2) };
    f.render_widget(Paragraph::new(lines).block(Block::bordered().title(title)), outer);
    let footer = Rect { x: area.x, y: area.y + area.height.saturating_sub(2), width: area.width, height: 2.min(area.height) };
    let (first_line, second) = match &screen.prompt {
        Prompt::Label { text, .. } => (Line::from(Span::styled(format!("Name for this line (Enter saves, Esc cancels, empty removes it): {text}_"), Style::new().fg(Color::Yellow).bold())), String::new()),
        Prompt::Summary { leaving, .. } => (Line::from(Span::styled(format!("You are leaving {leaving} message(s) on this branch. Keep a short summary of them where you land?  y = yes   n = no   Esc = stay"), Style::new().fg(Color::Yellow).bold())), String::new()),
        Prompt::None => (Line::from(screen.note.clone()), "Up/Down move  Enter go here  Left/Right/Space fold  t hide tool results  l name  f fork into a new chat  Esc close".to_string()),
    };
    f.render_widget(Paragraph::new(vec![first_line, Line::from(Span::styled(second, dim))]), footer);
}

/// The status colours of the LastHarness brand (Grove palette).
pub fn slot_colour(asking: bool, busy: bool) -> Color {
    if asking {
        Color::Rgb(0xd0, 0x78, 0x4f)
    } else if busy {
        Color::Rgb(0xe0, 0xa0, 0x50)
    } else {
        Color::Rgb(0x6f, 0xcf, 0x9f)
    }
}

pub fn draw(f: &mut Frame, app: &App) {
    if let Some(t) = &app.tree {
        draw_tree(f, t);
        return;
    }
    let rows = Layout::vertical([Constraint::Min(1), Constraint::Length(3), Constraint::Length(1)]).split(f.area());
    let body = lines(app);
    let para = Paragraph::new(body).wrap(Wrap { trim: false });
    let total = para.line_count(rows[0].width) as u16;
    let max_scroll = total.saturating_sub(rows[0].height);
    let from_top = max_scroll.saturating_sub(app.scroll.min(max_scroll));
    f.render_widget(para.scroll((from_top, 0)), rows[0]);

    let title = if app.incognito { "you (incognito: nothing is remembered)" } else if app.busy { "you (working, Esc to cancel)" } else { "you (Enter to send, /review to look at what to remember, /tree for branches)" };
    f.render_widget(Paragraph::new(app.input.as_str()).block(Block::bordered().title(title)), rows[1]);

    let usage = app.usage.map(|(p, c, k)| format!("  tokens: {p} in ({k} cached), {c} out")).unwrap_or_default();
    let hint = "Ctrl-T thinking  PgUp/PgDn scroll  Ctrl-D quit";
    // the brand's slot colour as a status dot: busy while a turn runs, needs you while a question waits, ready otherwise
    let dot = Span::styled(" ●", Style::new().fg(slot_colour(app.question.is_some(), app.busy)));
    let status = Line::from(vec![dot, Span::styled(format!(" {}{usage}   {hint}", app.model), Style::new().add_modifier(Modifier::DIM))]);
    f.render_widget(Paragraph::new(status), rows[2]);

    if let Some(sg) = app.review.first() {
        let area = centered(f.area(), 80, 16);
        f.render_widget(Clear, area);
        let what = match sg.kind.as_str() {
            "update" => "Change a note",
            "archive" => "Archive a note",
            "learn_rule" => "Learn this rule",
            "learn_rule_update" => "Change a rule",
            "learn_style" => "Add to your style profile",
            "learn_skill_new" => "Save as a skill",
            "learn_skill_update" => "Change a skill",
            _ => "Remember this",
        };
        let mut text = vec![
            Line::from(Span::styled(format!("{what} ({} left)", app.review.len()), Style::new().bold())),
            Line::raw(""),
            Line::from(Span::styled(if sg.learn { format!("({}) {}", sg.layer, sg.title) } else { format!("{}: {}", sg.layer, sg.title) }, Style::new().bold())),
            Line::raw(clip(&sg.text, 400)),
        ];
        if !sg.evidence.is_empty() {
            text.push(Line::from(Span::styled(format!("you said: \"{}\"", clip(&sg.evidence, 150)), Style::new().add_modifier(Modifier::DIM))));
        }
        text.push(Line::from(if sg.sensitive { format!("Sensitive ({}): stays on this machine", sg.reason) } else { "Not sensitive: may be shared with cloud models".to_string() }));
        if sg.learn {
            text.push(Line::from(Span::styled("It applies to every future session until you turn it off or roll it back.", Style::new().add_modifier(Modifier::DIM))));
        }
        text.push(Line::raw(""));
        text.push(Line::from(Span::styled("y = remember   n = drop   s = later   p = flip sensitive   Esc = close", Style::new().fg(Color::Yellow).bold())));
        f.render_widget(Paragraph::new(text).wrap(Wrap { trim: true }).block(Block::default().borders(Borders::ALL).title("Memory review")), area);
    }
    if let Some(q) = &app.question {
        // a file write shows its diff, as much as fits on the screen
        let diff: Vec<&str> = q.detail.lines().take(f.area().height.saturating_sub(12) as usize).collect();
        let area = centered(f.area(), 70, 9 + diff.len() as u16 + u16::from(!diff.is_empty()));
        f.render_widget(Clear, area);
        let mut text = vec![
            Line::from(Span::styled(if q.tool == "model_call" { "Spending check".to_string() } else { format!("{} wants to use {}", q.worker, q.tool) }, Style::new().bold())),
            Line::raw(""),
            Line::raw(clip(&q.what, 200)),
            Line::from(format!("Why it asks: {}", q.reason)),
        ];
        if !diff.is_empty() {
            text.push(Line::raw(""));
            for l in diff {
                let colour = if l.starts_with('+') { Color::Green } else if l.starts_with('-') { Color::Red } else { Color::Reset };
                text.push(Line::from(Span::styled(l.to_string(), Style::new().fg(colour))));
            }
        }
        text.push(Line::raw(""));
        text.push(Line::from(Span::styled("y = allow    n = refuse", Style::new().fg(Color::Yellow).bold())));
        f.render_widget(Paragraph::new(text).wrap(Wrap { trim: true }).block(Block::default().borders(Borders::ALL).title("Permission needed")), area);
    }
}

fn centered(area: Rect, width_pct: u16, height: u16) -> Rect {
    let w = area.width * width_pct / 100;
    Rect { x: area.x + (area.width - w) / 2, y: area.y + area.height.saturating_sub(height) / 2, width: w, height: height.min(area.height) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::ServerEvent;
    use ratatui::backend::TestBackend;

    fn screen(app: &App, w: u16, h: u16) -> String {
        let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
        t.draw(|f| draw(f, app)).unwrap();
        let buf = t.backend().buffer().clone();
        (0..h).map(|y| (0..w).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>()).collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn shows_conversation_tools_and_the_status_line() {
        let mut a = App::new("tiny-model");
        a.start_turn("read it".into());
        a.apply(ServerEvent::Reasoning("hmm".into()));
        a.apply(ServerEvent::ToolStart { id: "1".into(), name: "read_file".into(), arguments: "{\"path\":\"a.txt\"}".into() });
        a.apply(ServerEvent::ToolResult { id: "1".into(), text: "file text".into() });
        a.apply(ServerEvent::Text("It says file text.".into()));
        a.apply(ServerEvent::Usage { prompt_tokens: 10, completion_tokens: 5, cached_tokens: 2 });
        let s = screen(&a, 80, 20);
        for want in ["you", "read it", "thinking (3 characters", "read_file", "file text", "assistant", "It says file text.", "tiny-model", "10 in (2 cached)"] {
            assert!(s.contains(want), "missing {want:?} in:\n{s}");
        }
    }

    #[test]
    fn the_tree_screen_marks_the_branch_you_are_on_and_shows_names_folds_and_the_keys() {
        use crate::tree_ui::{Prompt, TreeScreen};
        let n = |id: i64, parent: Option<i64>, role: &str, preview: &str, label: Option<&str>| serde_json::json!({"id": id, "parent": parent, "role": role, "preview": preview, "chars": 1, "label": label});
        let v = serde_json::json!({"leaf": 4, "active": [1, 2, 4], "summarize": "ask", "nodes": [
            n(1, None, "user", "first question", None), n(2, Some(1), "assistant", "first answer", Some("good start")), n(3, Some(2), "user", "other path", None), n(4, Some(2), "user", "this path", None)]});
        let mut a = App::new("m");
        let mut t = TreeScreen::new(lh_client::tree::ChatTree::from_json(&v).unwrap());
        t.tree.toggle_fold(3);
        a.tree = Some(t);
        let s = screen(&a, 100, 16);
        for want in ["Chat tree", "start of the chat", "you: first question", "assistant: first answer [good start]", "you: this path", "◆", "●", "○", "Enter go here", "fold"] {
            assert!(s.contains(want), "missing {want:?} in:\n{s}");
        }
        a.tree.as_mut().unwrap().prompt = Prompt::Summary { node: 3, leaving: 2 };
        let s = screen(&a, 120, 16);
        assert!(s.contains("leaving 2 message(s)") && s.contains("y = yes"), "{s}");
        a.tree.as_mut().unwrap().prompt = Prompt::Label { node: 2, text: "keep".into() };
        assert!(screen(&a, 120, 16).contains("keep_"));
        a.tree = None;
        assert!(screen(&a, 100, 16).contains("Enter to send"), "closing it brings the chat back");
    }

    #[test]
    fn a_question_is_drawn_over_the_chat() {
        let mut a = App::new("m");
        a.apply(ServerEvent::Ask { id: "a1".into(), tool: "write_file".into(), what: "Write(\"/p/x\")".into(), reason: "changes files in the project".into(), worker: "assistant".into(), detail: String::new() });
        let s = screen(&a, 80, 20);
        assert!(s.contains("Permission needed") && s.contains("assistant wants to use write_file") && s.contains("y = allow"), "{s}");
    }

    #[test]
    fn the_review_screen_shows_a_suggestion_and_how_to_decide() {
        let mut a = App::new("m");
        a.review = vec![
            crate::app::Suggestion { id: 7, kind: "new".into(), layer: "facts".into(), title: "Home city".into(), text: "Lives in Berlin.".into(), evidence: "I live in Berlin".into(), sensitive: false, reason: String::new(), learn: false },
            crate::app::Suggestion { id: 8, kind: "update".into(), layer: "facts".into(), title: "Wifi".into(), text: "Password hunter2".into(), evidence: String::new(), sensitive: true, reason: "credentials".into(), learn: false },
        ];
        let s = screen(&a, 100, 24);
        for want in ["Memory review", "Remember this (2 left)", "facts: Home city", "Lives in Berlin.", "you said: \"I live in Berlin\"", "Not sensitive", "y = remember", "p = flip sensitive"] {
            assert!(s.contains(want), "missing {want:?} in:\n{s}");
        }
        a.review.remove(0);
        let s = screen(&a, 100, 24);
        assert!(s.contains("Change a note (1 left)") && s.contains("Sensitive (credentials): stays on this machine"), "{s}");
        a.incognito = true;
        assert!(screen(&a, 100, 24).contains("incognito: nothing is remembered"));
    }

    #[test]
    fn the_learn_screen_shows_what_would_change_and_why() {
        let mut a = App::new("m");
        let rule = serde_json::json!({"id": 4, "kind": "rule", "source": "repeat", "payload": {"topic": "bullets", "text": "Do not use bullet points.", "privacy": "normal"}, "evidence": ["never use bullet points"]});
        let upd = serde_json::json!({"id": 5, "kind": "skill_update", "source": "skill", "payload": {"name": "release-notes", "note": "use the last tag", "body": "1. Run git log since the last tag."}, "evidence": []});
        a.review = vec![crate::app::Suggestion::from_learn_json(&rule), crate::app::Suggestion::from_learn_json(&upd)];
        let s = screen(&a, 100, 24);
        for want in ["Learn this rule (2 left)", "(repeat) bullets", "Do not use bullet points.", "you said: \"never use bullet points\"", "every future session", "y = remember"] {
            assert!(s.contains(want), "missing {want:?} in:\n{s}");
        }
        a.review.remove(0);
        let s = screen(&a, 100, 24);
        assert!(s.contains("Change a skill (1 left)") && s.contains("release-notes") && s.contains("use the last tag"), "{s}");
    }

    #[test]
    fn long_chats_follow_the_bottom_and_can_scroll_up() {
        let mut a = App::new("m");
        for i in 0..60 {
            a.apply(ServerEvent::Text(format!("line {i}\n")));
        }
        assert!(screen(&a, 60, 12).contains("line 59"));
        a.scroll = 1000;
        let s = screen(&a, 60, 12);
        assert!(s.contains("line 0") && !s.contains("line 59"), "{s}");
    }
}
