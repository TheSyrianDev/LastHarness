//! A painted landscape made in memory, so a theme can have a sky, clouds, a sun and a meadow without any photo
//! in the repository. A theme describes it in TOML (`[scene]`); `render` turns that into pixels. No toolkit here.

use crate::theme::Color;

/// What the scene looks like. Every colour list runs top to bottom.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneSpec {
    /// Sky colours from the top of the window down to the horizon.
    pub sky: Vec<Color>,
    /// Meadow colours from the far edge down to the bottom of the window.
    pub ground: Vec<Color>,
    /// Distant trees and hills; empty means none.
    pub hills: Vec<Color>,
    /// Where the horizon sits, as a fraction of the height (0 top, 1 bottom).
    pub horizon: f32,
    /// The glow that hangs along the horizon.
    pub haze: Color,
    pub haze_strength: f32,
    /// Sun (or moon) position as fractions of width and height; `sun_size` is its glow radius as a fraction of the height.
    pub sun_x: f32,
    pub sun_y: f32,
    pub sun_size: f32,
    pub sun_color: Color,
    /// Faint translucent rings in a line away from the sun.
    pub flare: bool,
    pub clouds: u32,
    pub cloud_light: Color,
    pub cloud_shade: Color,
    pub stars: u32,
    pub flowers: u32,
    pub seed: u32,
}

impl Default for SceneSpec {
    fn default() -> Self {
        let c = |s: &str| Color::parse(s).unwrap();
        SceneSpec {
            sky: vec![c("#0a1fb8"), c("#0b57d6"), c("#1fb7f0"), c("#7fdcff"), c("#eafaff")],
            ground: vec![c("#b6e04a"), c("#7cc832"), c("#4aa626")],
            hills: vec![c("#74bf3a"), c("#4f9f2c")],
            horizon: 0.64,
            haze: c("#ffffff"),
            haze_strength: 0.6,
            sun_x: 0.14,
            sun_y: 0.25,
            sun_size: 0.2,
            sun_color: c("#fffbe0"),
            flare: true,
            clouds: 6,
            cloud_light: c("#ffffff"),
            cloud_shade: c("#b8d4f0"),
            stars: 0,
            flowers: 40,
            seed: 7,
        }
    }
}

struct Rng(u32);
impl Rng {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.0 >> 8) as f32 / (1u32 << 24) as f32
    }
    fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.next()
    }
}

type Rgb = [f32; 3];

fn rgb(c: Color) -> Rgb {
    [c.r as f32, c.g as f32, c.b as f32]
}

fn lerp(a: Rgb, b: Rgb, t: f32) -> Rgb {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

/// Blend `top` over `px` with opacity `a`.
fn over(px: &mut Rgb, top: Rgb, a: f32) {
    let a = a.clamp(0., 1.);
    *px = lerp(*px, top, a);
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0., 1.);
    t * t * (3. - 2. * t)
}

/// A colour along evenly spaced stops, `t` from 0 to 1.
fn ramp(stops: &[Color], t: f32) -> Rgb {
    match stops.len() {
        0 => [0.; 3],
        1 => rgb(stops[0]),
        n => {
            let x = t.clamp(0., 1.) * (n - 1) as f32;
            let i = (x.floor() as usize).min(n - 2);
            lerp(rgb(stops[i]), rgb(stops[i + 1]), smooth(x - i as f32))
        }
    }
}

/// Height of the far edge of the meadow at column `x` (0 to 1), as a fraction of the height above the horizon line.
fn hill_edge(x: f32) -> f32 {
    // a long rise on the right and a soft dip on the left, like the pictures this look comes from
    0.035 * smooth((x - 0.35) / 0.65) + 0.012 * (x * 9.0).sin() + 0.008 * (x * 23.0 + 1.0).sin()
}

/// Paint the scene as RGBA bytes, `w` by `h`, opaque.
pub fn render(spec: &SceneSpec, w: usize, h: usize) -> Vec<u8> {
    let (wf, hf) = (w as f32, h as f32);
    let mut rng = Rng(spec.seed.wrapping_mul(2654435761).wrapping_add(1));
    let horizon = spec.horizon.clamp(0.2, 0.9) * hf;
    let mut img: Vec<Rgb> = vec![[0.; 3]; w * h];

    // sky and meadow
    for y in 0..h {
        let yf = y as f32;
        for x in 0..w {
            let xf = x as f32 / wf;
            let edge = horizon - hill_edge(xf) * hf;
            let p = &mut img[y * w + x];
            if yf < edge {
                *p = ramp(&spec.sky, yf / horizon);
            } else {
                let t = (yf - edge) / (hf - edge).max(1.);
                // the meadow darkens a little toward the viewer and lightens where the sun falls
                *p = ramp(&spec.ground, t.powf(0.8));
            }
        }
    }

    // stars, for a night sky
    for _ in 0..spec.stars {
        let (sx, sy, br) = (rng.range(0., wf), rng.range(0., horizon * 0.8), rng.range(0.3, 1.0));
        let r = rng.range(0.5, 1.4);
        splat(&mut img, w, h, sx, sy, r * 2.5, [255., 255., 255.], br * 0.8, |d, r| (-(d / r) * (d / r) * 3.).exp());
    }

    // the glow along the horizon, over sky, trees and the far meadow
    {
        let hz = rgb(spec.haze);
        let sigma = hf * 0.085;
        for y in 0..h {
            let d = (y as f32 - horizon) / sigma;
            let g = (-d * d).exp() * spec.haze_strength;
            for x in 0..w {
                // stronger on the sun's side
                let side = 1.0 - 0.35 * ((x as f32 / wf) - spec.sun_x).abs();
                over(&mut img[y * w + x], hz, g * side);
            }
        }
    }

    // distant trees and hills: two rows of round tops, hazier the farther they are
    if !spec.hills.is_empty() {
        for (row, base) in [(0usize, horizon - hf * 0.012), (1, horizon + hf * 0.002)] {
            let n = (wf / 40.) as usize;
            for i in 0..n {
                let cx = (i as f32 + rng.range(0., 1.)) * wf / n as f32;
                let r = rng.range(14., 36.) * hf / 720. * if row == 0 { 1.2 } else { 1. };
                let cy = base - r * 0.45 - hill_edge(cx / wf) * hf * 0.3;
                let tone = ramp(&spec.hills, if row == 0 { 0.0 } else { 0.9 });
                let (x0, x1) = (((cx - r) as i32).max(0), ((cx + r) as i32).min(w as i32 - 1));
                let (y0, y1) = (((cy - r) as i32).max(0), ((cy + r * 0.6).min(base + hf * 0.01) as i32).min(h as i32 - 1));
                for y in y0..=y1 {
                    for x in x0..=x1 {
                        let (dx, dy) = (x as f32 - cx, (y as f32 - cy) * 1.15);
                        let d = (dx * dx + dy * dy).sqrt();
                        if d < r {
                            // lit from the sun's side, with a soft rim
                            let light = (0.5 - (dx * (spec.sun_x - 0.5).signum() + dy * 0.8) / (r * 2.)).clamp(0., 1.);
                            let col = lerp([tone[0] * 0.78, tone[1] * 0.82, tone[2] * 0.7], [tone[0] * 1.12, tone[1] * 1.1, tone[2] * 0.95], light);
                            over(&mut img[y as usize * w + x as usize], lerp(col, rgb(spec.haze), if row == 0 { 0.3 } else { 0.12 }), smooth((r - d) / 2.5));
                        }
                    }
                }
            }
        }
    }

    // the sun's glow
    {
        let (sx, sy) = (spec.sun_x * wf, spec.sun_y * hf);
        let r = spec.sun_size * hf;
        let col = rgb(spec.sun_color);
        let glow = |k: f32| move |d: f32, r: f32| ((-(d / r) * (d / r) * k).exp() - (-k).exp()).max(0.);
        splat(&mut img, w, h, sx, sy, r * 3.2, col, 0.4, glow(5.));
        splat(&mut img, w, h, sx, sy, r * 1.1, col, 0.8, glow(4.));
        splat(&mut img, w, h, sx, sy, r * 0.2, [255., 255., 255.], 1.0, glow(3.));
        // the star: long rays up, down, left and right, shorter on the diagonals
        for (ang, len, a) in [(0.0f32, 1.7, 0.75), (90., 1.7, 0.75), (45., 0.95, 0.5), (135., 0.95, 0.5)] {
            ray(&mut img, w, h, sx, sy, ang.to_radians(), r * len * 1.4, r * 0.05, [255., 255., 255.], a);
        }
    }

    // lens flare: faint discs and rings spaced along the line from the sun through the middle of the window
    if spec.flare {
        let (sx, sy) = (spec.sun_x * wf, spec.sun_y * hf);
        let (dx, dy) = (wf * 0.5 - sx, hf * 0.5 - sy);
        let tint = rgb(spec.sun_color);
        for (t, r, a, ring) in [(0.22f32, 0.011, 0.35, false), (0.31, 0.0075, 0.45, false), (0.52, 0.034, 0.14, true), (0.74, 0.016, 0.22, true), (0.93, 0.05, 0.1, true), (1.15, 0.022, 0.16, false)] {
            let (cx, cy, rr) = (sx + dx * t, sy + dy * t, r * hf);
            let reach = (rr + 4.) as i32;
            for y in ((cy as i32) - reach).max(0)..=((cy as i32) + reach).min(h as i32 - 1) {
                for x in ((cx as i32) - reach).max(0)..=((cx as i32) + reach).min(w as i32 - 1) {
                    let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
                    let fill = smooth((rr - d) / 1.5) * a * 0.55;
                    let edge = (-((d - rr) / 1.6).powi(2)).exp() * a * if ring { 1.2 } else { 0.7 };
                    over(&mut img[y as usize * w + x as usize], tint, fill + edge);
                }
            }
        }
    }

    clouds(&mut img, w, h, spec, horizon, &mut rng);

    // flowers in the near meadow: five white petals round a yellow centre
    for _ in 0..spec.flowers {
        let y = rng.range(horizon + (hf - horizon) * 0.45, hf * 0.985);
        let near = (y - horizon) / (hf - horizon);
        let size = (3.0 + near * 9.0) * hf / 720.;
        let x = rng.range(0., wf);
        let pink = rng.next() < 0.12;
        let petal = if pink { [236., 128., 210.] } else { [255., 255., 255.] };
        for k in 0..5 {
            let a = k as f32 * 1.2566 + rng.range(0., 0.4);
            splat(&mut img, w, h, x + a.cos() * size * 0.9, y + a.sin() * size * 0.55, size * 0.75, petal, 0.95, |d, r| smooth((1. - d / r) * 2.));
        }
        splat(&mut img, w, h, x, y, size * 0.45, [250., 200., 20.], 1.0, |d, r| smooth((1. - d / r) * 2.));
    }

    // a touch of vignette so the corners sit back
    for y in 0..h {
        for x in 0..w {
            let (nx, ny) = (x as f32 / wf - 0.5, y as f32 / hf - 0.5);
            let v = ((nx * nx + ny * ny) * 0.55).min(0.3);
            let p = &mut img[y * w + x];
            *p = [p[0] * (1. - v * 0.5), p[1] * (1. - v * 0.5), p[2] * (1. - v * 0.4)];
        }
    }

    let mut out = Vec::with_capacity(w * h * 4);
    for p in img {
        out.extend_from_slice(&[p[0].round().clamp(0., 255.) as u8, p[1].round().clamp(0., 255.) as u8, p[2].round().clamp(0., 255.) as u8, 255]);
    }
    out
}

#[allow(clippy::too_many_arguments)]
/// Add a round soft spot of `col` at (cx, cy) reaching `radius`, with the falloff `f(distance, radius)`.
fn splat(img: &mut [Rgb], w: usize, h: usize, cx: f32, cy: f32, radius: f32, col: Rgb, strength: f32, f: impl Fn(f32, f32) -> f32) {
    let reach = radius.ceil() as i32 + 1;
    for y in ((cy as i32) - reach).max(0)..=((cy as i32) + reach).min(h as i32 - 1) {
        for x in ((cx as i32) - reach).max(0)..=((cx as i32) + reach).min(w as i32 - 1) {
            let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
            if d <= radius {
                over(&mut img[y as usize * w + x as usize], col, f(d, radius) * strength);
            }
        }
    }
}

/// A thin tapering ray from the centre outward in both directions.
#[allow(clippy::too_many_arguments)]
fn ray(img: &mut [Rgb], w: usize, h: usize, cx: f32, cy: f32, ang: f32, len: f32, width: f32, col: Rgb, a: f32) {
    let (ca, sa) = (ang.cos(), ang.sin());
    let reach = len.ceil() as i32;
    for y in ((cy as i32) - reach).max(0)..=((cy as i32) + reach).min(h as i32 - 1) {
        for x in ((cx as i32) - reach).max(0)..=((cx as i32) + reach).min(w as i32 - 1) {
            let (dx, dy) = (x as f32 - cx, y as f32 - cy);
            let along = (dx * ca + dy * sa).abs();
            let across = (-dx * sa + dy * ca).abs();
            if along < len {
                let taper = (1. - along / len).powi(2);
                let g = (-(across / (width * (0.4 + 1.6 * taper) + 0.6)).powi(2)).exp() * taper;
                over(&mut img[y as usize * w + x as usize], col, g * a);
            }
        }
    }
}

/// Cumulus: clusters of overlapping soft discs with a flat base, lit from above and tinted blue underneath.
fn clouds(img: &mut [Rgb], w: usize, h: usize, spec: &SceneSpec, horizon: f32, rng: &mut Rng) {
    let (wf, hf) = (w as f32, h as f32);
    let mut dens = vec![0f32; w * h];
    let mut light = vec![0f32; w * h];
    let (lit, shade) = (rgb(spec.cloud_light), rgb(spec.cloud_shade));
    // big heaps stand on the horizon; the first ones sit at fixed places so the picture is balanced, the rest drift higher up
    let heaps = [(0.70f32, 0.21f32), (0.06, 0.15), (0.46, 0.1)];
    for n in 0..spec.clouds as usize {
        let (cx, scale, base, puffs, flat) = if n % 3 == 0 && n / 3 < heaps.len() {
            let (x, s) = heaps[n / 3];
            (x * wf, s * hf, horizon + hf * 0.01, 26, true)
        } else {
            // keep clear of the sun
            let mut x = rng.range(0.05, 0.95);
            for _ in 0..6 {
                if (x - spec.sun_x).abs() > 0.2 {
                    break;
                }
                x = rng.range(0.05, 0.95);
            }
            let s = rng.range(0.05, 0.1);
            let y = rng.range(0.1, 0.5) * horizon;
            (x * wf, s * hf, y + s * hf * 0.5, 12, true)
        };
        // stacked rows of puffs, each narrower than the one under it, so the heap rises to a rounded top
        let layers = if puffs > 20 { 5 } else { 2 };
        let mut discs: Vec<(f32, f32, f32)> = vec![];
        for l in 0..layers {
            let width = scale * 2.3 * (1. - 0.2 * l as f32);
            let n = (width / (scale * 0.34)).round().max(2.) as usize;
            for j in 0..n {
                let u = (j as f32 + 0.5 + rng.range(-0.3, 0.3)) / n as f32 * 2. - 1.;
                let r = scale * 0.4 * (1. - 0.1 * l as f32) * rng.range(0.8, 1.25) * (1. - 0.35 * u.abs());
                discs.push((cx + u * width * 0.5, base - r * 0.55 - l as f32 * scale * 0.3 - rng.range(0., 0.1) * scale, r));
            }
        }
        for (px, py, r) in discs {
            let reach = r.ceil() as i32;
            for y in ((py as i32) - reach).max(0)..=((py as i32) + reach).min(h as i32 - 1) {
                for x in ((px as i32) - reach).max(0)..=((px as i32) + reach).min(w as i32 - 1) {
                    let d = ((x as f32 - px).powi(2) + (y as f32 - py).powi(2)).sqrt() / r;
                    if d < 1. {
                        let mut k = (1. - d * d).powf(1.1);
                        if flat {
                            k *= smooth((base - y as f32) / (scale * 0.1 + 1.));
                        }
                        let i = y as usize * w + x as usize;
                        // brighter on the top of each puff
                        let top = ((py - y as f32) / r * 0.5 + 0.5).clamp(0., 1.);
                        dens[i] += k;
                        light[i] += k * top;
                    }
                }
            }
        }
    }
    for i in 0..w * h {
        if dens[i] > 0.002 {
            let a = smooth((dens[i] * 1.6).min(1.));
            let l = (light[i] / dens[i]).clamp(0., 1.);
            let col = lerp(shade, lit, smooth(l * 1.5 - 0.1));
            over(&mut img[i], col, a * 0.97);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scene_has_the_requested_size_and_is_opaque() {
        let px = render(&SceneSpec::default(), 160, 90);
        assert_eq!(px.len(), 160 * 90 * 4);
        assert!(px.chunks(4).all(|p| p[3] == 255));
    }

    #[test]
    fn the_sky_is_blue_on_top_and_the_meadow_green_at_the_bottom() {
        let (w, h) = (200, 120);
        let px = render(&SceneSpec::default(), w, h);
        let at = |x: usize, y: usize| &px[(y * w + x) * 4..(y * w + x) * 4 + 3];
        let top = at(w / 2, 2);
        assert!(top[2] > top[0] && top[2] > top[1], "top is blue: {top:?}");
        let bottom = at(w / 2, h - 3);
        assert!(bottom[1] > bottom[2] && bottom[1] > 100, "bottom is green: {bottom:?}");
        let near_horizon = at(w - 4, (h as f32 * 0.6) as usize);
        assert!(near_horizon[0] > top[0] && near_horizon[1] > top[1], "the horizon is lighter than the top");
    }

    #[test]
    fn the_same_spec_gives_the_same_picture() {
        let a = render(&SceneSpec::default(), 96, 54);
        let b = render(&SceneSpec::default(), 96, 54);
        assert_eq!(a, b);
        let c = render(&SceneSpec { seed: 99, ..SceneSpec::default() }, 96, 54);
        assert_ne!(a, c, "another seed moves the clouds");
    }

    #[test]
    fn stops_run_from_the_first_to_the_last_colour() {
        let stops = [Color::parse("#000000").unwrap(), Color::parse("#ffffff").unwrap()];
        assert_eq!(ramp(&stops, 0.), [0.; 3]);
        assert_eq!(ramp(&stops, 1.), [255.; 3]);
        assert_eq!(ramp(&[], 0.5), [0.; 3]);
    }
}
