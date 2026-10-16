//! Mixed Arabic and English lines in visual order. GPUI shapes a whole line with the Unicode bidi rules
//! (cosmic-text, paragraph direction from the first strong letter), but it wraps by cutting the shaped glyphs
//! from the left, so a wrapped right-to-left line breaks at the wrong end. This module gives the order and the
//! mirrored brackets for a line, so a caller that wraps by itself can wrap first and order each row after.

use std::ops::Range;

use unicode_bidi::{BidiInfo, Level};

/// One piece of a row, in the order it is drawn from left to right.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisualRun {
    pub range: Range<usize>,
    pub rtl: bool,
}

/// Does the paragraph run right to left? Decided by its first strong letter, as the Unicode rules say.
pub fn base_is_rtl(text: &str) -> bool {
    BidiInfo::new(text, None).paragraphs.first().is_some_and(|p| p.level.is_rtl())
}

/// The runs of one line, left to right as drawn. `base_rtl` forces the paragraph direction (the app's RTL mode);
/// `None` follows the first strong letter.
pub fn visual_runs(text: &str, base_rtl: Option<bool>) -> Vec<VisualRun> {
    let level = base_rtl.map(|r| if r { Level::rtl() } else { Level::ltr() });
    let info = BidiInfo::new(text, level);
    let mut out = Vec::new();
    for para in &info.paragraphs {
        let (levels, runs) = info.visual_runs(para, para.range.clone());
        for r in runs {
            out.push(VisualRun { rtl: levels[r.start].is_rtl(), range: r });
        }
    }
    out
}

fn mirror(c: char) -> char {
    match c {
        '(' => ')',
        ')' => '(',
        '[' => ']',
        ']' => '[',
        '{' => '}',
        '}' => '{',
        '<' => '>',
        '>' => '<',
        '«' => '»',
        '»' => '«',
        c => c,
    }
}

/// The text of a line as it reads on screen from left to right, with brackets mirrored inside right-to-left runs.
pub fn visual_string(text: &str, base_rtl: Option<bool>) -> String {
    let mut s = String::with_capacity(text.len());
    for run in visual_runs(text, base_rtl) {
        let part = &text[run.range];
        if run.rtl {
            s.extend(part.chars().rev().map(mirror));
        } else {
            s.push_str(part);
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_strong_letter_sets_the_direction() {
        assert!(base_is_rtl("(مرحبا) hello"));
        assert!(!base_is_rtl("Hello مرحبا"));
        assert!(!base_is_rtl("123"));
    }

    #[test]
    fn english_inside_arabic_keeps_its_order_and_the_first_arabic_word_is_drawn_last() {
        let t = "كيف حالك؟ (I'm well, thanks and you?)";
        let runs = visual_runs(t, None);
        assert!(runs.iter().any(|r| r.rtl) && runs.iter().any(|r| !r.rtl));
        let en = runs.iter().find(|r| !r.rtl).expect("english run");
        assert!(t[en.range.clone()].contains("I'm well, thanks and you"), "one run in reading order");
        let last = runs.last().unwrap();
        assert!(last.rtl && t[last.range.clone()].starts_with("كيف"));
    }

    #[test]
    fn brackets_are_mirrored_in_right_to_left_runs_only() {
        // reversed, then each bracket mirrored: the pair still encloses the letter on screen
        assert_eq!(visual_string("(أ)", None), "(أ)");
        assert_eq!(visual_string("أ (ب", None), "ب) أ");
        assert_eq!(visual_string("(a)", None), "(a)");
    }

    #[test]
    fn forcing_the_base_direction_moves_a_leading_english_phrase_right() {
        let t = "Hello مرحبا";
        let ltr = visual_runs(t, Some(false));
        let rtl = visual_runs(t, Some(true));
        assert!(!ltr[0].rtl && t[ltr[0].range.clone()].starts_with("Hello"));
        assert!(rtl.last().is_some_and(|r| t[r.range.clone()].contains("Hello")));
    }

    #[test]
    fn empty_text_has_no_runs() {
        assert!(visual_runs("", None).is_empty());
    }
}
