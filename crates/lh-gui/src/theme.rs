//! Themes: a TOML file sets colours, background, roundness, blur, shadows and animation. Eight come with
//! the app (Calm, Matte, Glass and Frutiger Aero, each light and dark); users add their own in a folder and
//! changes are picked up while the app runs. This module knows nothing about the window toolkit.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::Deserialize;

/// A colour with opacity, parsed from `#rgb`, `#rrggbb` or `#rrggbbaa`.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn parse(s: &str) -> Result<Color, String> {
        let h = s.trim().strip_prefix('#').ok_or_else(|| format!("\"{s}\" is not a colour: it must start with #"))?;
        if !h.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("\"{s}\" is not a colour: only 0-9 and a-f are allowed after the #"));
        }
        let nib = |c: char| c.to_digit(16).unwrap() as u8;
        let b: Vec<u8> = h.chars().map(nib).collect();
        Ok(match b.len() {
            3 => Color { r: b[0] * 17, g: b[1] * 17, b: b[2] * 17, a: 255 },
            6 => Color { r: b[0] * 16 + b[1], g: b[2] * 16 + b[3], b: b[4] * 16 + b[5], a: 255 },
            8 => Color { r: b[0] * 16 + b[1], g: b[2] * 16 + b[3], b: b[4] * 16 + b[5], a: b[6] * 16 + b[7] },
            _ => return Err(format!("\"{s}\" is not a colour: use 3, 6 or 8 digits after the #")),
        })
    }

    pub fn is_translucent(&self) -> bool {
        self.a < 255
    }

    /// Linear mix toward `to`; `t` of 0 gives `self`, 1 gives `to`.
    pub fn mix(self, to: Color, t: f32) -> Color {
        let m = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t.clamp(0., 1.)).round() as u8;
        Color { r: m(self.r, to.r), g: m(self.g, to.g), b: m(self.b, to.b), a: m(self.a, to.a) }
    }

    pub fn with_alpha(self, a: f32) -> Color {
        Color { a: (a.clamp(0., 1.) * 255.).round() as u8, ..self }
    }

    /// 0xRRGGBBAA, the form the toolkit takes.
    pub fn rgba(self) -> u32 {
        (self.r as u32) << 24 | (self.g as u32) << 16 | (self.b as u32) << 8 | self.a as u32
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackgroundKind {
    #[default]
    Solid,
    Gradient,
    Image,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
struct RawColors {
    background: String,
    surface: String,
    sunken: String,
    text: String,
    muted: String,
    accent: String,
    accent_text: String,
    button: String,
    button_hover: String,
    border: String,
    danger: String,
}
impl Default for RawColors {
    fn default() -> Self {
        // Calm light, so a theme file only has to say what it changes
        let d = |s: &str| s.to_string();
        Self { background: d("#f5f5f3"), surface: d("#ffffff"), sunken: d("#ecece9"), text: d("#1d1d1b"), muted: d("#6d6d68"), accent: d("#3b6fd4"), accent_text: d("#ffffff"), button: d("#e8e8e4"), button_hover: d("#dcdcd6"), border: d("#dadad5"), danger: d("#c0392b") }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
struct RawBackground {
    /// `solid`, `gradient` or `image`. Read as text so an old file that says `scene` gets a notice, not an error.
    kind: String,
    from: Option<String>,
    to: Option<String>,
    angle: f32,
    image: Option<String>,
    image_opacity: f32,
}
impl Default for RawBackground {
    fn default() -> Self {
        Self { kind: "solid".into(), from: None, to: None, angle: 0., image: None, image_opacity: 1. }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
struct RawShape {
    roundness: f32,
    match_window: bool,
}
impl Default for RawShape {
    fn default() -> Self {
        Self { roundness: 10., match_window: false }
    }
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(default, deny_unknown_fields)]
struct RawEffects {
    blur: bool,
    shadow_blur: f32,
    shadow_y: f32,
    shadow_color: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
struct RawAnimation {
    duration_ms: u64,
}
impl Default for RawAnimation {
    fn default() -> Self {
        Self { duration_ms: 140 }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
struct RawGlass {
    top: String,
    bottom: String,
    rim: String,
    edge: String,
    sheen: f32,
    /// Opacity of the fine diagonal hairlines on the glass; 0 for none.
    lines: f32,
    /// Distance between the hairlines, in pixels.
    lines_gap: f32,
    /// Opacity of the diagonal reflection in the top left corner of a pane; 0 for none.
    reflection: f32,
}
impl Default for RawGlass {
    fn default() -> Self {
        let d = |s: &str| s.to_string();
        Self { top: d("#ffffffd9"), bottom: d("#ffffff8c"), rim: d("#ffffffe6"), edge: d("#7fb8dd99"), sheen: 0.3, lines: 0., lines_gap: 6., reflection: 0. }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
struct RawGel {
    top: Option<String>,
    bottom: Option<String>,
    primary_top: Option<String>,
    primary_bottom: Option<String>,
    gloss: f32,
    rim: String,
    edge: String,
    glow: String,
    press: f32,
}
impl Default for RawGel {
    fn default() -> Self {
        let d = |s: &str| s.to_string();
        Self { top: None, bottom: None, primary_top: None, primary_bottom: None, gloss: 0.5, rim: d("#ffffffcc"), edge: d("#3d86b8"), glow: d("#46d4ffcc"), press: 0.18 }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
struct RawFocus {
    from: String,
    to: String,
    width: f32,
}
impl Default for RawFocus {
    fn default() -> Self {
        Self { from: "#1174a1".into(), to: "#6b60c4".into(), width: 2. }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
struct RawMotes {
    count: u32,
    color: String,
}
impl Default for RawMotes {
    fn default() -> Self {
        Self { count: 0, color: "#ffffff".into() }
    }
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(default, deny_unknown_fields)]
struct Raw {
    name: String,
    mode: Mode,
    colors: RawColors,
    background: RawBackground,
    shape: RawShape,
    effects: RawEffects,
    animation: RawAnimation,
    /// The painted-landscape table of older theme files. It is read and ignored.
    scene: Option<toml::Value>,
    glass: Option<RawGlass>,
    gel: Option<RawGel>,
    focus: Option<RawFocus>,
    motes: Option<RawMotes>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Colors {
    pub background: Color,
    pub surface: Color,
    pub sunken: Color,
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub accent_text: Color,
    pub button: Color,
    pub button_hover: Color,
    pub border: Color,
    pub danger: Color,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Background {
    pub kind: BackgroundKind,
    pub from: Color,
    pub to: Color,
    pub angle: f32,
    /// An absolute path, resolved against the theme file's folder.
    pub image: Option<PathBuf>,
    pub image_opacity: f32,
}

/// Glass panels: a translucent fill that is lighter at the top, a light line just inside the edge and a pale
/// line outside it. A theme without a `[glass]` table has plain flat panels.
#[derive(Clone, Debug, PartialEq)]
pub struct Glass {
    pub top: Color,
    pub bottom: Color,
    pub rim: Color,
    pub edge: Color,
    /// Opacity of the white sheen over the top part of a panel.
    pub sheen: f32,
    /// Opacity of the fine diagonal hairlines, the spacing between them in pixels, and the opacity of the diagonal
    /// reflection in the top left corner. Zero opacity draws nothing.
    pub lines: f32,
    pub lines_gap: f32,
    pub reflection: f32,
}

/// Gel buttons: a gradient with a glossy top half, a rim light and a glow when hovered.
#[derive(Clone, Debug, PartialEq)]
pub struct Gel {
    pub top: Color,
    pub bottom: Color,
    pub primary_top: Color,
    pub primary_bottom: Color,
    pub gloss: f32,
    pub rim: Color,
    pub edge: Color,
    pub glow: Color,
    /// How much a pressed button darkens, 0 to 1.
    pub press: f32,
}

/// The border of the focused panel: a gradient from one colour to another, corner to corner.
#[derive(Clone, Debug, PartialEq)]
pub struct Focus {
    pub from: Color,
    pub to: Color,
    pub width: f32,
}

/// Soft bubbles drifting up the window.
#[derive(Clone, Debug, PartialEq)]
pub struct Motes {
    pub count: u32,
    pub color: Color,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub name: String,
    pub mode: Mode,
    pub colors: Colors,
    pub background: Background,
    pub roundness: f32,
    pub match_window: bool,
    pub blur: bool,
    pub shadow_blur: f32,
    pub shadow_y: f32,
    pub shadow_color: Color,
    pub animation_ms: u64,
    pub glass: Option<Glass>,
    pub gel: Option<Gel>,
    pub focus: Option<Focus>,
    pub motes: Option<Motes>,
    /// Something the person should know about this file (an old setting that no longer exists), shown in Settings.
    pub notice: Option<String>,
}

fn key(name: &str, v: &str) -> Result<Color, String> {
    Color::parse(v).map_err(|e| format!("{name}: {e}"))
}

fn col(key: &str, v: &str) -> Result<Color, String> {
    Color::parse(v).map_err(|e| format!("colors.{key}: {e}"))
}

impl Theme {
    /// Read a theme from TOML. `dir` is where relative image paths start.
    pub fn parse(src: &str, dir: &Path) -> Result<Theme, String> {
        let raw: Raw = toml::from_str(src).map_err(|e| e.message().to_string())?;
        if raw.name.trim().is_empty() {
            return Err("the theme needs a name".into());
        }
        let c = &raw.colors;
        let colors = Colors {
            background: col("background", &c.background)?,
            surface: col("surface", &c.surface)?,
            sunken: col("sunken", &c.sunken)?,
            text: col("text", &c.text)?,
            muted: col("muted", &c.muted)?,
            accent: col("accent", &c.accent)?,
            accent_text: col("accent_text", &c.accent_text)?,
            button: col("button", &c.button)?,
            button_hover: col("button_hover", &c.button_hover)?,
            border: col("border", &c.border)?,
            danger: col("danger", &c.danger)?,
        };
        let b = &raw.background;
        let bg = |k: &str, v: &Option<String>| -> Result<Color, String> {
            match v {
                Some(s) => Color::parse(s).map_err(|e| format!("background.{k}: {e}")),
                None => Ok(colors.background),
            }
        };
        let mut notice = None;
        let kind = match b.kind.trim().to_lowercase().as_str() {
            "solid" => BackgroundKind::Solid,
            "gradient" => BackgroundKind::Gradient,
            "image" => BackgroundKind::Image,
            // the painted landscape of earlier versions is gone: the file still loads, with a plain gradient
            "scene" => {
                notice = Some(format!("{}: the painted background (kind = \"scene\") no longer exists, so a plain gradient is used", raw.name.trim()));
                BackgroundKind::Gradient
            }
            other => return Err(format!("background.kind must be solid, gradient or image, not \"{other}\"")),
        };
        let from = bg("from", &b.from)?;
        let mut to = bg("to", &b.to)?;
        let mut angle = b.angle;
        if notice.is_some() && b.to.is_none() {
            // a gradient the person never wrote: from the theme's own background toward its text colour
            to = from.mix(colors.text, 0.22);
            angle = 180.;
        }
        if kind == BackgroundKind::Image && b.image.is_none() {
            return Err("background.kind is \"image\" but background.image is not set".into());
        }
        if !(0.0..=1.0).contains(&b.image_opacity) {
            return Err("background.image_opacity must be between 0 and 1".into());
        }
        if !(0.0..=64.0).contains(&raw.shape.roundness) {
            return Err("shape.roundness must be between 0 and 64".into());
        }
        let shadow_color = match &raw.effects.shadow_color {
            Some(s) => Color::parse(s).map_err(|e| format!("effects.shadow_color: {e}"))?,
            None => Color { r: 0, g: 0, b: 0, a: 0 },
        };
        let glass = match &raw.glass {
            Some(g) => Some(Glass { top: key("glass.top", &g.top)?, bottom: key("glass.bottom", &g.bottom)?, rim: key("glass.rim", &g.rim)?, edge: key("glass.edge", &g.edge)?, sheen: g.sheen.clamp(0., 1.), lines: g.lines.clamp(0., 1.), lines_gap: g.lines_gap.clamp(3., 24.), reflection: g.reflection.clamp(0., 1.) }),
            None => None,
        };
        let gel = match &raw.gel {
            Some(g) => {
                let opt = |k: &str, v: &Option<String>, d: Color| -> Result<Color, String> { v.as_ref().map_or(Ok(d), |s| key(k, s)) };
                let white = Color { r: 255, g: 255, b: 255, a: 255 };
                let black = Color { r: 0, g: 0, b: 0, a: 255 };
                Some(Gel {
                    top: opt("gel.top", &g.top, colors.button.mix(white, 0.45))?,
                    bottom: opt("gel.bottom", &g.bottom, colors.button.mix(black, 0.06))?,
                    primary_top: opt("gel.primary_top", &g.primary_top, colors.accent.mix(white, 0.2))?,
                    primary_bottom: opt("gel.primary_bottom", &g.primary_bottom, colors.accent.mix(black, 0.1))?,
                    gloss: g.gloss.clamp(0., 1.),
                    rim: key("gel.rim", &g.rim)?,
                    edge: key("gel.edge", &g.edge)?,
                    glow: key("gel.glow", &g.glow)?,
                    press: g.press.clamp(0., 1.),
                })
            }
            None => None,
        };
        let focus = match &raw.focus {
            Some(f) => Some(Focus { from: key("focus.from", &f.from)?, to: key("focus.to", &f.to)?, width: f.width.clamp(0.5, 8.) }),
            None => None,
        };
        let motes = match &raw.motes {
            Some(m) => Some(Motes { count: m.count.min(40), color: key("motes.color", &m.color)? }),
            None => None,
        };
        Ok(Theme {
            name: raw.name.trim().to_string(),
            mode: raw.mode,
            colors,
            background: Background { kind, from, to, angle, image: b.image.as_ref().map(|i| dir.join(i)), image_opacity: b.image_opacity },
            roundness: raw.shape.roundness,
            match_window: raw.shape.match_window,
            blur: raw.effects.blur,
            shadow_blur: raw.effects.shadow_blur.max(0.),
            shadow_y: raw.effects.shadow_y,
            shadow_color,
            animation_ms: raw.animation.duration_ms,
            glass,
            gel,
            focus,
            motes,
            notice,
        })
    }

    /// The unique label of a theme in lists: "Calm light".
    pub fn label(&self) -> String {
        format!("{} {}", self.name, if self.mode == Mode::Dark { "dark" } else { "light" })
    }

    /// Does anything in the look let the desktop show through? Then the window must be transparent.
    pub fn is_translucent(&self) -> bool {
        let c = &self.colors;
        self.background_is_translucent() || c.surface.is_translucent() || self.blur || self.glass.is_some()
    }

    fn background_is_translucent(&self) -> bool {
        match self.background.kind {
            BackgroundKind::Solid => self.colors.background.is_translucent(),
            BackgroundKind::Gradient => self.background.from.is_translucent() || self.background.to.is_translucent(),
            BackgroundKind::Image => self.background.image_opacity < 1.0 || self.colors.background.is_translucent(),
        }
    }

    /// Radius of a control that sits inside a panel with `padding`: the panel's radius minus the padding,
    /// never less than 3, so the two curves stay parallel.
    pub fn inner_radius(outer: f32, padding: f32) -> f32 {
        (outer - padding).max(3.)
    }
}

const BUILTIN: &[(&str, &str)] = &[
    ("calm-light", include_str!("../themes/calm-light.toml")),
    ("calm-dark", include_str!("../themes/calm-dark.toml")),
    ("matte-light", include_str!("../themes/matte-light.toml")),
    ("matte-dark", include_str!("../themes/matte-dark.toml")),
    ("glass-light", include_str!("../themes/glass-light.toml")),
    ("glass-dark", include_str!("../themes/glass-dark.toml")),
    ("frutiger-aero-light", include_str!("../themes/frutiger-aero-light.toml")),
    ("frutiger-aero-dark", include_str!("../themes/frutiger-aero-dark.toml")),
];

/// The themes that ship with the app. Calm light is first, and is what a fresh install uses.
pub fn builtin() -> Vec<Theme> {
    BUILTIN.iter().map(|(n, s)| Theme::parse(s, Path::new("")).unwrap_or_else(|e| panic!("built-in theme {n}: {e}"))).collect()
}

/// Built-in themes plus the user's own, with a note for each file that could not be read.
pub struct ThemeSet {
    pub themes: Vec<Theme>,
    pub problems: Vec<String>,
    /// One line for the person when theme files use something that no longer exists (the painted background); `None`
    /// when all is well. Shown once in Settings.
    pub notice: Option<String>,
    dir: PathBuf,
    stamp: Option<SystemTime>,
    count: usize,
}

impl ThemeSet {
    pub fn load(dir: &Path) -> ThemeSet {
        let mut themes = builtin();
        let mut problems = vec![];
        let mut old_background: Vec<String> = vec![];
        let mut stamp = None;
        let mut count = 0;
        let mut files: Vec<PathBuf> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "toml")).collect();
        files.sort();
        for p in files {
            count += 1;
            let m = std::fs::metadata(&p).and_then(|m| m.modified()).ok();
            if m > stamp {
                stamp = m;
            }
            let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            match std::fs::read_to_string(&p).map_err(|e| e.to_string()).and_then(|s| Theme::parse(&s, p.parent().unwrap_or(dir))) {
                Ok(t) => {
                    if t.notice.is_some() && !old_background.contains(&t.name) {
                        old_background.push(t.name.clone());
                    }
                    // a user's theme with the same label replaces the built-in one
                    if let Some(i) = themes.iter().position(|x| x.label() == t.label()) {
                        themes[i] = t;
                    } else {
                        themes.push(t);
                    }
                }
                Err(e) => problems.push(format!("{name}: {e}")),
            }
        }
        let notice = (!old_background.is_empty()).then(|| format!("{} still ask{} for the painted background, which no longer exists. A plain gradient is used instead. Set kind to solid, gradient or image in the file to change it.", old_background.join(", "), if old_background.len() == 1 { "s" } else { "" }));
        ThemeSet { themes, problems, notice, dir: dir.to_path_buf(), stamp, count }
    }

    /// Has any file in the folder changed since the last load?
    pub fn changed(&self) -> bool {
        let mut newest = None;
        let mut count = 0;
        for e in std::fs::read_dir(&self.dir).into_iter().flatten().flatten() {
            if e.path().extension().is_some_and(|x| x == "toml") {
                count += 1;
                let m = e.metadata().and_then(|m| m.modified()).ok();
                if m > newest {
                    newest = m;
                }
            }
        }
        newest != self.stamp || count != self.count
    }

    pub fn reload(&mut self) {
        *self = ThemeSet::load(&self.dir.clone());
    }

    /// The families of looks, once each, in the order they first appear: "Calm", "Matte", "Glass", "Frutiger Aero",
    /// then the person's own.
    pub fn families(&self) -> Vec<String> {
        let mut v: Vec<String> = vec![];
        for t in &self.themes {
            if !v.contains(&t.name) {
                v.push(t.name.clone());
            }
        }
        v
    }

    /// The look of a family in the wanted mode. A family with only one mode (a theme file the person wrote for
    /// one) gives that one for both, so the switch leaves it as it is. `None` for a family that does not exist.
    pub fn variant(&self, family: &str, dark: bool) -> Option<usize> {
        let want = if dark { Mode::Dark } else { Mode::Light };
        self.themes.iter().position(|t| t.name == family && t.mode == want).or_else(|| self.themes.iter().position(|t| t.name == family))
    }

    /// Does the family have both a light and a dark look?
    pub fn has_both(&self, family: &str) -> bool {
        [Mode::Light, Mode::Dark].iter().all(|m| self.themes.iter().any(|t| t.name == family && t.mode == *m))
    }

    /// The look to use for a saved family and mode; a family that is gone (a deleted file) gives the first look.
    pub fn resolve(&self, family: &str, dark: bool) -> usize {
        self.variant(family, dark).unwrap_or_else(|| self.variant(&self.themes[0].name, dark).unwrap_or(0))
    }

    /// Where a saved theme label sits in the list; a label that is gone (a deleted file) gives the first look.
    pub fn index_of(&self, label: &str) -> usize {
        self.themes.iter().position(|t| t.label() == label).unwrap_or(0)
    }

    pub fn find(&self, label: &str) -> Option<&Theme> {
        self.themes.iter().find(|t| t.label() == label)
    }
}

/// The user's theme folder: `$XDG_CONFIG_HOME/lastharness/themes` or `~/.config/lastharness/themes`.
pub fn user_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config"))).unwrap_or_default();
    base.join("lastharness").join("themes")
}

/// The window's own corner radius where the desktop reports it. Hyprland does (`decoration:rounding`).
pub fn window_radius() -> Option<f32> {
    let o = std::process::Command::new("hyprctl").args(["getoption", "decoration:rounding", "-j"]).output().ok()?;
    parse_hypr_rounding(&String::from_utf8_lossy(&o.stdout))
}

pub fn parse_hypr_rounding(json: &str) -> Option<f32> {
    json.split("\"int\":").nth(1)?.trim().split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()
}

impl Theme {
    /// The corner radius of panels: the window's radius if the theme asks for that and the desktop says it.
    pub fn panel_radius(&self, window: Option<f32>) -> f32 {
        if self.match_window { window.unwrap_or(self.roundness) } else { self.roundness }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_parse_in_three_forms_and_say_what_is_wrong() {
        assert_eq!(Color::parse("#fff").unwrap(), Color { r: 255, g: 255, b: 255, a: 255 });
        assert_eq!(Color::parse("#10203040").unwrap(), Color { r: 16, g: 32, b: 48, a: 64 });
        assert_eq!(Color::parse("#ff0000").unwrap().rgba(), 0xff0000ff);
        assert!(Color::parse("red").unwrap_err().contains("must start with #"));
        assert!(Color::parse("#12345").unwrap_err().contains("3, 6 or 8 digits"));
        assert!(Color::parse("#12g").unwrap_err().contains("0-9 and a-f"));
    }

    #[test]
    fn all_eight_built_in_themes_load_and_have_unique_labels() {
        let t = builtin();
        assert_eq!(t.len(), 8);
        let mut labels: Vec<String> = t.iter().map(|t| t.label()).collect();
        labels.sort();
        labels.dedup();
        assert_eq!(labels.len(), 8);
        assert_eq!(t[0].label(), "Calm light", "the plain default comes first");
        for name in ["Calm", "Matte", "Glass", "Frutiger Aero"] {
            for mode in ["light", "dark"] {
                assert!(t.iter().any(|x| x.label() == format!("{name} {mode}")), "{name} {mode}");
            }
        }
    }

    #[test]
    fn four_families_each_with_both_modes_and_the_mode_picks_the_variant() {
        let set = ThemeSet::load(Path::new("/no/such/folder"));
        assert_eq!(set.families(), ["Calm", "Matte", "Glass", "Frutiger Aero"]);
        for f in set.families() {
            assert!(set.has_both(&f), "{f}");
            let (l, d) = (set.variant(&f, false).unwrap(), set.variant(&f, true).unwrap());
            assert_eq!((set.themes[l].mode, set.themes[d].mode), (Mode::Light, Mode::Dark), "{f}");
            assert_eq!((set.themes[l].name.as_str(), set.themes[d].name.as_str()), (f.as_str(), f.as_str()));
        }
        assert_eq!(set.themes[set.resolve("Glass", true)].label(), "Glass dark");
        assert_eq!(set.themes[set.resolve("Nonesuch", false)].label(), "Calm light", "a family that is gone gives the first look");
        assert_eq!(set.themes[set.resolve("Nonesuch", true)].label(), "Calm dark", "in the mode asked for");
    }

    #[test]
    fn a_theme_with_one_mode_stays_as_it_is_when_the_mode_switches() {
        let dir = std::env::temp_dir().join(format!("lh-gui-onemode-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("mine.toml"), "name = \"Mine\"\nmode = \"dark\"\n[colors]\naccent = \"#ff0066\"\n").unwrap();
        let set = ThemeSet::load(&dir);
        assert_eq!(set.families().last().unwrap(), "Mine");
        assert!(!set.has_both("Mine"));
        let (l, d) = (set.resolve("Mine", false), set.resolve("Mine", true));
        assert_eq!(l, d, "light asked for, dark is all there is, so it stays");
        assert_eq!(set.themes[l].label(), "Mine dark");
        // adding the other mode later (live reload) makes the switch work for it
        std::fs::write(dir.join("mine-light.toml"), "name = \"Mine\"\nmode = \"light\"\n").unwrap();
        let set = ThemeSet::load(&dir);
        assert!(set.has_both("Mine"));
        assert_eq!(set.themes[set.resolve("Mine", false)].label(), "Mine light");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_looks_differ_the_way_they_should() {
        let t = builtin();
        let by = |l: &str| t.iter().find(|x| x.label() == l).unwrap();
        assert!(!by("Calm light").is_translucent() && !by("Calm light").blur);
        assert!(!by("Matte dark").is_translucent());
        assert!(by("Glass dark").is_translucent() && by("Glass dark").blur);
        assert!(by("Glass light").is_translucent());
        assert_eq!(by("Frutiger Aero light").background.kind, BackgroundKind::Gradient);
        assert!(by("Frutiger Aero light").is_translucent() && by("Frutiger Aero dark").is_translucent());
        assert!(by("Glass light").roundness > by("Calm light").roundness, "glass is rounder than calm");
    }

    #[test]
    fn frutiger_aero_is_windows_7_style_glass_in_both_modes_with_no_picture() {
        let t = builtin();
        for label in ["Frutiger Aero light", "Frutiger Aero dark"] {
            let x = t.iter().find(|x| x.label() == label).unwrap_or_else(|| panic!("{label} is built in"));
            assert_eq!(x.background.kind, BackgroundKind::Gradient, "{label}: a tint over the desktop, not a picture");
            assert!(x.background.image.is_none(), "{label} ships no wallpaper");
            assert!(x.background.from.is_translucent() && x.background.to.is_translucent(), "{label}: the tint lets the desktop through");
            assert!(x.is_translucent() && x.blur, "{label} is drawn over the compositor's blur");
            assert!(!x.colors.background.is_translucent(), "{label} has a solid colour to sit on when nothing is translucent");
            assert!(x.glass.is_some() && x.gel.is_some() && x.focus.is_some(), "{label}");
            assert!(x.motes.is_none(), "{label} has no bubbles: they belonged to the nature look");
            assert!(x.roundness >= 6. && x.roundness <= 12., "{label} has the small corners of Windows 7 windows, not 16");
            let g = x.glass.as_ref().unwrap();
            assert!(g.reflection > 0. && g.lines > 0. && g.lines_gap >= 3., "{label} has the diagonal reflection and the hairlines");
            assert!(g.rim.a >= 40 && g.rim.r > 200 && g.top.a < 255 && g.bottom.a < 255, "{label}: a light rim inside, translucent fill");
            assert!(g.top.a >= g.bottom.a || x.mode == Mode::Dark, "{label}: lighter at the top");
            assert_eq!(x.name, "Frutiger Aero");
            // the blue glow on hover is blue, in both modes
            let glow = x.gel.as_ref().unwrap().glow;
            assert!(glow.b > glow.r && glow.b > 200, "{label}: a blue glow, not {glow:?}");
            assert!(x.focus.as_ref().unwrap().to.b > 200, "{label}: the focus border is blue");
        }
        let light = t.iter().find(|x| x.label() == "Frutiger Aero light").unwrap();
        let dark = t.iter().find(|x| x.label() == "Frutiger Aero dark").unwrap();
        assert!(light.colors.text.r < 40 && dark.colors.text.r > 215, "dark text on light glass, light text on dark glass");
        assert!(dark.glass.as_ref().unwrap().top.r < light.glass.as_ref().unwrap().top.r, "the dark glass is darker");
        // the other looks stay flat
        let calm = t.iter().find(|x| x.label() == "Calm light").unwrap();
        assert!(calm.glass.is_none() && calm.gel.is_none() && calm.focus.is_none() && calm.motes.is_none());
        let plain = t.iter().find(|x| x.label() == "Glass light").unwrap();
        let pg = plain.glass.as_ref();
        assert!(pg.is_none() || (pg.unwrap().lines == 0. && pg.unwrap().reflection == 0.), "Glass does not get the Aero marks");
    }

    #[test]
    fn glass_gel_focus_and_motes_are_checked_with_the_key_that_is_wrong() {
        let ok = Theme::parse("name = \"x\"\n[glass]\nlines = 0.1\nlines_gap = 5\nreflection = 0.3\n[gel]\n[focus]\n[motes]\ncount = 4\n", Path::new("")).unwrap();
        assert!(ok.glass.is_some() && ok.gel.is_some() && ok.focus.is_some());
        assert_eq!(ok.motes.unwrap().count, 4);
        let e = |src: &str| Theme::parse(&format!("name = \"x\"\n{src}"), Path::new("")).unwrap_err();
        assert!(e("[glass]\ntop = \"x\"").contains("glass.top"));
        assert!(e("[gel]\nglow = \"x\"").contains("gel.glow"));
        assert!(e("[focus]\nfrom = \"x\"").contains("focus.from"));
        assert!(e("[motes]\ncolor = \"x\"").contains("motes.color"));
        assert!(e("[glass]\nshine = 1").contains("unknown field"));
        assert!(e("[background]\nkind = \"wallpaper\"").contains("solid, gradient or image"));
        let capped = Theme::parse("name = \"x\"\n[motes]\ncount = 9999\n", Path::new("")).unwrap();
        assert_eq!(capped.motes.unwrap().count, 40, "bubbles are capped");
        let g = Theme::parse("name = \"x\"\n[glass]\nlines = 7\nlines_gap = 1\nreflection = -2\n", Path::new("")).unwrap().glass.unwrap();
        assert_eq!((g.lines, g.lines_gap, g.reflection), (1., 3., 0.), "the marks are kept inside their limits");
        let plain = Theme::parse("name = \"x\"\n[glass]\n", Path::new("")).unwrap().glass.unwrap();
        assert_eq!((plain.lines, plain.reflection), (0., 0.), "a glass table without them has none, as before");
    }

    #[test]
    fn an_old_theme_that_asks_for_the_painted_scene_loads_with_a_plain_gradient_and_a_notice() {
        let old = "name = \"Mine\"\nmode = \"dark\"\n[colors]\nbackground = \"#102030\"\ntext = \"#e0e0e0\"\n[background]\nkind = \"scene\"\n[scene]\nsky = [\"#000\", \"#fff\"]\nclouds = 3\nnothing_like_a_real_key = true\n[glass]\n[gel]\n";
        let t = Theme::parse(old, Path::new("")).expect("an old file must still load");
        assert_eq!(t.background.kind, BackgroundKind::Gradient);
        assert_eq!(t.background.from, Color::parse("#102030").unwrap(), "from the theme's own background colour");
        assert_ne!(t.background.to, t.background.from, "a visible gradient, not a flat fill");
        assert!(t.glass.is_some() && t.gel.is_some(), "the rest of the file still counts");
        assert!(t.notice.as_deref().is_some_and(|n| n.contains("Mine") && n.contains("scene")), "{:?}", t.notice);
        // a gradient the person wrote themselves is kept
        let own = Theme::parse("name = \"Own\"\n[background]\nkind = \"scene\"\nfrom = \"#112233\"\nto = \"#445566\"\n", Path::new("")).unwrap();
        assert_eq!((own.background.from, own.background.to), (Color::parse("#112233").unwrap(), Color::parse("#445566").unwrap()));
        assert!(own.notice.is_some());
        // a leftover [scene] table with another kind is ignored without a fuss
        let quiet = Theme::parse("name = \"Q\"\n[scene]\nclouds = 2\n", Path::new("")).unwrap();
        assert!(quiet.notice.is_none() && quiet.background.kind == BackgroundKind::Solid);
        // and no other kind has a notice
        assert!(builtin().iter().all(|t| t.notice.is_none()));
    }

    #[test]
    fn settings_shows_one_notice_for_all_old_theme_files() {
        let dir = std::env::temp_dir().join(format!("lh-gui-oldscene-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for n in ["one", "two"] {
            std::fs::write(dir.join(format!("{n}.toml")), format!("name = \"Old {n}\"\n[background]\nkind = \"scene\"\n")).unwrap();
        }
        std::fs::write(dir.join("fine.toml"), "name = \"Fine\"\n").unwrap();
        let set = ThemeSet::load(&dir);
        assert!(set.problems.is_empty(), "{:?}", set.problems);
        let n = set.notice.expect("one notice");
        assert!(n.contains("Old one") && n.contains("Old two") && !n.contains("Fine") && n.matches("painted background").count() == 1, "{n}");
        assert!(set.themes.iter().any(|t| t.name == "Old one"), "the theme is there and usable");
        std::fs::remove_file(dir.join("one.toml")).unwrap();
        std::fs::remove_file(dir.join("two.toml")).unwrap();
        assert!(ThemeSet::load(&dir).notice.is_none(), "no old file, no notice");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_painted_scene_is_gone_from_the_source() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        assert!(!src.join("src").join("scene.rs").exists());
        assert!(!src.join("examples").join("scene_preview.rs").exists());
        // the needles are written in two halves so this test does not find itself
        let needles = [concat!("crate::sc", "ene"), concat!("mod sc", "ene"), concat!("Background", "Kind::Sc", "ene"), concat!("lh_gui::sc", "ene"), concat!("Sc", "eneSpec")];
        for entry in std::fs::read_dir(src.join("src")).unwrap().flatten() {
            let p = entry.path();
            if p.extension().is_some_and(|e| e == "rs") {
                let text = std::fs::read_to_string(&p).unwrap();
                for needle in needles {
                    assert!(!text.contains(needle), "{} still has {needle}", p.display());
                }
            }
        }
        for t in ["frutiger-aero-light.toml", "frutiger-aero-dark.toml"] {
            let text = std::fs::read_to_string(src.join("themes").join(t)).unwrap();
            assert!(!text.contains("[scene]") && !text.contains("\"scene\""), "{t}");
        }
    }

    #[test]
    fn a_file_only_needs_to_say_what_it_changes() {
        let t = Theme::parse("name = \"Mine\"\n[colors]\naccent = \"#ff0066\"\n", Path::new("/themes")).unwrap();
        assert_eq!(t.colors.accent.rgba(), 0xff0066ff);
        assert_eq!(t.colors.text, Color::parse("#1d1d1b").unwrap(), "the rest comes from Calm light");
        assert_eq!(t.label(), "Mine light");
        assert_eq!(t.animation_ms, 140);
    }

    #[test]
    fn mistakes_are_reported_with_the_key_that_is_wrong() {
        assert!(Theme::parse("", Path::new("")).unwrap_err().contains("needs a name"));
        let e = Theme::parse("name = \"x\"\n[colors]\naccent = \"blue\"", Path::new("")).unwrap_err();
        assert!(e.contains("colors.accent"), "{e}");
        let e = Theme::parse("name = \"x\"\n[colours]\n", Path::new("")).unwrap_err();
        assert!(e.contains("unknown field"), "{e}");
        let e = Theme::parse("name = \"x\"\n[background]\nkind = \"image\"\n", Path::new("")).unwrap_err();
        assert!(e.contains("background.image"), "{e}");
        assert!(Theme::parse("name = \"x\"\n[shape]\nroundness = 200\n", Path::new("")).unwrap_err().contains("between 0 and 64"));
        assert!(Theme::parse("name = \"x\"\n[background]\nimage_opacity = 2\n", Path::new("")).unwrap_err().contains("image_opacity"));
    }

    #[test]
    fn an_image_path_is_relative_to_the_theme_file() {
        let t = Theme::parse("name = \"x\"\n[background]\nkind = \"image\"\nimage = \"wall.png\"\n", Path::new("/home/me/themes")).unwrap();
        assert_eq!(t.background.image.unwrap(), PathBuf::from("/home/me/themes/wall.png"));
    }

    #[test]
    fn controls_inside_a_panel_follow_the_panels_curve() {
        assert_eq!(Theme::inner_radius(15., 8.), 7.);
        assert_eq!(Theme::inner_radius(6., 8.), 3., "never sharper than 3");
        let mut t = builtin().remove(0);
        t.roundness = 10.;
        assert_eq!(t.panel_radius(Some(15.)), 10.);
        t.match_window = true;
        assert_eq!(t.panel_radius(Some(15.)), 15., "the window's own radius wins when asked for");
        assert_eq!(t.panel_radius(None), 10., "and the theme's value is the fallback");
        assert_eq!(parse_hypr_rounding("{\"option\": \"decoration:rounding\", \"int\": 15, \"set\": true }"), Some(15.));
        assert_eq!(parse_hypr_rounding("garbage"), None);
    }

    #[test]
    fn a_users_folder_adds_themes_replaces_built_ins_and_reports_bad_files() {
        let dir = std::env::temp_dir().join(format!("lh-gui-themes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("mine.toml"), "name = \"Mine\"\nmode = \"dark\"\n").unwrap();
        std::fs::write(dir.join("calm.toml"), "name = \"Calm\"\n[colors]\naccent = \"#00aa00\"\n").unwrap();
        std::fs::write(dir.join("bad.toml"), "name = \"Bad\"\n[colors]\ntext = \"nope\"\n").unwrap();
        std::fs::write(dir.join("notes.txt"), "ignored").unwrap();
        let mut set = ThemeSet::load(&dir);
        assert_eq!(set.themes.len(), 9);
        assert!(set.find("Mine dark").is_some());
        assert_eq!(set.find("Calm light").unwrap().colors.accent.rgba(), 0x00aa00ff, "replaced the built-in");
        assert_eq!(set.problems.len(), 1);
        assert!(set.problems[0].starts_with("bad.toml: colors.text"), "{:?}", set.problems);
        assert!(!set.changed());
        std::thread::sleep(std::time::Duration::from_millis(30));
        std::fs::write(dir.join("mine.toml"), "name = \"Mine\"\nmode = \"dark\"\n[colors]\naccent = \"#123456\"\n").unwrap();
        assert!(set.changed(), "an edited file is noticed");
        set.reload();
        assert_eq!(set.find("Mine dark").unwrap().colors.accent.rgba(), 0x123456ff);
        std::fs::remove_file(dir.join("bad.toml")).unwrap();
        assert!(set.changed(), "a removed file is noticed too");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn every_look_can_be_chosen_saved_and_found_again_after_a_restart() {
        let set = ThemeSet::load(Path::new("/nonexistent-themes-folder"));
        let dir = std::env::temp_dir().join(format!("lh-gui-switch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let file = dir.join("gui.toml");
        // go through every look in turn, as a person clicking through the menu would, including back to the first
        for i in (0..set.themes.len()).chain([3, 0, 7, 1]) {
            let mut prefs = crate::prefs::Prefs::load(&file);
            prefs.theme = set.themes[i].label();
            prefs.save(&file).unwrap();
            let restarted = crate::prefs::Prefs::load(&file);
            assert_eq!(set.index_of(&restarted.theme), i, "{} comes back as itself", set.themes[i].label());
        }
        assert_eq!(set.index_of("A look that was deleted"), 0);
        assert_eq!(set.index_of(""), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn mixing_colours_goes_from_one_to_the_other() {
        let a = Color::parse("#000000").unwrap();
        let b = Color::parse("#ffffff").unwrap();
        assert_eq!(a.mix(b, 0.), a);
        assert_eq!(a.mix(b, 1.), b);
        assert_eq!(a.mix(b, 0.5).r, 128);
        assert_eq!(b.with_alpha(0.5).a, 128);
    }
}
