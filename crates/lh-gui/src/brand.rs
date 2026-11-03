//! The LastHarness mark, drawn natively. Geometry and colours are copied from the user's brand kit
//! (the LastHarness brand kit); nothing here is designed anew.
//! Everything is on the kit's 64-unit grid and scaled when painted.

use gpui::{Hsla, Path, PathBuilder, Pixels, Point, point, px};

/// A rounded rectangle on the 64 grid: x, y, width, height, corner radius.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RRect(pub f32, pub f32, pub f32, pub f32, pub f32);

pub const FACE: RRect = RRect(8., 18., 38., 38., 15.);
pub const SLOT: (f32, f32, f32) = (49., 15., 6.);
pub const EYE_NEAR: RRect = RRect(18.6, 30.6, 6., 11., 3.);
pub const EYE_FAR_FRONT: RRect = RRect(28.4, 30.6, 6., 11., 3.);
pub const EYE_FAR_GLANCE: RRect = RRect(30.1, 31.4, 4.6, 10.2, 2.3);
/// The glance: translate (2.4, -2), rotate -8 degrees about (27, 37).
pub const GLANCE_SHIFT: (f32, f32) = (2.4, -2.);
pub const GLANCE_TURN_DEG: f32 = -8.;
pub const GLANCE_PIVOT: (f32, f32) = (27., 37.);
/// The glance turn takes frames 159 to 181 at 60 fps in the timing sheet.
pub const GLANCE_MS: u64 = 367;

/// The Grove palette from tokens.json, as 0xRRGGBB.
pub mod grove {
    pub const FOREST: u32 = 0x15251c;
    pub const MOSS: u32 = 0x5f7d48;
    pub const LICHEN: u32 = 0xb2c27c;
    pub const CREAM: u32 = 0xf2ead6;
    pub const BUSY: u32 = 0xe0a050;
    pub const READY: u32 = 0x6fcf9f;
    pub const NEEDS_YOU: u32 = 0xd0784f;
}

/// What the slot says: the status colours of the brand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Busy,
    Ready,
    NeedsYou,
}

impl Status {
    pub fn slot(self) -> u32 {
        match self {
            Status::Busy => grove::BUSY,
            Status::Ready => grove::READY,
            Status::NeedsYou => grove::NEEDS_YOU,
        }
    }
}

/// The mark's three colours (face, eye, slot) per `markColours` in tokens.json.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Colours {
    pub face: u32,
    pub eye: u32,
    pub slot: u32,
}

impl Colours {
    pub fn on_dark() -> Self {
        Colours { face: grove::LICHEN, eye: grove::FOREST, slot: grove::BUSY }
    }
    pub fn on_light() -> Self {
        Colours { face: grove::FOREST, eye: grove::CREAM, slot: grove::BUSY }
    }
    pub fn for_mode(dark: bool) -> Self {
        if dark { Self::on_dark() } else { Self::on_light() }
    }
}

/// cubic-bezier(x1, y1, x2, y2) at progress `t`, like CSS.
pub fn bezier(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
    let t = t.clamp(0., 1.);
    let curve = |a: f32, b: f32, s: f32| 3. * a * s * (1. - s) * (1. - s) + 3. * b * s * s * (1. - s) + s * s * s;
    let (mut lo, mut hi) = (0f32, 1f32);
    for _ in 0..30 {
        let mid = (lo + hi) / 2.;
        if curve(x1, x2, mid) < t { lo = mid } else { hi = mid }
    }
    curve(y1, y2, (lo + hi) / 2.)
}

/// The timing sheet's `inOut` easing, cubic-bezier(0.65, 0, 0.35, 1).
pub fn ease_in_out(t: f32) -> f32 {
    bezier(0.65, 0., 0.35, 1., t)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Points round a rounded rectangle, clockwise, `per_corner` steps on each corner.
pub fn rrect_points(r: RRect, per_corner: usize) -> Vec<(f32, f32)> {
    let RRect(x, y, w, h, rad) = r;
    let rad = rad.min(w / 2.).min(h / 2.);
    let corners = [(x + w - rad, y + rad, -90f32), (x + w - rad, y + h - rad, 0.), (x + rad, y + h - rad, 90.), (x + rad, y + rad, 180.)];
    let mut pts = Vec::with_capacity(4 * (per_corner + 1));
    for (cx, cy, start) in corners {
        for i in 0..=per_corner {
            let a = (start + 90. * i as f32 / per_corner as f32).to_radians();
            pts.push((cx + rad * a.cos(), cy + rad * a.sin()));
        }
    }
    pts
}

/// Points round a circle.
pub fn circle_points(cx: f32, cy: f32, r: f32, steps: usize) -> Vec<(f32, f32)> {
    (0..steps).map(|i| {
        let a = std::f32::consts::TAU * i as f32 / steps as f32;
        (cx + r * a.cos(), cy + r * a.sin())
    }).collect()
}

/// The eyes' transform at glance amount `g` (0 looks at you, 1 is the logo pose), as SVG applies
/// `translate(2.4g -2g) rotate(-8g 27 37)`.
pub fn glance_point(p: (f32, f32), g: f32) -> (f32, f32) {
    let a = (GLANCE_TURN_DEG * g).to_radians();
    let (cx, cy) = GLANCE_PIVOT;
    let (dx, dy) = (p.0 - cx, p.1 - cy);
    (cx + dx * a.cos() - dy * a.sin() + GLANCE_SHIFT.0 * g, cy + dx * a.sin() + dy * a.cos() + GLANCE_SHIFT.1 * g)
}

/// The far eye's shape at glance amount `g`: narrower as it turns away.
pub fn far_eye(g: f32) -> RRect {
    let (a, b) = (EYE_FAR_FRONT, EYE_FAR_GLANCE);
    RRect(lerp(a.0, b.0, g), lerp(a.1, b.1, g), lerp(a.2, b.2, g), lerp(a.3, b.3, g), lerp(a.4, b.4, g))
}

/// The mark's shapes on the 64 grid, in paint order, with the colour of each: face, two eyes, slot.
pub fn shapes(g: f32, colours: Colours) -> Vec<(Vec<(f32, f32)>, u32)> {
    let eye = |r: RRect| rrect_points(r, 8).into_iter().map(|p| glance_point(p, g)).collect::<Vec<_>>();
    vec![
        (rrect_points(FACE, 16), colours.face),
        (eye(EYE_NEAR), colours.eye),
        (eye(far_eye(g)), colours.eye),
        (circle_points(SLOT.0, SLOT.1, SLOT.2, 48), colours.slot),
    ]
}

pub fn hsla(rgb: u32) -> Hsla {
    gpui::rgb(rgb).into()
}

/// The mark as GPUI paths, `size` pixels square with its top-left at `origin`.
pub fn paths(origin: Point<Pixels>, size: f32, g: f32, colours: Colours) -> Vec<(Path<Pixels>, Hsla)> {
    let k = size / 64.;
    shapes(g, colours).into_iter().filter_map(|(pts, c)| {
        let mut b = PathBuilder::fill();
        for (i, (x, y)) in pts.into_iter().enumerate() {
            let p = point(origin.x + px(x * k), origin.y + px(y * k));
            if i == 0 { b.move_to(p) } else { b.line_to(p) }
        }
        b.close();
        b.build().ok().map(|p| (p, hsla(c)))
    }).collect()
}

/// A canvas element that paints the mark at `size` pixels, glance amount `g`.
pub fn mark(size: f32, g: f32, colours: Colours) -> impl gpui::IntoElement {
    use gpui::Styled;
    gpui::canvas(|_, _, _| {}, move |bounds, _, window, _| {
        for (path, c) in paths(bounds.origin, size, g, colours) {
            window.paint_path(path, c);
        }
    })
    .size(px(size))
    .flex_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_logo_pose_matches_the_svg_transform() {
        // translate(2.4 -2) rotate(-8 27 37) on the near eye's top-left corner, worked by hand
        let (x, y) = glance_point((18.6, 30.6), 1.);
        let a = (-8f32).to_radians();
        let (dx, dy) = (18.6 - 27., 30.6 - 37.);
        assert!((x - (27. + dx * a.cos() - dy * a.sin() + 2.4)).abs() < 1e-4);
        assert!((y - (37. + dx * a.sin() + dy * a.cos() - 2.)).abs() < 1e-4);
        assert_eq!(glance_point((10., 10.), 0.), (10., 10.));
        assert_eq!(far_eye(1.), EYE_FAR_GLANCE);
        assert_eq!(far_eye(0.), EYE_FAR_FRONT);
    }

    #[test]
    fn everything_fits_the_safe_circle_and_eyes_stay_on_the_face() {
        for g in [0., 0.5, 1.] {
            let s = shapes(g, Colours::on_dark());
            for (pts, _) in &s {
                for &(x, y) in pts {
                    assert!(((x - 32.).powi(2) + (y - 32.).powi(2)).sqrt() <= 30.05, "({x}, {y}) leaves r30");
                }
            }
            for (pts, _) in &s[1..3] {
                assert!(pts.iter().all(|&(x, y)| x > 8. && x < 46. && y > 18. && y < 56.));
            }
        }
    }

    #[test]
    fn in_out_easing_is_symmetric_and_ends_at_the_ends() {
        assert!(ease_in_out(0.).abs() < 1e-3 && (ease_in_out(1.) - 1.).abs() < 1e-3);
        assert!((ease_in_out(0.5) - 0.5).abs() < 1e-3);
        assert!(ease_in_out(0.2) < 0.2 && ease_in_out(0.8) > 0.8);
    }
}
