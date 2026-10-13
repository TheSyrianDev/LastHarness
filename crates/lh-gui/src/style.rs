//! The theme as the toolkit wants it: colours, radii, shadows, and the pieces every screen shares.

use std::time::Duration;

use gpui::prelude::*;
use gpui::*;

use lh_gui::sheen;
use lh_gui::theme::{BackgroundKind, Color, Mode, Theme};

pub fn hsla(c: Color) -> Hsla {
    Rgba { r: c.r as f32 / 255., g: c.g as f32 / 255., b: c.b as f32 / 255., a: c.a as f32 / 255. }.into()
}

/// A theme turned into values ready to paint with.
#[derive(Clone)]
pub struct Pal {
    pub bg: Hsla,
    pub surface: Hsla,
    pub sunken: Hsla,
    pub text: Hsla,
    pub muted: Hsla,
    pub accent: Hsla,
    pub accent_text: Hsla,
    pub button: Hsla,
    pub hover: Hsla,
    pub border: Hsla,
    pub danger: Hsla,
    pub radius: Pixels,
    pub inner: Pixels,
    pub shadow: Vec<BoxShadow>,
    pub anim: Duration,
    pub dark: bool,
    pub background: Background,
    pub glass: Option<GlassP>,
    pub gel: Option<GelP>,
    pub focus: Option<FocusP>,
    pub motes: Option<(u32, Hsla)>,
}

#[derive(Clone)]
pub struct GlassP {
    pub top: Hsla,
    pub bottom: Hsla,
    pub rim: Hsla,
    pub edge: Hsla,
    pub sheen: f32,
    pub lines: f32,
    pub lines_gap: f32,
    pub reflection: f32,
}

#[derive(Clone)]
pub struct GelP {
    pub top: Hsla,
    pub bottom: Hsla,
    pub primary_top: Hsla,
    pub primary_bottom: Hsla,
    pub gloss: f32,
    pub rim: Hsla,
    pub edge: Hsla,
    pub glow: Hsla,
    pub press: f32,
}

#[derive(Clone)]
pub struct FocusP {
    pub from: Hsla,
    pub to: Hsla,
    pub width: f32,
}

#[derive(Clone)]
pub enum Background {
    Solid(Hsla),
    Gradient { angle: f32, from: Hsla, to: Hsla },
    Image { path: std::path::PathBuf, opacity: f32, under: Hsla },
}

fn white(a: f32) -> Hsla {
    gpui::hsla(0., 0., 1., a)
}

fn black(a: f32) -> Hsla {
    gpui::hsla(0., 0., 0., a)
}

impl Pal {
    /// `reduce_motion`: the desktop asked for less movement, so nothing animates.
    pub fn from(t: &Theme, window_radius: Option<f32>, reduce_motion: bool) -> Pal {
        let c = &t.colors;
        let outer = t.panel_radius(window_radius);
        let shadow = if t.shadow_blur > 0. && t.shadow_color.a > 0 {
            vec![BoxShadow { color: hsla(t.shadow_color), offset: point(px(0.), px(t.shadow_y)), blur_radius: px(t.shadow_blur), spread_radius: px(0.), inset: false }]
        } else {
            vec![]
        };
        let background = match t.background.kind {
            BackgroundKind::Solid => Background::Solid(hsla(c.background)),
            BackgroundKind::Gradient => Background::Gradient { angle: t.background.angle, from: hsla(t.background.from), to: hsla(t.background.to) },
            BackgroundKind::Image => match &t.background.image {
                Some(p) => Background::Image { path: p.clone(), opacity: t.background.image_opacity, under: hsla(c.background) },
                None => Background::Solid(hsla(c.background)),
            },
        };
        Pal {
            bg: hsla(c.background),
            surface: hsla(c.surface),
            sunken: hsla(c.sunken),
            text: hsla(c.text),
            muted: hsla(c.muted),
            accent: hsla(c.accent),
            accent_text: hsla(c.accent_text),
            button: hsla(c.button),
            hover: hsla(c.button_hover),
            border: hsla(c.border),
            danger: hsla(c.danger),
            radius: px(outer),
            inner: px(Theme::inner_radius(outer, 8.)),
            shadow,
            anim: if reduce_motion { Duration::ZERO } else { Duration::from_millis(t.animation_ms) },
            dark: t.mode == Mode::Dark,
            background,
            glass: t.glass.as_ref().map(|g| GlassP { top: hsla(g.top), bottom: hsla(g.bottom), rim: hsla(g.rim), edge: hsla(g.edge), sheen: g.sheen, lines: g.lines, lines_gap: g.lines_gap, reflection: g.reflection }),
            gel: t.gel.as_ref().map(|g| GelP { top: hsla(g.top), bottom: hsla(g.bottom), primary_top: hsla(g.primary_top), primary_bottom: hsla(g.primary_bottom), gloss: g.gloss, rim: hsla(g.rim), edge: hsla(g.edge), glow: hsla(g.glow), press: g.press }),
            focus: t.focus.as_ref().map(|f| FocusP { from: hsla(f.from), to: hsla(f.to), width: f.width }),
            motes: t.motes.as_ref().filter(|m| m.count > 0 && !reduce_motion && t.animation_ms > 0).map(|m| (m.count, hsla(m.color))),
        }
    }

    /// Paint the window background under everything.
    pub fn paint_background(&self, root: Div) -> Div {
        match &self.background {
            Background::Solid(c) => root.bg(*c),
            Background::Gradient { angle, from, to } => root.bg(linear_gradient(*angle, linear_color_stop(*from, 0.), linear_color_stop(*to, 1.))),
            Background::Image { under, .. } => root.bg(*under),
        }
    }

    pub fn image_layer(&self) -> Option<AnyElement> {
        match &self.background {
            Background::Image { path, opacity, .. } => Some(img(path.clone()).absolute().top_0().left_0().size_full().object_fit(ObjectFit::Cover).opacity(*opacity).into_any_element()),
            _ => None,
        }
    }

    /// Soft bubbles that drift up behind the panels, or nothing when the look has none or motion is off.
    pub fn motes_layer(&self, clock: f32) -> Option<AnyElement> {
        let (count, color) = self.motes?;
        let bubbles: Vec<AnyElement> = (0..count)
            .map(|i| {
                // each bubble has its own size, lane and speed, so they never move together
                let f = |k: u32| ((i.wrapping_mul(2654435761).wrapping_add(k.wrapping_mul(40503)) >> 8) & 0xffff) as f32 / 65535.;
                let size = 18. + 54. * f(1) * f(2);
                let x = 0.04 + 0.92 * f(3);
                let secs = 38. + 40. * f(4);
                let t = (clock / secs + f(5)).fract();
                let sway = 0.012 + 0.02 * f(6);
                div()
                    .absolute()
                    .w(px(size))
                    .h(px(size))
                    .rounded_full()
                    .border_1()
                    .border_color(color.opacity(0.5))
                    .bg(linear_gradient(145., linear_color_stop(color.opacity(0.32), 0.), linear_color_stop(color.opacity(0.04), 1.)))
                    .child(div().absolute().top(px(size * 0.14)).left(px(size * 0.2)).w(px(size * 0.3)).h(px(size * 0.16)).rounded_full().bg(white(0.55)))
                    .left(relative(x + sway * (t * std::f32::consts::TAU).sin()))
                    .top(relative(1.08 - 1.2 * t))
                    .into_any_element()
            })
            .collect();
        Some(div().absolute().top_0().left_0().size_full().overflow_hidden().children(bubbles).into_any_element())
    }
}

/// A panel's own look: flat, or glass with a rim, a sheen and, for the panel in use, a gradient border.
pub trait PanelExt: Sized {
    fn panel(self, p: &Pal, focused: bool) -> Self;
}

macro_rules! impl_panel {
    ($t:ty) => {
impl PanelExt for $t {
    fn panel(self, p: &Pal, focused: bool) -> $t {
        let d = self.rounded(p.radius).shadow(p.shadow.clone());
        let Some(g) = &p.glass else { return d.bg(p.surface) };
        let inner = (f32::from(p.radius) - 1.).max(0.);
        let mut d = d
            .bg(linear_gradient(180., linear_color_stop(g.top, 0.), linear_color_stop(g.bottom, 1.)))
            .border_1()
            .border_color(g.edge)
            // the shine over the top of the glass
            .child(div().absolute().top_0().left_0().w_full().h(relative(0.42)).max_h(px(150.)).rounded_tl(px(inner)).rounded_tr(px(inner)).bg(linear_gradient(180., linear_color_stop(white(g.sheen), 0.), linear_color_stop(white(0.), 1.))));
        // the diagonal reflection and the fine diagonal hairlines, for the looks that ask for them
        if g.lines > 0. || g.reflection > 0. {
            d = d.child(glass_marks(g.clone(), inner));
        }
        // the light line just inside the edge
        d = d.child(div().absolute().top_0().left_0().size_full().rounded(px(inner)).border_1().border_color(g.rim));
        if let (true, Some(f)) = (focused, &p.focus) {
            d = d.child(focus_ring(f.clone(), px(inner)));
        }
        d
    }
}
    };
}

impl_panel!(Div);
impl_panel!(Stateful<Div>);

/// The marks of Windows 7 style glass, drawn over a pane: the reflection is twelve nested triangles, each a flat
/// sliver of white, so it fades out in steps too small to see (no hard edge, no stripe), and the hairlines are one stroked path of straight 45 degree lines
/// (at most 220, see `sheen`), a pixel wide and a few percent white. Where they go is worked out in `sheen.rs`.
fn glass_marks(g: GlassP, radius: f32) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |b, _, window, _| {
            let (w, h): (f32, f32) = (b.size.width.into(), b.size.height.into());
            let (ox, oy): (f32, f32) = (b.left().into(), b.top().into());
            let at = |x: f32, y: f32| point(px(ox + x), px(oy + y));
            if g.reflection > 0. {
                // nested triangles, each a flat share of the opacity: the glow is brightest in the corner and
                // fades out along the diagonal in small steps, with no hard edge
                for (poly, share) in sheen::reflection(w, h, radius) {
                    let mut pb = PathBuilder::fill();
                    pb.move_to(at(poly[0].0, poly[0].1));
                    for q in &poly[1..] {
                        pb.line_to(at(q.0, q.1));
                    }
                    pb.close();
                    if let Ok(path) = pb.build() {
                        window.paint_path(path, white(g.reflection * share));
                    }
                }
            }
            if g.lines > 0. {
                let (ax, ay, aw, ah) = sheen::hairline_area(w, h, radius);
                let segs = sheen::hairlines(aw, ah, g.lines_gap);
                if !segs.is_empty() {
                    let mut pb = PathBuilder::stroke(px(1.));
                    for (a, c) in segs {
                        pb.move_to(at(ax + a.0, ay + a.1));
                        pb.line_to(at(ax + c.0, ay + c.1));
                    }
                    if let Ok(path) = pb.build() {
                        window.paint_path(path, white(g.lines));
                    }
                }
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// A gradient border drawn on top of a panel, corner to corner.
fn focus_ring(f: FocusP, radius: Pixels) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |b, _, window, _| {
            let w = px(f.width);
            let half = w / 2.;
            let r = (radius - half).max(px(0.));
            let (l, t, rt, bt) = (b.left() + half, b.top() + half, b.right() - half, b.bottom() - half);
            let mut pb = PathBuilder::stroke(w);
            pb.move_to(point(l + r, t));
            pb.line_to(point(rt - r, t));
            pb.arc_to(point(r, r), px(0.), false, true, point(rt, t + r));
            pb.line_to(point(rt, bt - r));
            pb.arc_to(point(r, r), px(0.), false, true, point(rt - r, bt));
            pb.line_to(point(l + r, bt));
            pb.arc_to(point(r, r), px(0.), false, true, point(l, bt - r));
            pb.line_to(point(l, t + r));
            pb.arc_to(point(r, r), px(0.), false, true, point(l + r, t));
            pb.close();
            if let Ok(path) = pb.build() {
                window.paint_path(path, linear_gradient(135., linear_color_stop(f.from, 0.), linear_color_stop(f.to, 1.)));
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// The pieces of a gel button's surface.
#[derive(Clone, Copy)]
pub struct Look {
    pub top: Hsla,
    pub bottom: Hsla,
    pub edge: Hsla,
    pub glow: f32,
    pub shown: f32,
}

impl Look {
    pub fn mix(a: Look, b: Look, t: f32) -> Look {
        Look { top: mix(a.top, b.top, t), bottom: mix(a.bottom, b.bottom, t), edge: mix(a.edge, b.edge, t), glow: a.glow + (b.glow - a.glow) * t, shown: a.shown + (b.shown - a.shown) * t }
    }
}

impl GelP {
    /// What a button of this kind looks like at rest and under the pointer.
    pub fn looks(&self, danger: Hsla, ghost: bool, primary: bool, is_danger: bool) -> (Look, Look) {
        let (top, bottom) = if is_danger { (mix(danger, white(1.), 0.3), mix(danger, black(1.), 0.2)) } else if primary { (self.primary_top, self.primary_bottom) } else { (self.top, self.bottom) };
        let rest = Look { top, bottom, edge: self.edge, glow: 0., shown: if ghost { 0. } else { 1. } };
        // under the pointer a button takes on the blue of its glow, as Windows 7 buttons did, not a plain brightening
        let blue = gpui::hsla(self.glow.h, self.glow.s, self.glow.l, 1.);
        let hover = Look { top: mix(mix(top, white(1.), 0.1), blue, 0.16), bottom: mix(mix(bottom, white(1.), 0.06), blue, 0.30), edge: mix(self.edge, blue, 0.5), glow: 1., shown: 1. };
        (rest, hover)
    }

    pub fn skin(&self, radius: Pixels, look: Look) -> Div {
        let glow = gpui::hsla(self.glow.h, self.glow.s, self.glow.l, self.glow.a * look.glow);
        let inner = (f32::from(radius) - 1.).max(0.);
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .rounded(radius)
            .opacity(look.shown)
            .bg(linear_gradient(180., linear_color_stop(look.top, 0.), linear_color_stop(look.bottom, 1.)))
            .border_1()
            .border_color(look.edge)
            .shadow(vec![
                BoxShadow { color: glow, offset: point(px(0.), px(0.)), blur_radius: px(16.), spread_radius: px(0.), inset: false },
                BoxShadow { color: black(0.12), offset: point(px(0.), px(1.)), blur_radius: px(3.), spread_radius: px(0.), inset: false },
            ])
            // the glossy top half, then the rim light just inside the edge
            .child(div().absolute().top_0().left_0().w_full().h(relative(0.62)).rounded_tl(px(inner)).rounded_tr(px(inner)).bg(linear_gradient(180., linear_color_stop(white(self.gloss), 0.), linear_color_stop(white(0.), 1.))))
            .child(div().absolute().top_0().left_0().size_full().rounded(px(inner)).border_1().border_color(self.rim))
    }
}

/// Keep the toolkit's own components (text input, markdown view) in the same colours.
pub fn apply_component_theme(p: &Pal, cx: &mut App) {
    use gpui_component::theme::{Theme as CT, ThemeMode};
    let mode = if p.dark { ThemeMode::Dark } else { ThemeMode::Light };
    CT::change(mode, None, cx);
    let t = CT::global_mut(cx);
    // the root surface of the toolkit paints this behind everything; ours is painted by the app itself, and a
    // translucent look must be able to show the desktop through
    t.tokens.background = gpui::hsla(0., 0., 0., 0.).into();
    let c = &mut t.colors;
    c.foreground = p.text;
    // text fields take the background colour; on glass that must be the field colour, not the sky
    c.background = if p.glass.is_some() { p.sunken } else { p.bg };
    c.border = p.border;
    c.input = p.border;
    c.muted = p.sunken;
    c.muted_foreground = p.muted;
    c.accent = p.accent;
    c.accent_foreground = p.accent_text;
    c.primary = p.accent;
    c.primary_foreground = p.accent_text;
    c.caret = p.text;
    c.link = p.accent;
    c.popover = p.surface;
    c.popover_foreground = p.text;
    c.list = p.surface;
    c.scrollbar_thumb = p.muted;
}

/// Linear mix of two colours, for fading a button between its resting and hovered look.
pub fn mix(a: Hsla, b: Hsla, t: f32) -> Hsla {
    let (a, b): (Rgba, Rgba) = (a.into(), b.into());
    let t = t.clamp(0., 1.);
    Rgba { r: a.r + (b.r - a.r) * t, g: a.g + (b.g - a.g) * t, b: a.b + (b.b - a.b) * t, a: a.a + (b.a - a.a) * t }.into()
}

#[cfg(test)]
mod tests {
    use super::{Pal, PanelExt};
    use lh_gui::theme::Theme;

    fn aero() -> Theme {
        lh_gui::theme::builtin().into_iter().find(|t| t.label() == "Frutiger Aero light").unwrap()
    }

    /// A look with bubbles: the built-in looks have none now, but the feature is still there until the user decides.
    fn bubbly() -> Theme {
        let mut t = aero();
        t.motes = Some(lh_gui::theme::Motes { count: 5, color: lh_gui::theme::Color { r: 255, g: 255, b: 255, a: 255 } });
        t
    }

    #[test]
    fn the_aero_glass_carries_its_reflection_and_hairlines_into_the_paint_values() {
        for label in ["Frutiger Aero light", "Frutiger Aero dark"] {
            let t = lh_gui::theme::builtin().into_iter().find(|t| t.label() == label).unwrap();
            let p = Pal::from(&t, None, false);
            let g = p.glass.expect("glass");
            assert!(g.reflection > 0. && g.lines <= lh_gui::theme::HAIRLINE_MAX_OPACITY && g.lines_gap >= lh_gui::theme::HAIRLINE_MIN_GAP, "{label}");
            assert!(p.motes.is_none(), "{label} has no bubbles");
            assert!(matches!(p.background, super::Background::Gradient { .. }), "{label} is a tint, not a picture");
        }
        let flat = Pal::from(&lh_gui::theme::builtin().remove(0), None, false);
        assert!(flat.glass.is_none());
    }

    #[test]
    fn bubbles_and_animation_stop_when_motion_is_reduced_or_the_theme_turns_it_off() {
        let t = bubbly();
        let normal = Pal::from(&t, None, false);
        assert!(normal.motes.is_some() && !normal.anim.is_zero());
        let reduced = Pal::from(&t, None, true);
        assert!(reduced.motes.is_none() && reduced.anim.is_zero(), "reduced motion switches both off");
        let mut off = t.clone();
        off.animation_ms = 0;
        assert!(Pal::from(&off, None, false).motes.is_none(), "animation 0 means no bubbles");
        let mut none = t;
        none.motes.as_mut().unwrap().count = 0;
        assert!(Pal::from(&none, None, false).motes.is_none());
    }

    #[test]
    fn a_panel_keeps_the_position_it_was_given() {
        // a pop-up menu is absolute; making it a glass panel must not pull it back into the layout (it did, and the
        // theme menu disappeared under the Frutiger Aero look)
        use gpui::{div, Position, Styled};
        let p = Pal::from(&aero(), None, false);
        let mut menu = div().absolute().panel(&p, false);
        assert_eq!(menu.style().position, Some(Position::Absolute));
        let flat = Pal::from(&lh_gui::theme::builtin().remove(0), None, false);
        let mut menu = div().absolute().panel(&flat, false);
        assert_eq!(menu.style().position, Some(Position::Absolute));
    }

    #[test]
    fn a_flat_theme_has_no_glass_gel_or_bubbles() {
        let calm = lh_gui::theme::builtin().remove(0);
        let p = Pal::from(&calm, None, false);
        assert!(p.glass.is_none() && p.gel.is_none() && p.focus.is_none() && p.motes.is_none());
    }
}
