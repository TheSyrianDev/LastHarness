use std::path::{Path, PathBuf};
use std::time::Duration;

use lh_provider::ToolDef;
use serde_json::{json, Value};
use tokio::io::AsyncReadExt;

use crate::Action;

/// A failure the model should see as text and may recover from. Never fatal to the turn.
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("bad arguments: {0}")]
    Args(String),
    #[error("{0}")]
    Failed(String),
}

/// How to put a file back the way it was. `previous` is `None` when the file did not exist.
#[derive(Clone, Debug, PartialEq)]
pub struct Undo {
    pub path: PathBuf,
    pub previous: Option<Vec<u8>>,
}

pub struct ToolOutput {
    pub text: String,
    pub undo: Option<Undo>,
}

impl From<String> for ToolOutput {
    fn from(text: String) -> Self {
        Self { text, undo: None }
    }
}

/// Files bigger than this are not kept for undo.
const MAX_UNDO: u64 = 1024 * 1024;

const MAX_READ: usize = 256 * 1024;
const MAX_OUT: usize = 32 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    ReadFile,
    WriteFile,
    ListDir,
    RunCommand,
    WebFetch,
    WebSearch,
    GenerateImage,
    UseSkill,
    ReadSkillFile,
    Recall,
    OpenNote,
    Remember,
    Forget,
}

impl Tool {
    pub fn name(self) -> &'static str {
        match self {
            Tool::ReadFile => "read_file",
            Tool::WriteFile => "write_file",
            Tool::ListDir => "list_dir",
            Tool::RunCommand => "run_command",
            Tool::WebFetch => "web_fetch",
            Tool::WebSearch => "web_search",
            Tool::GenerateImage => "generate_image",
            Tool::UseSkill => "use_skill",
            Tool::ReadSkillFile => "read_skill_file",
            Tool::Recall => "recall",
            Tool::OpenNote => "open_note",
            Tool::Remember => "remember",
            Tool::Forget => "forget",
        }
    }

    pub fn def(self) -> ToolDef {
        let (description, parameters) = match self {
            Tool::ReadFile => ("Read a text file in the project.", json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]})),
            Tool::WriteFile => (
                "Create or replace a text file in the project.",
                json!({"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"]}),
            ),
            Tool::ListDir => ("List the entries of a folder in the project.", json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]})),
            Tool::RunCommand => (
                "Run a shell command in the project folder and return its output.",
                json!({"type":"object","properties":{"command":{"type":"string"},"timeout_secs":{"type":"integer"}},"required":["command"]}),
            ),
            Tool::WebFetch => ("Fetch a web page or text file and return its text.", json!({"type":"object","properties":{"url":{"type":"string"}},"required":["url"]})),
            Tool::WebSearch => ("Search the web and return titles, addresses and snippets.", json!({"type":"object","properties":{"query":{"type":"string"}},"required":["query"]})),
            Tool::GenerateImage => (
                "Generate an image from a text description and save it in the project's `generated` folder.",
                json!({"type":"object","properties":{"prompt":{"type":"string"},"size":{"type":"string","description":"for example 512x512"}},"required":["prompt"]}),
            ),
            Tool::Recall => ("Search the user's memory for notes about a topic. Returns note ids, titles and text.", json!({"type":"object","properties":{"query":{"type":"string"}},"required":["query"]})),
            Tool::UseSkill => ("Read the instructions of one of your skills before doing a task it covers. Returns the steps and the list of its other files.", json!({"type":"object","properties":{"name":{"type":"string"}},"required":["name"]})),
            Tool::ReadSkillFile => ("Read one of a skill's other files (a reference document or a script), by the path use_skill listed.", json!({"type":"object","properties":{"name":{"type":"string"},"path":{"type":"string"}},"required":["name","path"]})),
            Tool::OpenNote => ("Read one whole note from the user's memory, by the id that recall gave.", json!({"type":"object","properties":{"id":{"type":"string"}},"required":["id"]})),
            Tool::Remember => (
                "Save something about the user to their memory, when they ask you to remember it. The user is asked to confirm.",
                json!({"type":"object","properties":{"text":{"type":"string"},"title":{"type":"string"},"layer":{"type":"string","enum":["preferences","facts","projects"]}},"required":["text"]}),
            ),
            Tool::Forget => ("Remove one note from the user's memory, when they ask you to forget it. Use recall first to find its id. The user is asked to confirm.", json!({"type":"object","properties":{"id":{"type":"string"}},"required":["id"]})),
        };
        ToolDef { name: self.name().into(), description: description.into(), parameters }
    }

    /// What this call would do, for the permission rules. Nothing runs here.
    pub fn action(self, args: &Value, root: &Path) -> Result<Action, ToolError> {
        let path = |a: &Value| -> Result<PathBuf, ToolError> {
            let p = a["path"].as_str().ok_or_else(|| ToolError::Args("`path` must be a string".into()))?;
            Ok(root.join(p))
        };
        Ok(match self {
            Tool::ReadFile | Tool::ListDir => Action::Read(path(args)?),
            Tool::WriteFile => Action::Write(path(args)?),
            Tool::WebFetch => Action::Fetch(args["url"].as_str().ok_or_else(|| ToolError::Args("`url` must be a string".into()))?.to_string()),
            Tool::WebSearch => Action::Fetch(format!("search: {}", args["query"].as_str().ok_or_else(|| ToolError::Args("`query` must be a string".into()))?)),
            Tool::GenerateImage => {
                let prompt = args["prompt"].as_str().ok_or_else(|| ToolError::Args("`prompt` must be a string".into()))?;
                Action::Write(root.join("generated").join(format!("{}.png", image_stem(prompt))))
            }
            Tool::UseSkill | Tool::ReadSkillFile => Action::Memory { write: false, what: format!("read skill {}", args["name"].as_str().unwrap_or("")) },
            Tool::Recall => Action::Memory { write: false, what: format!("recall: {}", args["query"].as_str().unwrap_or("")) },
            Tool::OpenNote => Action::Memory { write: false, what: format!("open note {}", args["id"].as_str().unwrap_or("")) },
            Tool::Remember => Action::Memory { write: true, what: format!("remember: {}", lh_memory::excerpt(args["text"].as_str().ok_or_else(|| ToolError::Args("`text` must be a string".into()))?, 300)) },
            Tool::Forget => Action::Memory { write: true, what: format!("forget note {}", args["id"].as_str().ok_or_else(|| ToolError::Args("`id` must be a string".into()))?) },
            Tool::RunCommand => Action::Run(args["command"].as_str().ok_or_else(|| ToolError::Args("`command` must be a string".into()))?.to_string()),
        })
    }

    pub async fn run(self, args: &Value, root: &Path, set: &ToolSet) -> Result<ToolOutput, ToolError> {
        let web = set.web();
        match self {
            Tool::UseSkill | Tool::ReadSkillFile => {
                let l = set.skills().ok_or_else(|| ToolError::Failed("skills are not available in this session".into()))?;
                crate::memory_tools::run_skill(self, args, l).map(Into::into)
            }
            Tool::Recall | Tool::OpenNote | Tool::Remember | Tool::Forget => crate::memory_tools::run(self, args, set.memory().ok_or_else(|| ToolError::Failed("memory is not available in this session".into()))?).await.map(Into::into),
            Tool::GenerateImage => Err(ToolError::Failed("image generation runs through the turn loop".into())),
            Tool::WebFetch => crate::web::fetch(web, args["url"].as_str().unwrap_or("")).await.map(Into::into),
            Tool::WebSearch => crate::web::search(web, args["query"].as_str().unwrap_or(""), 5).await.map(Into::into),
            Tool::ReadFile => {
                let p = match self.action(args, root)? {
                    Action::Read(p) => p,
                    _ => unreachable!(),
                };
                let f = tokio::fs::File::open(&p).await.map_err(|e| ToolError::Failed(format!("cannot open {}: {e}", p.display())))?;
                let mut buf = Vec::new();
                f.take(MAX_READ as u64 + 1).read_to_end(&mut buf).await.map_err(|e| ToolError::Failed(e.to_string()))?;
                let cut = buf.len() > MAX_READ;
                buf.truncate(MAX_READ);
                let mut s = String::from_utf8(buf).map_err(|_| ToolError::Failed("the file is not valid UTF-8 text".into()))?;
                if cut {
                    s.push_str("\n[file cut at 256 KB]");
                }
                Ok(s.into())
            }
            Tool::WriteFile => {
                let Action::Write(p) = self.action(args, root)? else { unreachable!() };
                let content = args["content"].as_str().ok_or_else(|| ToolError::Args("`content` must be a string".into()))?;
                let undo = match tokio::fs::metadata(&p).await {
                    Ok(m) if m.len() > MAX_UNDO => None,
                    Ok(_) => tokio::fs::read(&p).await.ok().map(|b| Undo { path: p.clone(), previous: Some(b) }),
                    Err(_) => Some(Undo { path: p.clone(), previous: None }),
                };
                if let Some(dir) = p.parent() {
                    tokio::fs::create_dir_all(dir).await.map_err(|e| ToolError::Failed(e.to_string()))?;
                }
                // write to a sibling and rename, so a crash never leaves half a file
                let tmp = p.with_extension("lh-tmp");
                tokio::fs::write(&tmp, content).await.map_err(|e| ToolError::Failed(e.to_string()))?;
                tokio::fs::rename(&tmp, &p).await.map_err(|e| ToolError::Failed(e.to_string()))?;
                Ok(ToolOutput { text: format!("wrote {} bytes to {}", content.len(), p.strip_prefix(root).unwrap_or(&p).display()), undo })
            }
            Tool::ListDir => {
                let Action::Read(p) = self.action(args, root)? else { unreachable!() };
                let mut rd = tokio::fs::read_dir(&p).await.map_err(|e| ToolError::Failed(format!("cannot list {}: {e}", p.display())))?;
                let mut names = Vec::new();
                while let Some(e) = rd.next_entry().await.map_err(|e| ToolError::Failed(e.to_string()))? {
                    let dir = e.file_type().await.map(|t| t.is_dir()).unwrap_or(false);
                    names.push(format!("{}{}", e.file_name().to_string_lossy(), if dir { "/" } else { "" }));
                }
                names.sort();
                Ok(names.join("\n").into())
            }
            Tool::RunCommand => {
                let Action::Run(cmd) = self.action(args, root)? else { unreachable!() };
                let secs = args["timeout_secs"].as_u64().unwrap_or(60).clamp(1, 600);
                let child = tokio::process::Command::new("sh")
                    .arg("-c")
                    .arg(&cmd)
                    .current_dir(root)
                    .stdin(std::process::Stdio::null())
                    .kill_on_drop(true)
                    .output();
                match tokio::time::timeout(Duration::from_secs(secs), child).await {
                    Err(_) => Err(ToolError::Failed(format!("the command ran longer than {secs} s and was stopped"))),
                    Ok(Err(e)) => Err(ToolError::Failed(format!("cannot start the command: {e}"))),
                    Ok(Ok(o)) => {
                        let mut text = String::from_utf8_lossy(&o.stdout).into_owned();
                        let err = String::from_utf8_lossy(&o.stderr);
                        if !err.is_empty() {
                            text.push_str("\n[stderr]\n");
                            text.push_str(&err);
                        }
                        Ok(format!("exit code {}\n{}", o.status.code().map_or("none (killed by a signal)".into(), |c| c.to_string()), shorten(&text)).into())
                    }
                }
            }
        }
    }
}

/// Keep the start and the end of long output; the middle is rarely the useful part.
fn shorten(s: &str) -> String {
    if s.len() <= MAX_OUT {
        return s.to_string();
    }
    let (a, b) = (floor(s, MAX_OUT / 2), ceil(s, s.len() - MAX_OUT / 2));
    format!("{}\n[... {} bytes cut ...]\n{}", &s[..a], b - a, &s[b..])
}
fn floor(s: &str, mut i: usize) -> usize {
    while !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}
fn ceil(s: &str, mut i: usize) -> usize {
    while !s.is_char_boundary(i) {
        i += 1;
    }
    i
}

/// The tools one worker may use. Anything not listed does not exist for it.
#[derive(Clone, Debug, Default)]
pub struct ToolSet {
    tools: Vec<Tool>,
    web: crate::WebConfig,
    image_model: Option<String>,
    memory: Option<std::sync::Arc<crate::MemoryHandle>>,
    skills: Option<std::sync::Arc<lh_learn::Learn>>,
}

impl ToolSet {
    pub fn new(tools: impl IntoIterator<Item = Tool>) -> Self {
        Self { tools: tools.into_iter().collect(), web: crate::WebConfig::default(), image_model: None, memory: None, skills: None }
    }
    pub fn with_web(mut self, web: crate::WebConfig) -> Self {
        self.web = web;
        self
    }
    /// Turn image generation on, with the model to use. Without this the tool does not exist.
    pub fn with_images(mut self, model: impl Into<String>) -> Self {
        self.image_model = Some(model.into());
        if !self.tools.contains(&Tool::GenerateImage) {
            self.tools.push(Tool::GenerateImage);
        }
        self
    }
    /// Give the worker the user's memory: `recall`, `remember` and `forget`. Leave this out for
    /// incognito sessions.
    pub fn with_memory(mut self, m: std::sync::Arc<crate::MemoryHandle>) -> Self {
        self.memory = Some(m);
        for t in [Tool::Recall, Tool::OpenNote, Tool::Remember, Tool::Forget] {
            if !self.tools.contains(&t) {
                self.tools.push(t);
            }
        }
        self
    }
    /// Let the worker read the user's skills (`use_skill`, `read_skill_file`).
    pub fn with_skills(mut self, l: std::sync::Arc<lh_learn::Learn>) -> Self {
        self.skills = Some(l);
        for t in [Tool::UseSkill, Tool::ReadSkillFile] {
            if !self.tools.contains(&t) {
                self.tools.push(t);
            }
        }
        self
    }
    pub fn skills(&self) -> Option<&lh_learn::Learn> {
        self.skills.as_deref()
    }
    pub fn memory(&self) -> Option<&crate::MemoryHandle> {
        self.memory.as_deref()
    }
    pub fn image_model(&self) -> Option<&str> {
        self.image_model.as_deref()
    }
    pub fn web(&self) -> &crate::WebConfig {
        &self.web
    }
    pub fn get(&self, name: &str) -> Option<Tool> {
        self.tools.iter().copied().find(|t| t.name() == name)
    }
    pub fn defs(&self) -> Vec<ToolDef> {
        self.tools.iter().map(|t| t.def()).collect()
    }
}

pub fn default_tools() -> ToolSet {
    ToolSet::new([Tool::ReadFile, Tool::WriteFile, Tool::ListDir, Tool::RunCommand, Tool::WebFetch, Tool::WebSearch])
}

/// File name stem for a generated image: the first words of the prompt, lower case, letters and digits.
pub(crate) fn image_stem(prompt: &str) -> String {
    let s: String = prompt.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect();
    let s = s.split('-').filter(|w| !w.is_empty()).collect::<Vec<_>>().join("-");
    let s: String = s.chars().take(40).collect();
    let s = s.trim_end_matches('-').to_string();
    if s.is_empty() { "image".into() } else { s }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_names_are_safe() {
        assert_eq!(image_stem("A red fox, in the snow!"), "a-red-fox-in-the-snow");
        assert_eq!(image_stem("../../etc/passwd"), "etc-passwd");
        assert_eq!(image_stem("مرحبا"), "image");
        assert_eq!(image_stem(&"word ".repeat(30)).len(), 39);
    }
}
