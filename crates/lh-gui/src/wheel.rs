//! The Glass colour wheel's maths: hue runs around the ring, saturation grows from the centre out, and brightness
//! is kept per colour. No drawing here, only numbers and the wheel's pixels, so it can be tested without a window.

/// Read `#RRGGBB` or `RRGGBB` (spaces around are fine) into red, green and blue.
pub fn parse_hex(s: &str) -> Option<(u8, u8, u8)> {
    let s = s.trim();
    let s = s.strip_prefix('#').unwrap_or(s);
    if s.len() != 6 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let n = u32::from_str_radix(s, 16).ok()?;
    Some(((n >> 16) as u8, (n >> 8) as u8, n as u8))
}

/// `#RRGGBB` in capitals.
pub fn to_hex(r: u8, g: u8, b: u8) -> String {
    format!("#{r:02X}{g:02X}{b:02X}")
}

/// Hue in degrees (0..360), saturation and value in 0..1, to red, green and blue.
pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let h = h.rem_euclid(360.) / 60.;
    let (s, v) = (s.clamp(0., 1.), v.clamp(0., 1.));
    let c = v * s;
    let x = c * (1. - (h % 2. - 1.).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.),
        1 => (x, c, 0.),
        2 => (0., c, x),
        3 => (0., x, c),
        4 => (x, 0., c),
        _ => (c, 0., x),
    };
    let m = v - c;
    let q = |f: f32| ((f + m) * 255.).round().clamp(0., 255.) as u8;
    (q(r), q(g), q(b))
}

/// Red, green and blue to hue (degrees), saturation and value.
pub fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let (r, g, b) = (r as f32 / 255., g as f32 / 255., b as f32 / 255.);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d == 0. {
        0.
    } else if max == r {
        60. * ((g - b) / d).rem_euclid(6.)
    } else if max == g {
        60. * ((b - r) / d + 2.)
    } else {
        60. * ((r - g) / d + 4.)
    };
    (h, if max == 0. { 0. } else { d / max }, max)
}

/// Where a colour sits on a wheel of `radius`, as an offset from its centre (x right, y down). Hue 0 points right
/// and turns clockwise on screen; grey sits in the middle.
pub fn position(hex: &str, radius: f32) -> (f32, f32) {
    let (r, g, b) = parse_hex(hex).unwrap_or((128, 128, 128));
    let (h, s, _) = rgb_to_hsv(r, g, b);
    let a = h.to_radians();
    (a.cos() * s * radius, a.sin() * s * radius)
}

/// The colour under an offset from the centre, keeping `value` (brightness). Points outside the ring are pulled
/// onto its edge, so a drag that slips off the wheel still follows the pointer's direction.
pub fn colour_at(dx: f32, dy: f32, radius: f32, value: f32) -> String {
    let s = ((dx * dx + dy * dy).sqrt() / radius.max(1.)).min(1.);
    let h = dy.atan2(dx).to_degrees();
    let (r, g, b) = hsv_to_rgb(h, s, value);
    to_hex(r, g, b)
}

/// The brightness of a colour, for keeping it while the ball moves.
pub fn value_of(hex: &str) -> f32 {
    parse_hex(hex).map(|(r, g, b)| rgb_to_hsv(r, g, b).2).unwrap_or(1.)
}

/// The wheel's pixels, `size` square, RGBA, full brightness. The edge fades over one pixel so the circle is smooth.
pub fn disc_rgba(size: u32) -> Vec<u8> {
    let mut out = vec![0u8; (size * size * 4) as usize];
    let c = size as f32 / 2.;
    let radius = c - 0.5;
    for y in 0..size {
        for x in 0..size {
            let (dx, dy) = (x as f32 + 0.5 - c, y as f32 + 0.5 - c);
            let d = (dx * dx + dy * dy).sqrt();
            let alpha = (radius - d + 0.5).clamp(0., 1.);
            if alpha <= 0. {
                continue;
            }
            let (r, g, b) = hsv_to_rgb(dy.atan2(dx).to_degrees(), (d / radius).min(1.), 1.);
            let i = ((y * size + x) * 4) as usize;
            out[i..i + 4].copy_from_slice(&[r, g, b, (alpha * 255.).round() as u8]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_reads_with_or_without_the_hash() {
        assert_eq!(parse_hex("#FF8000"), Some((255, 128, 0)));
        assert_eq!(parse_hex("ff8000"), Some((255, 128, 0)));
        assert_eq!(parse_hex(" #00aaFF "), Some((0, 170, 255)));
        assert_eq!(parse_hex("#FFF"), None);
        assert_eq!(parse_hex("GG0000"), None);
        assert_eq!(parse_hex("#1234567"), None);
        assert_eq!(to_hex(255, 128, 0), "#FF8000");
    }

    #[test]
    fn hsv_round_trips() {
        for hex in ["#FF0000", "#00FF00", "#0000FF", "#3A7BD5", "#808080", "#FFFFFF", "#000000", "#C0FFEE"] {
            let (r, g, b) = parse_hex(hex).unwrap();
            let (h, s, v) = rgb_to_hsv(r, g, b);
            let (r2, g2, b2) = hsv_to_rgb(h, s, v);
            assert_eq!(to_hex(r2, g2, b2), hex);
        }
    }

    #[test]
    fn a_ball_lands_where_its_colour_is() {
        for hex in ["#FF0000", "#3A7BD5", "#C0FFEE", "#7F00FF"] {
            let (x, y) = position(hex, 80.);
            let back = colour_at(x, y, 80., value_of(hex));
            let (a, b) = (parse_hex(hex).unwrap(), parse_hex(&back).unwrap());
            assert!((a.0 as i32 - b.0 as i32).abs() <= 2 && (a.1 as i32 - b.1 as i32).abs() <= 2 && (a.2 as i32 - b.2 as i32).abs() <= 2, "{hex} came back {back}");
        }
        assert_eq!(position("#FF0000", 80.), (80., 0.));
        // off the wheel is pulled onto its edge
        assert_eq!(colour_at(500., 0., 80., 1.), "#FF0000");
        assert_eq!(colour_at(0., 0., 80., 1.), "#FFFFFF");
    }

    #[test]
    fn the_disc_is_round_and_smooth() {
        let n = 64;
        let px = disc_rgba(n);
        let alpha = |x: u32, y: u32| px[((y * n + x) * 4 + 3) as usize];
        assert_eq!(alpha(0, 0), 0);
        assert_eq!(alpha(32, 32), 255);
        // some edge pixels are partly covered: anti-aliased, not stepped
        let partial = (0..n).filter(|&x| (1..255).contains(&alpha(x, 5))).count();
        assert!(partial > 0);
    }
}
