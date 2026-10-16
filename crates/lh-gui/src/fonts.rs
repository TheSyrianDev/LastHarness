//! Fonts for Latin and Arabic text. The Latin font is the main font of the window. The Arabic font is its explicit
//! fallback: GPUI (cosmic-text) picks, grapheme by grapheme, the first font in the chain that has the glyph, so every
//! Arabic-script run is drawn by one Arabic font and Latin stays in the Latin font. No window code here.

use std::ops::Range;

/// Wanted for Arabic when the person has not chosen one, best first.
pub const ARABIC_DEFAULTS: [&str; 2] = ["Noto Naskh Arabic", "Noto Sans Arabic"];

/// The family names `fc-list` knows, one entry per family, sorted. Empty when fontconfig is not there.
pub fn installed_families() -> Vec<String> {
    let Some(o) = std::process::Command::new("fc-list").args([":", "family"]).output().ok().filter(|o| o.status.success()) else { return Vec::new() };
    parse_families(&String::from_utf8_lossy(&o.stdout))
}

/// `fc-list : family` prints `Name,Other name` per face. Take the first name of each line, once.
pub fn parse_families(out: &str) -> Vec<String> {
    let mut v: Vec<String> = out.lines().filter_map(|l| l.split(',').next()).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    v.sort_by_key(|s| s.to_lowercase());
    v.dedup();
    v
}

/// Families worth offering for Arabic: the ones whose name says so first, then the rest.
pub fn arabic_first(all: &[String]) -> Vec<String> {
    let says_arabic = |f: &String| {
        let l = f.to_lowercase();
        l.contains("arab") || l.contains("naskh") || l.contains("kufi")
    };
    let (a, b): (Vec<_>, Vec<_>) = all.iter().cloned().partition(says_arabic);
    a.into_iter().chain(b).collect()
}

/// The Arabic font to use: the one chosen if it is installed, else Noto Naskh Arabic, else Noto Sans Arabic, else none
/// (the system's own fallback then does the work).
pub fn arabic_font(chosen: &str, installed: &[String]) -> Option<String> {
    let has = |n: &str| installed.iter().any(|f| f.eq_ignore_ascii_case(n));
    if !chosen.is_empty() && has(chosen) {
        return Some(chosen.to_string());
    }
    ARABIC_DEFAULTS.iter().find(|n| has(n)).map(|n| n.to_string())
}

/// Letters of the Arabic script, with the marks and presentation forms that belong to a word.
pub fn is_arabic_script(c: char) -> bool {
    matches!(c as u32, 0x0600..=0x06FF | 0x0750..=0x077F | 0x08A0..=0x08FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF | 0x10EC0..=0x10EFF | 0x1EE00..=0x1EEFF)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Script {
    Arabic,
    Other,
}

/// Split text into runs of Arabic script and everything else. Spaces, digits and punctuation between two Arabic
/// runs stay in the Arabic run, so one Arabic phrase is one run; at the edges they go to the other side.
pub fn script_runs(text: &str) -> Vec<(Range<usize>, Script)> {
    let mut runs: Vec<(Range<usize>, Script)> = Vec::new();
    for (i, c) in text.char_indices() {
        let end = i + c.len_utf8();
        let s = if is_arabic_script(c) { Script::Arabic } else { Script::Other };
        match runs.last_mut() {
            Some((r, last)) if *last == s => r.end = end,
            _ => runs.push((i..end, s)),
        }
    }
    let mut out: Vec<(Range<usize>, Script)> = Vec::new();
    for (ix, (r, s)) in runs.iter().enumerate() {
        let between = *s == Script::Other && text[r.clone()].chars().all(|c| !c.is_alphabetic()) && ix > 0 && ix + 1 < runs.len();
        let s = if between { Script::Arabic } else { *s };
        match out.last_mut() {
            Some((o, last)) if *last == s => o.end = r.end,
            _ => out.push((r.clone(), s)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn families_are_parsed_once_each_and_sorted() {
        let out = "Noto Sans Arabic,Noto Sans Arabic Light\nDejaVu Sans\nNoto Sans Arabic\n\nabc,x\n";
        assert_eq!(parse_families(out), vec!["abc", "DejaVu Sans", "Noto Sans Arabic"]);
    }

    #[test]
    fn arabic_font_prefers_the_choice_then_naskh_then_sans() {
        let all: Vec<String> = ["DejaVu Sans", "Noto Sans Arabic", "Noto Kufi Arabic"].map(String::from).to_vec();
        assert_eq!(arabic_font("", &all).as_deref(), Some("Noto Sans Arabic"), "naskh missing");
        assert_eq!(arabic_font("Noto Kufi Arabic", &all).as_deref(), Some("Noto Kufi Arabic"));
        assert_eq!(arabic_font("Gone", &all).as_deref(), Some("Noto Sans Arabic"), "a chosen font that is not installed");
        let both: Vec<String> = ["Noto Naskh Arabic", "Noto Sans Arabic"].map(String::from).to_vec();
        assert_eq!(arabic_font("", &both).as_deref(), Some("Noto Naskh Arabic"));
        assert_eq!(arabic_font("", &["DejaVu Sans".to_string()]), None);
    }

    #[test]
    fn arabic_families_are_listed_first() {
        let all: Vec<String> = ["A Serif", "Noto Naskh Arabic", "Z Sans"].map(String::from).to_vec();
        assert_eq!(arabic_first(&all)[0], "Noto Naskh Arabic");
        assert_eq!(arabic_first(&all).len(), 3);
    }

    #[test]
    fn text_splits_into_arabic_and_other_runs() {
        let t = "Hi مرحبا بك (ok) x";
        let runs = script_runs(t);
        let got: Vec<(&str, Script)> = runs.iter().map(|(r, s)| (&t[r.clone()], *s)).collect();
        assert_eq!(got, vec![("Hi ", Script::Other), ("مرحبا بك", Script::Arabic), (" (ok) x", Script::Other)]);
        assert!(script_runs("").is_empty());
        assert_eq!(script_runs("abc").len(), 1);
        assert_eq!(runs.iter().map(|(r, _)| r.len()).sum::<usize>(), t.len(), "no gaps");
    }

    #[test]
    fn a_phrase_with_a_space_and_a_number_between_arabic_words_is_one_run() {
        let runs = script_runs("واحد 2 اثنان");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].1, Script::Arabic);
    }
}
