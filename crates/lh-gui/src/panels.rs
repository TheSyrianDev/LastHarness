//! The right panel (departments, memory, settings, opened content) and a department's own page.

use gpui::prelude::*;
use gpui::*;
use gpui_component::input::Textarea;
use gpui_component::Disableable as _;
use gpui_component::text::TextView;

use crate::app::App;
use lh_gui::chrome::{panel_label, DOCK_TABS};
use lh_gui::layout::Panel;
use lh_gui::runs::{Outcome, Question, Run, StepState};
use lh_gui::settings;
use crate::style::PanelExt;
use crate::ui::Kind;

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
    pub fn right_panel(&mut self, width: f32, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let rtl = self.rtl;
        let panel = self.layout().panel;
        let expanded = self.layout().right_expanded;
        let title = panel_label(panel, rtl);
        let widen = self.button("panel-widen", if expanded { if rtl { "تضييق" } else { "Narrow" } } else if rtl { "توسيع" } else { "Widen" }, Kind::Ghost, cx, |a, _, cx| {
            a.layout_mut().cycle_width();
            a.persist();
            cx.notify();
        });
        let popout = (panel == Panel::Content && !self.content.is_empty()).then(|| {
            let item = self.content[self.content_index.min(self.content.len() - 1)].clone();
            self.button("panel-popout", if rtl { "فصل في نافذة" } else { "Pop out" }, Kind::Ghost, cx, move |a, _, cx| a.pop_out(item.clone(), cx))
        });
        let hide = self.button("panel-hide", if rtl { "إخفاء" } else { "Hide" }, Kind::Ghost, cx, |a, _, cx| {
            a.layout_mut().hide_right();
            a.persist();
            cx.notify();
        });
        let body = match panel {
            Panel::Departments => self.departments_body(cx),
            Panel::Memory => self.memory_body(cx),
            Panel::Content => self.content_body(window, cx),
            Panel::Tree => self.tree_body(cx),
        };
        // the tabs, then the size controls beneath them
        let tabs: Vec<AnyElement> = DOCK_TABS
            .into_iter()
            .map(|t| self.button(format!("dock-{}", panel_label(t, false)), panel_label(t, rtl), if t == panel { Kind::Primary } else { Kind::Ghost }, cx, move |a, _, cx| a.show_in_dock(t, cx)))
            .collect();
        let tab_row = div().id("dock-tabs").role(Role::TabList).aria_label(if rtl { "أقسام اللوحة" } else { "Side panel sections" }).flex().when(rtl, |d| d.flex_row_reverse()).flex_wrap().gap_1().children(tabs);
        let controls = div().id("dock-controls").flex().when(rtl, |d| d.flex_row_reverse()).items_center().gap_1().child(div().flex_1()).child(widen).children(popout).child(hide);
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
            .panel(&p, self.focus == crate::app::Focus::Side)
            .on_mouse_down(MouseButton::Left, cx.listener(|a, _, _, cx| a.focus_on(crate::app::Focus::Side, cx)))
            .child(tab_row)
            .child(controls)
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
    fn appearance_block(&mut self, cx: &mut Context<Self>) -> AnyElement {
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
        let dark_switch = gpui_component::switch::Switch::new("dark-mode-switch")
            .checked(sw.dark_on)
            .disabled(!sw.dark_enabled)
            .label(if rtl { "الوضع الداكن" } else { "Dark mode" })
            .on_click(cx.listener(|a, on: &bool, w, cx| a.set_appearance(lh_gui::appearance::after_dark_switch(*on), w, cx)));
        let match_switch = gpui_component::switch::Switch::new("match-desktop-switch")
            .checked(sw.match_on)
            .disabled(!sw.match_enabled)
            .label(if rtl { "مطابقة سطح المكتب" } else { "Match the desktop" })
            .on_click(cx.listener(|a, on: &bool, w, cx| {
                let dark = a.dark();
                a.set_appearance(lh_gui::appearance::after_match_switch(*on, dark), w, cx)
            }));
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
            .child(self.font_picker_row(crate::app::FontSlot::Arabic, cx));
        let pages_switch = gpui_component::switch::Switch::new("pages-inside-switch")
            .checked(!self.prefs.pages_off)
            .label(if rtl { "عرض صفحات الويب داخل التطبيق" } else { "Show web pages inside the app" })
            .on_click(cx.listener(|a, on: &bool, _, cx| {
                a.prefs.pages_off = !*on;
                a.persist();
                cx.notify();
            }));
        let pages_note = div().text_xs().text_color(p.muted).child(if rtl { "لا تستطيع أي صفحة تحميل شيء من الإنترنت أو من هذا الحاسوب، ولا تعمل فيها سكربتات. الصفحات الأكبر من 2 ميغابايت تظهر كبطاقة." } else { "No page can load anything from the internet or from this computer, and no scripts run. Pages over 2 MB show as a card instead." });
        div().flex().flex_col().gap_2().p_2().rounded(p.inner).bg(p.sunken).text_color(p.text)
                .id("appearance").role(Role::Group).aria_label("Appearance")
                .child(modes)
                .child(looks)
                .child(widths)
                .child(fonts)
                .child(div().flex().flex_col().gap_1().child(pages_switch).child(pages_note))
                .child(div().flex().flex_wrap().gap_1().child(dir_btn).child(narrower).child(wider))
                .child(div().text_xs().text_color(p.muted).child(format!("Your own looks go in {} as .toml files; changes show at once.", self.cfg.themes.display())))
                .children(self.themes.problems.iter().map(|e| div().text_xs().text_color(p.danger).child(format!("Theme file problem: {e}"))))
            .children(self.themes.notice.clone().map(|n| div().text_xs().text_color(p.muted).child(n)))
            .children(self.look_notice.clone().map(|n| div().text_xs().text_color(p.muted).child(n)))
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
        let shown = match slot {
            FontSlot::Latin if chosen.is_empty() => (if rtl { "الافتراضي" } else { "Default" }).to_string(),
            FontSlot::Arabic if chosen.is_empty() => lh_gui::fonts::arabic_font("", &self.fonts).unwrap_or_else(|| (if rtl { "افتراضي النظام" } else { "System default" }).to_string()),
            _ => chosen.clone(),
        };
        let open = self.font_picker == Some(slot);
        let toggle = self.button(format!("font-{tag}"), format!("{title}: {shown}"), Kind::Normal, cx, move |a, _, cx| {
            a.font_picker = if a.font_picker == Some(slot) { None } else { Some(slot) };
            cx.notify();
        });
        let mut col = div().flex().flex_col().gap_1().child(toggle);
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
                    let label = if n.is_empty() { (if rtl { "الافتراضي" } else { "Default" }).to_string() } else { n.clone() };
                    let kind = if n == chosen { Kind::Primary } else { Kind::Ghost };
                    self.button(format!("font-{tag}-{i}"), label, kind, cx, move |a, _, cx| {
                        a.set_font(slot, &n);
                        cx.notify();
                    })
                })
                .collect();
            col = col.child(div().id(SharedString::from(format!("font-list-{tag}"))).role(Role::List).aria_label(title).max_h(px(220.)).overflow_y_scroll().flex().flex_col().gap_1().p_1().rounded(p.inner).bg(p.sunken).children(rows));
        }
        col.into_any_element()
    }

    /// One setting as a card: its plain title, one sentence, the control, and the value in words.
    fn setting_card(&mut self, i: usize, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let s = self.panels.settings[i].clone();
        let d = lh_gui::setting_names::describe(&s.key, &s.help);
        let words = lh_gui::setting_names::value_text(&s.key, &s.value);
        let control: AnyElement = match &s.kind {
            settings::Kind::Bool | settings::Kind::Choice(_) => {
                let (key, cur, kind) = (s.key.clone(), s.value.clone(), s.kind.clone());
                self.button(format!("set-{}", s.key), words.clone(), Kind::Normal, cx, move |a, _, cx| {
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
                div().flex().items_center().gap_1().child(minus).child(div().min_w(px(36.)).flex().justify_center().child(words.clone())).child(plus).into_any_element()
            }
            settings::Kind::Other => div().text_color(p.muted).child(words.clone()).into_any_element(),
        };
        let tip = format!("Name in the settings file: {}", s.key);
        let aria = format!("{}: {}", d.title, words);
        let rtl = self.rtl;
        div().flex().flex_col().gap_1().p_2().rounded(p.inner).bg(p.sunken).text_color(p.text)
            .when(d.careful, |c| c.border_1().border_color(p.danger.opacity(0.55)))
            .id(SharedString::from(format!("setting-{}", s.key))).role(Role::Group).aria_label(aria).aria_description(d.about.clone())
            .tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(tip.clone()).build(window, cx))
            .child(div().flex().when(rtl, |r| r.flex_row_reverse()).items_center().gap_2().child(div().flex_1().child(d.title.clone())).child(control))
            .child(div().text_xs().text_color(p.muted).child(d.about.clone()))
            .when(d.careful, |c| c.child(div().text_xs().text_color(p.danger).child(if rtl { "إعداد حساس، غيّره بحذر." } else { "Take care with this one." })))
            .when(!s.kind.editable(), |c| c.child(div().text_xs().text_color(p.muted).child("To change this, run lh settings in a terminal.")))
            .into_any_element()
    }

    /// The page for the chosen category, or the settings that match what is typed in the search box.
    pub fn settings_body(&mut self, cx: &mut Context<Self>) -> AnyElement {
        use lh_gui::setting_names::{appearance_matches, describe, matches, Category};
        let p = self.pal.clone();
        let mut col = div().flex().flex_col().gap_2();
        if let Some(e) = self.panels.settings_error.clone() {
            col = col.child(div().px_2().py_1().rounded(p.inner).border_1().border_color(p.danger).text_color(p.danger).child(e));
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
            col = col.child(div().text_color(p.text).child(page.title())).child(div().text_xs().text_color(p.muted).child(page.blurb()));
        }
        if show_appearance {
            col = col.child(self.appearance_block(cx));
        }
        if !self.connected && !show_appearance {
            return col.child(self.note("The core is not running, so these settings cannot be shown.")).into_any_element();
        }
        for i in shown.iter().copied() {
            col = col.child(self.setting_card(i, cx));
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
            .w(px(168.))
            .flex_none()
            .flex()
            .flex_col()
            .gap_1()
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
            let danger = matches!(c, lh_gui::setting_names::Category::Permissions | lh_gui::setting_names::Category::Spending);
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
                    .text_color(if on { p.accent_text } else { p.text })
                    .bg(if on { p.accent } else { hsla(0., 0., 0., 0.) })
                    .hover(|s| s.bg(p.button))
                    .when(rtl, |d| d.text_right())
                    .on_click(cx.listener(move |a, _, w, cx| a.set_settings_page(c, w, cx)))
                    .child(div().flex().when(rtl, |r| r.flex_row_reverse()).items_center().gap_1().child(c.title()).when(danger && !on, |r| r.child(div().w(px(6.)).h(px(6.)).rounded_full().bg(p.danger)))),
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
            Some(path) => match std::process::Command::new("xdg-open").arg(&path).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn() {
                Ok(mut child) => {
                    // reap it when it ends, without waiting here
                    std::thread::spawn(move || {
                        let _ = child.wait();
                    });
                }
                Err(e) => self.content_note = Some(format!("Could not start the browser (xdg-open): {e}")),
            },
        }
        cx.notify();
    }

    /// Bring the side panel back on a piece of content that was opened earlier, by its title.
    pub fn show_content_again(&mut self, title: &str, cx: &mut Context<Self>) {
        if let Some(i) = self.content.iter().rposition(|c| c.title == title) {
            self.content_index = i;
        }
        self.layout_mut().show_panel(Panel::Content);
        self.persist();
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
        let tabs: Vec<AnyElement> = (0..self.content.len()).map(|i| {
            let t = self.content[i].title.chars().take(24).collect::<String>();
            self.button(format!("content-{i}"), if t.is_empty() { "Untitled".to_string() } else { t }, if i == cur { Kind::Primary } else { Kind::Ghost }, cx, move |a, _, cx| {
                a.content_index = i;
                cx.notify();
            })
        }).collect();
        let item = self.content[cur].clone();
        let close = self.button("content-close", "Close", Kind::Ghost, cx, move |a, _, cx| {
            a.content.remove(cur);
            a.content_index = a.content_index.saturating_sub(1);
            if a.content.is_empty() {
                a.layout_mut().toggle_panel(Panel::Content);
            }
            cx.notify();
        });
        col = col.child(div().flex().flex_wrap().gap_1().children(tabs)).child(div().flex().gap_2().child(close)).child(if item.kind == ContentKind::Html { self.page_element(&item, window, cx).unwrap_or_else(|| self.html_card(&item, cur, cx)) } else { viewer(&item, "panel", &p) });
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

fn code_view(body: &str, scope: &str, title: &str, p: &crate::style::Pal) -> AnyElement {
    let lines: Vec<AnyElement> = body.lines().enumerate().map(|(i, l)| div().flex().gap_3().child(div().w(px(36.)).flex().justify_end().text_color(p.muted).child((i + 1).to_string())).child(div().child(if l.is_empty() { " ".to_string() } else { l.to_string() })).into_any_element()).collect();
    div().id(SharedString::from(format!("code-{scope}"))).role(Role::Document).aria_label(title.to_string()).p_2().rounded(p.inner).bg(p.sunken).text_color(p.text).font_family("DejaVu Sans Mono").text_sm().overflow_x_scroll().children(lines).into_any_element()
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

