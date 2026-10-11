//! Roles: a Markdown file with a header and instructions. The files are the source of truth; the app
//! edits them through a form or chat. Every change is versioned and can be rolled back.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use lh_learn::yaml::{self, Value as Y};
use lh_learn::{valid_name, Learn};
use lh_memory::Layer;
use serde_json::json;

use crate::{Error, Result};

/// A role's instructions go with every step it does, on models that read them slowly, so they stay short.
pub const INSTRUCTIONS_MAX: usize = 600;

/// Tools a role may list. Memory and skill tools only work in a session that has them.
pub const KNOWN_TOOLS: &[&str] = &[
    "read_file", "write_file", "list_dir", "run_command", "web_fetch", "web_search", "generate_image", "recall", "open_note", "remember", "forget", "use_skill", "read_skill_file",
];

/// Which model a role uses: a kind of model (resolved through the model types) or one exact model.
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    Type(String),
    Model(String),
}

/// How much of the user's memory a role may see.
#[derive(Clone, Debug, PartialEq)]
pub enum MemorySlice {
    None,
    All,
    Layers(Vec<Layer>),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RoleSampling {
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub top_k: Option<u32>,
    pub min_p: Option<f32>,
    pub repeat_penalty: Option<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Role {
    pub name: String,
    /// One line the lead reads when deciding who gets a step.
    pub description: String,
    pub target: Target,
    pub tools: Vec<String>,
    pub memory: MemorySlice,
    pub sampling: RoleSampling,
    /// Most model calls in one step.
    pub max_steps: u32,
    /// `Some(false)` asks a model that can think to skip it. `None` leaves it to the model.
    pub thinking: Option<bool>,
    pub instructions: String,
}

impl Role {
    pub fn parse(text: &str, file_stem: Option<&str>) -> std::result::Result<Role, String> {
        let text = text.replace("\r\n", "\n");
        let rest = text.strip_prefix("---\n").ok_or("a role file must start with a header between --- lines")?;
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
        let description = s("description")?.ok_or("the header needs a description (one line for the lead)")?;
        let target = match (s("type")?, s("model")?) {
            (Some(t), None) => Target::Type(t.to_lowercase()),
            (None, Some(m)) => Target::Model(m),
            (None, None) => return Err("the header needs `type:` (a kind of model) or `model:` (one exact model)".into()),
            (Some(_), Some(_)) => return Err("give either `type:` or `model:`, not both".into()),
        };
        let tools: Vec<String> = match get("tools") {
            Some(Y::Str(v)) => v.split([' ', ',']).filter(|t| !t.is_empty()).map(str::to_string).collect(),
            Some(Y::List(v)) => v.clone(),
            None => vec![],
            Some(_) => return Err("tools must be a list of names".into()),
        };
        if let Some(bad) = tools.iter().find(|t| !KNOWN_TOOLS.contains(&t.as_str()) && !t.starts_with("mcp__")) {
            return Err(format!("unknown tool \"{bad}\". Known tools: {}", KNOWN_TOOLS.join(", ")));
        }
        let memory = match s("memory")?.as_deref() {
            None | Some("none") => MemorySlice::None,
            Some("all") => MemorySlice::All,
            Some(list) => MemorySlice::Layers(list.split([' ', ',']).filter(|p| !p.is_empty()).map(|p| Layer::parse(p).ok_or(format!("memory must be none, all, or layers from preferences, facts, projects (not \"{p}\")"))).collect::<std::result::Result<_, _>>()?),
        };
        let num = |k: &str| -> std::result::Result<Option<f32>, String> { s(k)?.map(|v| v.parse::<f32>().map_err(|_| format!("{k} must be a number"))).transpose() };
        let sampling = RoleSampling { temperature: num("temperature")?, top_p: num("top_p")?, min_p: num("min_p")?, repeat_penalty: num("repeat_penalty")?, top_k: s("top_k")?.map(|v| v.parse().map_err(|_| "top_k must be a whole number".to_string())).transpose()? };
        let max_steps = s("max_steps")?.map(|v| v.parse::<u32>().ok().filter(|n| (1..=100).contains(n)).ok_or("max_steps must be a whole number from 1 to 100".to_string())).transpose()?.unwrap_or(12);
        let thinking = match s("thinking")?.as_deref() {
            None => None,
            Some("on" | "true") => Some(true),
            Some("off" | "false") => Some(false),
            Some(o) => return Err(format!("thinking must be on or off, not \"{o}\"")),
        };
        if instructions.is_empty() {
            return Err("the role needs instructions after the header".into());
        }
        if instructions.chars().count() > INSTRUCTIONS_MAX {
            return Err(format!("the instructions are longer than {INSTRUCTIONS_MAX} characters. They go with every step the role does, so keep them to a short role line; the lead's brief carries the details"));
        }
        Ok(Role { name, description, target, tools, memory, sampling, max_steps, thinking, instructions })
    }

    pub fn render(&self) -> String {
        let q = |s: &str| format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"));
        let mut h = format!("---\nname: {}\ndescription: {}\n", self.name, q(&self.description));
        match &self.target {
            Target::Type(t) => h.push_str(&format!("type: {t}\n")),
            Target::Model(m) => h.push_str(&format!("model: {}\n", q(m))),
        }
        if !self.tools.is_empty() {
            h.push_str(&format!("tools: {}\n", self.tools.join(" ")));
        }
        match &self.memory {
            MemorySlice::None => h.push_str("memory: none\n"),
            MemorySlice::All => h.push_str("memory: all\n"),
            MemorySlice::Layers(l) => h.push_str(&format!("memory: {}\n", l.iter().map(|x| x.as_str()).collect::<Vec<_>>().join(" "))),
        }
        let sm = &self.sampling;
        for (k, v) in [("temperature", sm.temperature.map(|x| x.to_string())), ("top_p", sm.top_p.map(|x| x.to_string())), ("top_k", sm.top_k.map(|x| x.to_string())), ("min_p", sm.min_p.map(|x| x.to_string())), ("repeat_penalty", sm.repeat_penalty.map(|x| x.to_string()))] {
            if let Some(v) = v {
                h.push_str(&format!("{k}: {v}\n"));
            }
        }
        h.push_str(&format!("max_steps: {}\n", self.max_steps));
        if let Some(t) = self.thinking {
            h.push_str(&format!("thinking: {}\n", if t { "on" } else { "off" }));
        }
        format!("{h}---\n\n{}\n", self.instructions.trim())
    }
}

fn starters() -> Vec<Role> {
    let r = |name: &str, desc: &str, ty: &str, tools: &str, memory: MemorySlice, temp: f32, text: &str| Role {
        name: name.into(),
        description: desc.into(),
        target: Target::Type(ty.into()),
        tools: tools.split(' ').map(str::to_string).collect(),
        memory,
        sampling: RoleSampling { temperature: Some(temp), ..Default::default() },
        max_steps: 12,
        thinking: None,
        instructions: text.into(),
    };
    vec![
        r("researcher", "Finds and checks information in files and on the web, with sources.", "mid-tier", "read_file list_dir web_search web_fetch recall open_note", MemorySlice::All, 0.3,
          "You find and check information. Use the files you are given and the web. Say where each fact came from. Say plainly what you could not confirm. Do not guess. Finish with a short summary of what you found."),
        r("writer", "Writes and edits clear text to a brief.", "mid-tier", "read_file write_file list_dir recall", MemorySlice::Layers(vec![Layer::Preferences]), 0.7,
          "You write clear text for the brief you are given. Keep to the brief and the length asked for. Plain words, no filler. Save the finished text where the brief says. Finish with a one-line summary of what you wrote and where it is."),
        r("coder", "Writes, changes and tests code.", "mid-tier", "read_file write_file list_dir run_command", MemorySlice::Layers(vec![Layer::Preferences, Layer::Projects]), 0.2,
          "You write and change code. Read the existing code first and keep your changes small. Run the tests or the program when you can, and report the real result. Never say something works if you did not run it. Finish with a summary of what changed and what you checked."),
        r("designer", "Plans structure and layout and makes images when asked.", "mid-tier", "read_file write_file list_dir generate_image recall", MemorySlice::Layers(vec![Layer::Preferences, Layer::Projects]), 0.7,
          "You design structure, layout and wording for the brief. Explain each choice in a sentence. Make images only when asked. Do not invent a brand name, logo or visual identity: if the brief needs one, say so in your summary so the lead can ask the user."),
        r("planner", "Breaks a goal into small steps and checks the results.", "frontier", "read_file list_dir recall", MemorySlice::All, 0.2,
          "You plan and you check. Given a goal and a team, you split the goal into small steps, give each step to the best member, and say which steps must wait for others. When a result comes back you compare it with the step and say what is missing. You do not do the work yourself unless no member can."),
    ]
}

pub struct RoleBook {
    dir: PathBuf,
    learn: Arc<Learn>,
}

#[derive(Debug)]
pub struct RoleEntry {
    pub name: String,
    pub role: std::result::Result<Role, String>,
}

impl RoleBook {
    pub fn open(dir: impl Into<PathBuf>, learn: Arc<Learn>) -> Result<Self> {
        let dir = dir.into();
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir, learn })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Write the starter roles the first time. After that, roles the user deleted stay deleted.
    pub fn install_starters(&self) -> Result<usize> {
        let marker = self.dir.join(".starters-installed");
        if marker.exists() {
            return Ok(0);
        }
        let mut n = 0;
        for r in starters() {
            if !self.dir.join(format!("{}.md", r.name)).exists() {
                self.save(&r, "starter set")?;
                n += 1;
            }
        }
        std::fs::write(marker, "")?;
        Ok(n)
    }

    pub fn list(&self) -> Result<Vec<RoleEntry>> {
        let mut out = vec![];
        for e in std::fs::read_dir(&self.dir)?.flatten() {
            let p = e.path();
            if p.extension().is_none_or(|x| x != "md") {
                continue;
            }
            let stem = p.file_stem().unwrap().to_string_lossy().to_string();
            let role = std::fs::read_to_string(&p).map_err(|e| e.to_string()).and_then(|t| Role::parse(&t, Some(&stem)));
            out.push(RoleEntry { name: stem, role });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    pub fn get(&self, name: &str) -> Result<Role> {
        if !valid_name(name) {
            return Err(Error::NotFound(format!("role {name}")));
        }
        let t = std::fs::read_to_string(self.dir.join(format!("{name}.md"))).map_err(|_| Error::NotFound(format!("role {name}")))?;
        Role::parse(&t, Some(name)).map_err(Error::Invalid)
    }

    pub fn save(&self, role: &Role, by: &str) -> Result<i64> {
        let text = role.render();
        Role::parse(&text, Some(&role.name)).map_err(Error::Invalid)?; // never write a file that cannot be read back
        let path = self.dir.join(format!("{}.md", role.name));
        let existed = path.exists();
        let tmp = path.with_extension("md.tmp");
        std::fs::write(&tmp, &text)?;
        std::fs::rename(&tmp, &path)?;
        let version = self.learn.history("role", &role.name)?.last().map_or(1, |h| h.version + 1);
        self.learn.record_version(("role", &role.name), version, if existed { "edit" } else { "create" }, by, &json!({"file": text}), "")?;
        Ok(version)
    }

    pub fn rollback(&self, name: &str, version: i64, by: &str) -> Result<i64> {
        let snap = self.learn.version_content("role", name, version)?;
        let text = snap["file"].as_str().ok_or_else(|| Error::NotFound(format!("version {version} of role {name}")))?;
        let role = Role::parse(text, Some(name)).map_err(Error::Invalid)?;
        let cur = self.learn.history("role", name)?.last().map_or(0, |h| h.version);
        let path = self.dir.join(format!("{name}.md"));
        std::fs::write(&path, role.render())?;
        self.learn.record_version(("role", name), cur + 1, "rollback", by, &json!({"file": role.render()}), &format!("back to version {version}"))?;
        Ok(cur + 1)
    }

    pub fn delete(&self, name: &str, by: &str) -> Result<()> {
        self.get(name)?;
        let cur = self.learn.history("role", name)?.last().map_or(0, |h| h.version);
        self.learn.scrub("role", name)?;
        std::fs::remove_file(self.dir.join(format!("{name}.md")))?;
        self.learn.record_version(("role", name), cur + 1, "delete", by, &json!({}), "")?;
        Ok(())
    }

    pub fn history(&self, name: &str) -> Result<Vec<lh_learn::HistoryEntry>> {
        Ok(self.learn.history("role", name)?)
    }

    /// Roles that name this model type.
    pub fn using_type(&self, ty: &str) -> Result<Vec<String>> {
        Ok(self.list()?.into_iter().filter_map(|e| e.role.ok()).filter(|r| r.target == Target::Type(ty.to_string())).map(|r| r.name).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_starter_role_is_valid_and_survives_a_write_and_read() {
        for r in starters() {
            let back = Role::parse(&r.render(), Some(&r.name)).unwrap_or_else(|e| panic!("{}: {e}", r.name));
            assert_eq!(back, r);
        }
        assert_eq!(starters().iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), ["researcher", "writer", "coder", "designer", "planner"]);
    }

    #[test]
    fn starter_role_instructions_are_short() {
        for r in starters() {
            assert!(r.instructions.chars().count() <= INSTRUCTIONS_MAX, "{} has {} characters", r.name, r.instructions.chars().count());
        }
    }

    #[test]
    fn a_role_header_is_checked_with_a_reason() {
        let ok = "---\nname: scout\ndescription: Looks around.\ntype: fastest\ntools: read_file list_dir\nmemory: preferences facts\ntemperature: 0.4\nmax_steps: 5\nthinking: off\n---\n\nLook around and report.\n";
        let r = Role::parse(ok, Some("scout")).unwrap();
        assert_eq!((r.target.clone(), r.tools.len(), r.max_steps, r.thinking), (Target::Type("fastest".into()), 2, 5, Some(false)));
        assert_eq!(r.memory, MemorySlice::Layers(vec![Layer::Preferences, Layer::Facts]));
        let bad = |t: &str, want: &str| {
            let e = Role::parse(t, None).unwrap_err();
            assert!(e.contains(want), "{t:?} -> {e}");
        };
        bad("no header", "must start with a header");
        bad("---\ndescription: x\ntype: a\n---\nb", "needs a name");
        bad("---\nname: a\ntype: fastest\n---\nb", "needs a description");
        bad("---\nname: a\ndescription: x\n---\nb", "needs `type:`");
        bad("---\nname: a\ndescription: x\ntype: t\nmodel: m\n---\nb", "not both");
        bad("---\nname: a\ndescription: x\ntype: t\ntools: fly\n---\nb", "unknown tool");
        bad("---\nname: a\ndescription: x\ntype: t\nmemory: secrets\n---\nb", "memory must be");
        bad("---\nname: a\ndescription: x\ntype: t\nmax_steps: 0\n---\nb", "max_steps");
        bad("---\nname: a\ndescription: x\ntype: t\n---\n", "needs instructions");
        bad(&format!("---\nname: a\ndescription: x\ntype: t\n---\n{}", "word ".repeat(130)), "keep them to a short role line");
        bad("---\nname: Bad Name\ndescription: x\ntype: t\n---\nb", "not valid");
        assert!(Role::parse("---\nname: a\ndescription: x\ntype: t\n---\nb", Some("other")).unwrap_err().contains("must match its file name"));
    }
}
