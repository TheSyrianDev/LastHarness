//! Offscreen render of the brand mark as the app paints it (logo pose, on dark and light, and the three
//! status slots). Nothing is shown on screen and the mouse is not touched. Set `LH_BRAND_SHOTS=<dir>` to save PNGs.

#![cfg(target_os = "linux")]

use gpui::{Bounds, ContentMask, DevicePixels, PlatformHeadlessRenderer, Point, Quad, Scene, Size, point, px};
use gpui_wgpu::WgpuHeadlessRenderer;
use lh_gui::brand::{self, Colours, Status, grove};

fn render(r: &mut WgpuHeadlessRenderer, bg: u32, g: f32, colours: Colours) -> image::RgbaImage {
    let mut scene = Scene::default();
    let full = Bounds { origin: Point::default(), size: Size { width: 256f32.into(), height: 256f32.into() } };
    let c = brand::hsla(bg);
    scene.insert_primitive(Quad { order: 0, border_style: Default::default(), bounds: full, content_mask: ContentMask { bounds: full }, background: c.into(), border_color: c, corner_radii: Default::default(), border_widths: Default::default() });
    for (path, colour) in brand::paths(point(px(0.), px(0.)), 256., g, colours) {
        let mut p = path.scale(1.);
        p.content_mask = ContentMask { bounds: full };
        p.color = colour.into();
        scene.insert_primitive(p);
    }
    scene.finish();
    r.render_scene_to_image(&scene, Size { width: DevicePixels(256), height: DevicePixels(256) }).expect("render")
}

#[test]
fn the_mark_renders_offscreen_in_its_colours() {
    let Ok(mut r) = WgpuHeadlessRenderer::new() else {
        eprintln!("no GPU adapter for offscreen rendering; skipped");
        return;
    };
    let shots = std::env::var_os("LH_BRAND_SHOTS").map(std::path::PathBuf::from);
    let rgb = |c: u32| [(c >> 16) as u8, (c >> 8) as u8, c as u8];
    let near = |a: [u8; 4], b: [u8; 3]| a[..3].iter().zip(b).all(|(x, y)| x.abs_diff(y) <= 3);
    let cases = [
        ("on-dark", grove::FOREST, 1., Colours::on_dark()),
        ("on-light", grove::CREAM, 1., Colours::on_light()),
        ("front-on-dark", grove::FOREST, 0., Colours::on_dark()),
        ("ready", grove::FOREST, 1., Colours { slot: Status::Ready.slot(), ..Colours::on_dark() }),
        ("needs-you", grove::FOREST, 1., Colours { slot: Status::NeedsYou.slot(), ..Colours::on_dark() }),
    ];
    for (name, bg, g, c) in cases {
        let img = render(&mut r, bg, g, c);
        if let Some(dir) = &shots {
            std::fs::create_dir_all(dir).unwrap();
            img.save(dir.join(format!("mark-{name}.png"))).unwrap();
        }
        // 4 px per grid unit: the face middle, the slot middle, the corner, and the near eye's middle
        assert!(near(img.get_pixel(27 * 4, 50 * 4).0, rgb(c.face)), "{name}: face");
        assert!(near(img.get_pixel(49 * 4, 15 * 4).0, rgb(c.slot)), "{name}: slot");
        assert!(near(img.get_pixel(2, 2).0, rgb(bg)), "{name}: background");
        let eye = brand::glance_point((21.6, 36.1), g);
        assert!(near(img.get_pixel((eye.0 * 4.) as u32, (eye.1 * 4.) as u32).0, rgb(c.eye)), "{name}: eye");
    }
}
