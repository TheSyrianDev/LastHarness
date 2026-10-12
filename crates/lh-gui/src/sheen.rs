//! The two marks that make a pane look like Windows 7 style glass: a diagonal reflection in the top left corner
//! and a fine pattern of diagonal hairlines. This file only works out where the lines and shapes go, in pixels
//! from the top left corner of a pane, so it is tested without a window. `style.rs` draws them.

/// Most hairlines drawn on one pane. The gap grows on a very large pane so the cost stays the same.
pub const MAX_HAIRLINES: usize = 220;

/// A straight line from `a` to `b`.
pub type Seg = ((f32, f32), (f32, f32));

/// Lines at 45 degrees (running from the lower left up to the upper right), `gap` pixels apart, covering a
/// `w` by `h` area and cut off at its edges. A gap below 3 px is taken as 3 px, and the gap is widened until there
/// are at most `MAX_HAIRLINES` lines.
pub fn hairlines(w: f32, h: f32, gap: f32) -> Vec<Seg> {
    if !(w.is_finite() && h.is_finite() && gap.is_finite()) || w < 1. || h < 1. {
        return vec![];
    }
    let mut gap = gap.max(3.);
    let reach = w + h;
    if reach / gap > MAX_HAIRLINES as f32 {
        gap = reach / MAX_HAIRLINES as f32;
    }
    let mut out = vec![];
    let mut d = gap;
    // the line x + y = d crosses the area from (d, 0) or (w, d - w) to (0, d) or (d - h, h)
    while d < reach {
        let a = if d <= w { (d, 0.) } else { (w, d - w) };
        let b = if d <= h { (0., d) } else { (d - h, h) };
        out.push((a, b));
        d += gap;
    }
    out
}

/// The reflection: a large triangle in the top left corner and a narrow band running parallel to its long edge, as
/// two polygons. Both stay inside the `w` by `h` pane, and neither touches the rounded corner (the corner is cut
/// by `radius`).
pub fn reflection(w: f32, h: f32, radius: f32) -> [Vec<(f32, f32)>; 2] {
    if !(w.is_finite() && h.is_finite()) || w < 24. || h < 24. {
        return [vec![], vec![]];
    }
    let cut = (radius.max(0.) * 0.62).min(w / 4.).min(h / 4.);
    // how far the long edge of the triangle reaches along the top and the left side
    let d = (w * 0.62).min(h * 0.6);
    let slope = 0.9;
    let (xa, ya) = (d.min(w), (d * slope).min(h));
    let tri = vec![(cut, 0.), (xa, 0.), (0., ya), (0., cut)];
    // a thinner band beyond it, the same slope
    let (x0, x1) = ((d * 1.12).min(w), (d * 1.34).min(w));
    let (y0, y1) = ((d * 1.12 * slope).min(h), (d * 1.34 * slope).min(h));
    let band = if x0 < x1 - 1. && y0 < y1 - 1. { vec![(x0, 0.), (x1, 0.), (0., y1), (0., y0)] } else { vec![] };
    [tri, band]
}

/// The area the hairlines may use: the pane inset so that the corners of the area stay inside rounded corners.
pub fn hairline_area(w: f32, h: f32, radius: f32) -> (f32, f32, f32, f32) {
    let inset = (radius.max(0.) * 0.3).min(w / 4.).min(h / 4.);
    (inset, inset, (w - 2. * inset).max(0.), (h - 2. * inset).max(0.))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inside(p: (f32, f32), w: f32, h: f32) -> bool {
        p.0 >= -0.01 && p.0 <= w + 0.01 && p.1 >= -0.01 && p.1 <= h + 0.01
    }

    #[test]
    fn every_hairline_starts_and_ends_on_the_edge_of_the_area_at_45_degrees() {
        for (w, h, gap) in [(300., 200., 6.), (200., 300., 5.), (800., 700., 4.), (50., 50., 3.), (1000., 40., 8.)] {
            let lines = hairlines(w, h, gap);
            assert!(!lines.is_empty(), "{w}x{h}");
            for (a, b) in &lines {
                assert!(inside(*a, w, h) && inside(*b, w, h), "{a:?} {b:?} in {w}x{h}");
                assert!(((b.0 - a.0) + (b.1 - a.1)).abs() < 0.01 + (b.0 - a.0).abs() * 0.0, "a 45 degree line: dx = -dy ({a:?} to {b:?})");
                assert!(a.0 > b.0 && a.1 < b.1, "runs from the lower left to the upper right");
            }
        }
    }

    #[test]
    fn the_number_of_lines_is_capped_however_big_the_pane_is() {
        assert!(hairlines(5000., 4000., 3.).len() <= MAX_HAIRLINES);
        assert!(hairlines(900., 1000., 3.).len() <= MAX_HAIRLINES);
        assert!(hairlines(300., 200., 6.).len() < 90);
        assert!(hairlines(900., 1000., 6.).len() > 100, "a normal pane gets a fine pattern");
    }

    #[test]
    fn a_gap_below_three_pixels_is_three_and_bad_numbers_give_nothing() {
        assert_eq!(hairlines(100., 100., 0.).len(), hairlines(100., 100., 3.).len());
        assert_eq!(hairlines(100., 100., -4.).len(), hairlines(100., 100., 3.).len());
        for (w, h, g) in [(f32::NAN, 100., 5.), (100., f32::INFINITY, 5.), (100., 100., f32::NAN), (0., 100., 5.), (100., 0.5, 5.), (-5., 100., 5.)] {
            assert!(hairlines(w, h, g).is_empty(), "{w} {h} {g}");
        }
    }

    #[test]
    fn the_reflection_stays_inside_the_pane_and_leaves_the_rounded_corner_alone() {
        for (w, h, r) in [(300., 200., 16.), (900., 1000., 16.), (200., 900., 12.), (60., 60., 16.), (1500., 90., 18.)] {
            let [tri, band] = reflection(w, h, r);
            assert!(tri.len() >= 3, "{w}x{h}");
            for p in tri.iter().chain(band.iter()) {
                assert!(inside(*p, w, h), "{p:?} in {w}x{h}");
            }
            // the corner point itself (0, 0) is never in the shape: it would poke out of the rounded corner
            assert!(!tri.contains(&(0., 0.)) && !band.contains(&(0., 0.)));
            // the first and last points are cut back from the corner by most of the radius
            let cut = tri[0].0;
            assert!(cut >= r.min(w / 4.).min(h / 4.) * 0.5, "cut {cut} for radius {r}");
        }
    }

    #[test]
    fn a_pane_too_small_for_a_reflection_gets_none() {
        assert_eq!(reflection(10., 200., 16.), [vec![], vec![]]);
        assert_eq!(reflection(f32::NAN, 200., 16.), [vec![], vec![]]);
    }

    #[test]
    fn the_hairline_area_is_inset_by_a_third_of_the_corner_and_never_negative() {
        assert_eq!(hairline_area(300., 200., 16.), (4.8, 4.8, 290.4, 190.4));
        let (x, y, w, h) = hairline_area(10., 10., 40.);
        assert!(x <= 2.5 && y <= 2.5 && w >= 0. && h >= 0.);
        assert_eq!(hairline_area(300., 200., -5.), (0., 0., 300., 200.));
    }
}
