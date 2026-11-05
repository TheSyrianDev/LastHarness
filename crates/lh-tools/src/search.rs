//! `grep` and `glob`: search the project, skipping what .gitignore leaves out and hidden folders.

use std::path::Path;

use serde_json::Value;

use crate::ToolError;

/// Most lines or paths returned, so a broad search does not flood the model.
const MAX_HITS: usize = 200;
/// Files bigger than this are skipped by grep, so one huge file cannot fill memory.
const MAX_FILE: u64 = 4 * 1024 * 1024;

fn walker(start: &Path) -> ignore::Walk {
    ignore::WalkBuilder::new(start).require_git(false).build()
}

fn rel(p: &Path, root: &Path) -> String {
    p.strip_prefix(root).unwrap_or(p).display().to_string()
}

fn more(n: usize) -> String {
    format!("\n[more than {MAX_HITS} results; narrow the search]").repeat(usize::from(n > MAX_HITS))
}

fn file_glob(args: &Value) -> Result<Option<globset::GlobMatcher>, ToolError> {
    match args["glob"].as_str().filter(|g| !g.is_empty()) {
        None => Ok(None),
        Some(g) => globset::GlobBuilder::new(g).literal_separator(false).build().map(|g| Some(g.compile_matcher())).map_err(|e| ToolError::Args(format!("bad `glob`: {e}"))),
    }
}

pub(crate) fn grep(start: &Path, root: &Path, args: &Value) -> Result<String, ToolError> {
    let pat = args["pattern"].as_str().ok_or_else(|| ToolError::Args("`pattern` must be a string".into()))?;
    let ic = args["ignore_case"].as_bool().or_else(|| args["-i"].as_bool()).unwrap_or(false);
    let mode = args["output_mode"].as_str().unwrap_or("content");
    if !matches!(mode, "content" | "files_with_matches" | "count") {
        return Err(ToolError::Args("`output_mode` must be content, files_with_matches or count".into()));
    }
    let re = regex::RegexBuilder::new(pat).case_insensitive(ic).build().map_err(|e| ToolError::Args(format!("bad `pattern`: {e}")))?;
    let only = file_glob(args)?;
    let mut out = Vec::new();
    let mut n = 0;
    for e in walker(start).flatten() {
        if !e.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        if let Some(g) = &only {
            if !g.is_match(e.path().file_name().unwrap_or_default()) && !g.is_match(e.path().strip_prefix(start).unwrap_or(e.path())) {
                continue;
            }
        }
        if e.metadata().map_or(true, |m| m.len() > MAX_FILE) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(e.path()) else { continue }; // binary or unreadable
        let mut file_hits = 0;
        for (i, line) in text.lines().enumerate() {
            if re.is_match(line) {
                file_hits += 1;
                if mode == "content" {
                    n += 1;
                    if n <= MAX_HITS {
                        out.push(format!("{}:{}:{}", rel(e.path(), root), i + 1, line.chars().take(300).collect::<String>()));
                    }
                } else if mode == "files_with_matches" {
                    break;
                }
            }
        }
        if file_hits > 0 && mode != "content" {
            n += 1;
            if n <= MAX_HITS {
                out.push(if mode == "count" { format!("{}:{file_hits}", rel(e.path(), root)) } else { rel(e.path(), root) });
            }
        }
    }
    if out.is_empty() {
        return Ok("no matches".into());
    }
    Ok(out.join("\n") + &more(n))
}

pub(crate) fn glob(start: &Path, root: &Path, args: &Value) -> Result<String, ToolError> {
    let pat = args["pattern"].as_str().ok_or_else(|| ToolError::Args("`pattern` must be a string".into()))?;
    let g = globset::GlobBuilder::new(pat).literal_separator(true).build().map_err(|e| ToolError::Args(format!("bad `pattern`: {e}")))?.compile_matcher();
    let bare = !pat.contains('/');
    let mut out: Vec<String> = walker(start)
        .flatten()
        .filter(|e| e.depth() > 0)
        .filter(|e| {
            let r = e.path().strip_prefix(start).unwrap_or(e.path());
            g.is_match(r) || (bare && g.is_match(e.path().file_name().unwrap_or_default()))
        })
        .map(|e| rel(e.path(), root))
        .collect();
    out.sort();
    let n = out.len();
    out.truncate(MAX_HITS);
    if out.is_empty() {
        return Ok("no files match".into());
    }
    Ok(out.join("\n") + &more(n))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tree() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("src/deep")).unwrap();
        std::fs::write(d.path().join("src/a.rs"), "fn main() {}\nlet x = 1;\n").unwrap();
        std::fs::write(d.path().join("src/deep/b.rs"), "fn helper() {}\n").unwrap();
        std::fs::write(d.path().join("notes.md"), "Fn in caps\n").unwrap();
        std::fs::write(d.path().join(".gitignore"), "skip/\n").unwrap();
        std::fs::create_dir_all(d.path().join("skip")).unwrap();
        std::fs::write(d.path().join("skip/c.rs"), "fn ignored() {}\n").unwrap();
        d
    }

    #[test]
    fn grep_finds_lines_and_respects_gitignore_and_globs() {
        let d = tree();
        let r = grep(d.path(), d.path(), &json!({"pattern": "^fn "})).unwrap();
        assert!(r.contains("src/a.rs:1:fn main() {}") && r.contains("src/deep/b.rs:1:fn helper"), "{r}");
        assert!(!r.contains("ignored"), "gitignored files are skipped: {r}");
        let r = grep(d.path(), d.path(), &json!({"pattern": "fn", "glob": "*.md", "ignore_case": true})).unwrap();
        assert_eq!(r, "notes.md:1:Fn in caps");
        assert_eq!(grep(d.path(), d.path(), &json!({"pattern": "nothing here"})).unwrap(), "no matches");
        assert!(matches!(grep(d.path(), d.path(), &json!({"pattern": "("})), Err(ToolError::Args(_))));
        let mut files: Vec<_> = grep(d.path(), d.path(), &json!({"pattern": "fn", "-i": true, "output_mode": "files_with_matches"})).unwrap().lines().map(String::from).collect();
        files.sort();
        assert_eq!(files, ["notes.md", "src/a.rs", "src/deep/b.rs"]);
        assert_eq!(grep(d.path(), d.path(), &json!({"pattern": "let", "output_mode": "count"})).unwrap(), "src/a.rs:1");
        assert!(matches!(grep(d.path(), d.path(), &json!({"pattern": "x", "output_mode": "bogus"})), Err(ToolError::Args(_))));
    }

    #[test]
    fn glob_matches_names_and_paths() {
        let d = tree();
        assert_eq!(glob(d.path(), d.path(), &json!({"pattern": "**/*.rs"})).unwrap(), "src/a.rs\nsrc/deep/b.rs");
        assert_eq!(glob(d.path(), d.path(), &json!({"pattern": "*.rs"})).unwrap(), "src/a.rs\nsrc/deep/b.rs", "a bare pattern matches file names anywhere");
        assert_eq!(glob(d.path(), d.path(), &json!({"pattern": "src/*.rs"})).unwrap(), "src/a.rs");
        assert_eq!(glob(&d.path().join("src"), d.path(), &json!({"pattern": "*.md"})).unwrap(), "no files match");
    }
}
