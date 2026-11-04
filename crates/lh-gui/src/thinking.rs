//! Live progress while the model is thinking. A small model can think for minutes before it writes a word, and the
//! chat used to show only "Working… 1 step". This keeps a count of the thinking pieces that arrive, how fast they
//! come, and how long it has been, and words them for the folded line, for a screen reader (rarely) and for the
//! calm hint beside Stop. No window code, so it is tested. Nothing here stops anything: only the person does.
//!
//! A "token" is one piece of thinking from the stream. llama-server sends one token to a piece, so counting pieces is
//! counting tokens; another server may batch, and then the number is lower than the real count (the speed too).

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::chrome::thousands;
use crate::transcript::{duration, duration_ar};

/// After this long, or this many tokens, the hint beside Stop appears. (The pick is parked for the user.)
pub const LONG_SECS: u64 = 60;
pub const LONG_TOKENS: u64 = 2000;
/// The speed is measured over the last few seconds, so it follows a slowdown.
const WINDOW: Duration = Duration::from_secs(5);

#[derive(Clone, Debug, Default)]
pub struct Thinking {
    start: Option<Instant>,
    tokens: u64,
    seen: VecDeque<Instant>,
}

/// What can be said about the thinking right now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snap {
    pub tokens: u64,
    pub secs: u64,
    /// Tokens a second over the last few seconds; `None` in the first second, when a rate would mean nothing yet.
    pub rate: Option<u32>,
}

impl Thinking {
    pub fn on_piece(&mut self, now: Instant) {
        self.start.get_or_insert(now);
        self.tokens += 1;
        self.seen.push_back(now);
        while self.seen.front().is_some_and(|t| now.saturating_duration_since(*t) > WINDOW + Duration::from_secs(1)) {
            self.seen.pop_front();
        }
    }

    /// Thinking is over: an answer began, a tool started, or the turn ended.
    pub fn reset(&mut self) {
        *self = Thinking::default();
    }

    pub fn active(&self) -> bool {
        self.start.is_some()
    }

    pub fn snapshot(&self, now: Instant) -> Option<Snap> {
        let start = self.start?;
        let elapsed = now.saturating_duration_since(start);
        let span = elapsed.min(WINDOW);
        let rate = (span >= Duration::from_secs(1)).then(|| {
            let recent = self.seen.iter().filter(|t| now.saturating_duration_since(**t) <= WINDOW).count();
            (recent as f64 / span.as_secs_f64()).round() as u32
        });
        Some(Snap { tokens: self.tokens, secs: elapsed.as_secs(), rate })
    }
}

fn tokens_en(n: u64) -> String {
    if n == 1 { "1 token".into() } else { format!("{} tokens", thousands(n)) }
}

/// "Thinking… 3,414 tokens, 31 tokens a second, 1 min 50 s". The speed is left out until there is one.
pub fn progress_text(s: Snap, rtl: bool) -> String {
    if rtl {
        let mut v = vec![format!("{} رمز", thousands(s.tokens))];
        if let Some(r) = s.rate {
            v.push(format!("{r} رمز في الثانية"));
        }
        v.push(duration_ar(s.secs));
        return format!("يفكر… {}", v.join("، "));
    }
    let mut v = vec![tokens_en(s.tokens)];
    if let Some(r) = s.rate {
        v.push(if r == 1 { "1 token a second".into() } else { format!("{r} tokens a second") });
    }
    v.push(duration(s.secs));
    format!("Thinking… {}", v.join(", "))
}

/// What a screen reader is told. It changes only every 10 seconds, with the count rounded to the nearest hundred and
/// no speed, so a polite status does not chatter.
pub fn spoken_text(s: Snap, rtl: bool) -> String {
    let secs = s.secs / 10 * 10;
    let tokens = if s.tokens < 100 { s.tokens } else { (s.tokens + 50) / 100 * 100 };
    if rtl {
        return format!("يفكر، حوالي {} رمز، {}", thousands(tokens), duration_ar(secs));
    }
    format!("Thinking, about {}, {}", tokens_en(tokens), duration(secs))
}

/// Has it gone on long enough to say so?
pub fn is_long(s: Snap) -> bool {
    s.secs >= LONG_SECS || s.tokens >= LONG_TOKENS
}

/// The calm hint beside Stop. It never stops anything by itself.
pub fn hint_text(rtl: bool) -> &'static str {
    if rtl { "ما زال يفكر. أوقفه لتتخطى ذلك، أو انتظر." } else { "Still thinking. Stop to skip, or wait." }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(n: u64, per_second: u64, from: Instant) -> (Thinking, Instant) {
        let mut t = Thinking::default();
        let mut now = from;
        for i in 0..n {
            now = from + Duration::from_millis(i * 1000 / per_second);
            t.on_piece(now);
        }
        (t, now)
    }

    #[test]
    fn nothing_is_said_before_any_thinking_and_after_a_reset() {
        let mut t = Thinking::default();
        assert!(!t.active() && t.snapshot(Instant::now()).is_none());
        t.on_piece(Instant::now());
        assert!(t.active());
        t.reset();
        assert!(!t.active() && t.snapshot(Instant::now()).is_none());
    }

    #[test]
    fn it_counts_the_pieces_and_the_seconds_since_the_first() {
        let t0 = Instant::now();
        let (t, last) = feed(3414, 31, t0);
        let s = t.snapshot(last).unwrap();
        assert_eq!(s.tokens, 3414);
        assert_eq!(s.secs, 110, "3414 pieces at 31 a second is 110 s");
    }

    #[test]
    fn the_speed_is_over_the_last_five_seconds_and_follows_a_slowdown() {
        let t0 = Instant::now();
        let (t, last) = feed(600, 30, t0);
        let fast = t.snapshot(last).unwrap();
        assert!((29..=31).contains(&fast.rate.unwrap()), "{fast:?}");
        // ten seconds of nothing: the speed falls to zero instead of staying at the old number
        let later = last + Duration::from_secs(10);
        assert_eq!(t.snapshot(later).unwrap().rate, Some(0));
        // two seconds after the last piece the window still holds some
        let r = t.snapshot(last + Duration::from_secs(2)).unwrap().rate.unwrap();
        assert!((15..=20).contains(&r), "{r}");
    }

    #[test]
    fn there_is_no_speed_in_the_first_second() {
        let t0 = Instant::now();
        let (t, last) = feed(20, 31, t0);
        assert_eq!(t.snapshot(last).unwrap().rate, None);
    }

    #[test]
    fn the_line_reads_plainly_in_english_with_and_without_a_speed() {
        assert_eq!(progress_text(Snap { tokens: 3414, secs: 110, rate: Some(31) }, false), "Thinking… 3,414 tokens, 31 tokens a second, 1 min 50 s");
        assert_eq!(progress_text(Snap { tokens: 1, secs: 0, rate: None }, false), "Thinking… 1 token, 0 s");
        assert_eq!(progress_text(Snap { tokens: 12, secs: 3, rate: Some(1) }, false), "Thinking… 12 tokens, 1 token a second, 3 s");
        assert_eq!(progress_text(Snap { tokens: 120_500, secs: 3600, rate: Some(33) }, false), "Thinking… 120,500 tokens, 33 tokens a second, 60 min");
    }

    #[test]
    fn the_line_has_an_arabic_form_with_the_same_numbers() {
        let a = progress_text(Snap { tokens: 3414, secs: 110, rate: Some(31) }, true);
        assert!(a.starts_with("يفكر…") && a.contains("3,414") && a.contains("31") && a.contains("1 د 50 ث"), "{a}");
        let b = progress_text(Snap { tokens: 5, secs: 0, rate: None }, true);
        assert!(!b.contains("في الثانية"), "{b}");
    }

    #[test]
    fn the_spoken_status_only_changes_every_ten_seconds_and_a_hundred_tokens() {
        let say = |tokens, secs| spoken_text(Snap { tokens, secs, rate: Some(31) }, false);
        assert_eq!(say(3414, 110), "Thinking, about 3,400 tokens, 1 min 50 s");
        let all: std::collections::BTreeSet<String> = (110..120).map(|s| say(3414, s)).collect();
        assert_eq!(all.len(), 1, "ten seconds in a row say the same thing");
        assert_ne!(say(3414, 110), say(3414, 120), "and the next ten say something new");
        assert_eq!(say(3449, 110), say(3401, 110), "the count moves in hundreds");
        assert_eq!(say(7, 3), "Thinking, about 7 tokens, 0 s");
        assert!(!say(3414, 110).contains("a second"), "no speed in the spoken form");
        assert!(spoken_text(Snap { tokens: 3414, secs: 110, rate: None }, true).starts_with("يفكر"));
    }

    #[test]
    fn the_hint_comes_after_a_minute_or_two_thousand_tokens_and_not_before() {
        assert!(!is_long(Snap { tokens: 1999, secs: 59, rate: None }));
        assert!(is_long(Snap { tokens: 1999, secs: 60, rate: None }));
        assert!(is_long(Snap { tokens: 2000, secs: 5, rate: None }));
        assert!(!is_long(Snap { tokens: 0, secs: 0, rate: None }));
        assert_eq!(hint_text(false), "Still thinking. Stop to skip, or wait.");
        assert!(!hint_text(true).is_empty());
    }

    #[test]
    fn the_memory_for_the_speed_stays_small_however_long_it_thinks() {
        let t0 = Instant::now();
        let (t, _) = feed(100_000, 100, t0);
        assert!(t.seen.len() < 700, "{}", t.seen.len());
        assert_eq!(t.tokens, 100_000);
    }
}
