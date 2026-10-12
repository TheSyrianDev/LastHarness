//! The chat width controls in Settings: a slider, a box to type the number in, and presets. The numbers and the
//! rules (clamping, steps, presets, saving) are in `lh_gui::layout` and are tested there; this file connects them
//! to the slider from gpui-component, a text box, the keyboard and the screen reader.
//!
//! The slider only knows left to right. In right-to-left it is given the mirrored value (`layout::mirror`) so the
//! narrow end is on the right and the fill grows from there. The row around it carries its own slider role with
//! the real number, because the slider inside announces the mirrored one.

use gpui::prelude::*;
use gpui::*;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::slider::{Slider, SliderEvent, SliderState};

use crate::app::App;
use crate::ui::Kind;
use lh_gui::layout::{mirror, slider_max, ColumnWidth, COLUMN_MIN, COLUMN_STEP, BUILT_IN_WIDTHS};

/// Everything the width controls remember between frames.
pub struct WidthControls {
    pub slider: Entity<SliderState>,
    /// What the slider was built for. It is built again when the room or the direction changes.
    built_max: f32,
    built_rtl: bool,
    _sub: Subscription,
    /// The number box.
    pub input: Entity<InputState>,
    /// Keyboard focus for the whole slider row.
    pub focus: FocusHandle,
    /// Pixels in the middle of the window for the column, as of the last frame.
    pub room: f32,
    /// The box for naming a preset, and the saved preset it is about.
    pub preset_name: Entity<InputState>,
    pub selected: Option<usize>,
    /// A word about the last thing tried, such as a name that is taken.
    pub note: Option<String>,
}

fn build_slider(window: &mut Window, cx: &mut Context<App>, column: ColumnWidth, room: f32, rtl: bool) -> (Entity<SliderState>, Subscription) {
    let pos = column.position(room);
    let value = if rtl { mirror(pos, room) } else { pos };
    let state = cx.new(|_| SliderState::new().min(COLUMN_MIN).max(slider_max(room)).step(COLUMN_STEP).default_value(value));
    let sub = cx.subscribe_in(&state, window, |this: &mut App, _, ev: &SliderEvent, window, cx| match ev {
        SliderEvent::Change(v) => this.width_dragged(v.end(), window, cx),
        SliderEvent::Release(_) => this.persist(),
    });
    (state, sub)
}

impl WidthControls {
    pub fn new(window: &mut Window, cx: &mut Context<App>, column: ColumnWidth, rtl: bool) -> Self {
        let room = 1200.;
        let (slider, sub) = build_slider(window, cx, column, room, rtl);
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("px"));
        input.update(cx, |s, cx| s.set_value(column.input_text(room), window, cx));
        cx.subscribe_in(&input, window, |this: &mut App, _, ev: &InputEvent, window, cx| {
            if matches!(ev, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                this.commit_width_text(window, cx);
            }
        })
        .detach();
        let preset_name = cx.new(|cx| InputState::new(window, cx).placeholder("Name this width"));
        WidthControls { slider, built_max: slider_max(room), built_rtl: rtl, _sub: sub, input, focus: cx.focus_handle(), room, preset_name, selected: None, note: None }
    }
}

impl App {
    /// Set from the slider while it is dragged: the column follows live; the file is written when the drag ends.
    pub fn width_dragged(&mut self, slider_value: f32, window: &mut Window, cx: &mut Context<Self>) {
        let room = self.width.room;
        let value = if self.rtl { mirror(slider_value, room) } else { slider_value };
        let w = ColumnWidth::from_slider(value, room);
        if self.layout().column != w {
            self.layout_mut().column = w;
            self.width.note = None;
            let text = w.input_text(room);
            self.width.input.update(cx, |s, cx| s.set_value(text, window, cx));
            cx.notify();
        }
    }

    /// Use this width everywhere: the column, the slider, the number box, the file. The one way keys, presets, the
    /// palette and the number box change the width.
    pub fn set_column(&mut self, w: ColumnWidth, window: &mut Window, cx: &mut Context<Self>) {
        self.layout_mut().column = w;
        self.width.note = None;
        self.sync_width_widgets(window, cx);
        self.persist();
        cx.notify();
    }

    /// Make the slider and the number box show the width now in use.
    pub fn sync_width_widgets(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (room, w) = (self.width.room, self.layout().column);
        let pos = w.position(room);
        let value = if self.rtl { mirror(pos, room) } else { pos };
        self.width.slider.update(cx, |s, cx| s.set_value(value, window, cx));
        let text = w.input_text(room);
        self.width.input.update(cx, |s, cx| s.set_value(text, window, cx));
    }

    /// Called every frame the Settings window is up: builds the slider again if the room or the direction changed,
    /// and takes a value a screen reader set on the slider inside.
    pub fn keep_width_slider_current(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let room = self.width.room;
        if (slider_max(room) - self.width.built_max).abs() > 0.1 || self.width.built_rtl != self.rtl {
            let (slider, sub) = build_slider(window, cx, self.layout().column, room, self.rtl);
            self.width.slider = slider;
            self.width._sub = sub;
            self.width.built_max = slider_max(room);
            self.width.built_rtl = self.rtl;
            return;
        }
        let v = self.width.slider.read(cx).value().end();
        let real = if self.rtl { mirror(v, room) } else { v };
        if (real - self.layout().column.position(room)).abs() > 0.5 {
            let w = ColumnWidth::from_slider(real, room);
            self.set_column(w, window, cx);
        }
    }

    /// The number box was finished (Enter, or the cursor left it): read it, or put the real number back.
    pub fn commit_width_text(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (room, text) = (self.width.room, self.width.input.read(cx).value().to_string());
        let current = self.layout().column;
        if text == current.input_text(room) {
            return;
        }
        match ColumnWidth::parse(&text, room) {
            Some(w) => self.set_column(w, window, cx),
            None => {
                self.width.note = Some(if self.rtl { "اكتب رقمًا من 480 فما فوق، أو Full." } else { "Type a number from 480 up, or Full." }.into());
                let back = current.input_text(room);
                self.width.input.update(cx, |s, cx| s.set_value(back, window, cx));
                cx.notify();
            }
        }
    }

    /// An arrow, Page, Home or End key on the slider row. Left and right follow the direction of the text.
    /// Returns whether the key was one of them.
    pub fn width_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let big = ev.keystroke.modifiers.shift;
        let (room, col, rtl) = (self.width.room, self.layout().column, self.rtl);
        let Some(next) = col.after_key(ev.keystroke.key.as_str(), big, rtl, room) else { return false };
        if next != col {
            self.set_column(next, window, cx);
        }
        true
    }

    /// Pick a width from the list: the slider moves to it. A saved preset is also remembered as the one chosen, so
    /// it can be renamed or deleted.
    pub fn choose_width(&mut self, w: ColumnWidth, saved: Option<usize>, window: &mut Window, cx: &mut Context<Self>) {
        self.set_column(w, window, cx);
        self.width.selected = saved;
        if let Some(p) = saved.and_then(|i| self.layout().presets.get(i)) {
            let name = p.name.clone();
            self.width.preset_name.update(cx, |s, cx| s.set_value(name, window, cx));
        }
    }

    fn preset_result(&mut self, r: Result<usize, lh_gui::layout::PresetError>, cx: &mut Context<Self>) {
        match r {
            Ok(i) => {
                self.width.selected = Some(i);
                self.width.note = None;
                self.persist();
            }
            Err(e) => self.width.note = Some(e.message(self.rtl).into()),
        }
        cx.notify();
    }

    pub fn save_width_preset(&mut self, cx: &mut Context<Self>) {
        let name = self.width.preset_name.read(cx).value().to_string();
        let r = self.layout_mut().save_preset(&name);
        self.preset_result(r, cx);
    }

    pub fn rename_width_preset(&mut self, cx: &mut Context<Self>) {
        let name = self.width.preset_name.read(cx).value().to_string();
        let r = match self.width.selected {
            Some(i) => self.layout_mut().rename_preset(i, &name).map(|_| i),
            None => Err(lh_gui::layout::PresetError::NoSuchPreset),
        };
        self.preset_result(r, cx);
    }

    pub fn delete_width_preset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(i) = self.width.selected else {
            self.width.note = Some(lh_gui::layout::PresetError::NoSuchPreset.message(self.rtl).into());
            return cx.notify();
        };
        let _ = self.layout_mut().delete_preset(i);
        self.width.selected = None;
        self.width.note = None;
        self.width.preset_name.update(cx, |s, cx| s.set_value("", window, cx));
        self.persist();
        cx.notify();
    }

    /// The Chat width part of Settings: the slider row, then the presets.
    pub fn width_controls(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let (rtl, room, col) = (self.rtl, self.width.room, self.layout().column);
        let max = slider_max(room);
        let label = if rtl { "عرض المحادثة" } else { "Chat width" };
        let ring = BoxShadow { color: p.accent, offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: px(2.), inset: false };
        let me = cx.entity();
        let step = move |narrower: bool, big: bool| {
            let me = me.clone();
            move |_: Option<&accesskit::ActionData>, window: &mut Window, cx: &mut gpui::App| {
                me.update(cx, |a, cx| {
                    let next = a.layout().column.nudge(narrower, big, a.width.room);
                    a.set_column(next, window, cx);
                })
            }
        };
        let focus = self.width.focus.clone();
        let row = div()
            .id("chat-width")
            .role(Role::Slider)
            .aria_label(label)
            .aria_numeric_value(col.position(room) as f64)
            .aria_min_numeric_value(COLUMN_MIN as f64)
            .aria_max_numeric_value(max as f64)
            .aria_numeric_value_step(COLUMN_STEP as f64)
            .aria_value(col.label(rtl))
            .on_a11y_action(accesskit::Action::Increment, step(false, false))
            .on_a11y_action(accesskit::Action::Decrement, step(true, false))
            .track_focus(&self.width.focus)
            .tab_stop(true)
            .focus_visible(move |s| s.shadow(vec![ring]))
            .rounded(p.inner)
            .flex_1()
            .min_w(px(120.))
            .px_2()
            .py_1()
            // a press on the track or the thumb moves keyboard focus here too, so the arrows work straight after a drag
            .capture_any_mouse_down(move |_, window, cx| focus.focus(window, cx))
            .on_key_down(cx.listener(|a, ev: &KeyDownEvent, w, cx| {
                if a.width_key(ev, w, cx) {
                    cx.stop_propagation();
                }
            }))
            .child(Slider::new(&self.width.slider).when(rtl, |s| s.reverse()));
        let unit = div().text_sm().text_color(p.muted).child(if col == ColumnWidth::Full { if rtl { "الكامل" } else { "Full" } } else { "px" });
        let slider_row = div()
            .flex()
            .when(rtl, |d| d.flex_row_reverse())
            .items_center()
            .gap_2()
            .child(div().text_sm().text_color(p.muted).child(label))
            .child(row)
            .child(div().w(px(84.)).child(Input::new(&self.width.input)))
            .child(unit);

        // the presets: built-in numbers first, then the ones the person made, each labelled by its number
        let mut chips = div().flex().flex_wrap().when(rtl, |d| d.flex_row_reverse()).items_center().gap_1();
        chips = chips.child(div().text_sm().text_color(p.muted).child(if rtl { "إعدادات مسبقة" } else { "Presets" }));
        for (i, w) in BUILT_IN_WIDTHS.iter().copied().enumerate() {
            let kind = if col == w { Kind::Primary } else { Kind::Normal };
            chips = chips.child(self.button(format!("width-built-in-{i}"), w.label(rtl), kind, cx, move |a, window, cx| a.choose_width(w, None, window, cx)));
        }
        let saved: Vec<(usize, String, ColumnWidth)> = self.layout().presets.iter().enumerate().map(|(i, s)| (i, s.name.clone(), s.width)).collect();
        for (i, name, w) in saved {
            let chosen = self.width.selected == Some(i);
            let kind = if col == w || chosen { Kind::Primary } else { Kind::Normal };
            chips = chips.child(self.button(format!("width-preset-{i}"), format!("{name} · {}", w.label(rtl)), kind, cx, move |a, window, cx| a.choose_width(w, Some(i), window, cx)));
        }
        let save = self.button("width-preset-save", if rtl { "احفظ العرض الحالي" } else { "Save this width" }, Kind::Normal, cx, |a, _, cx| a.save_width_preset(cx));
        let mut manage = div().flex().flex_wrap().when(rtl, |d| d.flex_row_reverse()).items_center().gap_1().child(div().w(px(180.)).child(Input::new(&self.width.preset_name))).child(save);
        if self.width.selected.is_some() {
            manage = manage
                .child(self.button("width-preset-rename", if rtl { "إعادة تسمية" } else { "Rename" }, Kind::Normal, cx, |a, _, cx| a.rename_width_preset(cx)))
                .child(self.button("width-preset-delete", if rtl { "حذف" } else { "Delete" }, Kind::Danger, cx, |a, window, cx| a.delete_width_preset(window, cx)));
        }
        let note = self.width.note.clone().map(|n| div().text_xs().text_color(p.danger).child(n));
        div()
            .flex()
            .flex_col()
            .gap_2()
            .id("chat-width-group")
            .role(Role::Group)
            .aria_label(label)
            .child(slider_row)
            .child(chips)
            .child(manage)
            .children(note)
            .into_any_element()
    }
}
