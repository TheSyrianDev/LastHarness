//! The right panel (departments, memory, settings, opened content) and a department's own page.

use gpui::prelude::*;
use gpui::*;
use gpui_component::input::{Input, InputEvent, InputState, Textarea};
use gpui_component::text::TextView;

use crate::app::App;
use lh_gui::chrome::{panel_label, DOCK_TABS};
use lh_gui::layout::Panel;
use lh_gui::runs::{Outcome, Question, Run, StepState};
use lh_gui::settings;
use crate::style::PanelExt;

/// The brightness knob's size at rest, in pixels.
const GLASS_KNOB: f32 = 18.;
use crate::ui::Kind;
use lh_gui::model::Block;

pub use lh_gui::content::{Content, ContentKind};

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
    /// How much each tab of the side panel holds for the chat that is open. A tab with nothing is not shown.
    pub fn dock_counts(&self) -> Vec<(Panel, usize)> {
        let chat = self.chats.current();
        let tools = chat.map(|c| c.blocks.iter().filter(|b| matches!(b, Block::Tool { .. })).count()).unwrap_or(0);
        let runs = self.panels.runs.values().filter(|r| !r.finished).count();
        let recalled = chat.map(|c| c.blocks.iter().filter(|b| matches!(b, Block::Tool { name, .. } if name == "recall" || name == "remember")).count()).unwrap_or(0);
        let branches = self.branches().map(|(n, _)| n).filter(|n| *n > 1).unwrap_or(0);
        vec![(Panel::Content, self.content.len()), (Panel::Work, tools + runs), (Panel::Memory, recalled + self.panels.proposals.len()), (Panel::Tree, branches)]
    }

    /// The side panel: about the chat that is open. One row of tabs with counts, then the size, pop-out and
    /// close icons; the tab's body under it.
    pub fn right_panel(&mut self, width: f32, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        use crate::ui::icons;
        let p = self.pal.clone();
        let rtl = self.rtl;
        let counts = self.dock_counts();
        let mut panel = self.layout().panel;
        if !DOCK_TABS.contains(&panel) {
            panel = Panel::Content;
        }
        let title = panel_label(panel, rtl);
        let expanded = self.layout().right_expanded;
        let widen = self.icon_button("panel-widen", icons::widen(), if expanded { if rtl { "تضييق" } else { "Narrow" } } else if rtl { "توسيع" } else { "Widen" }, cx, |a, _, cx| {
            a.layout_mut().cycle_width();
            a.persist();
            cx.notify();
        });
        let popout = (panel == Panel::Content && !self.content.is_empty()).then(|| {
            let item = self.content[self.content_index.min(self.content.len() - 1)].clone();
            self.icon_button("panel-popout", icons::popout(), if rtl { "فصل في نافذة" } else { "Pop out" }, cx, move |a, _, cx| a.pop_out(item.clone(), cx))
        });
        let hide = self.icon_button("panel-hide", icons::close(), if rtl { "إخفاء" } else { "Close" }, cx, |a, _, cx| {
            a.layout_mut().hide_right();
            a.persist();
            cx.notify();
        });
        let body = match panel {
            Panel::Memory => self.memory_used_body(cx),
            Panel::Content => self.content_body(window, cx),
            Panel::Tree => self.tree_body(cx),
            _ => self.work_body(cx),
        };
        let lift = BoxShadow { color: hsla(0., 0., 0., if p.dark { 0.3 } else { 0.08 }), offset: point(px(0.), px(1.)), blur_radius: px(2.), spread_radius: px(0.), inset: false };
        let tabs: Vec<AnyElement> = counts
            .iter()
            .filter(|(t, n)| *n > 0 || *t == panel)
            .map(|(t, n)| {
                let (t, n, on) = (*t, *n, *t == panel);
                div().id(SharedString::from(format!("dock-{}", panel_label(t, false)))).role(Role::Tab).aria_label(panel_label(t, rtl)).aria_selected(on)
                    .flex().when(rtl, |d| d.flex_row_reverse()).items_center().gap_1p5().px_2p5().py_1().rounded(p.inner).cursor_pointer().text_sm()
                    .text_color(if on { p.text } else { p.muted })
                    .when(on, |d| d.bg(p.surface).shadow(vec![lift.clone()]).font_weight(gpui::FontWeight::MEDIUM))
                    .hover(|s| s.text_color(p.text))
                    .on_click(cx.listener(move |a, _, _, cx| a.show_in_dock(t, cx)))
                    .child(panel_label(t, rtl))
                    .when(n > 0, |d| d.child(div().text_xs().text_color(p.muted).child(n.to_string())))
                    .into_any_element()
            })
            .collect();
        let tab_row = div().id("dock-tabs").role(Role::TabList).aria_label(if rtl { "أقسام اللوحة" } else { "Side panel sections" })
            .flex().when(rtl, |d| d.flex_row_reverse()).items_center().gap_0p5().p_0p5().rounded(p.inner).bg(p.sunken.opacity(0.6)).min_w_0().overflow_hidden().children(tabs);
        let head = div().flex().when(rtl, |d| d.flex_row_reverse()).items_center().gap_1()
            .child(div().flex_1().min_w_0().flex().when(rtl, |d| d.justify_end()).child(tab_row))
            .child(widen).children(popout).child(hide);
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
            .side_panel(&p)
            .on_mouse_down(MouseButton::Left, cx.listener(|a, _, _, cx| a.focus_on(crate::app::Focus::Side, cx)))
            .child(head)
            .child(div().id("panel-body").flex_1().overflow_y_scroll().flex().flex_col().gap_2().child(div().id(SharedString::from(format!("panel-tab-{}", panel_label(panel, false)))).flex().flex_col().gap_2().child(body).with_animation(SharedString::from(format!("panel-in-{}", panel_label(panel, false))), Animation::new(lh_gui::motion::press(p.anim).max(std::time::Duration::from_millis(1))).with_easing(lh_gui::motion::ease_out), |e, t| e.opacity(t))))
            .into_any_element()
    }

    /// A run a chat started shows only with that chat; one started elsewhere (the Departments page) shows with all.
    fn run_is_for_this_chat(&self, run: &str) -> bool {
        lh_gui::runs::shown_with(self.panels.run_sessions.get(run).map(String::as_str), self.chats.current().and_then(|c| c.session.as_deref()))
    }

    /// Work: department runs going now, as a board (step count and who waits), then this chat's tool steps.
    fn work_body(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let rtl = self.rtl;
        let mut col = div().flex().flex_col().gap_1();
        let runs: Vec<Run> = self.panels.runs.values().filter(|r| !r.finished && self.run_is_for_this_chat(&r.id)).cloned().collect();
        if !runs.is_empty() {
            col = col.child(div().text_xs().font_weight(gpui::FontWeight::MEDIUM).text_color(p.muted).child(if rtl { "أقسام تعمل الآن" } else { "Running now" }));
            for r in runs {
                let (done, all) = r.progress();
                let (state, color) = if r.is_waiting() { (if rtl { "بانتظارك" } else { "Asks you" }, p.danger) } else { (if rtl { "يعمل" } else { "Working" }, p.accent) };
                let name = r.dept.clone();
                col = col.child(
                    div().id(SharedString::from(format!("work-run-{}", r.id))).flex().when(rtl, |d| d.flex_row_reverse()).items_center().gap_2().px_2().py_1p5().rounded(p.inner).cursor_pointer().hover(|s| s.bg(p.button))
                        .on_click(cx.listener(move |a, _, _, cx| a.open_department(&name, cx)))
                        .child(div().flex_1().min_w_0().flex().flex_col()
                            .child(div().truncate().text_sm().text_color(p.text).child(r.goal.clone()))
                            .child(div().text_xs().text_color(p.muted).child(format!("{} · {done}/{all}", r.dept))))
                        .child(div().text_xs().font_weight(gpui::FontWeight::MEDIUM).text_color(color).child(state)),
                );
            }
        }
        let key = self.chats.current().map(|c| c.key).unwrap_or(0);
        let steps: Vec<(usize, String, String, bool)> = self.chats.current().map(|c| c.blocks.iter().enumerate().filter_map(|(bi, b)| match b {
            Block::Tool { name, arguments, result, .. } => Some((bi, lh_gui::model::step_words(name, arguments), lh_gui::model::one_line(arguments), result.is_some())),
            _ => None,
        }).collect()).unwrap_or_default();
        if !steps.is_empty() {
            col = col.child(div().pt_2().text_xs().font_weight(gpui::FontWeight::MEDIUM).text_color(p.muted).child(if rtl { "خطوات هذه المحادثة" } else { "Steps in this chat" }));
            for (bi, words, args, done) in steps.into_iter().rev() {
                let open = self.open_steps.contains(&(key, bi));
                let mut row = div().id(SharedString::from(format!("work-step-{bi}"))).role(Role::Button).aria_label(SharedString::from(words.clone()))
                    .flex().flex_col().gap_1().px_2().py_1().rounded(p.inner).cursor_pointer().hover(|s| s.bg(p.button))
                    .on_click(cx.listener(move |a, _, _, cx| {
                        if !a.open_steps.remove(&(key, bi)) { a.open_steps.insert((key, bi)); }
                        cx.notify();
                    }))
                    .child(div().flex().when(rtl, |d| d.flex_row_reverse()).items_center().gap_2()
                        .child(div().size(px(8.)).flex_none().rounded_full().bg(if done { p.muted.opacity(0.5) } else { p.accent }))
                        .child(div().flex_1().min_w_0().truncate().text_sm().text_color(p.text).child(words)));
                if open {
                    row = row.child(div().pl_4().text_xs().text_color(p.muted).font_family(p.mono.clone()).child(args));
                }
                col = col.child(row);
            }
        }
        if self.panels.runs.values().all(|r| r.finished || !self.run_is_for_this_chat(&r.id)) && self.chats.current().map_or(true, |c| !c.blocks.iter().any(|b| matches!(b, Block::Tool { .. }))) {
            col = col.child(self.note(if rtl { "لا شيء يجري لهذه المحادثة. خطوات الأدوات وأعمال الأقسام تظهر هنا." } else { "Nothing is being done for this chat. Tool steps and department work show up here." }));
        }
        col.into_any_element()
    }

    /// Memory used: what this chat looked up or saved, then suggestions waiting for a yes.
    fn memory_used_body(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let rtl = self.rtl;
        let mut col = div().flex().flex_col().gap_1();
        let used: Vec<(String, String)> = self.chats.current().map(|c| c.blocks.iter().filter_map(|b| match b {
            Block::Tool { name, arguments, result, .. } if name == "recall" || name == "remember" => Some((name.clone(), result.clone().unwrap_or_else(|| lh_gui::model::one_line(arguments)))),
            _ => None,
        }).collect()).unwrap_or_default();
        for (i, (name, text)) in used.into_iter().enumerate() {
            let what = if name == "remember" { if rtl { "حُفظ" } else { "Saved" } } else if rtl { "استُرجع" } else { "Looked up" };
            col = col.child(div().id(SharedString::from(format!("used-{i}"))).flex().flex_col().gap_0p5().p_2().rounded(p.inner).bg(p.surface)
                .child(div().text_xs().text_color(p.muted).child(what))
                .child(div().text_sm().text_color(p.text).child(lh_gui::model::truncate_title(&lh_gui::model::one_line(&text), 220))));
        }
        if !self.panels.proposals.is_empty() {
            col = col.child(div().pt_2().text_xs().font_weight(gpui::FontWeight::MEDIUM).text_color(p.muted).child(if rtl { "تذكّر هذا؟" } else { "Remember this?" }));
            col = col.child(self.memory_body(cx));
        } else if col_is_empty(&self.chats, rtl) {
            col = col.child(self.note(if rtl { "لم تستخدم هذه المحادثة الذاكرة بعد." } else { "This chat has not used memory yet. What it looks up or saves shows here." }));
            col = col.child(self.button("open-memory-page", if rtl { "كل الذاكرة" } else { "See all memory" }, Kind::Ghost, cx, |a, _, cx| a.open_page(lh_gui::layout::Page::Memory, cx)));
        }
        col.into_any_element()
    }

    /// The Departments or Memory page in the middle: the same lists as before, with room to read.
    pub fn list_page(&mut self, departments: bool, cx: &mut Context<Self>) -> AnyElement {
        let body = if departments { self.departments_body(cx) } else { self.memory_body(cx) };
        let back = self.button("page-back", if self.rtl { "رجوع إلى المحادثة" } else { "Back to chat" }, Kind::Ghost, cx, |a, _, cx| {
            a.layout_mut().back_to_chat();
            cx.notify();
        });
        div().id("list-page").flex_1().h_full().overflow_y_scroll().flex().flex_col().items_center().p_3()
            .child(div().w_full().max_w(px(760.)).flex().flex_col().gap_3().child(div().flex().when(self.rtl, |d| d.justify_end()).child(back)).child(body))
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
            col = col.child(self.note("No departments yet. A department is a small team with a lead that the assistant can hand bigger jobs to."));
            // in its own row so it keeps its own width instead of stretching across the page
            col = col.child(div().flex().child(self.button("make-department", "Make a department", Kind::Primary, cx, |a, w, cx| {
                if !a.settings_open {
                    a.toggle_settings(w, cx);
                }
                a.set_settings_page(lh_gui::setting_names::Category::Departments, w, cx);
            })));
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
        // suggestions come first: they are waiting for an answer
        for pr in self.panels.proposals.clone() {
            let (a, b, id) = (pr.id, pr.id, pr.id);
            let yes = self.button(format!("prop-yes-{id}"), "Remember this", Kind::Primary, cx, move |app, _, cx| app.memory_action(format!("/v1/memory/proposals/{a}/accept"), Some(serde_json::json!({})), cx));
            let no = self.button(format!("prop-no-{id}"), "Not this", Kind::Normal, cx, move |app, _, cx| app.memory_action(format!("/v1/memory/proposals/{b}/reject"), Some(serde_json::json!({})), cx));
            col = col.child(
                div().flex().flex_col().gap_1().p_2().rounded(p.inner).border_1().border_color(p.accent).text_color(p.text)
                    .child(div().text_xs().text_color(p.accent).child(format!("Suggestion ({})", pr.kind)))
                    .child(div().child(pr.title.clone()))
                    .child(div().text_sm().text_color(p.muted).child(pr.text.clone()))
                    .when(!pr.evidence.is_empty(), |d| d.child(div().text_xs().text_color(p.muted).child(format!("You said: {}", pr.evidence))))
                    .child(div().flex().gap_2().child(yes).child(no)),
            );
        }
        if self.panels.pages.is_empty() && self.panels.proposals.is_empty() {
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
            let (pid1, pid2, pid3) = (pg.id.clone(), pg.id.clone(), pg.id.clone());
            let pinned = pg.pinned;
            let sensitive = pg.privacy == "sensitive";
            let pin = self.button(format!("mem-pin-{i}"), if pinned { "Unpin" } else { "Pin" }, Kind::Ghost, cx, move |a, _, cx| a.memory_action(format!("/v1/memory/pages/{pid1}/pin"), Some(serde_json::json!({"pinned": !pinned})), cx));
            let priv_btn = self.button(format!("mem-priv-{i}"), if sensitive { "Make normal" } else { "Make sensitive" }, Kind::Ghost, cx, move |a, _, cx| a.memory_action(format!("/v1/memory/pages/{pid2}/privacy"), Some(serde_json::json!({"privacy": if sensitive { "normal" } else { "sensitive" }})), cx));
            let archive = self.button(format!("mem-arch-{i}"), "Archive", Kind::Ghost, cx, move |a, _, cx| a.memory_action(format!("/v1/memory/pages/{pid3}/archive"), None, cx));
            col = col.child(
                div().flex().flex_col().gap_1().p_2().rounded(p.inner).bg(p.sunken).text_color(p.text)
                    .child(div().flex().items_center().gap_2().child(div().flex_1().child(format!("{}{}", if pg.pinned { "📌 " } else { "" }, pg.title))).child(toggle))
                    .child(div().text_xs().text_color(p.muted).child(format!("{} · {}{}", pg.layer, pg.privacy, if pg.gist.is_empty() { String::new() } else { format!(" · {}", pg.gist) })))
                    .when(open, |d| d.child(TextView::markdown(SharedString::from(format!("page{i}")), pg.body.clone())).child(div().flex().flex_wrap().gap_1().child(show).child(pin).child(priv_btn).child(archive))),
            );
        }
        col.into_any_element()
    }

    /// The Appearance page: the app's own look, mode, direction and widths. It works when the core is away.
    fn appearance_block(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let rtl = self.rtl;
        // a look is a family with a light and a dark variant; the mode below picks the variant for all of them
        let mut looks = div().flex().flex_wrap().items_center().gap_1().child(div().text_sm().text_color(p.muted).child(if rtl { "المظهر" } else { "Look" }));
        let current = self.family();
        for f in self.themes.families() {
            let one_mode = !self.themes.has_both(&f);
            let (name, label) = (f.clone(), if one_mode { format!("{f} ({})", if rtl { "نمط واحد" } else { "one mode only" }) } else { f.clone() });
            looks = looks.child(self.button(format!("look-{f}"), label, if f == current { Kind::Primary } else { Kind::Normal }, cx, move |a, w, cx| a.set_family(&name, w, cx)));
        }
        // light and dark are a switch; following the desktop is its own switch beneath it
        let reports = self.system_dark.is_some() || lh_gui::appearance::system_dark().is_some();
        let sw = lh_gui::appearance::switches(self.prefs.mode, self.system_dark, reports);
        let dark_note = if sw.dark_enabled { None } else { Some(if rtl { if sw.dark_on { "يتبع سطح المكتب: داكن الآن" } else { "يتبع سطح المكتب: فاتح الآن" } } else if sw.dark_on { "Following the desktop: dark now" } else { "Following the desktop: light now" }) };
        let dark_switch = self.glass_switch("dark-mode-switch", sw.dark_on, sw.dark_enabled, if rtl { "الوضع الداكن" } else { "Dark mode" }, true, cx, |a, on, w, cx| a.set_appearance(lh_gui::appearance::after_dark_switch(on), w, cx));
        let match_switch = self.glass_switch("match-desktop-switch", sw.match_on, sw.match_enabled, if rtl { "مطابقة سطح المكتب" } else { "Match the desktop" }, true, cx, |a, on, w, cx| {
            let dark = a.dark();
            a.set_appearance(lh_gui::appearance::after_match_switch(on, dark), w, cx)
        });
        let modes = div().flex().flex_col().gap_1().id("mode-switches").role(Role::Group).aria_label(if rtl { "الوضع" } else { "Light or dark" })
            .child(div().flex().when(rtl, |d| d.flex_row_reverse()).items_center().gap_3().child(dark_switch).children(dark_note.map(|n| div().text_xs().text_color(p.muted).child(n))))
            .child(div().flex().when(rtl, |d| d.flex_row_reverse()).items_center().gap_3().child(match_switch).children((!reports && !sw.match_on).then(|| div().text_xs().text_color(p.muted).child(if rtl { "سطح المكتب لا يخبرنا بوضعه." } else { "This desktop does not say whether it is light or dark." }))));
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
        let narrower = self.button("left-narrower", "Chat list narrower", Kind::Ghost, cx, |a, _, cx| {
            let w = a.layout().left_width - 30.;
            a.layout_mut().resize_left(w);
            a.persist();
            cx.notify();
        });
        let wider = self.button("left-wider", "Chat list wider", Kind::Ghost, cx, |a, _, cx| {
            let w = a.layout().left_width + 30.;
            a.layout_mut().resize_left(w);
            a.persist();
            cx.notify();
        });
        let widths = self.width_controls(cx);
        let fonts = div().flex().flex_col().gap_1()
            .child(self.font_picker_row(crate::app::FontSlot::Latin, cx))
            .child(self.font_picker_row(crate::app::FontSlot::Arabic, cx))
            .child(self.added_fonts_block(cx));
        let pages_switch = self.glass_switch("pages-inside-switch", !self.prefs.pages_off, true, if rtl { "عرض صفحات الويب داخل التطبيق" } else { "Show web pages inside the app" }, true, cx, |a, on, _, cx| {
            a.prefs.pages_off = !on;
            a.persist();
            cx.notify();
        });
        let pages_note = div().text_xs().text_color(p.muted).child(if rtl { "لا تستطيع أي صفحة تحميل شيء من الإنترنت أو من هذا الحاسوب. التطبيقات الصغيرة تعمل فيها سكربتات بلا اتصال بالشبكة. الصفحات الأكبر من 2 ميغابايت تظهر كبطاقة." } else { "No page can load anything from the internet or from this computer. Small apps run their scripts with no network. Pages over 2 MB show as a card instead." });
        let name_row = div().flex().flex_col().gap_1()
            .child(div().child(if rtl { "اسمك في شاشة المحادثة الجديدة" } else { "Your name on the new-chat screen" }))
            .child(div().w(px(220.)).child(Input::new(&self.name_input).aria_label("Your name")))
            .child(div().text_xs().text_color(p.muted).child(if rtl { "إن تركته فارغًا يُستخدم اسمك من النظام." } else { "Leave it empty to use your name from the system." }));
        div().flex().flex_col().gap_2().p_2().rounded(p.inner).bg(p.sunken).text_color(p.text)
                .id("appearance").role(Role::Group).aria_label("Appearance")
                .child(modes)
                .child(looks)
                .children((current == "Glass").then(|| self.glass_colours_block(window, cx)))
                .children((current == lh_gui::omarchy::TRANSPARENT && self.themes.needs_window_rule()).then(|| {
                    div().flex().flex_col().gap_1().text_xs().text_color(p.muted)
                        .child(format!("No Hyprland window rule makes this app see-through yet, so this look is opaque. Add this line to your Hyprland config (window class {}) and it follows within a few seconds:", lh_gui::omarchy::APP_CLASS))
                        .child(div().font_family(p.mono.clone()).text_color(p.text).child(lh_gui::omarchy::RULE_HINT))
                }))
                .child(widths)
                .child(fonts)
                .child(name_row)
                .child(div().flex().flex_col().gap_1().child(pages_switch).child(pages_note))
                .child(div().flex().flex_wrap().gap_1().child(dir_btn).child(narrower).child(wider))
                .child(div().text_xs().text_color(p.muted).child(format!("Your own looks go in {} as .toml files; changes show at once.", self.cfg.themes.display())))
                .children(self.themes.problems.iter().map(|e| div().text_xs().text_color(p.danger).child(format!("Theme file problem: {e}"))))
            .children(self.themes.notice.clone().map(|n| div().text_xs().text_color(p.muted).child(n)))
            .children(self.look_notice.clone().map(|n| div().text_xs().text_color(p.muted).child(n)))
            .into_any_element()
    }

    /// The Glass look's colours: up to three glass balls on a colour wheel (hue around, saturation out from the
    /// middle), a hex box for each, and how strongly they show.
    fn glass_colours_block(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        use lh_gui::wheel;
        const SIZE: f32 = 176.;
        const BALL: f32 = 22.;
        let p = self.pal.clone();
        let rtl = self.rtl;
        let picks = self.prefs.glass_colours.clone();
        let disc = self.glass_disc.get_or_insert_with(|| {
            // drawn at twice the size so it stays sharp on a scaled screen; red and blue swapped for the toolkit
            let n = (SIZE * 2.) as u32;
            let mut px = wheel::disc_rgba(n);
            for c in px.as_chunks_mut::<4>().0 {
                c.swap(0, 2);
            }
            std::sync::Arc::new(RenderImage::new(vec![image::Frame::new(image::RgbaImage::from_raw(n, n, px).expect("wheel size"))]))
        }).clone();
        let dot = |hex: &str| { let (r, g, b) = wheel::parse_hex(hex).unwrap_or((128, 128, 128)); gpui::rgb(((r as u32) << 16) | ((g as u32) << 8) | b as u32) };
        let centre = self.glass_wheel.clone();
        let mark = gpui::canvas(|_, _, _| (), move |b, _, _, _| centre.set((f32::from(b.origin.x) + SIZE / 2., f32::from(b.origin.y) + SIZE / 2.)))
            .absolute().top_0().left_0().size_full();
        let mut ring = div().id("glass-wheel").role(Role::Group).aria_label(if rtl { "عجلة ألوان الزجاج" } else { "Glass colour wheel" })
            .relative().w(px(SIZE)).h(px(SIZE)).flex_none().cursor_pointer()
            .child(img(disc).absolute().top_0().left_0().size_full())
            .child(mark)
            .on_mouse_down(MouseButton::Left, cx.listener(|a, ev: &MouseDownEvent, w, cx| {
                let (dx, dy) = a.glass_offset(ev.position);
                let r = SIZE / 2.;
                if dx * dx + dy * dy > (r + BALL / 2.).powi(2) {
                    return;
                }
                // the ball under the pointer, else a new ball, else the nearest one comes to the pointer
                let near = a.prefs.glass_colours.iter().enumerate()
                    .map(|(i, c)| { let (x, y) = a.glass_placed.position(c, r); (i, (x - dx).powi(2) + (y - dy).powi(2)) })
                    .min_by(|x, y| x.1.total_cmp(&y.1));
                let i = match near {
                    Some((i, d)) if d <= (BALL / 2. + 2.).powi(2) || a.prefs.glass_colours.len() >= 3 => i,
                    _ => { let hex = a.glass_placed.place(dx, dy, r, 1.); a.prefs.glass_colours.push(hex); a.prefs.glass_colours.len() - 1 }
                };
                a.glass_drag = Some(i);
                a.glass_sel = i;
                a.glass_move(i, dx, dy, w, cx);
            }))
            .on_mouse_move(cx.listener(|a, ev: &MouseMoveEvent, w, cx| {
                if let Some(i) = a.glass_drag {
                    let (dx, dy) = a.glass_offset(ev.position);
                    a.glass_move(i, dx, dy, w, cx);
                }
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(|a, _, w, cx| a.glass_release(w, cx)))
            .on_mouse_up_out(MouseButton::Left, cx.listener(|a, _, w, cx| a.glass_release(w, cx)));
        let sel = self.glass_sel.min(picks.len().saturating_sub(1));
        self.glass_sel = sel;
        for (i, c) in picks.iter().enumerate() {
            let (x, y) = self.glass_placed.position(c, SIZE / 2.);
            let held = self.glass_drag == Some(i);
            let grow = self.swell(&format!("glass-ball-{i}"), held, 4., Some(window));
            let d = BALL + grow;
            let ball = div().absolute().left(px(SIZE / 2. + x - d / 2.)).top(px(SIZE / 2. + y - d / 2.)).w(px(d)).h(px(d)).rounded_full().bg(dot(c));
            let ball = self.glass(ball, &format!("glass-ball-{i}"), held, d / 2.);
            // the ball the brightness slider follows wears a ring in the accent colour
            ring = ring.child(if i == sel {
                let o = d / 2. + 4.;
                div().absolute().left(px(SIZE / 2. + x - o)).top(px(SIZE / 2. + y - o)).w(px(o * 2.)).h(px(o * 2.)).rounded_full().border_2().border_color(p.accent).child(ball.left(px(2.)).top(px(2.)))
            } else {
                ball
            });
        }
        // brightness for the selected ball: bright at the top, dark at the bottom, a glass knob like the other sliders
        let bar = picks.get(sel).map(|c| {
            let (h, s) = self.glass_placed.hs(c);
            let full = { let (r, g, b) = wheel::hsv_to_rgb(h, s, 1.); wheel::to_hex(r, g, b) };
            let v = wheel::value_of(c);
            let held = self.glass_vdrag;
            let swell = self.swell("glass-value", held, 4., Some(window));
            let k = GLASS_KNOB + swell;
            // the knob's edges stay inside the bar (as the other sliders), and a press swells it about its middle
            let edge = lh_gui::slide::edge_at(1. - v, SIZE, GLASS_KNOB);
            let top = self.glass_bar.clone();
            let mark = gpui::canvas(|_, _, _| (), move |b, _, _, _| top.set(f32::from(b.origin.y))).absolute().top_0().left_0().size_full();
            let knob = div().absolute().left(px((14. - k) / 2.)).top(px(edge - swell / 2.)).w(px(k)).h(px(k)).rounded_full().bg(dot(c));
            let knob = self.glass(knob, "glass-value", held, k / 2.);
            div().id("glass-value").role(Role::Slider).aria_label(if rtl { "السطوع" } else { "Brightness" })
                .relative().w(px(14.)).h(px(SIZE)).flex_none().rounded_full().cursor_pointer().border_1().border_color(p.border)
                .bg(linear_gradient(180., linear_color_stop(dot(&full), 0.), linear_color_stop(gpui::black(), 1.)))
                .child(mark)
                .child(knob)
                .on_mouse_down(MouseButton::Left, cx.listener(|a, ev: &MouseDownEvent, w, cx| { a.glass_vdrag = true; a.glass_value(ev.position.y, w, cx); }))
                .on_mouse_move(cx.listener(|a, ev: &MouseMoveEvent, w, cx| if a.glass_vdrag { a.glass_value(ev.position.y, w, cx) }))
                .on_mouse_up(MouseButton::Left, cx.listener(|a, _, w, cx| a.glass_release(w, cx)))
                .on_mouse_up_out(MouseButton::Left, cx.listener(|a, _, w, cx| a.glass_release(w, cx)))
        });
        // one hex box per ball: typing a good colour moves its ball at once, a bad one goes back on Enter or leaving
        while self.glass_hex.len() < picks.len() {
            let i = self.glass_hex.len();
            let st = cx.new(|cx| InputState::new(window, cx).placeholder("#RRGGBB"));
            cx.subscribe_in(&st, window, move |a, st, ev: &InputEvent, w, cx| {
                let typed = st.read(cx).value().to_string();
                match ev {
                    InputEvent::Change => {
                        if let (Some((r, g, b)), true) = (wheel::parse_hex(&typed), i < a.prefs.glass_colours.len()) {
                            let hex = wheel::to_hex(r, g, b);
                            if a.prefs.glass_colours[i] != hex {
                                a.prefs.glass_colours[i] = hex;
                                a.reapply_look(w, cx);
                            }
                        }
                    }
                    InputEvent::PressEnter { .. } | InputEvent::Blur => {
                        if let Some(now) = a.prefs.glass_colours.get(i).cloned() {
                            if wheel::parse_hex(&typed).map(|(r, g, b)| wheel::to_hex(r, g, b)) != Some(now.clone()) {
                                st.update(cx, |s, cx| s.set_value(now, w, cx));
                            }
                        }
                    }
                    _ => {}
                }
            })
            .detach();
            self.glass_hex.push(st);
        }
        let mut rows = div().flex().flex_col().gap_2();
        for (i, c) in picks.iter().enumerate() {
            let st = self.glass_hex[i].clone();
            let (typing, shown) = st.read_with(cx, |s, cx| (s.focus_handle(cx).is_focused(window), s.value().to_string()));
            if !typing && shown != *c {
                let v = c.clone();
                st.update(cx, |s, cx| s.set_value(v, window, cx));
            }
            let rm = self.button(format!("glass-remove-{i}"), if rtl { "إزالة" } else { "Remove" }, Kind::Ghost, cx, move |a, w, cx| {
                if i < a.prefs.glass_colours.len() {
                    a.prefs.glass_colours.remove(i);
                    a.reapply_look(w, cx);
                }
            });
            rows = rows.child(
                div().flex().items_center().gap_2()
                    .child(div().id(SharedString::from(format!("glass-pick-{i}"))).w(px(16.)).h(px(16.)).flex_none().rounded_full().bg(dot(c)).border_2().border_color(if i == sel { p.accent } else { p.border }).cursor_pointer()
                        .on_click(cx.listener(move |a, _, _, cx| { a.glass_sel = i; cx.notify(); })))
                    .child(div().w(px(110.)).child(Input::new(&st)))
                    .child(rm),
            );
        }
        if picks.len() < 3 {
            rows = rows.child(self.button("glass-colour-add", if rtl { "إضافة لون" } else { "Add colour" }, Kind::Normal, cx, |a, w, cx| {
                // a new ball starts across the wheel from the last one, so it is easy to see and grab
                let next = match a.prefs.glass_colours.last().and_then(|c| wheel::parse_hex(c)) {
                    Some((r, g, b)) => { let (h, s, v) = wheel::rgb_to_hsv(r, g, b); let (r, g, b) = wheel::hsv_to_rgb(h + 120., s.max(0.6), v.max(0.6)); wheel::to_hex(r, g, b) }
                    None => "#4F8CFF".to_string(),
                };
                a.prefs.glass_colours.push(next);
                a.reapply_look(w, cx);
            }));
        }
        let opacity = self.prefs.glass_opacity.unwrap_or(0.5);
        let mut strength = div().flex().flex_wrap().items_center().gap_1().child(div().text_sm().text_color(p.muted).child(if rtl { "الشدة" } else { "Opacity" }));
        for v in [0.2_f32, 0.4, 0.6, 0.8, 1.0] {
            strength = strength.child(self.button(format!("glass-opacity-{}", (v * 100.) as u32), format!("{}%", (v * 100.) as u32), if (opacity - v).abs() < 0.05 { Kind::Primary } else { Kind::Normal }, cx, move |a, w, cx| {
                a.prefs.glass_opacity = Some(v);
                a.reapply_look(w, cx);
            }));
        }
        let clear = self.button("glass-colours-clear", if rtl { "بلا ألوان" } else { "No colours" }, Kind::Ghost, cx, |a, w, cx| {
            a.prefs.glass_colours.clear();
            a.reapply_look(w, cx);
        });
        div().flex().flex_col().gap_2()
            .child(div().flex().items_center().gap_2().child(div().text_sm().text_color(p.muted).child(if rtl { "ألوان الزجاج (حتى ثلاثة)" } else { "Glass colours (up to three)" })).child(clear))
            .child(div().flex().flex_wrap().items_center().gap_4().child(div().flex().items_center().gap_3().child(ring).children(bar)).child(rows))
            .child(strength)
            .child(div().text_xs().text_color(p.muted).child(if rtl { "تبقى الكلمات واضحة: يخف اللون إن لزم." } else { "Words stay readable: the tint softens itself when it has to." }))
            .into_any_element()
    }

    /// Where the pointer is from the colour wheel's centre.
    fn glass_offset(&self, at: Point<Pixels>) -> (f32, f32) {
        let (cx0, cy0) = self.glass_wheel.get();
        (f32::from(at.x) - cx0, f32::from(at.y) - cy0)
    }

    /// Move ball `i` to the pointer, keeping its brightness. The look follows live; saving waits for the release.
    fn glass_move(&mut self, i: usize, dx: f32, dy: f32, window: &mut Window, cx: &mut Context<Self>) {
        let Some(old) = self.prefs.glass_colours.get(i) else { return };
        let v = lh_gui::wheel::value_of(old).max(lh_gui::wheel::MIN_VALUE);
        let hex = self.glass_placed.place(dx, dy, 88., v);
        if self.prefs.glass_colours[i] != hex {
            self.prefs.glass_colours[i] = hex;
            self.active_theme = self.themes.resolve(&self.prefs.theme, self.dark());
            self.refresh_pal(cx);
            self.apply_window(window);
        }
        cx.notify();
    }

    /// Set the selected ball's brightness from the pointer's height on the slider, live; saved on release.
    fn glass_value(&mut self, y: Pixels, window: &mut Window, cx: &mut Context<Self>) {
        let i = self.glass_sel;
        let Some(old) = self.prefs.glass_colours.get(i) else { return };
        let f = lh_gui::slide::fraction_at(f32::from(y) - self.glass_bar.get(), 176., GLASS_KNOB);
        let old = old.clone();
        let hex = self.glass_placed.with_value(&old, 1. - f);
        if self.prefs.glass_colours[i] != hex {
            self.prefs.glass_colours[i] = hex;
            self.active_theme = self.themes.resolve(&self.prefs.theme, self.dark());
            self.refresh_pal(cx);
            self.apply_window(window);
        }
        cx.notify();
    }

    fn glass_release(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let slid = std::mem::take(&mut self.glass_vdrag);
        if self.glass_drag.take().is_some() || slid {
            self.reapply_look(window, cx);
        }
    }

    /// "Add font…" and the list of fonts added so far, each with Remove.
    fn added_fonts_block(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let rtl = self.rtl;
        let add = self.button("font-add", if rtl { "إضافة خط…" } else { "Add font…" }, Kind::Normal, cx, |a, _, cx| a.add_font_dialog(cx));
        let files = lh_gui::fonts::user_font_dir().map(|d| lh_gui::fonts::user_font_files(&d)).unwrap_or_default();
        let rows: Vec<AnyElement> = files
            .into_iter()
            .enumerate()
            .map(|(i, f)| {
                let name = f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                let rm = self.button(format!("font-remove-{i}"), if rtl { "إزالة" } else { "Remove" }, Kind::Ghost, cx, move |a, _, cx| a.remove_font(&f, cx));
                div().flex().when(rtl, |r| r.flex_row_reverse()).items_center().gap_2().child(div().min_w_0().text_sm().text_color(p.muted).child(name)).child(rm).into_any_element()
            })
            .collect();
        div().flex().flex_col().items_start().gap_1()
            .child(add)
            .children((!self.font_note.is_empty()).then(|| div().text_xs().text_color(p.muted).child(self.font_note.clone())))
            .children(rows)
            .into_any_element()
    }

    /// One font picker: a button with the font in use that opens a short scrolling list of installed fonts.
    fn font_picker_row(&mut self, slot: crate::app::FontSlot, cx: &mut Context<Self>) -> AnyElement {
        use crate::app::FontSlot;
        let p = self.pal.clone();
        let rtl = self.rtl;
        let (title, chosen, tag) = match slot {
            FontSlot::Latin => (if rtl { "الخط اللاتيني" } else { "Latin font" }, self.prefs.latin_font.clone(), "latin"),
            FontSlot::Arabic => (if rtl { "الخط العربي" } else { "Arabic font" }, self.prefs.arabic_font.clone(), "arabic"),
        };
        // the "Default" entry says "Theme default (X)" when the look sets a font that is installed
        let tf = self.themes.themes[self.active_theme].fonts.clone();
        let theme_pick = match slot {
            FontSlot::Latin => lh_gui::fonts::first_installed(&tf.latin, &self.fonts),
            FontSlot::Arabic => lh_gui::fonts::first_installed(&tf.arabic, &self.fonts),
        };
        let default_label = match &theme_pick {
            Some(x) => lh_gui::fonts::theme_default_label(x, rtl),
            None if slot == FontSlot::Latin => format!("{} ({})", if rtl { "الافتراضي" } else { "Default" }, lh_gui::fonts::UI_DEFAULT),
            None => (if rtl { "الافتراضي" } else { "Default" }).to_string(),
        };
        let shown = match slot {
            FontSlot::Latin if chosen.is_empty() => default_label.clone(),
            FontSlot::Arabic if chosen.is_empty() => match &theme_pick {
                Some(_) => default_label.clone(),
                None => lh_gui::fonts::arabic_font("", &[], &self.fonts).unwrap_or_else(|| (if rtl { "افتراضي النظام" } else { "System default" }).to_string()),
            },
            _ => chosen.clone(),
        };
        let open = self.font_picker == Some(slot);
        let toggle = self.button(format!("font-{tag}"), format!("{title}: {shown}"), Kind::Normal, cx, move |a, _, cx| {
            a.font_picker = if a.font_picker == Some(slot) { None } else { Some(slot) };
            cx.notify();
        });
        let mut col = div().flex().flex_col().items_start().gap_1().child(div().min_w(px(140.)).max_w(px(320.)).overflow_hidden().whitespace_nowrap().child(toggle));
        if open {
            let mut names = match slot {
                FontSlot::Arabic => lh_gui::fonts::arabic_first(&self.fonts),
                FontSlot::Latin => self.fonts.clone(),
            };
            names.insert(0, String::new());
            let rows: Vec<AnyElement> = names
                .into_iter()
                .enumerate()
                .map(|(i, n)| {
                    let label = if n.is_empty() { default_label.clone() } else { n.clone() };
                    let kind = if n == chosen { Kind::Primary } else { Kind::Ghost };
                    self.button(format!("font-{tag}-{i}"), label, kind, cx, move |a, _, cx| {
                        a.set_font(slot, &n);
                        cx.notify();
                    })
                })
                .collect();
            // the wheel scrolls only this list while the pointer is over it, never the Settings page behind
            col = col.child(div().on_scroll_wheel(|_, _, cx| cx.stop_propagation()).child(div().id(SharedString::from(format!("font-list-{tag}"))).role(Role::List).aria_label(title).min_w(px(200.)).max_w(px(360.)).max_h(px(220.)).overflow_y_scroll().flex().flex_col().gap_1().p_1().rounded(p.inner).bg(p.sunken).children(rows)));
        }
        col.into_any_element()
    }

    /// The text box of one setting. It is made the first time the card is shown, saves on Enter and when it
    /// loses focus, and follows the core's value while nobody is typing in it.
    fn setting_input(&mut self, s: &crate::app::Setting, window: &mut Window, cx: &mut Context<Self>) -> Entity<InputState> {
        if let Some(st) = self.setting_inputs.get(&s.key).cloned() {
            let (typing, shown) = st.read_with(cx, |i, cx| (i.focus_handle(cx).is_focused(window), i.value().to_string()));
            if !typing && shown != s.value && !self.panels.setting_errors.contains_key(&s.key) {
                let v = s.value.clone();
                st.update(cx, |i, cx| i.set_value(v, window, cx));
            }
            return st;
        }
        let hint = match &s.kind {
            settings::Kind::Number => "A number, or empty",
            settings::Kind::AutoInt => "A number or auto",
            settings::Kind::Int { .. } => "A number",
            _ if settings::is_folder_key(&s.key) => "/path/to/a/folder",
            _ => "Not set",
        };
        let st = cx.new(|cx| InputState::new(window, cx).placeholder(hint).default_value(s.value.clone()));
        let key = s.key.clone();
        cx.subscribe_in(&st, window, move |a, st, ev: &InputEvent, _, cx| {
            if matches!(ev, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                let typed = st.read(cx).value().to_string();
                a.commit_setting_text(&key, &typed, cx);
            }
        })
        .detach();
        self.setting_inputs.insert(s.key.clone(), st.clone());
        st
    }

    /// Send what was typed, if it differs from the saved value and passes the local check.
    pub fn commit_setting_text(&mut self, key: &str, typed: &str, cx: &mut Context<Self>) {
        let Some(s) = self.panels.settings.iter().find(|s| s.key == key) else { return };
        if typed.trim() == s.value.trim() {
            self.panels.setting_errors.remove(key);
            cx.notify();
            return;
        }
        match settings::check_typed(&s.kind, typed) {
            Ok(()) => self.change_setting(key, typed.trim().to_string(), cx),
            Err(e) => {
                self.panels.setting_errors.insert(key.to_string(), e);
                cx.notify();
            }
        }
    }

    /// A button that opens or closes the list of choices for one setting. A long value is cut with "…".
    fn list_toggle(&mut self, key: &str, label: String, cx: &mut Context<Self>) -> AnyElement {
        let k = key.to_string();
        let open = self.setting_list.as_deref() == Some(key);
        let shown = format!("{} {}", lh_gui::model::truncate_title(&label, 30), if open { "▴" } else { "▾" });
        self.button(format!("set-{key}"), shown, Kind::Normal, cx, move |a, _, cx| {
            a.setting_list = if a.setting_list.as_deref() == Some(k.as_str()) { None } else { Some(k.clone()) };
            cx.notify();
        })
    }

    /// One setting as a card: its plain title, one sentence, the control, and the value in words.
    fn setting_card(&mut self, i: usize, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let s = self.panels.settings[i].clone();
        let d = lh_gui::setting_names::describe(&s.key, &s.help);
        let words = lh_gui::setting_names::value_text(&s.key, &s.value);
        let control_kind = settings::control_for(&s.key, &s.kind);
        let rtl = self.rtl;
        let mut below: Vec<AnyElement> = vec![];
        let open = self.setting_list.as_deref() == Some(s.key.as_str());
        let control: AnyElement = match control_kind {
            settings::Control::Switch => {
                let (key, cur, kind) = (s.key.clone(), s.value.clone(), s.kind.clone());
                self.button(format!("set-{}", s.key), words.clone(), Kind::Normal, cx, move |a, _, cx| {
                    if let Some(v) = kind.next(&cur) {
                        a.change_setting(&key, v, cx);
                    }
                })
            }
            settings::Control::Dropdown => {
                let toggle = self.list_toggle(&s.key, words.clone(), cx);
                if open {
                    if let settings::Kind::Choice(options) = &s.kind {
                        let rows: Vec<AnyElement> = options
                            .iter()
                            .enumerate()
                            .map(|(n, o)| {
                                let (key, val) = (s.key.clone(), o.clone());
                                let label = lh_gui::setting_names::value_text(&s.key, o);
                                let kind = if *o == s.value { Kind::Primary } else { Kind::Ghost };
                                self.button(format!("set-{}-{n}", s.key), label, kind, cx, move |a, _, cx| {
                                    a.setting_list = None;
                                    a.change_setting(&key, val.clone(), cx);
                                })
                            })
                            .collect();
                        below.push(self.option_list(&s.key, rows));
                    }
                }
                toggle
            }
            settings::Control::Stepper => {
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
                let input = self.setting_input(&s, window, cx);
                div().flex().items_center().gap_1().child(minus).child(div().w(px(84.)).child(Input::new(&input))).child(plus).into_any_element()
            }
            settings::Control::Field => {
                let input = self.setting_input(&s, window, cx);
                div().w_full().child(Input::new(&input)).into_any_element()
            }
            settings::Control::Folder => {
                let input = self.setting_input(&s, window, cx);
                let key = s.key.clone();
                let choose = self.button(format!("set-{}-choose", s.key), if rtl { "اختيار مجلد" } else { "Choose folder" }, Kind::Normal, cx, move |_, _, cx| {
                    let rx = cx.prompt_for_paths(PathPromptOptions { files: false, directories: true, multiple: false, prompt: Some("Choose".into()) });
                    let key = key.clone();
                    cx.spawn(async move |this: WeakEntity<App>, cx: &mut AsyncApp| {
                        if let Ok(Ok(Some(paths))) = rx.await {
                            if let Some(path) = paths.first() {
                                let path = path.to_string_lossy().to_string();
                                let _ = this.update(cx, |a, cx| a.change_setting(&key, path, cx));
                            }
                        }
                    })
                    .detach();
                });
                div().w_full().flex().when(rtl, |r| r.flex_row_reverse()).items_center().gap_1().child(div().flex_1().min_w_0().child(Input::new(&input))).child(choose).into_any_element()
            }
            settings::Control::ModelPicker => {
                let none = if rtl { "لا شيء" } else { "None" };
                let current = if s.value.trim().is_empty() { none.to_string() } else { self.models.label_of(&s.value) };
                let toggle = self.list_toggle(&s.key, current.clone(), cx);
                if open {
                    let mut names: Vec<(String, String)> = vec![(none.to_string(), String::new())];
                    if !s.value.trim().is_empty() && !self.models.has(&s.value) && !self.models.names().contains(&self.models.id_for(&s.value)) {
                        names.push((current.clone(), s.value.clone()));
                    }
                    names.extend(self.models.names().iter().map(|n| (self.models.label_of(n), n.clone())));
                    let rows: Vec<AnyElement> = names
                        .into_iter()
                        .enumerate()
                        .map(|(n, (label, val))| {
                            let key = s.key.clone();
                            let kind = if label == current { Kind::Primary } else { Kind::Ghost };
                            self.button(format!("set-{}-{n}", s.key), label, kind, cx, move |a, _, cx| {
                                a.setting_list = None;
                                a.change_setting(&key, val.clone(), cx);
                            })
                        })
                        .collect();
                    below.push(self.option_list(&s.key, rows));
                    if self.models.is_empty() {
                        below.push(div().text_xs().text_color(p.muted).child("The model list is empty. Is the model server running?").into_any_element());
                    }
                }
                toggle
            }
            settings::Control::ReadOnly => div().text_color(p.muted).child(words.clone()).into_any_element(),
        };
        let default_words = lh_gui::setting_names::value_text(&s.key, &s.default);
        let tip = format!("Default: {default_words}. Name in the settings file: {}", s.key);
        let reset_tip = tip.clone();
        let aria = format!("{}: {}", d.title, words);
        let wide = control_kind.wide();
        let differs = !settings::is_default(&s.value, &s.default);
        // red only while the value is a risky one; the same setting at a safe value is a plain card
        let risky = d.careful && lh_gui::setting_names::is_risky(&s.key, &s.value);
        let title = div()
            .flex_1()
            .min_w(px(180.))
            .flex()
            .flex_col()
            .child(div().min_w_0().child(d.title.clone()))
            .when(!s.live, |c| c.child(div().text_xs().text_color(p.muted).child(if rtl { "يحتاج إعادة تشغيل" } else { "Needs a restart" })));
        let mut control = Some(control);
        let mut head = div().flex().flex_wrap().when(rtl, |r| r.flex_row_reverse()).items_center().gap_2().child(title);
        if !wide {
            head = head.child(div().flex_none().max_w_full().flex().when(rtl, |r| r.flex_row_reverse()).children(control.take()));
        }
        let mut card = div().flex().flex_col().gap_1().p_2().rounded(p.inner).bg(p.sunken).text_color(p.text).overflow_hidden()
            .when(risky, |c| c.border_1().border_color(p.danger.opacity(0.55)))
            .id(SharedString::from(format!("setting-{}", s.key))).role(Role::Group).aria_label(aria).aria_description(d.about.clone())
            .tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(tip.clone()).build(window, cx))
            .child(head)
            .child(div().min_w_0().text_xs().text_color(p.muted).child(d.about.clone()));
        if wide {
            card = card.children(control.take());
        }
        for b in below {
            card = card.child(b);
        }
        if let Some(e) = self.panels.setting_errors.get(&s.key) {
            card = card.child(div().text_xs().text_color(p.danger).child(e.clone()));
        }
        if differs && s.kind.editable() {
            let key = s.key.clone();
            let default = s.default.clone();
            let reset = self.button(format!("reset-{}", s.key), if rtl { "إعادة إلى الافتراضي" } else { "Reset to default" }, Kind::Ghost, cx, move |a, _, cx| {
                a.change_setting(&key, default.clone(), cx);
            });
            card = card.child(div().id(SharedString::from(format!("reset-tip-{}", s.key))).flex().when(rtl, |r| r.flex_row_reverse()).tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(reset_tip.clone()).build(window, cx)).child(reset));
        }
        card = card
            .when(d.careful, |c| c.child(div().text_xs().text_color(if risky { p.danger } else { p.muted }).child(if rtl { "إعداد حساس، غيّره بحذر." } else { "Take care with this one." })))
            .when(!s.kind.editable(), |c| c.child(div().text_xs().text_color(p.muted).child("The app does not know this kind of setting yet. Use lh settings in a terminal.")));
        card.into_any_element()
    }

    /// A short scrolling list of options under a setting.
    fn option_list(&self, key: &str, rows: Vec<AnyElement>) -> AnyElement {
        let p = &self.pal;
        div().on_scroll_wheel(|_, _, cx| cx.stop_propagation()).child(div().id(SharedString::from(format!("options-{key}"))).role(Role::List).max_h(px(220.)).overflow_y_scroll().flex().flex_col().gap_1().p_1().rounded(p.inner).bg(p.button).children(rows)).into_any_element()
    }

    /// The page for the chosen category, or the settings that match what is typed in the search box.
    pub fn settings_body(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        use lh_gui::setting_names::{appearance_matches, describe, matches, Category};
        let p = self.pal.clone();
        let mut col = div().flex().flex_col().gap_2();
        if let Some(e) = self.panels.settings_error.clone() {
            col = col.child(div().px_3().py_2().rounded(p.inner).bg(p.danger.opacity(0.08)).border_1().border_color(p.danger.opacity(0.35)).text_sm().text_color(p.danger).child(e));
        }
        let query = self.settings_search.read(cx).value().to_string();
        let searching = !query.trim().is_empty();
        let page = self.settings_page;
        let core_settings = |a: &Self| -> Vec<usize> { (0..a.panels.settings.len()).collect() };
        let mut shown: Vec<usize> = vec![];
        let mut show_appearance = false;
        if searching {
            show_appearance = appearance_matches(&query);
            shown = core_settings(self).into_iter().filter(|&i| {
                let s = &self.panels.settings[i];
                matches(&s.key, &describe(&s.key, &s.help), &query)
            }).collect();
        } else if page == Category::Appearance {
            show_appearance = true;
        } else {
            shown = core_settings(self).into_iter().filter(|&i| describe(&self.panels.settings[i].key, "").category == page).collect();
        }
        if searching {
            col = col.child(div().text_sm().text_color(p.muted).child(format!("Results for \"{}\"", query.trim())));
        } else {
            col = col.child(div().flex().flex_col().gap_0p5().pb_1().child(div().text_lg().font_weight(gpui::FontWeight::MEDIUM).text_color(p.text).child(page.title())).child(div().text_sm().text_color(p.muted).child(page.blurb())));
        }
        if show_appearance {
            col = col.child(self.appearance_block(window, cx));
        }
        if !searching {
            if let Some(body) = self.pg_page(page, window, cx) {
                return col.child(body).into_any_element();
            }
        }
        // Models and keys, Learning and Departments draw their own editors; the cards they replace are left out
        if !searching && self.connected {
            if let Some(extra) = self.extra_page(page, window, cx) {
                col = col.child(extra);
                shown.retain(|&i| !matches!(page, Category::Models) || !self.panels.settings[i].key.starts_with("provider."));
                if page == Category::Departments {
                    shown.clear();
                }
            }
        }
        if !self.connected && !show_appearance {
            return col.child(self.note("The core is not running, so these settings cannot be shown.")).into_any_element();
        }
        for i in shown.iter().copied() {
            col = col.child(self.setting_card(i, window, cx));
        }
        if !searching && page == Category::Web {
            col = col.child(self.pg_devices(window, cx));
        }
        if !searching && page == Category::Models && self.connected {
            col = col.child(self.pg_providers(window, cx));
        }
        if searching && !show_appearance && shown.is_empty() {
            col = col.child(self.note("No setting matches. Try fewer or different words."));
        }
        if !self.connected && show_appearance {
            col = col.child(self.note("The core is not running, so the other settings cannot be shown."));
        }
        col.into_any_element()
    }

    /// The list of pages down the side. Up and Down move between them; Tab moves on into the page.
    pub fn settings_sidebar(&mut self, cx: &mut Context<Self>) -> AnyElement {
        use lh_gui::setting_names::pages;
        let p = self.pal.clone();
        let rtl = self.rtl;
        let list = pages(self.panels.settings.iter().map(|s| s.key.as_str()));
        let ring = BoxShadow { color: p.accent, offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: px(2.), inset: false };
        let searching = !self.settings_search.read(cx).value().trim().is_empty();
        let mut nav = div()
            .id("settings-pages")
            .role(Role::TabList)
            .aria_label("Settings pages")
            .track_focus(&self.settings_nav_focus)
            .tab_stop(true)
            .focus_visible(move |s| s.shadow(vec![ring]))
            .w(px(220.))
            .flex_none()
            .flex()
            .flex_col()
            .gap_0p5()
            .rounded(p.inner)
            .on_key_down(cx.listener(|a, ev: &KeyDownEvent, w, cx| {
                let down = match ev.keystroke.key.as_str() {
                    "down" => true,
                    "up" => false,
                    _ => return,
                };
                let pages = lh_gui::setting_names::pages(a.panels.settings.iter().map(|s| s.key.as_str()));
                let next = lh_gui::setting_names::neighbour(&pages, a.settings_page, down);
                a.set_settings_page(next, w, cx);
                cx.stop_propagation();
            }));
        for c in list {
            let on = c == self.settings_page && !searching;
            let danger = lh_gui::setting_names::page_is_risky(c, self.panels.settings.iter().map(|s| (s.key.as_str(), s.value.as_str())));
            nav = nav.child(
                div()
                    .id(SharedString::from(format!("settings-page-{}", c.id())))
                    .role(Role::Tab)
                    .aria_label(c.title())
                    .aria_selected(on)
                    .px_3()
                    .py_1p5()
                    .rounded(p.inner)
                    .cursor_pointer()
                    .w_full()
                    .text_sm()
                    .text_color(p.text)
                    .when(on, |d| d.bg(p.button).font_weight(gpui::FontWeight::MEDIUM))
                    .hover(|s| s.bg(p.button))
                    .when(rtl, |d| d.text_right())
                    .on_click(cx.listener(move |a, _, w, cx| a.set_settings_page(c, w, cx)))
                    .child(div().flex().when(rtl, |r| r.flex_row_reverse()).items_center().gap_2()
                        .child(div().flex_none().size(px(16.)).child(crate::ui::icon_view(&p, crate::ui::icons::category(c), if on { p.accent } else { p.muted }, 16.)))
                        .child(c.title()).when(danger && !on, |r| r.child(div().w(px(6.)).h(px(6.)).rounded_full().bg(p.danger)))),
            );
        }
        nav.into_any_element()
    }

    /// A web page is not run here. The card says so and offers the two honest things to do: open the file in the
    /// person's own browser, or read its source.
    fn html_card(&mut self, item: &Content, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let target = item.browser_target();
        let key = (u64::MAX, index);
        let source_shown = self.open_blocks.contains(&key);
        let it = item.clone();
        let open = target.is_some().then(|| self.button(format!("html-open-{index}"), "Open in your browser", Kind::Primary, cx, move |a, _, cx| a.open_in_browser(&it, cx)));
        let source = self.button(format!("html-source-{index}"), if source_shown { "Hide source" } else { "View source" }, Kind::Normal, cx, move |a, _, cx| {
            if !a.open_blocks.remove(&key) {
                a.open_blocks.insert(key);
            }
            cx.notify();
        });
        let why = match &target {
            Some(path) => format!("It is the file {}. LastHarness does not run web pages itself; your own browser opens this one file.", path.display()),
            None => "LastHarness does not run web pages itself, and this one is not a file on this computer, so there is nothing to open in a browser. You can read its source.".to_string(),
        };
        // when the page could be drawn inside the app but was not, say why in plain words
        let reason = self.page_reason(item);
        div().flex().flex_col().gap_2()
            .child(div().flex().flex_col().gap_2().p_3().rounded(p.inner).bg(p.sunken).text_color(p.text).id(SharedString::from(format!("html-card-{index}"))).role(Role::Group).aria_label("This is a web page")
                .child(div().child("This is a web page"))
                .child(div().text_sm().text_color(p.muted).child(why))
                .when_some(reason, |d, r| d.child(div().text_sm().text_color(p.text).child(r)))
                .when_some(self.content_note.clone(), |d, n| d.child(div().text_sm().text_color(p.danger).child(n)))
                .child(div().flex().flex_wrap().gap_2().children(open).child(source)))
            .when(source_shown, |d| d.child(code_view(&item.body, &format!("html-{index}"), &item.title, &p)))
            .into_any_element()
    }

    /// Ask the desktop to open an approved local web page in the person's browser. Nothing else is ever opened.
    pub fn open_in_browser(&mut self, item: &Content, cx: &mut Context<Self>) {
        self.content_note = None;
        match item.browser_target() {
            None => self.content_note = Some("This page is not a local file, so it cannot be opened.".into()),
            Some(path) => match lh_gui::config::desktop_open(&path).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn() {
                Ok(mut child) => {
                    // reap it when it ends, without waiting here
                    std::thread::spawn(move || {
                        let _ = child.wait();
                    });
                }
                Err(e) => self.content_note = Some(format!("Could not start the browser: {e}")),
            },
        }
        cx.notify();
    }

    /// The branches of the open chat: a line per message, the branch you are on marked, and what can be done with the chosen line.
    fn tree_body(&mut self, cx: &mut Context<Self>) -> AnyElement {
        use lh_gui::treeview::Prompt;
        let p = self.pal.clone();
        let rtl = self.rtl;
        let mut col = div().flex().flex_col().gap_2();
        let Some(tree) = self.tree.tree.clone() else {
            let why = if self.tree.note.is_empty() { "Loading…".to_string() } else { self.tree.note.clone() };
            return col.child(self.note(why)).into_any_element();
        };
        let at = self.tree.at;
        col = col.child(self.note(if rtl { "● الفرع الحالي   ◆ مكان المحادثة" } else { "● the branch you are on   ◆ where the chat is. Choose a line, then Go here." }));
        // the whole-tree controls
        let tools = self.button("tree-tools", if tree.hide_tools { "Show tool results" } else { "Hide tool results" }, Kind::Ghost, cx, |a, _, cx| {
            a.tree.toggle_tools();
            cx.notify();
        });
        let copy = self.button("tree-clone", "Copy this branch to a new chat", Kind::Ghost, cx, |a, _, cx| {
            a.tree_clone();
            cx.notify();
        });
        col = col.child(div().flex().flex_wrap().gap_1().child(tools).child(copy));
        if !self.tree.note.is_empty() {
            col = col.child(self.note(self.tree.note.clone()));
        }
        // a question about keeping a summary of the branch being left
        if let Prompt::Summary { node, leaving } = self.tree.prompt.clone() {
            let yes = self.button("tree-sum-yes", "Keep a summary", Kind::Primary, cx, move |a, _, cx| {
                a.tree_go(node, Some(true));
                cx.notify();
            });
            let no = self.button("tree-sum-no", "No summary", Kind::Normal, cx, move |a, _, cx| {
                a.tree_go(node, Some(false));
                cx.notify();
            });
            let stay = self.button("tree-sum-stay", "Stay here", Kind::Ghost, cx, |a, _, cx| {
                a.tree.prompt = Prompt::None;
                cx.notify();
            });
            col = col.child(
                div().flex().flex_col().gap_2().p_2().rounded(p.inner).bg(p.sunken).border_1().border_color(p.accent).text_color(p.text)
                    .id("tree-summary-question").role(Role::Group).aria_label("Keep a summary of the branch you are leaving")
                    .child(div().child(format!("You are leaving {leaving} message(s) on this branch. Keep a short summary of them where you land?")))
                    .child(div().flex().flex_wrap().gap_2().child(yes).child(no).child(stay)),
            );
        }
        // a name for the chosen line
        if self.tree.prompt == Prompt::Label {
            let save = self.button("tree-label-save", "Save name", Kind::Primary, cx, move |a, _, cx| {
                let text = a.tree_label.read(cx).value().to_string();
                a.tree_label(at, text);
                cx.notify();
            });
            let cancel = self.button("tree-label-cancel", "Cancel", Kind::Ghost, cx, |a, _, cx| {
                a.tree.prompt = Prompt::None;
                cx.notify();
            });
            col = col.child(div().flex().flex_col().gap_2().p_2().rounded(p.inner).bg(p.sunken).child(gpui_component::input::Input::new(&self.tree_label)).child(div().flex().gap_2().child(save).child(cancel)));
        }
        // the lines
        let rows = tree.rows();
        let lines: Vec<AnyElement> = rows
            .iter()
            .map(|r| {
                let (id, chosen) = (r.id, r.id == at);
                let marker = if r.current { "◆" } else if r.active { "●" } else { "○" };
                let who = match r.role.as_str() {
                    "user" => "you",
                    "assistant" => "assistant",
                    "tool" => "tool",
                    "summary" => "summary",
                    _ => "",
                };
                let label = r.label.as_ref().map(|l| format!("  [{l}]")).unwrap_or_default();
                let text = format!("{marker} {}{}{label}", if who.is_empty() { String::new() } else { format!("{who}: ") }, r.text.chars().take(90).collect::<String>());
                let fold = r.foldable.then(|| self.button(format!("tree-fold-{id}"), if r.folded { format!("▸ +{}", r.hidden_below) } else { "▾".to_string() }, Kind::Ghost, cx, move |a, _, cx| {
                    a.tree.toggle_fold(id);
                    cx.notify();
                }));
                div()
                    .id(SharedString::from(format!("tree-row-{id}")))
                    .role(Role::ListItem)
                    .aria_label(text.clone())
                    .aria_selected(chosen)
                    .flex()
                    .when(rtl, |d| d.flex_row_reverse())
                    .items_center()
                    .gap_1()
                    .pl(px(r.depth as f32 * 14.))
                    .px_1()
                    .py_0p5()
                    .rounded(p.inner)
                    .cursor_pointer()
                    .text_sm()
                    .text_color(if r.current || r.active { p.text } else { p.muted })
                    .bg(if chosen { p.accent.opacity(0.22) } else { hsla(0., 0., 0., 0.) })
                    .on_click(cx.listener(move |a, _, _, cx| {
                        a.tree.select(id);
                        cx.notify();
                    }))
                    .child(div().flex_1().overflow_hidden().child(text))
                    .children(fold)
                    .into_any_element()
            })
            .collect();
        col = col.child(div().id("tree-lines").role(Role::List).aria_label("Messages in this chat").flex().flex_col().gap_0p5().children(lines));
        // what can be done with the chosen line
        let chosen_is_message = at != 0;
        let go = self.button("tree-go", "Go here", Kind::Primary, cx, move |a, _, cx| {
            a.tree_go(at, None);
            cx.notify();
        });
        let name = chosen_is_message.then(|| self.button("tree-name", "Name…", Kind::Normal, cx, move |a, window, cx| {
            let current = a.tree.tree.as_ref().and_then(|t| t.node(at)).and_then(|n| n.label.clone()).unwrap_or_default();
            a.tree_label.update(cx, |s, cx| s.set_value(current, window, cx));
            a.tree.prompt = Prompt::Label;
            cx.notify();
        }));
        let fork = self.button("tree-fork", "New chat from here", Kind::Normal, cx, move |a, _, cx| {
            a.tree_fork(at);
            cx.notify();
        });
        col = col.child(div().flex().flex_wrap().gap_2().child(go).children(name).child(fork));
        col.into_any_element()
    }

    fn content_body(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let mut col = div().flex().flex_col().gap_2();
        if self.content.is_empty() {
            return col.child(self.note("Nothing opened yet. Use \"Open in panel\" on a reply, a tool result or a memory note to read it here, larger, or in its own window.")).into_any_element();
        }
        let cur = self.content_index.min(self.content.len() - 1);
        // one chip per opened thing; the open one is raised and carries its own close cross
        let rtl = self.rtl;
        let tabs: Vec<AnyElement> = (0..self.content.len()).map(|i| {
            let t = self.content[i].title.chars().take(28).collect::<String>();
            let on = i == cur;
            let close = on.then(|| div().id(SharedString::from(format!("content-close-{i}"))).role(Role::Button).aria_label(if rtl { "إغلاق" } else { "Close" }).cursor_pointer().rounded_full().hover(|s| s.bg(p.button))
                .on_click(cx.listener(move |a, _, _, cx| {
                    // the chat still holds it; "Open beside" there brings it back
                    a.content.remove(i);
                    a.content_index = a.content_index.saturating_sub(1);
                    if a.content.is_empty() {
                        a.layout_mut().toggle_panel(Panel::Content);
                    }
                    cx.notify();
                }))
                .child(crate::ui::icon_view(&p, crate::ui::icons::close(), p.muted, 12.)));
            div().id(SharedString::from(format!("content-{i}"))).role(Role::Tab).aria_label(t.clone()).aria_selected(on)
                .flex().when(rtl, |d| d.flex_row_reverse()).items_center().gap_1p5().px_2p5().py_1().rounded(p.inner).border_1().cursor_pointer().text_sm()
                .border_color(if on { p.border } else { hsla(0., 0., 0., 0.) })
                .bg(if on { p.surface } else { hsla(0., 0., 0., 0.) })
                .text_color(if on { p.text } else { p.muted })
                .hover(|s| s.text_color(p.text))
                .on_click(cx.listener(move |a, _, _, cx| {
                    a.content_index = i;
                    cx.notify();
                }))
                .child(if t.is_empty() { "Untitled".to_string() } else { t })
                .children(close)
                .into_any_element()
        }).collect();
        let item = self.content[cur].clone();
        col = col.child(div().flex().when(rtl, |d| d.flex_row_reverse()).flex_wrap().gap_1().children(tabs)).child(if item.kind == ContentKind::Html { self.page_element(&item, "panel", window, cx).unwrap_or_else(|| self.html_card(&item, cur, cx)) } else { viewer(&item, "panel", &p) });
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
            div().flex().flex_col().gap_2().p_2().panel(&p, false)
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
        let mut col = div().flex().flex_col().gap_2().p_3().panel(&p, false).text_color(p.text)
            .child(div().flex().items_center().gap_2().child(div().flex_1().child(r.goal.clone())).child(div().text_sm().text_color(if r.is_waiting() { p.accent } else { p.muted }).child(format!("{} {}", r.status, if total > 0 { format!("({done}/{total})") } else { String::new() }))));
        for s in &r.steps {
            col = col.child(div().flex().gap_2().text_sm().child(div().w(px(16.)).text_color(match s.state { StepState::Failed => p.danger, StepState::Done => p.good, StepState::Parked | StepState::Blocked => p.warn, _ => p.accent }).child(state_mark(s.state))).child(div().flex_1().child(format!("{} · {}{}", s.title, s.assignee, if s.note.is_empty() { String::new() } else { format!(" — {}", s.note.chars().take(160).collect::<String>()) }))));
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

fn code_view(body: &str, scope: &str, title: &str, p: &crate::style::Pal) -> AnyElement {
    let lines: Vec<AnyElement> = body.lines().enumerate().map(|(i, l)| div().flex().gap_3().child(div().w(px(36.)).flex().justify_end().text_color(p.muted).child((i + 1).to_string())).child(div().child(if l.is_empty() { " ".to_string() } else { l.to_string() })).into_any_element()).collect();
    div().id(SharedString::from(format!("code-{scope}"))).role(Role::Document).aria_label(title.to_string()).p_2().rounded(p.inner).bg(p.sunken).text_color(p.text).font_family(p.mono.clone()).text_sm().overflow_x_scroll().children(lines).into_any_element()
}

/// The native viewer for a piece of content. Used in the panel and in a pop-out window.
pub fn viewer(item: &Content, scope: &str, p: &crate::style::Pal) -> AnyElement {
    let rtl = lh_gui::rtl::paragraph_is_rtl(&item.body);
    match item.kind {
        ContentKind::Markdown => div().text_color(p.text).when(rtl, |d| d.text_right()).child(TextView::markdown(SharedString::from(format!("view-{scope}-{}", item.title)), item.body.clone())).into_any_element(),
        ContentKind::Code => code_view(&item.body, scope, &item.title, p),
        ContentKind::Html => div().flex().flex_col().gap_2().child(div().text_sm().text_color(p.muted).child("Web page source (open the page from the side panel).")).child(code_view(&item.body, scope, &item.title, p)).into_any_element(),
        ContentKind::Image => div().child(img(std::path::PathBuf::from(item.body.clone())).max_w_full().object_fit(ObjectFit::Contain)).into_any_element(),
        ContentKind::Text => div().text_color(p.text).when(rtl, |d| d.text_right()).child(item.body.clone()).into_any_element(),
        ContentKind::Table => lh_gui::content::Table::parse(&item.body).map(|t| table_view(&t, p)).unwrap_or_else(|| code_view(&item.body, scope, &item.title, p)),
        ContentKind::Chart => lh_gui::content::Chart::parse(&item.body).map(|c| chart_view(&c, p)).unwrap_or_else(|| code_view(&item.body, scope, &item.title, p)),
    }
}

/// The colour of a chart's series: the theme's accent first, then its hue turned a step at a time.
pub(crate) fn series_color(p: &crate::style::Pal, i: usize) -> Hsla {
    // the first series is the accent; the rest take well-spaced hues, skipping any too near the accent. Lightness
    // follows the mode, so every series stands at least 3:1 off the surface in light and dark looks alike.
    if !p.series.is_empty() {
        return p.series[i % p.series.len()];
    }
    let a = p.accent;
    let (s, l) = if p.dark { (0.62, 0.68) } else { (0.62, 0.36) };
    let grey = a.s < 0.15;
    if i == 0 && !grey {
        return Hsla { h: a.h, s: a.s.max(0.45), l, a: 1.0 };
    }
    const HUES: [f32; 7] = [0.58, 0.47, 0.11, 0.76, 0.95, 0.33, 0.03];
    let near = |h: f32| !grey && { let d = (h - a.h).abs(); d.min(1. - d) < 0.07 };
    let hues: Vec<f32> = HUES.iter().copied().filter(|h| !near(*h)).collect();
    let k = if grey { i } else { i - 1 };
    Hsla { h: hues[k % hues.len()], s, l, a: 1.0 }
}

/// A table drawn with the app's own type and lines: a header row, then rows; number columns line up on the right.
pub fn table_view(t: &lh_gui::content::Table, p: &crate::style::Pal) -> AnyElement {
    let cell = |text: String, right: bool, head: bool| {
        // numbers line up digit under digit
        div().flex_1().min_w(px(64.)).px_3().py_1p5().when(right, |d| d.text_right().font_features(gpui::FontFeatures(std::sync::Arc::new(vec![("tnum".into(), 1)])))).when(head, |d| d.text_xs().text_color(p.muted)).when(!head, |d| d.text_sm().text_color(p.text)).child(text)
    };
    let head = div().flex().border_b_1().border_color(p.border).children(t.columns.iter().enumerate().map(|(c, n)| cell(n.clone(), t.numeric.get(c).copied().unwrap_or(false), true)));
    let rows = t.rows.iter().enumerate().map(|(r, row)| {
        div().flex().when(r % 2 == 1, |d| d.bg(p.sunken.opacity(0.5))).children(row.iter().enumerate().map(|(c, v)| {
            let num = t.numeric.get(c).copied().unwrap_or(false);
            let v = if num { v.parse::<f64>().map(lh_gui::content::short_number).unwrap_or_else(|_| v.clone()) } else { v.clone() };
            cell(v, num, false)
        }))
    });
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().flex().flex_col().rounded(p.inner).border_1().border_color(p.border).overflow_hidden().child(head).children(rows))
        .children(t.caption.clone().map(|c| div().text_xs().text_color(p.muted).child(c)))
        .into_any_element()
}

/// A chart drawn from numbers: bars, a line, or a pie, with the value axis, labels and a legend.
pub fn chart_view(c: &lh_gui::content::Chart, p: &crate::style::Pal) -> AnyElement {
    use lh_gui::content::ChartKind;
    let legend = (c.series.len() > 1 || c.kind == ChartKind::Pie).then(|| {
        let names: Vec<String> = if c.kind == ChartKind::Pie { c.labels.clone() } else { c.series.iter().map(|s| s.0.clone()).collect() };
        div().flex().flex_wrap().gap_3().children(names.into_iter().enumerate().map(|(i, n)| div().flex().items_center().gap_1p5().text_xs().text_color(p.muted).child(div().size(px(8.)).rounded_full().bg(series_color(p, i))).child(n)))
    });
    let caption = c.caption.clone().map(|t| div().text_xs().text_color(p.muted).child(t));
    let plot = match c.kind {
        ChartKind::Pie => {
            let slices = c.slices();
            let colors: Vec<Hsla> = (0..slices.len()).map(|i| series_color(p, i)).collect();
            div().h(px(200.)).w_full().child(canvas(|_, _, _| (), move |b, _, window, _| {
                let r = (f32::from(b.size.height).min(f32::from(b.size.width)) / 2.) - 4.;
                let o = point(b.origin.x + b.size.width / 2., b.origin.y + b.size.height / 2.);
                for ((s, e), col) in slices.iter().zip(&colors) {
                    if e - s <= 0. {
                        continue;
                    }
                    let mut pb = PathBuilder::fill();
                    pb.move_to(o);
                    let steps = ((e - s) * 96.).ceil().max(2.) as usize;
                    for k in 0..=steps {
                        let t = (s + (e - s) * k as f32 / steps as f32) * std::f32::consts::TAU;
                        pb.line_to(point(o.x + px(r * t.sin()), o.y - px(r * t.cos())));
                    }
                    pb.close();
                    if let Ok(path) = pb.build() {
                        window.paint_path(path, *col);
                    }
                }
            }).size_full()).into_any_element()
        }
        _ => {
            let (lo, hi, step) = c.axis();
            let ticks: Vec<f64> = (0..=(((hi - lo) / step).round() as usize).min(10)).map(|k| lo + step * k as f64).collect();
            let levels: Vec<Vec<f32>> = c.series.iter().map(|s| s.1.iter().map(|v| c.level(*v)).collect()).collect();
            let zero = c.level(0.);
            let colors: Vec<Hsla> = (0..levels.len()).map(|i| series_color(p, i)).collect();
            let (grid, kind, n) = (p.border, c.kind, c.labels.len().max(1));
            // one series of bars: the biggest value takes the accent, the rest sit in a quiet neutral, so the bar
            // that matters reads first
            let (rest, lead) = (p.muted, levels.first().filter(|_| levels.len() == 1).and_then(|ls| ls.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).map(|(i, _)| i)));
            let tick_levels: Vec<f32> = ticks.iter().map(|t| c.level(*t)).collect();
            let area = canvas(|_, _, _| (), move |b, _, window, _| {
                let (x0, y0, w, h) = (b.origin.x, b.origin.y, f32::from(b.size.width), f32::from(b.size.height));
                let y_of = |l: f32| y0 + px(h * (1. - l));
                for l in &tick_levels {
                    let mut g = PathBuilder::stroke(px(1.));
                    g.move_to(point(x0, y_of(*l)));
                    g.line_to(point(x0 + px(w), y_of(*l)));
                    if let Ok(path) = g.build() {
                        window.paint_path(path, grid);
                    }
                }
                let slot = w / n as f32;
                match kind {
                    ChartKind::Bar => {
                        let k = levels.len().max(1) as f32;
                        let bar = (slot * 0.7 / k).min(48.);
                        let group = bar * k;
                        for (si, ls) in levels.iter().enumerate() {
                            for (i, l) in ls.iter().enumerate() {
                                let x = slot * i as f32 + (slot - group) / 2. + bar * si as f32;
                                let (top, bottom) = (l.max(zero), l.min(zero));
                                let r = Bounds { origin: point(x0 + px(x + 1.), y_of(top)), size: size(px((bar - 2.).max(1.)), px((h * (top - bottom)).max(1.))) };
                                let col = match lead { Some(m) if m != i => rest, _ => colors[si] };
                                window.paint_quad(fill(r, col).corner_radii(Corners { top_left: px(3.), top_right: px(3.), bottom_left: px(0.), bottom_right: px(0.) }));
                            }
                        }
                    }
                    _ => {
                        for (si, ls) in levels.iter().enumerate() {
                            let mut pb = PathBuilder::stroke(px(2.));
                            for (i, l) in ls.iter().enumerate() {
                                let pt = point(x0 + px(slot * (i as f32 + 0.5)), y_of(*l));
                                if i == 0 { pb.move_to(pt) } else { pb.line_to(pt) }
                            }
                            if let Ok(path) = pb.build() {
                                window.paint_path(path, colors[si]);
                            }
                            for (i, l) in ls.iter().enumerate() {
                                let c = point(x0 + px(slot * (i as f32 + 0.5) - 3.), y_of(*l) - px(3.));
                                window.paint_quad(fill(Bounds { origin: c, size: size(px(6.), px(6.)) }, colors[si]).corner_radii(px(3.)));
                            }
                        }
                    }
                }
            }).size_full();
            let axis = div().relative().w(px(44.)).h_full().children(ticks.iter().map(|t| {
                let l = c.level(*t);
                div().absolute().right(px(6.)).top(relative(1. - l)).mt(px(-8.)).text_xs().text_color(p.muted).child(lh_gui::content::short_number(*t))
            }));
            let every = (c.labels.len() / 12).max(1);
            let labels = div().flex().ml(px(44.)).children(c.labels.iter().enumerate().map(|(i, l)| div().flex_1().min_w_0().overflow_hidden().text_center().text_xs().text_color(p.muted).child(if i % every == 0 { l.clone() } else { String::new() })));
            div().flex().flex_col().gap_1().child(div().flex().h(px(200.)).child(axis).child(div().flex_1().h_full().child(area))).child(labels).into_any_element()
        }
    };
    div().flex().flex_col().gap_2().w_full().child(plot).children(legend).children(caption).into_any_element()
}

/// A piece of content in a window of its own.
pub struct PopOut {
    pub item: Content,
    pub pal: crate::style::Pal,
}

impl Render for PopOut {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.pal.clone();
        p.paint_background(div().size_full().relative().p_4().text_color(p.text)).children(p.image_layer()).child(div().id("popout").size_full().overflow_y_scroll().child(viewer(&self.item, "popout", &p)))
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

/// Whether the open chat has no memory steps.
fn col_is_empty(chats: &lh_gui::model::Chats, _rtl: bool) -> bool {
    chats.current().map_or(true, |c| !c.blocks.iter().any(|b| matches!(b, Block::Tool { name, .. } if name == "recall" || name == "remember")))
}
