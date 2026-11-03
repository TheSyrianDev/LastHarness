//! Fonts for Latin and Arabic text. The Latin font is the main font of the window. The Arabic font is its explicit
//! fallback: GPUI (cosmic-text) picks, grapheme by grapheme, the first font in the chain that has the glyph, so every
//! Arabic-script run is drawn by one Arabic font and Latin stays in the Latin font. No window code here.

use std::ops::Range;

/// Wanted for Arabic when the person has not chosen one, best first.
pub const ARABIC_DEFAULTS: [&str; 2] = ["Noto Naskh Arabic", "Noto Sans Arabic"];

/// The family names `fc-list` knows, one entry per family, sorted. Empty when fontconfig is not there.
pub fn installed_families() -> Vec<String> {
    let Some(o) = std::process::Command::new("fc-list").args([":", "family"]).output().ok().filter(|o| o.status.success()) else { return Vec::new() };
    parse_families(&String::from_utf8_lossy(&o.stdout))
}

/// `fc-list : family` prints `Name,Other name` per face. Take the first name of each line, once.
pub fn parse_families(out: &str) -> Vec<String> {
    let mut v: Vec<String> = out.lines().filter_map(|l| l.split(',').next()).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    v.sort_by_key(|s| s.to_lowercase());
    v.dedup();
    v
}

/// Families worth offering for Arabic: the ones whose name says so first, then the rest.
pub fn arabic_first(all: &[String]) -> Vec<String> {
    let says_arabic = |f: &String| {
        let l = f.to_lowercase();
        l.contains("arab") || l.contains("naskh") || l.contains("kufi")
    };
    let (a, b): (Vec<_>, Vec<_>) = all.iter().cloned().partition(says_arabic);
    a.into_iter().chain(b).collect()
}

fn has_family(installed: &[String], n: &str) -> bool {
    installed.iter().any(|f| f.eq_ignore_ascii_case(n))
}

/// The first of a theme's wanted families that is installed.
pub fn first_installed(wanted: &[String], installed: &[String]) -> Option<String> {
    wanted.iter().find_map(|n| installed.iter().find(|f| f.eq_ignore_ascii_case(n)).cloned())
}

/// The Arabic font to use. Order: the one chosen in Settings if installed, then the theme's first installed one, then
/// Noto Naskh Arabic, then Noto Sans Arabic, else none (the system's own fallback then does the work).
pub fn arabic_font(chosen: &str, theme: &[String], installed: &[String]) -> Option<String> {
    if !chosen.is_empty() && has_family(installed, chosen) {
        return Some(chosen.to_string());
    }
    first_installed(theme, installed).or_else(|| ARABIC_DEFAULTS.iter().find(|n| has_family(installed, n)).map(|n| n.to_string()))
}

/// The Latin font to set on the window: the choice in Settings, else the theme's first installed one, else none (the
/// default of the window). A choice is kept even when it is not installed, as before.
pub fn latin_font(chosen: &str, theme: &[String], installed: &[String]) -> Option<String> {
    if !chosen.is_empty() {
        return Some(chosen.to_string());
    }
    first_installed(theme, installed)
}

/// The text of the picker's empty entry when the theme sets a font: "Theme default (Noto Serif)".
pub fn theme_default_label(font: &str, rtl: bool) -> String {
    if rtl {
        format!("افتراضي السمة ({font})")
    } else {
        format!("Theme default ({font})")
    }
}

/// The font for code blocks: the theme's first installed one, else DejaVu Sans Mono.
pub fn mono_font(theme: &[String], installed: &[String]) -> String {
    first_installed(theme, installed).unwrap_or_else(|| "DejaVu Sans Mono".to_string())
}

/// Letters of the Arabic script, with the marks and presentation forms that belong to a word.
pub fn is_arabic_script(c: char) -> bool {
    matches!(c as u32, 0x0600..=0x06FF | 0x0750..=0x077F | 0x08A0..=0x08FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF | 0x10EC0..=0x10EFF | 0x1EE00..=0x1EEFF)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Script {
    Arabic,
    Other,
}

/// Split text into runs of Arabic script and everything else. Spaces, digits and punctuation between two Arabic
/// runs stay in the Arabic run, so one Arabic phrase is one run; at the edges they go to the other side.
pub fn script_runs(text: &str) -> Vec<(Range<usize>, Script)> {
    let mut runs: Vec<(Range<usize>, Script)> = Vec::new();
    for (i, c) in text.char_indices() {
        let end = i + c.len_utf8();
        let s = if is_arabic_script(c) { Script::Arabic } else { Script::Other };
        match runs.last_mut() {
            Some((r, last)) if *last == s => r.end = end,
            _ => runs.push((i..end, s)),
        }
    }
    let mut out: Vec<(Range<usize>, Script)> = Vec::new();
    for (ix, (r, s)) in runs.iter().enumerate() {
        let between = *s == Script::Other && text[r.clone()].chars().all(|c| !c.is_alphabetic()) && ix > 0 && ix + 1 < runs.len();
        let s = if between { Script::Arabic } else { *s };
        match out.last_mut() {
            Some((o, last)) if *last == s => o.end = r.end,
            _ => out.push((r.clone(), s)),
        }
    }
    out
}


// ---- fonts the person adds ------------------------------------------------------------------------------------

use std::path::{Path, PathBuf};

/// Where added fonts are kept: `$XDG_DATA_HOME/fonts/lastharness`, else `~/.local/share/fonts/lastharness`.
pub fn user_font_dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()).map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))?;
    Some(base.join("fonts").join("lastharness"))
}

/// The family names inside font bytes, or why the bytes are not a font. `ext` is the lowercase file extension.
pub fn validate_font(bytes: &[u8], ext: &str) -> Result<Vec<String>, String> {
    match ext {
        "ttf" | "otf" | "ttc" | "otc" => {}
        "woff2" | "woff" => return Err("Web fonts (.woff, .woff2) are not supported. Use a .ttf, .otf or .ttc file.".into()),
        _ => return Err("That file type is not a font. Use a .ttf, .otf or .ttc file.".into()),
    }
    let count = ttf_parser::fonts_in_collection(bytes).unwrap_or(1);
    let mut names = Vec::new();
    for i in 0..count {
        let face = ttf_parser::Face::parse(bytes, i).map_err(|_| "That file is not a readable font.".to_string())?;
        let pick = |id: u16| face.names().into_iter().filter(|n| n.name_id == id).find_map(|n| n.to_string());
        let fam = pick(16).or_else(|| pick(1)).ok_or_else(|| "The font has no family name.".to_string())?;
        if !names.contains(&fam) {
            names.push(fam);
        }
    }
    Ok(names)
}

/// Check a font file and copy it into `dir`. Returns the stored path, the family names and the bytes.
pub fn add_font_file(src: &Path, dir: &Path) -> Result<(PathBuf, Vec<String>, Vec<u8>), String> {
    let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let bytes = std::fs::read(src).map_err(|e| format!("Could not read the file: {e}"))?;
    let names = validate_font(&bytes, &ext)?;
    let stem: String = src.file_stem().and_then(|s| s.to_str()).unwrap_or("font").chars().map(|c| if c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | ' ') { c } else { '_' }).collect();
    std::fs::create_dir_all(dir).map_err(|e| format!("Could not make the fonts folder: {e}"))?;
    let dest = dir.join(format!("{stem}.{ext}"));
    std::fs::write(&dest, &bytes).map_err(|e| format!("Could not save the font: {e}"))?;
    Ok((dest, names, bytes))
}

/// The font files added so far, sorted.
pub fn user_font_files(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.extension().and_then(|e| e.to_str()).is_some_and(|e| matches!(e.to_lowercase().as_str(), "ttf" | "otf" | "ttc" | "otc"))).collect();
    v.sort();
    v
}

/// Delete one added font. Only files inside `dir` can go.
pub fn remove_font_file(file: &Path, dir: &Path) -> Result<(), String> {
    if file.parent() != Some(dir) {
        return Err("That font is not one you added.".into());
    }
    std::fs::remove_file(file).map_err(|e| format!("Could not remove the font: {e}"))
}

/// Tell fontconfig about a folder so the new fonts show in `installed_families`. Silent when fc-cache is missing.
pub fn refresh_cache(dir: &Path) {
    let _ = std::process::Command::new("fc-cache").arg(dir).output();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn families_are_parsed_once_each_and_sorted() {
        let out = "Noto Sans Arabic,Noto Sans Arabic Light\nDejaVu Sans\nNoto Sans Arabic\n\nabc,x\n";
        assert_eq!(parse_families(out), vec!["abc", "DejaVu Sans", "Noto Sans Arabic"]);
    }

    #[test]
    fn arabic_font_prefers_the_choice_then_naskh_then_sans() {
        let all: Vec<String> = ["DejaVu Sans", "Noto Sans Arabic", "Noto Kufi Arabic"].map(String::from).to_vec();
        assert_eq!(arabic_font("", &[], &all).as_deref(), Some("Noto Sans Arabic"), "naskh missing");
        assert_eq!(arabic_font("Noto Kufi Arabic", &[], &all).as_deref(), Some("Noto Kufi Arabic"));
        assert_eq!(arabic_font("Gone", &[], &all).as_deref(), Some("Noto Sans Arabic"), "a chosen font that is not installed");
        let both: Vec<String> = ["Noto Naskh Arabic", "Noto Sans Arabic"].map(String::from).to_vec();
        assert_eq!(arabic_font("", &[], &both).as_deref(), Some("Noto Naskh Arabic"));
        assert_eq!(arabic_font("", &[], &["DejaVu Sans".to_string()]), None);
    }

    #[test]
    fn a_font_chosen_in_settings_beats_the_theme_which_beats_the_default() {
        let inst: Vec<String> = ["Noto Sans", "Noto Serif", "Noto Naskh Arabic", "Noto Sans Arabic", "Noto Kufi Arabic"].map(String::from).to_vec();
        let th = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(latin_font("Mine", &th(&["Noto Serif"]), &inst).as_deref(), Some("Mine"), "settings win");
        assert_eq!(latin_font("", &th(&["Missing", "noto serif"]), &inst).as_deref(), Some("Noto Serif"), "first installed of the theme");
        assert_eq!(latin_font("", &th(&["Missing"]), &inst), None, "nothing installed: the default");
        assert_eq!(latin_font("", &[], &inst), None);
        assert_eq!(arabic_font("Noto Kufi Arabic", &th(&["Noto Sans Arabic"]), &inst).as_deref(), Some("Noto Kufi Arabic"));
        assert_eq!(arabic_font("", &th(&["Noto Sans Arabic"]), &inst).as_deref(), Some("Noto Sans Arabic"), "theme over default");
        assert_eq!(arabic_font("Gone", &th(&["Noto Sans Arabic"]), &inst).as_deref(), Some("Noto Sans Arabic"), "a missing choice falls to the theme");
        assert_eq!(arabic_font("", &th(&["Missing"]), &inst).as_deref(), Some("Noto Naskh Arabic"), "then the default");
        assert_eq!(theme_default_label("Noto Serif", false), "Theme default (Noto Serif)");
        assert_eq!(mono_font(&th(&["Missing"]), &inst), "DejaVu Sans Mono");
        assert_eq!(mono_font(&th(&["Noto Sans"]), &inst), "Noto Sans");
    }

    #[test]
    fn arabic_families_are_listed_first() {
        let all: Vec<String> = ["A Serif", "Noto Naskh Arabic", "Z Sans"].map(String::from).to_vec();
        assert_eq!(arabic_first(&all)[0], "Noto Naskh Arabic");
        assert_eq!(arabic_first(&all).len(), 3);
    }

    #[test]
    fn text_splits_into_arabic_and_other_runs() {
        let t = "Hi مرحبا بك (ok) x";
        let runs = script_runs(t);
        let got: Vec<(&str, Script)> = runs.iter().map(|(r, s)| (&t[r.clone()], *s)).collect();
        assert_eq!(got, vec![("Hi ", Script::Other), ("مرحبا بك", Script::Arabic), (" (ok) x", Script::Other)]);
        assert!(script_runs("").is_empty());
        assert_eq!(script_runs("abc").len(), 1);
        assert_eq!(runs.iter().map(|(r, _)| r.len()).sum::<usize>(), t.len(), "no gaps");
    }

    #[test]
    fn a_phrase_with_a_space_and_a_number_between_arabic_words_is_one_run() {
        let runs = script_runs("واحد 2 اثنان");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].1, Script::Arabic);
    }

    #[test]
    fn only_real_fonts_are_taken() {
        assert!(validate_font(b"not a font", "ttf").is_err());
        assert!(validate_font(b"x", "txt").unwrap_err().contains("not a font"));
        assert!(validate_font(b"x", "woff2").unwrap_err().contains("Web fonts"));
        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("bad.ttf");
        std::fs::write(&bad, b"junk").unwrap();
        assert!(add_font_file(&bad, &dir.path().join("out")).is_err());
        assert!(user_font_files(&dir.path().join("out")).is_empty());
    }

    #[test]
    fn a_real_system_font_is_copied_listed_and_removed() {
        let Some(src) = std::process::Command::new("fc-list").args(["-f", "%{file}\\n"]).output().ok().and_then(|o| String::from_utf8(o.stdout).ok()).and_then(|t| t.lines().find(|l| l.ends_with(".ttf")).map(PathBuf::from)) else { return };
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("fonts");
        let (dest, names, bytes) = add_font_file(&src, &out).unwrap();
        assert!(!names.is_empty() && !bytes.is_empty());
        assert_eq!(user_font_files(&out), vec![dest.clone()]);
        assert!(remove_font_file(&src, &out).is_err(), "files outside the folder stay");
        remove_font_file(&dest, &out).unwrap();
        assert!(user_font_files(&out).is_empty());
    }
}
