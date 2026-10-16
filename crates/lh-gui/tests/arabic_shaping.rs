//! Arabic marks (the dots) are separate zero-advance glyphs placed by x/y offsets from the shaper. GPUI's cosmic-text
//! system must keep those offsets, or every dot lands on the base glyph's origin: a word-final ب loses its dot place and
//! a final ي looks like ى. This test shapes through the same text system the window uses.

use gpui::{font, px, FontRun, PlatformTextSystem};
use gpui_wgpu::CosmicTextSystem;

#[test]
fn arabic_marks_keep_their_shaper_offsets() {
    let ts = CosmicTextSystem::new("Noto Sans");
    let text = "اكتب لي في";
    let Ok(id) = ts.font_id(&font("Noto Naskh Arabic")) else {
        eprintln!("skipped: Noto Naskh Arabic is not installed");
        return;
    };
    let line = ts.layout_line(text, px(16.), &[FontRun { len: text.len(), font_id: id }]);
    let glyphs: Vec<_> = line.runs.iter().flat_map(|r| r.glyphs.iter()).collect();
    // a mark is a second glyph for the same character as its base glyph: same index, and it must not sit at the base x
    let mut marks = 0;
    for w in glyphs.windows(2) {
        if w[0].index == w[1].index {
            marks += 1;
            assert!(w[0].position.x != w[1].position.x || w[0].position.y != w[1].position.y, "mark at index {} sits on its base glyph origin", w[0].index);
        }
    }
    assert!(marks >= 4, "expected dot marks for ت ب ي ف, found {marks}");
}

/// Cause of the old bug: GPUI's loader dropped any font without a Latin "m", so an Arabic-only font set as the explicit
/// fallback never loaded and cosmic-text's own script fallback (Noto Sans Arabic) drew the Arabic. The vendored crate
/// now lets fallback families skip that check. Here: the face that draws Arabic is the chosen one, Latin stays Latin.
#[test]
fn the_chosen_arabic_font_draws_arabic_and_latin_stays_in_the_main_font() {
    use gpui::FontFallbacks;
    let ts = CosmicTextSystem::new("Noto Sans");
    for chosen in ["Noto Naskh Arabic", "Noto Kufi Arabic", "Noto Sans Arabic"] {
        let mut f = font("Noto Sans");
        f.fallbacks = Some(FontFallbacks::from_fonts(vec![chosen.to_string()]));
        let Ok(id) = ts.font_id(&f) else {
            eprintln!("skipped: Noto Sans is not installed");
            return;
        };
        if ts.font_id(&font(chosen)).is_err() {
            eprintln!("skipped: {chosen} is not installed");
            continue;
        }
        let text = "Hi \u{645}\u{631}\u{62d}\u{628}\u{627} ok";
        let line = ts.layout_line(text, px(16.), &[FontRun { len: text.len(), font_id: id }]);
        let mut seen: Vec<(String, bool)> = Vec::new();
        for r in &line.runs {
            let fam = ts.font_family_name(r.font_id).unwrap_or_default();
            for g in &r.glyphs {
                let arabic = text[g.index..].chars().next().is_some_and(lh_gui::fonts::is_arabic_script);
                seen.push((fam.clone(), arabic));
            }
        }
        assert!(seen.iter().any(|(_, a)| *a), "no Arabic glyphs");
        for (fam, arabic) in &seen {
            if *arabic {
                assert_eq!(fam, chosen, "Arabic must be drawn by the chosen font");
            } else {
                assert_eq!(fam, "Noto Sans", "Latin must stay in the main font");
            }
        }
    }
}
