//! Offscreen swatch sheet of every built-in look: background, a panel, an input, a button, the accent, the warn and
//! danger marks, a glass knob, and bars in the theme's own surface. No text, no window, the mouse is not touched.
//! Set `LH_THEME_SHOTS=<dir>` to save one PNG per look.

#![cfg(target_os = "linux")]

use gpui::{Bounds, ContentMask, Corners, DevicePixels, Hsla, PlatformHeadlessRenderer, Point, Quad, Scene, Size};
use gpui_wgpu::WgpuHeadlessRenderer;
use lh_gui::theme::{builtin, Color, Mode};

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

#[test]
fn every_look_renders_offscreen() {
    let Ok(mut r) = WgpuHeadlessRenderer::new() else {
        eprintln!("no GPU adapter for offscreen rendering; skipped");
        return;
    };
    let shots = std::env::var_os("LH_THEME_SHOTS").map(std::path::PathBuf::from);
    // plus the Omarchy look from this machine's current Omarchy theme, when there is one
    let omarchy = lh_gui::omarchy::current_dir().and_then(|d| lh_gui::omarchy::load(&d, lh_gui::omarchy::Hypr::read()));
    for t in builtin().into_iter().chain(omarchy) {
        let c = &t.colors;
        let dark = t.mode == Mode::Dark;
        let rad = t.roundness;
        let mut s = Scene::default();
        quad(&mut s, rect(0., 0., 360., 200.), hsla(if dark { Color { r: 0, g: 0, b: 0, a: 255 } } else { Color { r: 255, g: 255, b: 255, a: 255 } }), 0., None);
        quad(&mut s, rect(0., 0., 360., 200.), hsla(c.background), 0., None);
        quad(&mut s, rect(12., 12., 336., 176.), hsla(c.surface), rad, Some(hsla(c.border)));
        // text levels as bars
        quad(&mut s, rect(28., 28., 160., 8.), hsla(c.text), 4., None);
        quad(&mut s, rect(28., 44., 120., 6.), hsla(c.muted), 3., None);
        quad(&mut s, rect(28., 62., 200., 28.), hsla(c.sunken), (rad - 6.).max(4.), Some(hsla(c.focus_ring)));
        quad(&mut s, rect(28., 100., 70., 26.), hsla(c.button), (rad - 6.).max(4.), None);
        quad(&mut s, rect(104., 100., 70., 26.), hsla(c.accent), (rad - 6.).max(4.), None);
        quad(&mut s, rect(124., 110., 30., 6.), hsla(c.accent_text), 3., None);
        quad(&mut s, rect(180., 100., 22., 26.), hsla(c.warn), 6., None);
        quad(&mut s, rect(208., 100., 22., 26.), hsla(c.danger), 6., None);
        // a switch track and a glass knob tinted as App::glass tints it
        quad(&mut s, rect(250., 100., 34., 20.), hsla(c.accent), 10., None);
        let surf = hsla(c.surface);
        let kb = rect(266., 102., 16., 16.);
        s.insert_primitive(gpui::Glass {
            order: 0, pad: 0, bounds: kb, content_mask: ContentMask { bounds: kb }, corner_radius: 8.0.into(), blur: 1.5.into(), refraction: 8.0.into(), rim: 0.6,
            tint: Hsla { h: surf.h, s: surf.s.min(0.35), l: if dark { 0.92 } else { 1. }, a: if dark { 0.12 } else { 0.3 } }, variant: gpui::GlassVariant::Regular.index(), energy: 0.,
        });
        s.finish();
        let img = r.render_scene_to_image(&s, Size { width: DevicePixels(360), height: DevicePixels(200) }).expect("render");
        if let Some(dir) = &shots {
            std::fs::create_dir_all(dir).unwrap();
            img.save(dir.join(format!("{}.png", t.label().replace(' ', "-").to_lowercase()))).unwrap();
        }
    }
}
