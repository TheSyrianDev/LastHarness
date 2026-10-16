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
