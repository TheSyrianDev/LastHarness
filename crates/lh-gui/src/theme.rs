//! Themes: a TOML file sets colours, background, roundness, blur, shadows and animation. Fourteen looks come
//! with the app in seven families (Calm, Matte, Glass, Grove, Cobalt, Claude and ChatGPT), each light and dark; users add their own in a folder and
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
    warn: Option<String>,
    /// The three status colours under their plain names. `bad` is `danger` and `caution` is `warn`; `good` is new.
    good: Option<String>,
    caution: Option<String>,
    bad: Option<String>,
    focus_ring: Option<String>,
    bubble: Option<String>,
}
impl Default for RawColors {
    fn default() -> Self {
        // Calm light, so a theme file only has to say what it changes
        let d = |s: &str| s.to_string();
        Self { background: d("#f5f5f3"), surface: d("#ffffff"), sunken: d("#ecece9"), text: d("#1d1d1b"), muted: d("#6d6d68"), accent: d("#3b6fd4"), accent_text: d("#ffffff"), button: d("#e8e8e4"), button_hover: d("#dcdcd6"), border: d("#dadad5"), danger: d("#c0392b"), warn: None, good: None, caution: None, bad: None, focus_ring: None, bubble: None }
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

/// One family name, or a list of names to try in order (the first one installed is used).
#[derive(Deserialize, Clone, Debug, Default)]
#[serde(untagged)]
enum OneOrMany {
    #[default]
    None,
    One(String),
    Many(Vec<String>),
}

impl OneOrMany {
    fn into_vec(self) -> Vec<String> {
        let v = match self {
            OneOrMany::None => vec![],
            OneOrMany::One(s) => vec![s],
            OneOrMany::Many(v) => v,
        };
        v.into_iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
    }
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(default, deny_unknown_fields)]
struct RawFonts {
    latin: OneOrMany,
    arabic: OneOrMany,
    mono: OneOrMany,
}

/// The fonts a theme asks for. Each is a list, best first; empty means the theme has no opinion.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ThemeFonts {
    pub latin: Vec<String>,
    pub arabic: Vec<String>,
    pub mono: Vec<String>,
}

/// Which liquid glass a surface is made of, if any. `none` keeps it a flat fill.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GlassKind {
    #[default]
    None,
    /// Frosted, lightly bent.
    Regular,
    /// See-through, a strong bend at the edge and a sharp rim.
    Clear,
    /// Frosted, carried by the theme's own colour.
    RegularTinted,
    /// See-through, carried by the theme's own colour.
    ClearTinted,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
struct RawMaterial {
    panels: GlassKind,
    popups: GlassKind,
    side_panel: GlassKind,
    controls: GlassKind,
    blur: f32,
    refraction: f32,
    rim: f32,
}
impl Default for RawMaterial {
    fn default() -> Self {
        // flat panels; the glass knob every look already has
        Self { panels: GlassKind::None, popups: GlassKind::None, side_panel: GlassKind::None, controls: GlassKind::Regular, blur: 4., refraction: 6., rim: 0.5 }
    }
}

/// The `[material]` table: which surfaces are liquid glass. The glass is tinted with the surface's own colour (the
/// pop-up fill for pop-ups), so the contrast measured for the flat colours holds for the glass too.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Material {
    /// The sidebar, the rail and cards.
    pub panels: GlassKind,
    /// Menus, pop-up cards and dialogs.
    pub popups: GlassKind,
    /// The panel on the right.
    pub side_panel: GlassKind,
    /// Switch knobs, slider knobs and the segmented thumb. Never `none`: a flat theme still gets the glass knob.
    pub controls: GlassKind,
    /// Blur of what is behind a glass panel or pop-up, in pixels.
    pub blur: f32,
    /// How far the edge of a glass panel bends what is behind, in pixels.
    pub refraction: f32,
    /// Strength of the bright rim of a glass panel, 0 to 1.
    pub rim: f32,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(default, deny_unknown_fields)]
struct Raw {
    fonts: RawFonts,
    material: RawMaterial,
    /// Icon name to an SVG path, plus an optional `folder`. See [`crate::icons::load`].
    icons: std::collections::BTreeMap<String, String>,
    name: String,
    mode: Mode,
    colors: RawColors,
    background: RawBackground,
    shape: RawShape,
    effects: RawEffects,
    animation: RawAnimation,
    /// The painted-landscape table of older theme files. It is read and ignored.
    scene: Option<toml::Value>,
    /// The tables of looks that no longer exist (glass panes, gel buttons, a focus border, drifting bubbles). They are
    /// read and ignored, and the person is told once.
    glass: Option<toml::Value>,
    gel: Option<toml::Value>,
    focus: Option<toml::Value>,
    motes: Option<toml::Value>,
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
    /// Cautions that are not errors. Optional in a file; a readable amber for the mode when left out.
    pub warn: Color,
    /// Success: done ticks, passed checks, saved notices. Optional in a file (`good`); a readable green for the mode when left out.
    pub good: Color,
    /// The ring around the control that has the keyboard. Optional in a file; the accent when left out.
    pub focus_ring: Color,
    /// The person's own message bubble. Optional in a file; the button colour when left out.
    pub bubble: Color,
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
    /// The `[fonts]` table: optional families for Latin, Arabic and code.
    pub fonts: ThemeFonts,
    /// What in this file no longer exists and is ignored (the painted background, glass panes, gel buttons, a focus
    /// border, bubbles). Settings tells the person once, naming the file.
    pub ignored: Vec<&'static str>,
    /// Which surfaces are liquid glass.
    pub material: Material,
    /// The theme's own icons by name; any it leaves out are drawn by the app.
    pub icons: crate::icons::IconSet,
    /// Icons named in the file that could not be used (they fall back to the app's own). Shown with the theme problems.
    pub icon_problems: Vec<String>,
    /// The brand mark's colours in this look; `None` uses the brand kit's own for the mode. Set by the Omarchy look.
    pub mark: Option<crate::brand::Colours>,
    /// Chart series colours; empty uses the app's own. Set by the Omarchy look from the desktop theme's hues.
    pub series: Vec<Color>,
    /// The desktop theme's text-selection colour, when it gives one.
    pub selection: Option<Color>,
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
            danger: match &c.bad {
                Some(v) => col("bad", v)?,
                None => col("danger", &c.danger)?,
            },
            warn: match (&c.caution, &c.warn) {
                (Some(v), _) => col("caution", v)?,
                (None, Some(v)) => col("warn", v)?,
                (None, None) => Color::parse(if raw.mode == Mode::Dark { "#e0b25c" } else { "#8a5a00" })?,
            },
            good: match &c.good {
                Some(v) => col("good", v)?,
                None => Color::parse(if raw.mode == Mode::Dark { "#6fcf8a" } else { "#1e7a3c" })?,
            },
            focus_ring: match &c.focus_ring {
                Some(v) => col("focus_ring", v)?,
                None => col("accent", &c.accent)?,
            },
            bubble: match &c.bubble {
                Some(v) => col("bubble", v)?,
                None => col("button", &c.button)?,
            },
        };
        let b = &raw.background;
        let bg = |k: &str, v: &Option<String>| -> Result<Color, String> {
            match v {
                Some(s) => Color::parse(s).map_err(|e| format!("background.{k}: {e}")),
                None => Ok(colors.background),
            }
        };
        let mut ignored: Vec<&'static str> = vec![];
        let mut was_scene = false;
        let kind = match b.kind.trim().to_lowercase().as_str() {
            "solid" => BackgroundKind::Solid,
            "gradient" => BackgroundKind::Gradient,
            "image" => BackgroundKind::Image,
            // the painted landscape of earlier versions is gone: the file still loads, with a plain gradient
            "scene" => {
                was_scene = true;
                ignored.push("a painted background");
                BackgroundKind::Gradient
            }
            other => return Err(format!("background.kind must be solid, gradient or image, not \"{other}\"")),
        };
        // tables of looks that no longer exist: ignored, and the person is told once
        for (table, what) in [(&raw.glass, "glass panes"), (&raw.gel, "gel buttons"), (&raw.focus, "a focus border"), (&raw.motes, "drifting bubbles")] {
            if table.is_some() {
                ignored.push(what);
            }
        }
        let from = bg("from", &b.from)?;
        let mut to = bg("to", &b.to)?;
        let mut angle = b.angle;
        if was_scene && b.to.is_none() {
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
        let m = &raw.material;
        if !(0.0..=24.0).contains(&m.blur) || !(0.0..=24.0).contains(&m.refraction) || !(0.0..=1.0).contains(&m.rim) {
            return Err("material: blur and refraction must be between 0 and 24, rim between 0 and 1".into());
        }
        let material = Material {
            panels: m.panels,
            popups: m.popups,
            side_panel: m.side_panel,
            controls: if m.controls == GlassKind::None { GlassKind::Regular } else { m.controls },
            blur: m.blur,
            refraction: m.refraction,
            rim: m.rim,
        };
        let (icons, icon_problems) = crate::icons::load(&raw.icons, dir);
        Ok(Theme {
            material,
            icons,
            icon_problems,
            mark: None,
            series: vec![],
            selection: None,
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
            fonts: ThemeFonts { latin: raw.fonts.latin.into_vec(), arabic: raw.fonts.arabic.into_vec(), mono: raw.fonts.mono.into_vec() },
            ignored,
        })
    }

    /// The unique label of a theme in lists: "Calm light".
    pub fn label(&self) -> String {
        format!("{} {}", self.name, if self.mode == Mode::Dark { "dark" } else { "light" })
    }

    /// Does anything in the look let the desktop show through? Then the window must be transparent.
    pub fn is_translucent(&self) -> bool {
        let c = &self.colors;
        self.background_is_translucent() || c.surface.is_translucent() || self.blur
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
    // from the user's brand kit palettes
    ("grove-light", include_str!("../themes/grove-light.toml")),
    ("grove-dark", include_str!("../themes/grove-dark.toml")),
    ("cobalt-light", include_str!("../themes/cobalt-light.toml")),
    ("cobalt-dark", include_str!("../themes/cobalt-dark.toml")),
    // "Claude" and "ChatGPT" are looks named after two products; they use open fonts only.
    ("claude-light", include_str!("../themes/claude-light.toml")),
    ("claude-dark", include_str!("../themes/claude-dark.toml")),
    ("chatgpt-light", include_str!("../themes/chatgpt-light.toml")),
    ("chatgpt-dark", include_str!("../themes/chatgpt-dark.toml")),
];

/// The themes that ship with the app. Calm light is first, and is what a fresh install uses.
pub fn builtin() -> Vec<Theme> {
    BUILTIN.iter().map(|(n, s)| {
        let mut t = Theme::parse(s, Path::new("")).unwrap_or_else(|e| panic!("built-in theme {n}: {e}"));
        for (look, icon, svg) in BUILTIN_ICONS {
            if look == n {
                t.icons.insert(icon.to_string(), std::sync::Arc::from(*svg));
            }
        }
        t
    }).collect()
}

/// Icons a built-in look brings with it, compiled in: (look file, icon name, SVG). A built-in look gives its own icon
/// with a line here such as `("glass-light", "send", include_bytes!("../themes/glass/icons/send.svg"))`. None yet.
const BUILTIN_ICONS: &[(&str, &str, &[u8])] = &[];

/// Built-in themes plus the user's own, with a note for each file that could not be read.
pub struct ThemeSet {
    pub themes: Vec<Theme>,
    pub problems: Vec<String>,
    /// One line for the person when theme files use something that no longer exists (a painted background, glass panes,
    /// gel buttons, a focus border, drifting bubbles), naming each file; `None` when all is well. Shown once in Settings.
    pub notice: Option<String>,
    dir: PathBuf,
    stamp: Option<SystemTime>,
    count: usize,
    /// Omarchy's current-theme folder, when the app looks for one.
    omarchy: Option<PathBuf>,
    omarchy_stamp: crate::omarchy::Stamp,
    /// Hyprland's rounding, opacity and blur as last read, and when they were last asked for (every 5 s at most).
    hypr: Option<crate::omarchy::Hypr>,
    hypr_asked: std::cell::Cell<std::time::Instant>,
}

/// How often the theme tick asks Hyprland again; each ask is one short `hyprctl` run.
const HYPR_EVERY: std::time::Duration = std::time::Duration::from_secs(5);

/// Hyprland's settings when the Omarchy look is in play; never in unit tests, which must not see this machine.
fn ask_hypr(omarchy: Option<&Path>) -> Option<crate::omarchy::Hypr> {
    if cfg!(test) || omarchy.is_none() { None } else { crate::omarchy::Hypr::read() }
}

impl ThemeSet {
    /// Built-in looks, the person's own, and the Omarchy look when the desktop has an Omarchy theme.
    pub fn load(dir: &Path) -> ThemeSet {
        // unit tests never pick up the desktop of the machine they run on
        ThemeSet::load_with(dir, if cfg!(test) { None } else { crate::omarchy::current_dir() })
    }

    /// As [`ThemeSet::load`], with Omarchy's current-theme folder given (`None`: never look for it).
    pub fn load_with(dir: &Path, omarchy: Option<PathBuf>) -> ThemeSet {
        let mut themes = builtin();
        // on Omarchy its look comes first, so a fresh install (no look chosen) matches the desktop
        let omarchy_stamp = omarchy.as_deref().map(crate::omarchy::stamp).unwrap_or_default();
        let hypr = ask_hypr(omarchy.as_deref());
        if let Some(t) = omarchy.as_deref().and_then(|d| crate::omarchy::load(d, hypr)) {
            themes.insert(0, t);
        }
        let mut problems = vec![];
        let mut old_things: Vec<(String, Vec<&'static str>)> = vec![];
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
                    problems.extend(t.icon_problems.iter().map(|e| format!("{name}: {e}")));
                    if !t.ignored.is_empty() && !old_things.iter().any(|(n, _)| *n == t.name) {
                        old_things.push((t.name.clone(), t.ignored.clone()));
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
        let notice = (!old_things.is_empty()).then(|| {
            let files: Vec<String> = old_things.iter().map(|(n, what)| format!("{n} ({})", what.join(", "))).collect();
            format!("{} {} something that no longer exists. It is ignored and the rest of the file still counts.", files.join(", "), if old_things.len() == 1 { "uses" } else { "use" })
        });
        ThemeSet { themes, problems, notice, dir: dir.to_path_buf(), stamp, count, omarchy, omarchy_stamp, hypr, hypr_asked: std::cell::Cell::new(std::time::Instant::now()) }
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
        newest != self.stamp || count != self.count || self.omarchy.as_deref().is_some_and(|d| crate::omarchy::stamp(d) != self.omarchy_stamp) || self.hypr_changed()
    }

    /// Has Hyprland's rounding, opacity or blur changed? Asked at most every 5 s, only when Omarchy is there.
    fn hypr_changed(&self) -> bool {
        if self.omarchy.is_none() || self.hypr_asked.get().elapsed() < HYPR_EVERY {
            return false;
        }
        self.hypr_asked.set(std::time::Instant::now());
        ask_hypr(self.omarchy.as_deref()) != self.hypr
    }

    pub fn reload(&mut self) {
        *self = ThemeSet::load_with(&self.dir.clone(), self.omarchy.clone());
    }

    /// The families of looks, once each, in the order they first appear: "Calm", "Matte", "Glass",
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

/// What to tell the person when their saved look is not among the looks there are now (a look that was taken out, or
/// a theme file they deleted): the app uses the first look instead. `None` when nothing was saved or it is there.
pub fn missing_look_notice(saved: &str, families: &[String]) -> Option<String> {
    let saved = saved.trim();
    if saved.is_empty() || families.iter().any(|f| f == saved) {
        return None;
    }
    let first = families.first().map(String::as_str).unwrap_or("Calm");
    Some(format!("The look \"{saved}\" is not there any more, so {first} is used. Pick another in the list above if you like."))
}

/// WCAG relative luminance of an opaque colour.
pub fn luminance(c: Color) -> f64 {
    let f = |v: u8| {
        let x = v as f64 / 255.;
        if x <= 0.04045 { x / 12.92 } else { ((x + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * f(c.r) + 0.7152 * f(c.g) + 0.0722 * f(c.b)
}

/// `fg` painted over `bg`, by its own opacity.
fn over(fg: Color, bg: Color) -> Color {
    Color { a: 255, ..bg.mix(Color { a: 255, ..fg }, fg.a as f32 / 255.) }
}

/// Every place a look's words or marks are too faint. Translucent looks are measured over black (dark) or white
/// (light) behind the window; text needs 4.5:1 (WCAG AA), the accent and the focus ring need 3:1 as non-text marks.
pub fn readability_problems(t: &Theme) -> Vec<String> {
    let c = &t.colors;
    let desk = if t.mode == Mode::Dark { Color { r: 0, g: 0, b: 0, a: 255 } } else { Color { r: 255, g: 255, b: 255, a: 255 } };
    let bg = over(c.background, desk);
    let (surface, sunken, button, accent) = (over(c.surface, bg), over(c.sunken, bg), over(c.button, bg), over(c.accent, bg));
    let checks = [
        ("text on surface", c.text, surface, 4.5),
        ("text on background", c.text, bg, 4.5),
        ("text on input", c.text, sunken, 4.5),
        ("text on button", c.text, button, 4.5),
        ("text on bubble", c.text, over(c.bubble, bg), 4.5),
        ("muted on surface", c.muted, surface, 4.5),
        ("muted on background", c.muted, bg, 4.5),
        ("muted on input", c.muted, sunken, 4.5),
        ("accent text on accent", c.accent_text, accent, 4.5),
        ("danger on surface", c.danger, surface, 4.5),
        ("warn on surface", c.warn, surface, 4.5),
        ("good on surface", c.good, surface, 4.5),
        ("accent on surface", accent, surface, 3.0),
        ("focus ring on background", c.focus_ring, bg, 3.0),
    ];
    let mut low = vec![];
    for (what, fg, back, min) in checks {
        let r = contrast(over(fg, back), back);
        if r < min {
            low.push(format!("{}: {what} is {r:.2}, needs {min}", t.label()));
        }
    }
    low
}

/// The Glass look tinted around up to three colours the person picked, like a gradient picker: the first leads the
/// accent and the top of the background, the last the bottom, the middle one the panels. `opacity` (0 to 1) is how
/// strongly the colours show. The tint backs off until every readability check passes, so it can never make words faint.
pub fn tint_glass(t: &Theme, picks: &[Color], opacity: f32) -> Theme {
    let Some(&first) = picks.first() else { return t.clone() };
    let last = *picks.last().unwrap();
    let mid = picks.get(1).copied().unwrap_or(first);
    let dark = t.mode == Mode::Dark;
    let (black, white) = (Color { r: 0, g: 0, b: 0, a: 255 }, Color { r: 255, g: 255, b: 255, a: 255 });
    let mut s = opacity.clamp(0., 1.);
    loop {
        let mut g = t.clone();
        let towards = |c: Color, p: Color, k: f32| Color { a: c.a, ..c.mix(Color { a: c.a, ..p }, k) };
        let c = &mut g.colors;
        c.background = towards(c.background, first.mix(last, 0.5), s * 0.45);
        c.surface = towards(c.surface, mid, s * 0.25);
        c.button = towards(c.button, mid, s * 0.3);
        c.button_hover = towards(c.button_hover, mid, s * 0.35);
        g.background.from = towards(g.background.from, first, s * 0.6);
        g.background.to = towards(g.background.to, last, s * 0.6);
        // the accent is the first colour, darkened (light) or lightened (dark) until it reads as a mark
        let bg = over(c.background, if dark { black } else { white });
        let surface = over(c.surface, bg);
        let mut accent = Color { a: 255, ..first };
        for _ in 0..20 {
            if contrast(accent, surface) >= 3.2 {
                break;
            }
            accent = accent.mix(if dark { white } else { black }, 0.12);
        }
        c.accent = accent;
        c.focus_ring = accent;
        c.accent_text = if contrast(white, accent) >= contrast(black.mix(accent, 0.1), accent) { white } else { black.mix(accent, 0.1) };
        // status and quiet words deepen a little against the tinted panels rather than the tint fading away
        let ink = if dark { white } else { black };
        for col in [&mut c.danger, &mut c.warn, &mut c.good, &mut c.muted] {
            for _ in 0..12 {
                if contrast(*col, surface) >= 4.7 && contrast(*col, bg) >= 4.7 {
                    break;
                }
                *col = col.mix(ink, 0.08);
            }
        }
        if readability_problems(&g).is_empty() || s <= 0. {
            return if s <= 0. && !readability_problems(&g).is_empty() { t.clone() } else { g };
        }
        s = (s - 0.1).max(0.);
    }
}

/// The Glass colour field, like a gradient picker: hue runs across (`col` of `FIELD_COLS`), from pale at the top to
/// deep at the bottom (`row` of `FIELD_ROWS`). Returns "#rrggbb".
pub const FIELD_COLS: usize = 24;
pub const FIELD_ROWS: usize = 6;
pub fn field_colour(col: usize, row: usize) -> String {
    let h = col as f32 / FIELD_COLS as f32;
    let l = 0.85 - 0.55 * row as f32 / (FIELD_ROWS - 1) as f32;
    let s = 0.8_f32;
    let q = if l < 0.5 { l * (1. + s) } else { l + s - l * s };
    let p = 2. * l - q;
    let ch = |mut t: f32| {
        t = t.rem_euclid(1.);
        let v = if t < 1. / 6. { p + (q - p) * 6. * t } else if t < 0.5 { q } else if t < 2. / 3. { p + (q - p) * (2. / 3. - t) * 6. } else { p };
        (v * 255.).round() as u8
    };
    format!("#{:02x}{:02x}{:02x}", ch(h + 1. / 3.), ch(h), ch(h - 1. / 3.))
}

/// Clicking a colour in the field: a picked one is taken off, a new one is added, and with three already picked the
/// newest is replaced.
pub fn toggle_pick(picks: &mut Vec<String>, colour: &str) {
    if let Some(i) = picks.iter().position(|c| c == colour) {
        picks.remove(i);
    } else {
        if picks.len() >= 3 {
            picks.truncate(2);
        }
        picks.push(colour.to_string());
    }
}

/// WCAG contrast ratio of two opaque colours, 1 to 21.
pub fn contrast(a: Color, b: Color) -> f64 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
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
    fn all_fourteen_built_in_themes_load_and_have_unique_labels() {
        let t = builtin();
        assert_eq!(t.len(), 14);
        let mut labels: Vec<String> = t.iter().map(|t| t.label()).collect();
        labels.sort();
        labels.dedup();
        assert_eq!(labels.len(), 14);
        assert_eq!(t[0].label(), "Calm light", "the plain default comes first");
        for name in ["Calm", "Matte", "Glass", "Grove", "Cobalt", "Claude", "ChatGPT"] {
            for mode in ["light", "dark"] {
                assert!(t.iter().any(|x| x.label() == format!("{name} {mode}")), "{name} {mode}");
            }
        }
        assert!(t.iter().all(|x| x.ignored.is_empty()), "no built-in look uses anything that no longer exists");
    }

    #[test]
    fn three_families_each_with_both_modes_and_the_mode_picks_the_variant() {
        let set = ThemeSet::load(Path::new("/no/such/folder"));
        assert_eq!(set.families(), ["Calm", "Matte", "Glass", "Grove", "Cobalt", "Claude", "ChatGPT"]);
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
        assert!(by("Glass light").roundness > by("Calm light").roundness, "glass is rounder than calm");
    }

    #[test]
    fn a_theme_with_a_bad_value_is_still_reported_with_the_key_that_is_wrong() {
        let e = |src: &str| Theme::parse(&format!("name = \"x\"\n{src}"), Path::new("")).unwrap_err();
        assert!(e("[background]\nkind = \"wallpaper\"").contains("solid, gradient or image"));
        assert!(e("[effects]\nblurr = true").contains("unknown field"));
    }

    /// Looks that were taken out. A user's file that still has their tables must load, ignore them, and say so.
    #[test]
    fn an_old_theme_with_removed_tables_or_the_painted_scene_loads_and_says_what_it_ignored() {
        let old = "name = \"Mine\"\nmode = \"dark\"\n[colors]\nbackground = \"#102030\"\ntext = \"#e0e0e0\"\n[background]\nkind = \"scene\"\n[scene]\nsky = [\"#000\", \"#fff\"]\nnothing_like_a_real_key = true\n[glass]\ntop = \"#ffffff80\"\nsheen = 0.2\n[gel]\ngloss = 0.4\n[focus]\nfrom = \"#123456\"\n[motes]\ncount = 5\n[shape]\nroundness = 12\n";
        let t = Theme::parse(old, Path::new("")).expect("an old file must still load");
        assert_eq!(t.background.kind, BackgroundKind::Gradient);
        assert_eq!(t.background.from, Color::parse("#102030").unwrap(), "from the theme's own background colour");
        assert_ne!(t.background.to, t.background.from, "a visible gradient, not a flat fill");
        assert_eq!(t.roundness, 12., "the rest of the file still counts");
        assert_eq!(t.ignored, ["a painted background", "glass panes", "gel buttons", "a focus border", "drifting bubbles"]);
        // a gradient the person wrote themselves is kept
        let own = Theme::parse("name = \"Own\"\n[background]\nkind = \"scene\"\nfrom = \"#112233\"\nto = \"#445566\"\n", Path::new("")).unwrap();
        assert_eq!((own.background.from, own.background.to), (Color::parse("#112233").unwrap(), Color::parse("#445566").unwrap()));
        // a leftover [scene] table with another kind is ignored without a fuss
        let quiet = Theme::parse("name = \"Q\"\n[scene]\nclouds = 2\n", Path::new("")).unwrap();
        assert!(quiet.ignored.is_empty() && quiet.background.kind == BackgroundKind::Solid);
        // each of the removed tables alone is noticed, and none is an error, even empty
        for (table, what) in [("glass", "glass panes"), ("gel", "gel buttons"), ("focus", "a focus border"), ("motes", "drifting bubbles")] {
            let t = Theme::parse(&format!("name = \"x\"\n[{table}]\n"), Path::new("")).unwrap();
            assert_eq!(t.ignored, [what], "{table}");
        }
    }

    #[test]
    fn settings_shows_one_notice_naming_every_old_theme_file() {
        let dir = std::env::temp_dir().join(format!("lh-gui-oldtables-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("one.toml"), "name = \"Old one\"\n[background]\nkind = \"scene\"\n").unwrap();
        std::fs::write(dir.join("two.toml"), "name = \"Old two\"\n[glass]\n[motes]\ncount = 3\n").unwrap();
        std::fs::write(dir.join("fine.toml"), "name = \"Fine\"\n").unwrap();
        let set = ThemeSet::load(&dir);
        assert!(set.problems.is_empty(), "{:?}", set.problems);
        let n = set.notice.expect("one notice");
        assert!(n.contains("Old one (a painted background)") && n.contains("Old two (glass panes, drifting bubbles)") && !n.contains("Fine"), "{n}");
        assert_eq!(n.lines().count(), 1, "one line");
        assert!(set.themes.iter().any(|t| t.name == "Old one") && set.themes.iter().any(|t| t.name == "Old two"), "the themes are there and usable");
        std::fs::remove_file(dir.join("one.toml")).unwrap();
        std::fs::remove_file(dir.join("two.toml")).unwrap();
        assert!(ThemeSet::load(&dir).notice.is_none(), "no old file, no notice");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The glass look of Windows 7 was tried and taken out on 2026-10-13. Nothing in the code, the built-in themes or
    /// the readme may name it, except the code that reads an old saved choice of it (prefs.rs and its tests).
    #[test]
    fn the_removed_look_is_named_nowhere_but_the_code_that_reads_an_old_saved_choice() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        // written in halves so this test does not find itself
        let needles = [concat!("frut", "iger"), concat!("ae", "ro")];
        let mut files: Vec<std::path::PathBuf> = vec![];
        for dir in ["src", "themes", "examples", "tests"] {
            for e in std::fs::read_dir(root.join(dir)).into_iter().flatten().flatten() {
                files.push(e.path());
            }
        }
        files.push(root.join("Cargo.toml"));
        files.push(root.join("../../README.md"));
        let mut checked = 0;
        for p in files.into_iter().filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "rs" || e == "toml" || e == "md")) {
            if p.file_name().is_some_and(|n| n == "prefs.rs") {
                continue;
            }
            checked += 1;
            let text = std::fs::read_to_string(&p).unwrap().to_lowercase();
            for n in needles {
                assert!(!text.contains(n), "{} still mentions it ({n})", p.display());
            }
        }
        assert!(checked > 20, "the scan looked at {checked} files");
        let (light, dark) = (concat!("themes/frut", "iger-ae", "ro-light.toml"), concat!("themes/frut", "iger-ae", "ro-dark.toml"));
        for gone in ["src/scene.rs", "src/sheen.rs", "examples/scene_preview.rs", "examples/glass_preview.rs", light, dark] {
            assert!(!root.join(gone).exists(), "{gone} is still there");
        }
    }

    #[test]
    fn the_fonts_table_takes_a_name_or_a_list_and_is_optional() {
        let none = Theme::parse("name = \"x\"\n", Path::new("")).unwrap();
        assert_eq!(none.fonts, ThemeFonts::default());
        let t = Theme::parse("name = \"x\"\n[fonts]\nlatin = \"Noto Serif\"\narabic = [\"Noto Naskh Arabic\", \" \", \"Noto Sans Arabic\"]\n", Path::new("")).unwrap();
        assert_eq!(t.fonts.latin, ["Noto Serif"]);
        assert_eq!(t.fonts.arabic, ["Noto Naskh Arabic", "Noto Sans Arabic"], "blank names dropped");
        assert!(t.fonts.mono.is_empty());
        assert!(Theme::parse("name = \"x\"\n[fonts]\nlatn = \"a\"\n", Path::new("")).unwrap_err().contains("unknown field"));
    }

    #[test]
    fn the_dev_themes_set_fonts_and_the_calm_looks_set_none() {
        let t = builtin();
        for x in &t {
            let sets = !x.fonts.latin.is_empty();
            assert_eq!(sets, x.name == "Claude" || x.name == "ChatGPT", "{}", x.label());
            if sets {
                assert!(!x.fonts.arabic.is_empty() && !x.fonts.mono.is_empty(), "{}", x.label());
            }
        }
        let claude = t.iter().find(|x| x.label() == "Claude light").unwrap();
        assert_eq!(claude.colors.background, Color::parse("#F5F4EE").unwrap());
        assert_eq!(claude.colors.accent, Color::parse("#B5532F").unwrap());
        assert_eq!(t.iter().find(|x| x.label() == "Claude dark").unwrap().colors.background, Color::parse("#262624").unwrap());
        assert_eq!(t.iter().find(|x| x.label() == "ChatGPT dark").unwrap().colors.background, Color::parse("#212121").unwrap());
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
        assert_eq!(set.themes.len(), 15, "fourteen built in, one of the person's own, and one replaced");
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
        for i in (0..set.themes.len()).chain([3, 0, 5, 1]) {
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

    pub(crate) use super::contrast;

    #[test]
    fn every_look_keeps_text_readable_on_every_surface() {
        let low: Vec<String> = builtin().iter().flat_map(readability_problems).collect();
        assert!(low.is_empty(), "{low:#?}");
    }

    #[test]
    fn the_colour_field_and_picks() {
        assert_eq!(field_colour(0, 2).len(), 7);
        assert!(Color::parse(&field_colour(FIELD_COLS - 1, FIELD_ROWS - 1)).is_ok());
        let mut p = vec![];
        for c in ["#a", "#b", "#c", "#d"] {
            toggle_pick(&mut p, c);
        }
        assert_eq!(p, ["#a", "#b", "#d"]);
        toggle_pick(&mut p, "#b");
        assert_eq!(p, ["#a", "#d"]);
    }

    #[test]
    fn a_glass_tint_keeps_every_check() {
        let picks: Vec<Color> = ["#ff0000", "#ffff00", "#00ff00", "#0000ff", "#ffffff", "#000000", "#ff00ff", "#808080"].iter().map(|c| Color::parse(c).unwrap()).collect();
        for t in builtin().into_iter().filter(|t| t.name == "Glass") {
            for w in picks.windows(3) {
                for n in 1..=3 {
                    for o in [0.2, 0.6, 1.0] {
                        let g = tint_glass(&t, &w[..n], o);
                        let low = readability_problems(&g);
                        assert!(low.is_empty(), "{:?} {o}: {low:#?}", &w[..n]);
                    }
                }
            }
            assert_eq!(tint_glass(&t, &[], 1.0), t, "no colours leaves the look alone");
            assert_ne!(tint_glass(&t, &picks[..1], 1.0).colors.background, t.colors.background);
        }
    }

    #[test]
    fn glass_is_made_of_glass_and_the_other_looks_stay_flat_with_a_glass_knob() {
        for t in builtin() {
            let m = t.material;
            assert_ne!(m.controls, GlassKind::None, "{}", t.label());
            if t.name == "Glass" {
                assert!(m.panels != GlassKind::None && m.popups != GlassKind::None && m.side_panel != GlassKind::None, "{}", t.label());
            } else {
                assert_eq!((m.panels, m.popups, m.side_panel), (GlassKind::None, GlassKind::None, GlassKind::None), "{}", t.label());
            }
        }
    }

    #[test]
    fn a_resting_button_stands_off_a_glass_panel() {
        // it once barely showed on the Glass light panel
        for t in builtin().into_iter().filter(|t| t.name == "Glass") {
            let c = &t.colors;
            let desk = if t.mode == Mode::Dark { Color { r: 0, g: 0, b: 0, a: 255 } } else { Color { r: 255, g: 255, b: 255, a: 255 } };
            let surface = over(c.surface, over(c.background, desk));
            let r = contrast(over(c.button, surface), surface);
            assert!(r >= 1.15, "{}: button on panel is {r:.2}", t.label());
        }
    }

    #[test]
    fn a_users_theme_brings_its_own_icons_and_the_rest_fall_back() {
        // the fixture: a theme file with an icons folder and one named file, one of them missing
        let d = tempfile::tempdir().unwrap();
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><circle cx="12" cy="12" r="8"/></svg>"#;
        std::fs::create_dir_all(d.path().join("mine/icons")).unwrap();
        std::fs::write(d.path().join("mine/icons/send.svg"), svg).unwrap();
        std::fs::write(d.path().join("mine/gear.svg"), svg).unwrap();
        std::fs::write(d.path().join("mine.toml"), "name = \"Mine\"\n[icons]\nfolder = \"mine/icons\"\nsettings = \"mine/gear.svg\"\nsearch = \"mine/gone.svg\"\n").unwrap();
        let set = ThemeSet::load(d.path());
        let t = set.find("Mine light").expect("loaded");
        assert_eq!(t.icons.keys().collect::<Vec<_>>(), ["send", "settings"]);
        assert!(set.problems.iter().any(|p| p.starts_with("mine.toml: icons.search")), "{:?}", set.problems);
        assert!(builtin().iter().all(|t| t.icons.is_empty()), "no built-in look ships icon art yet");
    }

    #[test]
    fn a_bad_material_is_reported() {
        assert!(Theme::parse("name = \"X\"\n[material]\npanels = \"frosted\"", Path::new(".")).is_err());
        assert!(Theme::parse("name = \"X\"\n[material]\nblur = 99", Path::new(".")).unwrap_err().contains("material"));
    }

    #[test]
    fn warn_and_focus_ring_are_optional_and_fall_back() {
        let light = Theme::parse("name = \"X\"", Path::new(".")).unwrap();
        assert_eq!(light.colors.focus_ring, light.colors.accent);
        let dark = Theme::parse("name = \"X\"\nmode = \"dark\"", Path::new(".")).unwrap();
        assert_ne!(light.colors.warn, dark.colors.warn, "the fallback amber follows the mode");
        assert_ne!(light.colors.good, dark.colors.good, "the fallback green follows the mode");
    }

    #[test]
    fn good_caution_bad_are_the_status_keys_and_old_names_still_work() {
        let t = Theme::parse("name = \"X\"\n[colors]\ngood = \"#00ff00\"\ncaution = \"#ffff00\"\nbad = \"#ff0000\"", Path::new(".")).unwrap();
        assert_eq!((t.colors.good, t.colors.warn, t.colors.danger), (Color::parse("#00ff00").unwrap(), Color::parse("#ffff00").unwrap(), Color::parse("#ff0000").unwrap()));
        let old = Theme::parse("name = \"X\"\n[colors]\ndanger = \"#aa0000\"\nwarn = \"#aaaa00\"", Path::new(".")).unwrap();
        assert_eq!((old.colors.danger, old.colors.warn), (Color::parse("#aa0000").unwrap(), Color::parse("#aaaa00").unwrap()));
    }


    #[test]
    fn on_omarchy_its_look_comes_first_and_follows_a_theme_change() {
        let root = std::env::temp_dir().join(format!("lh-theme-omarchy-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let cur = root.join("current");
        std::fs::create_dir_all(cur.join("theme")).unwrap();
        std::fs::write(cur.join("theme.name"), "a").unwrap();
        std::fs::write(cur.join("theme/colors.toml"), include_str!("../tests/fixtures/omarchy-dark.toml")).unwrap();
        let mut set = ThemeSet::load_with(&root.join("none"), Some(cur.clone()));
        assert_eq!(set.families()[0], "Omarchy");
        assert_eq!(set.themes[set.resolve("", false)].label(), "Omarchy dark", "no look chosen: the desktop's, in its own mode");
        assert!(!set.changed());
        std::fs::write(cur.join("theme.name"), "b").unwrap();
        std::fs::write(cur.join("theme/colors.toml"), include_str!("../tests/fixtures/omarchy-light.toml")).unwrap();
        assert!(set.changed(), "omarchy theme set is noticed");
        set.reload();
        assert_eq!(set.themes[0].label(), "Omarchy light");
        // the theme folder goes away (not on Omarchy any more): the look is gone and Calm is first again
        std::fs::remove_dir_all(&cur).unwrap();
        assert!(set.changed());
        set.reload();
        assert_eq!(set.families()[0], "Calm");
        assert!(missing_look_notice("Omarchy", &set.families()).is_some());
        let _ = std::fs::remove_dir_all(&root);
    }
}
