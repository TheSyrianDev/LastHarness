//! The theme as the toolkit wants it: colours, radii, shadows, and the pieces every screen shares.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui::prelude::*;
use gpui::*;

use lh_gui::scene::{self, SceneSpec};
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
    Scene { image: Arc<RenderImage>, under: Hsla },
}

const SCENE_SIZE: (usize, usize) = (1600, 900);

/// The picture for a scene, made once and kept while the theme stays the same.
fn scene_image(spec: &SceneSpec) -> Arc<RenderImage> {
    static CACHE: Mutex<Vec<(SceneSpec, Arc<RenderImage>)>> = Mutex::new(Vec::new());
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((_, img)) = cache.iter().find(|(s, _)| s == spec) {
        return img.clone();
    }
    let (w, h) = SCENE_SIZE;
    let mut px = scene::render(spec, w, h);
    // the toolkit wants blue, green, red
    for p in px.as_chunks_mut::<4>().0 {
        p.swap(0, 2);
    }
    let buf = image::RgbaImage::from_raw(w as u32, h as u32, px).expect("size matches");
    let img = Arc::new(RenderImage::new(vec![image::Frame::new(buf)]));
    cache.retain(|_| false);
    cache.push((spec.clone(), img.clone()));
    img
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
            BackgroundKind::Scene => Background::Scene { image: scene_image(&t.background.scene), under: hsla(c.background) },
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
            glass: t.glass.as_ref().map(|g| GlassP { top: hsla(g.top), bottom: hsla(g.bottom), rim: hsla(g.rim), edge: hsla(g.edge), sheen: g.sheen }),
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
            Background::Image { under, .. } | Background::Scene { under, .. } => root.bg(*under),
        }
    }

    pub fn image_layer(&self) -> Option<AnyElement> {
        match &self.background {
            Background::Image { path, opacity, .. } => Some(img(path.clone()).absolute().top_0().left_0().size_full().object_fit(ObjectFit::Cover).opacity(*opacity).into_any_element()),
            Background::Scene { image, .. } => Some(img(image.clone()).absolute().top_0().left_0().size_full().object_fit(ObjectFit::Cover).into_any_element()),
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
            .child(div().absolute().top_0().left_0().w_full().h(relative(0.42)).max_h(px(150.)).rounded_tl(px(inner)).rounded_tr(px(inner)).bg(linear_gradient(180., linear_color_stop(white(g.sheen), 0.), linear_color_stop(white(0.), 1.))))
            // the light line just inside the edge
            .child(div().absolute().top_0().left_0().size_full().rounded(px(inner)).border_1().border_color(g.rim));
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
        let hover = Look { top: mix(top, white(1.), 0.3), bottom: mix(bottom, white(1.), 0.18), edge: self.edge, glow: 1., shown: 1. };
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
                BoxShadow { color: glow, offset: point(px(0.), px(0.)), blur_radius: px(12.), spread_radius: px(1.), inset: false },
                BoxShadow { color: black(0.18), offset: point(px(0.), px(1.)), blur_radius: px(2.), spread_radius: px(0.), inset: false },
            ])
            // the glossy top half, then the rim light just inside the edge
            .child(div().absolute().top_0().left_0().w_full().h(relative(0.5)).rounded_tl(px(inner)).rounded_tr(px(inner)).bg(linear_gradient(180., linear_color_stop(white(self.gloss), 0.), linear_color_stop(white(self.gloss * 0.35), 1.))))
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
    use super::{Background, Pal, SCENE_SIZE};
    use lh_gui::theme::Theme;
    use std::sync::Arc;

    fn aero() -> Theme {
        lh_gui::theme::builtin().into_iter().find(|t| t.label() == "Frutiger Aero light").unwrap()
    }

    #[test]
    fn bubbles_and_animation_stop_when_motion_is_reduced_or_the_theme_turns_it_off() {
        let t = aero();
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
    fn a_scene_becomes_a_picture_of_the_right_size_and_is_made_only_once() {
        let t = aero();
        let a = Pal::from(&t, None, false);
        let b = Pal::from(&t, None, false);
        match (&a.background, &b.background) {
            (Background::Scene { image: x, .. }, Background::Scene { image: y, .. }) => {
                assert!(Arc::ptr_eq(x, y), "the same scene is not painted twice");
                assert_eq!(x.size(0).width.0, SCENE_SIZE.0 as i32);
            }
            _ => panic!("the Frutiger Aero look paints a scene"),
        }
    }

    #[test]
    fn a_flat_theme_has_no_glass_gel_or_bubbles() {
        let calm = lh_gui::theme::builtin().remove(0);
        let p = Pal::from(&calm, None, false);
        assert!(p.glass.is_none() && p.gel.is_none() && p.focus.is_none() && p.motes.is_none());
    }
}
