//! The theme as the toolkit wants it: colours, radii, shadows, and the pieces every screen shares.

use std::time::Duration;

use gpui::prelude::*;
use gpui::*;

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
    pub warn: Hsla,
    pub good: Hsla,
    pub focus_ring: Hsla,
    /// The person's own message bubble.
    pub bubble: Hsla,
    pub radius: Pixels,
    pub inner: Pixels,
    pub shadow: Vec<BoxShadow>,
    pub anim: Duration,
    pub dark: bool,
    pub background: Background,
    /// Font for code blocks and step lists: the theme's first installed `mono` font, else DejaVu Sans Mono.
    pub mono: SharedString,
    /// The theme's own icons by name (most looks bring none).
    pub icons: std::sync::Arc<lh_gui::icons::IconSet>,
    /// Which surfaces are liquid glass.
    pub material: lh_gui::theme::Material,
    /// The brand mark's colours: the look's own (Omarchy) or the brand kit's for the mode.
    pub mark: lh_gui::brand::Colours,
    /// Chart series colours the look brings; empty uses the app's own.
    pub series: Vec<Hsla>,
}

#[derive(Clone)]
pub enum Background {
    Solid(Hsla),
    Gradient { angle: f32, from: Hsla, to: Hsla },
    Image { path: std::path::PathBuf, opacity: f32, under: Hsla },
}

impl Pal {
    /// `reduce_motion`: the desktop asked for less movement, so nothing animates.
    pub fn from(t: &Theme, window_radius: Option<f32>, reduce_motion: bool, installed_fonts: &[String]) -> Pal {
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
            warn: hsla(c.warn),
            good: hsla(c.good),
            focus_ring: hsla(c.focus_ring),
            bubble: hsla(c.bubble),
            radius: px(outer),
            inner: px(Theme::inner_radius(outer, 8.)),
            shadow,
            anim: if reduce_motion { Duration::ZERO } else { Duration::from_millis(t.animation_ms) },
            dark: t.mode == Mode::Dark,
            background,
            mono: lh_gui::fonts::mono_font(&t.fonts.mono, installed_fonts).into(),
            icons: std::sync::Arc::new(t.icons.clone()),
            material: t.material,
            mark: t.mark.unwrap_or_else(|| lh_gui::brand::Colours::for_mode(t.mode == Mode::Dark)),
            series: t.series.iter().map(|c| hsla(*c)).collect(),
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
}

/// A panel's own look: rounded, with the theme's shadow and its surface colour. The panel in use is not marked.
pub trait PanelExt: Sized {
    fn panel(self, p: &Pal, focused: bool) -> Self;
    /// A surface that floats: 2 for menus and pop-ups, 3 for dialogs. Solid fill, a hairline border, and a shadow
    /// that grows with the level, so what is on top always reads as on top, in every look.
    fn elevated(self, p: &Pal, level: u8) -> Self;
    /// The panel on the right: a panel, in the theme's side-panel material.
    fn side_panel(self, p: &Pal) -> Self;
}

/// The shadows of an elevation level: a tight one for the edge and a wide soft one for the lift.
pub fn elevation_shadow(p: &Pal, level: u8) -> Vec<BoxShadow> {
    let k = level.clamp(1, 3) as f32;
    let a = if p.dark { 0.5 } else { 0.1 };
    vec![
        BoxShadow { color: gpui::hsla(0., 0., 0., a * 0.8), offset: point(px(0.), px(1.)), blur_radius: px(2.), spread_radius: px(0.), inset: false },
        BoxShadow { color: gpui::hsla(0., 0., 0., a * (0.6 + 0.2 * k)), offset: point(px(0.), px(4. * k)), blur_radius: px(12. * k), spread_radius: px(-2.), inset: false },
    ]
}

/// The fill of an elevation level: the background made solid, then lifted one small step per level (toward the
/// text colour in a dark look, toward white in a light one), so stacked surfaces read apart without loud shadows.
pub fn elevation_fill(p: &Pal, level: u8) -> Hsla {
    let base = Hsla { a: 1.0, ..p.bg };
    let k = level.clamp(1, 3) as f32;
    if p.dark { mix(base, Hsla { a: 1.0, ..p.text }, 0.035 * k) } else { mix(base, gpui::hsla(0., 0., 1., 1.), 0.3 * k) }
}

/// Which surface a glass layer is for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Surface {
    Panel,
    Popup,
    Side,
}

/// The shader's material for a theme's glass kind; `None` for a flat surface.
pub fn glass_variant(k: lh_gui::theme::GlassKind) -> Option<GlassVariant> {
    use lh_gui::theme::GlassKind as K;
    match k {
        K::None => None,
        K::Regular => Some(GlassVariant::Regular),
        K::Clear => Some(GlassVariant::Clear),
        K::RegularTinted => Some(GlassVariant::RegularTinted),
        K::ClearTinted => Some(GlassVariant::ClearTinted),
    }
}

/// How a glass panel or pop-up is painted, or `None` when the theme keeps that surface flat. The tint is the
/// surface's own colour at its own opacity, so the glass reads like the colour the contrast test measured; a pop-up
/// is never less than nine tenths covered, so the words in a menu stay clear over busy content.
pub fn surface_glass(p: &Pal, s: Surface) -> Option<GlassParams> {
    let m = &p.material;
    let kind = match s {
        Surface::Panel => m.panels,
        Surface::Popup => m.popups,
        Surface::Side => m.side_panel,
    };
    let variant = glass_variant(kind)?;
    let tint = match s {
        Surface::Popup => Hsla { a: p.surface.a.max(0.9), ..elevation_fill(p, 2) },
        _ => p.surface,
    };
    Some(GlassParams { blur: px(m.blur), refraction: px(m.refraction), tint, rim: m.rim, variant, energy: glass_energy::REST })
}

/// A glass layer filling its parent, painted under the parent's children.
pub fn glass_layer(params: GlassParams, radius: Pixels) -> AnyElement {
    canvas(|_, _, _| (), move |b, _, window, _| window.paint_glass(b, radius, params)).absolute().top_0().left_0().size_full().into_any_element()
}

macro_rules! impl_panel {
    ($t:ty) => {
impl PanelExt for $t {
    fn panel(self, p: &Pal, _focused: bool) -> $t {
        match surface_glass(p, Surface::Panel) {
            Some(g) => self.rounded(p.radius).shadow(p.shadow.clone()).border_1().border_color(p.border).child_under(glass_layer(g, p.radius)),
            None => self.rounded(p.radius).shadow(p.shadow.clone()).bg(p.surface),
        }
    }
    fn elevated(self, p: &Pal, level: u8) -> $t {
        let d = self.rounded(p.radius).border_1().border_color(p.border).shadow(elevation_shadow(p, level));
        match surface_glass(p, Surface::Popup) {
            Some(g) => d.child_under(glass_layer(g, p.radius)),
            None => d.bg(elevation_fill(p, level)),
        }
    }
    fn side_panel(self, p: &Pal) -> $t {
        match surface_glass(p, Surface::Side) {
            Some(g) => self.rounded(p.radius).shadow(p.shadow.clone()).border_1().border_color(p.border).child_under(glass_layer(g, p.radius)),
            None => self.panel(p, false),
        }
    }
}
    };
}

impl_panel!(Div);
impl_panel!(Stateful<Div>);

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
    // text fields take the background colour
    c.background = p.bg;
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
    c.ring = p.focus_ring;
    c.danger = p.danger;
    c.warning = p.warn;
    c.success = p.good;
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

    fn calm() -> lh_gui::theme::Theme {
        lh_gui::theme::builtin().remove(0)
    }

    #[test]
    fn a_panel_keeps_the_position_it_was_given() {
        // a pop-up menu is absolute; making it a panel must not pull it back into the layout (it did once, and the
        // theme menu disappeared)
        use gpui::{div, Position, Styled};
        for t in lh_gui::theme::builtin() {
            let p = Pal::from(&t, None, false, &[]);
            let mut menu = div().absolute().panel(&p, false);
            assert_eq!(menu.style().position, Some(Position::Absolute), "{}", t.label());
        }
    }

    #[test]
    fn animation_stops_when_motion_is_reduced_or_the_theme_turns_it_off() {
        let t = calm();
        assert!(!Pal::from(&t, None, false, &[]).anim.is_zero());
        assert!(Pal::from(&t, None, true, &[]).anim.is_zero(), "reduced motion switches hover fades off");
        let mut off = t;
        off.animation_ms = 0;
        assert!(Pal::from(&off, None, false, &[]).anim.is_zero());
    }

    #[test]
    fn every_built_in_look_turns_into_paint_values_without_a_picture() {
        for t in lh_gui::theme::builtin() {
            let p = Pal::from(&t, None, false, &[]);
            assert!(!matches!(p.background, super::Background::Image { .. }), "{}", t.label());
        }
    }

    #[test]
    fn chart_series_stand_off_the_surface_and_from_each_other_in_every_look() {
        let lum = |c: gpui::Hsla| {
            let r: gpui::Rgba = c.into();
            let f = |x: f32| if x <= 0.04045 { x / 12.92 } else { ((x + 0.055) / 1.055).powf(2.4) };
            0.2126 * f(r.r) + 0.7152 * f(r.g) + 0.0722 * f(r.b)
        };
        for t in lh_gui::theme::builtin() {
            let p = Pal::from(&t, None, false, &[]);
            let s = super::mix(gpui::Hsla { a: 1., ..p.bg }, gpui::Hsla { a: 1., ..p.surface }, p.surface.a);
            let cols: Vec<gpui::Hsla> = (0..6).map(|i| crate::panels::series_color(&p, i)).collect();
            for (i, c) in cols.iter().enumerate() {
                let (a, b) = (lum(*c), lum(s));
                let r = (a.max(b) + 0.05) / (a.min(b) + 0.05);
                assert!(r >= 3.0, "{} series {i}: {r:.2}", t.label());
            }
            for i in 0..cols.len() {
                for j in i + 1..cols.len() {
                    let d = (cols[i].h - cols[j].h).abs();
                    assert!(d.min(1. - d) > 0.03, "{} series {i} and {j} share a hue", t.label());
                }
            }
        }
    }

}
