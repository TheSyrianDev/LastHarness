//! Offscreen render of the liquid glass shader: every variant at rest, hover and pressed over coloured stripes.
//! Nothing is shown on screen and the mouse is not touched. Set `LH_GLASS_SHOTS=<dir>` to also save the pictures as PNG.

#![cfg(target_os = "linux")]

use gpui::{Bounds, ContentMask, DevicePixels, PlatformHeadlessRenderer, Point, Quad, Scene, Size};
use gpui_wgpu::WgpuHeadlessRenderer;

fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds<gpui::ScaledPixels> {
    Bounds { origin: Point { x: x.into(), y: y.into() }, size: Size { width: w.into(), height: h.into() } }
}

fn render(r: &mut WgpuHeadlessRenderer, variant: gpui::GlassVariant, energy: f32) -> image::RgbaImage {
    let mut scene = Scene::default();
    let colors = [gpui::red(), gpui::blue(), gpui::green(), gpui::yellow()];
    for i in 0..24 {
        let bounds = rect(i as f32 * 8., 0., 8., 96.);
        let c = colors[i % 4];
        scene.insert_primitive(Quad { order: 0, border_style: Default::default(), bounds, content_mask: ContentMask { bounds }, background: c.into(), border_color: c, corner_radii: Default::default(), border_widths: Default::default() });
    }
    let bounds = rect(24., 24., 144., 48.);
    scene.insert_primitive(gpui::Glass {
        order: 0, pad: 0, bounds, content_mask: ContentMask { bounds },
        corner_radius: 24.0.into(), blur: 2.0.into(), refraction: 8.0.into(), rim: 0.6,
        tint: gpui::hsla(0., 0., 1., 0.12), variant: variant.index(), energy,
    });
    scene.finish();
    r.render_scene_to_image(&scene, Size { width: DevicePixels(192), height: DevicePixels(96) }).expect("render")
}

#[test]
fn glass_variants_and_energy_offscreen() {
    let Ok(mut r) = WgpuHeadlessRenderer::new() else {
        eprintln!("no GPU adapter for offscreen rendering; skipped");
        return;
    };
    use gpui::{GlassVariant::*, glass_energy::*};
    let shots = std::env::var_os("LH_GLASS_SHOTS").map(std::path::PathBuf::from);
    let mut rest = Vec::new();
    for v in [Regular, Clear, RegularTinted, ClearTinted] {
        let [still, hover, pressed] = [REST, HOVER, PRESSED].map(|e| render(&mut r, v, e));
        if let Some(dir) = &shots {
            std::fs::create_dir_all(dir).unwrap();
            for (name, img) in [("rest", &still), ("hover", &hover), ("pressed", &pressed)] {
                img.save(dir.join(format!("glass-{v:?}-{name}.png"))).unwrap();
            }
        }
        // outside the glass the stripes are untouched
        assert_eq!(still.get_pixel(4, 4).0, [255, 0, 0, 255], "{v:?}");
        // more energy, brighter middle
        let sum = |img: &image::RgbaImage| img.get_pixel(96, 48).0[..3].iter().map(|&c| c as u32).sum::<u32>();
        assert!(sum(&still) < sum(&hover) && sum(&hover) < sum(&pressed), "{v:?}: energy does not brighten");
        rest.push(still);
    }
    for a in 0..4 {
        for b in a + 1..4 {
            assert_ne!(rest[a].as_raw(), rest[b].as_raw(), "variants {a} and {b} look the same");
        }
    }
}
