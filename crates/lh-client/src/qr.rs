//! QR codes for the phone sign-in link, drawn as dark/light modules so each front end can paint them.

/// The modules of a QR code for `text`, row by row (`true` is dark), with the 4-module quiet zone.
pub fn modules(text: &str) -> Option<Vec<Vec<bool>>> {
    let code = qrcode::QrCode::with_error_correction_level(text.as_bytes(), qrcode::EcLevel::L).ok()?;
    let w = code.width();
    let colors = code.to_colors();
    let q = 4;
    let n = w + 2 * q;
    Some((0..n).map(|y| (0..n).map(|x| x >= q && y >= q && x < w + q && y < w + q && colors[(y - q) * w + (x - q)] == qrcode::Color::Dark).collect()).collect())
}

/// The QR code as terminal text, two rows per line with half blocks. Light text on a dark
/// terminal would invert it, so it paints dark modules as spaces on a light (`█`) background.
pub fn text(text: &str) -> Option<String> {
    let m = modules(text)?;
    let mut out = String::new();
    for pair in m.chunks(2) {
        for x in 0..pair[0].len() {
            let top = !pair[0][x];
            let bottom = pair.get(1).map(|r| !r[x]).unwrap_or(true);
            out.push(match (top, bottom) {
                (true, true) => '█',
                (true, false) => '▀',
                (false, true) => '▄',
                (false, false) => ' ',
            });
        }
        out.push('\n');
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_link_makes_a_square_code_with_finder_corners() {
        let m = super::modules("https://100.64.0.1:7843/#login=0123456789abcdef").unwrap();
        assert!(m.len() > 25 && m.iter().all(|r| r.len() == m.len()));
        assert!(m[4][4] && m[4][10] && !m[0][0], "finder pattern after the quiet zone");
        let t = super::text("x").unwrap();
        assert_eq!(t.lines().count(), (super::modules("x").unwrap().len() + 1) / 2);
    }
}
