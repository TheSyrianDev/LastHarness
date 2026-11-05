//! A tray icon (StatusNotifierItem) so the app can be reached when its window is hidden or closed.
//! Where the desktop has no tray, registering fails quietly and the app runs without one.
//!
//! The icon is the brand mark alone, in one colour, on a clear background (the 16 px pixel mark from
//! `brand/motion-kit/tools/tray16.js` at 16 px, the 64-unit mark from `brand.rs` at 22 and 32 px). Its colour:
//! on Omarchy, the theme's foreground; elsewhere, light on a dark system scheme and dark on a light one. The colour
//! is re-read every few seconds and when the Omarchy theme changes (the app's theme tick calls [`sync`]).

use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};

use lh_gui::omarchy;
use lh_gui::theme::Color;

/// What the icon shows: a flat colour as 0xRRGGBB, or the symbolic icon name when nothing can say what colour fits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    Pixmap(u32),
    Symbolic,
}

/// The fallback colours: light for a dark system scheme, dark for a light one.
pub const LIGHT_ICON: u32 = 0xf0f0f0;
pub const DARK_ICON: u32 = 0x1e1e1e;

/// The system's light or dark setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scheme {
    Dark,
    Light,
}

/// The icon's colour rule: Omarchy's foreground when there is one, else a colour from the system scheme, else the
/// symbolic name.
pub fn pick(omarchy_fg: Option<Color>, scheme: Option<Scheme>) -> Pick {
    if let Some(c) = omarchy_fg {
        return Pick::Pixmap((c.r as u32) << 16 | (c.g as u32) << 8 | c.b as u32);
    }
    match scheme {
        Some(Scheme::Dark) => Pick::Pixmap(LIGHT_ICON),
        Some(Scheme::Light) => Pick::Pixmap(DARK_ICON),
        None => Pick::Symbolic,
    }
}

/// Read the portal's answer to `Settings.Read org.freedesktop.appearance color-scheme`: `(<<uint32 1>>,)` prefers
/// dark, 2 prefers light, 0 has no preference.
pub fn parse_portal(out: &str) -> Option<Scheme> {
    if out.contains("uint32 1") {
        Some(Scheme::Dark)
    } else if out.contains("uint32 2") {
        Some(Scheme::Light)
    } else {
        None
    }
}

/// Read gsettings' answer for `org.gnome.desktop.interface color-scheme`: `'prefer-dark'` or `'prefer-light'`.
pub fn parse_gsettings(out: &str) -> Option<Scheme> {
    match out.trim().trim_matches('\'') {
        "prefer-dark" => Some(Scheme::Dark),
        "prefer-light" => Some(Scheme::Light),
        _ => None,
    }
}

/// The system's light or dark setting: the freedesktop portal first, then gsettings. `None` when neither answers.
fn read_scheme() -> Option<Scheme> {
    let run = |cmd: &str, args: &[&str]| {
        Command::new(cmd).args(args).stdin(Stdio::null()).stderr(Stdio::null()).output().ok().filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
    };
    let portal = run(
        "gdbus",
        &["call", "--session", "--dest", "org.freedesktop.portal.Desktop", "--object-path", "/org/freedesktop/portal/desktop", "--method", "org.freedesktop.portal.Settings.Read", "org.freedesktop.appearance", "color-scheme"],
    )
    .and_then(|out| parse_portal(&out));
    portal.or_else(|| run("gsettings", &["get", "org.gnome.desktop.interface", "color-scheme"]).and_then(|out| parse_gsettings(&out)))
}

/// What the icon should show right now.
fn current_pick() -> Pick {
    let fg = omarchy::current_dir().and_then(|d| omarchy::foreground(&d));
    let scheme = if fg.is_some() { None } else { read_scheme() };
    pick(fg, scheme)
}

/// The mark as ARGB32 (network byte order) at `size` pixels square, filled with `rgb`, clear where there is no mark.
/// At 16 px it is the hand-placed pixel mark; larger, the vector mark sampled 4 by 4 per pixel. The eyes are holes.
pub fn icon(size: i32, rgb: u32) -> ksni::Icon {
    let n = size.max(1) as usize;
    let mut data = Vec::with_capacity(n * n * 4);
    let argb = |a: u8| [a, (rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8];
    if n == PIXEL_SIZE {
        for row in PIXEL_MARK {
            for ch in row.chars() {
                data.extend(argb(if ch == 'X' || ch == 'O' { 255 } else { 0 }));
            }
        }
    } else {
        let shapes = lh_gui::brand::shapes(0., lh_gui::brand::Colours { face: 0, eye: 0, slot: 0 });
        const S: usize = 4;
        for y in 0..n {
            for x in 0..n {
                let mut hits = 0usize;
                for sy in 0..S {
                    for sx in 0..S {
                        let gx = (x as f32 + (sx as f32 + 0.5) / S as f32) * 64. / n as f32;
                        let gy = (y as f32 + (sy as f32 + 0.5) / S as f32) * 64. / n as f32;
                        hits += covered(&shapes, gx, gy) as usize;
                    }
                }
                data.extend(argb((hits * 255 / (S * S)) as u8));
            }
        }
    }
    ksni::Icon { width: size, height: size, data }
}

/// Is the point (64-grid) on the mark: the slot, or the face with both eyes cut out. Shapes are face, near eye,
/// far eye, slot, as `brand::shapes` returns them.
fn covered(shapes: &[(Vec<(f32, f32)>, u32)], x: f32, y: f32) -> bool {
    let inside = |k: usize| inside(&shapes[k].0, x, y);
    inside(3) || (inside(0) && !inside(1) && !inside(2))
}

/// Even-odd point in polygon.
fn inside(poly: &[(f32, f32)], x: f32, y: f32) -> bool {
    let mut hit = false;
    let mut j = poly.len().saturating_sub(1);
    for i in 0..poly.len() {
        let ((xi, yi), (xj, yj)) = (poly[i], poly[j]);
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            hit = !hit;
        }
        j = i;
    }
    hit
}

/// The 16 px pixel mark from `brand/motion-kit/source/brand-geometry.js` (LH_PX): X is the face, O the slot, the
/// rest (including the eyes) clear.
const PIXEL_SIZE: usize = 16;
const PIXEL_MARK: [&str; 16] = [
    ".............OO.",
    "............OOOO",
    "............OOOO",
    ".............OO.",
    "................",
    "...XXXXXXX......",
    "..XXXXXXXXX.....",
    ".XXXXXXXeXXX....",
    ".XXXeeXXeXXX....",
    ".XXXeeXXeXXX....",
    ".XXXeeXXXXXX....",
    ".XXXXXXXXXXX....",
    ".XXXXXXXXXXX....",
    ".XXXXXXXXXXX....",
    "..XXXXXXXXX.....",
    "...XXXXXXX......",
];

struct Tray {
    pick: Pick,
}

impl ksni::Tray for Tray {
    fn id(&self) -> String {
        "lastharness-gui".into()
    }
    fn title(&self) -> String {
        "LastHarness".into()
    }
    fn icon_name(&self) -> String {
        // hosts prefer a name over pixmaps, so the name is empty unless the colour is unknown
        match self.pick {
            Pick::Symbolic => "lastharness-symbolic".into(),
            Pick::Pixmap(_) => String::new(),
        }
    }
    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        match self.pick {
            Pick::Pixmap(rgb) => [16, 22, 32].iter().map(|&s| icon(s, rgb)).collect(),
            Pick::Symbolic => vec![],
        }
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        vec![StandardItem { label: "Quit".into(), activate: Box::new(|_| std::process::exit(0)), ..Default::default() }.into()]
    }
}

static HANDLE: OnceLock<ksni::blocking::Handle<Tray>> = OnceLock::new();
/// What the icon shows now, so an unchanged colour does not reach the host again.
static SHOWN: Mutex<Option<Pick>> = Mutex::new(None);

pub fn spawn() {
    use ksni::blocking::TrayMethods;
    if std::env::var_os("LH_NO_TRAY").is_some() {
        return;
    }
    let first = current_pick();
    *SHOWN.lock().unwrap_or_else(|e| e.into_inner()) = Some(first);
    match (Tray { pick: first }).spawn() {
        Ok(handle) => {
            let _ = HANDLE.set(handle);
        }
        Err(e) => eprintln!("no tray icon: {e}"),
    }
}

/// Re-read the colour and, if it changed, repaint the icon. Reads a small file and, off Omarchy, asks gdbus or
/// gsettings, so the app calls [`refresh`] rather than this on its UI thread.
pub fn sync() {
    let Some(handle) = HANDLE.get() else { return };
    let next = current_pick();
    {
        let mut shown = SHOWN.lock().unwrap_or_else(|e| e.into_inner());
        if *shown == Some(next) {
            return;
        }
        *shown = Some(next);
    }
    handle.update(|t| t.pick = next);
}

/// [`sync`] on a background thread, so the UI never waits on the desktop.
pub fn refresh() {
    if HANDLE.get().is_some() {
        std::thread::spawn(sync);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omarchy_foreground_wins_then_the_system_scheme_then_the_symbolic_name() {
        let fg = Color::parse("#e1e7d9").unwrap();
        assert_eq!(pick(Some(fg), Some(Scheme::Light)), Pick::Pixmap(0xe1e7d9), "the Omarchy colour, whatever the scheme");
        assert_eq!(pick(None, Some(Scheme::Dark)), Pick::Pixmap(LIGHT_ICON), "dark scheme: light icon");
        assert_eq!(pick(None, Some(Scheme::Light)), Pick::Pixmap(DARK_ICON), "light scheme: dark icon");
        assert_eq!(pick(None, None), Pick::Symbolic, "nothing known: the symbolic name, the last resort");
    }

    #[test]
    fn the_portal_and_gsettings_answers_are_read() {
        assert_eq!(parse_portal("(<<uint32 1>>,)\n"), Some(Scheme::Dark));
        assert_eq!(parse_portal("(<<uint32 2>>,)\n"), Some(Scheme::Light));
        assert_eq!(parse_portal("(<<uint32 0>>,)\n"), None, "no preference");
        assert_eq!(parse_gsettings("'prefer-dark'\n"), Some(Scheme::Dark));
        assert_eq!(parse_gsettings("'prefer-light'\n"), Some(Scheme::Light));
        assert_eq!(parse_gsettings("'default'\n"), None);
    }

    #[test]
    fn the_icon_is_square_argb_of_the_requested_size() {
        for size in [16, 22, 32] {
            let i = icon(size, 0x123456);
            assert_eq!((i.width, i.height), (size, size));
            assert_eq!(i.data.len(), (size * size * 4) as usize, "four bytes a pixel");
        }
    }

    #[test]
    fn the_corners_are_clear_and_the_face_is_the_colour() {
        for size in [16i32, 22, 32] {
            let n = size as usize;
            let i = icon(size, 0xabcdef);
            let px = |x: usize, y: usize| &i.data[(y * n + x) * 4..(y * n + x) * 4 + 4];
            assert_eq!(px(0, 0)[0], 0, "top-left is transparent at {size}");
            assert_eq!(px(n - 1, n - 1)[0], 0, "bottom-right is transparent at {size}");
            assert_eq!(px(0, n - 1)[0], 0, "bottom-left is transparent at {size}");
            // a point well inside the face, left of the eyes: (12, 45) on the 64 grid
            let (x, y) = (12 * n / 64, 45 * n / 64);
            assert_eq!(px(x, y), &[255, 0xab, 0xcd, 0xef], "the face at {size} px, one colour");
        }
    }
}
