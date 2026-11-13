//! Project instructions and `@file` references.
//!
//! A chat's folder may hold `AGENTS.md`, `CLAUDE.md` or `.cursorrules`; their text joins the system
//! prompt. A message may name files with `@path`; a file inside the chat folder is added to that
//! message. Paths that leave the folder (absolute, `..`) are never read.

use std::path::{Component, Path};

/// Files read for project instructions, in this order.
pub const INSTRUCTION_FILES: [&str; 3] = ["AGENTS.md", "CLAUDE.md", ".cursorrules"];
/// Largest instruction file read, in bytes. Longer files are cut at a character boundary.
pub const INSTRUCTIONS_MAX: usize = 8_000;
/// Largest referenced file added to one message, in bytes.
pub const REFERENCE_MAX: usize = 20_000;

fn cut(text: &str, max: usize) -> &str {
    if text.len() <= max {
        return text;
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// The instruction files in `folder` as one block, each under a heading. Empty when there are none.
pub fn project_instructions(folder: &Path) -> String {
    let mut out = String::new();
    for name in INSTRUCTION_FILES {
        let Ok(text) = std::fs::read_to_string(folder.join(name)) else { continue };
        if text.trim().is_empty() {
            continue;
        }
        out.push_str(&format!("# Project instructions from {name}\n{}\n\n", cut(text.trim(), INSTRUCTIONS_MAX)));
    }
    out.trim_end().to_string()
}

/// A reference path that stays inside the folder: relative, no `..`, no root.
pub fn safe_relative(path: &str) -> Option<&Path> {
    let p = Path::new(path);
    let ok = !path.is_empty() && p.components().all(|c| matches!(c, Component::Normal(_) | Component::CurDir));
    ok.then_some(p)
}

/// `text` with each `@path` that names a readable file in `folder` followed by that file's text.
/// Unknown or unsafe references stay as written, so nothing is lost.
pub fn expand_references(text: &str, folder: &Path) -> String {
    let mut out = text.to_string();
    let mut added = String::new();
    for word in text.split_whitespace() {
        let Some(path) = word.strip_prefix('@') else { continue };
        let path = path.trim_end_matches([',', '.', ';', ':', ')']);
        let Some(rel) = safe_relative(path) else { continue };
        let Ok(body) = std::fs::read_to_string(folder.join(rel)) else { continue };
        if added.contains(&format!("@{path}\n")) {
            continue;
        }
        added.push_str(&format!("\n\n@{path}\n```\n{}\n```", cut(&body, REFERENCE_MAX)));
    }
    out.push_str(&added);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("lh-proj-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn each_instruction_file_is_in_the_block() {
        let d = dir("instr");
        std::fs::write(d.join("AGENTS.md"), "use tabs").unwrap();
        std::fs::write(d.join(".cursorrules"), "no emoji").unwrap();
        let block = project_instructions(&d);
        assert!(block.contains("# Project instructions from AGENTS.md\nuse tabs"));
        assert!(block.contains("no emoji"));
        assert!(!block.contains("CLAUDE.md"));
        assert_eq!(project_instructions(&dir("none")), "");
    }

    #[test]
    fn long_instructions_are_cut_on_a_char_boundary() {
        let d = dir("long");
        std::fs::write(d.join("CLAUDE.md"), "é".repeat(INSTRUCTIONS_MAX)).unwrap();
        let block = project_instructions(&d);
        assert!(block.len() < INSTRUCTIONS_MAX + 100);
    }

    #[test]
    fn at_reference_adds_the_file_and_unsafe_paths_do_not() {
        let d = dir("refs");
        std::fs::create_dir_all(d.join("src")).unwrap();
        std::fs::write(d.join("src/lib.rs"), "fn main() {}").unwrap();
        std::fs::write(d.join("secret.txt"), "should not leak").unwrap();
        let out = expand_references("look at @src/lib.rs, then @../etc/passwd and @/etc/hosts", &d);
        assert!(out.contains("@src/lib.rs\n```\nfn main() {}\n```"));
        assert!(!out.contains("passwd\n```"));
        assert!(!out.contains("root:"));
        let plain = expand_references("nothing here @missing.rs", &d);
        assert_eq!(plain, "nothing here @missing.rs");
        let twice = expand_references("@src/lib.rs @src/lib.rs", &d);
        assert_eq!(twice.matches("@src/lib.rs\n```").count(), 1);
    }
}
