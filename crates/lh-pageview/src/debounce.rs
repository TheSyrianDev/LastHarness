//! A resize is not acted on at every pixel of a drag: the size must hold still for a moment first. The first size
//! is applied at once, so the page appears without waiting.

use std::time::{Duration, Instant};

/// A size in physical pixels with the screen scale.
pub type Size = (u32, u32, f32);

pub const SETTLE: Duration = Duration::from_millis(150);

#[derive(Debug, Default)]
pub struct Debounce {
    applied: Option<Size>,
    seen: Option<(Size, Instant)>,
}

impl Debounce {
    /// Called often with the size the panel has now. Returns `Some(size)` when the engine should be resized to it.
    pub fn poll(&mut self, now_size: Size, now: Instant) -> Option<Size> {
        if now_size.0 == 0 || now_size.1 == 0 {
            return None; // a panel with no room has nothing to draw
        }
        if self.applied.is_none() {
            self.applied = Some(now_size);
            self.seen = None;
            return Some(now_size);
        }
        if self.applied == Some(now_size) {
            self.seen = None;
            return None;
        }
        match self.seen {
            Some((s, since)) if s == now_size => {
                if now.saturating_duration_since(since) >= SETTLE {
                    self.applied = Some(now_size);
                    self.seen = None;
                    return Some(now_size);
                }
            }
            _ => self.seen = Some((now_size, now)),
        }
        None
    }

    /// Forget what was applied (the page was closed or replaced).
    pub fn reset(&mut self) {
        *self = Debounce::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_size_is_applied_at_once_and_a_steady_size_is_left_alone() {
        let (mut d, t0) = (Debounce::default(), Instant::now());
        assert_eq!(d.poll((400, 800, 1.), t0), Some((400, 800, 1.)));
        assert_eq!(d.poll((400, 800, 1.), t0 + Duration::from_secs(1)), None);
    }

    #[test]
    fn a_drag_is_applied_once_when_it_settles() {
        let (mut d, t0) = (Debounce::default(), Instant::now());
        d.poll((400, 800, 1.), t0);
        let ms = |n| t0 + Duration::from_millis(n);
        // the size changes every 20 ms for a while: nothing is applied
        for i in 1..=10u32 {
            assert_eq!(d.poll((400 + i * 10, 800, 1.), ms(i as u64 * 20)), None, "step {i}");
        }
        // it holds at 500 wide from 200 ms: not yet at 100 ms later, applied after 150 ms
        assert_eq!(d.poll((500, 800, 1.), ms(300)), None);
        assert_eq!(d.poll((500, 800, 1.), ms(340)), None);
        assert_eq!(d.poll((500, 800, 1.), ms(360)), Some((500, 800, 1.)));
        assert_eq!(d.poll((500, 800, 1.), ms(900)), None, "and only once");
    }

    #[test]
    fn a_change_of_screen_scale_counts_and_zero_sized_panels_are_ignored() {
        let (mut d, t0) = (Debounce::default(), Instant::now());
        assert_eq!(d.poll((0, 800, 1.), t0), None);
        assert_eq!(d.poll((400, 0, 1.), t0), None);
        d.poll((400, 800, 1.), t0);
        assert_eq!(d.poll((400, 800, 2.), t0 + Duration::from_millis(10)), None);
        assert_eq!(d.poll((400, 800, 2.), t0 + Duration::from_millis(200)), Some((400, 800, 2.)));
    }

    #[test]
    fn a_reset_starts_over() {
        let (mut d, t0) = (Debounce::default(), Instant::now());
        d.poll((400, 800, 1.), t0);
        d.reset();
        assert_eq!(d.poll((300, 700, 1.), t0 + Duration::from_millis(1)), Some((300, 700, 1.)));
    }
}
