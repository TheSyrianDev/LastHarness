//! The chat width controls in Settings: a slider with a glass knob, a box to type the number in, and presets. The
//! numbers and the rules (clamping, steps, presets, saving) are in `lh_gui::layout` and are tested there; this file
//! draws the slider and connects it to a text box, the keyboard and the screen reader.
//!
//! In right-to-left the narrow end is on the right and the fill grows from there.

use gpui::prelude::*;
use gpui::*;
use gpui_component::input::{Input, InputEvent, InputState};
use std::cell::Cell;
use std::rc::Rc;

use crate::app::App;
use crate::ui::Kind;
use lh_gui::layout::{slider_max, ColumnWidth, BUILT_IN_WIDTHS, COLUMN_MIN, COLUMN_STEP};

/// Everything the width controls remember between frames.
pub struct WidthControls {
    /// Where the track was drawn last frame (left edge and width), to turn a pointer position into a width.
    track: Rc<Cell<(f32, f32)>>,
    /// The knob is held.
    pub dragging: bool,
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

impl WidthControls {
    pub fn new(window: &mut Window, cx: &mut Context<App>, column: ColumnWidth, _rtl: bool) -> Self {
        let room = 1200.;
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("px"));
        input.update(cx, |s, cx| s.set_value(column.input_text(room), window, cx));
        cx.subscribe_in(&input, window, |this: &mut App, _, ev: &InputEvent, window, cx| {
            if matches!(ev, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                this.commit_width_text(window, cx);
            }
        })
        .detach();
        let preset_name = cx.new(|cx| InputState::new(window, cx).placeholder("Name this width"));
        WidthControls { track: Rc::new(Cell::new((0., 1.))), dragging: false, input, focus: cx.focus_handle(), room, preset_name, selected: None, note: None }
    }
}

impl App {
    /// Set from the slider while it is dragged: the column follows live; the file is written when the drag ends.
    pub fn width_dragged(&mut self, pointer_x: f32, window: &mut Window, cx: &mut Context<Self>) {
        let room = self.width.room;
        let (left, wide) = self.width.track.get();
        let frac = ((pointer_x - left) / wide.max(1.)).clamp(0., 1.);
        let frac = if self.rtl { 1. - frac } else { frac };
        let value = COLUMN_MIN + frac * (slider_max(room) - COLUMN_MIN);
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

    /// Make the number box show the width now in use; the slider is drawn from the width itself.
    pub fn sync_width_widgets(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (room, w) = (self.width.room, self.layout().column);
        let text = w.input_text(room);
        self.width.input.update(cx, |s, cx| s.set_value(text, window, cx));
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

    /// The track and its glass knob. A press anywhere on it moves the knob there; the drag follows the pointer
    /// even outside the track until the button is let go.
    fn width_track(&self, cx: &mut Context<Self>) -> AnyElement {
        const KNOB: f32 = 18.;
        let p = &self.pal;
        let room = self.width.room;
        let max = slider_max(room);
        let frac = ((self.layout().column.position(room) - COLUMN_MIN) / (max - COLUMN_MIN).max(1.)).clamp(0., 1.);
        let frac = if self.rtl { 1. - frac } else { frac };
        let held = self.width.dragging;
        let g = self.swell("width-thumb", held, 3., None);
        if held || self.glass_fx.borrow().target("width-thumb/swell").is_some_and(|t| t > 0.) {
            cx.notify();
        }
        let (track, me, dragging) = (self.width.track.clone(), cx.entity().downgrade(), held);
        let fill = div().absolute().top(px(KNOB / 2. - 2.)).h(px(4.)).rounded_full().bg(p.accent.opacity(0.7));
        let fill = if self.rtl { fill.right(px(0.)).w(relative(1. - frac)) } else { fill.left(px(0.)).w(relative(frac)) };
        div().id("chat-width-track").relative().w_full().h(px(KNOB)).cursor_pointer()
            .on_mouse_down(MouseButton::Left, cx.listener(|a, ev: &MouseDownEvent, w, cx| {
                a.width.dragging = true;
                a.width_dragged(f32::from(ev.position.x), w, cx);
                cx.notify();
            }))
            .child(div().absolute().top(px(KNOB / 2. - 2.)).left_0().right_0().h(px(4.)).rounded_full().bg(p.border))
            .child(fill)
            // the knob sits inside the track: at 0 its left edge meets the start, at 1 its right edge meets the end
            .child(div().absolute().top_0().left_0().right(px(KNOB)).h(px(KNOB))
                .child(self.glass(div().absolute().top(px(-g)).left(relative(frac)).ml(px(-g)).size(px(KNOB + g * 2.)).rounded_full(), "width-thumb", held, KNOB / 2. + g)
                    .child(div().size_full().rounded_full().border_2().border_color(p.accent.opacity(if held { 0.5 } else { 0.85 })))))
            .child(canvas(move |b, _, _| track.set((f32::from(b.origin.x) + KNOB / 2., f32::from(b.size.width) - KNOB)), move |_, _, window, _| {
                if !dragging {
                    return;
                }
                let m = me.clone();
                window.on_mouse_event(move |ev: &MouseMoveEvent, phase, window, cx| {
                    if phase == DispatchPhase::Bubble {
                        m.update(cx, |a, cx| a.width_dragged(f32::from(ev.position.x), window, cx)).ok();
                    }
                });
                let m = me.clone();
                window.on_mouse_event(move |_: &MouseUpEvent, phase, _, cx| {
                    if phase == DispatchPhase::Bubble {
                        m.update(cx, |a, cx| {
                            a.width.dragging = false;
                            a.persist();
                            cx.notify();
                        }).ok();
                    }
                });
            }).absolute().size_full())
            .into_any_element()
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
            .child(self.width_track(cx));
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
