//! Drawing: the window as the theme describes it. Reads the state in `App`, never changes it except through
//! the click handlers it attaches.

use gpui::prelude::*;
use gpui::*;
use gpui_component::input::{Input, Textarea};
use gpui_component::text::TextView;

use crate::app::{App, Focus, Pending};
use lh_gui::chrome::{chevron, scrim_alpha, topmost, Layer, RailItem, TitleInfo, RAIL_WIDTH, SETTINGS_WIDTH};
use lh_gui::permissions::may_offer;
use lh_gui::transcript::{self, work_label, Item, Step, StepKind};
use lh_gui::layout::{Page, Panel};
use lh_gui::motion;
use lh_gui::model::{Block, Mode};
use lh_gui::rtl;
use crate::style::{mix, Look, PanelExt};

actions!(lastharness, [NewChat, ToggleSidebar, ShowDepartments, ShowMemory, ShowSettings, ShowContent, ShowTree, ToggleMode, CycleDirection, StopTurn, NextChat, PrevChat, ToggleExpand, ConfirmResume, Dismiss, OpenLastReply, PopOutContent, AllowAsk, DenyAsk, OpenPalette, JumpToLatest]);

/// The keys for every action; also how the app is driven without a mouse.
pub fn bind_keys(cx: &mut gpui::App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-k", OpenPalette, None),
        KeyBinding::new("ctrl-n", NewChat, None),
        KeyBinding::new("ctrl-b", ToggleSidebar, None),
        KeyBinding::new("ctrl-shift-d", ShowDepartments, None),
        KeyBinding::new("ctrl-shift-m", ShowMemory, None),
        KeyBinding::new("ctrl-shift-s", ShowSettings, None),
        KeyBinding::new("ctrl-shift-o", ShowContent, None),
        KeyBinding::new("ctrl-shift-t", ShowTree, None),
        KeyBinding::new("ctrl-shift-e", ToggleExpand, None),
        KeyBinding::new("ctrl-t", ToggleMode, None),
        KeyBinding::new("ctrl-shift-r", CycleDirection, None),
        KeyBinding::new("ctrl-.", StopTurn, None),
        KeyBinding::new("ctrl-enter", ConfirmResume, None),
        KeyBinding::new("escape", Dismiss, None),
        KeyBinding::new("ctrl-shift-l", OpenLastReply, None),
        KeyBinding::new("ctrl-shift-j", JumpToLatest, None),
        KeyBinding::new("ctrl-shift-p", PopOutContent, None),
        KeyBinding::new("ctrl-pagedown", NextChat, None),
        KeyBinding::new("ctrl-pageup", PrevChat, None),
        // only while a permission question waits and the message box is empty, so typing is never taken
        KeyBinding::new("y", AllowAsk, Some("Asking")),
        KeyBinding::new("n", DenyAsk, Some("Asking")),
    ]);
}

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Normal,
    Primary,
    Danger,
    Ghost,
}

/// A panel part way open: it shows `shown` pixels of itself and keeps the layout of its full width, so it slides
/// instead of squeezing its text.
fn slide(shown: f32, full: f32, child: AnyElement) -> AnyElement {
    if (shown - full).abs() < 0.5 {
        return child;
    }
    div().w(px(shown)).h_full().flex_none().overflow_hidden().child(child).into_any_element()
}

/// An icon drawn as lines, so no picture files are needed. `lines` are on a 24 by 24 grid.
fn icon_view(lines: Vec<Vec<(f32, f32)>>, color: Hsla, size: f32) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |b, _, window, _| {
            let scale = f32::from(b.size.width.min(b.size.height)) / 24.;
            for line in &lines {
                let mut pb = PathBuilder::stroke(px(1.7 * scale));
                for (i, (x, y)) in line.iter().enumerate() {
                    let at = point(b.left() + px(x * scale), b.top() + px(y * scale));
                    if i == 0 {
                        pb.move_to(at);
                    } else {
                        pb.line_to(at);
                    }
                }
                if let Ok(path) = pb.build() {
                    window.paint_path(path, color);
                }
            }
        },
    )
    .size(px(size))
}

impl App {
    /// A button that fades between its resting and hovered look. `id` must be unique on screen.
    pub fn button(&self, id: impl Into<SharedString>, label: impl Into<SharedString>, kind: Kind, cx: &mut Context<Self>, on_click: impl Fn(&mut App, &mut Window, &mut Context<App>) + 'static) -> AnyElement {
        let (id, label): (SharedString, SharedString) = (id.into(), label.into());
        let p = &self.pal;
        let (base, hover, fg) = match kind {
            Kind::Normal => (p.button, p.hover, p.text),
            Kind::Primary => (p.accent, mix(p.accent, p.text, 0.18), p.accent_text),
            Kind::Danger => (p.danger.opacity(0.9), p.danger, p.accent_text),
            Kind::Ghost => (hsla(0., 0., 0., 0.), p.button, p.text),
        };
        let hovered = self.hover.as_ref() == Some(&id);
        let key = id.clone();
        let aria = SharedString::from(match label.as_ref() {
            "×" => "Delete chat",
            "−" => "Make narrower",
            "+" => "Make wider",
            other => other,
        }.to_string());
        let on_hover = cx.listener(move |this, h: &bool, _, cx| {
            this.hover_ever = true;
            if *h {
                this.hover = Some(key.clone());
            } else if this.hover.as_ref() == Some(&key) {
                this.hover = None;
            }
            cx.notify();
        });
        let animate = !self.pal.anim.is_zero() && self.hover_ever;
        let anim_key = SharedString::from(format!("{id}-{hovered}"));
        let on_click = cx.listener(move |this, _, window, cx| on_click(this, window, cx));
        let focusable = self.tab_buttons;
        let ring = BoxShadow { color: p.accent, offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: px(2.), inset: false };
        let base_el = div().id(id.clone()).role(Role::Button).aria_label(aria).when(focusable, |d| d.focusable().tab_stop(true).focus_visible(move |s| s.shadow(vec![ring]))).px_3().py_1p5().rounded(p.inner).text_color(fg).cursor_pointer().on_hover(on_hover).on_click(on_click);

        // gel: a glossy skin under the label, a rim light, a glow under the pointer and a dark veil while pressed
        if let Some(g) = &p.gel {
            let (rest, over) = g.looks(p.danger, kind == Kind::Ghost, kind == Kind::Primary, kind == Kind::Danger);
            let (from, to) = if hovered { (rest, over) } else { (over, rest) };
            let (radius, press) = (p.inner, g.press);
            let group = SharedString::from(format!("gel-{id}"));
            let skin = g.skin(radius, if hovered { over } else { rest });
            let skin = if animate {
                let g2 = g.clone();
                skin.with_animation(anim_key, Animation::new(motion::press(self.pal.anim)).with_easing(motion::ease_out), move |_, d| g2.skin(radius, Look::mix(from, to, d))).into_any_element()
            } else {
                skin.into_any_element()
            };
            let veil = div().id(SharedString::from(format!("veil-{id}"))).absolute().top_0().left_0().size_full().rounded(radius).group_active(group.clone(), move |s| s.bg(hsla(0.6, 0.6, 0.1, press)));
            return base_el.group(group).child(skin).child(veil).child(label).into_any_element();
        }
        // pressing darkens the button a little more, answering at once
        let pressed = mix(hover, p.text, 0.12);
        let el = base_el.bg(if hovered { hover } else { base }).active(move |s| s.bg(pressed)).child(label);
        if !animate {
            return el.into_any_element();
        }
        let (from, to) = if hovered { (base, hover) } else { (hover, base) };
        el.with_animation(anim_key, Animation::new(motion::press(self.pal.anim)).with_easing(motion::ease_out), move |el, d| el.bg(mix(from, to, d))).into_any_element()
    }

    /// The slim column of icons on the side: the chat list and the panels that are not the chat.
    fn rail(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let rtl = self.rtl;
        let l = self.layout().clone();
        let mut items: Vec<AnyElement> = vec![];
        for item in RailItem::ALL {
            let on = item.pressed(&l, self.settings_open);
            // settings sits at the bottom, away from the places you go to
            if item == RailItem::Settings {
                items.push(div().flex_1().into_any_element());
            }
            items.push(
                div()
                    .id(SharedString::from(format!("rail-{}", item.label(false))))
                    .role(Role::Button)
                    .aria_label(item.label(rtl))
                    .aria_keyshortcuts(item.shortcut())
                    .aria_toggled(if on { Toggled::True } else { Toggled::False })
                    .w(px(36.))
                    .h(px(36.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(p.inner)
                    .cursor_pointer()
                    .bg(if on { p.surface } else { hsla(0., 0., 0., 0.) })
                    .hover(|s| s.bg(p.button))
                    .child(icon_view(item.icon(), if on { p.accent } else { p.muted }, 18.))
                    .on_click(cx.listener(move |a, _, w, cx| match (item, item.panel()) {
                        (RailItem::Settings, _) => a.toggle_settings(w, cx),
                        (_, None) => {
                            a.layout_mut().toggle_left();
                            a.persist();
                            cx.notify();
                        }
                        (_, Some(panel)) => a.open_panel(panel, cx),
                    }))
                    .into_any_element(),
            );
        }
        div()
            .id("rail")
            .role(Role::Navigation)
            .aria_label(if rtl { "الأقسام" } else { "Sections" })
            .flex()
            .flex_col()
            .items_center()
            .gap_1p5()
            .w(px(RAIL_WIDTH))
            .h_full()
            .py_2()
            .panel(&p, false)
            .children(items)
            .into_any_element()
    }

    /// The bar over the chat: its name, the branch it is on, how much the model has read, and which model.
    fn top_bar(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let rtl = self.rtl;
        let info = match self.page().clone() {
            Page::Department(name) => TitleInfo::page(&name),
            Page::Chat => {
                let chat = self.chats.current().cloned();
                TitleInfo::chat(chat.as_ref().map(|c| c.title.as_str()).unwrap_or(""), chat.as_ref().map(|c| c.model.as_str()).unwrap_or(""), chat.as_ref().and_then(|c| c.context_tokens), self.branches(), rtl)
            }
        };
        let model_btn = info.model.as_ref().map(|m| {
            self.button("model", format!("{m} ▾"), Kind::Normal, cx, |a, _, cx| {
                a.model_menu = !a.model_menu;
                cx.notify();
            })
        });
        div()
            .id("title-bar")
            .role(Role::Banner)
            .flex()
            .when(rtl, |d| d.flex_row_reverse())
            .items_center()
            .gap_3()
            .p_2()
            // over a sky the title needs glass behind it
            .when(p.glass.is_some(), |d| d.m_2().mb_0().panel(&p, false))
            .child(div().flex_1().overflow_hidden().text_color(p.text).child(info.title))
            .when_some(info.branch, |d, b| d.child(div().id("branch").role(Role::Label).text_sm().text_color(p.muted).child(b)))
            .when_some(info.context, |d, c| d.child(div().id("context").role(Role::Label).text_sm().text_color(p.muted).child(c)))
            .children(model_btn)
            .into_any_element()
    }

    fn menus(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let p = self.pal.clone();
        if !self.model_menu {
            return None;
        }
        let items: Vec<AnyElement> = self.models.names().to_vec().into_iter().enumerate().map(|(i, m)| {
            let name = m.clone();
            let cur = self.chats.current().map(|c| c.model == m).unwrap_or(false);
            self.button(format!("model-{i}"), m, if cur { Kind::Primary } else { Kind::Ghost }, cx, move |a, _, cx| a.choose_model(&name, cx))
        }).collect();
        let rtl = self.rtl;
        Some(
            div().absolute().top(px(if p.glass.is_some() { 58. } else { 46. })).when(rtl, |d| d.left(px(12.))).when(!rtl, |d| d.right(px(12.))).w(px(300.)).max_h(px(420.)).overflow_hidden().p_2().flex().flex_col().gap_1().panel(&p, false)
                .id("menu").role(Role::Menu).children(items).into_any_element(),
        )
    }

    fn sidebar(&mut self, width: f32, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let rtl = self.rtl;
        let new = self.button("new-chat", if rtl { "محادثة جديدة" } else { "New chat" }, Kind::Primary, cx, |a, _, cx| a.new_chat(cx));
        let mut rows: Vec<AnyElement> = vec![];
        let active = self.chats.active;
        // a department's page opens from here, and the chat keeps running while it is open
        let on_page = self.page().clone();
        let depts: Vec<AnyElement> = self.panels.departments.clone().into_iter().map(|d| {
            let name = d.name.clone();
            let waiting = self.panels.runs.values().any(|r| r.dept == d.name && r.is_waiting());
            let working = self.panels.runs.values().any(|r| r.dept == d.name && !r.finished);
            let label = format!("{}{}", d.name, if waiting { "  ● waiting for you" } else if working { "  … working" } else { "" });
            let open = on_page == Page::Department(d.name.clone());
            self.button(format!("side-dept-{name}"), label, if open { Kind::Primary } else { Kind::Ghost }, cx, move |a, _, cx| a.open_department(&name, cx))
        }).collect();
        for (i, c) in self.chats.list.clone().into_iter().enumerate() {
            let key = c.key;
            let on = i == active;
            let arming = self.confirm_delete == Some(key);
            let del = self.button(format!("del-{key}"), if arming { if rtl { "حذف؟" } else { "Delete?" } } else { "×" }, if arming { Kind::Danger } else { Kind::Ghost }, cx, move |a, _, cx| a.delete_chat(key, cx));
            let pick = div()
                .id(SharedString::from(format!("chat-{key}")))
                .role(Role::ListItem)
                .aria_label(c.title.clone())
                .aria_selected(on)
                .flex_1()
                .overflow_hidden()
                .px_2()
                .py_1p5()
                .rounded(p.inner)
                .cursor_pointer()
                .text_color(if c.dormant { p.muted } else { p.text })
                .bg(if on { p.accent.opacity(0.22) } else { hsla(0., 0., 0., 0.) })
                .hover(|s| s.bg(p.button))
                .on_click(cx.listener(move |a, _, _, cx| a.select_chat(key, cx)))
                .child(div().child(c.title.clone()))
                .child(div().text_xs().text_color(p.muted).child(c.model.clone()));
            rows.push(div().flex().when(rtl, |d| d.flex_row_reverse()).items_center().gap_1().child(pick).child(del).into_any_element());
        }
        div()
            .flex()
            .flex_col()
            .gap_2()
            .w(px(width))
            .h_full()
            .p_2()
            .panel(&p, self.focus == Focus::Sidebar)
            .on_mouse_down(MouseButton::Left, cx.listener(|a, _, _, cx| a.focus_on(Focus::Sidebar, cx)))
            .child(new)
            .children((!depts.is_empty()).then(|| div().flex().flex_col().gap_0p5().child(div().text_xs().text_color(p.muted).px_2().child(if rtl { "الأقسام" } else { "Departments" })).children(depts)))
            .child(div().id("chat-list").role(Role::List).aria_label("Chats").flex_1().flex().flex_col().gap_0p5().overflow_y_scroll().children(rows))
            .into_any_element()
    }

    fn block_view(&self, key: u64, i: usize, b: &Block, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let rtl = self.rtl;
        match b {
            Block::User(t) => {
                let is_rtl = rtl::paragraph_is_rtl(t);
                div().flex().w_full().when(!rtl, |d| d.justify_end()).when(rtl, |d| d.justify_start()).child(
                    div().id(SharedString::from(format!("u{key}-{i}"))).role(Role::Label).max_w(relative(0.8)).px_3().py_2().rounded(p.inner).bg(p.accent.opacity(0.18)).text_color(p.text).when(is_rtl, |d| d.text_right()).child(t.clone()),
                ).into_any_element()
            }
            Block::Assistant { .. } | Block::Tool { .. } => div().into_any_element(),
            Block::Shown(title) => {
                let t = title.clone();
                let again = self.button(format!("shown-{key}-{i}"), if rtl { "عرض مرة أخرى" } else { "Show again" }, Kind::Normal, cx, move |a, _, cx| a.show_content_again(&t, cx));
                div().w_full().flex().justify_center().items_center().gap_2().text_sm().text_color(p.muted).child(format!("Opened \"{title}\" beside the chat.")).child(again).into_any_element()
            }
            Block::Notice(t) => div().w_full().flex().justify_center().text_sm().text_color(p.muted).child(t.clone()).into_any_element(),
            Block::Error(t) => div().px_3().py_2().rounded(p.inner).border_1().border_color(p.danger).text_color(p.danger).child(t.clone()).into_any_element(),
        }
    }

    /// Settings as a window over the app: the rest is dimmed, the window is centred, a click outside or Escape closes it.
    fn settings_view(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.settings_open {
            return None;
        }
        self.keep_width_slider_current(window, cx);
        let p = self.pal.clone();
        let rtl = self.rtl;
        self.tab_buttons = true;
        let body = self.settings_body(cx);
        let sidebar = self.settings_sidebar(cx);
        let close = self.button("settings-close", if rtl { "إغلاق" } else { "Close" }, Kind::Ghost, cx, |a, w, cx| a.close_settings(w, cx));
        self.tab_buttons = false;
        let shadow = BoxShadow { color: hsla(0., 0., 0., 0.4), offset: point(px(0.), px(14.)), blur_radius: px(44.), spread_radius: px(0.), inset: false };
        Some(
            div()
                .id("settings-scrim")
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex()
                .justify_center()
                .items_center()
                .p_4()
                .bg(hsla(0., 0., 0., scrim_alpha(p.dark)))
                .on_mouse_down(MouseButton::Left, cx.listener(|a, _, w, cx| a.close_settings(w, cx)))
                .child(
                    div()
                        .id("settings-window")
                        .role(Role::Dialog)
                        .aria_label(if rtl { "الإعدادات" } else { "Settings" })
                        .track_focus(&self.settings_focus)
                        .w(px(SETTINGS_WIDTH))
                        .max_w_full()
                        .h(relative(0.84))
                        .max_h(relative(0.9))
                        .flex()
                        .flex_col()
                        .gap_2()
                        .p_3()
                        .panel(&p, true)
                        .shadow(vec![shadow])
                        // clicks inside must not reach the dimmed area behind and close the window
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .child(div().flex().when(rtl, |d| d.flex_row_reverse()).items_center().gap_2().child(div().flex_1().text_color(p.text).child(if rtl { "الإعدادات" } else { "Settings" })).child(div().text_xs().text_color(p.muted).child(if rtl { "Esc للإغلاق" } else { "Esc closes" })).child(close))
                        .child(
                            div().flex().flex_1().min_h_0().when(rtl, |d| d.flex_row_reverse()).gap_3()
                                .child(div().flex().flex_col().flex_none().gap_2().child(div().w(px(168.)).child(Input::new(&self.settings_search))).child(sidebar))
                                .child(div().id("settings-body").flex_1().min_w_0().overflow_y_scroll().flex().flex_col().gap_2().child(body)),
                        ),
                )
                .into_any_element(),
        )
    }

    /// The command palette over everything else: a box to type in and the rows that match.
    fn palette_view(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let pal = self.palette.clone()?;
        let p = self.pal.clone();
        let rtl = self.rtl;
        let (entries, idx) = self.palette_rows();
        let range = pal.visible(idx.len(), 9);
        let rows: Vec<AnyElement> = range
            .map(|pos| {
                let e = entries[idx[pos]].clone();
                let on = pos == pal.selected;
                let command = e.command.clone();
                div()
                    .id(SharedString::from(format!("palette-row-{pos}")))
                    .role(Role::ListBoxOption)
                    .aria_label(format!("{}: {}", e.group, e.title))
                    .aria_selected(on)
                    .flex()
                    .when(rtl, |d| d.flex_row_reverse())
                    .items_center()
                    .gap_3()
                    .px_2()
                    .py_1p5()
                    .rounded(p.inner)
                    .cursor_pointer()
                    .bg(if on { p.accent.opacity(0.22) } else { hsla(0., 0., 0., 0.) })
                    .hover(|s| s.bg(p.button))
                    .on_click(cx.listener(move |a, _, w, cx| a.run_command(command.clone(), w, cx)))
                    .child(div().w(px(76.)).text_xs().text_color(p.muted).child(e.group))
                    .child(div().flex_1().overflow_hidden().text_color(p.text).when(rtl, |d| d.text_right()).child(e.title))
                    .child(div().text_xs().text_color(p.muted).child(e.hint))
                    .into_any_element()
            })
            .collect();
        let none = idx.is_empty().then(|| div().px_2().py_1p5().text_sm().text_color(p.muted).child(if rtl { "لا شيء يطابق ما كتبت." } else { "Nothing matches what you typed." }));
        Some(
            div()
                .id("palette")
                .role(Role::Dialog)
                .aria_label(if rtl { "لوحة الأوامر" } else { "Command palette" })
                .aria_keyshortcuts("Control+K")
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex()
                .justify_center()
                .items_start()
                .pt(px(90.))
                .bg(hsla(0., 0., 0., 0.28))
                .on_mouse_down(MouseButton::Left, cx.listener(|a, _, w, cx| a.close_palette(w, cx)))
                .capture_key_down(cx.listener(|a, ev: &KeyDownEvent, w, cx| {
                    let ks = &ev.keystroke;
                    let ctrl = ks.modifiers.control;
                    match ks.key.as_str() {
                        "down" => a.palette_step(true),
                        "up" => a.palette_step(false),
                        "n" if ctrl => a.palette_step(true),
                        "p" if ctrl => a.palette_step(false),
                        "escape" => a.close_palette(w, cx),
                        _ => return,
                    }
                    cx.stop_propagation();
                    cx.notify();
                }))
                .child(
                    div()
                        .w(px(560.))
                        .max_w_full()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .p_2()
                        .panel(&p, true)
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .child(Input::new(&self.palette_input))
                        .child(div().id("palette-list").role(Role::ListBox).aria_label(if rtl { "الأوامر" } else { "Commands" }).flex().flex_col().gap_0p5().children(rows).children(none)),
                )
                .into_any_element(),
        )
    }

    /// The model's words as plain text in the column, no bubble.
    fn answer_view(&self, key: u64, i: usize, text: &str, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let is_rtl = rtl::paragraph_is_rtl(text);
        let content = crate::panels::Content::detect(&self.chats.list.iter().find(|c| c.key == key).map(|c| c.title.clone()).unwrap_or_default(), text);
        let open_btn = self.button(format!("open-{key}-{i}"), "Open in panel", Kind::Ghost, cx, move |a, _, cx| a.open_content(content.clone(), cx));
        div().id(SharedString::from(format!("a{key}-{i}"))).role(Role::Label).w_full().flex().flex_col().gap_2().text_color(p.text).when(is_rtl, |d| d.text_right())
            .child(TextView::markdown(SharedString::from(format!("md{key}-{i}")), text.to_string()))
            .when(lh_gui::content::Content::worth_opening(text), |d| d.child(div().flex().when(is_rtl, |d| d.flex_row_reverse()).child(open_btn)))
            .into_any_element()
    }

    /// Thinking and tool calls as one line that opens to the list of steps.
    fn work_view(&self, key: u64, work: (usize, &[Step], Option<u64>, bool), blocks: &[Block], cx: &mut Context<Self>) -> AnyElement {
        let (first, steps, secs, running) = work;
        let p = self.pal.clone();
        let rtl = self.rtl;
        let open = self.open_blocks.contains(&(key, first));
        let label = work_label(secs, steps.len(), running, rtl);
        let head = div()
            .id(SharedString::from(format!("work-{key}-{first}")))
            .role(Role::Button)
            .aria_label(label.clone())
            .aria_expanded(open)
            .flex()
            .when(rtl, |d| d.flex_row_reverse())
            .items_center()
            .gap_1p5()
            .text_sm()
            .text_color(p.muted)
            .cursor_pointer()
            .hover(|s| s.text_color(p.text))
            .on_click(cx.listener(move |a, _, _, cx| {
                if !a.open_blocks.remove(&(key, first)) {
                    a.open_blocks.insert((key, first));
                }
                cx.notify();
            }))
            .child(icon_view(chevron(open, rtl), p.muted, 12.))
            .child(label);
        let mut rows: Vec<AnyElement> = vec![];
        if open {
            for s in steps {
                match (&s.kind, blocks.get(s.block)) {
                    (StepKind::Thinking, Some(Block::Assistant { reasoning, .. })) => {
                        rows.push(div().flex().flex_col().gap_0p5().child(div().text_color(p.text).child(if rtl { "تفكير" } else { "Thinking" })).child(div().child(reasoning.clone())).into_any_element());
                    }
                    (StepKind::Tool, Some(Block::Tool { name, result, .. })) => {
                        let at = s.block;
                        let shown = self.open_blocks.contains(&(key, at));
                        let body = result.clone().map(|r| r.chars().take(4000).collect::<String>()).unwrap_or_else(|| "running…".into());
                        let content = crate::panels::Content::detect(name, result.as_deref().unwrap_or(""));
                        let open_btn = self.button(format!("tool-open-{key}-{at}"), "Open in panel", Kind::Ghost, cx, move |a, _, cx| a.open_content(content.clone(), cx));
                        rows.push(
                            div().flex().flex_col().gap_1()
                                .child(
                                    div().id(SharedString::from(format!("step-{key}-{at}"))).role(Role::Button).aria_label(s.label.clone()).aria_expanded(shown).cursor_pointer().hover(|x| x.text_color(p.text))
                                        .on_click(cx.listener(move |a, _, _, cx| {
                                            if !a.open_blocks.remove(&(key, at)) {
                                                a.open_blocks.insert((key, at));
                                            }
                                            cx.notify();
                                        }))
                                        .child(s.label.clone()),
                                )
                                .when(shown, |d| d.child(div().text_color(p.text).child(body)).child(div().flex().child(open_btn)))
                                .into_any_element(),
                        );
                    }
                    _ => {}
                }
            }
        }
        div().flex().flex_col().gap_1p5()
            .child(div().flex().when(rtl, |d| d.flex_row_reverse()).child(head))
            .when(open, |d| d.child(div().id(SharedString::from(format!("steps-{key}-{first}"))).role(Role::List).aria_label(if rtl { "الخطوات" } else { "Steps" }).flex().flex_col().gap_1().p_2().rounded(p.inner).bg(p.sunken).text_sm().text_color(p.muted).font_family("DejaVu Sans Mono").children(rows)))
            .into_any_element()
    }

    fn pending_card(&mut self, pending: &Pending, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let msg = pending.warning["message"].as_str().unwrap_or("The core will read this chat again before it can continue.").to_string();
        let cloud = pending.warning["kind"] == "cloud";
        let title = if pending.warning.is_null() { "Checking what this costs…".to_string() } else if cloud { format!("Continue with {} in the cloud?", pending.model) } else { format!("Continue with {}?", pending.model) };
        let ok = self.button("resume-ok", if cloud { "Continue and spend credits" } else { "Continue" }, if cloud { Kind::Danger } else { Kind::Primary }, cx, |a, _, cx| a.confirm_resume(cx));
        let no = self.button("resume-no", "Cancel", Kind::Normal, cx, |a, _, cx| a.cancel_resume(cx));
        div().flex().flex_col().gap_2().p_3().panel(&p, false).border_1().border_color(p.accent).text_color(p.text)
            .id("resume-warning").role(Role::Group).aria_label("Continue this chat")
            .child(div().child(title)).child(div().text_sm().text_color(p.muted).child(msg))
            .when(!pending.warning.is_null(), |d| d.child(div().flex().gap_2().child(ok).child(no)).child(div().text_xs().text_color(p.muted).child("Ctrl+Enter continues, Escape cancels.")))
            .into_any_element()
    }

    fn chat_view(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let rtl = self.rtl;
        let Some(chat) = self.chats.current().cloned() else { return div().into_any_element() };
        let blocks: Vec<AnyElement> = transcript::items(&chat.blocks, chat.busy)
            .into_iter()
            .map(|item| match item {
                Item::Block(i) => self.block_view(chat.key, i, &chat.blocks[i], cx),
                Item::Answer(i) => match &chat.blocks[i] {
                    Block::Assistant { text, .. } => self.answer_view(chat.key, i, text, cx),
                    _ => div().into_any_element(),
                },
                Item::Work { first, steps, secs, running } => self.work_view(chat.key, (first, &steps, secs, running), &chat.blocks, cx),
            })
            .collect();
        let empty = chat.blocks.is_empty() && !chat.dormant;
        let mut stack: Vec<AnyElement> = vec![];
        if let Some(b) = &self.banner {
            stack.push(div().px_3().py_2().rounded(p.inner).border_1().border_color(p.danger).text_color(p.danger).child(b.clone()).into_any_element());
        }
        for a in chat.asks.clone() {
            let (id, id2, id3) = (a.id.clone(), a.id.clone(), a.id.clone());
            let (tool, reason) = (a.tool.clone(), a.reason.clone());
            let yes = self.button(format!("ask-yes-{id}"), if rtl { "نعم (Y)" } else { "Yes (Y)" }, Kind::Primary, cx, move |app, _, cx| app.answer(&id, true, cx));
            let no = self.button(format!("ask-no-{id2}"), if rtl { "لا (N)" } else { "No (N)" }, Kind::Normal, cx, move |app, _, cx| app.answer(&id2, false, cx));
            let always = may_offer(&a.reason).then(|| self.button(format!("ask-always-{id3}"), if rtl { "دائمًا في هذه المحادثة" } else { "Always for this chat" }, Kind::Ghost, cx, move |app, _, cx| app.answer_always(&id3, &tool, &reason, cx)));
            let question = if rtl { format!("السماح بتشغيل {}؟", a.tool) } else { format!("Allow {}?", a.tool) };
            stack.push(
                div().flex().flex_col().gap_2().p_3().panel(&p, false).border_1().border_color(p.accent).text_color(p.text)
                    .id(SharedString::from(format!("ask-{}", a.id))).role(Role::Group).aria_label(format!("Permission question. {question}")).aria_keyshortcuts("Y N")
                    .child(div().child(question))
                    .child(div().text_sm().text_color(p.muted).child(a.what.clone()))
                    .when(!a.reason.is_empty(), |d| d.child(div().text_sm().text_color(p.muted).child(a.reason.clone())))
                    .child(div().flex().when(rtl, |d| d.flex_row_reverse()).flex_wrap().gap_2().child(yes).child(no).children(always)).into_any_element(),
            );
        }
        if let Some(pen) = self.pending.clone().filter(|x| x.chat == chat.key) {
            stack.push(self.pending_card(&pen, cx));
        } else if chat.dormant {
            stack.push(div().text_sm().text_color(p.muted).child("This chat is stored. Choosing it asks before the model reads it again.").into_any_element());
        }
        // before the first message the chat can still be given its kind
        if chat.session.is_none() && chat.blocks.is_empty() && !chat.dormant {
            let kinds: Vec<AnyElement> = [(Mode::Chat, if rtl { "محادثة" } else { "Chat" }), (Mode::Assistant, if rtl { "مساعد" } else { "Assistant" }), (Mode::Project, if rtl { "مشروع" } else { "Project" })]
                .into_iter()
                .map(|(m, label)| self.button(format!("mode-{label}"), label, if chat.mode == m { Kind::Primary } else { Kind::Normal }, cx, move |a, _, cx| a.set_mode(m, cx)))
                .collect();
            let hint = match chat.mode {
                Mode::Chat => if chat.incognito { "Private chat: nothing is remembered or stored." } else { "Plain conversation, no tools." },
                Mode::Assistant => "Works from your home folder. It asks before it changes anything.",
                Mode::Project => "Works inside one folder. It asks before it changes anything.",
            };
            let private = (chat.mode == Mode::Chat).then(|| self.button("mode-private", if chat.incognito { "Private: on" } else { "Private: off" }, if chat.incognito { Kind::Primary } else { Kind::Ghost }, cx, |a, _, cx| a.toggle_private(cx)));
            stack.push(
                div().flex().flex_col().gap_2().p_3().panel(&p, false).text_color(p.text)
                    .id("chat-kind").role(Role::Group).aria_label("Kind of chat")
                    .child(div().flex().when(rtl, |d| d.flex_row_reverse()).gap_2().children(kinds).children(private))
                    .child(div().text_sm().text_color(p.muted).child(hint))
                    .when(chat.mode == Mode::Project, |d| d.child(Input::new(&self.project_input))).into_any_element(),
            );
        }
        let action = self.button("send", if chat.busy { if rtl { "إيقاف" } else { "Stop" } } else if rtl { "إرسال" } else { "Send" }, if chat.busy { Kind::Danger } else { Kind::Primary }, cx, |a, w, cx| a.send(w, cx));
        let composer = div().flex().when(rtl, |d| d.flex_row_reverse()).items_end().gap_2().p_2().panel(&p, false)
            .child(div().flex_1().child(Textarea::new(&self.composer).aria_label("Message")))
            .child(action);
        // the conversation reads in one column in the middle; the person chooses how wide
        let width = self.layout().column.max_px();
        let column = div().flex().flex_col().gap_4().w_full().when_some(width, |d, w| d.max_w(px(w)))
            .when(empty, |d| d.child(div().flex().justify_center().text_color(p.muted).child(if rtl { "ابدأ بكتابة رسالة." } else { "Write a message to begin." })))
            .children(blocks);
        let scroll = div().id("chat-scroll").role(Role::Log).aria_label("Conversation").flex_1().overflow_y_scroll().track_scroll(&self.scroll).flex().flex_col().items_center().p_3().child(column);
        // on glass the conversation sits on a panel of its own, so text never lands straight on the sky
        // the view stays where the person scrolled to while text streams in; a button takes them to the newest line
        let max = self.scroll.max_offset().y;
        self.follow.observe(self.scroll.offset().y.into(), max.into());
        let jump = self.follow.wants_jump_button().then(|| {
            self.tab_buttons = true;
            let b = self.button("jump-to-latest", if rtl { "إلى الأحدث" } else { "Jump to latest" }, Kind::Primary, cx, |a, w, cx| {
                a.scroll_to_end();
                a.composer.update(cx, |s, cx| s.focus(w, cx));
                cx.notify();
            });
            self.tab_buttons = false;
            div().absolute().bottom_2().w_full().flex().justify_center().child(b)
        });
        let pane = div().relative().flex().flex_col().flex_1().overflow_hidden().when(p.glass.is_some(), |d| d.panel(&p, self.focus == Focus::Chat)).child(scroll).children(jump);
        // questions and the message box stay in view under the conversation, in the same column
        let beneath = div().flex().flex_col().items_center().px_3().child(div().flex().flex_col().gap_2().w_full().when_some(width, |d, w| d.max_w(px(w))).children(stack).child(composer));
        div().flex().flex_col().flex_1().h_full().gap_2().overflow_hidden()
            .on_mouse_down(MouseButton::Left, cx.listener(|a, _, _, cx| a.focus_on(Focus::Chat, cx)))
            .child(pane)
            .child(beneath)
            .into_any_element()
    }
}

impl Render for App {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.pal.clone();
        let rtl = self.rtl;
        if let Some(t) = self.pending_input.take() {
            self.composer.update(cx, |s, cx| s.set_value(t, window, cx));
        }
        let asking = self.asking(cx);
        let total: f32 = window.viewport_size().width.into();
        let (left_w, right_w) = self.layout().widths(total - 24. - RAIL_WIDTH, 420.);
        self.width.room = self.layout().middle_room(total - 24. - RAIL_WIDTH, 420.);
        let rail = self.rail(cx);
        let top = self.top_bar(cx);
        let menus = self.menus(cx);
        // the chat list and the side panel slide (ease out, 220 ms); with no animation they simply appear
        let (now, dur) = (std::time::Instant::now(), motion::panel(p.anim));
        let (left_now, right_now) = (self.left_reveal.width(left_w, now, dur), self.right_reveal.width(right_w, now, dur));
        if self.left_reveal.animating(now, dur) || self.right_reveal.animating(now, dur) {
            window.request_animation_frame();
        }
        let (left_full, right_full) = (self.left_reveal.full(), self.right_reveal.full());
        let left = (left_now > 0.).then(|| slide(left_now, left_full, self.sidebar(left_full, cx)));
        let middle = match self.page().clone() {
            Page::Chat => self.chat_view(cx),
            Page::Department(name) => self.department_page(&name, cx),
        };
        let right = (right_now > 0.).then(|| slide(right_now, right_full, self.right_panel(right_full, cx)));
        let root = p.paint_background(div().size_full().relative().overflow_hidden().text_color(p.text))
            .key_context(if asking { "LastHarness Asking" } else { "LastHarness" })
            .on_action(cx.listener(|a, _: &NewChat, _, cx| a.new_chat(cx)))
            .on_action(cx.listener(|a, _: &ToggleSidebar, _, cx| {
                a.layout_mut().toggle_left();
                a.persist();
                cx.notify();
            }))
            .on_action(cx.listener(|a, _: &ShowDepartments, _, cx| a.open_panel(Panel::Departments, cx)))
            .on_action(cx.listener(|a, _: &ShowMemory, _, cx| a.open_panel(Panel::Memory, cx)))
            .on_action(cx.listener(|a, _: &ShowSettings, w, cx| a.toggle_settings(w, cx)))
            .on_action(cx.listener(|a, _: &ShowContent, _, cx| a.open_panel(Panel::Content, cx)))
            .on_action(cx.listener(|a, _: &ShowTree, _, cx| a.open_panel(Panel::Tree, cx)))
            .on_action(cx.listener(|a, _: &ToggleExpand, _, cx| {
                a.layout_mut().toggle_expand();
                a.persist();
                cx.notify();
            }))
            .on_action(cx.listener(|a, _: &ToggleMode, w, cx| a.toggle_mode(w, cx)))
            .on_action(cx.listener(|a, _: &CycleDirection, _, cx| {
                let n = a.prefs.direction.next();
                a.set_direction(n);
                cx.notify();
            }))
            .on_action(cx.listener(|a, _: &OpenPalette, w, cx| a.toggle_palette(w, cx)))
            .on_action(cx.listener(|a, _: &StopTurn, _, cx| a.stop(cx)))
            .on_action(cx.listener(|a, _: &AllowAsk, _, cx| a.answer_first(true, cx)))
            .on_action(cx.listener(|a, _: &DenyAsk, _, cx| a.answer_first(false, cx)))
            .on_action(cx.listener(|a, _: &ConfirmResume, _, cx| {
                if a.pending.as_ref().is_some_and(|p| !p.warning.is_null()) {
                    a.confirm_resume(cx);
                }
            }))
            .on_action(cx.listener(|a, _: &JumpToLatest, _, cx| {
                a.scroll_to_end();
                cx.notify();
            }))
            .on_action(cx.listener(|a, _: &OpenLastReply, _, cx| {
                // the newest reply or tool result of this chat, in the side panel
                let found = a.chats.current().and_then(|c| c.blocks.iter().rev().find_map(|b| match b {
                    Block::Assistant { text, .. } if !text.is_empty() => Some(crate::panels::Content::detect(&c.title, text)),
                    Block::Tool { name, result: Some(r), .. } => Some(crate::panels::Content::detect(name, r)),
                    _ => None,
                }));
                if let Some(c) = found {
                    a.open_content(c, cx);
                }
            }))
            .on_action(cx.listener(|a, _: &PopOutContent, _, cx| {
                if let Some(c) = a.content.get(a.content_index).cloned() {
                    a.pop_out(c, cx);
                }
            }))
            .on_action(cx.listener(|a, _: &Dismiss, w, cx| {
                // Escape closes whatever is on top: the palette, Settings, a menu, then a question
                match topmost(a.palette.is_some(), a.settings_open, a.model_menu, a.pending.is_some(), a.confirm_delete.is_some()) {
                    Layer::Palette => a.close_palette(w, cx),
                    Layer::Settings => a.close_settings(w, cx),
                    Layer::ModelMenu => a.model_menu = false,
                    Layer::ResumeQuestion => a.pending = None,
                    Layer::DeleteQuestion => a.confirm_delete = None,
                    Layer::Nothing => {}
                }
                cx.notify();
            }))
            .on_action(cx.listener(|a, _: &NextChat, _, cx| {
                let i = (a.chats.active + 1).min(a.chats.list.len().saturating_sub(1));
                if let Some(k) = a.chats.list.get(i).map(|c| c.key) {
                    a.select_chat(k, cx);
                }
            }))
            .on_action(cx.listener(|a, _: &PrevChat, _, cx| {
                let i = a.chats.active.saturating_sub(1);
                if let Some(k) = a.chats.list.get(i).map(|c| c.key) {
                    a.select_chat(k, cx);
                }
            }));
        // the rail, the chat list, the chat under its title bar, and the side panel; right to left mirrors the order
        let main = div().relative().flex().flex_col().flex_1().h_full().overflow_hidden().child(top).child(div().flex().flex_col().flex_1().overflow_hidden().child(middle)).children(menus);
        let settings = self.settings_view(window, cx);
        let palette = self.palette_view(cx);
        root.children(p.image_layer())
            .children(p.motes_layer(self.mote_clock))
            .child(
                div().size_full().flex().when(rtl, |d| d.flex_row_reverse()).gap_2().p_2().overflow_hidden().child(rail).children(left).child(main).children(right),
            )
            .children(settings)
            .children(palette)
    }
}
