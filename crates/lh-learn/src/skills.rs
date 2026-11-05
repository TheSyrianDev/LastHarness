//! Skills in the open Agent Skills format: a folder with a required `SKILL.md` (a header with `name`
//! and `description`, then Markdown instructions) and optional `scripts/`, `references/` and `assets/`.
//! The folders are the real thing and stay usable by any tool that reads the format. Every version is
//! also kept in the learning history, so a skill can be rolled back, and a skill only changes through a
//! suggestion the user accepts.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::yaml::{self, Value as Y};
use crate::{Error, Kind, Learn, Result};

pub const NAME_MAX: usize = 64;
pub const DESCRIPTION_MAX: usize = 1024;
pub const COMPATIBILITY_MAX: usize = 500;
const FILE_MAX: u64 = 256 * 1024;
const SKILL_TOTAL_MAX: usize = 512 * 1024;

#[derive(Clone, Debug, PartialEq)]
pub struct SkillMeta {
    pub name: String,
    pub description: String,
    pub license: Option<String>,
    pub compatibility: Option<String>,
    /// Experimental in the format: tools the skill is meant to use.
    pub allowed_tools: Vec<String>,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Skill {
    pub meta: SkillMeta,
    /// The instructions: everything after the header.
    pub body: String,
    pub dir: PathBuf,
    /// Other files in the folder, relative to it (not `SKILL.md`).
    pub files: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SkillEntry {
    pub name: String,
    pub description: String,
    pub error: Option<String>,
}

pub fn valid_name(n: &str) -> bool {
    !n.is_empty() && n.len() <= NAME_MAX && !n.starts_with('-') && !n.ends_with('-') && !n.contains("--") && n.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Read and check a `SKILL.md`. `folder` is the name of the folder it sits in; the format requires it to equal `name`.
pub fn parse_skill_md(text: &str, folder: Option<&str>) -> std::result::Result<(SkillMeta, String), String> {
    let text = text.replace("\r\n", "\n");
    let rest = text.strip_prefix("---\n").ok_or("SKILL.md must start with a header between --- lines")?;
    let end = rest.find("\n---\n").map(|i| (i, 5)).or_else(|| rest.strip_suffix("\n---").map(|r| (r.len(), 4))).ok_or("the header is not closed with ---")?;
    let header = yaml::parse(&rest[..end.0])?;
    let body = rest.get(end.0 + end.1..).unwrap_or("").trim().to_string();
    let get = |k: &str| header.iter().find(|(x, _)| x == k).map(|(_, v)| v);
    let s = |k: &str| match get(k) {
        Some(Y::Str(v)) if !v.trim().is_empty() => Ok(Some(v.trim().to_string())),
        Some(Y::Str(_)) | None => Ok(None),
        Some(_) => Err(format!("{k} must be text")),
    };
    let name = s("name")?.ok_or("the header needs a name")?;
    if !valid_name(&name) {
        return Err(format!("the name \"{name}\" is not valid: use lower case letters, digits and single hyphens, at most {NAME_MAX} characters, not starting or ending with a hyphen"));
    }
    if folder.is_some_and(|f| f != name) {
        return Err(format!("the name \"{name}\" must match its folder \"{}\"", folder.unwrap_or("")));
    }
    let description = s("description")?.ok_or("the header needs a description (what it does and when to use it)")?;
    if description.chars().count() > DESCRIPTION_MAX {
        return Err(format!("the description is longer than {DESCRIPTION_MAX} characters"));
    }
    let compatibility = s("compatibility")?;
    if compatibility.as_ref().is_some_and(|c| c.chars().count() > COMPATIBILITY_MAX) {
        return Err(format!("compatibility is longer than {COMPATIBILITY_MAX} characters"));
    }
    let allowed_tools = match get("allowed-tools") {
        Some(Y::Str(v)) => v.split_whitespace().map(str::to_string).collect(),
        Some(Y::List(v)) => v.clone(),
        None => vec![],
        Some(_) => return Err("allowed-tools must be a list or a line of names".into()),
    };
    let metadata = match get("metadata") {
        Some(Y::Map(m)) => m.clone(),
        None => BTreeMap::new(),
        Some(_) => return Err("metadata must be a map of names to text".into()),
    };
    Ok((SkillMeta { name, description, license: s("license")?, compatibility, allowed_tools, metadata }, body))
}

fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"))
}

/// Write a `SKILL.md` text from its parts, in a header that any YAML reader accepts.
pub fn render_skill_md(meta: &SkillMeta, body: &str) -> String {
    let mut h = format!("---\nname: {}\ndescription: {}\n", meta.name, quote(&meta.description));
    if let Some(l) = &meta.license {
        h.push_str(&format!("license: {}\n", quote(l)));
    }
    if let Some(c) = &meta.compatibility {
        h.push_str(&format!("compatibility: {}\n", quote(c)));
    }
    if !meta.allowed_tools.is_empty() {
        h.push_str(&format!("allowed-tools: {}\n", quote(&meta.allowed_tools.join(" "))));
    }
    if !meta.metadata.is_empty() {
        h.push_str("metadata:\n");
        for (k, v) in &meta.metadata {
            h.push_str(&format!("  {k}: {}\n", quote(v)));
        }
    }
    format!("{h}---\n\n{}\n", body.trim())
}

/// A path inside a skill folder: relative, no `..`, no hidden path parts.
fn safe_rel(p: &str) -> bool {
    let path = Path::new(p);
    !p.is_empty() && path.is_relative() && path.components().all(|c| matches!(c, std::path::Component::Normal(n) if !n.to_string_lossy().starts_with('.')))
}

impl Learn {
    pub fn with_skills_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.skills_dir = Some(dir.into());
        self
    }

    fn skills_root(&self) -> Result<&Path> {
        self.skills_dir.as_deref().ok_or_else(|| Error::Invalid("skills are not set up (no skills folder)".into()))
    }

    /// Every skill folder, valid or not. Invalid ones carry the reason and are not offered to a model.
    pub fn list_skills(&self) -> Result<Vec<SkillEntry>> {
        let root = self.skills_root()?;
        let Ok(rd) = std::fs::read_dir(root) else { return Ok(vec![]) };
        let mut out = vec![];
        for e in rd.flatten().filter(|e| e.path().is_dir()) {
            let folder = e.file_name().to_string_lossy().to_string();
            if folder.starts_with('.') {
                continue;
            }
            match std::fs::read_to_string(e.path().join("SKILL.md")).map_err(|e| e.to_string()).and_then(|t| parse_skill_md(&t, Some(&folder))) {
                Ok((m, _)) => out.push(SkillEntry { name: m.name, description: m.description, error: None }),
                Err(err) => out.push(SkillEntry { name: folder, description: String::new(), error: Some(err) }),
            }
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    pub fn load_skill(&self, name: &str) -> Result<Skill> {
        if !valid_name(name) {
            return Err(Error::NotFound(format!("skill {name}")));
        }
        let dir = self.skills_root()?.join(name);
        let text = std::fs::read_to_string(dir.join("SKILL.md")).map_err(|_| Error::NotFound(format!("skill {name}")))?;
        let (meta, body) = parse_skill_md(&text, Some(name)).map_err(Error::Invalid)?;
        let mut files = vec![];
        collect(&dir, &dir, &mut files);
        files.retain(|f| f != "SKILL.md");
        files.sort();
        Ok(Skill { meta, body, dir, files })
    }

    /// Read one file of a skill (a reference document, a script), for the model to look at.
    pub fn read_skill_file(&self, name: &str, rel: &str) -> Result<String> {
        let skill = self.load_skill(name)?;
        if !safe_rel(rel) || !skill.files.iter().any(|f| f == rel) {
            return Err(Error::NotFound(format!("file {rel} in skill {name}")));
        }
        let p = skill.dir.join(rel);
        if std::fs::metadata(&p)?.len() > FILE_MAX {
            return Err(Error::Invalid("the file is larger than 256 KB".into()));
        }
        String::from_utf8(std::fs::read(&p)?).map_err(|_| Error::Invalid("the file is not text".into()))
    }

    /// Write a skill's files as a new version. `files` maps relative paths to text and must hold `SKILL.md`.
    pub fn install_skill(&self, files: &BTreeMap<String, String>, op: &str, by: &str, note: &str) -> Result<String> {
        let md = files.get("SKILL.md").ok_or_else(|| Error::Invalid("a skill needs a SKILL.md".into()))?;
        let (meta, _) = parse_skill_md(md, None).map_err(Error::Invalid)?;
        if let Some(bad) = files.keys().find(|k| *k != "SKILL.md" && !safe_rel(k)) {
            return Err(Error::Invalid(format!("the path {bad} is not allowed in a skill")));
        }
        if files.values().map(String::len).sum::<usize>() > SKILL_TOTAL_MAX {
            return Err(Error::Invalid("the skill is larger than 512 KB".into()));
        }
        let dir = self.skills_root()?.join(&meta.name);
        // files that are no longer part of the skill go; the rest are replaced
        let mut old = vec![];
        collect(&dir, &dir, &mut old);
        for f in old.iter().filter(|f| !files.contains_key(*f)) {
            let _ = std::fs::remove_file(dir.join(f));
        }
        for (rel, text) in files {
            let p = dir.join(rel);
            if let Some(d) = p.parent() {
                std::fs::create_dir_all(d)?;
            }
            let tmp = p.with_extension("lh-tmp");
            std::fs::write(&tmp, text)?;
            std::fs::rename(&tmp, &p)?;
        }
        let version = self.history("skill", &meta.name)?.last().map_or(1, |h| h.version + 1);
        self.note(("skill", &meta.name), version, op, by, &json!({"files": files}), note)?;
        Ok(meta.name)
    }

    /// The files of a skill as text, for snapshots and updates.
    pub fn skill_files(&self, name: &str) -> Result<BTreeMap<String, String>> {
        let s = self.load_skill(name)?;
        let mut m = BTreeMap::new();
        m.insert("SKILL.md".to_string(), std::fs::read_to_string(s.dir.join("SKILL.md"))?);
        for f in s.files {
            if let Ok(t) = self.read_skill_file(name, &f) {
                m.insert(f, t);
            }
        }
        Ok(m)
    }

    pub fn rollback_skill(&self, name: &str, version: i64, by: &str) -> Result<String> {
        let snap = self.version_content("skill", name, version)?;
        let files: BTreeMap<String, String> = snap["files"].as_object().map(|o| o.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))).collect()).unwrap_or_default();
        self.install_skill(&files, "rollback", by, &format!("back to version {version}"))
    }

    /// Remove a skill for good, with its text in the history.
    pub fn delete_skill(&self, name: &str, by: &str) -> Result<()> {
        let s = self.load_skill(name)?;
        let v = self.history("skill", name)?.last().map_or(1, |h| h.version + 1);
        self.scrub("skill", name)?;
        std::fs::remove_dir_all(&s.dir)?;
        self.note(("skill", name), v, "delete", by, &json!({}), "")?;
        Ok(())
    }

    /// Apply an accepted skill suggestion. Payload fields: `name`, `description`, `body`; for a new skill
    /// optionally `files` (relative path to text), `license`, `compatibility`, `allowed_tools`.
    pub(crate) fn apply_skill_proposal(&self, kind: Kind, pl: &Value, by: &str) -> Result<String> {
        let s = |k: &str| pl[k].as_str().map(str::to_string);
        let name = s("name").ok_or_else(|| Error::Invalid("a skill needs a name".into()))?;
        let body = s("body").unwrap_or_default();
        if body.trim().is_empty() {
            return Err(Error::Invalid("a skill needs instructions".into()));
        }
        let extra: BTreeMap<String, String> = pl["files"].as_object().map(|o| o.iter().filter_map(|(k, v)| v.as_str().map(|t| (k.clone(), t.to_string()))).collect()).unwrap_or_default();
        match kind {
            Kind::SkillNew => {
                if self.load_skill(&name).is_ok() {
                    return Err(Error::Invalid(format!("a skill called {name} already exists; suggest a change to it instead")));
                }
                let meta = SkillMeta {
                    name: name.clone(),
                    description: s("description").ok_or_else(|| Error::Invalid("a skill needs a description".into()))?,
                    license: s("license"),
                    compatibility: s("compatibility"),
                    allowed_tools: pl["allowed_tools"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default(),
                    metadata: BTreeMap::new(),
                };
                let mut files = extra;
                files.insert("SKILL.md".into(), render_skill_md(&meta, &body));
                self.install_skill(&files, "create", by, "")
            }
            _ => {
                let old = self.load_skill(&name)?;
                let mut meta = old.meta.clone();
                if let Some(d) = s("description") {
                    meta.description = d;
                }
                let mut files = self.skill_files(&name)?;
                files.extend(extra);
                files.insert("SKILL.md".into(), render_skill_md(&meta, &body));
                self.install_skill(&files, "edit", by, &s("note").unwrap_or_default())
            }
        }
    }
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if e.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        if p.is_dir() {
            collect(root, &p, out);
        } else if let Ok(rel) = p.strip_prefix(root) {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_follow_the_format() {
        for ok in ["pdf", "pdf-tools", "a1", "x-2-y"] {
            assert!(valid_name(ok), "{ok}");
        }
        for bad in ["", "PDF", "-a", "a-", "a--b", "a_b", "a b", &"x".repeat(65)] {
            assert!(!valid_name(bad), "{bad}");
        }
    }

    #[test]
    fn a_header_is_read_checked_and_written_back_unchanged() {
        let src = "---\nname: pdf-tools\ndescription: >\n  Work with PDF files.\n  Use for forms.\nlicense: Apache-2.0\ncompatibility: Needs python3\nallowed-tools: Bash Read\nmetadata:\n  author: omar\n---\n\n# PDF tools\n\nDo the steps.\n";
        let (m, body) = parse_skill_md(src, Some("pdf-tools")).unwrap();
        assert_eq!((m.name.as_str(), m.description.as_str(), m.license.as_deref(), m.compatibility.as_deref()), ("pdf-tools", "Work with PDF files. Use for forms.", Some("Apache-2.0"), Some("Needs python3")));
        assert_eq!((m.allowed_tools.as_slice(), m.metadata.get("author").map(String::as_str), body.as_str()), (&["Bash".to_string(), "Read".to_string()][..], Some("omar"), "# PDF tools\n\nDo the steps."));
        let again = parse_skill_md(&render_skill_md(&m, &body), Some("pdf-tools")).unwrap();
        assert_eq!(again, (m, body));
    }

    #[test]
    fn bad_skills_are_refused_with_a_reason() {
        let e = |t: &str, f: Option<&str>| parse_skill_md(t, f).unwrap_err();
        assert!(e("# no header", None).contains("start with a header"));
        assert!(e("---\nname: a\n", None).contains("not closed"));
        assert!(e("---\ndescription: x\n---\nbody", None).contains("needs a name"));
        assert!(e("---\nname: a\n---\nbody", None).contains("needs a description"));
        assert!(e("---\nname: Bad_Name\ndescription: x\n---\nb", None).contains("not valid"));
        assert!(e("---\nname: a\ndescription: x\n---\nb", Some("b")).contains("must match its folder"));
        assert!(e(&format!("---\nname: a\ndescription: {}\n---\nb", "x".repeat(1025)), None).contains("longer than 1024"));
    }

    #[test]
    fn paths_inside_a_skill_cannot_escape() {
        for ok in ["scripts/run.sh", "references/a.md", "a.txt"] {
            assert!(safe_rel(ok), "{ok}");
        }
        for bad in ["", "../x", "/etc/passwd", "a/../../b", ".hidden", "a/.git/config"] {
            assert!(!safe_rel(bad), "{bad}");
        }
    }
}
