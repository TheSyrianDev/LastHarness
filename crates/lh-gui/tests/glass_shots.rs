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

/// A grey page with rows of dark "letters" (small bars), and a glass panel over its middle.
fn page(r: &mut WgpuHeadlessRenderer, refraction: f32, rim: f32, energy: f32, tint_a: f32) -> image::RgbaImage {
    let mut scene = Scene::default();
    let q = |s: &mut Scene, bounds: Bounds<gpui::ScaledPixels>, c: gpui::Hsla| {
        s.insert_primitive(Quad { order: 0, border_style: Default::default(), bounds, content_mask: ContentMask { bounds }, background: c.into(), border_color: c, corner_radii: Default::default(), border_widths: Default::default() });
    };
    q(&mut scene, rect(0., 0., 240., 160.), gpui::hsla(0., 0., 0.55, 1.));
    for row in 0..12 {
        for col in 0..40 {
            if (row * 7 + col * 3) % 5 != 0 {
                q(&mut scene, rect(4. + col as f32 * 6., 6. + row as f32 * 13., 3., 8.), gpui::hsla(0., 0., 0.08, 1.));
            }
        }
    }
    let bounds = rect(40., 30., 160., 100.);
    scene.insert_primitive(gpui::Glass {
        order: 0, pad: 0, bounds, content_mask: ContentMask { bounds },
        corner_radius: 20.0.into(), blur: 0.0.into(), refraction: refraction.into(), rim,
        tint: gpui::hsla(0., 0., 1., tint_a), variant: gpui::GlassVariant::Regular.index(), energy,
    });
    scene.finish();
    r.render_scene_to_image(&scene, Size { width: DevicePixels(240), height: DevicePixels(160) }).expect("render")
}

#[test]
fn glass_bends_text_behind_its_edge_and_lights_its_rim() {
    let Ok(mut r) = WgpuHeadlessRenderer::new() else {
        eprintln!("no GPU adapter for offscreen rendering; skipped");
        return;
    };
    use gpui::glass_energy::*;
    let flat = page(&mut r, 0., 0., REST, 0.08);
    let bent = page(&mut r, 8., 0., REST, 0.08);
    let lit = page(&mut r, 8., 0.6, REST, 0.08);
    let pressed = page(&mut r, 8., 0.6, PRESSED, 0.08);
    let popup = page(&mut r, 8., 0.6, REST, 0.9);
    if let Some(dir) = std::env::var_os("LH_GLASS_SHOTS").map(std::path::PathBuf::from) {
        std::fs::create_dir_all(&dir).unwrap();
        for (n, img) in [("flat", &flat), ("bent", &bent), ("lit", &lit), ("pressed", &pressed), ("popup", &popup)] {
            img.save(dir.join(format!("glass-text-{n}.png"))).unwrap();
        }
    }
    let differ = |a: &image::RgbaImage, b: &image::RgbaImage, x0: u32, x1: u32, y: u32| (x0..x1).filter(|&x| a.get_pixel(x, y).0 != b.get_pixel(x, y).0).count();
    // the letters behind the bevel move: along a row crossing the left edge (glass from x 40, bevel about 24 px)
    let edge: usize = (34..44).map(|y| differ(&flat, &bent, 40, 64, y)).sum();
    assert!(edge > 60, "refraction barely moves what is behind the edge: {edge} pixels changed");
    // outside the glass nothing changes
    assert_eq!(differ(&flat, &bent, 0, 40, 80), 0);
    let lum = |img: &image::RgbaImage, x: u32, y: u32| img.get_pixel(x, y).0[..3].iter().map(|&c| c as u32).sum::<u32>();
    // the light: the top left rim and the bevel just inside it get clearly brighter
    let band = |img: &image::RgbaImage| (60..120).map(|x| lum(img, x, 31) + lum(img, x, 35)).sum::<u32>();
    assert!(band(&lit) > band(&bent) + 60 * 2 * 3 * 15, "the rim light is too faint: {} vs {}", band(&lit), band(&bent));
    // pressed glass catches more light than resting glass
    assert!(band(&pressed) > band(&lit), "pressing does not brighten the rim");
    // a nearly solid pop-up still lets the bend show at its rim
    let edge: usize = (34..44).map(|y| differ(&page(&mut r, 0., 0.6, REST, 0.9), &popup, 40, 56, y)).sum();
    assert!(edge > 30, "the pop-up's rim hides the bend: {edge}");
}
