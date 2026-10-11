//! The theme as the toolkit wants it: colours, radii, shadows, and the pieces every screen shares.

use std::time::Duration;

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
    pub radius: Pixels,
    pub inner: Pixels,
    pub shadow: Vec<BoxShadow>,
    pub anim: Duration,
    pub dark: bool,
    pub background: Background,
}

#[derive(Clone)]
pub enum Background {
    Solid(Hsla),
    Gradient { angle: f32, from: Hsla, to: Hsla },
    Image { path: std::path::PathBuf, opacity: f32, under: Hsla },
}

impl Pal {
    pub fn from(t: &Theme, window_radius: Option<f32>) -> Pal {
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
            anim: Duration::from_millis(t.animation_ms),
            dark: t.mode == Mode::Dark,
            background,
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

    pub fn image_layer(&self) -> Option<impl IntoElement> {
        match &self.background {
            Background::Image { path, opacity, .. } => Some(img(path.clone()).absolute().top_0().left_0().size_full().object_fit(ObjectFit::Cover).opacity(*opacity)),
            _ => None,
        }
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
}

/// Linear mix of two colours, for fading a button between its resting and hovered look.
pub fn mix(a: Hsla, b: Hsla, t: f32) -> Hsla {
    let (a, b): (Rgba, Rgba) = (a.into(), b.into());
    let t = t.clamp(0., 1.);
    Rgba { r: a.r + (b.r - a.r) * t, g: a.g + (b.g - a.g) * t, b: a.b + (b.b - a.b) * t, a: a.a + (b.a - a.a) * t }.into()
}
