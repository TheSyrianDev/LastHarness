//! Drawing: the window as the theme describes it. Reads the state in `App`, never changes it except through
//! the click handlers it attaches.

use gpui::prelude::*;
use gpui::*;
use gpui_component::input::{Input, Textarea};
use gpui_component::text::TextView;

use crate::app::{App, Focus, Pending};
use lh_gui::layout::{Page, Panel};
use lh_gui::model::{Block, Mode};
use lh_gui::rtl;
use crate::style::{mix, Look, PanelExt};

actions!(lastharness, [NewChat, ToggleSidebar, ShowDepartments, ShowMemory, ShowSettings, ShowContent, ShowTree, NextTheme, CycleDirection, StopTurn, NextChat, PrevChat, ToggleExpand, ConfirmResume, Dismiss, OpenLastReply, PopOutContent]);

/// The keys for every action; also how the app is driven without a mouse.
pub fn bind_keys(cx: &mut gpui::App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-n", NewChat, None),
        KeyBinding::new("ctrl-b", ToggleSidebar, None),
        KeyBinding::new("ctrl-shift-d", ShowDepartments, None),
        KeyBinding::new("ctrl-shift-m", ShowMemory, None),
        KeyBinding::new("ctrl-shift-s", ShowSettings, None),
        KeyBinding::new("ctrl-shift-o", ShowContent, None),
        KeyBinding::new("ctrl-shift-t", ShowTree, None),
        KeyBinding::new("ctrl-shift-e", ToggleExpand, None),
        KeyBinding::new("ctrl-t", NextTheme, None),
        KeyBinding::new("ctrl-shift-r", CycleDirection, None),
        KeyBinding::new("ctrl-.", StopTurn, None),
        KeyBinding::new("ctrl-enter", ConfirmResume, None),
        KeyBinding::new("escape", Dismiss, None),
        KeyBinding::new("ctrl-shift-l", OpenLastReply, None),
        KeyBinding::new("ctrl-shift-p", PopOutContent, None),
        KeyBinding::new("ctrl-pagedown", NextChat, None),
        KeyBinding::new("ctrl-pageup", PrevChat, None),
    ]);
}

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Normal,
    Primary,
    Danger,
    Ghost,
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
        let base_el = div().id(id.clone()).role(Role::Button).aria_label(aria).px_3().py_1p5().rounded(p.inner).text_color(fg).cursor_pointer().on_hover(on_hover).on_click(on_click);

        // gel: a glossy skin under the label, a rim light, a glow under the pointer and a dark veil while pressed
        if let Some(g) = &p.gel {
            let (rest, over) = g.looks(p.danger, kind == Kind::Ghost, kind == Kind::Primary, kind == Kind::Danger);
            let (from, to) = if hovered { (rest, over) } else { (over, rest) };
            let (radius, press) = (p.inner, g.press);
            let group = SharedString::from(format!("gel-{id}"));
            let skin = g.skin(radius, if hovered { over } else { rest });
            let skin = if animate {
                let g2 = g.clone();
                skin.with_animation(anim_key, Animation::new(self.pal.anim).with_easing(ease_in_out), move |_, d| g2.skin(radius, Look::mix(from, to, d))).into_any_element()
            } else {
                skin.into_any_element()
            };
            let veil = div().id(SharedString::from(format!("veil-{id}"))).absolute().top_0().left_0().size_full().rounded(radius).group_active(group.clone(), move |s| s.bg(hsla(0.6, 0.6, 0.1, press)));
            return base_el.group(group).child(skin).child(veil).child(label).into_any_element();
        }
        let el = base_el.bg(if hovered { hover } else { base }).child(label);
        if !animate {
            return el.into_any_element();
        }
        let (from, to) = if hovered { (base, hover) } else { (hover, base) };
        el.with_animation(anim_key, Animation::new(self.pal.anim).with_easing(ease_in_out), move |el, d| el.bg(mix(from, to, d))).into_any_element()
    }

    fn top_bar(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let rtl = self.rtl;
        let chat = self.chats.current().cloned();
        let title = chat.as_ref().map(|c| c.title.clone()).unwrap_or_default();
        let model = chat.as_ref().map(|c| c.model.clone()).unwrap_or_default();
        let ctx = chat.as_ref().and_then(|c| c.context_tokens).map(|n| format!("{n} tokens of context"));
        let l = self.layout().clone();
        let tabs: Vec<AnyElement> = [(Panel::Departments, if rtl { "الأقسام" } else { "Departments" }), (Panel::Memory, if rtl { "الذاكرة" } else { "Memory" }), (Panel::Settings, if rtl { "الإعدادات" } else { "Settings" })]
            .into_iter()
            .map(|(panel, label)| self.button(format!("tab-{label}"), label, if l.right_visible && l.panel == panel { Kind::Primary } else { Kind::Ghost }, cx, move |a, _, cx| a.open_panel(panel, cx)))
            .collect();
        let opened = (!self.content.is_empty()).then(|| self.button("tab-opened", if rtl { "المفتوح" } else { "Opened" }, if l.right_visible && l.panel == Panel::Content { Kind::Primary } else { Kind::Ghost }, cx, |a, _, cx| a.open_panel(Panel::Content, cx)));
        let branches = self.button("tab-tree", if rtl { "الفروع" } else { "Branches" }, if l.right_visible && l.panel == Panel::Tree { Kind::Primary } else { Kind::Ghost }, cx, |a, _, cx| a.open_panel(Panel::Tree, cx));
        let toggle_left = self.button("toggle-left", if rtl { "المحادثات" } else { "Chats" }, if l.left_visible { Kind::Normal } else { Kind::Ghost }, cx, |a, _, cx| {
            a.layout_mut().toggle_left();
            a.persist();
            cx.notify();
        });
        let model_btn = self.button("model", format!("{} ▾", if model.is_empty() { "no model" } else { &model }), Kind::Normal, cx, |a, _, cx| {
            a.model_menu = !a.model_menu;
            a.theme_menu = false;
            cx.notify();
        });
        let theme_label = self.themes.themes[self.active_theme].label();
        let theme_btn = self.button("theme", theme_label, Kind::Ghost, cx, |a, _, cx| {
            a.theme_menu = !a.theme_menu;
            a.model_menu = false;
            cx.notify();
        });
        let dir_label = match self.prefs.direction {
            rtl::Direction::Auto => "Auto",
            rtl::Direction::Ltr => "LTR",
            rtl::Direction::Rtl => "RTL",
        };
        let dir_btn = self.button("direction", dir_label, Kind::Ghost, cx, |a, _, cx| {
            let n = a.prefs.direction.next();
            a.set_direction(n);
            cx.notify();
        });
        div()
            .flex()
            .when(rtl, |d| d.flex_row_reverse())
            .items_center()
            .gap_2()
            .p_2()
            // over a sky the title needs glass behind it
            .when(p.glass.is_some(), |d| d.m_2().mb_0().panel(&p, false))
            .child(toggle_left)
            .child(div().flex_1().overflow_hidden().text_color(p.text).child(title))
            .when_some(ctx, |d, c| d.child(div().text_sm().text_color(p.muted).child(c)))
            .child(model_btn)
            .children(opened)
            .child(branches)
            .children(tabs)
            .child(theme_btn)
            .child(dir_btn)
            .into_any_element()
    }

    fn menus(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let p = self.pal.clone();
        let items: Vec<AnyElement> = if self.model_menu {
            self.models.names().to_vec().into_iter().enumerate().map(|(i, m)| {
                let name = m.clone();
                let cur = self.chats.current().map(|c| c.model == m).unwrap_or(false);
                self.button(format!("model-{i}"), m, if cur { Kind::Primary } else { Kind::Ghost }, cx, move |a, _, cx| a.choose_model(&name, cx))
            }).collect()
        } else if self.theme_menu {
            (0..self.themes.themes.len()).map(|i| {
                let label = self.themes.themes[i].label();
                self.button(format!("theme-{i}"), label, if i == self.active_theme { Kind::Primary } else { Kind::Ghost }, cx, move |a, w, cx| a.set_theme(i, w, cx))
            }).collect()
        } else {
            return None;
        };
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
                    div().id(SharedString::from(format!("u{key}-{i}"))).role(Role::Label).max_w(px(680.)).px_3().py_2().rounded(p.inner).bg(p.accent.opacity(0.18)).text_color(p.text).when(is_rtl, |d| d.text_right()).child(t.clone()),
                ).into_any_element()
            }
            Block::Assistant { text, reasoning } => {
                let is_rtl = rtl::paragraph_is_rtl(text);
                let open = self.open_blocks.contains(&(key, i));
                let think = (!reasoning.is_empty()).then(|| {
                    let r = reasoning.clone();
                    div().flex().flex_col().gap_1().child(self.button(format!("think-{key}-{i}"), if open { "Hide thinking" } else { "Show thinking" }, Kind::Ghost, cx, move |a, _, cx| {
                        if !a.open_blocks.remove(&(key, i)) {
                            a.open_blocks.insert((key, i));
                        }
                        cx.notify();
                    })).when(open, |d| d.child(div().px_2().text_sm().text_color(p.muted).child(r)))
                });
                let content = crate::panels::Content::detect(&self.chats.list.iter().find(|c| c.key == key).map(|c| c.title.clone()).unwrap_or_default(), text);
                let open_btn = self.button(format!("open-{key}-{i}"), "Open in panel", Kind::Ghost, cx, move |a, _, cx| a.open_content(content.clone(), cx));
                div().flex().w_full().child(
                    div().id(SharedString::from(format!("a{key}-{i}"))).role(Role::Label).max_w(px(760.)).px_3().py_2().rounded(p.inner).bg(p.sunken).text_color(p.text).when(is_rtl, |d| d.text_right())
                        .children(think)
                        .child(TextView::markdown(SharedString::from(format!("md{key}-{i}")), text.clone()))
                        .when(text.len() > 400, |d| d.child(div().flex().child(open_btn))),
                ).into_any_element()
            }
            Block::Tool { name, arguments, result, .. } => {
                let open = self.open_blocks.contains(&(key, i));
                let head = format!("{name} {}", arguments.chars().take(90).collect::<String>());
                let toggle = self.button(format!("tool-{key}-{i}"), if open { "Hide" } else { "Details" }, Kind::Ghost, cx, move |a, _, cx| {
                    if !a.open_blocks.remove(&(key, i)) {
                        a.open_blocks.insert((key, i));
                    }
                    cx.notify();
                });
                let body = result.clone().map(|r| r.chars().take(4000).collect::<String>()).unwrap_or_else(|| "running…".into());
                let content = crate::panels::Content::detect(name, result.as_deref().unwrap_or(""));
                let open_btn = self.button(format!("tool-open-{key}-{i}"), "Open in panel", Kind::Ghost, cx, move |a, _, cx| a.open_content(content.clone(), cx));
                div().flex().flex_col().gap_1().px_3().py_2().rounded(p.inner).border_1().border_color(p.border).text_sm().text_color(p.muted)
                    .child(div().flex().items_center().gap_2().child(div().flex_1().child(head)).child(toggle))
                    .when(open, |d| d.child(div().text_color(p.text).child(body)).child(div().flex().child(open_btn)))
                    .into_any_element()
            }
            Block::Shown(title) => {
                let t = title.clone();
                let again = self.button(format!("shown-{key}-{i}"), if rtl { "عرض مرة أخرى" } else { "Show again" }, Kind::Normal, cx, move |a, _, cx| a.show_content_again(&t, cx));
                div().w_full().flex().justify_center().items_center().gap_2().text_sm().text_color(p.muted).child(format!("Opened \"{title}\" beside the chat.")).child(again).into_any_element()
            }
            Block::Notice(t) => div().w_full().flex().justify_center().text_sm().text_color(p.muted).child(t.clone()).into_any_element(),
            Block::Error(t) => div().px_3().py_2().rounded(p.inner).border_1().border_color(p.danger).text_color(p.danger).child(t.clone()).into_any_element(),
        }
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
        let blocks: Vec<AnyElement> = chat.blocks.iter().enumerate().map(|(i, b)| self.block_view(chat.key, i, b, cx)).collect();
        let empty = chat.blocks.is_empty() && !chat.dormant;
        let mut stack: Vec<AnyElement> = vec![];
        if let Some(b) = &self.banner {
            stack.push(div().px_3().py_2().rounded(p.inner).border_1().border_color(p.danger).text_color(p.danger).child(b.clone()).into_any_element());
        }
        for a in chat.asks.clone() {
            let id = a.id.clone();
            let id2 = a.id.clone();
            let yes = self.button(format!("ask-yes-{id}"), if rtl { "السماح" } else { "Allow" }, Kind::Primary, cx, move |app, _, cx| app.answer(&id, true, cx));
            let no = self.button(format!("ask-no-{id2}"), if rtl { "رفض" } else { "Deny" }, Kind::Normal, cx, move |app, _, cx| app.answer(&id2, false, cx));
            stack.push(
                div().flex().flex_col().gap_2().p_3().panel(&p, false).border_1().border_color(p.accent).text_color(p.text)
                    .child(div().child(format!("May {} run?", a.tool)))
                    .child(div().text_sm().text_color(p.muted).child(a.what.clone()))
                    .when(!a.reason.is_empty(), |d| d.child(div().text_sm().text_color(p.muted).child(a.reason.clone())))
                    .child(div().flex().gap_2().child(yes).child(no)).into_any_element(),
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
        let scroll = div().id("chat-scroll").role(Role::Log).aria_label("Conversation").flex_1().overflow_y_scroll().track_scroll(&self.scroll).flex().flex_col().gap_3().p_3()
            .when(empty, |d| d.child(div().flex().justify_center().text_color(p.muted).child(if rtl { "ابدأ بكتابة رسالة." } else { "Write a message to begin." })))
            .children(blocks);
        // on glass the conversation sits on a panel of its own, so text never lands straight on the sky
        let pane = div().flex().flex_col().flex_1().overflow_hidden().when(p.glass.is_some(), |d| d.panel(&p, self.focus == Focus::Chat)).child(scroll);
        div().flex().flex_col().flex_1().h_full().gap_2().overflow_hidden()
            .on_mouse_down(MouseButton::Left, cx.listener(|a, _, _, cx| a.focus_on(Focus::Chat, cx)))
            .child(pane)
            .children(stack)
            .child(composer)
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
        let total: f32 = window.viewport_size().width.into();
        let (left_w, right_w) = self.layout().widths(total - 24., 420.);
        let top = self.top_bar(cx);
        let menus = self.menus(cx);
        let left = (left_w > 0.).then(|| self.sidebar(left_w, cx));
        let middle = match self.page().clone() {
            Page::Chat => self.chat_view(cx),
            Page::Department(name) => self.department_page(&name, cx),
        };
        let right = (right_w > 0.).then(|| self.right_panel(right_w, cx));
        let root = p.paint_background(div().size_full().relative().overflow_hidden().text_color(p.text))
            .key_context("LastHarness")
            .on_action(cx.listener(|a, _: &NewChat, _, cx| a.new_chat(cx)))
            .on_action(cx.listener(|a, _: &ToggleSidebar, _, cx| {
                a.layout_mut().toggle_left();
                a.persist();
                cx.notify();
            }))
            .on_action(cx.listener(|a, _: &ShowDepartments, _, cx| a.open_panel(Panel::Departments, cx)))
            .on_action(cx.listener(|a, _: &ShowMemory, _, cx| a.open_panel(Panel::Memory, cx)))
            .on_action(cx.listener(|a, _: &ShowSettings, _, cx| a.open_panel(Panel::Settings, cx)))
            .on_action(cx.listener(|a, _: &ShowContent, _, cx| a.open_panel(Panel::Content, cx)))
            .on_action(cx.listener(|a, _: &ShowTree, _, cx| a.open_panel(Panel::Tree, cx)))
            .on_action(cx.listener(|a, _: &ToggleExpand, _, cx| {
                a.layout_mut().toggle_expand();
                a.persist();
                cx.notify();
            }))
            .on_action(cx.listener(|a, _: &NextTheme, w, cx| {
                let n = (a.active_theme + 1) % a.themes.themes.len();
                a.set_theme(n, w, cx);
            }))
            .on_action(cx.listener(|a, _: &CycleDirection, _, cx| {
                let n = a.prefs.direction.next();
                a.set_direction(n);
                cx.notify();
            }))
            .on_action(cx.listener(|a, _: &StopTurn, _, cx| a.stop(cx)))
            .on_action(cx.listener(|a, _: &ConfirmResume, _, cx| {
                if a.pending.as_ref().is_some_and(|p| !p.warning.is_null()) {
                    a.confirm_resume(cx);
                }
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
            .on_action(cx.listener(|a, _: &Dismiss, _, cx| {
                // Escape closes whatever is open on top: a menu, then the question about continuing a chat
                if a.model_menu || a.theme_menu {
                    a.model_menu = false;
                    a.theme_menu = false;
                } else if a.pending.is_some() {
                    a.pending = None;
                } else if a.confirm_delete.is_some() {
                    a.confirm_delete = None;
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
        root.children(p.image_layer())
            .children(p.motes_layer(self.mote_clock))
            .child(
                div().size_full().flex().flex_col().child(top).child(
                    div().flex_1().flex().when(rtl, |d| d.flex_row_reverse()).gap_2().px_2().pb_2().overflow_hidden().children(left).child(div().flex().flex_col().flex_1().overflow_hidden().child(middle)).children(right),
                ),
            )
            .children(menus)
    }
}
