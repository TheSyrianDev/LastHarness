//! The "Omarchy" look: it takes the colours of the desktop's current Omarchy theme
//! (`~/.local/state/omarchy/current/theme/colors.toml`) and follows `omarchy theme set` while the app runs. Off
//! Omarchy the file is missing and the look is simply not offered. Nothing is written into Omarchy's folders.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::theme::{contrast, Color, Mode, Theme};

/// Where Omarchy keeps the theme in use: `~/.local/state/omarchy/current`. `None` without a home folder.
pub fn current_dir() -> Option<PathBuf> {
    // built without the `omarchy` feature: the Omarchy looks, the Hyprland reads and the window-rule hint are never offered
    if !cfg!(feature = "omarchy") {
        return None;
    }
    let state = std::env::var_os("XDG_STATE_HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))?;
    Some(state.join("omarchy").join("current"))
}

/// What tells the app the Omarchy theme changed: the name in `theme.name` and the time `colors.toml` was written. Checked on
/// the app's 400 ms theme tick; two small reads, no watcher thread.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Stamp {
    name: String,
    colors: Option<SystemTime>,
}

/// What the look takes from Hyprland: window rounding, window opacity and the blur behind windows. Read live with
/// `hyprctl -j --batch "getoption ..."` (works the same whether Hyprland is set up in hyprland.conf or in lua).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hypr {
    /// `decoration:rounding`, pixels.
    pub rounding: f32,
    /// `decoration:active_opacity`, 0 to 1.
    pub active_opacity: f32,
    /// `decoration:inactive_opacity`, 0 to 1.
    pub inactive_opacity: f32,
    /// `decoration:blur:size`.
    pub blur_size: f32,
    /// `decoration:blur:passes`.
    pub blur_passes: f32,
    /// The opacity a window rule gives this app's own window (`hyprctl getprop pid:<ours> opacity`), and whether
    /// the rule says `override` (the rule's value replaces `active_opacity` instead of multiplying it). `None` when
    /// Hyprland has no window of ours yet or cannot say.
    pub window_opacity: Option<(f32, bool)>,
}

impl Default for Hypr {
    /// Off Hyprland (or when hyprctl cannot answer): the look's own roundness, opaque, a gentle blur.
    fn default() -> Self {
        Hypr { rounding: 10., active_opacity: 1., inactive_opacity: 1., blur_size: 4., blur_passes: 2., window_opacity: None }
    }
}

pub const HYPR_OPTIONS: [&str; 5] = ["decoration:rounding", "decoration:active_opacity", "decoration:inactive_opacity", "decoration:blur:size", "decoration:blur:passes"];

impl Hypr {
    /// Ask the running Hyprland. `None` when hyprctl is missing, Hyprland is not running, or nothing was readable.
    pub fn read() -> Option<Hypr> {
        if !cfg!(feature = "omarchy") {
            return None;
        }
        std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE")?;
        // the window's own opacity last: when Hyprland has no window of ours it answers in plain text, which ends the read
        let pid = std::process::id();
        let mut batch = HYPR_OPTIONS.iter().map(|o| format!("getoption {o}")).collect::<Vec<_>>();
        batch.push(format!("getprop pid:{pid} opacity"));
        batch.push(format!("getprop pid:{pid} opacity_override"));
        let batch = batch.join("; ");
        let out = std::process::Command::new("hyprctl").args(["-j", "--batch", &batch]).stdin(std::process::Stdio::null()).stderr(std::process::Stdio::null()).output().ok()?;
        if !out.status.success() {
            return None;
        }
        Hypr::parse(&String::from_utf8_lossy(&out.stdout))
    }

    /// Read hyprctl's answer: one JSON object per option, such as `{"option": "decoration:rounding", "int": 24,
    /// "set": true}`, separated by blank lines. Options it does not know ("no such option") keep the default.
    pub fn parse(out: &str) -> Option<Hypr> {
        let mut h = Hypr::default();
        let mut found = false;
        for v in serde_json::Deserializer::from_str(out).into_iter::<serde_json::Value>() {
            let Ok(v) = v else { break };
            // the window rule's answers (`getprop`): plain text instead when there is no window of ours, ending the read
            if let Some(o) = v.get("opacity").and_then(|n| n.as_f64()).filter(|n| n.is_finite()) {
                h.window_opacity = Some((o.clamp(0., 1.) as f32, h.window_opacity.is_some_and(|w| w.1)));
                found = true;
                continue;
            }
            if let Some(b) = v.get("opacity_override").and_then(|b| b.as_bool()) {
                if let Some(w) = h.window_opacity.as_mut() {
                    w.1 = b;
                }
                continue;
            }
            let Some(name) = v.get("option").and_then(|o| o.as_str()) else { continue };
            let Some(n) = v.get("int").or(v.get("float")).and_then(|n| n.as_f64()) else { continue };
            let n = n as f32;
            if !n.is_finite() {
                continue;
            }
            match name {
                "decoration:rounding" => h.rounding = n.clamp(0., 64.),
                "decoration:active_opacity" => h.active_opacity = n.clamp(0., 1.),
                "decoration:inactive_opacity" => h.inactive_opacity = n.clamp(0., 1.),
                "decoration:blur:size" => h.blur_size = n.max(0.),
                "decoration:blur:passes" => h.blur_passes = n.max(0.),
                _ => continue,
            }
            found = true;
        }
        found.then_some(h)
    }

    /// The glass blur in pixels: Hyprland's blur spreads about `size` per pass, so the product, halved to suit
    /// small panels, kept within the material's 0 to 24.
    pub fn glass_blur(&self) -> f32 {
        (self.blur_size * self.blur_passes / 2.).clamp(0., 24.)
    }

    /// The window background's opacity: the active window opacity.
    pub fn background_alpha(&self) -> f32 {
        self.active_opacity.clamp(0.3, 1.)
    }

    /// How see-through the transparent Omarchy look is: what the window rule for this app gives, times
    /// `active_opacity` unless the rule says `override`, as Hyprland itself works it out. No window rule (or no
    /// window yet): 1, opaque.
    pub fn rule_alpha(&self) -> f32 {
        match self.window_opacity {
            Some((o, true)) => o,
            Some((o, false)) => o * self.active_opacity,
            None => 1.,
        }
        .clamp(0.3, 1.)
    }

    /// Does a window rule make this app see-through? False when there is none, so Settings can show the line to add.
    pub fn has_see_through_rule(&self) -> bool {
        self.window_opacity.is_some_and(|(o, _)| o < 0.98)
    }

    /// Panel opacity: always see-through enough for the glass to show, a little more so when the desktop
    /// itself makes windows see-through.
    pub fn surface_alpha(&self) -> f32 {
        surface_alpha_for(self.active_opacity)
    }
}

fn surface_alpha_for(window: f32) -> f32 {
    (0.62 * window).clamp(0.35, 0.62)
}

/// This app's window class on Wayland (the `app_id`), for the window rule that makes it see-through.
pub const APP_CLASS: &str = "lastharness-gui";

/// The name of the see-through variant of the Omarchy look.
pub const TRANSPARENT: &str = "Omarchy (transparent)";

/// The line to add to Hyprland's config for the transparent look, shown in Settings when no rule matches.
/// In Omarchy's Lua config (`~/.config/hypr/hyprland.lua`); the number is how see-through the window is.
pub const RULE_HINT: &str = "o.window(\"lastharness-gui\", { opacity = \"0.85 override 0.85 override\" })";

/// The Omarchy theme's foreground colour (the text in its bar and panels), from `colors.toml`. `None` when there is
/// no theme, or it has no readable `foreground` key.
pub fn foreground(current: &Path) -> Option<Color> {
    let src = std::fs::read_to_string(current.join("theme").join("colors.toml")).ok()?;
    let table: toml::Table = toml::from_str(&src).ok()?;
    Color::parse(table.get("foreground")?.as_str()?).ok()
}

pub fn stamp(current: &Path) -> Stamp {
    Stamp {
        name: std::fs::read_to_string(current.join("theme.name")).unwrap_or_default().trim().to_string(),
        colors: std::fs::metadata(current.join("theme").join("colors.toml")).and_then(|m| m.modified()).ok(),
    }
}

/// The Omarchy look from the current theme folder, or `None` when there is no readable `colors.toml`.
pub fn load(current: &Path, hypr: Option<Hypr>) -> Option<Theme> {
    let src = std::fs::read_to_string(current.join("theme").join("colors.toml")).ok()?;
    from_colors_with(&src, hypr).ok()
}

/// The transparent Omarchy look from the current theme folder, as [`load`].
pub fn load_transparent(current: &Path, hypr: Option<Hypr>) -> Option<Theme> {
    let src = std::fs::read_to_string(current.join("theme").join("colors.toml")).ok()?;
    transparent_from_colors(&src, hypr).ok()
}

/// The see-through variant: Omarchy's colours, the window as see-through as Hyprland's window rule for this app
/// makes it ([`Hypr::rule_alpha`]; opaque without a rule), Hyprland's rounding and blur, and the liquid glass
/// material of the built-in Glass look in the same mode.
pub fn transparent_from_colors(src: &str, hypr: Option<Hypr>) -> Result<Theme, String> {
    let h = hypr.unwrap_or_default();
    let mut t = build(src, &h, TRANSPARENT, h.rule_alpha())?;
    let glass = glass_material(t.mode);
    t.material = crate::theme::Material { blur: h.glass_blur(), ..glass };
    Ok(t)
}

/// The `[material]` of the built-in Glass look in that mode.
pub fn glass_material(mode: Mode) -> crate::theme::Material {
    let src = if mode == Mode::Light { include_str!("../themes/glass-light.toml") } else { include_str!("../themes/glass-dark.toml") };
    Theme::parse(src, Path::new("")).expect("the built-in Glass look parses").material
}

/// Turn an Omarchy `colors.toml` into a look. Keys that are missing are made from the ones that are there; only
/// `background` and `foreground` are needed. Pairs that would be hard to read are nudged until they pass the same
/// contrast rules as the built-in looks (text 4.5:1, marks 3:1).
pub fn from_colors(src: &str) -> Result<Theme, String> {
    from_colors_with(src, None)
}

/// As [`from_colors`], with the shape, opacity and blur taken from Hyprland (`None`: the defaults). The look is
/// liquid glass throughout, like the Glass look.
pub fn from_colors_with(src: &str, hypr: Option<Hypr>) -> Result<Theme, String> {
    let h = hypr.unwrap_or_default();
    build(src, &h, "Omarchy", h.background_alpha())
}

/// The look from `colors.toml`, named `name`, with the window background covered `window` (0 to 1).
fn build(src: &str, h: &Hypr, name: &str, window: f32) -> Result<Theme, String> {
    let table: toml::Table = toml::from_str(src).map_err(|e| e.message().to_string())?;
    let mut keys: HashMap<String, Color> = HashMap::new();
    for (k, v) in &table {
        if let Some(c) = v.as_str().and_then(|s| Color::parse(s).ok()) {
            keys.insert(k.clone(), Color { a: 255, ..c });
        }
    }
    let get = |k: &str| keys.get(k).copied();
    let bg = get("background").ok_or("colors.toml has no background")?;
    let fg = get("foreground").ok_or("colors.toml has no foreground")?;
    let mode = match table.get("mode").and_then(|v| v.as_str()) {
        Some("light") => Mode::Light,
        Some("dark") => Mode::Dark,
        _ if luminance(bg) > 0.4 => Mode::Light,
        _ => Mode::Dark,
    };
    let dark = mode == Mode::Dark;

    let surface = get("lighter_background").unwrap_or_else(|| bg.mix(fg, 0.06));
    let sunken = get("dark_background").unwrap_or_else(|| bg.mix(if dark { BLACK } else { fg }, if dark { 0.35 } else { 0.04 }));
    let button = surface.mix(fg, 0.08);
    let button_hover = surface.mix(fg, 0.15);
    let border = bg.mix(fg, 0.16);
    let backs = [bg, surface, sunken, button];

    let text = readable(fg, &backs, 4.5);
    let muted = readable(get("dark_foreground").unwrap_or_else(|| fg.mix(bg, 0.4)), &backs, 4.5);
    let accent = readable(get("accent").or(get("blue")).unwrap_or(fg), &[surface, bg], 3.0);
    // the accent's own text: the background or the brightest foreground, whichever reads better, nudged if needed
    let light = get("bright_foreground").unwrap_or(fg);
    let accent_text = readable(if contrast(bg, accent) >= contrast(light, accent) { bg } else { light }, &[accent], 4.5);
    let danger = readable(get("red").unwrap_or(Color { r: 0xe0, g: 0x5a, b: 0x5a, a: 255 }).mix(fg, 0.1), &[surface, bg], 4.5);
    let warn = readable(get("yellow").or(get("orange")).unwrap_or(Color { r: 0xe0, g: 0xb2, b: 0x5c, a: 255 }).mix(fg, 0.1), &[surface, bg], 4.5);
    let good = readable(get("green").unwrap_or(Color { r: 0x6f, g: 0xcf, b: 0x8a, a: 255 }).mix(fg, 0.1), &[surface, bg], 4.5);
    let focus_ring = readable(accent, &[bg, surface], 3.0);

    let hex = |c: Color| format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b);
    let alpha = |a: f32| format!("{:02x}", (a.clamp(0., 1.) * 255.).round() as u8);
    let (bg_a, surface_a) = (alpha(window), alpha(surface_alpha_for(window)));
    let see_through = window < 1.;
    let file = format!(
        "name = \"{name}\"\nmode = \"{}\"\n[colors]\nbackground = \"{}{bg_a}\"\nsurface = \"{}{surface_a}\"\nsunken = \"{}\"\ntext = \"{}\"\nmuted = \"{}\"\naccent = \"{}\"\naccent_text = \"{}\"\nbutton = \"{}\"\nbutton_hover = \"{}\"\nborder = \"{}\"\ndanger = \"{}\"\nwarn = \"{}\"\ngood = \"{}\"\nfocus_ring = \"{}\"\n[shape]\nroundness = {}\n[effects]\nblur = {see_through}\n[material]\npanels = \"regular\"\nside_panel = \"clear\"\npopups = \"regular_tinted\"\ncontrols = \"clear\"\nblur = {}\nrefraction = 8\nrim = 0.45\n[animation]\nduration_ms = 140\n",
        if dark { "dark" } else { "light" },
        hex(bg), hex(surface), hex(sunken), hex(text), hex(muted), hex(accent), hex(accent_text), hex(button), hex(button_hover), hex(border), hex(danger), hex(warn), hex(good), hex(focus_ring),
        h.rounding.round(), h.glass_blur(),
    );
    let mut t = Theme::parse(&file, Path::new(""))?;
    // the brand mark per tokens.json markColours.omarchy: face = foreground, eye = background, slot = accent
    let rgb = |c: Color| (c.r as u32) << 16 | (c.g as u32) << 8 | c.b as u32;
    t.mark = Some(crate::brand::Colours { face: rgb(text), eye: rgb(bg), slot: rgb(accent) });
    // chart series from the theme's named hues, each nudged to stand 3:1 off the surface; the accent first
    let mut series = vec![accent];
    for k in ["blue", "green", "yellow", "magenta", "cyan", "orange", "red"] {
        if let Some(c) = get(k) {
            let c = readable(c, &[surface], 3.0);
            if !series.contains(&c) {
                series.push(c);
            }
        }
    }
    t.series = if series.len() > 1 { series } else { vec![] };
    t.selection = get("selection");
    Ok(t)
}

const BLACK: Color = Color { r: 0, g: 0, b: 0, a: 255 };
const WHITE: Color = Color { r: 255, g: 255, b: 255, a: 255 };

fn luminance(c: Color) -> f64 {
    crate::theme::luminance(c)
}

/// `fg`, moved step by step toward white (on dark backs) or black (on light ones) until it reaches `min` against
/// every colour in `backs`. A pair that already passes is left alone.
pub fn readable(fg: Color, backs: &[Color], min: f64) -> Color {
    let worst = |c: Color| backs.iter().map(|b| contrast(c, *b)).fold(f64::MAX, f64::min);
    if worst(fg) >= min {
        return fg;
    }
    let avg = backs.iter().map(|b| luminance(*b)).sum::<f64>() / backs.len().max(1) as f64;
    let target = if avg < 0.18 { WHITE } else { BLACK };
    let mut best = fg;
    for step in 1..=20 {
        let c = fg.mix(target, step as f32 * 0.05);
        best = c;
        if worst(c) >= min {
            break;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    const DARK: &str = include_str!("../tests/fixtures/omarchy-dark.toml");
    const LIGHT: &str = include_str!("../tests/fixtures/omarchy-light.toml");

    fn readable_everywhere(t: &Theme) {
        let c = &t.colors;
        for (what, fg, back, min) in [
            ("text on surface", c.text, c.surface, 4.5),
            ("text on background", c.text, c.background, 4.5),
            ("text on input", c.text, c.sunken, 4.5),
            ("text on button", c.text, c.button, 4.5),
            ("muted on surface", c.muted, c.surface, 4.5),
            ("muted on background", c.muted, c.background, 4.5),
            ("muted on input", c.muted, c.sunken, 4.5),
            ("accent text on accent", c.accent_text, c.accent, 4.5),
            ("danger on surface", c.danger, c.surface, 4.5),
            ("warn on surface", c.warn, c.surface, 4.5),
            ("good on surface", c.good, c.surface, 4.5),
            ("accent on surface", c.accent, c.surface, 3.0),
            ("focus ring on background", c.focus_ring, c.background, 3.0),
        ] {
            let r = contrast(fg, back);
            assert!(r >= min, "{}: {what} is {r:.2}, needs {min}", t.label());
        }
        for s in &t.series {
            assert!(contrast(*s, c.surface) >= 3.0);
        }
    }

    #[test]
    fn a_dark_omarchy_theme_maps_onto_the_tokens() {
        let t = from_colors(DARK).unwrap();
        assert_eq!(t.label(), "Omarchy dark");
        assert_eq!(t.colors.background, Color::parse("#0c1202").unwrap());
        assert_eq!(t.colors.surface, Color::parse("#161e089e").unwrap(), "the panel colour, see-through so the glass shows");
        assert_eq!(t.colors.sunken, Color::parse("#070b01").unwrap());
        assert_eq!(t.colors.text, Color::parse("#e1e7d9").unwrap());
        assert_eq!(t.colors.accent, Color::parse("#a3c75d").unwrap());
        let m = t.mark.unwrap();
        assert_eq!((m.face, m.eye, m.slot), (0xe1e7d9, 0x0c1202, 0xa3c75d));
        assert!(t.series.len() >= 6 && t.series[0] == t.colors.accent);
        assert_eq!(t.selection, Some(Color::parse("#2b390b").unwrap()));
        readable_everywhere(&t);
    }

    #[test]
    fn a_light_omarchy_theme_maps_and_stays_readable() {
        let t = from_colors(LIGHT).unwrap();
        assert_eq!(t.label(), "Omarchy light");
        readable_everywhere(&t);
    }

    #[test]
    fn the_foreground_is_read_from_the_theme_folder() {
        let dir = std::env::temp_dir().join(format!("lh-omarchy-fg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(foreground(&dir), None, "no theme folder: no foreground");
        std::fs::create_dir_all(dir.join("theme")).unwrap();
        std::fs::write(dir.join("theme/colors.toml"), DARK).unwrap();
        assert_eq!(foreground(&dir), Some(Color::parse("#e1e7d9").unwrap()));
        std::fs::write(dir.join("theme/colors.toml"), "background = \"#000000\"\n").unwrap();
        assert_eq!(foreground(&dir), None, "no foreground key: none");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_keys_are_made_from_the_ones_there() {
        let t = from_colors("background = \"#1a1b26\"\nforeground = \"#c0caf5\"\n").unwrap();
        assert_eq!(t.mode, Mode::Dark, "no mode key: the background decides");
        assert!(t.series.is_empty(), "no named hues: the app's own chart colours");
        readable_everywhere(&t);
        let l = from_colors("background = \"#fafafa\"\nforeground = \"#333333\"\n").unwrap();
        assert_eq!(l.mode, Mode::Light);
        readable_everywhere(&l);
        assert!(from_colors("accent = \"#ff0000\"\n").is_err(), "no background or foreground: no look");
        assert!(from_colors("not toml [").is_err());
    }

    #[test]
    fn a_hard_to_read_pair_is_nudged_not_refused() {
        // the accent is nearly the background and the muted text is far too dim
        let src = "mode = \"dark\"\nbackground = \"#101010\"\nforeground = \"#d0d0d0\"\naccent = \"#181820\"\ndark_foreground = \"#202020\"\nred = \"#300000\"\nyellow = \"#202000\"\n";
        let t = from_colors(src).unwrap();
        assert_ne!(t.colors.accent, Color::parse("#181820").unwrap(), "the accent moved");
        readable_everywhere(&t);
        assert_eq!(readable(WHITE, &[BLACK], 4.5), WHITE, "a pair that passes is left alone");
    }

    #[test]
    fn a_theme_change_shows_in_the_stamp_and_a_missing_folder_gives_no_look() {
        let dir = std::env::temp_dir().join(format!("lh-omarchy-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(load(&dir, None).is_none());
        let empty = stamp(&dir);
        std::fs::create_dir_all(dir.join("theme")).unwrap();
        std::fs::write(dir.join("theme.name"), "one\n").unwrap();
        std::fs::write(dir.join("theme/colors.toml"), DARK).unwrap();
        let a = stamp(&dir);
        assert_ne!(a, empty);
        assert_eq!(load(&dir, None).unwrap().mode, Mode::Dark);
        std::fs::write(dir.join("theme.name"), "two\n").unwrap();
        std::fs::write(dir.join("theme/colors.toml"), LIGHT).unwrap();
        let b = stamp(&dir);
        assert_ne!(a, b, "a new theme name changes the stamp");
        assert_eq!(load(&dir, None).unwrap().mode, Mode::Light);
        assert_eq!(stamp(&dir), b, "nothing changed, same stamp");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn hyprctl_batch_output_is_read_and_unknown_options_keep_the_default() {
        let out = "{\"option\": \"decoration:rounding\", \"int\": 24, \"set\": true }\n\n\n{\"option\": \"decoration:active_opacity\", \"float\": 0.900000, \"set\": true }\n\n\n{\"option\": \"decoration:inactive_opacity\", \"float\": 0.800000, \"set\": true }\n\n\n{\"option\": \"decoration:blur:size\", \"int\": 8, \"set\": true }\n\n\n{\"option\": \"decoration:blur:passes\", \"int\": 3, \"set\": true }\n\n";
        let h = Hypr::parse(out).unwrap();
        assert_eq!(h, Hypr { rounding: 24., active_opacity: 0.9, inactive_opacity: 0.8, blur_size: 8., blur_passes: 3., window_opacity: None });
        // an older Hyprland that lacks one option answers "no such option" in plain text, which ends the read
        let part = Hypr::parse("{\"option\": \"decoration:rounding\", \"int\": 6, \"set\": true }\nno such option").unwrap();
        assert_eq!((part.rounding, part.active_opacity), (6., 1.));
        assert_eq!(Hypr::parse(""), None, "no hyprctl output: no Hyprland values");
        assert_eq!(Hypr::parse("no such option"), None);
        assert_eq!(Hypr::parse("{\"option\": \"decoration:rounding\", \"int\": 500}").unwrap().rounding, 64., "kept in range");
    }

    #[test]
    fn hyprland_sets_the_roundness_opacity_and_glass_blur_and_the_look_is_glass() {
        let h = Hypr { rounding: 24., active_opacity: 0.9, inactive_opacity: 0.8, blur_size: 8., blur_passes: 3., window_opacity: None };
        let t = from_colors_with(DARK, Some(h)).unwrap();
        assert_eq!(t.roundness, 24.);
        assert_eq!(t.material.blur, 12., "size 8 times 3 passes, halved");
        assert!(t.colors.background.a < 255 && t.blur, "a see-through desktop: a see-through window, blurred behind");
        assert!(t.colors.surface.a < 200, "panels see-through enough for the glass");
        use crate::theme::GlassKind as K;
        assert_ne!(t.material.panels, K::None);
        assert_ne!(t.material.popups, K::None);
        assert_ne!(t.material.side_panel, K::None);
        assert_ne!(t.material.controls, K::None);
        // without Hyprland: the defaults, opaque window, still glass
        let f = from_colors(DARK).unwrap();
        assert_eq!((f.roundness, f.colors.background.a, f.blur), (10., 255, false));
        assert_ne!(f.material.panels, K::None);
        assert_eq!(f.material.blur, Hypr::default().glass_blur());
    }

    #[test]
    fn the_window_rule_for_this_app_is_read_from_hyprctl() {
        let opts = "{\"option\": \"decoration:active_opacity\", \"float\": 0.900000, \"set\": true }\n\n";
        let h = Hypr::parse(&format!("{opts}{{\"opacity\": 0.75}}\n\n{{\"opacity_override\": true}}\n")).unwrap();
        assert_eq!(h.window_opacity, Some((0.75, true)));
        assert_eq!(h.rule_alpha(), 0.75, "override: the rule's own value");
        assert!(h.has_see_through_rule());
        let h = Hypr::parse(&format!("{opts}{{\"opacity\": 0.8}}\n\n{{\"opacity_override\": false}}\n")).unwrap();
        assert!((h.rule_alpha() - 0.72).abs() < 1e-5, "no override: times active_opacity, as Hyprland does");
        // no window of ours yet: plain text, the rest of the read still counts and the look is opaque
        let h = Hypr::parse(&format!("{opts}window not found\n\nwindow not found")).unwrap();
        assert_eq!((h.window_opacity, h.rule_alpha(), h.has_see_through_rule()), (None, 1., false));
        // a rule that keeps it opaque is no see-through rule
        let h = Hypr::parse("{\"opacity\": 1}\n{\"opacity_override\": true}").unwrap();
        assert!(!h.has_see_through_rule());
        assert_eq!(Hypr::parse("{\"opacity\": 0.1}").unwrap().rule_alpha(), 0.3, "never so faint the words go");
    }

    #[test]
    fn the_transparent_look_follows_the_rule_and_takes_the_glass_material() {
        let h = Hypr { window_opacity: Some((0.75, true)), blur_size: 8., blur_passes: 3., rounding: 14., ..Hypr::default() };
        let t = transparent_from_colors(DARK, Some(h)).unwrap();
        assert_eq!(t.name, TRANSPARENT);
        assert_eq!(t.colors.background.a, (0.75f32 * 255.).round() as u8, "the window rule's level");
        assert!(t.blur, "see-through: blurred behind");
        assert_eq!(t.roundness, 14.);
        let g = glass_material(Mode::Dark);
        assert_eq!((t.material.panels, t.material.side_panel, t.material.popups, t.material.controls), (g.panels, g.side_panel, g.popups, g.controls));
        assert_eq!((t.material.refraction, t.material.rim), (g.refraction, g.rim), "Glass's bend and rim");
        assert_eq!(t.material.blur, 12., "blur still from Hyprland");
        readable_everywhere(&t);
        // the colours are Omarchy's, not Glass's
        let o = from_colors_with(DARK, Some(h)).unwrap();
        assert_eq!((t.colors.text, t.colors.accent), (o.colors.text, o.colors.accent));
        // no rule: opaque, no blur behind
        let f = transparent_from_colors(DARK, Some(Hypr::default())).unwrap();
        assert_eq!((f.colors.background.a, f.blur), (255, false));
        // a light Omarchy theme takes the light Glass material
        let l = transparent_from_colors(include_str!("../tests/fixtures/omarchy-light.toml"), None).unwrap();
        assert_eq!(l.mode, Mode::Light);
        assert_eq!(l.material.panels, glass_material(Mode::Light).panels);
    }
}
