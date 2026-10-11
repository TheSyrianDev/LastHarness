//! The right panel (departments, memory, settings, opened content) and a department's own page.

use gpui::prelude::*;
use gpui::*;
use gpui_component::input::Textarea;
use gpui_component::text::TextView;

use crate::app::App;
use lh_gui::layout::Panel;
use lh_gui::runs::{Outcome, Question, Run, StepState};
use lh_gui::settings;
use crate::ui::Kind;

/// Something the agent (or the user) opened to look at properly.
#[derive(Clone, Debug, PartialEq)]
pub struct Content {
    pub title: String,
    pub kind: ContentKind,
    pub body: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentKind {
    Markdown,
    Code,
    Text,
    /// `body` is a file path.
    Image,
}

impl Content {
    /// Choose a viewer for a piece of text or a path.
    pub fn detect(title: &str, body: &str) -> Content {
        let t = body.trim();
        let kind = if !t.contains('\n') && [".png", ".jpg", ".jpeg", ".gif", ".webp", ".bmp"].iter().any(|e| t.to_lowercase().ends_with(e)) && std::path::Path::new(t).is_file() {
            ContentKind::Image
        } else if t.starts_with("```") && t.trim_end().ends_with("```") && t.matches("```").count() == 2 {
            ContentKind::Code
        } else if t.lines().any(|l| l.starts_with('#') || l.starts_with("- ") || l.starts_with("* ") || l.starts_with("> ") || l.contains("](") || l.starts_with("|") || l.contains("**") || l.starts_with("```")) {
            ContentKind::Markdown
        } else {
            ContentKind::Text
        };
        let body = if kind == ContentKind::Code { t.trim_start_matches("```").lines().skip(1).collect::<Vec<_>>().join("\n").trim_end_matches("```").trim_end().to_string() } else { body.to_string() };
        Content { title: title.to_string(), kind, body }
    }
}

fn state_mark(s: StepState) -> &'static str {
    match s {
        StepState::Waiting => "·",
        StepState::Running => "…",
        StepState::Done => "✓",
        StepState::Failed => "✗",
        StepState::Parked => "‖",
        StepState::Stopped => "■",
        StepState::Blocked => "⊘",
    }
}

impl App {
    pub fn right_panel(&mut self, width: f32, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let rtl = self.rtl;
        let panel = self.layout().panel;
        let expanded = self.layout().right_expanded;
        let title = match panel {
            Panel::Departments => if rtl { "الأقسام" } else { "Departments" },
            Panel::Memory => if rtl { "الذاكرة" } else { "Memory" },
            Panel::Settings => if rtl { "الإعدادات" } else { "Settings" },
            Panel::Content => if rtl { "المحتوى" } else { "Opened content" },
        };
        let expand = self.button("panel-expand", if expanded { "Smaller" } else { "Larger" }, Kind::Ghost, cx, |a, _, cx| {
            a.layout_mut().toggle_expand();
            a.persist();
            cx.notify();
        });
        let narrower = self.button("panel-narrower", "−", Kind::Ghost, cx, |a, _, cx| {
            let w = a.layout().right_width - 60.;
            a.layout_mut().resize_right(w);
            a.persist();
            cx.notify();
        });
        let wider = self.button("panel-wider", "+", Kind::Ghost, cx, |a, _, cx| {
            let w = a.layout().right_width + 60.;
            a.layout_mut().resize_right(w);
            a.persist();
            cx.notify();
        });
        let hide = self.button("panel-hide", if rtl { "إخفاء" } else { "Hide" }, Kind::Ghost, cx, |a, _, cx| {
            a.layout_mut().hide_right();
            a.persist();
            cx.notify();
        });
        let body = match panel {
            Panel::Departments => self.departments_body(cx),
            Panel::Memory => self.memory_body(cx),
            Panel::Settings => self.settings_body(cx),
            Panel::Content => self.content_body(cx),
        };
        let mut head = div().flex().when(rtl, |d| d.flex_row_reverse()).items_center().gap_1().child(div().flex_1().text_color(p.text).child(title));
        if !expanded {
            head = head.child(narrower).child(wider);
        }
        div()
            .id("right-panel")
            .role(Role::Group)
            .aria_label(title)
            .flex()
            .flex_col()
            .gap_2()
            .w(px(width))
            .h_full()
            .p_2()
            .rounded(p.radius)
            .bg(p.surface)
            .shadow(p.shadow.clone())
            .child(head.child(expand).child(hide))
            .child(div().id("panel-body").flex_1().overflow_y_scroll().flex().flex_col().gap_2().child(body))
            .into_any_element()
    }

    fn note(&self, t: impl Into<SharedString>) -> AnyElement {
        div().text_sm().text_color(self.pal.muted).child(t.into()).into_any_element()
    }

    fn departments_body(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let mut col = div().flex().flex_col().gap_2();
        if let Some(e) = self.panels.departments_error.clone() {
            return col.child(self.note(format!("Departments are not available: {e}"))).into_any_element();
        }
        if !self.connected {
            return col.child(self.note("The core is not running, so there are no departments to show.")).into_any_element();
        }
        if self.panels.departments.is_empty() {
            col = col.child(self.note("No departments yet. Make one with `lh team` and it appears here."));
        }
        for d in self.panels.departments.clone() {
            let name = d.name.clone();
            let running = self.panels.runs.values().any(|r| r.dept == d.name && !r.finished);
            let waiting = self.panels.runs.values().any(|r| r.dept == d.name && r.is_waiting());
            let open = self.button(format!("dept-{name}"), "Open page", Kind::Primary, cx, {
                let n = name.clone();
                move |a, _, cx| a.open_department(&n, cx)
            });
            col = col.child(
                div().flex().flex_col().gap_1().p_2().rounded(p.inner).bg(p.sunken).text_color(p.text)
                    .child(div().flex().items_center().gap_2().child(div().flex_1().child(d.name.clone())).child(div().text_sm().text_color(if waiting { p.accent } else { p.muted }).child(if waiting { "waiting for you" } else if running { "working" } else { "idle" })))
                    .when(!d.description.is_empty(), |x| x.child(div().text_sm().text_color(p.muted).child(d.description.clone())))
                    .child(div().text_xs().text_color(p.muted).child(format!("lead: {}   members: {}", d.lead, d.members.join(", "))))
                    .child(div().flex().child(open)),
            );
        }
        col.into_any_element()
    }

    fn memory_body(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let mut col = div().flex().flex_col().gap_2();
        if let Some(e) = self.panels.memory_error.clone() {
            return col.child(self.note(format!("Memory is not available: {e}"))).into_any_element();
        }
        if !self.connected {
            return col.child(self.note("The core is not running, so memory cannot be shown.")).into_any_element();
        }
        if self.panels.pages.is_empty() {
            col = col.child(self.note("Nothing is remembered yet. What the assistant learns about you shows up here, and you can edit or remove it."));
        }
        for (i, pg) in self.panels.pages.clone().into_iter().enumerate() {
            let open = self.open_blocks.contains(&(0, i));
            let toggle = self.button(format!("mem-{i}"), if open { "Hide" } else { "Read" }, Kind::Ghost, cx, move |a, _, cx| {
                if !a.open_blocks.remove(&(0, i)) {
                    a.open_blocks.insert((0, i));
                }
                cx.notify();
            });
            let show = {
                let c = Content::detect(&pg.title, &pg.body);
                self.button(format!("mem-open-{i}"), "Open in panel", Kind::Ghost, cx, move |a, _, cx| a.open_content(c.clone(), cx))
            };
            col = col.child(
                div().flex().flex_col().gap_1().p_2().rounded(p.inner).bg(p.sunken).text_color(p.text)
                    .child(div().flex().items_center().gap_2().child(div().flex_1().child(format!("{}{}", if pg.pinned { "📌 " } else { "" }, pg.title))).child(toggle))
                    .child(div().text_xs().text_color(p.muted).child(format!("{} · {}{}", pg.layer, pg.privacy, if pg.gist.is_empty() { String::new() } else { format!(" · {}", pg.gist) })))
                    .when(open, |d| d.child(TextView::markdown(SharedString::from(format!("page{i}")), pg.body.clone())).child(div().flex().child(show))),
            );
        }
        col.into_any_element()
    }

    fn settings_body(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let mut col = div().flex().flex_col().gap_2();
        if let Some(e) = self.panels.settings_error.clone() {
            col = col.child(div().px_2().py_1().rounded(p.inner).border_1().border_color(p.danger).text_color(p.danger).child(e));
        }
        // how the app looks is the app's own setting, so it works even when the core is away
        col = col.child(div().text_sm().text_color(p.muted).child("appearance"));
        let mut looks = div().flex().flex_wrap().gap_1();
        for i in 0..self.themes.themes.len() {
            let label = self.themes.themes[i].label();
            looks = looks.child(self.button(format!("look-{i}"), label, if i == self.active_theme { Kind::Primary } else { Kind::Normal }, cx, move |a, w, cx| a.set_theme(i, w, cx)));
        }
        let dir = match self.prefs.direction {
            lh_gui::rtl::Direction::Auto => "Direction: follow the desktop language",
            lh_gui::rtl::Direction::Ltr => "Direction: left to right",
            lh_gui::rtl::Direction::Rtl => "Direction: right to left (mirrored)",
        };
        let dir_btn = self.button("look-direction", dir, Kind::Normal, cx, |a, _, cx| {
            let n = a.prefs.direction.next();
            a.set_direction(n);
            cx.notify();
        });
        let narrower = self.button("left-narrower", "Chat list −", Kind::Ghost, cx, |a, _, cx| {
            let w = a.layout().left_width - 30.;
            a.layout_mut().resize_left(w);
            a.persist();
            cx.notify();
        });
        let wider = self.button("left-wider", "Chat list +", Kind::Ghost, cx, |a, _, cx| {
            let w = a.layout().left_width + 30.;
            a.layout_mut().resize_left(w);
            a.persist();
            cx.notify();
        });
        col = col.child(
            div().flex().flex_col().gap_2().p_2().rounded(p.inner).bg(p.sunken).text_color(p.text)
                .id("appearance").role(Role::Group).aria_label("Appearance")
                .child(looks)
                .child(div().flex().flex_wrap().gap_1().child(dir_btn).child(narrower).child(wider))
                .child(div().text_xs().text_color(p.muted).child(format!("Your own looks go in {} as .toml files; changes show at once.", self.cfg.themes.display())))
                .children(self.themes.problems.iter().map(|e| div().text_xs().text_color(p.danger).child(format!("Theme file problem: {e}")))),
        );
        if !self.connected {
            return col.child(self.note("The core is not running, so its settings cannot be shown.")).into_any_element();
        }
        let groups = settings::groups(&self.panels.settings, |s| s.group.as_str());
        for (g, idx) in groups {
            col = col.child(div().text_sm().text_color(p.muted).pt_2().child(g));
            for i in idx {
                let s = self.panels.settings[i].clone();
                let control: AnyElement = match &s.kind {
                    settings::Kind::Bool | settings::Kind::Choice(_) => {
                        let (key, cur, kind) = (s.key.clone(), s.value.clone(), s.kind.clone());
                        self.button(format!("set-{}", s.key), s.value.clone(), Kind::Normal, cx, move |a, _, cx| {
                            if let Some(v) = kind.next(&cur) {
                                a.change_setting(&key, v, cx);
                            }
                        })
                    }
                    settings::Kind::Int { .. } => {
                        let (k1, c1, kind1) = (s.key.clone(), s.value.clone(), s.kind.clone());
                        let (k2, c2, kind2) = (s.key.clone(), s.value.clone(), s.kind.clone());
                        let minus = self.button(format!("set-{}-minus", s.key), "−", Kind::Ghost, cx, move |a, _, cx| {
                            if let Some(v) = kind1.step(&c1, false) {
                                a.change_setting(&k1, v, cx);
                            }
                        });
                        let plus = self.button(format!("set-{}-plus", s.key), "+", Kind::Ghost, cx, move |a, _, cx| {
                            if let Some(v) = kind2.step(&c2, true) {
                                a.change_setting(&k2, v, cx);
                            }
                        });
                        div().flex().items_center().gap_1().child(minus).child(div().min_w(px(36.)).flex().justify_center().child(s.value.clone())).child(plus).into_any_element()
                    }
                    settings::Kind::Other => div().text_color(p.muted).child(if s.value.is_empty() { "(not set)".to_string() } else { s.value.clone() }).into_any_element(),
                };
                col = col.child(
                    div().flex().flex_col().gap_1().p_2().rounded(p.inner).bg(p.sunken).text_color(p.text)
                        .id(SharedString::from(format!("setting-{}", s.key))).role(Role::Group).aria_label(s.key.clone())
                        .child(div().flex().items_center().gap_2().child(div().flex_1().child(s.key.clone())).child(control))
                        .child(div().text_xs().text_color(p.muted).child(s.help.clone()))
                        .when(!s.kind.editable(), |d| d.child(div().text_xs().text_color(p.muted).child("Change this with `lh settings`."))),
                );
            }
        }
        col.into_any_element()
    }

    fn content_body(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let mut col = div().flex().flex_col().gap_2();
        if self.content.is_empty() {
            return col.child(self.note("Nothing opened yet. Use \"Open in panel\" on a reply, a tool result or a memory note to read it here, larger, or in its own window.")).into_any_element();
        }
        let cur = self.content_index.min(self.content.len() - 1);
        let tabs: Vec<AnyElement> = (0..self.content.len()).map(|i| {
            let t = self.content[i].title.chars().take(24).collect::<String>();
            self.button(format!("content-{i}"), if t.is_empty() { "Untitled".to_string() } else { t }, if i == cur { Kind::Primary } else { Kind::Ghost }, cx, move |a, _, cx| {
                a.content_index = i;
                cx.notify();
            })
        }).collect();
        let item = self.content[cur].clone();
        let pop = { let it = item.clone(); self.button("content-popout", "Pop out", Kind::Normal, cx, move |a, _, cx| a.pop_out(it.clone(), cx)) };
        let close = self.button("content-close", "Close", Kind::Ghost, cx, move |a, _, cx| {
            a.content.remove(cur);
            a.content_index = a.content_index.saturating_sub(1);
            if a.content.is_empty() {
                a.layout_mut().toggle_panel(Panel::Content);
            }
            cx.notify();
        });
        col = col.child(div().flex().flex_wrap().gap_1().children(tabs)).child(div().flex().gap_2().child(pop).child(close)).child(viewer(&item, "panel", &p));
        col.into_any_element()
    }


    pub fn open_content(&mut self, c: Content, cx: &mut Context<Self>) {
        self.content.push(c);
        self.content_index = self.content.len() - 1;
        self.layout_mut().show_panel(Panel::Content);
        self.persist();
        cx.notify();
    }

    pub fn department_page(&mut self, name: &str, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let rtl = self.rtl;
        let dept = self.panels.departments.iter().find(|d| d.name == name).cloned();
        let back = self.button("dept-back", if rtl { "العودة إلى المحادثة" } else { "Back to the chat" }, Kind::Normal, cx, |a, _, cx| {
            a.layout_mut().back_to_chat();
            cx.notify();
        });
        let n = name.to_string();
        let start = self.button("dept-start", "Start", Kind::Primary, cx, move |a, w, cx| a.start_run(&n, w, cx));
        let mut col = div().flex().flex_col().gap_3().p_3();
        col = col.child(div().flex().when(rtl, |d| d.flex_row_reverse()).items_center().gap_2().child(div().flex_1().text_color(p.text).child(name.to_string())).child(back));
        match &dept {
            Some(d) => {
                col = col.child(div().text_sm().text_color(p.muted).child(format!("{}\nlead: {}   members: {}", d.description, d.lead, d.members.join(", "))));
            }
            None => col = col.child(self.note("This department is not known to the core (it may have been removed).")),
        }
        col = col.child(
            div().flex().flex_col().gap_2().p_2().rounded(p.radius).bg(p.surface).shadow(p.shadow.clone())
                .child(Textarea::new(&self.goal).aria_label("Goal"))
                .when_some(self.panels.goal_note.clone(), |d, n| d.child(div().text_sm().text_color(p.danger).child(n)))
                .child(div().flex().child(start)),
        );
        let runs: Vec<Run> = self.panels.run_order.iter().rev().filter_map(|id| self.panels.runs.get(id)).filter(|r| r.dept == name).cloned().collect();
        if runs.is_empty() {
            col = col.child(self.note("No runs yet. Give the department a goal above."));
        }
        for r in runs {
            col = col.child(self.run_card(&r, cx));
        }
        div().id("dept-page").role(Role::Group).aria_label(format!("Department {name}")).flex_1().overflow_y_scroll().child(col).into_any_element()
    }

    fn run_card(&mut self, r: &Run, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let (done, total) = r.progress();
        let mut col = div().flex().flex_col().gap_2().p_3().rounded(p.radius).bg(p.surface).shadow(p.shadow.clone()).text_color(p.text)
            .child(div().flex().items_center().gap_2().child(div().flex_1().child(r.goal.clone())).child(div().text_sm().text_color(if r.is_waiting() { p.accent } else { p.muted }).child(format!("{} {}", r.status, if total > 0 { format!("({done}/{total})") } else { String::new() }))));
        for s in &r.steps {
            col = col.child(div().flex().gap_2().text_sm().child(div().w(px(18.)).text_color(if s.state == StepState::Failed { p.danger } else { p.accent }).child(state_mark(s.state))).child(div().flex_1().child(format!("{} · {}{}", s.title, s.assignee, if s.note.is_empty() { String::new() } else { format!(" — {}", s.note.chars().take(160).collect::<String>()) }))));
        }
        for q in r.questions.clone() {
            let (run, q1, q2) = (r.id.clone(), q.clone(), q.clone());
            let (run2, label_yes, label_no, text): (String, &str, &str, String) = match &q {
                Question::Plan { goal, .. } => (r.id.clone(), "Approve the plan", "Reject", format!("Approve this plan for: {goal}?")),
                Question::CheckIn { milestone, summaries, .. } => (r.id.clone(), "Carry on", "Stop here", format!("Milestone reached: {milestone}. {}", summaries.join(" "))),
                Question::Permission { tool, what, worker, .. } => (r.id.clone(), "Allow", "Deny", format!("{worker} wants to use {tool}: {what}")),
            };
            let id = q.id().to_string();
            let yes = self.button(format!("q-yes-{id}"), label_yes, Kind::Primary, cx, move |a, _, cx| a.answer_run(&run, &q1, true, cx));
            let no = self.button(format!("q-no-{id}"), label_no, Kind::Normal, cx, move |a, _, cx| a.answer_run(&run2, &q2, false, cx));
            col = col.child(div().flex().flex_col().gap_2().p_2().rounded(p.inner).border_1().border_color(p.accent).child(div().child(text)).child(div().flex().gap_2().child(yes).child(no)));
        }
        match &r.outcome {
            Some(Outcome::Done { result, parked }) => col = col.child(TextView::markdown(SharedString::from(format!("res-{}", r.id)), result.clone())).when(*parked > 0, |d| d.child(self.note(format!("{parked} step(s) were parked and need you.")))),
            Some(Outcome::Failed(t)) | Some(Outcome::Rejected(t)) => col = col.child(div().text_color(p.danger).child(t.clone())),
            _ => {}
        }
        if !r.finished {
            let id = r.id.clone();
            col = col.child(div().flex().child(self.button(format!("run-stop-{}", r.id), "Stop", Kind::Danger, cx, move |a, _, cx| {
                a.backend.send(lh_gui::api::Cmd::StopRun { run: id.clone() });
                cx.notify();
            })));
        } else if let Some(Outcome::Done { result, .. }) = &r.outcome {
            let c = Content::detect(&r.goal, result);
            col = col.child(div().flex().child(self.button(format!("run-open-{}", r.id), "Open result in panel", Kind::Ghost, cx, move |a, _, cx| a.open_content(c.clone(), cx))));
        }
        col.into_any_element()
    }
}

/// The native viewer for a piece of content. Used in the panel and in a pop-out window.
pub fn viewer(item: &Content, scope: &str, p: &crate::style::Pal) -> AnyElement {
    let rtl = lh_gui::rtl::paragraph_is_rtl(&item.body);
    match item.kind {
        ContentKind::Markdown => div().text_color(p.text).when(rtl, |d| d.text_right()).child(TextView::markdown(SharedString::from(format!("view-{scope}-{}", item.title)), item.body.clone())).into_any_element(),
        ContentKind::Code => {
            let lines: Vec<AnyElement> = item.body.lines().enumerate().map(|(i, l)| div().flex().gap_3().child(div().w(px(36.)).flex().justify_end().text_color(p.muted).child((i + 1).to_string())).child(div().child(if l.is_empty() { " ".to_string() } else { l.to_string() })).into_any_element()).collect();
            div().id(SharedString::from(format!("code-{scope}"))).role(Role::Document).aria_label(item.title.clone()).p_2().rounded(p.inner).bg(p.sunken).text_color(p.text).font_family("DejaVu Sans Mono").text_sm().overflow_x_scroll().children(lines).into_any_element()
        }
        ContentKind::Image => div().child(img(std::path::PathBuf::from(item.body.clone())).max_w_full().object_fit(ObjectFit::Contain)).into_any_element(),
        ContentKind::Text => div().text_color(p.text).when(rtl, |d| d.text_right()).child(item.body.clone()).into_any_element(),
    }
}

/// A piece of content in a window of its own.
pub struct PopOut {
    pub item: Content,
    pub pal: crate::style::Pal,
}

impl Render for PopOut {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.pal.clone();
        p.paint_background(div().size_full().p_4().text_color(p.text)).child(div().id("popout").size_full().overflow_y_scroll().child(viewer(&self.item, "popout", &p)))
    }
}

impl App {
    /// Open content in its own window; it keeps the look it had when it was opened.
    pub fn pop_out(&mut self, item: Content, cx: &mut Context<Self>) {
        let pal = self.pal.clone();
        let title = item.title.clone();
        let opts = WindowOptions { titlebar: Some(TitlebarOptions { title: Some(title.into()), ..Default::default() }), window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, size(px(760.), px(640.)), cx))), app_id: Some("lastharness-gui".into()), ..Default::default() };
        let _ = cx.open_window(opts, move |window, cx| {
            let view = cx.new(|_| PopOut { item, pal });
            cx.new(|cx| gpui_component::Root::new(view, window, cx))
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // `gpui::*` brings its own `test` attribute into scope; the standard one is wanted here
    use core::prelude::v1::test;

    #[test]
    fn content_gets_the_viewer_that_suits_it() {
        assert_eq!(Content::detect("t", "plain words\nmore words").kind, ContentKind::Text);
        assert_eq!(Content::detect("t", "# Title\n\ntext").kind, ContentKind::Markdown);
        assert_eq!(Content::detect("t", "- one\n- two").kind, ContentKind::Markdown);
        assert_eq!(Content::detect("t", "| a | b |\n|---|---|\n| 1 | 2 |").kind, ContentKind::Markdown, "tables go to the markdown viewer");
        let c = Content::detect("t", "```rust\nfn main() {}\n```");
        assert_eq!(c.kind, ContentKind::Code);
        assert_eq!(c.body, "fn main() {}");
        assert_eq!(Content::detect("t", "intro\n```x\ncode\n```\noutro").kind, ContentKind::Markdown, "text around code is markdown");
        assert_eq!(Content::detect("t", "/no/such/file.png").kind, ContentKind::Text, "an image path that does not exist is just text");
    }

    #[test]
    fn an_existing_image_file_opens_in_the_image_viewer() {
        let p = std::env::temp_dir().join(format!("lh-gui-img-{}.png", std::process::id()));
        std::fs::write(&p, b"x").unwrap();
        let c = Content::detect("pic", &p.to_string_lossy());
        assert_eq!(c.kind, ContentKind::Image);
        let _ = std::fs::remove_file(&p);
    }
}
