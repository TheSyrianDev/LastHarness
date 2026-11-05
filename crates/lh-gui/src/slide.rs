//! One set of thumb maths for every glass slider and switch (the model slider, the context-length slider, the
//! Chat/Assistant/Project switch and the colour brightness slider), so they all feel the same. It follows the
//! approach of Radix's Slider and Switch (the primitives behind shadcn/ui):
//!
//! - the value maps to a fraction of the track, and the thumb's edges stay inside the track: at 0 its start edge
//!   meets the track's start, at 1 its end edge meets the track's end (Radix's `getThumbInBoundsOffset`);
//! - the value snaps to the nearest step while the pointer moves (Radix's `getClosestValueIndex` and step rounding);
//! - the drawn thumb eases to the snapped place (the CSS transition), instead of being dragged pixel by pixel.
//!
//! The glass touch on top: while held, the thumb's leading edge stretches toward the pointer, softly capped, and the
//! thumb flattens a little to keep its volume. The stretch never pushes the thumb past either end of the track.

/// The nearest stop to a thumb whose start edge is `raw` pixels along the track, with `gap` pixels between `n` stops.
pub fn snap(raw: f32, gap: f32, n: usize) -> usize {
    let last = n.max(1) - 1;
    if gap <= 0. || !raw.is_finite() {
        return 0;
    }
    ((raw / gap).round().max(0.) as usize).min(last)
}

/// How far the thumb's start edge can travel: the track's length less the thumb.
pub fn travel(gap: f32, n: usize) -> f32 {
    gap.max(0.) * (n.max(1) - 1) as f32
}

/// How a held thumb is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Held {
    /// The stop the pointer is nearest; the thumb eases there.
    pub stop: usize,
    /// Where that stop puts the thumb's start edge.
    pub target: f32,
    /// Where the thumb's start edge is drawn this frame.
    pub left: f32,
    /// How much longer than usual the thumb is drawn, toward the pointer.
    pub stretch: f32,
    /// How much shorter across it is drawn, so it keeps its volume.
    pub squash: f32,
}

/// The held thumb for a pointer that puts the start edge at `raw`, while the eased thumb is drawn at `drawn`, on a
/// track of `n` stops `gap` apart, for a thumb `size` pixels across. The stretch is the gap between the drawn thumb
/// and the pointer, eased toward at most 40% of `size`, and only the edge on the pointer's side moves.
pub fn held(raw: f32, drawn: f32, gap: f32, n: usize, size: f32) -> Held {
    let end = travel(gap, n);
    let raw = if raw.is_finite() { raw.clamp(0., end) } else { 0. };
    let drawn = if drawn.is_finite() { drawn.clamp(0., end) } else { raw };
    let stop = snap(raw, gap, n);
    let pull = raw - drawn;
    let cap = (size * 0.4).max(1.);
    // 1 - e^-x never exceeds x, so the stretch never reaches past the pointer and so never past the track
    let stretch = cap * (1. - (-pull.abs() / cap).exp());
    let left = if pull < 0. { drawn - stretch } else { drawn };
    Held { stop, target: stop as f32 * gap, left, stretch, squash: squash(stretch) }
}

/// How much a thumb stretched by `stretch` pixels flattens: a quarter of the stretch, taken evenly off both sides.
pub fn squash(stretch: f32) -> f32 {
    stretch.max(0.) * 0.25
}

/// A continuous slider: the fraction (0..1) for a pointer at `at` pixels along a `track` long, for a thumb `size`
/// across. The pointer holds the thumb by its middle, so the ends are reached with the thumb fully inside.
pub fn fraction_at(at: f32, track: f32, size: f32) -> f32 {
    let room = (track - size).max(1.);
    ((at - size / 2.) / room).clamp(0., 1.)
}

/// Where a continuous slider's thumb start edge sits for fraction `f`, inside a `track` long.
pub fn edge_at(f: f32, track: f32, size: f32) -> f32 {
    let f = if f.is_finite() { f.clamp(0., 1.) } else { 0. };
    f * (track - size).max(0.)
}

/// `f` rounded to steps of `step` (0 means no steps), kept in 0..1.
pub fn step_round(f: f32, step: f32) -> f32 {
    let f = f.clamp(0., 1.);
    if step > 0. { ((f / step).round() * step).clamp(0., 1.) } else { f }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snaps_to_the_nearest_stop_and_stays_on_the_track() {
        assert_eq!(snap(0., 60., 5), 0);
        assert_eq!(snap(29., 60., 5), 0);
        assert_eq!(snap(31., 60., 5), 1);
        assert_eq!(snap(-50., 60., 5), 0);
        assert_eq!(snap(9999., 60., 5), 4);
        assert_eq!(snap(f32::NAN, 60., 5), 0);
        assert_eq!(snap(10., 0., 5), 0);
        assert_eq!(snap(10., 60., 0), 0);
    }

    #[test]
    fn a_resting_thumb_is_not_stretched() {
        let h = held(120., 120., 60., 5, 22.);
        assert_eq!((h.stop, h.target, h.left, h.stretch, h.squash), (2, 120., 120., 0., 0.));
    }

    #[test]
    fn the_thumb_never_overhangs_the_track() {
        let (gap, n, size) = (60., 5, 22.);
        let end = travel(gap, n);
        for d in 0..=48 {
            for p in -20..=60 {
                let (drawn, raw) = (d as f32 * 5., p as f32 * 5.);
                let h = held(raw, drawn, gap, n, size);
                assert!(h.left >= -1e-4, "start edge inside at raw {raw} drawn {drawn}");
                assert!(h.left + size + h.stretch <= end + size + 1e-4, "end edge inside at raw {raw} drawn {drawn}");
                assert!(h.stretch <= size * 0.4 + 1e-4);
            }
        }
    }

    #[test]
    fn the_stretch_grows_toward_the_pull_and_is_smooth() {
        let mut prev = held(120., 120., 60., 5, 22.).stretch;
        for i in 1..=200 {
            let h = held(120. + i as f32 * 0.5, 120., 60., 5, 22.);
            assert_eq!(h.left, 120., "pulling right keeps the left edge");
            assert!(h.stretch >= prev && h.stretch - prev < 0.6, "no jump at {i}");
            prev = h.stretch;
        }
        let h = held(100., 120., 60., 5, 22.);
        assert!(h.left < 120. && (h.left + h.stretch - 120.).abs() < 1e-4, "pulling left keeps the right edge");
    }

    #[test]
    fn every_control_uses_the_same_feel() {
        // the same pull, as a share of the thumb, stretches each thumb by the same share of its size
        let a = held(60. + 11., 60., 60., 5, 22.);
        let b = held(88. + 14., 88., 88., 3, 28.);
        assert!((a.stretch / 22. - b.stretch / 28.).abs() < 0.02);
    }

    #[test]
    fn continuous_sliders_keep_the_thumb_inside() {
        assert_eq!(fraction_at(0., 176., 18.), 0.);
        assert_eq!(fraction_at(9., 176., 18.), 0.);
        assert_eq!(fraction_at(167., 176., 18.), 1.);
        assert_eq!(fraction_at(500., 176., 18.), 1.);
        assert_eq!(edge_at(0., 176., 18.), 0.);
        assert_eq!(edge_at(1., 176., 18.), 158.);
        for i in 0..=100 {
            let f = i as f32 / 100.;
            let mid = edge_at(f, 176., 18.) + 9.;
            assert!((fraction_at(mid, 176., 18.) - f).abs() < 1e-5, "round trip at {f}");
        }
        assert!((step_round(0.333, 0.25) - 0.25).abs() < 1e-6);
        assert_eq!(step_round(1.4, 0.), 1.);
    }
}
