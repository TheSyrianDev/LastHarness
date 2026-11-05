//! The assistant's view of memory: look things up, remember on request, forget on request.
//! Everything it can see goes through `lh_memory::visible`, so a cloud model gets no sensitive notes.

use std::collections::HashSet;
use std::sync::Arc;

use lh_memory::embed::AnyEmbedder;
use lh_memory::{detect, excerpt, By, Destination, Layer, Memory, NewPage, Privacy, RecallConfig};
use serde_json::Value;

use crate::{Tool, ToolError};

/// Memory as one worker sees it.
pub struct MemoryHandle {
    pub memory: Arc<Memory>,
    pub embedder: Arc<AnyEmbedder>,
    /// Where this worker's model is: decides what it may see.
    pub dest: Destination,
    pub cfg: RecallConfig,
    /// A role's memory slice: only these layers are visible. `None` means all of them.
    pub layers: Option<Vec<lh_memory::Layer>>,
    /// A model on this machine that judges what the patterns let through, as the end-of-chat review does.
    /// `None`: patterns only. Never a cloud model, since judging means reading the note.
    pub judge: Option<PrivacyJudge>,
}

/// Reads a note and says whether it is sensitive.
pub type PrivacyJudge = Arc<dyn Fn(String) -> std::pin::Pin<Box<dyn std::future::Future<Output = lh_memory::Judgement> + Send>> + Send + Sync>;

impl std::fmt::Debug for MemoryHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryHandle").field("trust", &self.dest.trust).finish_non_exhaustive()
    }
}

fn fail(m: impl Into<String>) -> ToolError {
    ToolError::Failed(m.into())
}

pub(crate) async fn run(tool: Tool, args: &Value, h: &MemoryHandle) -> Result<String, ToolError> {
    match tool {
        Tool::Recall => {
            let q = args["query"].as_str().unwrap_or("");
            let semantic = if h.embedder.is_on() { h.memory.semantic_search(&*h.embedder, q, 10).await.unwrap_or_default() } else { vec![] };
            let mut r = h.memory.recall(q, &h.dest, &h.cfg, &semantic, &HashSet::new()).map_err(|e| fail(e.to_string()))?;
            r.notes.retain(|n| h.layers.as_ref().is_none_or(|l| l.contains(&n.layer)));
            if r.notes.is_empty() {
                return Ok("Nothing in memory matches.".into());
            }
            // a cheap list first; the full text of one note comes from `open_note` when it is needed
            let mut out = r.notes.iter().map(|n| format!("- [{}] ({}) {}: {}", n.page.id, n.layer.as_str(), n.title, excerpt(&n.text.replace('\n', " "), 160))).collect::<Vec<_>>().join("\n");
            out.push_str("\n(Use open_note with an id to read a whole note.)");
            Ok(out)
        }
        Tool::OpenNote => {
            let id = args["id"].as_str().unwrap_or("");
            let page = h.memory.get(id).ok().filter(|p| lh_memory::visible(p, &h.dest) && h.layers.as_ref().is_none_or(|l| l.contains(&p.layer))).ok_or_else(|| fail(format!("there is no note with id {id}")))?;
            Ok(format!("{} ({}, version {})\n\n{}", page.title, page.layer.as_str(), page.version, page.body))
        }
        Tool::Remember => {
            let text = args["text"].as_str().unwrap_or("").trim();
            if text.is_empty() {
                return Err(ToolError::Args("`text` must not be empty".into()));
            }
            let layer = args["layer"].as_str().and_then(Layer::parse).unwrap_or(Layer::Facts);
            if h.layers.as_ref().is_some_and(|l| !l.contains(&layer)) {
                return Err(fail("this role may not save to that part of memory"));
            }
            let title = args["title"].as_str().map(str::trim).filter(|t| !t.is_empty()).map(str::to_string).unwrap_or_else(|| excerpt(text, 50));
            let mut class = detect(&format!("{title} {text}"));
            if let (Privacy::Normal, Some(judge)) = (class.privacy, &h.judge) {
                class = match judge(format!("{title}\n{text}")).await {
                    lh_memory::Judgement::Normal => class,
                    lh_memory::Judgement::Sensitive(why) => lh_memory::Classification { privacy: Privacy::Sensitive, reason: format!("judged sensitive: {why}") },
                    lh_memory::Judgement::Unsure => lh_memory::Classification { privacy: Privacy::Sensitive, reason: "the model was unsure".into() },
                };
            }
            let mut n = NewPage::new(layer, title.clone(), text);
            n.gist = excerpt(text, 120);
            n.privacy = class.privacy;
            n.privacy_reason = class.reason.clone();
            n.source = "assistant".into();
            n.project = (layer == Layer::Projects).then(|| h.dest.project.clone()).flatten();
            let page = h.memory.create(n, By::Assistant).map_err(|e| fail(e.to_string()))?;
            let _ = h.memory.index_pending(&*h.embedder).await; // best effort: keyword search works without it
            let tail = if page.privacy == Privacy::Sensitive { format!(" It is marked sensitive ({}), so it stays on this machine.", class.reason) } else { String::new() };
            Ok(format!("Remembered as \"{}\" in {} (id {}).{tail}", page.title, layer.as_str(), page.id))
        }
        Tool::Forget => {
            let id = args["id"].as_str().unwrap_or("");
            let page = h.memory.get(id).map_err(|_| fail(format!("there is no note with id {id}")))?;
            // a model may only remove what it is allowed to see
            if !lh_memory::visible(&page, &h.dest) || h.layers.as_ref().is_some_and(|l| !l.contains(&page.layer)) {
                return Err(fail(format!("there is no note with id {id}")));
            }
            h.memory.delete(id, By::Assistant).map_err(|e| fail(e.to_string()))?;
            Ok(format!("Removed \"{}\" from memory.", page.title))
        }
        _ => Err(fail("not a memory tool")),
    }
}

/// The assistant reads a skill: first the instructions, then other files only if it needs them.
pub(crate) fn run_skill(tool: Tool, args: &Value, learn: &lh_learn::Learn) -> Result<String, ToolError> {
    let name = args["name"].as_str().unwrap_or("");
    match tool {
        Tool::UseSkill => {
            let s = learn.load_skill(name).map_err(|_| fail(format!("there is no skill called {name}")))?;
            let files = if s.files.is_empty() { "none".to_string() } else { s.files.join(", ") };
            Ok(format!("Skill: {}\nFolder: {}\nOther files: {files} (read with read_skill_file; run scripts with bash)\n\n{}", s.meta.name, s.dir.display(), s.body))
        }
        Tool::ReadSkillFile => learn.read_skill_file(name, args["path"].as_str().unwrap_or("")).map_err(|e| fail(e.to_string())),
        _ => Err(fail("not a skill tool")),
    }
}
