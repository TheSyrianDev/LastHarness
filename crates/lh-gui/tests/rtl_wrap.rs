//! A wrapped right-to-left line must break in logical order: the first words of the paragraph sit on the first
//! row, at its right edge. This drives GPUI's real `shape_text` (wrapping included) over the real cosmic-text
//! system, so it covers every text path that goes through GPUI (chat text, markdown, the input box).

use std::sync::Arc;

use gpui::{font, px, rgba, Pixels, PlatformTextSystem, SharedString, TextRun, TextSystem, WindowTextSystem};
use gpui_wgpu::CosmicTextSystem;

const ARABIC: &str = "ذهبت إلى السوق صباح اليوم لأشتري بعض الفاكهة والخضروات الطازجة ثم عدت إلى البيت مع أصدقائي القدامى";

struct Row {
    /// (byte index, x) of every glyph in this row, in glyph-list order
    glyphs: Vec<(usize, Pixels)>,
}

fn system() -> Option<WindowTextSystem> {
    let platform = Arc::new(CosmicTextSystem::new("Noto Sans"));
    platform.font_id(&font("Noto Sans")).expect("Noto Sans must be installed for the wrapping tests");
    let ts = Arc::new(TextSystem::new(platform));
    Some(WindowTextSystem::new(ts))
}

fn rows(wts: &WindowTextSystem, text: &str, family: &'static str, wrap: Pixels) -> Vec<Row> {
    let run = TextRun {
        len: text.len(),
        font: font(family),
        color: rgba(0x000000ff).into(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let lines = wts.shape_text(SharedString::from(text.to_string()), px(16.), &[run], Some(wrap), None).unwrap();
    let layout = &*lines[0];
    let mut starts: Vec<(usize, usize)> = layout.wrap_boundaries().iter().map(|b| (b.run_ix, b.glyph_ix)).collect();
    starts.push((usize::MAX, 0));
    let mut out = vec![Row { glyphs: vec![] }];
    let mut next = 0;
    for (ri, run) in layout.runs().iter().enumerate() {
        for (gi, g) in run.glyphs.iter().enumerate() {
            if (ri, gi) == starts[next] {
                next += 1;
                out.push(Row { glyphs: vec![] });
            }
            out.last_mut().unwrap().glyphs.push((g.index, g.position.x));
        }
    }
    out
}

fn width_of(wts: &WindowTextSystem, text: &str, family: &'static str) -> Pixels {
    rows(wts, text, family, px(100000.)).iter().flat_map(|r| r.glyphs.iter()).map(|g| g.1).fold(px(0.), |a, b| a.max(b))
}

#[test]
fn arabic_paragraph_wraps_in_logical_order() {
    let Some(wts) = system() else {
        eprintln!("skipped: Noto Sans is not installed");
        return;
    };
    let full = width_of(&wts, ARABIC, "Noto Sans");
    let r = rows(&wts, ARABIC, "Noto Sans", full / 2.6);
    assert_eq!(r.len(), 3, "expected 3 rows");
    let min = |row: &Row| row.glyphs.iter().map(|g| g.0).min().unwrap();
    let max = |row: &Row| row.glyphs.iter().map(|g| g.0).max().unwrap();
    assert_eq!(min(&r[0]), 0, "row 0 must hold the first logical word");
    assert!(max(&r[0]) < min(&r[1]) && max(&r[1]) < min(&r[2]), "rows must follow logical order");
    // inside a row the first logical glyph sits at the right edge, the last at the left
    for row in &r {
        let first_ix = min(row);
        let first_x = row.glyphs.iter().filter(|g| g.0 == first_ix).map(|g| g.1).fold(px(-1.), |a, b| a.max(b));
        let rightmost = row.glyphs.iter().map(|g| g.1).fold(px(-1.), |a, b| a.max(b));
        // a mark of a neighbouring letter may sit a hair further over than the first letter's own glyph
        assert!(first_x + px(4.) >= rightmost, "first logical glyph of a row must be at its right edge");
        let last_ix = max(row);
        let last_x = row.glyphs.iter().filter(|g| g.0 == last_ix).map(|g| g.1).fold(px(1e9), |a, b| a.min(b));
        let leftmost = row.glyphs.iter().map(|g| g.1).fold(px(1e9), |a, b| a.min(b));
        assert!(last_x <= leftmost + px(4.), "last logical glyph of a row must be at its left edge");
    }
    // rows are laid out left to right in one growing x space, so the painter can subtract the row start
    let starts: Vec<Pixels> = r.iter().map(|row| row.glyphs.iter().map(|g| g.1).fold(px(1e9), |a, b| a.min(b))).collect();
    assert!(starts[0] < starts[1] && starts[1] < starts[2]);
}

#[test]
fn mixed_line_keeps_english_phrase_whole_and_in_order() {
    let Some(wts) = system() else {
        eprintln!("skipped: Noto Sans is not installed");
        return;
    };
    let text = format!("{ARABIC} the quick brown fox {ARABIC}");
    let full = width_of(&wts, &text, "Noto Sans");
    let r = rows(&wts, &text, "Noto Sans", full / 3.5);
    assert!(r.len() >= 3);
    // each English word stays whole on one row and reads left to right (rows may break between the words)
    for word in ["the", "quick", "brown", "fox"] {
        let start = text.find(word).unwrap();
        let end = start + word.len();
        let row = r.iter().find(|row| row.glyphs.iter().any(|g| g.0 == start)).unwrap();
        let mut eng: Vec<(usize, Pixels)> = row.glyphs.iter().copied().filter(|g| g.0 >= start && g.0 < end).collect();
        assert_eq!(eng.len(), end - start, "{word} must stay on one row");
        eng.sort_by_key(|g| g.0);
        assert!(eng.windows(2).all(|w| w[0].1 < w[1].1), "{word} must run left to right");
    }
}

#[test]
fn english_wrapping_is_plain_left_to_right() {
    let Some(wts) = system() else {
        eprintln!("skipped: fonts missing");
        return;
    };
    let text = "the quick brown fox jumps over the lazy dog and keeps running through the green field";
    let r = rows(&wts, text, "Noto Sans", px(160.));
    assert!(r.len() >= 3);
    let mut last = 0;
    for (i, row) in r.iter().enumerate() {
        let first = row.glyphs[0].0;
        assert!(first >= last, "rows must follow text order");
        if i > 0 {
            assert!(text[..first].ends_with(' '), "English rows start at a word: {:?}", &text[first..]);
        }
        assert!(row.glyphs.windows(2).all(|w| w[0].0 < w[1].0 && w[0].1 < w[1].1), "glyphs ascend left to right");
        last = row.glyphs.last().unwrap().0;
    }
}

const VOWELLED: &str = "البَحْرُ كِتَابٌ مَفْتُوحٌ أَوَّلًا تَنْقُلُ أَوْرَاقَهُ الرِّيَاحُ وَيَحْفِرُ الصَّيَّادُونَ أَسْمَاءَهُمْ عَلَى صُخُورِهِ";

#[test]
fn vowelled_arabic_breaks_only_at_spaces() {
    let probe = CosmicTextSystem::new("Noto Sans");
    let wts = system().unwrap();
    let mut tried = 0;
    for family in ["Noto Sans", "Noto Naskh Arabic", "Noto Naskh Arabic Medium", "Noto Sans Arabic UI", "Noto Kufi Arabic"] {
        if probe.font_id(&font(family)).is_err() {
            eprintln!("font not loadable here: {family}");
            continue;
        }
        tried += 1;
        let full = width_of(&wts, VOWELLED, family);
        for div in [2.5f32, 4.0, 6.0] {
            let r = rows(&wts, VOWELLED, family, full / div);
            assert!(r.len() >= 2, "{family} /{div}: expected wrapping");
            for row in &r[1..] {
                let first = row.glyphs.iter().map(|g| g.0).min().unwrap();
                let before = VOWELLED[..first].chars().next_back().unwrap();
                assert_eq!(before, ' ', "{family} /{div}: row starts inside a word at byte {first}");
            }
        }
    }
    assert!(tried >= 1);
}
