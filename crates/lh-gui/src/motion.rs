//! The one rule for motion: things ease out, a press answers in 140 ms, a panel opens or closes in 220 ms, and
//! under reduced motion (or a look that turns animation off) nothing moves. This module holds the numbers and the
//! timing of a panel sliding, apart from the drawing, so they can be tested.

use std::time::{Duration, Instant};

pub const PRESS_MS: u64 = 140;
pub const PANEL_MS: u64 = 220;

/// How long a press or hover takes. `theme` is the look's own animation time; zero means that look has none, and
/// reduced motion arrives as zero too.
pub fn press(theme: Duration) -> Duration {
    if theme.is_zero() { Duration::ZERO } else { Duration::from_millis(PRESS_MS) }
}

/// How long a panel takes to open or close.
pub fn panel(theme: Duration) -> Duration {
    if theme.is_zero() { Duration::ZERO } else { Duration::from_millis(PANEL_MS) }
}

/// The CSS curve `cubic-bezier(.22, 1, .36, 1)`: fast at first, then settling. `t` and the result are 0 to 1.
pub fn ease_out(t: f32) -> f32 {
    cubic_bezier(0.22, 1.0, 0.36, 1.0, t)
}

fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, x: f32) -> f32 {
    let x = x.clamp(0., 1.);
    let axis = |a: f32, b: f32, t: f32| 3. * a * (1. - t).powi(2) * t + 3. * b * (1. - t) * t * t + t.powi(3);
    // find the curve parameter whose x is the one asked for, then read y there
    let (mut lo, mut hi) = (0f32, 1f32);
    for _ in 0..24 {
        let mid = (lo + hi) / 2.;
        if axis(x1, x2, mid) < x { lo = mid } else { hi = mid }
    }
    axis(y1, y2, (lo + hi) / 2.)
}

/// A panel that is opening or closing. Its width is the full width times how far open it is; the full width is
/// remembered while it is shut, so closing keeps the panel's size as it slides away.
#[derive(Clone, Debug, Default)]
pub struct Reveal {
    seen: bool,
    open: bool,
    from: f32,
    started: Option<Instant>,
    full: f32,
}

impl Reveal {
    /// How far open, 0 to 1, at `now`.
    pub fn fraction(&self, now: Instant, dur: Duration) -> f32 {
        let to = if self.open { 1. } else { 0. };
        match self.started {
            Some(t0) if !dur.is_zero() => {
                let t = now.saturating_duration_since(t0).as_secs_f32() / dur.as_secs_f32();
                if t >= 1. { to } else { self.from + (to - self.from) * ease_out(t) }
            }
            _ => to,
        }
    }

    pub fn animating(&self, now: Instant, dur: Duration) -> bool {
        !dur.is_zero() && self.started.is_some_and(|t0| now.saturating_duration_since(t0) < dur)
    }

    /// The width to draw now. `target` is the width the layout gives the panel, 0 when it is hidden. The first
    /// call only learns the state; it never animates in from nothing.
    pub fn width(&mut self, target: f32, now: Instant, dur: Duration) -> f32 {
        let open = target > 0.;
        if open {
            self.full = target;
        }
        if !self.seen {
            self.seen = true;
            self.open = open;
        } else if open != self.open {
            self.from = self.fraction(now, dur);
            self.open = open;
            self.started = Some(now);
        }
        self.full * self.fraction(now, dur)
    }

    /// The width the panel is laid out at, whatever part of it shows: it keeps its size while it slides.
    pub fn full(&self) -> f32 {
        self.full
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: Duration = Duration::from_millis(PANEL_MS);

    #[test]
    fn the_numbers_are_the_ones_agreed_and_all_stop_when_the_look_has_no_animation() {
        assert_eq!((PRESS_MS, PANEL_MS), (140, 220));
        assert_eq!(press(Duration::from_millis(160)), Duration::from_millis(140));
        assert_eq!(panel(Duration::from_millis(160)), Duration::from_millis(220));
        assert_eq!(press(Duration::ZERO), Duration::ZERO, "reduced motion");
        assert_eq!(panel(Duration::ZERO), Duration::ZERO);
    }

    #[test]
    fn the_curve_starts_fast_and_settles() {
        assert!(ease_out(0.).abs() < 1e-3 && (ease_out(1.) - 1.).abs() < 1e-3);
        assert!(ease_out(0.2) > 0.4, "already well on the way a fifth in: {}", ease_out(0.2));
        let mut last = 0.;
        for i in 0..=20 {
            let v = ease_out(i as f32 / 20.);
            assert!(v >= last - 1e-4, "never goes back");
            last = v;
        }
        assert!(ease_out(0.9) < 1.0 + 1e-3);
    }

    #[test]
    fn a_panel_opens_over_220_ms_and_never_starts_from_nothing_on_the_first_look() {
        let t0 = Instant::now();
        let mut r = Reveal::default();
        assert_eq!(r.width(300., t0, D), 300., "shown from the start: no slide in");
        let mut r = Reveal::default();
        assert_eq!(r.width(0., t0, D), 0.);
        let w = r.width(300., t0, D);
        assert!(w < 1., "just opened: {w}");
        assert!(r.animating(t0, D));
        let mid = r.width(300., t0 + Duration::from_millis(60), D);
        assert!(mid > 100. && mid < 300., "part way: {mid}");
        let end = r.width(300., t0 + D, D);
        assert_eq!(end, 300.);
        assert!(!r.animating(t0 + D, D));
    }

    #[test]
    fn closing_keeps_the_size_and_turning_back_mid_way_goes_on_from_where_it_is() {
        let t0 = Instant::now();
        let mut r = Reveal::default();
        r.width(300., t0, D);
        let w = r.width(0., t0, D);
        assert!((w - 300.).abs() < 1., "just started closing: {w}");
        assert_eq!(r.full(), 300., "the panel keeps its size while it slides away");
        let mid = r.width(0., t0 + Duration::from_millis(60), D);
        assert!(mid > 0. && mid < 300.);
        // open again at that moment: no jump
        let again = r.width(300., t0 + Duration::from_millis(60), D);
        assert!((again - mid).abs() < 1.0, "{again} vs {mid}");
        let later = t0 + Duration::from_secs(5);
        r.width(0., later, D);
        assert_eq!(r.width(0., later + D, D), 0., "shut once the time is up");
        assert_eq!(r.full(), 300.);
    }

    #[test]
    fn without_animation_it_is_instant() {
        let t0 = Instant::now();
        let mut r = Reveal::default();
        r.width(0., t0, Duration::ZERO);
        assert_eq!(r.width(300., t0, Duration::ZERO), 300.);
        assert_eq!(r.width(0., t0, Duration::ZERO), 0.);
        assert!(!r.animating(t0, Duration::ZERO));
    }

    #[test]
    fn resizing_the_window_while_open_does_not_slide() {
        let t0 = Instant::now();
        let mut r = Reveal::default();
        r.width(300., t0, D);
        assert_eq!(r.width(260., t0 + Duration::from_millis(10), D), 260., "a new width while open follows at once");
    }
}
