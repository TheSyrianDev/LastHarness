//! Pure decision logic for the window backdrop: which Windows compositor calls to make for a
//! requested `WindowBackgroundAppearance`, on a given Windows build. Kept free of Win32 calls so
//! it can be unit tested.

use gpui::WindowBackgroundAppearance;

/// First build with `SetWindowCompositionAttribute` accent states that work (Windows 10 1809).
pub(crate) const BUILD_ACCENT: u32 = 17763;
/// First build with `DWMWA_SYSTEMBACKDROP_TYPE` (Windows 11 22H2).
pub(crate) const BUILD_SYSTEM_BACKDROP: u32 = 22621;

/// `ACCENT_DISABLED`.
pub(crate) const ACCENT_DISABLED: u32 = 0;
/// `ACCENT_ENABLE_TRANSPARENTGRADIENT`.
pub(crate) const ACCENT_TRANSPARENT: u32 = 2;
/// `ACCENT_ENABLE_ACRYLICBLURBEHIND`.
pub(crate) const ACCENT_ACRYLIC: u32 = 4;

/// `DWMSBT_NONE`.
pub(crate) const BACKDROP_NONE: u32 = 1;
/// `DWMSBT_MAINWINDOW` (Mica).
pub(crate) const BACKDROP_MICA: u32 = 2;
/// `DWMSBT_TRANSIENTWINDOW` (Acrylic).
pub(crate) const BACKDROP_ACRYLIC: u32 = 3;
/// `DWMSBT_TABBEDWINDOW` (Mica Alt).
pub(crate) const BACKDROP_MICA_ALT: u32 = 4;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BackdropPlan {
    /// Accent state for `SetWindowCompositionAttribute`; only applied on `BUILD_ACCENT` or later.
    pub accent_state: u32,
    /// Value for `DWMWA_SYSTEMBACKDROP_TYPE`, when the build has it.
    pub system_backdrop: Option<u32>,
    /// Extend the DWM frame over the whole client area, so a system backdrop shows behind it.
    pub extend_frame: bool,
    /// What the window can really show; the renderer clears for this one.
    pub effective: WindowBackgroundAppearance,
}

/// Decide the compositor calls. `composition` is false when the window does not render through
/// DirectComposition: its swap chain then has no alpha, so the only honest result is opaque.
pub(crate) fn backdrop_plan(
    requested: WindowBackgroundAppearance,
    build: u32,
    composition: bool,
) -> BackdropPlan {
    use WindowBackgroundAppearance as A;
    let has_system_backdrop = build >= BUILD_SYSTEM_BACKDROP;
    let reset_backdrop = has_system_backdrop.then_some(BACKDROP_NONE);
    let plain = |accent_state, effective| BackdropPlan {
        accent_state,
        system_backdrop: reset_backdrop,
        extend_frame: false,
        effective,
    };
    let system = |backdrop, effective| BackdropPlan {
        accent_state: ACCENT_DISABLED,
        system_backdrop: Some(backdrop),
        extend_frame: true,
        effective,
    };

    if !composition {
        return plain(ACCENT_DISABLED, A::Opaque);
    }
    match requested {
        A::Opaque => plain(ACCENT_DISABLED, A::Opaque),
        A::Transparent => plain(ACCENT_TRANSPARENT, A::Transparent),
        A::Blurred | A::MicaBackdrop | A::MicaAltBackdrop if has_system_backdrop => {
            let backdrop = match requested {
                A::MicaBackdrop => BACKDROP_MICA,
                A::MicaAltBackdrop => BACKDROP_MICA_ALT,
                _ => BACKDROP_ACRYLIC,
            };
            system(backdrop, requested)
        }
        // Windows 10: the acrylic accent is the only blur on offer.
        A::Blurred | A::MicaBackdrop | A::MicaAltBackdrop if build >= BUILD_ACCENT => {
            plain(ACCENT_ACRYLIC, A::Blurred)
        }
        // Too old for any blur: plain see-through is still possible.
        _ => plain(ACCENT_TRANSPARENT, A::Transparent),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::WindowBackgroundAppearance as A;

    #[test]
    fn no_composition_is_always_opaque() {
        for a in [A::Opaque, A::Transparent, A::Blurred, A::MicaBackdrop] {
            let p = backdrop_plan(a, 26100, false);
            assert_eq!(p.effective, A::Opaque);
            assert_eq!(p.accent_state, ACCENT_DISABLED);
            assert!(!p.extend_frame);
        }
    }

    #[test]
    fn windows_11_blur_uses_system_acrylic() {
        let p = backdrop_plan(A::Blurred, 22621, true);
        assert_eq!(p.system_backdrop, Some(BACKDROP_ACRYLIC));
        assert_eq!(p.accent_state, ACCENT_DISABLED);
        assert!(p.extend_frame);
        assert_eq!(
            backdrop_plan(A::MicaBackdrop, 26100, true).system_backdrop,
            Some(BACKDROP_MICA)
        );
    }

    #[test]
    fn windows_10_blur_uses_acrylic_accent() {
        let p = backdrop_plan(A::Blurred, 19045, true);
        assert_eq!(p.accent_state, ACCENT_ACRYLIC);
        assert_eq!(p.system_backdrop, None);
        assert_eq!(
            backdrop_plan(A::MicaBackdrop, 19045, true).effective,
            A::Blurred
        );
    }

    #[test]
    fn leaving_a_backdrop_resets_it() {
        let p = backdrop_plan(A::Transparent, 22621, true);
        assert_eq!(p.system_backdrop, Some(BACKDROP_NONE));
        assert!(!p.extend_frame);
        assert_eq!(p.accent_state, ACCENT_TRANSPARENT);
    }

    #[test]
    fn old_windows_blur_falls_back_to_transparent() {
        assert_eq!(
            backdrop_plan(A::Blurred, 17134, true).effective,
            A::Transparent
        );
    }
}
