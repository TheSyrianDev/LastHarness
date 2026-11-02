//! Offscreen picture of each Glass look as its `[material]` table makes it: a busy backdrop, a frosted sidebar with a
//! resting button and an input, a clear tinted side panel, a frosted pop-up and a clear switch knob. No text, no
//! window, the mouse is not touched. Set `LH_GLASS_THEME_SHOTS=<dir>` to save one PNG per look.

#![cfg(target_os = "linux")]

use gpui::{Bounds, ContentMask, Corners, DevicePixels, Hsla, PlatformHeadlessRenderer, Point, Quad, Scene, Size};
use gpui_wgpu::WgpuHeadlessRenderer;
use lh_gui::theme::{builtin, Color, GlassKind, Mode};

fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds<gpui::ScaledPixels> {
    Bounds { origin: Point { x: x.into(), y: y.into() }, size: Size { width: w.into(), height: h.into() } }
}

fn hsla(c: Color) -> Hsla {
    gpui::Rgba { r: c.r as f32 / 255., g: c.g as f32 / 255., b: c.b as f32 / 255., a: c.a as f32 / 255. }.into()
}

fn quad(s: &mut Scene, b: Bounds<gpui::ScaledPixels>, c: Hsla, r: f32, border: Option<Hsla>) {
    s.insert_primitive(Quad {
        order: 0, border_style: Default::default(), bounds: b, content_mask: ContentMask { bounds: b }, background: c.into(),
        border_color: border.unwrap_or(c), corner_radii: Corners::all(r.into()), border_widths: gpui::Edges::all((if border.is_some() { 1. } else { 0. }).into()),
    });
}

fn variant(k: GlassKind) -> u32 {
    match k {
        GlassKind::None | GlassKind::Regular => 0,
        GlassKind::Clear => 1,
        GlassKind::RegularTinted => 2,
        GlassKind::ClearTinted => 3,
    }
}

#[allow(clippy::too_many_arguments)]
fn glass(s: &mut Scene, b: Bounds<gpui::ScaledPixels>, r: f32, blur: f32, refraction: f32, rim: f32, tint: Hsla, k: GlassKind) {
    glass_e(s, b, r, blur, refraction, rim, tint, k, 0.);
}

#[allow(clippy::too_many_arguments)]
fn glass_e(s: &mut Scene, b: Bounds<gpui::ScaledPixels>, r: f32, blur: f32, refraction: f32, rim: f32, tint: Hsla, k: GlassKind, energy: f32) {
    s.insert_primitive(gpui::Glass { order: 0, pad: 0, bounds: b, content_mask: ContentMask { bounds: b }, corner_radius: r.into(), blur: blur.into(), refraction: refraction.into(), rim, tint, variant: variant(k), energy });
}

/// The Glass looks, and the Omarchy look twice: from the dark fixture with sample Hyprland values (rounding 24,
/// opacity 0.9, blur 8 x 3), and from this desktop as it is now when Omarchy and Hyprland are there.
fn looks() -> Vec<lh_gui::theme::Theme> {
    let mut v: Vec<_> = builtin().into_iter().filter(|t| t.name == "Glass").collect();
    let h = lh_gui::omarchy::Hypr { rounding: 24., active_opacity: 0.9, inactive_opacity: 0.8, blur_size: 8., blur_passes: 3. };
    let mut fixture = lh_gui::omarchy::from_colors_with(include_str!("fixtures/omarchy-dark.toml"), Some(h)).unwrap();
    fixture.name = "Omarchy sample".into();
    v.push(fixture);
    if let Some(t) = lh_gui::omarchy::current_dir().and_then(|d| lh_gui::omarchy::load(&d, lh_gui::omarchy::Hypr::read())) {
        let mut t = t;
        t.name = "Omarchy live".into();
        v.push(t);
    }
    v
}

#[test]
fn each_glass_look_renders_offscreen() {
    let Ok(mut r) = WgpuHeadlessRenderer::new() else {
        eprintln!("no GPU adapter for offscreen rendering; skipped");
        return;
    };
    let shots = std::env::var_os("LH_GLASS_THEME_SHOTS").map(std::path::PathBuf::from);
    for t in looks() {
        let c = &t.colors;
        let m = t.material;
        let dark = t.mode == Mode::Dark;
        let rad = t.roundness;
        let inner = (rad - 8.).max(3.);
        let (w, h) = (640., 400.);
        let mut s = Scene::default();
        // the desktop behind the window: colourful stripes, so blur and bend show
        quad(&mut s, rect(0., 0., w, h), hsla(if dark { Color { r: 0, g: 0, b: 0, a: 255 } } else { Color { r: 255, g: 255, b: 255, a: 255 } }), 0., None);
        for i in 0..16 {
            let hue = i as f32 / 16.;
            quad(&mut s, rect(i as f32 * 40., 0., 18., h), gpui::hsla(hue, 0.7, if dark { 0.45 } else { 0.6 }, 1.), 0., None);
        }
        quad(&mut s, rect(0., 0., w, h), hsla(c.background), 0., None);
        // the sidebar: glass, then its content
        let side = rect(10., 10., 190., 380.);
        glass(&mut s, side, rad, m.blur, m.refraction, m.rim, hsla(c.surface), m.panels);
        quad(&mut s, side, gpui::hsla(0., 0., 0., 0.), rad, Some(hsla(c.border)));
        quad(&mut s, rect(24., 24., 22., 22.), hsla(c.accent), 6., None); // the mark's place
        quad(&mut s, rect(24., 60., 160., 34.), hsla(c.button), inner, None); // a resting button
        quad(&mut s, rect(40., 74., 90., 6.), hsla(c.text), 3., None);
        for i in 0..5 {
            quad(&mut s, rect(30., 112. + i as f32 * 26., 120. - i as f32 * 10., 6.), hsla(if i == 0 { c.text } else { c.muted }), 3., None);
        }
        // the chat area: an input on the background
        quad(&mut s, rect(214., 330., 240., 52.), hsla(c.sunken), inner, Some(hsla(c.border)));
        quad(&mut s, rect(228., 352., 120., 6.), hsla(c.muted), 3., None);
        // the side panel on the right
        let right = rect(466., 10., 164., 380.);
        glass(&mut s, right, rad, m.blur, m.refraction, m.rim, hsla(c.surface), m.side_panel);
        quad(&mut s, right, gpui::hsla(0., 0., 0., 0.), rad, Some(hsla(c.border)));
        for i in 0..4 {
            quad(&mut s, rect(482., 30. + i as f32 * 24., 110., 6.), hsla(c.text), 3., None);
        }
        // a switch with a clear knob
        quad(&mut s, rect(482., 140., 34., 20.), hsla(c.accent), 10., None);
        let surf = hsla(c.surface);
        glass(&mut s, rect(498., 142., 16., 16.), 8., 1.5, 8., 0.6, Hsla { h: surf.h, s: surf.s.min(0.35), l: if dark { 0.92 } else { 1. }, a: if dark { 0.12 } else { 0.3 } }, m.controls);
        // a slider: its thumb at rest, hovered, pressed and pressed while stretched toward the pull
        quad(&mut s, rect(214., 250., 240., 2.), hsla(c.border), 1., None);
        for (i, (e, stretch)) in [(0., 0.), (0.48, 0.), (1., 0.), (1., 22.)].into_iter().enumerate() {
            let e: f32 = e;
            let g = 3. * e;
            let x = 220. + i as f32 * 56.;
            let tint = Hsla { h: surf.h, s: surf.s.min(0.35), l: if dark { 0.92 } else { 1. }, a: if dark { 0.12 - 0.06 * e } else { 0.3 - 0.18 * e } };
            glass_e(&mut s, rect(x - g, 242. - g, 18. + stretch + 2. * g, 18. + 2. * g), 9. + g, 1.5 - 0.5 * e, 8. + 4. * e, 0.6 + 0.2 * e, tint, m.controls, e);
        }
        // a pop-up menu over the chat
        let pop = rect(240., 60., 200., 140.);
        let tint = Hsla { a: hsla(c.surface).a.max(0.9), ..hsla(Color { a: 255, ..c.background.mix(Color { r: 255, g: 255, b: 255, a: 255 }, if dark { 0.0 } else { 0.6 }) }) };
        glass(&mut s, pop, rad, m.blur, m.refraction, m.rim, tint, m.popups);
        quad(&mut s, pop, gpui::hsla(0., 0., 0., 0.), rad, Some(hsla(c.border)));
        for i in 0..4 {
            quad(&mut s, rect(256., 80. + i as f32 * 28., 140., 6.), hsla(c.text), 3., None);
        }
        s.finish();
        let img = r.render_scene_to_image(&s, Size { width: DevicePixels(w as i32), height: DevicePixels(h as i32) }).expect("render");
        if let Some(dir) = &shots {
            std::fs::create_dir_all(dir).unwrap();
            img.save(dir.join(format!("{}.png", t.label().replace(' ', "-").to_lowercase()))).unwrap();
        }
    }
}
