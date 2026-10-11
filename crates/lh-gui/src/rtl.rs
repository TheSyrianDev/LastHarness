//! Right-to-left text and layout. GPUI shapes Arabic and orders mixed lines correctly but leaves the
//! direction of paragraphs and of the whole layout to the app, so the app decides both here.

/// Is this character from a right-to-left script (Hebrew, Arabic, Syriac, Thaana, N'Ko, and their
/// presentation forms)?
pub fn is_rtl_char(c: char) -> bool {
    matches!(c as u32, 0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF | 0x10800..=0x10FFF | 0x1E800..=0x1EFFF)
}

/// A paragraph runs right to left when its first letter does. Digits, spaces, punctuation and markup
/// characters are skipped, as the Unicode bidi rules do for the paragraph's base direction.
pub fn paragraph_is_rtl(text: &str) -> bool {
    for c in text.chars() {
        if c.is_alphabetic() {
            return is_rtl_char(c);
        }
    }
    false
}

/// How the whole layout is mirrored.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    /// Follow the language of the desktop.
    #[default]
    Auto,
    Ltr,
    Rtl,
}

impl Direction {
    pub fn is_rtl(self, locale: &str) -> bool {
        match self {
            Direction::Ltr => false,
            Direction::Rtl => true,
            Direction::Auto => locale_is_rtl(locale),
        }
    }

    pub fn next(self) -> Direction {
        match self {
            Direction::Auto => Direction::Rtl,
            Direction::Rtl => Direction::Ltr,
            Direction::Ltr => Direction::Auto,
        }
    }
}

/// `ar_EG.UTF-8`, `he`, `fa_IR` and `ur_PK` are right to left.
pub fn locale_is_rtl(locale: &str) -> bool {
    let lang = locale.split(['_', '.', '@', '-']).next().unwrap_or("").to_ascii_lowercase();
    matches!(lang.as_str(), "ar" | "he" | "iw" | "fa" | "ur" | "ps" | "sd" | "ug" | "yi" | "dv" | "ckb")
}

/// The desktop's language as the environment gives it.
pub fn desktop_locale() -> String {
    for k in ["LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE"] {
        if let Ok(v) = std::env::var(k) {
            if !v.is_empty() && v != "C" && v != "POSIX" {
                return v;
            }
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_paragraph_follows_its_first_letter() {
        assert!(paragraph_is_rtl("مرحبا بالعالم"));
        assert!(paragraph_is_rtl("שלום עולם"));
        assert!(!paragraph_is_rtl("Hello مرحبا"));
        assert!(paragraph_is_rtl("123 - مرحبا hello"), "digits and punctuation do not decide");
        assert!(paragraph_is_rtl("«مرحبا»"));
        assert!(!paragraph_is_rtl(""));
        assert!(!paragraph_is_rtl("12345"));
        assert!(!paragraph_is_rtl("Mixed: النص العربي"));
        assert!(paragraph_is_rtl("ﻣﺮﺣﺒﺎ"), "presentation forms count");
    }

    #[test]
    fn locales_decide_the_automatic_direction() {
        for l in ["ar_EG.UTF-8", "ar", "he_IL", "fa_IR.UTF-8", "ur_PK", "ar-SA"] {
            assert!(locale_is_rtl(l), "{l}");
        }
        for l in ["en_US.UTF-8", "de_DE", "", "C", "fr"] {
            assert!(!locale_is_rtl(l), "{l}");
        }
        assert!(Direction::Auto.is_rtl("ar_EG.UTF-8"));
        assert!(!Direction::Auto.is_rtl("en_US"));
        assert!(Direction::Rtl.is_rtl("en_US"), "the user can force it");
        assert!(!Direction::Ltr.is_rtl("ar_EG"));
    }

    #[test]
    fn the_direction_switch_cycles() {
        assert_eq!(Direction::Auto.next().next().next(), Direction::Auto);
    }
}
