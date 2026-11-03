//! The arithmetic of scrolling a page in a view, in CSS pixels. Kept apart from the engine so it is tested alone.

/// One arrow key press, and how much of a screen a Page key moves (a little less than all of it, so the eye keeps
/// its place).
pub const LINE: f32 = 40.;
pub const PAGE_FRACTION: f32 = 0.9;

/// The furthest the page can be scrolled: the page's height less the view's, never below zero.
pub fn max_offset(content: f32, view: f32) -> f32 {
    if !content.is_finite() || !view.is_finite() {
        return 0.;
    }
    (content - view).max(0.)
}

/// Keep an offset inside the page.
pub fn clamp(offset: f32, content: f32, view: f32) -> f32 {
    if !offset.is_finite() {
        return 0.;
    }
    offset.clamp(0., max_offset(content, view))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Step {
    /// A number of pixels, positive to go down: the mouse wheel.
    Pixels(f32),
    Line(i32),
    Page(i32),
    Home,
    End,
}

/// Where the view is after a step.
pub fn apply(offset: f32, step: Step, content: f32, view: f32) -> f32 {
    let to = match step {
        Step::Pixels(d) => offset + d,
        Step::Line(n) => offset + n as f32 * LINE,
        Step::Page(n) => offset + n as f32 * view * PAGE_FRACTION,
        Step::Home => 0.,
        Step::End => f32::MAX,
    };
    clamp(to, content, view)
}

/// What a key does to the view, by the name the toolkit gives the key; `shift` makes space go up.
pub fn key_step(key: &str, shift: bool) -> Option<Step> {
    Some(match key {
        "down" => Step::Line(1),
        "up" => Step::Line(-1),
        "pagedown" => Step::Page(1),
        "pageup" => Step::Page(-1),
        "space" => Step::Page(if shift { -1 } else { 1 }),
        "home" => Step::Home,
        "end" => Step::End,
        _ => return None,
    })
}

/// Where the scroll bar thumb sits, as a fraction of the track (start, length), for drawing a thin indicator.
pub fn thumb(offset: f32, content: f32, view: f32) -> Option<(f32, f32)> {
    if !(content > view && content.is_finite() && view > 0.) {
        return None;
    }
    let len = (view / content).clamp(0.05, 1.);
    let start = (offset / (content - view)).clamp(0., 1.) * (1. - len);
    Some((start, len))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_limit_is_the_page_less_the_view_and_never_negative() {
        assert_eq!(max_offset(2000., 500.), 1500.);
        assert_eq!(max_offset(300., 500.), 0.);
        assert_eq!(max_offset(f32::NAN, 500.), 0.);
        assert_eq!(clamp(-40., 2000., 500.), 0.);
        assert_eq!(clamp(9999., 2000., 500.), 1500.);
        assert_eq!(clamp(f32::NAN, 2000., 500.), 0.);
    }

    #[test]
    fn wheel_arrows_pages_home_and_end_stay_inside_the_page() {
        let (c, v) = (2000., 500.);
        assert_eq!(apply(0., Step::Pixels(120.), c, v), 120.);
        assert_eq!(apply(100., Step::Pixels(-500.), c, v), 0.);
        assert_eq!(apply(0., Step::Line(1), c, v), 40.);
        assert_eq!(apply(0., Step::Line(-1), c, v), 0.);
        assert_eq!(apply(0., Step::Page(1), c, v), 450.);
        assert_eq!(apply(450., Step::Page(-1), c, v), 0.);
        assert_eq!(apply(300., Step::Home, c, v), 0.);
        assert_eq!(apply(300., Step::End, c, v), 1500.);
        assert_eq!(apply(1480., Step::Page(1), c, v), 1500., "stops at the end");
        assert_eq!(apply(10., Step::Line(1), 300., 500.), 0., "a short page does not move");
    }

    #[test]
    fn keys_map_to_steps_and_shift_space_goes_up() {
        assert_eq!(key_step("down", false), Some(Step::Line(1)));
        assert_eq!(key_step("pageup", false), Some(Step::Page(-1)));
        assert_eq!(key_step("space", false), Some(Step::Page(1)));
        assert_eq!(key_step("space", true), Some(Step::Page(-1)));
        assert_eq!(key_step("home", false), Some(Step::Home));
        assert_eq!(key_step("a", false), None);
        assert_eq!(key_step("tab", false), None, "Tab is for moving focus, not for the page");
    }

    #[test]
    fn the_thumb_shows_where_the_view_is() {
        assert_eq!(thumb(0., 300., 500.), None);
        let (s, l) = thumb(0., 2000., 500.).unwrap();
        assert_eq!((s, l), (0., 0.25));
        let (s, l) = thumb(1500., 2000., 500.).unwrap();
        assert!((s + l - 1.).abs() < 1e-6, "at the end the thumb touches the end of the track");
        assert!(thumb(0., 1e9, 500.).unwrap().1 >= 0.05, "never smaller than a few percent");
    }
}
