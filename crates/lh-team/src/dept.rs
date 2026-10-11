//! Departments: a named group of roles with a lead, in a Markdown file. A member can be another
//! department, so departments nest.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use lh_learn::yaml::{self, Value as Y};
use lh_learn::{valid_name, Learn};
use serde_json::json;

use crate::roles::RoleBook;
use crate::{Error, Result};

/// How many departments may sit inside each other.
pub const MAX_DEPTH: usize = 4;

#[derive(Clone, Debug, PartialEq)]
pub enum Member {
    Role(String),
    Dept(String),
}

impl Member {
    fn parse(s: &str) -> Member {
        match s.strip_prefix("dept:") {
            Some(d) => Member::Dept(d.to_string()),
            None => Member::Role(s.to_string()),
        }
    }
    fn render(&self) -> String {
        match self {
            Member::Role(r) => r.clone(),
            Member::Dept(d) => format!("dept:{d}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Department {
    pub name: String,
    pub description: String,
    /// A role. The lead plans, hands out steps, checks results and handles failures.
    pub lead: String,
    pub members: Vec<Member>,
    /// Overrides the workers-at-once setting for this department.
    pub parallel: Option<usize>,
    /// Overrides the attempts-per-step setting for this department.
    pub attempts: Option<u32>,
    pub instructions: String,
}

impl Department {
    pub fn parse(text: &str, file_stem: Option<&str>) -> std::result::Result<Department, String> {
        let text = text.replace("\r\n", "\n");
        let rest = text.strip_prefix("---\n").ok_or("a department file must start with a header between --- lines")?;
        let end = rest.find("\n---\n").map(|i| (i, 5)).or_else(|| rest.strip_suffix("\n---").map(|r| (r.len(), 4))).ok_or("the header is not closed with ---")?;
        let header = yaml::parse(&rest[..end.0])?;
        let instructions = rest.get(end.0 + end.1..).unwrap_or("").trim().to_string();
        let get = |k: &str| header.iter().find(|(x, _)| x == k).map(|(_, v)| v);
        let s = |k: &str| match get(k) {
            Some(Y::Str(v)) if !v.trim().is_empty() => Ok(Some(v.trim().to_string())),
            Some(Y::Str(_)) | None => Ok(None),
            Some(_) => Err(format!("{k} must be text")),
        };
        let name = s("name")?.ok_or("the header needs a name")?;
        if !valid_name(&name) {
            return Err(format!("the name \"{name}\" is not valid: lower case letters, digits and single hyphens"));
        }
        if file_stem.is_some_and(|f| f != name) {
            return Err(format!("the name \"{name}\" must match its file name \"{}.md\"", file_stem.unwrap_or("")));
        }
        let lead = s("lead")?.ok_or("the header needs a lead (a role)")?;
        let members: Vec<Member> = match get("members") {
            Some(Y::Str(v)) => v.split([' ', ',']).filter(|m| !m.is_empty()).map(Member::parse).collect(),
            Some(Y::List(v)) => v.iter().map(|m| Member::parse(m)).collect(),
            None => vec![],
            Some(_) => return Err("members must be a list of names".into()),
        };
        if members.is_empty() {
            return Err("a department needs at least one member".into());
        }
        let parallel = s("parallel")?.map(|v| v.parse::<usize>().ok().filter(|n| (1..=crate::settings::MAX_PARALLEL).contains(n)).ok_or("parallel must be a number from 1 to 16".to_string())).transpose()?;
        let attempts = s("attempts")?.map(|v| v.parse::<u32>().ok().filter(|n| (1..=crate::settings::MAX_ATTEMPTS).contains(n)).ok_or("attempts must be a number from 1 to 10".to_string())).transpose()?;
        Ok(Department { name, description: s("description")?.unwrap_or_default(), lead, members, parallel, attempts, instructions })
    }

    pub fn render(&self) -> String {
        let q = |s: &str| format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"));
        let mut h = format!("---\nname: {}\n", self.name);
        if !self.description.is_empty() {
            h.push_str(&format!("description: {}\n", q(&self.description)));
        }
        h.push_str(&format!("lead: {}\nmembers: {}\n", self.lead, self.members.iter().map(Member::render).collect::<Vec<_>>().join(" ")));
        if let Some(p) = self.parallel {
            h.push_str(&format!("parallel: {p}\n"));
        }
        if let Some(a) = self.attempts {
            h.push_str(&format!("attempts: {a}\n"));
        }
        format!("{h}---\n\n{}\n", self.instructions.trim())
    }
}

pub struct DeptBook {
    dir: PathBuf,
    learn: Arc<Learn>,
}

impl DeptBook {
    pub fn open(dir: impl Into<PathBuf>, learn: Arc<Learn>) -> Result<Self> {
        let dir = dir.into();
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir, learn })
    }

    pub fn list(&self) -> Result<Vec<(String, std::result::Result<Department, String>)>> {
        let mut out = vec![];
        for e in std::fs::read_dir(&self.dir)?.flatten() {
            let p = e.path();
            if p.extension().is_none_or(|x| x != "md") {
                continue;
            }
            let stem = p.file_stem().unwrap().to_string_lossy().to_string();
            out.push((stem.clone(), std::fs::read_to_string(&p).map_err(|e| e.to_string()).and_then(|t| Department::parse(&t, Some(&stem)))));
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out)
    }

    pub fn get(&self, name: &str) -> Result<Department> {
        if !valid_name(name) {
            return Err(Error::NotFound(format!("department {name}")));
        }
        let t = std::fs::read_to_string(self.dir.join(format!("{name}.md"))).map_err(|_| Error::NotFound(format!("department {name}")))?;
        Department::parse(&t, Some(name)).map_err(Error::Invalid)
    }

    /// Check that everything a department names exists and that departments do not contain themselves.
    pub fn validate(&self, d: &Department, roles: &RoleBook) -> Result<()> {
        let mut path = vec![d.name.clone()];
        self.check(d, roles, &mut path)
    }

    fn check(&self, d: &Department, roles: &RoleBook, path: &mut Vec<String>) -> Result<()> {
        if path.len() > MAX_DEPTH {
            return Err(Error::Invalid(format!("departments are nested more than {MAX_DEPTH} deep: {}", path.join(" > "))));
        }
        roles.get(&d.lead).map_err(|_| Error::Invalid(format!("the lead \"{}\" is not a role", d.lead)))?;
        let mut seen = HashSet::new();
        for m in &d.members {
            if !seen.insert(m.render()) {
                return Err(Error::Invalid(format!("{} is listed twice", m.render())));
            }
            match m {
                Member::Role(r) => {
                    roles.get(r).map_err(|_| Error::Invalid(format!("the member \"{r}\" is not a role")))?;
                }
                Member::Dept(n) => {
                    if path.contains(n) {
                        return Err(Error::Invalid(format!("departments contain each other: {} > {n}", path.join(" > "))));
                    }
                    let sub = self.get(n).map_err(|_| Error::Invalid(format!("the member dept:{n} is not a department")))?;
                    path.push(n.clone());
                    self.check(&sub, roles, path)?;
                    path.pop();
                }
            }
        }
        Ok(())
    }

    pub fn save(&self, d: &Department, roles: &RoleBook, by: &str) -> Result<i64> {
        self.validate(d, roles)?;
        let text = d.render();
        Department::parse(&text, Some(&d.name)).map_err(Error::Invalid)?;
        let path = self.dir.join(format!("{}.md", d.name));
        let existed = path.exists();
        let tmp = path.with_extension("md.tmp");
        std::fs::write(&tmp, &text)?;
        std::fs::rename(&tmp, &path)?;
        let version = self.learn.history("department", &d.name)?.last().map_or(1, |h| h.version + 1);
        self.learn.record_version(("department", &d.name), version, if existed { "edit" } else { "create" }, by, &json!({"file": text}), "")?;
        Ok(version)
    }

    pub fn rollback(&self, name: &str, version: i64, by: &str) -> Result<i64> {
        let snap = self.learn.version_content("department", name, version)?;
        let text = snap["file"].as_str().ok_or_else(|| Error::NotFound(format!("version {version} of department {name}")))?;
        Department::parse(text, Some(name)).map_err(Error::Invalid)?;
        let cur = self.learn.history("department", name)?.last().map_or(0, |h| h.version);
        std::fs::write(self.dir.join(format!("{name}.md")), text)?;
        self.learn.record_version(("department", name), cur + 1, "rollback", by, &json!({"file": text}), &format!("back to version {version}"))?;
        Ok(cur + 1)
    }

    pub fn delete(&self, name: &str, by: &str) -> Result<()> {
        self.get(name)?;
        // another department may still contain it
        for (other, d) in self.list()? {
            if other != name && d.is_ok_and(|d| d.members.contains(&Member::Dept(name.to_string()))) {
                return Err(Error::Invalid(format!("the department {other} contains {name}; take it out there first")));
            }
        }
        let cur = self.learn.history("department", name)?.last().map_or(0, |h| h.version);
        self.learn.scrub("department", name)?;
        std::fs::remove_file(self.dir.join(format!("{name}.md")))?;
        self.learn.record_version(("department", name), cur + 1, "delete", by, &json!({}), "")?;
        Ok(())
    }
}
