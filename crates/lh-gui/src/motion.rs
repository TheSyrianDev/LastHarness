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

/// A value gliding from where it was to a new target, such as the raised part of a switch or a slider's thumb.
#[derive(Clone, Debug, Default)]
pub struct Glide {
    from: f32,
    to: f32,
    started: Option<Instant>,
}

impl Glide {
    /// Start moving to `to` from wherever it is now. The first target is taken at once, with no movement.
    pub fn go(&mut self, to: f32, now: Instant, dur: Duration) {
        if self.started.is_none() {
            *self = Glide { from: to, to, started: Some(now) };
        } else if to != self.to {
            self.from = self.value(now, dur);
            self.to = to;
            self.started = Some(now);
        }
    }

    pub fn value(&self, now: Instant, dur: Duration) -> f32 {
        match self.started {
            Some(t0) if !dur.is_zero() => {
                let t = now.saturating_duration_since(t0).as_secs_f32() / dur.as_secs_f32();
                if t >= 1. { self.to } else { self.from + (self.to - self.from) * ease_out(t) }
            }
            _ => self.to,
        }
    }

    /// Stand still at `v`, as when something was dragged there by hand; the next `go` moves on from here.
    pub fn hold(&mut self, v: f32) {
        *self = Glide { from: v, to: v, started: Some(Instant::now() - Duration::from_secs(1)) };
    }

    pub fn moving(&self, now: Instant, dur: Duration) -> bool {
        self.started.is_some_and(|t0| !dur.is_zero() && now.saturating_duration_since(t0) < dur)
    }
}

/// The moving parts of every glass control, by name: how lit each drop is (its interaction energy: rest, hover,
/// pressed), how far a released thumb is still stretched, where a switch knob is. Each eases to its new target in the
/// press time instead of jumping. Shared by the drawing code (which sets targets and reads values at paint, when
/// the pointer and the drop's real bounds are known) and the mouse handlers (which hold a value at release).
#[derive(Clone, Debug, Default)]
pub struct Eases(std::collections::HashMap<String, Glide>);

impl Eases {
    /// Move `key` toward `to` and say where it is now and whether it is still moving.
    pub fn to(&mut self, key: &str, to: f32, now: Instant, dur: Duration) -> (f32, bool) {
        let g = self.0.entry(key.to_string()).or_default();
        g.go(to, now, dur);
        let v = g.value(now, dur);
        (v, g.moving(now, dur) && (v - to).abs() > 1e-4)
    }

    /// Where `key` is now, without changing its target; `or` when it was never set.
    pub fn now(&self, key: &str, now: Instant, dur: Duration, or: f32) -> f32 {
        self.0.get(key).map_or(or, |g| g.value(now, dur))
    }

    /// Stand `key` still at `v`, as a thumb is while it is held by hand.
    pub fn hold(&mut self, key: &str, v: f32) {
        self.0.entry(key.to_string()).or_default().hold(v);
    }
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

    #[test]
    fn a_glide_starts_still_then_eases_to_the_new_target() {
        let (t0, d) = (Instant::now(), Duration::from_millis(200));
        let mut g = Glide::default();
        g.go(10., t0, d);
        assert_eq!(g.value(t0, d), 10.);
        g.go(20., t0, d);
        let mid = g.value(t0 + Duration::from_millis(100), d);
        assert!(mid > 15. && mid < 20., "eases out: {mid}");
        assert_eq!(g.value(t0 + d, d), 20.);
        assert!(!g.moving(t0 + d, d));
        assert_eq!(g.value(t0, Duration::ZERO), 20., "reduced motion jumps");
    }

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

    #[test]
    fn glass_energy_eases_instead_of_jumping() {
        let (t0, d) = (Instant::now(), Duration::from_millis(140));
        let mut e = Eases::default();
        assert_eq!(e.to("k", 0., t0, d), (0., false), "the first target is taken at once");
        let (v, moving) = e.to("k", 1., t0, d);
        assert!(v < 1e-3 && moving, "pressed: it starts from rest");
        let mid = e.to("k", 1., t0 + Duration::from_millis(50), d).0;
        assert!(mid > 0.2 && mid < 1., "part way: {mid}");
        assert_eq!(e.now("k", t0 + d, d, 9.), 1.);
        // let go part way: it eases back from where it is, not from 1
        let back = e.to("k", 0., t0 + Duration::from_millis(50), d).0;
        assert!((back - mid).abs() < 1e-4);
        assert_eq!(e.now("unknown", t0, d, 0.5), 0.5);
        assert_eq!(e.to("z", 1., t0, Duration::ZERO).0, 1., "reduced motion: no easing");
    }

    #[test]
    fn a_released_stretch_shrinks_back_smoothly() {
        let d = Duration::from_millis(220);
        let mut e = Eases::default();
        e.hold("s", 14.);
        let t0 = Instant::now();
        let (start, moving) = e.to("s", 0., t0, d);
        assert!((start - 14.).abs() < 1e-3 && moving, "it starts from the held stretch");
        let later = e.to("s", 0., t0 + Duration::from_millis(60), d).0;
        assert!(later > 0. && later < 14.);
        assert_eq!(e.to("s", 0., t0 + d, d).0, 0.);
    }
}
