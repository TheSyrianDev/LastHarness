//! Drawing: the window as the theme describes it. Reads the state in `App`, never changes it except through
//! the click handlers it attaches.

use gpui::prelude::*;
use gpui::*;
use gpui_component::input::{Input, Textarea};
use gpui_component::text::TextView;

use crate::app::{App, Focus, Pending};
use lh_gui::chrome::{chevron, topmost, Layer, RailItem, TitleInfo, RAIL_WIDTH};
use lh_gui::permissions::may_offer;
use lh_gui::thinking::{hint_text, is_long, progress_text, spoken_text, Snap};
use lh_gui::transcript::{self, work_label, Item, Step, StepKind};
use lh_gui::layout::{Page, Panel};
use lh_gui::motion;
use lh_gui::model::{Block, Mode};
use lh_gui::rtl;
use crate::style::{mix, PanelExt};

actions!(lastharness, [NewChat, ToggleSidebar, ShowDepartments, ShowMemory, ShowSettings, ShowContent, ShowTree, ShowWork, ToggleMode, CycleDirection, StopTurn, NextChat, PrevChat, ToggleExpand, ConfirmResume, Dismiss, OpenLastReply, PopOutContent, AllowAsk, DenyAsk, OpenPalette, JumpToLatest]);

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
        KeyBinding::new("ctrl-shift-w", ShowWork, None),
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
    // the inside keeps its full width while the outside narrows, so it slides out whole instead of squeezing
    div().w(px(shown)).h_full().flex_none().overflow_hidden().child(div().w(px(full)).h_full().flex_none().flex().child(child)).into_any_element()
}

/// Draw an icon: the theme's own SVG when it has one by this name, else the app's line drawing.
pub(crate) fn icon_view(p: &crate::style::Pal, icon: icons::Icon, color: Hsla, size: f32) -> AnyElement {
    if let Some(svg_bytes) = lh_gui::icons::resolve(&icon, &p.icons) {
        let mut el = svg().data(svg_bytes).size(px(size)).flex_none().text_color(color);
        if icon.mirror {
            el = el.with_transformation(Transformation::scale(gpui::size(-1., 1.)));
        }
        return el.into_any_element();
    }
    let lines = icon.lines;
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
    .into_any_element()
}

/// Pixels between two stops of the model slider.
const SLIDER_GAP: f32 = 60.;
/// Pixels between two stops of the context-length slider.
const CTX_GAP: f32 = 64.;
/// The model slider's ball, the context slider's ball and the mode switch's drop, in pixels tall.
const SLIDER_KNOB: f32 = 22.;
const CTX_KNOB: f32 = 18.;
const MODE_H: f32 = 42.;

/// Line icons on a 24 by 24 grid, drawn by the app in one stroke weight.
pub(crate) mod icons {
    pub use lh_gui::icons::Icon;
    pub type Lines = Icon;
    use lh_gui::icons::Lines as L;
    fn circle(cx: f32, cy: f32, r: f32) -> Vec<(f32, f32)> {
        (0..=24).map(|i| { let a = i as f32 / 24. * std::f32::consts::TAU; (cx + r * a.cos(), cy + r * a.sin()) }).collect()
    }
    fn flip(l: L) -> L { l.into_iter().map(|line| line.into_iter().map(|(x, y)| (24. - x, y)).collect()).collect() }
    pub fn pencil() -> Icon { Icon::new("new-chat", vec![vec![(4., 20.), (4.5, 15.5), (15., 5.), (19., 9.), (8.5, 19.5), (4., 20.)], vec![(13., 7.), (17., 11.)]]) }
    pub fn search() -> Icon { Icon::new("search", vec![circle(10.5, 10.5, 6.), vec![(15., 15.), (20., 20.)]]) }
    pub fn send() -> Icon { Icon::new("send", vec![vec![(12., 19.), (12., 5.5)], vec![(6., 11.), (12., 5.), (18., 11.)]]) }
    /// A paperclip: attach files.
    pub fn attach() -> Icon { Icon::new("attach", vec![vec![(19., 11.), (11.5, 18.5), (8., 18.5), (5.5, 16.), (5.5, 12.5), (13., 5.), (15.5, 5.), (17.5, 7.), (17.5, 9.5), (10., 17.), (8.8, 17.), (8., 16.2), (8., 15.), (14., 9.)]]) }
    pub fn stop() -> Icon { Icon::new("stop", vec![vec![(7., 7.), (17., 7.), (17., 17.), (7., 17.), (7., 7.)]]) }
    /// A window with its left pane marked: folds the sidebar away. Mirrored in right-to-left.
    pub fn sidebar_collapse(rtl: bool) -> Icon {
        let l = vec![vec![(4., 5.), (20., 5.), (20., 19.), (4., 19.), (4., 5.)], vec![(9.5, 5.), (9.5, 19.)], vec![(16., 9.5), (13.5, 12.), (16., 14.5)]];
        Icon::new("sidebar-collapse", if rtl { flip(l) } else { l }).mirrored(rtl)
    }
    /// The same window with the arrow pointing out: opens the sidebar again.
    pub fn sidebar_expand(rtl: bool) -> Icon {
        let l = vec![vec![(4., 5.), (20., 5.), (20., 19.), (4., 19.), (4., 5.)], vec![(9.5, 5.), (9.5, 19.)], vec![(13.5, 9.5), (16., 12.), (13.5, 14.5)]];
        Icon::new("sidebar-expand", if rtl { flip(l) } else { l }).mirrored(rtl)
    }
    pub fn lock() -> Icon {
        let mut shackle = vec![(8.5, 11.), (8.5, 8.5)];
        shackle.extend((0..=12).map(|i| { let a = std::f32::consts::PI * (1. + i as f32 / 12.); (12. + 3.5 * a.cos(), 8.5 + 3.5 * a.sin()) }));
        shackle.push((15.5, 11.));
        Icon::new("lock", vec![vec![(7.5, 11.), (16.5, 11.), (18., 12.5), (18., 18.5), (16.5, 20.), (7.5, 20.), (6., 18.5), (6., 12.5), (7.5, 11.)], shackle, vec![(12., 14.5), (12., 16.5)]])
    }
    pub fn panel(rtl: bool) -> Icon { let x = if rtl { 9. } else { 15. }; Icon::new("panel", vec![vec![(4., 5.), (20., 5.), (20., 19.), (4., 19.), (4., 5.)], vec![(x, 5.), (x, 19.)]]).mirrored(rtl) }
    pub fn check() -> Icon { Icon::new("check", vec![vec![(5., 12.5), (10., 17.), (19., 7.)]]) }
    pub fn close() -> Icon { Icon::new("close", vec![vec![(6., 6.), (18., 18.)], vec![(18., 6.), (6., 18.)]]) }
    pub fn widen() -> Icon { Icon::new("widen", vec![vec![(9., 6.), (4., 12.), (9., 18.)], vec![(15., 6.), (20., 12.), (15., 18.)]]) }
    pub fn popout() -> Icon { Icon::new("popout", vec![vec![(13., 4.), (20., 4.), (20., 11.)], vec![(20., 4.), (11., 13.)], vec![(17., 14.), (17., 20.), (4., 20.), (4., 7.), (10., 7.)]]) }
    pub fn chevron_right() -> Icon { Icon::new("chevron-right", vec![vec![(9., 6.), (15., 12.), (9., 18.)]]) }
    pub fn chevron_left() -> Icon { Icon::new("chevron-left", vec![vec![(15., 6.), (9., 12.), (15., 18.)]]) }
    pub fn chevron_down() -> Icon { Icon::new("chevron-down", vec![vec![(7., 10.), (12., 15.), (17., 10.)]]) }
    pub fn rail(i: lh_gui::chrome::RailItem) -> Icon { Icon::new(lh_gui::chrome::rail_icon_name(i), i.icon()) }
    pub fn category(c: lh_gui::setting_names::Category) -> Icon { Icon::new(lh_gui::chrome::category_icon_name(c), lh_gui::chrome::category_icon(c)) }
}

impl App {
    /// A sidebar row in the Quiet desk look: a drawn icon and a word; the open one is a raised white row.
    fn nav_row(&self, id: &str, icon: icons::Lines, label: impl Into<SharedString>, on: bool, cx: &mut Context<Self>, click: impl Fn(&mut App, &mut Window, &mut Context<App>) + 'static) -> AnyElement {
        self.nav_row_c(id, icon, label, on, false, cx, click)
    }

    /// `compact`: the icon alone, in the same place it has in the wide sidebar, so nothing moves when it folds.
    fn nav_row_c(&self, id: &str, icon: icons::Lines, label: impl Into<SharedString>, on: bool, compact: bool, cx: &mut Context<Self>, click: impl Fn(&mut App, &mut Window, &mut Context<App>) + 'static) -> AnyElement {
        let p = self.pal.clone();
        let label: SharedString = label.into();
        let lift = BoxShadow { color: hsla(0., 0., 0., if p.dark { 0.3 } else { 0.07 }), offset: point(px(0.), px(1.)), blur_radius: px(3.), spread_radius: px(0.), inset: false };
        div().id(SharedString::from(id.to_string())).role(Role::Button).aria_label(label.clone())
            .flex().when(self.rtl, |d| d.flex_row_reverse()).items_center().gap_2p5().px_2p5().h(px(36.)).flex_none().rounded(p.inner).cursor_pointer()
            .text_color(if on { p.text } else { p.text.opacity(0.82) })
            .when(on, |d| d.bg(p.surface).shadow(vec![lift]).font_weight(gpui::FontWeight::MEDIUM))
            .hover(|s| s.bg(p.button))
            .on_click(cx.listener(move |a, _, w, cx| click(a, w, cx)))
            .child(div().h(px(20.)).flex().items_center().child(icon_view(&p, icon, if on { p.text } else { p.muted }, 16.)))
            .when(!compact, |d| d.child(label))
            .into_any_element()
    }

    /// A small round icon button, such as search or hide.
    /// Fade something in as it appears; it plays again each time the thing is shown anew.
    pub(crate) fn appear<E: IntoElement + Styled + 'static>(&self, el: E, key: &'static str) -> AnyElement {
        let d = motion::press(self.pal.anim);
        // drawn after everything else, so nothing in the window paints over a menu
        if d.is_zero() {
            return deferred(el).with_priority(1).into_any_element();
        }
        deferred(el.with_animation(key, Animation::new(d + std::time::Duration::from_millis(80)).with_easing(motion::ease_out), |e, t| e.opacity(t).mt(px(6. * (1. - t))))).with_priority(1).into_any_element()
    }

    /// A new message or result card fades and rises into place once. Under reduced motion it simply appears.
    fn rise(&self, el: AnyElement, key: u64, at: usize) -> AnyElement {
        let d = motion::panel(self.pal.anim);
        if d.is_zero() {
            return el;
        }
        div().id(SharedString::from(format!("rise-{key}-{at}"))).child(el)
            .with_animation(SharedString::from(format!("rise-a-{key}-{at}")), Animation::new(d).with_easing(motion::ease_out), |e, t| e.opacity(t).mt(px(8. * (1. - t))))
            .into_any_element()
    }

    /// A thumb that looks like a drop of glass: see-through, a bright rim, a highlight along the top edge and a soft
    /// shadow. Held, it clears a little and the shadow spreads, as if lifted. `id` names its motion: the drop's
    /// energy (rest, hover, pressed) eases between states instead of jumping, and so do the lift and the clearing.
    pub(crate) fn glass(&self, d: Div, id: &str, held: bool, radius: f32) -> Div {
        let p = &self.pal;
        let dur = motion::press(p.anim);
        let key = format!("{id}/energy");
        let place = format!("{id}/place");
        // the energy as of the last paint, for what is decided at layout (the shadow's lift)
        let lift = self.glass_fx.borrow().now(&format!("{id}/swell"), std::time::Instant::now(), dur, 0.).clamp(0., 1.);
        let dark = p.dark;
        let rim = if dark { 0.3 } else { 0.85 };
        let surface = p.surface;
        let variant = crate::style::glass_variant(p.material.controls).unwrap_or_default();
        let fx = self.glass_fx.clone();
        // the energy is settled at paint, where the drop's real bounds and the pointer are known: pressed while held,
        // hover while the pointer is over it (the empty hover style below makes GPUI repaint when it enters or leaves)
        let lens = gpui::canvas(|_, _, _| (), move |bounds, _, window, _| {
            let mut fx = fx.borrow_mut();
            // while the drop itself glides (released, on its way to a stop) the pointer sweeping over it is not a
            // hover: the target would flip between hover and rest every frame and the lens would flicker
            let still = fx.at_rest(&place, f32::from(bounds.origin.x), f32::from(bounds.size.width));
            let hover = bounds.contains(&window.mouse_position());
            let target = if held { gpui::glass_energy::PRESSED } else if !still { fx.target(&key).unwrap_or(gpui::glass_energy::REST).min(gpui::glass_energy::HOVER) } else if hover { gpui::glass_energy::HOVER } else { gpui::glass_energy::REST };
            let (energy, moving) = fx.to(&key, target, std::time::Instant::now(), dur);
            drop(fx);
            // one more frame after the drop stops, so hover is judged again where it came to rest
            if moving || !still {
                window.request_animation_frame();
            }
            let e = energy.clamp(0., 1.);
            let lerp = |a: f32, b: f32| a + (b - a) * e;
            // the drop clears as it is pressed: less fill, more bend, a brighter rim
            let fill = if dark { lerp(0.12, 0.06) } else { lerp(0.3, 0.12) };
            // the tint leans toward the look's own surface hue (warm in Claude and Matte, cool in Glass), kept faint
            let tint = gpui::Hsla { h: surface.h, s: surface.s.min(0.35), l: if dark { 0.92 } else { 1. }, a: fill };
            window.paint_glass(bounds, px(radius), gpui::GlassParams { blur: px(lerp(1.5, 1.)), refraction: px(lerp(8., 12.)), tint, rim: lerp(0.6, 0.8), variant, energy });
        }).absolute().top_0().left_0().size_full();
        let l = |a: f32, b: f32| a + (b - a) * lift;
        d.hover(|s| s).child(lens).border_1().border_color(gpui::Hsla { h: 0., s: 0., l: 1., a: rim }).shadow(vec![
            BoxShadow { color: hsla(0., 0., 0., if dark { 0.45 } else { 0.14 }), offset: point(px(0.), px(l(2., 6.))), blur_radius: px(l(6., 16.)), spread_radius: px(0.), inset: false },
            BoxShadow { color: hsla(0., 0., 1., if dark { 0.25 } else { 0.95 }), offset: point(px(0.), px(1.)), blur_radius: px(1.), spread_radius: px(0.), inset: true },
            BoxShadow { color: hsla(0., 0., 0., 0.06), offset: point(px(0.), px(-1.)), blur_radius: px(2.), spread_radius: px(0.), inset: true },
        ])
    }

    /// How much a glass thumb swells, up to `max` pixels: it follows the drop's eased energy, so it grows as it is
    /// pressed and settles back after, instead of jumping by `max`.
    pub(crate) fn swell(&self, id: &str, held: bool, max: f32, window: Option<&mut Window>) -> f32 {
        // its own glide, set at layout from `held` alone: hover never changes a thumb's size, and the first frame of
        // a press starts from rest instead of from a full swell (it used to read the energy key before paint set it)
        let (e, moving) = self.glass_fx.borrow_mut().to(&format!("{id}/swell"), if held { 1. } else { 0. }, std::time::Instant::now(), motion::press(self.pal.anim));
        if let (true, Some(window)) = (moving, window) {
            window.request_animation_frame();
        }
        max * e.clamp(0., 1.)
    }

    /// A slider thumb's stretch: what the drag gives while held; after release it eases back to nothing instead of
    /// snapping, asking for frames while it does.
    pub(crate) fn thumb_stretch(&self, id: &str, held: Option<f32>, window: &mut Window) -> f32 {
        let key = format!("{id}/stretch");
        let mut fx = self.glass_fx.borrow_mut();
        match held {
            Some(s) => {
                fx.hold(&key, s);
                s
            }
            None => {
                let (v, moving) = fx.to(&key, 0., std::time::Instant::now(), motion::panel(self.pal.anim));
                if moving {
                    window.request_animation_frame();
                }
                v.max(0.)
            }
        }
    }

    /// A switch with a glass knob. `click` gets the new state. Without a label it still says `label` to screen readers.
    pub(crate) fn glass_switch(&self, id: impl Into<SharedString>, on: bool, enabled: bool, label: impl Into<SharedString>, show_label: bool, cx: &mut Context<Self>, click: impl Fn(&mut App, bool, &mut Window, &mut Context<App>) + 'static) -> AnyElement {
        let p = &self.pal;
        let (id, label): (SharedString, SharedString) = (id.into(), label.into());
        let rtl = self.rtl;
        // the knob glides across when the switch flips (12 px: the 34 px track less its border, padding and knob)
        let key = format!("{id}/pos");
        let (at, moving) = self.glass_fx.borrow_mut().to(&key, if on != rtl { 12. } else { 0. }, std::time::Instant::now(), motion::press(p.anim));
        let knob = self.glass(div().size(px(16.)).ml(px(at)).rounded_full(), &id, false, 8.);
        div().id(id).role(Role::Switch).aria_label(label.clone()).aria_toggled(if on { accesskit::Toggled::True } else { accesskit::Toggled::False })
            .flex().when(rtl, |d| d.flex_row_reverse()).items_center().gap_2()
            .when(enabled, |d| d.cursor_pointer().on_click(cx.listener(move |a, _, w, cx| click(a, !on, w, cx))))
            .when(!enabled, |d| d.opacity(0.5))
            .child(div().flex_none().w(px(34.)).h(px(20.)).p(px(2.)).rounded_full().bg(if on { p.accent } else { p.border }).border_1().border_color(p.border).flex().items_center().child(knob)
                .when(moving, |d| d.child(gpui::canvas(|_, _, _| (), |_, _, window, _| window.request_animation_frame()).absolute().size_0())))
            .when(show_label, |d| d.child(div().text_sm().text_color(p.text).child(label)))
            .into_any_element()
    }

    pub(crate) fn icon_button(&self, id: &str, icon: icons::Lines, aria: &str, cx: &mut Context<Self>, click: impl Fn(&mut App, &mut Window, &mut Context<App>) + 'static) -> AnyElement {
        let p = self.pal.clone();
        div().id(SharedString::from(id.to_string())).role(Role::Button).aria_label(SharedString::from(aria.to_string()))
            .size(px(32.)).flex().items_center().justify_center().rounded(p.inner).cursor_pointer().hover(|s| s.bg(p.button))
            .on_click(cx.listener(move |a, _, w, cx| click(a, w, cx)))
            .child(icon_view(&p, icon, p.muted, 17.))
            .into_any_element()
    }

    /// A small chip for a file. With `remove` it carries a close button.
    pub(crate) fn file_chip(&self, id: &str, name: &str, remove: Option<usize>, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let rtl = self.rtl;
        let label = SharedString::from(name.to_string());
        div().id(SharedString::from(id.to_string())).role(Role::ListItem).aria_label(label.clone())
            .flex().when(rtl, |d| d.flex_row_reverse()).items_center().gap_1().pl_2().pr_1().py_0p5().max_w(px(220.)).rounded(p.inner).border_1().border_color(p.border.opacity(0.9)).bg(p.button)
            .text_xs().text_color(p.text.opacity(0.85))
            .child(icon_view(&p, icons::attach(), p.muted, 12.))
            .child(div().min_w_0().truncate().child(label))
            .children(remove.map(|n| {
                div().id(SharedString::from(format!("{id}-x"))).role(Role::Button).aria_label(if rtl { "إزالة الملف" } else { "Remove file" }).size(px(18.)).flex().items_center().justify_center().rounded(p.inner).cursor_pointer().hover(|s| s.bg(p.hover))
                    .on_click(cx.listener(move |a, _, _, cx| a.unstage(n, cx)))
                    .child(icon_view(&p, icons::close(), p.muted, 11.))
            }))
            .into_any_element()
    }

    /// A pill on the message box: model, Private. `active` tints it with the accent.
    fn pill(&self, id: &str, icon: Option<icons::Lines>, label: impl Into<SharedString>, trailing: Option<icons::Lines>, active: bool, cx: &mut Context<Self>, click: impl Fn(&mut App, &mut Window, &mut Context<App>) + 'static) -> AnyElement {
        let p = self.pal.clone();
        let label: SharedString = label.into();
        let fg = if active { p.accent } else { p.text.opacity(0.8) };
        div().id(SharedString::from(id.to_string())).role(Role::Button).aria_label(label.clone())
            .flex().when(self.rtl, |d| d.flex_row_reverse()).items_center().gap_1p5().px_3().py_1().rounded(p.inner).border_1().cursor_pointer().text_sm()
            .border_color(if active { p.accent.opacity(0.45) } else { p.border.opacity(0.9) })
            .bg(if active { p.accent.opacity(0.1) } else { hsla(0., 0., 0., 0.) })
            .text_color(fg)
            .hover(|s| s.bg(p.button))
            .on_click(cx.listener(move |a, _, w, cx| click(a, w, cx)))
            .children(icon.map(|i| icon_view(&p, i, fg, 14.)))
            .child(label)
            .children(trailing.map(|i| icon_view(&p, i, p.muted, 13.)))
            .into_any_element()
    }

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
            "«" | "»" => "Hide the sidebar (Ctrl+B)",
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

        // pressing darkens the button a little more, answering at once
        let pressed = mix(hover, p.text, 0.12);
        let el = base_el.bg(if hovered { hover } else { base }).active(move |s| s.bg(pressed)).child(label);
        if !animate {
            return el.into_any_element();
        }
        let (from, to) = if hovered { (base, hover) } else { (hover, base) };
        el.with_animation(anim_key, Animation::new(motion::press(self.pal.anim)).with_easing(motion::ease_out), move |el, d| el.bg(mix(from, to, d))).into_any_element()
    }

    /// The bar over the chat: its name, the branch it is on, how much the model has read, and which model.
    fn top_bar(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let rtl = self.rtl;
        let info = match self.page().clone() {
            Page::Department(name) => TitleInfo::page(&name),
            Page::Departments => TitleInfo::page(if rtl { "الأقسام" } else { "Departments" }),
            Page::Memory => TitleInfo::page(if rtl { "الذاكرة" } else { "Memory" }),
            Page::Chat => {
                let chat = self.chats.current().cloned();
                TitleInfo::chat(chat.as_ref().map(|c| c.title.as_str()).unwrap_or(""), chat.as_ref().map(|c| c.model.as_str()).unwrap_or(""), chat.as_ref().and_then(|c| c.context_tokens), self.branches(), rtl)
            }
        };
        // the side panel's switch, with a dot when this chat has something there and the panel is closed
        let open = self.layout().right_visible;
        let has = self.dock_counts().iter().any(|(_, n)| *n > 0);
        // a new plain chat offers Private in this corner; once the chat starts, the side panel's switch takes its place
        let fresh_plain = self.page() == &Page::Chat && self.chats.current().is_some_and(|c| c.session.is_none() && c.blocks.is_empty() && !c.dormant && c.mode == Mode::Chat);
        let private_on = self.chats.current().is_some_and(|c| c.incognito);
        let dock = div().relative()
            .child(self.icon_button("dock-toggle", icons::panel(rtl), if open { if rtl { "إخفاء اللوحة" } else { "Hide the side panel" } } else if rtl { "إظهار اللوحة" } else { "Show the side panel" }, cx, |a, _, cx| a.toggle_dock(cx)))
            .when(has && !open, |d| d.child(div().absolute().top(px(5.)).right(px(5.)).size(px(8.)).rounded_full().bg(p.muted)));
        let right_corner = if fresh_plain {
            self.pill("mode-private", Some(icons::lock()), if rtl { "خاصة" } else { "Private" }, None, private_on, cx, |a, _, cx| a.toggle_private(cx))
        } else {
            dock.into_any_element()
        };
        div()
            .id("title-bar")
            .role(Role::Banner)
            .flex()
            .when(rtl, |d| d.flex_row_reverse())
            .items_center()
            .gap_3()
            .p_2()
            .child(div().flex_1().overflow_hidden().text_color(p.text).child(info.title))
            .when_some(info.branch, |d, b| d.child(div().id("branch").role(Role::Label).text_sm().text_color(p.muted).child(b)))
            .when_some(info.context, |d, c| d.child(div().id("context").role(Role::Label).text_sm().text_color(p.muted).child(c)))
            .child(right_corner)
            .into_any_element()
    }

    /// The model picker, like the `/pick` Pi extension: search and one row per model on one side, that model's
    /// settings (context, template, mode) on the other. A setting is used at once; the menu stays open.
    fn menus(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        use lh_gui::models::{mode_word, parse};
        if !self.model_menu {
            return None;
        }
        let p = self.pal.clone();
        let rtl = self.rtl;
        let rows = self.picker_rows(cx);
        let sel = self.picker_sel.min(rows.len().saturating_sub(1));
        let cur = self.chats.current().map(|c| c.model.clone()).unwrap_or_default();
        let cur_base = self.models.group_of(&cur).map(|g| g.base.clone());
        let list: Vec<AnyElement> = rows
            .iter()
            .enumerate()
            .map(|(i, g)| {
                let base = g.base.clone();
                let open = self.picker_open.as_ref().or(cur_base.as_ref()) == Some(&g.base);
                let mut row = div()
                    .id(SharedString::from(format!("pick-{i}")))
                    .role(Role::ListBoxOption)
                    .aria_label(g.label.clone())
                    .aria_selected(i == sel)
                    .flex()
                    .when(rtl, |d| d.flex_row_reverse())
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_1p5()
                    .rounded(p.inner)
                    .cursor_pointer()
                    .bg(if open { p.accent.opacity(0.22) } else if i == sel { p.button } else { hsla(0., 0., 0., 0.) })
                    .hover(|s| s.bg(p.button))
                    .on_click(cx.listener(move |a, _, _, cx| {
                        a.picker_open = Some(base.clone());
                        a.picker_sel = i;
                        let cur = a.chats.current().map(|c| c.model.clone()).unwrap_or_default();
                        if a.models.group_of(&cur).map(|x| &x.base) != Some(&base) {
                            if let Some(id) = a.models.groups().iter().find(|g| g.base == base).map(|g| g.default_entry().id.clone()) {
                                a.choose_model(&id, cx);
                                if a.pending.is_none() {
                                    a.model_menu = true;
                                }
                            }
                        }
                        cx.notify();
                    }));
                // a green dot: the router has this model in memory now
                row = row.child(div().size(px(7.)).rounded_full().bg(if g.loaded() { gpui::rgb(0x2e9e6a).into() } else { p.muted.opacity(0.35) }));
                row = row.child(div().flex_1().overflow_hidden().truncate().text_color(p.text).child(g.label.clone()));
                if !g.size.is_empty() {
                    row = row.child(div().text_xs().text_color(p.muted).child(g.size.clone()));
                }
                if cur_base.as_ref() == Some(&g.base) {
                    // drawn like the other icons, not a text glyph
                    row = row.child(div().flex_none().size(px(14.)).child(icon_view(&p, icons::check(), p.accent, 14.)));
                }
                row.into_any_element()
            })
            .collect();
        let none = rows.is_empty().then(|| div().px_2().py_1p5().text_sm().text_color(p.muted).child(if rtl { "لا نموذج يطابق." } else { "No model matches." }));

        // the open model's settings
        // with nothing chosen yet, the model in use shows its settings, so the right half is never empty
        let detail: Option<AnyElement> = self.picker_open.clone().or_else(|| cur_base.clone()).and_then(|b| self.models.groups().iter().find(|g| g.base == b).cloned()).map(|g| {
            let here = cur_base.as_ref() == Some(&g.base);
            let now = if here { parse(&self.models.id_for(&cur)) } else { parse(&g.default_entry().id) };
            let entry_id = g.entry(&now.tier, &now.tpl).id.clone();
            let info = self.models.info(&entry_id).cloned().unwrap_or_default();
            let mode = if here { now.mode.clone() } else { String::new() };
            let mut col = div().id("pick-detail").flex_1().flex().flex_col().gap_3().p_3().overflow_y_scroll();
            col = col.child(div().flex().gap_2().items_baseline().child(div().text_color(p.text).font_weight(gpui::FontWeight::MEDIUM).child(g.label.clone())).child(div().text_xs().text_color(p.muted).child(g.size.clone())));
            let section = |title: &str, chips: Vec<AnyElement>| {
                div().flex().flex_col().gap_1p5().child(div().text_xs().text_color(p.muted).child(title.to_string())).child(div().flex().flex_wrap().gap_1p5().children(chips)).into_any_element()
            };
            let pick = |tier: String, tpl: String, mode: String| {
                let gg = g.clone();
                move |a: &mut App, _: &mut Window, cx: &mut Context<App>| {
                    let e = gg.entry(&tier, &tpl).id.clone();
                    let choice = if mode.is_empty() { e } else { format!("{e}:{mode}") };
                    a.choose_model(&choice, cx);
                    // a chat that has started asks first (the resume warning); otherwise the menu stays open
                    if a.pending.is_none() {
                        a.model_menu = true;
                    }
                    cx.notify();
                }
            };
            let tiers = g.tiers(&now.tpl);
            if tiers.len() > 1 {
                // the same slider as the model slider: stops that hold, a thumb that stretches, a glide on release
                const STEP: f32 = CTX_GAP;
                const KNOB: f32 = CTX_KNOB;
                let n = tiers.len();
                let choices: Vec<String> = (0..n).map(|slot| {
                    let i = if rtl { n - 1 - slot } else { slot };
                    let e = g.entry(&tiers[i], &now.tpl).id.clone();
                    if mode.is_empty() { e } else { format!("{e}:{mode}") }
                }).collect();
                let at = tiers.iter().position(|t| here && *t == now.tier);
                let (gnow, dur) = (std::time::Instant::now(), motion::panel(p.anim));
                // held, the thumb eases to the stop nearest the pointer; let go, it rests on the chosen stop
                let raw = self.ctx_drag.as_ref().map(|d| d.2);
                let target = match (raw, at) {
                    (Some(r), _) => Some(lh_gui::slide::snap(r, STEP, n) as f32 * STEP),
                    (None, Some(i)) => Some((if rtl { n - 1 - i } else { i }) as f32 * STEP),
                    _ => None,
                };
                if let Some(t) = target {
                    self.ctx_glide.go(t, gnow, dur);
                    if self.ctx_glide.moving(gnow, dur) {
                        window.request_animation_frame();
                    }
                }
                let drawn = self.ctx_glide.value(gnow, dur);
                let (left, stretch) = match raw {
                    Some(r) => { let h = lh_gui::slide::held(r, drawn, STEP, n, KNOB); (h.left, h.stretch) }
                    None => (drawn, 0.),
                };
                let stretch = self.thumb_stretch("ctx-thumb", raw.map(|_| stretch), window);
                let mut track = div().id("ctx-slider").relative().w(px(STEP * (n - 1) as f32 + KNOB)).h(px(KNOB + 20.))
                    .child(div().absolute().top(px(KNOB / 2. - 1.)).left(px(KNOB / 2.)).w(px(STEP * (n - 1) as f32)).h(px(2.)).rounded_full().bg(p.border));
                for slot in 0..n {
                    let i = if rtl { n - 1 - slot } else { slot };
                    let t = tiers[i].clone();
                    let x = slot as f32 * STEP;
                    let ch = choices.clone();
                    track = track
                        .child(div().id(SharedString::from(format!("pick-tier-{t}"))).role(Role::Button).aria_label(t.clone()).absolute().left(px(x)).top(px(0.)).size(px(KNOB)).flex().items_center().justify_center().cursor_pointer()
                            .on_mouse_down(MouseButton::Left, cx.listener(move |a, ev: &MouseDownEvent, _, cx| {
                                a.ctx_drag = Some((f32::from(ev.position.x), x, x, ch.clone()));
                                cx.stop_propagation();
                                cx.notify();
                            }))
                            .child(div().size(px(6.)).rounded_full().bg(p.muted.opacity(0.5))))
                        .child(div().absolute().top(px(KNOB + 4.)).left(px(x + KNOB / 2. - 32.)).w(px(64.)).flex().justify_center().text_xs()
                            .text_color(if at == Some(i) { p.text } else { p.muted }).child(t));
                }
                if at.is_some() || self.ctx_drag.is_some() {
                    let held = self.ctx_drag.is_some();
                    let g = self.swell("ctx-thumb", held, 3., Some(window));
                    let q = lh_gui::slide::squash(stretch);
                    track = track.child(self.glass(div().absolute().top(px(-g + q / 2.)).left(px(left - g)).w(px(KNOB + stretch + g * 2.)).h(px(KNOB + g * 2. - q)).rounded_full(), "ctx-thumb", held, (KNOB - q) / 2. + g)
                        .child(div().size_full().rounded_full().border_2().border_color(p.accent.opacity(if held { 0.5 } else { 0.85 }))));
                }
                col = col.child(div().flex().flex_col().gap_1p5().child(div().text_xs().text_color(p.muted).child(if rtl { "طول السياق" } else { "Context length" })).child(div().px_1().child(track)));
            }
            let tpls = g.templates();
            if tpls.len() > 1 {
                let chips = tpls.iter().map(|t| self.button(format!("pick-tpl-{t}"), t.clone(), if here && *t == now.tpl { Kind::Primary } else { Kind::Normal }, cx, pick(now.tier.clone(), t.clone(), mode.clone()))).collect();
                col = col.child(section(if rtl { "القالب" } else { "Template" }, chips));
            }
            let on_off = info.modes.iter().any(|m| m == "instruct") && info.modes.iter().any(|m| m == "think" || m == "code");
            if on_off {
                let thinking_mode = info.modes.iter().find(|m| *m != "instruct").cloned().unwrap_or_default();
                let effective = if mode.is_empty() { info.default_mode.clone() } else { mode.clone() };
                let on = effective != "instruct";
                let to = if on { "instruct".to_string() } else { thinking_mode };
                let to = if to == info.default_mode { String::new() } else { to };
                let f = pick(now.tier.clone(), now.tpl.clone(), to);
                let switch = self.glass_switch("pick-thinking", on, true, if rtl { "التفكير" } else { "Thinking" }, true, cx, move |a, _, w, cx| f(a, w, cx));
                col = col.child(switch);
            } else if !info.modes.is_empty() {
                let dflt = if info.default_mode.is_empty() { "Default".to_string() } else { format!("{} (default)", mode_word(&info.default_mode)) };
                let mut chips = vec![self.button("pick-mode-default", dflt, if here && mode.is_empty() { Kind::Primary } else { Kind::Normal }, cx, pick(now.tier.clone(), now.tpl.clone(), String::new()))];
                for m in info.modes.iter().filter(|m| **m != info.default_mode) {
                    chips.push(self.button(format!("pick-mode-{m}"), mode_word(m), if here && mode == *m { Kind::Primary } else { Kind::Normal }, cx, pick(now.tier.clone(), now.tpl.clone(), m.clone())));
                }
                col = col.child(section(if rtl { "الوضع" } else { "Mode" }, chips));
            }
            {
                let stops = self.stops();
                let titles: Vec<String> = (0..self.stop_count()).map(|i| self.stop_title(i)).collect();
                let chips = (0..titles.len()).map(|i| {
                    let base = g.base.clone();
                    self.button(format!("pick-stop-{i}"), titles[i].clone(), if lh_gui::tiers::is_shown_stop(&stops, &g.base, i) { Kind::Primary } else { Kind::Normal }, cx, move |a, _, cx| a.set_stop(i, &base, cx))
                }).collect();
                col = col.child(section(if rtl { "مكانه على المنزلق" } else { "Its stop on the slider" }, chips));
            }
            col.into_any_element()
        });

        Some(
            self.appear(
            div()
                .id("model-picker")
                .role(Role::Dialog)
                .aria_label(if rtl { "اختر نموذجًا" } else { "Choose a model" })
                .absolute()
                .bottom(px(40.))
                .when(rtl, |d| d.left(px(0.)))
                .when(!rtl, |d| d.right(px(0.)))
                .w(px(600.))
                .max_h(px(560.))
                .overflow_hidden()
                .flex()
                .flex_col()
                .elevated(&p, 2)
                .on_mouse_down_out(cx.listener(|a, _, _, cx| {
                    if a.ctx_drag.is_none() {
                        a.model_menu = false;
                        cx.notify();
                    }
                }))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                // the wheel scrolls the picker's lists only, never the chat behind it
                .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                .capture_key_down(cx.listener(|a, ev: &KeyDownEvent, _, cx| {
                    match ev.keystroke.key.as_str() {
                        "down" => a.picker_step(true, cx),
                        "up" => a.picker_step(false, cx),
                        _ => return,
                    }
                    cx.stop_propagation();
                    cx.notify();
                }))
                .child(div().p_2().child(Input::new(&self.model_search)))
                .child(
                    div()
                        .flex()
                        .when(rtl, |d| d.flex_row_reverse())
                        .min_h(px(0.))
                        .flex_1()
                        .child(div().id("pick-list").role(Role::ListBox).aria_label(if rtl { "النماذج" } else { "Models" }).w(px(230.)).flex().flex_col().gap_0p5().p_1().overflow_y_scroll().children(list).children(none))
                        .children(detail),
                )
                , "model-picker"),
        )
    }

    /// A chat title on one line. A right-to-left title is cut by characters with "…" in the text, so the dots land
    /// on the left; GPUI's own ellipsis would sit on the right, the wrong end.
    fn chat_title(&self, title: &str, width: f32) -> AnyElement {
        let t = lh_gui::model::one_line(title);
        if lh_gui::rtl::paragraph_is_rtl(&t) {
            let max = (((width - 70.) / 8.5).max(8.)) as usize;
            div().overflow_hidden().whitespace_nowrap().child(lh_gui::model::truncate_title(&t, max)).into_any_element()
        } else {
            div().truncate().child(t).into_any_element()
        }
    }

    /// One chat in the sidebar: its title, then what it is doing or how old it is. Right-click opens its menu.
    fn side_chat_row(&mut self, c: &lh_gui::model::Chat, width: f32, now: i64, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let rtl = self.rtl;
        let key = c.key;
        let on = self.chats.current().map(|x| x.key) == Some(key);
        let pinned = c.session.as_ref().is_some_and(|s| self.prefs.pinned.contains(s));
        let (state, color) = if !c.asks.is_empty() {
            ((if rtl { "● يسأل" } else { "● asks you" }).to_string(), p.danger)
        } else if c.busy {
            ("…".to_string(), p.accent)
        } else {
            (lh_gui::sidebar::age(c.last, now), p.muted)
        };
        let title = format!("{}{}{}", if pinned { "★ " } else { "" }, if c.incognito { "🔒 " } else { "" }, c.title);
        let row = div()
            .id(SharedString::from(format!("chat-{key}")))
            .role(Role::ListItem)
            .aria_label(c.title.clone())
            .aria_selected(on)
            .flex()
            .when(rtl, |d| d.flex_row_reverse())
            .items_center()
            .gap_2()
            .px_2()
            .py_1()
            .rounded(p.inner)
            .cursor_pointer()
            .text_sm()
            .text_color(if c.dormant { p.muted } else { p.text })
            .bg(if on { p.accent.opacity(0.13) } else { hsla(0., 0., 0., 0.) })
            .hover(|s| s.bg(p.button))
            .on_click(cx.listener(move |a, _, _, cx| {
                a.chat_menu = None;
                a.select_chat(key, cx)
            }))
            .on_mouse_down(MouseButton::Right, cx.listener(move |a, _, _, cx| {
                a.chat_menu = if a.chat_menu == Some(key) { None } else { Some(key) };
                a.confirm_delete = None;
                cx.notify();
            }))
            .child(div().flex_1().min_w_0().overflow_hidden().child(self.chat_title(&title, width)))
            .child(div().flex_none().text_xs().text_color(color).child(state));
        if self.chat_menu != Some(key) {
            return row.into_any_element();
        }
        // the menu opens under the row, so it never covers the list or runs off the window
        let mut items = vec![];
        if c.session.is_some() {
            items.push(self.button(format!("pin-{key}"), if pinned { if rtl { "إلغاء التثبيت" } else { "Unpin" } } else if rtl { "تثبيت" } else { "Pin" }, Kind::Ghost, cx, move |a, _, cx| a.toggle_pin(key, cx)));
            for sz in lh_gui::sidebar::Snooze::ALL {
                items.push(self.button(format!("snooze-{key}-{sz:?}"), sz.label(rtl), Kind::Ghost, cx, move |a, _, cx| a.snooze(key, sz, cx)));
            }
        }
        let arming = self.confirm_delete == Some(key);
        items.push(self.button(format!("del-{key}"), if arming { if rtl { "تأكيد الحذف" } else { "Really delete" } } else if rtl { "حذف" } else { "Delete" }, if arming { Kind::Danger } else { Kind::Ghost }, cx, move |a, _, cx| a.delete_chat(key, cx)));
        div()
            .flex()
            .flex_col()
            .child(row)
            .child(div().id(SharedString::from(format!("chat-menu-{key}"))).role(Role::Menu).flex().flex_col().gap_0p5().ml_3().mr_1().my_1().p_1().text_sm().elevated(&p, 2).children(items))
            .into_any_element()
    }

    /// The folded sidebar: the same panel, narrow, with each icon where the wide sidebar has it.
    fn sidebar_compact(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let rtl = self.rtl;
        let drafting = self.chats.current().is_some_and(|c| c.is_draft()) && self.page() == &Page::Chat;
        let top = vec![
            self.nav_row_c("new-chat", icons::pencil(), if rtl { "محادثة جديدة" } else { "New chat" }, drafting, true, cx, |a, w, cx| {
                a.new_chat(cx);
                a.composer.update(cx, |s, cx| s.focus(w, cx));
            }),
            self.nav_row_c("side-search", icons::search(), if rtl { "بحث (Ctrl+K)" } else { "Search (Ctrl+K)" }, false, true, cx, |a, w, cx| a.toggle_palette(w, cx)),
        ];
        let foot = vec![
            self.nav_row_c("side-departments", icons::rail(RailItem::Departments), RailItem::Departments.label(rtl), self.page() == &Page::Departments, true, cx, |a, _, cx| a.open_page(Page::Departments, cx)),
            self.nav_row_c("side-memory", icons::rail(RailItem::Memory), RailItem::Memory.label(rtl), self.page() == &Page::Memory, true, cx, |a, _, cx| a.open_page(Page::Memory, cx)),
            self.nav_row_c("side-settings", icons::rail(RailItem::Settings), RailItem::Settings.label(rtl), self.settings_open, true, cx, |a, w, cx| a.toggle_settings(w, cx)),
        ];
        div().id("sidebar").role(Role::Navigation).aria_label(if rtl { "المحادثات" } else { "Chats" })
            .flex().flex_col().gap_0p5().w(px(RAIL_WIDTH)).flex_none().h_full().p_2().panel(&p, false)
            .child(self.sidebar_head(true, cx))
            .children(top).child(div().flex_1()).child(div().flex().flex_col().gap_0p5().pt_2().children(foot))
            .into_any_element()
    }

    /// The top of the sidebar, as in ChatGPT and claude.ai. Wide: the mark at the start and a fold button at the end.
    /// Folded: the mark alone, which turns into the open icon under the pointer; either click toggles (Ctrl+B too).
    fn sidebar_head(&self, compact: bool, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let rtl = self.rtl;
        let t = lh_gui::chrome::SidebarToggle::new(compact, rtl);
        let icon = if compact { icons::sidebar_expand(rtl) } else { icons::sidebar_collapse(rtl) };
        // the slot is as wide as a rail icon (16), so the mark centres on the same column as the icons below
        let logo = || div().flex_none().w(px(16.)).h(px(20.)).flex().items_center().justify_center().child(lh_gui::brand::mark(20., 1., p.mark));
        let toggle = move |a: &mut App, _: &mut Window, cx: &mut Context<App>| {
            a.layout_mut().toggle_left();
            a.persist();
            cx.notify();
        };
        let head = div().flex().when(rtl, |d| d.flex_row_reverse()).items_center().h(px(36.)).flex_none();
        if compact {
            // the mark and the open icon share one spot; hovering the button swaps them
            // same box as `nav_row_c`: 10px sides around a 16px icon slot, so the hover square matches the rail buttons
            let swap = div().relative().w(px(16.)).h(px(20.))
                .child(logo().absolute().top_0().left_0().group_hover("sidebar-toggle", |s| s.opacity(0.)))
                .child(div().absolute().top_0().left_0().size_full().flex().items_center().justify_center().opacity(0.).group_hover("sidebar-toggle", |s| s.opacity(1.)).child(icon_view(&p, icon, p.muted, 16.)));
            let button = div().id("side-toggle").group("sidebar-toggle").role(Role::Button).aria_label(SharedString::from(t.aria))
                .px_2p5().h(px(36.)).flex_none().flex().items_center().rounded(p.inner).cursor_pointer().hover(|s| s.bg(p.button))
                .on_click(cx.listener(move |a, _, w, cx| toggle(a, w, cx)))
                .child(swap);
            head.child(button).into_any_element()
        } else {
            let fold = self.icon_button("side-toggle", icon, t.aria, cx, toggle);
            head.px_2p5().child(logo()).child(div().flex_1()).child(fold).into_any_element()
        }
    }

    /// The sidebar, as in the Quiet desk design: New chat and search, then Needs you, Pinned, Active, the projects,
    /// and Earlier; Departments, Memory and Settings at the bottom. Sorting lives in `lh_gui::sidebar`.
    fn sidebar(&mut self, width: f32, cx: &mut Context<Self>) -> AnyElement {
        use lh_gui::sidebar::{group, Facts, Section, EARLIER_SHOWN, SETTLE_DAYS};
        let p = self.pal.clone();
        let rtl = self.rtl;
        let now = lh_gui::model::unix_now();
        let drafting = self.chats.current().is_some_and(|c| c.is_draft()) && self.page() == &Page::Chat;
        let new = self.nav_row("new-chat", icons::pencil(), if rtl { "محادثة جديدة" } else { "New chat" }, drafting, cx, |a, w, cx| {
            a.new_chat(cx);
            a.composer.update(cx, |s, cx| s.focus(w, cx));
        });
        let search = self.icon_button("side-search", icons::search(), if rtl { "بحث (Ctrl+K)" } else { "Search (Ctrl+K)" }, cx, |a, w, cx| a.toggle_palette(w, cx));
        // the fold button (Ctrl+B) is at the top of the sidebar, beside the mark
        let head = self.sidebar_head(false, cx);

        // departments: one waiting for an answer goes to Needs you, one at work to Active
        let on_page = self.page().clone();
        let (mut dept_wait, mut dept_work) = (vec![], vec![]);
        for d in self.panels.departments.clone() {
            let name = d.name.clone();
            let waiting = self.panels.runs.values().any(|r| r.dept == d.name && r.is_waiting());
            let working = self.panels.runs.values().any(|r| r.dept == d.name && !r.finished);
            if !waiting && !working {
                continue;
            }
            let label = format!("{}  {}", d.name, if waiting { if rtl { "● بانتظارك" } else { "● waiting for you" } } else if rtl { "… يعمل" } else { "… working" });
            let open = on_page == Page::Department(d.name.clone());
            let b = self.button(format!("side-dept-{name}"), label, if open { Kind::Primary } else { Kind::Ghost }, cx, move |a, _, cx| a.open_department(&name, cx));
            if waiting { dept_wait.push(b) } else { dept_work.push(b) }
        }

        let facts: Vec<Facts> = self.chats.list.iter().filter(|c| !c.is_draft()).map(|c| Facts { key: c.key, session: c.session.clone(), last: c.last, project: c.project.clone(), needs_you: !c.asks.is_empty(), busy: c.busy }).collect();
        let settle = if self.prefs.settle_days == 0 { SETTLE_DAYS } else { self.prefs.settle_days };
        let mut sections = group(&facts, &self.prefs.pinned, &self.prefs.snoozed, now, settle);
        if !dept_wait.is_empty() && !sections.iter().any(|(s, _)| *s == Section::NeedsYou) {
            sections.insert(0, (Section::NeedsYou, vec![]));
        }
        if !dept_work.is_empty() && !sections.iter().any(|(s, _)| *s == Section::Active) {
            let at = sections.iter().position(|(s, _)| !matches!(s, Section::NeedsYou | Section::Pinned)).unwrap_or(sections.len());
            sections.insert(at, (Section::Active, vec![]));
        }
        let mut body: Vec<AnyElement> = vec![];
        for (sec, keys) in sections {
            let total = keys.len();
            let head_color = if sec == Section::NeedsYou { p.danger } else { p.muted };
            let head = if sec == Section::Earlier || sec == Section::NeedsYou { format!("{} · {}", sec.title(rtl), total + if sec == Section::NeedsYou { dept_wait.len() } else { 0 }) } else { sec.title(rtl) };
            body.push(div().px_2().pt_3().pb_0p5().text_xs().font_weight(gpui::FontWeight::MEDIUM).text_color(head_color).when(rtl, |d| d.text_right()).child(head).into_any_element());
            match sec {
                Section::NeedsYou => body.append(&mut dept_wait),
                Section::Active => body.append(&mut dept_work),
                _ => {}
            }
            let shown = if sec == Section::Earlier && !self.earlier_all { EARLIER_SHOWN.min(total) } else { total };
            for k in keys.into_iter().take(shown) {
                if let Some(c) = self.chats.list.iter().find(|c| c.key == k).cloned() {
                    body.push(self.side_chat_row(&c, width, now, cx));
                }
            }
            if sec == Section::Earlier && total > EARLIER_SHOWN {
                let label = if self.earlier_all { (if rtl { "عرض أقل" } else { "Show less" }).to_string() } else if rtl { format!("عرض {} أخرى", total - shown) } else { format!("Show {} more", total - shown) };
                body.push(self.button("earlier-more", label, Kind::Ghost, cx, |a, _, cx| {
                    a.earlier_all = !a.earlier_all;
                    cx.notify();
                }));
            }
        }
        if body.is_empty() {
            body.push(div().px_2().pt_3().text_sm().text_color(p.muted).child(if rtl { "لا محادثات بعد. ما تكتبه يبقى على هذا الحاسوب." } else { "No chats yet. What you write stays on this computer." }).into_any_element());
        }

        let foot = vec![
            self.nav_row("side-departments", icons::rail(RailItem::Departments), RailItem::Departments.label(rtl), self.page() == &Page::Departments, cx, |a, _, cx| a.open_page(Page::Departments, cx)),
            self.nav_row("side-memory", icons::rail(RailItem::Memory), RailItem::Memory.label(rtl), self.page() == &Page::Memory, cx, |a, _, cx| a.open_page(Page::Memory, cx)),
            self.nav_row("side-settings", icons::rail(RailItem::Settings), RailItem::Settings.label(rtl), self.settings_open, cx, |a, w, cx| a.toggle_settings(w, cx)),
        ];
        div()
            .id("sidebar")
            .role(Role::Navigation)
            .aria_label(if rtl { "المحادثات" } else { "Chats" })
            .flex()
            .flex_col()
            .gap_1()
            .w(px(width))
            .h_full()
            .p_2()
            .panel(&p, self.focus == Focus::Sidebar)
            .on_mouse_down(MouseButton::Left, cx.listener(|a, _, _, cx| a.focus_on(Focus::Sidebar, cx)))
            .child(head)
            .child(div().flex().when(rtl, |d| d.flex_row_reverse()).items_center().gap_0p5().child(div().flex_1().flex().flex_col().child(new)).child(search))
            .child(div().id("chat-list").role(Role::List).flex_1().min_h_0().flex().flex_col().gap_0p5().overflow_y_scroll().children(body))
            .child(div().flex().flex_col().gap_0p5().pt_2().children(foot))
            .into_any_element()
    }

    /// Something the assistant showed, in the chat where it was made: a card with its title, a button to open it
    /// beside the chat, and the thing itself. A small app runs right here; only one page runs at a time, so an older
    /// app (or one while a page is open beside the chat) waits behind a "Run here" button.
    fn shown_view(&mut self, key: u64, i: usize, c: &lh_gui::content::Content, newest_app: Option<usize>, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        use lh_gui::content::ContentKind;
        let p = self.pal.clone();
        let rtl = self.rtl;
        let it = c.clone();
        let beside = self.button(format!("shown-open-{key}-{i}"), if rtl { "فتح بجانب المحادثة" } else { "Open beside" }, Kind::Ghost, cx, move |a, _, cx| a.open_content(it.clone(), cx));
        let head = div().flex().when(rtl, |d| d.flex_row_reverse()).items_center().gap_2().child(div().flex_1().min_w_0().text_sm().font_weight(gpui::FontWeight::MEDIUM).text_color(p.text).overflow_hidden().child(c.title.clone())).child(div().flex_none().text_sm().text_color(p.muted).child(beside));
        let body = if c.kind == ContentKind::Html {
            let place = format!("chat-{key}-{i}");
            let chosen = self.inline_pick.map(|(k, j)| k == key && j == i).unwrap_or(Some(i) == newest_app);
            let live = (chosen && !self.panel_page_on && !self.page_parked.contains(&place)) || self.page_live_at(c, &place);
            match live.then(|| self.page_element(c, &place, window, cx)).flatten() {
                Some(el) => el,
                None => {
                    let why = self.page_reason(c).unwrap_or_else(|| if self.panel_page_on { "A page is open beside the chat; one runs at a time.".into() } else { "Not running now; one app runs at a time.".into() });
                    let wake = place.clone();
                    let run = (!self.prefs.pages_off).then(|| self.button(format!("shown-run-{key}-{i}"), if rtl { "تشغيل هنا" } else { "Run here" }, Kind::Primary, cx, move |a, _, cx| {
                        a.page_parked.remove(&wake);
                        a.inline_pick = Some((key, i));
                        if a.panel_page_on {
                            a.layout_mut().hide_right();
                        }
                        cx.notify();
                    }));
                    match self.page_still(&place) {
                        // a stopped app keeps its last picture, with Run over it
                        Some((still, h)) if self.page_reason(c).is_none() => div().relative().w_full().h(px(h.max(120.))).overflow_hidden().rounded(p.inner).border_1().border_color(p.border).bg(gpui::white())
                            .child(img(still).absolute().top_0().left_0().size_full().object_fit(ObjectFit::Fill).opacity(0.85))
                            .child(div().absolute().bottom(px(10.)).left_0().right_0().flex().justify_center().children(run))
                            .into_any_element(),
                        _ => div().flex().flex_col().items_center().justify_center().gap_2().h(px(120.)).rounded(p.inner).bg(p.sunken).text_sm().text_color(p.muted).child(why).children(run).into_any_element(),
                    }
                }
            }
        } else {
            // long text keeps the chat moving: it scrolls inside its card, and "Open beside" gives it room
            let tall = matches!(c.kind, ContentKind::Markdown | ContentKind::Code | ContentKind::Text);
            div().id(SharedString::from(format!("shown-body-{key}-{i}"))).when(tall, |d| d.max_h(px(420.)).overflow_y_scroll()).child(crate::panels::viewer(c, &format!("chat-{key}-{i}"), &p)).into_any_element()
        };
        div()
            .id(SharedString::from(format!("shown-{key}-{i}")))
            .role(Role::Group)
            .aria_label(c.title.clone())
            .w_full()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .rounded(p.radius)
            .border_1()
            .border_color(p.border)
            .bg(p.surface)
            .child(head)
            .child(body)
            .into_any_element()
    }

    fn block_view(&self, key: u64, i: usize, b: &Block, _cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let rtl = self.rtl;
        match b {
            Block::User(t) => {
                let is_rtl = rtl::paragraph_is_rtl(t);
                div().flex().w_full().when(!rtl, |d| d.justify_end()).when(rtl, |d| d.justify_start()).child(
                    div().id(SharedString::from(format!("u{key}-{i}"))).role(Role::Label).max_w(relative(0.8)).px_3().py_2().rounded(p.inner).bg(p.bubble).when(p.material.panels != lh_gui::theme::GlassKind::None, |d| d.border_2().border_color(gpui::white().opacity(if p.dark { 0.35 } else { 0.85 }))).text_color(p.text).when(is_rtl, |d| d.text_right()).child(t.clone()),
                ).into_any_element()
            }
            Block::Files(names) => {
                let chips: Vec<AnyElement> = names.iter().enumerate().map(|(n, name)| self.file_chip(&format!("sent{key}-{i}-{n}"), name, None, _cx)).collect();
                div().flex().w_full().when(!rtl, |d| d.justify_end()).when(rtl, |d| d.justify_start()).child(div().flex().when(rtl, |d| d.flex_row_reverse()).flex_wrap().gap_1().max_w(relative(0.8)).children(chips)).into_any_element()
            }
            Block::Assistant { .. } | Block::Tool { .. } => div().into_any_element(),
            Block::Shown(_) => div().into_any_element(), // drawn by `shown_view`
            Block::Notice(t) => div().w_full().flex().justify_center().text_sm().text_color(p.muted).child(t.clone()).into_any_element(),
            Block::Error(t) => div().px_3().py_2().rounded(p.inner).bg(p.danger.opacity(0.08)).border_1().border_color(p.danger.opacity(0.35)).text_sm().text_color(p.danger).child(t.clone()).into_any_element(),
        }
    }

    /// Settings as a window over the app: the rest is dimmed, the window is centred, a click outside or Escape closes it.
    fn settings_view(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.settings_open {
            return None;
        }
        let p = self.pal.clone();
        let rtl = self.rtl;
        self.tab_buttons = true;
        let body = self.settings_body(window, cx);
        let sidebar = self.settings_sidebar(cx);
        let close = self.button("settings-close", if rtl { "رجوع إلى المحادثة" } else { "Back to chat" }, Kind::Ghost, cx, |a, w, cx| a.close_settings(w, cx));
        self.tab_buttons = false;
        Some(
            self.appear(
            div()
                .id("settings-scrim")
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex()
                .justify_center()
                .items_center()
                .items_center()
                .p_2()
                .bg(hsla(0., 0., 0., if p.dark { 0.35 } else { 0.18 }))
                .flex()
                .justify_center()
                .child(
                    div()
                        .id("settings-window")
                        .role(Role::Dialog)
                        .aria_label(if rtl { "الإعدادات" } else { "Settings" })
                        .track_focus(&self.settings_focus)
                        .w_full()
                        .max_w(px(1080.))
                        .h(px(760.))
                        .max_h_full()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .p_3()
                        .elevated(&p, 3)
                        // clicks inside must not reach the dimmed area behind and close the window
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .child(div().flex().when(rtl, |d| d.flex_row_reverse()).items_center().gap_2().child(div().flex_1().text_base().font_weight(gpui::FontWeight::MEDIUM).text_color(p.text).child(if rtl { "الإعدادات" } else { "Settings" })).child(div().id("esc-keycap").role(Role::Label).aria_label(if rtl { "Esc للرجوع" } else { "Esc goes back" }).px_1p5().rounded(px(4.)).border_1().border_color(p.border).bg(p.button).text_xs().font_family(p.mono.clone()).text_color(p.muted).child("Esc")).child(close))
                        .child(
                            div().flex().flex_1().min_h_0().when(rtl, |d| d.flex_row_reverse()).gap_3()
                                .child(div().flex().flex_col().flex_none().gap_2().child(div().w(px(220.)).child(Input::new(&self.settings_search))).child(sidebar))
                                .child(div().id("settings-body").flex_1().min_w_0().overflow_y_scroll().flex().flex_col().gap_2().child(body)),
                        ),
                )
                , "settings"),
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
            self.appear(
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
                        .elevated(&p, 2)
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .child(Input::new(&self.palette_input))
                        .child(div().id("palette-list").role(Role::ListBox).aria_label(if rtl { "الأوامر" } else { "Commands" }).flex().flex_col().gap_0p5().children(rows).children(none)),
                )
                , "palette"),
        )
    }

    /// The model's words as plain text in the column, no bubble.
    fn answer_view(&self, key: u64, i: usize, text: &str, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let is_rtl = rtl::paragraph_is_rtl(text);
        let content = crate::panels::Content::detect(&self.chats.list.iter().find(|c| c.key == key).map(|c| c.title.clone()).unwrap_or_default(), text);
        let open_btn = self.button(format!("open-{key}-{i}"), "Open in panel", Kind::Ghost, cx, move |a, _, cx| a.open_content(content.clone(), cx));
        div().id(SharedString::from(format!("a{key}-{i}"))).role(Role::Label).w_full().flex().flex_col().gap_2().text_color(p.text).when(is_rtl, |d| d.text_right())
            .child(TextView::markdown(SharedString::from(format!("md{key}-{i}")), lh_gui::content::keep_line_breaks(text)))
            .when(lh_gui::content::Content::worth_opening(text), |d| d.child(div().flex().when(is_rtl, |d| d.flex_row_reverse()).child(open_btn)))
            .into_any_element()
    }

    /// Thinking and tool calls as one line that opens to the list of steps.
    /// `thinking` is the live progress of the model's thinking, for the stretch of work that is still going.
    fn work_view(&self, key: u64, work: (usize, &[Step], Option<u64>, bool), blocks: &[Block], thinking: Option<Snap>, cx: &mut Context<Self>) -> AnyElement {
        let (first, steps, secs, running) = work;
        let p = self.pal.clone();
        let rtl = self.rtl;
        let open = self.open_blocks.contains(&(key, first));
        // while it thinks the line says how much, how fast and for how long; a screen reader hears it only now and then
        let label = match thinking {
            Some(s) => progress_text(s, rtl),
            None => work_label(secs, steps.len(), running, rtl),
        };
        let spoken = thinking.map(|s| spoken_text(s, rtl)).unwrap_or_else(|| label.clone());
        let head = div()
            .id(SharedString::from(format!("work-{key}-{first}")))
            .role(Role::Button)
            .aria_label(spoken.clone())
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
            .child(icon_view(&p, icons::Icon::new(match (open, rtl) { (true, _) => "chevron-down", (false, false) => "chevron-right", (false, true) => "chevron-left" }, chevron(open, rtl)), p.muted, 12.))
            .child(label)
            .when(thinking.is_some(), |d| d.child(div().id(SharedString::from(format!("think-status-{key}"))).role(Role::Status).aria_label(spoken).w(px(0.)).h(px(0.))));
        let live_block = if thinking.is_some() { steps.iter().rev().find(|s| s.kind == StepKind::Thinking).map(|s| s.block) } else { None };
        let mut rows: Vec<AnyElement> = vec![];
        if open {
            for s in steps {
                match (&s.kind, blocks.get(s.block)) {
                    (StepKind::Thinking, Some(Block::Assistant { reasoning, .. })) => {
                        // plain muted text in a box of its own with a height limit; the one still being written follows its end
                        let mut text = div().id(SharedString::from(format!("think-text-{key}-{}", s.block))).max_h(px(240.)).overflow_y_scroll().text_color(p.muted).when(rtl, |d| d.text_right()).child(reasoning.clone());
                        if live_block == Some(s.block) {
                            text = text.track_scroll(&self.think_scroll);
                        }
                        rows.push(div().flex().flex_col().gap_0p5().child(div().text_color(p.text).child(if rtl { "تفكير" } else { "Thinking" })).child(text).into_any_element());
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
            .when(open, |d| d.child(div().id(SharedString::from(format!("steps-{key}-{first}"))).role(Role::List).aria_label(if rtl { "الخطوات" } else { "Steps" }).flex().flex_col().gap_1().p_2().rounded(p.inner).bg(p.sunken).text_sm().text_color(p.muted).font_family(p.mono.clone()).children(rows)))
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

    fn chat_view(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let rtl = self.rtl;
        let Some(chat) = self.chats.current().cloned() else { return div().into_any_element() };
        let snap = if chat.busy { chat.think.snapshot(std::time::Instant::now()) } else { None };
        let newest_app = chat.blocks.iter().rposition(|b| matches!(b, Block::Shown(c) if c.kind == lh_gui::content::ContentKind::Html));
        // what was already in the chat when it was opened shows at once; what arrives after fades and rises in
        if self.rise_from.0 != chat.key {
            self.rise_from = (chat.key, chat.blocks.len());
        }
        let rise_from = self.rise_from.1;
        let blocks: Vec<AnyElement> = transcript::items(&chat.blocks, chat.busy)
            .into_iter()
            .map(|item| {
                let at = match &item { Item::Block(i) | Item::Answer(i) => *i, Item::Work { first, .. } => *first };
                (at, item)
            })
            .map(|(at, item)| (at, match item {
                Item::Block(i) => match &chat.blocks[i] {
                    Block::Shown(c) => self.shown_view(chat.key, i, c, newest_app, window, cx),
                    b => self.block_view(chat.key, i, b, cx),
                },
                Item::Answer(i) => match &chat.blocks[i] {
                    Block::Assistant { text, .. } => self.answer_view(chat.key, i, text, cx),
                    _ => div().into_any_element(),
                },
                Item::Work { first, steps, secs, running } => self.work_view(chat.key, (first, &steps, secs, running), &chat.blocks, snap.filter(|_| running), cx),
            }))
            .collect::<Vec<_>>()
            .into_iter()
            .map(|(at, el)| if at >= rise_from { self.rise(el, chat.key, at) } else { el })
            .collect();
        let empty = chat.blocks.is_empty() && !chat.dormant;
        let mut stack: Vec<AnyElement> = vec![];
        if let Some(b) = &self.banner {
            stack.push(div().px_3().py_2().rounded(p.inner).bg(p.danger.opacity(0.08)).border_1().border_color(p.danger.opacity(0.35)).text_sm().text_color(p.danger).child(b.clone()).into_any_element());
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
                    // a file write shows what it changes, in the code font, scrolling when long
                    .when(!a.detail.is_empty(), |d| {
                        d.child(
                            div().id(SharedString::from(format!("ask-diff-{}", a.id))).max_h(px(320.)).overflow_y_scroll().p_2().rounded(p.inner).bg(p.sunken)
                                .font_family(p.mono.clone()).text_xs().text_color(p.text).role(Role::Label).aria_label(format!("The change: {}", a.detail.lines().next().unwrap_or("")))
                                .children(a.detail.lines().map(|l| div().whitespace_nowrap().child(if l.is_empty() { " ".to_string() } else { l.to_string() }))),
                        )
                    })
                    .when(!a.reason.is_empty(), |d| d.child(div().text_sm().text_color(p.muted).child(a.reason.clone())))
                    .child(div().flex().when(rtl, |d| d.flex_row_reverse()).flex_wrap().gap_2().child(yes).child(no).children(always)).into_any_element(),
            );
        }
        if let Some(pen) = self.pending.clone().filter(|x| x.chat == chat.key) {
            stack.push(self.pending_card(&pen, cx));
        } else if chat.dormant {
            stack.push(div().text_sm().text_color(p.muted).child("This chat is stored. Choosing it asks before the model reads it again.").into_any_element());
        }
        // one message box holds everything about the next message: the text, the kind of chat (until the first
        // message), the model and Private. Private turns its border the accent colour.
        let fresh = chat.session.is_none() && chat.blocks.is_empty() && !chat.dormant;
        let mut bar: Vec<AnyElement> = vec![];
        if fresh {
            const SEG: f32 = 96.;
            let modes = [(Mode::Chat, if rtl { "محادثة" } else { "Chat" }), (Mode::Assistant, if rtl { "مساعد" } else { "Assistant" }), (Mode::Project, if rtl { "مشروع" } else { "Project" })];
            let idx = modes.iter().position(|(m, _)| *m == chat.mode).unwrap_or(0);
            let slot = if rtl { modes.len() - 1 - idx } else { idx };
            let (now, dur) = (std::time::Instant::now(), motion::panel(p.anim));
            let raw = self.mode_drag.map(|d| d.2);
            let target = raw.map(|r| lh_gui::slide::snap(r, SEG, 3)).unwrap_or(slot) as f32 * SEG;
            self.mode_glide.go(target, now, dur);
            if self.mode_glide.moving(now, dur) {
                window.request_animation_frame();
            }
            let held = self.mode_drag.is_some();
            let drawn = self.mode_glide.value(now, dur);
            let (left, stretch) = match raw {
                Some(r) => { let h = lh_gui::slide::held(r, drawn, SEG, 3, MODE_H); (h.left, h.stretch) }
                None => (drawn, 0.),
            };
            let stretch = self.thumb_stretch("mode-thumb", self.mode_drag.map(|_| stretch), window);
            // held, the drop swells a little and stretches toward the pull
            let grow = self.swell("mode-thumb", held, 2., Some(window));
            let q = lh_gui::slide::squash(stretch);
            // the drop sits 2 px inside the bar on every side, and its corner is the bar's less that inset
            let corner = ((f32::from(p.inner) - 2.).max(3.)).min((MODE_H - q) / 2.) + grow;
            let thumb = self.glass(div().absolute().top(px(2. - grow + q / 2.)).left(px(2. + left - grow)).w(px(SEG + stretch + grow * 2.)).h(px(MODE_H + grow * 2. - q)).rounded(px(corner)), "mode-thumb", held, corner);
            let seg: Vec<AnyElement> = modes
                .into_iter()
                .map(|(m, label)| {
                    let on = chat.mode == m;
                    div().id(SharedString::from(format!("mode-{label}"))).role(Role::Button).aria_label(label).aria_selected(on).w(px(SEG)).h(px(MODE_H)).flex().items_center().justify_center().rounded(p.inner).cursor_pointer()
                        .text_color(if on { p.text } else { p.muted })
                        .hover(|s| s.text_color(p.text))
                        .when(on, |d| d.font_weight(gpui::FontWeight::MEDIUM))
                        .when(on, |d| d.on_mouse_down(MouseButton::Left, cx.listener(move |a, ev: &MouseDownEvent, _, cx| {
                            let x = slot as f32 * SEG;
                            a.mode_drag = Some((f32::from(ev.position.x), x, x));
                            cx.notify();
                        })))
                        .on_click(cx.listener(move |a, _, _, cx| a.set_mode(m, cx)))
                        .child(label).into_any_element()
                })
                .collect();
            // the labels sit over the glass, so the drop is drawn first
            bar.push(div().id("chat-kind").role(Role::Group).aria_label("Kind of chat").relative().flex().text_sm().p(px(2.)).rounded(p.inner).bg(p.sunken.opacity(0.7)).child(thumb).children(if rtl { seg.into_iter().rev().collect::<Vec<_>>() } else { seg }).into_any_element());
        }
        let mut slider_card: Option<AnyElement> = None;
        // the model pill sits in the composer row just left of the send button, so the picker opens beside it
        let mut model_pill: Option<AnyElement> = None;
        let model_label = { let l = self.models.label_of(&chat.model); if l.is_empty() { chat.model.clone() } else { l } };
        if !model_label.is_empty() {
            let stops = self.stops();
            let base = self.models.group_of(&chat.model).map(|g| g.base.clone()).unwrap_or_default();
            let at = lh_gui::tiers::stop_of(&stops, &base);
            if stops.iter().any(|s| !s.is_empty()) {
                let gap = SLIDER_GAP;
                let n = self.stop_count();
                let titles: Vec<String> = (0..n).map(|i| self.stop_title(i)).collect();
                let (now, dur) = (std::time::Instant::now(), motion::panel(p.anim));
                if let Some(i) = at {
                    let slot = if rtl { n - 1 - i } else { i };
                    if self.slider_drag.is_none() {
                        self.tier_glide.go(slot as f32 * gap, now, dur);
                    }
                }
                // held: the thumb eases to the stop nearest the pointer and stretches toward the pull; let go: it
                // rests on the chosen stop
                let raw = self.slider_drag.map(|d| d.2);
                if let Some(r) = raw {
                    self.tier_glide.go(lh_gui::slide::snap(r, gap, n) as f32 * gap, now, dur);
                }
                if self.tier_glide.moving(now, dur) {
                    window.request_animation_frame();
                }
                let drawn = self.tier_glide.value(now, dur);
                let (knob_left, stretch) = match raw {
                    Some(r) => { let h = lh_gui::slide::held(r, drawn, gap, n, SLIDER_KNOB); (h.left, h.stretch) }
                    None => (drawn, 0.),
                };
                let stretch = self.thumb_stretch("model-thumb", self.slider_drag.map(|_| stretch), window);
                let thumb_at = at.map(|i| if rtl { n - 1 - i } else { i });
                const KNOB: f32 = SLIDER_KNOB;
                let dots: Vec<AnyElement> = (0..n).map(|i| {
                    let slot = if rtl { n - 1 - i } else { i };
                    let name = format!("{}: {}", titles[i], self.models.label_of(&stops[i]));
                    div().id(SharedString::from(format!("stop-{i}"))).role(Role::Button).aria_label(name).absolute().left(px(slot as f32 * gap)).top(px(0.)).size(px(KNOB)).flex().items_center().justify_center().cursor_pointer()
                        .on_mouse_down(MouseButton::Left, cx.listener(move |a, ev: &MouseDownEvent, _, cx| {
                            // the thumb's own stop starts a drag; another stop is chosen and the thumb glides there
                            if thumb_at == Some(slot) {
                                let x = slot as f32 * gap;
                                a.slider_drag = Some((f32::from(ev.position.x), x, x));
                            } else {
                                a.choose_stop(i, cx);
                            }
                            cx.stop_propagation();
                            cx.notify();
                        }))
                        .child(div().size(px(6.)).rounded_full().bg(p.muted.opacity(0.5)))
                        .into_any_element()
                }).collect();
                let names: Vec<AnyElement> = (0..n).map(|i| {
                    let slot = if rtl { n - 1 - i } else { i };
                    let on = at == Some(i);
                    div().absolute().top(px(KNOB + 6.)).left(px(slot as f32 * gap + KNOB / 2. - 30.)).w(px(60.)).flex().justify_center().text_xs()
                        .text_color(if on { p.text } else { p.muted }).child(titles[i].clone()).into_any_element()
                }).collect();
                let track = div().id("model-slider").role(Role::Group).aria_label(if rtl { "منزلق النموذج" } else { "Model slider" }).relative().w(px(gap * (n - 1) as f32 + KNOB)).h(px(KNOB + 22.))
                    .child(div().absolute().top(px(KNOB / 2. - 1.)).left(px(KNOB / 2.)).w(px(gap * (n - 1) as f32)).h(px(2.)).rounded_full().bg(p.border))
                    .children(dots)
                    .children(names)
                    .when(at.is_some() || self.slider_drag.is_some(), |d| {
                        let held = self.slider_drag.is_some();
                        let g = self.swell("model-thumb", held, 3., Some(window));
                        let q = lh_gui::slide::squash(stretch);
                        d.child(self.glass(div().absolute().top(px(-g + q / 2.)).left(px(knob_left - g)).w(px(KNOB + stretch + g * 2.)).h(px(KNOB + g * 2. - q)).rounded_full(), "model-thumb", held, (KNOB - q) / 2. + g)
                            .child(div().size_full().rounded_full().border_2().border_color(p.accent.opacity(if held { 0.5 } else { 0.85 }))))
                    });
                let title = at.map(|i| format!("{} · {model_label}", titles[i])).unwrap_or(model_label.clone());
                if self.slider_open {
                    let head = div().id("slider-name").role(Role::Button).aria_label(if rtl { "اختر نموذجًا بنفسك" } else { "Pick a model yourself" })
                        .flex().when(rtl, |d| d.flex_row_reverse()).items_center().gap_1p5().px_2().py_1().rounded(p.inner).cursor_pointer().hover(|s| s.bg(p.button))
                        .on_click(cx.listener(|a, _, w, cx| a.toggle_model_menu(w, cx)))
                        .child(div().text_sm().font_weight(gpui::FontWeight::MEDIUM).text_color(p.text).child(title.clone()))
                        .child(icon_view(&p, if rtl { icons::chevron_left() } else { icons::chevron_right() }, p.muted, 14.));
                    let card = div().id("slider-card").flex().flex_col().items_center().gap_2().p_3()
                        .on_mouse_down_out(cx.listener(|a, _, _, cx| {
                            if a.slider_drag.is_none() {
                                a.slider_open = false;
                                cx.notify();
                            }
                        }))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .child(head)
                        .child(div().px_4().pt_2().pb_1().rounded(p.inner).bg(p.sunken.opacity(0.8)).child(track));
                    // held at the pill's end edge (right, or left in Arabic), which stays put when a longer model name widens
                    // the pill; `anchored` keeps the card inside the window on narrow windows
                    let at = if rtl { gpui::Anchor::BottomLeft } else { gpui::Anchor::BottomRight };
                    let held = div().absolute().bottom(px(40.)).when(rtl, |d| d.left_0()).when(!rtl, |d| d.right_0())
                        .child(gpui::anchored().anchor(at).snap_to_window_with_margin(px(8.)).child(self.appear(div().w(px(400.)).flex().justify_center().elevated(&p, 2).child(card), "slider-card")));
                    slider_card = Some(held.into_any_element());
                }
                let pill = self.pill("model", None, title, Some(icons::chevron_down()), self.slider_open, cx, |a, _, cx| {
                    a.slider_open = !a.slider_open;
                    a.model_menu = false;
                    cx.notify();
                });
                model_pill = Some(div().relative().child(pill).children(slider_card.take()).into_any_element());
            } else {
                model_pill = Some(self.pill("model", None, model_label, Some(icons::chevron_down()), false, cx, |a, w, cx| a.toggle_model_menu(w, cx)));
            }
        }
        let kind_hint = fresh.then(|| match chat.mode {
            Mode::Chat => if chat.incognito { "Nothing is remembered or stored" } else { "Plain conversation, no tools" },
            Mode::Assistant => "Works from your home folder and asks before changing anything",
            Mode::Project => "Works inside one folder and asks before changing anything",
        });
        // a round button: an arrow to send, a square to stop
        let action = div().id("send").role(Role::Button).aria_label(if chat.busy { "Stop" } else { "Send" }).size(px(34.)).flex_none().rounded(p.inner).flex().items_center().justify_center().cursor_pointer()
            .bg(if chat.busy { p.danger } else { p.text }).hover(|s| s.opacity(0.85))
            .on_click(cx.listener(|a, _, w, cx| a.send(w, cx)))
            .child(icon_view(&p, if chat.busy { icons::stop() } else { icons::send() }, if p.dark { hsla(0., 0., 0.08, 1.) } else { hsla(0., 0., 1., 1.) }, 18.));
        // after a minute or a couple of thousand tokens of thinking: a calm hint beside Stop. Nothing is ever stopped for the person.
        let hint = snap.filter(|s| is_long(*s)).map(|_| div().id("still-thinking").role(Role::Status).aria_label(hint_text(rtl)).text_xs().text_color(p.muted).max_w(px(150.)).pb_1p5().when(rtl, |d| d.text_right()).child(hint_text(rtl)));
        let lift = vec![
            BoxShadow { color: hsla(0., 0., 0., if p.dark { 0.45 } else { 0.10 }), offset: point(px(0.), px(10.)), blur_radius: px(30.), spread_radius: px(-8.), inset: false },
            BoxShadow { color: if chat.incognito { p.accent.opacity(0.7) } else { p.border.opacity(0.9) }, offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: px(1.), inset: false },
        ];
        // the picker is drawn from the model pill, above it and anchored to its end, so it opens beside its button
        let picker = self.menus(window, cx);
        let composer = div().relative().flex().flex_col().gap_2().px_3().pt_2().pb_2().rounded(p.radius).glass_fill(&p, p.radius).shadow(lift)
            .children(slider_card)
            .when(!self.staged.is_empty(), |d| {
                let chips: Vec<AnyElement> = self.staged.files.iter().enumerate().map(|(n, f)| self.file_chip(&format!("staged-{n}"), &format!("{} ({})", f.name, lh_client::attach::size_label(f.size)), Some(n), cx)).collect();
                d.child(div().flex().when(rtl, |d| d.flex_row_reverse()).flex_wrap().gap_1().children(chips))
            })
            .children(self.attach_note.clone().map(|n| div().text_xs().text_color(p.danger).when(rtl, |d| d.text_right()).child(n)))
            .child(Textarea::new(&self.composer).appearance(false).aria_label("Message"))
            .when(fresh && chat.mode == Mode::Project, |d| d.child(Input::new(&self.project_input)))
            .child(
                div().flex().when(rtl, |d| d.flex_row_reverse()).items_center().gap_1()
                    .child(self.icon_button("attach", icons::attach(), if rtl { "إرفاق ملفات" } else { "Attach files" }, cx, |a, _, cx| a.attach_dialog(cx)))
                    .children(bar)
                    .child(div().flex_1().min_w_0().px_2().truncate().text_xs().text_color(p.muted).when(rtl, |d| d.text_right()).children(kind_hint))
                    .children(hint)
                    .children(model_pill.map(|pill| div().relative().child(pill).children(picker).into_any_element()))
                    .child(action),
            );
        // the conversation reads in one column in the middle; the person chooses how wide
        let width = self.layout().column.max_px();
        let column = div().flex().flex_col().gap_4().w_full().when_some(width, |d, w| d.max_w(px(w))).children(blocks);
        let scroll = div().id("chat-scroll").role(Role::Log).aria_label("Conversation").flex_1().overflow_y_scroll().track_scroll(&self.scroll).flex().flex_col().items_center().p_3().child(column);
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
        // the new-chat screen: the message box slides between the middle (1) and the bottom (0), ease out
        let (now, dur) = (std::time::Instant::now(), motion::panel(p.anim));
        let centre = self.centre_reveal.width(if empty { 1. } else { 0. }, now, dur).clamp(0., 1.);
        if self.centre_reveal.animating(now, dur) {
            window.request_animation_frame();
        }
        let greeting = (empty && !chat.busy).then(|| {
            let hour = ((lh_gui::model::unix_now() + self.tz_offset).rem_euclid(86_400) / 3600) as u32;
            let text = lh_gui::greeting::greeting_at(&lh_gui::greeting::system_name(&self.prefs.name), hour, rtl);
            let sub = if chat.incognito { if rtl { "محادثة خاصة: لا يُحفظ شيء" } else { "Private chat: nothing is remembered or stored" } } else if rtl { "كل شيء يبقى على هذا الحاسوب" } else { "Everything stays on this computer" };
            // the brand mark: it appears looking at you, then glances at its slot (the logo pose), with the
            // timing sheet's in-out easing; with animation off it shows the logo pose at once
            let colours = p.mark;
            let glance_ms = lh_gui::brand::GLANCE_MS;
            let mark = if p.anim.is_zero() {
                div().child(lh_gui::brand::mark(56., 1., colours)).into_any_element()
            } else {
                let hold = 0.6;
                div().id("greeting-mark").size(px(56.))
                    .with_animation("greeting-glance", Animation::new(std::time::Duration::from_millis((glance_ms as f32 / (1. - hold)) as u64)), move |d, t| {
                        let g = lh_gui::brand::ease_in_out(((t - hold) / (1. - hold)).clamp(0., 1.));
                        d.child(lh_gui::brand::mark(56., g, colours))
                    })
                    .into_any_element()
            };
            div().id("greeting").role(Role::Heading).aria_label(text.clone()).pb_4().w_full().flex().flex_col().items_center().gap_1()
                .child(mark)
                .child(div().text_3xl().font_weight(gpui::FontWeight::MEDIUM).text_color(p.text).child(text))
                .child(div().text_sm().text_color(p.muted).child(sub))
        });
        let chips = (empty && !chat.busy).then(|| {
            let items: Vec<AnyElement> = lh_gui::greeting::suggestions(rtl)
                .into_iter()
                .map(|(label, text)| {
                    div().id(SharedString::from(format!("suggest-{label}"))).role(Role::Button).aria_label(label).cursor_pointer().px_3p5().py_1p5().relative().rounded(p.inner).glass_fill(&p, p.inner).text_sm().text_color(p.muted)
                        .hover(|s| s.bg(p.button))
                        .on_click(cx.listener(move |a, _, w, cx| {
                            a.composer.update(cx, |s, cx| {
                                s.set_value(text, w, cx);
                                s.focus(w, cx);
                            });
                            cx.notify();
                        }))
                        .child(label)
                        .into_any_element()
                })
                .collect();
            div().flex().when(rtl, |d| d.flex_row_reverse()).flex_wrap().justify_center().gap_2().pt_3().children(items)
        });
        let pane = div().relative().flex().flex_col().overflow_hidden().child(scroll).children(jump);
        // questions and the message box stay in view under the conversation, in the same column
        let beneath = div().flex().flex_col().items_center().px_3().child(div().flex().flex_col().w_full().when_some(width, |d, w| d.max_w(px(w))).children(greeting).child(div().flex().flex_col().gap_2().w_full().children(stack).child(composer)).children(chips));
        let mut top_gap = div();
        let mut pane = pane;
        let mut bottom_gap = div();
        // spacers above and below take the room the conversation gives up while the box is in the middle
        top_gap.style().flex_grow = Some(centre);
        bottom_gap.style().flex_grow = Some(centre);
        pane.style().flex_grow = Some(1. - centre);
        pane.style().flex_basis = Some(relative(0.).into());
        top_gap.style().flex_basis = Some(relative(0.).into());
        bottom_gap.style().flex_basis = Some(relative(0.).into());
        div().flex().flex_col().flex_1().h_full().gap_2().overflow_hidden()
            .on_mouse_down(MouseButton::Left, cx.listener(|a, _, _, cx| a.focus_on(Focus::Chat, cx)))
            .child(top_gap)
            .child(pane)
            .child(beneath)
            .child(bottom_gap)
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
        // the sidebar has words; when it is hidden (Ctrl+B or a narrow window) the icon rail takes its place
        let rail_w = if self.layout().left_visible { 0. } else { RAIL_WIDTH };
        // on a narrow window the side panel is laid over the chat instead of squeezing it
        let overlay = self.layout().overlays(total - 24. - rail_w, 420.);
        let mut flow = self.layout().clone();
        flow.right_visible &= !overlay;
        let (left_w, mut right_w) = flow.widths(total - 24. - rail_w, 420.);
        self.width.room = flow.middle_room(total - 24. - rail_w, 420.);
        if overlay {
            right_w = self.layout().overlay_width(total - 24. - rail_w);
        }

        let top = self.top_bar(cx);
        // the chat list and the side panel slide (ease out, 220 ms); with no animation they simply appear
        let (now, dur) = (std::time::Instant::now(), motion::panel(p.anim));
        let (left_now, right_now) = (self.left_reveal.width(left_w, now, dur), self.right_reveal.width(right_w, now, dur));
        if self.left_reveal.animating(now, dur) || self.right_reveal.animating(now, dur) {
            window.request_animation_frame();
        }
        let (left_full, right_full) = (self.left_reveal.full(), self.right_reveal.full());
        self.panel_page_on = right_w > 0. && self.layout().panel == Panel::Content && self.content.get(self.content_index.min(self.content.len().saturating_sub(1))).is_some_and(|c| c.kind == lh_gui::content::ContentKind::Html);
        // one sidebar: wide, or folded to its icons. While it moves it is the wide one cut to the moving width, never
        // narrower than the folded one, so the icons stay put
        let left = if left_now > RAIL_WIDTH { slide(left_now, left_full, self.sidebar(left_full, cx)) } else if left_now > 0. && self.layout().left_visible {
            slide(RAIL_WIDTH, left_full, self.sidebar(left_full, cx))
        } else {
            self.sidebar_compact(cx)
        };
        let middle = match self.page().clone() {
            Page::Chat => self.chat_view(window, cx),
            Page::Department(name) => self.department_page(&name, cx),
            Page::Departments => self.list_page(true, cx),
            Page::Memory => self.list_page(false, cx),
        };
        // a web page drawn inside the Opened tab is kept only while that tab is on screen; otherwise its helper ends
        let in_chat = |place: &str| self.page() == &Page::Chat && self.chats.current().is_some_and(|c| place.starts_with(&format!("chat-{}-", c.key)));
        let showing_page = match self.page.as_ref().map(|p| p.place.clone()) {
            Some(place) if place == "panel" => right_now > 0. && self.panel_page_on,
            Some(place) => in_chat(&place),
            None => false,
        };
        self.page_release_if_hidden(showing_page, window);
        let right = (right_now > 0.).then(|| {
            let body = slide(right_now, right_full, self.right_panel(right_full, window, cx));
            // the inner edge can be dragged to make the panel wider or narrower; the width is remembered
            let held = self.panel_drag.is_some();
            let edge = div().id("panel-edge").absolute().top_0().bottom_0().w(px(8.)).when(rtl, |d| d.right(px(-6.))).when(!rtl, |d| d.left(px(-6.))).cursor_col_resize()
                .child(div().absolute().top_0().bottom_0().left(px(3.)).w(px(2.)).when(held, |d| d.bg(p.accent.opacity(0.6))))
                .hover(|s| s.bg(p.accent.opacity(0.15)))
                .on_mouse_down(MouseButton::Left, cx.listener(move |a, ev: &MouseDownEvent, _, cx| {
                    a.panel_drag = Some((f32::from(ev.position.x), right_now));
                    cx.stop_propagation();
                    cx.notify();
                }));
            div().relative().h_full().flex().flex_none().child(body).child(edge).into_any_element()
        });
        let (right, over) = if overlay { (None, right) } else { (right, None) };
        let mut root = p.paint_background(div().size_full().relative().overflow_hidden().text_color(p.text));
        // the main font is the Latin one; the Arabic font is its explicit fallback for Arabic script
        // order: font chosen in Settings, then the theme's own font, then the default
        let theme_fonts = self.themes.themes[self.active_theme].fonts.clone();
        if let Some(l) = lh_gui::fonts::latin_font(&self.prefs.latin_font, &theme_fonts.latin, &self.fonts) {
            root = root.font_family(l);
        }
        let arabic = lh_gui::fonts::arabic_font(&self.prefs.arabic_font, &theme_fonts.arabic, &self.fonts);
        root.text_style().font_fallbacks = arabic.map(|f| FontFallbacks::from_fonts(vec![f]));
        let root = root
            .key_context(if asking { "LastHarness Asking" } else { "LastHarness" })
            .on_mouse_move(cx.listener(|a, ev: &MouseMoveEvent, _, cx| {
                if let Some((x0, start, _, ch)) = a.ctx_drag.take() {
                    let n = ch.len().max(2) as f32;
                    let x = (start + f32::from(ev.position.x) - x0).clamp(-CTX_GAP / 2., CTX_GAP * (n - 0.5));
                    a.ctx_drag = Some((x0, start, x, ch));
                    cx.notify();
                }
                if let Some((x0, start, _)) = a.mode_drag {
                    let x = (start + f32::from(ev.position.x) - x0).clamp(-44., 88. * 2.5);
                    a.mode_drag = Some((x0, start, x));
                    cx.notify();
                }
                if let Some((x0, start)) = a.panel_drag {
                    let moved = f32::from(ev.position.x) - x0;
                    let w = if a.rtl { start + moved } else { start - moved };
                    a.layout_mut().resize_right(w);
                    cx.notify();
                }
                if let Some((x0, start, _)) = a.slider_drag {
                    let end = lh_gui::slide::travel(SLIDER_GAP, a.stop_count());
                    let x = (start + f32::from(ev.position.x) - x0).clamp(-SLIDER_GAP / 2., end + SLIDER_GAP / 2.);
                    a.slider_drag = Some((x0, start, x));
                    cx.notify();
                }
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(|a, _: &MouseUpEvent, _, cx| {
                if a.panel_drag.take().is_some() {
                    a.persist();
                    cx.notify();
                }
                if let Some((_, _, x, ch)) = a.ctx_drag.take() {
                    let slot = lh_gui::slide::snap(x, CTX_GAP, ch.len());
                    if let Some(id) = ch.get(slot) {
                        a.choose_model(id, cx);
                        if a.pending.is_none() {
                            a.model_menu = true;
                        }
                    }
                    cx.notify();
                }
                if let Some((_, start, x)) = a.mode_drag.take() {
                    let slot = lh_gui::slide::snap(x, 88., 3);
                    let i = if a.rtl { 2 - slot } else { slot };
                    if (x - start).abs() > 2. {
                        a.set_mode([Mode::Chat, Mode::Assistant, Mode::Project][i], cx);
                    }
                    cx.notify();
                }
                if let Some((_, _, x)) = a.slider_drag.take() {
                    let n = a.stop_count();
                    let slot = lh_gui::slide::snap(x, SLIDER_GAP, n);
                    let i = if a.rtl { n - 1 - slot } else { slot };
                    a.choose_stop(i, cx);
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|a, _: &NewChat, w, cx| {
                a.new_chat(cx);
                a.composer.update(cx, |s, cx| s.focus(w, cx));
            }))
            .on_action(cx.listener(|a, _: &ToggleSidebar, _, cx| {
                a.layout_mut().toggle_left();
                a.persist();
                cx.notify();
            }))
            .on_action(cx.listener(|a, _: &ShowDepartments, _, cx| a.open_page(Page::Departments, cx)))
            .on_action(cx.listener(|a, _: &ShowWork, _, cx| a.open_panel(Panel::Work, cx)))
            .on_action(cx.listener(|a, _: &ShowMemory, _, cx| a.open_page(Page::Memory, cx)))
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
                if a.chat_menu.take().is_some() {
                    cx.notify();
                    return;
                }
                // Escape closes whatever is on top: the palette, Settings, a menu, then a question
                match topmost(a.palette.is_some(), a.settings_open, a.model_menu, a.pending.is_some(), a.confirm_delete.is_some()) {
                    Layer::Palette => a.close_palette(w, cx),
                    Layer::Settings => a.close_settings(w, cx),
                    Layer::ModelMenu => a.model_menu = false,
                    Layer::Nothing if a.slider_open => a.slider_open = false,
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
        let main = div().relative().flex().flex_col().flex_1().h_full().overflow_hidden().child(top).child(div().flex().flex_col().flex_1().overflow_hidden().child(middle));
        let settings = self.settings_view(window, cx);
        let palette = self.palette_view(cx);
        root.children(p.image_layer())
            .child(
                div().size_full().flex().when(rtl, |d| d.flex_row_reverse()).gap_2().p_2().overflow_hidden().child(left).child(main).children(right),
            )
            .children(over.map(|o| div().absolute().top_0().bottom_0().p_2().when(rtl, |d| d.left_0()).when(!rtl, |d| d.right_0()).flex()
                .child(div().h_full().flex().rounded(p.radius).shadow(vec![BoxShadow { color: hsla(0., 0., 0., if p.dark { 0.5 } else { 0.18 }), offset: point(px(0.), px(8.)), blur_radius: px(28.), spread_radius: px(0.), inset: false }]).child(o))))
            .children(settings)
            .children(palette)
    }
}
