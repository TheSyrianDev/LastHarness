//! Plugins: bundles of data. In this release a plugin holds roles, departments and themes, in a folder:
//!
//! ```text
//! my-plugin/
//!   plugin.json        optional: {"name": "...", "description": "..."}
//!   roles/*.md         role files
//!   departments/*.md   department files
//!   themes/            theme files, copied as they are (the theme format is not fixed yet)
//! ```
//!
//! Nothing runs from a plugin: no scripts, no code. The user sees what is inside before accepting, and an
//! item that would replace one they already have is skipped unless they say to replace it. Skills and MCP
//! servers are separate things and are not part of a bundle.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::{Department, DeptBook, Error, Result, Role, RoleBook};

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub name: String,
    pub description: String,
    /// The user already has one with this name.
    pub exists: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Preview {
    pub name: String,
    pub description: String,
    pub roles: Vec<Item>,
    pub departments: Vec<Item>,
    pub themes: Vec<String>,
    /// Files that could not be read, with the reason. Nothing from them is installed.
    pub problems: Vec<String>,
}

fn md_files(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "md")).collect();
    v.sort();
    v
}

fn stem(p: &Path) -> String {
    p.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string()
}

pub fn preview(dir: &Path, roles: &RoleBook, depts: &DeptBook) -> Result<Preview> {
    if !dir.is_dir() {
        return Err(Error::Invalid(format!("{} is not a folder", dir.display())));
    }
    let manifest: Value = std::fs::read_to_string(dir.join("plugin.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(Value::Null);
    let mut p = Preview {
        name: manifest["name"].as_str().map(str::to_string).unwrap_or_else(|| dir.file_name().and_then(|n| n.to_str()).unwrap_or("plugin").to_string()),
        description: manifest["description"].as_str().unwrap_or("").to_string(),
        roles: vec![],
        departments: vec![],
        themes: vec![],
        problems: vec![],
    };
    for f in md_files(&dir.join("roles")) {
        match std::fs::read_to_string(&f).map_err(|e| e.to_string()).and_then(|t| Role::parse(&t, Some(&stem(&f)))) {
            Ok(r) => p.roles.push(Item { exists: roles.get(&r.name).is_ok(), name: r.name, description: r.description }),
            Err(e) => p.problems.push(format!("roles/{}: {e}", f.file_name().and_then(|n| n.to_str()).unwrap_or(""))),
        }
    }
    for f in md_files(&dir.join("departments")) {
        match std::fs::read_to_string(&f).map_err(|e| e.to_string()).and_then(|t| Department::parse(&t, Some(&stem(&f)))) {
            Ok(d) => p.departments.push(Item { exists: depts.get(&d.name).is_ok(), name: d.name, description: d.description }),
            Err(e) => p.problems.push(format!("departments/{}: {e}", f.file_name().and_then(|n| n.to_str()).unwrap_or(""))),
        }
    }
    for e in std::fs::read_dir(dir.join("themes")).into_iter().flatten().flatten() {
        if e.path().is_file() {
            p.themes.push(e.file_name().to_string_lossy().to_string());
        }
    }
    p.themes.sort();
    Ok(p)
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Report {
    pub installed: Vec<String>,
    /// Skipped because the user already has one, or because something it needs is missing.
    pub skipped: Vec<String>,
}

/// Install what the preview showed. Roles go in first so departments can use them. `overwrite` names the
/// items (roles and departments) that may replace the user's own; every replacement is a new version, so
/// it can be rolled back.
pub fn install(dir: &Path, roles: &RoleBook, depts: &DeptBook, themes_dir: &Path, overwrite: &[String], by: &str) -> Result<Report> {
    let pv = preview(dir, roles, depts)?;
    let mut rep = Report::default();
    let may = |name: &str, exists: bool| !exists || overwrite.iter().any(|o| o == name);
    for item in &pv.roles {
        if !may(&item.name, item.exists) {
            rep.skipped.push(format!("role {}: you already have one", item.name));
            continue;
        }
        let text = std::fs::read_to_string(dir.join("roles").join(format!("{}.md", item.name))).map_err(Error::Io)?;
        let role = Role::parse(&text, Some(&item.name)).map_err(Error::Invalid)?;
        roles.save(&role, by)?;
        rep.installed.push(format!("role {}", item.name));
    }
    // departments may nest, so keep trying until no more can go in
    let mut left: Vec<&Item> = pv.departments.iter().collect();
    loop {
        let before = left.len();
        let mut rest = vec![];
        for item in left {
            if !may(&item.name, item.exists) {
                rep.skipped.push(format!("department {}: you already have one", item.name));
                continue;
            }
            let text = std::fs::read_to_string(dir.join("departments").join(format!("{}.md", item.name))).map_err(Error::Io)?;
            let d = Department::parse(&text, Some(&item.name)).map_err(Error::Invalid)?;
            match depts.save(&d, roles, by) {
                Ok(_) => rep.installed.push(format!("department {}", item.name)),
                Err(_) => rest.push(item),
            }
        }
        left = rest;
        if left.is_empty() || left.len() == before {
            break;
        }
    }
    for item in left {
        let text = std::fs::read_to_string(dir.join("departments").join(format!("{}.md", item.name))).map_err(Error::Io)?;
        let why = Department::parse(&text, Some(&item.name)).map_err(Error::Invalid).and_then(|d| depts.validate(&d, roles).map(|_| ())).err().map(|e| e.to_string()).unwrap_or_default();
        rep.skipped.push(format!("department {}: {why}", item.name));
    }
    if !pv.themes.is_empty() {
        let target = themes_dir.join(&pv.name);
        std::fs::create_dir_all(&target)?;
        for t in &pv.themes {
            std::fs::copy(dir.join("themes").join(t), target.join(t))?;
            rep.installed.push(format!("theme {t}"));
        }
    }
    Ok(rep)
}
