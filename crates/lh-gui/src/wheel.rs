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

/// The lowest brightness the slider gives: below it hue and saturation would be lost in 8-bit colour, and the ball
/// would jump to the centre of the wheel.
pub const MIN_VALUE: f32 = 0.04;

/// The same colour at brightness `value`, keeping hue and saturation.
pub fn with_value(hex: &str, value: f32) -> String {
    let (r, g, b) = parse_hex(hex).unwrap_or((128, 128, 128));
    let (h, s, _) = rgb_to_hsv(r, g, b);
    let (r, g, b) = hsv_to_rgb(h, s, value.clamp(MIN_VALUE, 1.));
    to_hex(r, g, b)
}

/// The brightness at `y` pixels down a slider `height` tall: full at the top, darkest at the bottom.
pub fn value_at(y: f32, height: f32) -> f32 {
    (1. - y / height.max(1.)).clamp(MIN_VALUE, 1.)
}

/// Where the slider's knob sits for `value`, in pixels from the top.
pub fn value_y(value: f32, height: f32) -> f32 {
    (1. - value.clamp(0., 1.)) * height
}

/// Hue and saturation exactly as the user placed a ball, remembered next to the colour they made. Brightness is
/// saved in 8-bit colour, and reading hue and saturation back from a dark colour loses precision: each brightness
/// step would nudge the ball a little, so balls crept across the wheel while the slider moved. Keeping the placed
/// hue and saturation, and only reading them back from the colour when nothing is remembered, holds the ball still.
#[derive(Clone, Debug, Default)]
pub struct Placed(Vec<(String, f32, f32)>);

impl Placed {
    /// The hue and saturation for `hex`: the remembered ones if `hex` is the colour they made, otherwise read back.
    pub fn hs(&self, hex: &str) -> (f32, f32) {
        let key = hex.trim().to_ascii_uppercase();
        if let Some((_, h, s)) = self.0.iter().find(|(k, _, _)| *k == key) {
            return (*h, *s);
        }
        let (r, g, b) = parse_hex(hex).unwrap_or((128, 128, 128));
        let (h, s, _) = rgb_to_hsv(r, g, b);
        (h, s)
    }

    /// Remember that `hex` was made from hue `h` and saturation `s`.
    pub fn keep(&mut self, hex: &str, h: f32, s: f32) {
        let key = hex.trim().to_ascii_uppercase();
        self.0.retain(|(k, _, _)| *k != key);
        self.0.push((key, h, s));
        if self.0.len() > 64 {
            self.0.remove(0);
        }
    }

    /// Where the ball for `hex` sits on a wheel of `radius`, using the remembered hue and saturation.
    pub fn position(&self, hex: &str, radius: f32) -> (f32, f32) {
        let (h, s) = self.hs(hex);
        let a = h.to_radians();
        (a.cos() * s * radius, a.sin() * s * radius)
    }

    /// The colour under an offset from the centre at brightness `value`, remembering where it was placed.
    pub fn place(&mut self, dx: f32, dy: f32, radius: f32, value: f32) -> String {
        let s = ((dx * dx + dy * dy).sqrt() / radius.max(1.)).min(1.);
        let h = dy.atan2(dx).to_degrees().rem_euclid(360.);
        let hex = colour_at(dx, dy, radius, value);
        self.keep(&hex, h, s);
        hex
    }

    /// `hex` at brightness `value`, keeping its placed hue and saturation (not the ones read back from 8-bit colour).
    pub fn with_value(&mut self, hex: &str, value: f32) -> String {
        let (h, s) = self.hs(hex);
        let (r, g, b) = hsv_to_rgb(h, s, value.clamp(MIN_VALUE, 1.));
        let out = to_hex(r, g, b);
        self.keep(&out, h, s);
        out
    }
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
    fn brightness_is_kept_per_colour() {
        let dim = with_value("#FF0000", 0.5);
        assert_eq!(dim, "#800000");
        assert!((value_of(&dim) - 0.5).abs() < 0.01);
        // hue and saturation stay, so the ball does not move on the wheel
        let (x, y) = position("#3A7BD5", 80.);
        let (x2, y2) = position(&with_value("#3A7BD5", 0.3), 80.);
        assert!((x - x2).abs() < 2. && (y - y2).abs() < 2.);
        // never all the way to black, which would lose the hue
        assert_eq!(with_value("#00FF00", 0.), with_value("#00FF00", MIN_VALUE));
        assert_ne!(position(&with_value("#00FF00", 0.), 80.), (0., 0.));
    }

    #[test]
    fn balls_stay_put_while_the_brightness_moves() {
        // regression: dragging the brightness slider used to re-read hue and saturation from the dark 8-bit colour
        // on every step, and the ball crept across the wheel
        let mut placed = Placed::default();
        for &(dx, dy) in &[(61.3_f32, -22.7_f32), (-10.2, 70.9), (3.1, 2.4), (-80., -5.5)] {
            let start = placed.place(dx, dy, 88., 1.);
            let at = placed.position(&start, 88.);
            let mut c = start.clone();
            for round in 0..5 {
                for i in (0..=100).rev().chain(0..=100) {
                    c = placed.with_value(&c, i as f32 / 100.);
                    let (x, y) = placed.position(&c, 88.);
                    assert!((x - at.0).abs() < 1e-3 && (y - at.1).abs() < 1e-3, "ball moved at {i} in round {round}");
                }
            }
            assert_eq!(c, start, "full brightness gives the same colour back");
        }
    }

    #[test]
    fn the_old_way_drifted() {
        // the bug, kept as a check that the test above would catch it
        let mut c = colour_at(61.3, -22.7, 88., 1.);
        let at = position(&c, 88.);
        for i in (4..=100).rev().chain(4..=100) {
            c = with_value(&c, i as f32 / 100.);
        }
        let now = position(&c, 88.);
        assert!((now.0 - at.0).abs() + (now.1 - at.1).abs() > 0.5);
    }

    #[test]
    fn the_slider_runs_bright_to_dark() {
        assert_eq!(value_at(0., 160.), 1.);
        assert_eq!(value_at(160., 160.), MIN_VALUE);
        assert_eq!(value_at(-20., 160.), 1.);
        assert!((value_at(80., 160.) - 0.5).abs() < 1e-6);
        assert_eq!(value_y(1., 160.), 0.);
        assert_eq!(value_y(0.25, 160.), 120.);
        for v in [0.1_f32, 0.5, 0.9] {
            assert!((value_at(value_y(v, 160.), 160.) - v).abs() < 1e-6);
        }
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
