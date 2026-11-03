//! Whether a streaming list should keep following its newest line. The usual rule: follow while the view is at the
//! bottom, stop as soon as the person scrolls up, keep their place still while text streams in below, and follow
//! again when they come back to the bottom. This keeps only the decision, so it is tested without a window.
//!
//! Offsets follow GPUI's scroll handle: the offset is zero at the top and goes negative as the view moves down, and
//! `max` is how far the content can scroll (zero when it all fits).

/// How close to the bottom still counts as the bottom, in pixels.
pub const BOTTOM_SLOP: f32 = 24.;

pub fn at_bottom(offset_y: f32, max_y: f32) -> bool {
    max_y <= 0. || -offset_y >= max_y - BOTTOM_SLOP
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Follow {
    following: bool,
    /// The offset the program last put the view at. A different offset means the person moved it.
    placed_at: f32,
    /// New text arrived while the person was reading higher up.
    unseen: bool,
}

impl Default for Follow {
    fn default() -> Self {
        Follow { following: true, placed_at: 0., unseen: false }
    }
}

impl Follow {
    /// Look at where the view is now. If it is not where the program last put it, the person scrolled: they follow
    /// again only if they ended at the bottom. Call this before deciding anything, and every frame.
    pub fn observe(&mut self, offset_y: f32, max_y: f32) {
        if (offset_y - self.placed_at).abs() > 0.5 {
            self.following = at_bottom(offset_y, max_y);
            self.placed_at = offset_y;
            if self.following {
                self.unseen = false;
            }
        }
    }

    /// New text arrived. Returns whether to move the view to the end; if not, the person's place is left alone and
    /// the "Jump to latest" button is wanted.
    pub fn on_new_content(&mut self, offset_y: f32, max_y: f32) -> bool {
        self.observe(offset_y, max_y);
        if !self.following {
            self.unseen = true;
        }
        self.following
    }

    /// The program moved the view to this offset.
    pub fn placed(&mut self, offset_y: f32) {
        self.placed_at = offset_y;
        self.following = true;
        self.unseen = false;
    }

    pub fn following(&self) -> bool {
        self.following
    }

    /// Show the "Jump to latest" button: the view is above the bottom and text has arrived since.
    pub fn wants_jump_button(&self) -> bool {
        !self.following && self.unseen
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn near_the_bottom_counts_as_the_bottom_and_short_content_always_does() {
        assert!(at_bottom(-1000., 1000.));
        assert!(at_bottom(-980., 1000.), "within a few pixels");
        assert!(!at_bottom(-900., 1000.));
        assert!(!at_bottom(0., 1000.));
        assert!(at_bottom(0., 0.), "nothing to scroll");
        assert!(at_bottom(0., -5.));
    }

    #[test]
    fn it_follows_by_default_and_keeps_following_as_text_grows() {
        let mut f = Follow::default();
        assert!(f.following() && !f.wants_jump_button());
        // text arrives, the program moves to the end each time; the content grows between events
        let mut max = 100.;
        for _ in 0..5 {
            assert!(f.on_new_content(f.placed_at, max));
            f.placed(-max);
            max += 40.;
        }
        assert!(f.following() && !f.wants_jump_button());
    }

    #[test]
    fn scrolling_up_stops_following_and_leaves_their_place_alone() {
        let mut f = Follow::default();
        f.placed(-500.);
        // the person scrolls up to -200 while the model is still writing
        assert!(!f.on_new_content(-200., 500.), "no jump to the bottom");
        assert!(!f.following());
        assert!(f.wants_jump_button());
        // more text arrives, the content is taller, the view has not moved: still not following, place unchanged
        assert!(!f.on_new_content(-200., 900.));
        assert!(!f.on_new_content(-200., 1400.));
        assert!(f.wants_jump_button());
    }

    #[test]
    fn coming_back_to_the_bottom_follows_again_and_hides_the_button() {
        let mut f = Follow::default();
        f.placed(-500.);
        assert!(!f.on_new_content(-100., 600.));
        assert!(f.wants_jump_button());
        // the person scrolls down to within a few pixels of the end
        f.observe(-590., 600.);
        assert!(f.following() && !f.wants_jump_button());
        assert!(f.on_new_content(-590., 640.), "the next text is followed");
    }

    #[test]
    fn scrolling_a_little_but_staying_at_the_bottom_keeps_following() {
        let mut f = Follow::default();
        f.placed(-500.);
        f.observe(-490., 500.);
        assert!(f.following());
    }

    #[test]
    fn the_button_only_appears_once_text_has_arrived_after_scrolling_up() {
        let mut f = Follow::default();
        f.placed(-500.);
        f.observe(-100., 500.);
        assert!(!f.following() && !f.wants_jump_button(), "nothing new yet");
        f.on_new_content(-100., 520.);
        assert!(f.wants_jump_button());
    }

    #[test]
    fn sending_or_jumping_always_goes_to_the_bottom() {
        let mut f = Follow::default();
        f.placed(-500.);
        f.on_new_content(-100., 700.);
        assert!(f.wants_jump_button());
        f.placed(-700.);
        assert!(f.following() && !f.wants_jump_button());
        // and the program's own move is not mistaken for the person scrolling
        f.observe(-700., 700.);
        assert!(f.following());
    }

    #[test]
    fn a_view_the_layout_pulled_back_into_range_is_read_as_a_scroll_to_where_it_is() {
        let mut f = Follow::default();
        f.placed(-900.);
        // the content got shorter (another chat, a deleted message) and the offset was clamped to the new end
        f.observe(-300., 300.);
        assert!(f.following());
    }
}
