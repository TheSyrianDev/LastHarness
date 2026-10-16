//! Wrapping for lines that contain right-to-left text (LastHarness patch).
//!
//! The shaper returns glyphs in visual order for the whole line. Cutting that list from the left puts the
//! LAST logical words on the first row of a right-to-left paragraph. Here the break points are chosen in
//! logical order (same rules as `LineLayout::compute_wrap_boundaries`), then every row is reordered for
//! display with the bidi rules (UAX#9 L1 and L2, paragraph direction from the first strong character).
//! The result is a new layout whose glyph list is row after row, left to right, so the painter and the
//! hit-testing code work without change. Lines with no reordering never come here.

use super::LineWrapper;
use super::line_layout::{LineLayout, ShapedGlyph, ShapedRun, WrapBoundary};
use crate::{FontId, Pixels, px};
use smallvec::SmallVec;
use std::collections::BTreeMap;
use unicode_bidi::BidiInfo;

/// Arabic, Hebrew and other spaced scripts: letters stay in their word, only spaces end a word.
/// Upstream's list only knows Latin, Cyrillic and a few more, so every Arabic letter would be a break point.
fn is_spaced_script_letter(c: char) -> bool {
    c.is_alphanumeric() && !(('\u{2E80}'..='\u{9FFF}').contains(&c) || ('\u{AC00}'..='\u{D7AF}').contains(&c))
}

struct Cluster {
    ch: char,
    x: Pixels,
    width: Pixels,
    level: u8,
    glyphs: Vec<(FontId, ShapedGlyph)>,
}

/// Reflow `layout` into rows. `None` means the line needs no change (no reordering, or it fits).
pub(super) fn reflow(
    layout: &LineLayout,
    text: &str,
    wrap_width: Pixels,
    max_lines: Option<usize>,
) -> Option<(LineLayout, SmallVec<[WrapBoundary; 1]>)> {
    if layout.width <= wrap_width || !text.chars().any(|c| c as u32 >= 0x590) {
        return None;
    }
    let info = BidiInfo::new(text, None);
    if !info.has_rtl() {
        return None;
    }
    let para_level = info.paragraphs.first()?.level.number();

    // One cluster per source index (a base glyph with its marks). The shaper's glyph order is not reliable for
    // right-to-left runs, so clusters come from the index and their place from the x positions.
    let mut by_index: BTreeMap<usize, Vec<(FontId, ShapedGlyph)>> = BTreeMap::new();
    for run in &layout.runs {
        for g in &run.glyphs {
            by_index.entry(g.index).or_default().push((run.font_id, g.clone()));
        }
    }
    let mut clusters: Vec<Cluster> = Vec::with_capacity(by_index.len());
    for (index, mut glyphs) in by_index {
        // a combining mark (harakat) belongs to the letter before it: never a break point of its own
        if info.original_classes.get(index) == Some(&unicode_bidi::BidiClass::NSM)
            && let Some(prev) = clusters.last_mut()
        {
            prev.x = glyphs.iter().map(|g| g.1.position.x).fold(prev.x, |a, b| a.min(b));
            prev.glyphs.append(&mut glyphs);
            continue;
        }
        let ch = text.get(index..)?.chars().next()?;
        let x = glyphs.iter().map(|g| g.1.position.x).fold(Pixels::MAX, |a, b| a.min(b));
        clusters.push(Cluster { ch, x, width: px(0.), level: info.levels.get(index)?.number(), glyphs });
    }
    // widths: distance to the next cluster to the right (clusters are in logical order here)
    let mut visual: Vec<usize> = (0..clusters.len()).collect();
    visual.sort_by(|&a, &b| clusters[a].x.partial_cmp(&clusters[b].x).unwrap_or(std::cmp::Ordering::Equal));
    for (n, &i) in visual.iter().enumerate() {
        let end = visual.get(n + 1).map_or(layout.width, |&j| clusters[j].x);
        clusters[i].width = (end - clusters[i].x).max(px(0.));
    }

    // Logical x of each cluster start, then the break points (same rules as the left-to-right code).
    let mut lx = Vec::with_capacity(clusters.len() + 1);
    let mut acc = px(0.);
    for c in &clusters {
        lx.push(acc);
        acc += c.width;
    }
    lx.push(acc);

    let mut starts: Vec<usize> = vec![0];
    let mut first_non_whitespace = None;
    let mut candidate: Option<usize> = None;
    let mut last_boundary = 0usize;
    let mut last_boundary_x = px(0.);
    let mut prev_ch = '\0';
    for (i, c) in clusters.iter().enumerate() {
        let ch = c.ch;
        if ch == '\n' {
            continue;
        }
        if LineWrapper::is_word_char(ch) || is_spaced_script_letter(ch) {
            if prev_ch == ' ' && ch != ' ' && first_non_whitespace.is_some() {
                candidate = Some(i);
            }
        } else if ch != ' ' && first_non_whitespace.is_some() {
            candidate = Some(i);
        }
        if ch != ' ' && first_non_whitespace.is_none() {
            first_non_whitespace = Some(i);
        }
        if lx[i + 1] - last_boundary_x > wrap_width && i > last_boundary {
            if let Some(max_lines) = max_lines
                && starts.len() > max_lines.saturating_sub(1)
            {
                break;
            }
            let b = candidate.take().unwrap_or(i);
            last_boundary = b;
            last_boundary_x = lx[b];
            starts.push(b);
        }
        prev_ch = ch;
    }
    if starts.len() == 1 {
        return None;
    }
    starts.push(clusters.len());

    // Each row: L1 (trailing whitespace takes the paragraph level), L2 (reverse by level), then place.
    let mut runs: Vec<ShapedRun> = Vec::new();
    let mut boundaries: SmallVec<[WrapBoundary; 1]> = SmallVec::new();
    let mut base = px(0.);
    for (row_ix, w) in starts.windows(2).enumerate() {
        let row = &clusters[w[0]..w[1]];
        let mut levels: Vec<u8> = row.iter().map(|c| c.level).collect();
        for (l, c) in levels.iter_mut().zip(row).rev() {
            if c.ch.is_whitespace() {
                *l = para_level;
            } else {
                break;
            }
        }
        let mut order: Vec<usize> = (0..row.len()).collect();
        let highest = levels.iter().copied().max().unwrap_or(0);
        if let Some(lowest_odd) = levels.iter().copied().filter(|l| l % 2 == 1).min() {
            for level in (lowest_odd..=highest).rev() {
                let mut i = 0;
                while i < order.len() {
                    if levels[order[i]] >= level {
                        let mut j = i;
                        while j < order.len() && levels[order[j]] >= level {
                            j += 1;
                        }
                        order[i..j].reverse();
                        i = j;
                    } else {
                        i += 1;
                    }
                }
            }
        }

        let mut local = px(0.);
        let mut first = true;
        for &k in &order {
            let c = &row[k];
            for (font_id, g) in &c.glyphs {
                let mut g = g.clone();
                g.position.x = base + local + (g.position.x - c.x);
                if runs.last().is_none_or(|r| r.font_id != *font_id) {
                    runs.push(ShapedRun { font_id: *font_id, glyphs: Vec::new() });
                }
                let run_ix = runs.len() - 1;
                runs[run_ix].glyphs.push(g);
                if first && row_ix > 0 {
                    boundaries.push(WrapBoundary { run_ix, glyph_ix: runs[run_ix].glyphs.len() - 1 });
                }
                first = false;
            }
            local += c.width;
        }
        base += local;
    }

    Some((
        LineLayout {
            font_size: layout.font_size,
            width: base,
            ascent: layout.ascent,
            descent: layout.descent,
            runs,
            len: layout.len,
        },
        boundaries,
    ))
}
