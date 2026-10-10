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

const MAX_READ: usize = 256 * 1024;
const MAX_OUT: usize = 32 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    ReadFile,
    WriteFile,
    ListDir,
    RunCommand,
}

impl Tool {
    pub fn name(self) -> &'static str {
        match self {
            Tool::ReadFile => "read_file",
            Tool::WriteFile => "write_file",
            Tool::ListDir => "list_dir",
            Tool::RunCommand => "run_command",
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
            Tool::RunCommand => Action::Run(args["command"].as_str().ok_or_else(|| ToolError::Args("`command` must be a string".into()))?.to_string()),
        })
    }

    pub async fn run(self, args: &Value, root: &Path) -> Result<String, ToolError> {
        match self {
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
                Ok(s)
            }
            Tool::WriteFile => {
                let Action::Write(p) = self.action(args, root)? else { unreachable!() };
                let content = args["content"].as_str().ok_or_else(|| ToolError::Args("`content` must be a string".into()))?;
                if let Some(dir) = p.parent() {
                    tokio::fs::create_dir_all(dir).await.map_err(|e| ToolError::Failed(e.to_string()))?;
                }
                // write to a sibling and rename, so a crash never leaves half a file
                let tmp = p.with_extension("lh-tmp");
                tokio::fs::write(&tmp, content).await.map_err(|e| ToolError::Failed(e.to_string()))?;
                tokio::fs::rename(&tmp, &p).await.map_err(|e| ToolError::Failed(e.to_string()))?;
                Ok(format!("wrote {} bytes to {}", content.len(), p.strip_prefix(root).unwrap_or(&p).display()))
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
                Ok(names.join("\n"))
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
                        Ok(format!("exit code {}\n{}", o.status.code().map_or("none (killed by a signal)".into(), |c| c.to_string()), shorten(&text)))
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
pub struct ToolSet(Vec<Tool>);

impl ToolSet {
    pub fn new(tools: impl IntoIterator<Item = Tool>) -> Self {
        Self(tools.into_iter().collect())
    }
    pub fn get(&self, name: &str) -> Option<Tool> {
        self.0.iter().copied().find(|t| t.name() == name)
    }
    pub fn defs(&self) -> Vec<ToolDef> {
        self.0.iter().map(|t| t.def()).collect()
    }
}

pub fn default_tools() -> ToolSet {
    ToolSet::new([Tool::ReadFile, Tool::WriteFile, Tool::ListDir, Tool::RunCommand])
}
