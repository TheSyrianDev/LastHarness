//! `show`: lets the assistant open something beside the chat, so a long answer, a file or an image can be read
//! properly. The tool does not draw anything; it sends a `show` event on the turn's stream and the front end
//! (the desktop app's side panel; the terminal prints it) decides how to display it. Files are read here, under
//! the session's folder rules, so a front end never reads a path the assistant picked.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, Weak};

use lh_provider::ToolDef;
use lh_tools::{BoxFut, ExtraTool, Risk, ToolError};
use serde_json::{json, Value};

use crate::session::Session;

/// Most text sent to the panel in one call.
const MAX_TEXT: usize = 200_000;
const MAX_FILE: u64 = 400_000;

#[derive(Debug)]
pub(crate) struct ShowTool {
    pub session: Arc<OnceLock<Weak<Session>>>,
}

fn kind_of_name(name: &str) -> &'static str {
    let ext = Path::new(name).extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" => "image",
        "md" | "markdown" => "markdown",
        "html" | "htm" => "html",
        "rs" | "py" | "js" | "ts" | "c" | "h" | "cpp" | "go" | "java" | "sh" | "toml" | "json" | "yaml" | "yml" | "css" | "sql" | "lua" | "rb" | "php" => "code",
        _ => "text",
    }
}

/// Decide what to send for a call: `(kind, text or path)`. Pure, so it is tested without a session.
pub(crate) fn plan(a: &Value, allow: impl Fn(&Path) -> bool, root: &Path) -> Result<(String, String, Option<PathBuf>), ToolError> {
    let title = a["title"].as_str().map(str::trim).filter(|t| !t.is_empty()).ok_or(ToolError::Args("`title` is missing".into()))?;
    let title: String = title.chars().take(80).collect();
    let want = a["kind"].as_str().map(str::to_ascii_lowercase);
    if let Some(k) = &want {
        if !["markdown", "code", "text", "image", "html"].contains(&k.as_str()) {
            return Err(ToolError::Args("`kind` must be markdown, code, text, image or html".into()));
        }
    }
    match (a["content"].as_str(), a["path"].as_str()) {
        (Some(c), _) if !c.trim().is_empty() => {
            let kind = want.unwrap_or_else(|| "markdown".into());
            if kind == "image" {
                return Err(ToolError::Args("an image is shown from a `path`, not from `content`".into()));
            }
            let c: String = c.chars().take(MAX_TEXT).collect();
            Ok((kind, c, None)).map(|(k, c, p)| (k, format!("{title}\n{c}"), p))
        }
        (_, Some(p)) => {
            let path = if Path::new(p).is_absolute() { PathBuf::from(p) } else { root.join(p) };
            if !allow(&path) {
                return Err(ToolError::Failed(format!("{p} is outside this session's folder or is a private place, so it cannot be shown")));
            }
            let meta = std::fs::metadata(&path).map_err(|e| ToolError::Failed(format!("cannot read {p}: {e}")))?;
            if !meta.is_file() {
                return Err(ToolError::Failed(format!("{p} is not a file")));
            }
            let kind = want.unwrap_or_else(|| kind_of_name(p).to_string());
            if kind == "image" {
                let canon = path.canonicalize().map_err(|e| ToolError::Failed(e.to_string()))?;
                return Ok((kind, format!("{title}\n"), Some(canon)));
            }
            if meta.len() > MAX_FILE {
                return Err(ToolError::Failed(format!("{p} is {} bytes; at most {MAX_FILE} can be shown", meta.len())));
            }
            let bytes = std::fs::read(&path).map_err(|e| ToolError::Failed(format!("cannot read {p}: {e}")))?;
            let text = String::from_utf8(bytes).map_err(|_| ToolError::Failed(format!("{p} is not text")))?;
            // a web page is not run here: the front end offers to open this very file in the person's browser,
            // so the approved, resolved path travels with the source
            let page = if kind == "html" { Some(path.canonicalize().map_err(|e| ToolError::Failed(e.to_string()))?) } else { None };
            Ok((kind, format!("{title}\n{text}"), page))
        }
        _ => Err(ToolError::Args("give `content` (the text) or `path` (a file in the folder)".into())),
    }
}

impl ExtraTool for ShowTool {
    fn def(&self) -> ToolDef {
        ToolDef {
            name: "show".into(),
            description: "Open text, a file or an image beside the chat so the user can read it properly. Give content, or a path inside the folder.".into(),
            parameters: json!({"type": "object", "properties": {"title": {"type": "string"}, "content": {"type": "string"}, "path": {"type": "string"}, "kind": {"type": "string", "enum": ["markdown", "code", "text", "image", "html"]}}, "required": ["title"]}),
        }
    }
    fn risk(&self) -> Risk {
        Risk::ReadOnly
    }
    fn describe(&self, a: &Value) -> String {
        format!("show \"{}\" beside the chat", a["title"].as_str().unwrap_or("?"))
    }
    fn guideline(&self) -> Option<String> {
        Some("show opens a long answer, file or image beside the chat.".into())
    }
    fn run(&self, a: Value) -> BoxFut<'_, Result<String, ToolError>> {
        Box::pin(async move {
            let session = self.session.get().and_then(|w| w.upgrade()).ok_or(ToolError::Failed("there is no window to show this in".into()))?;
            let (kind, body, image) = plan(&a, |p| session.perms.inside(p), session.perms.root())?;
            let (title, content) = body.split_once('\n').map(|(t, c)| (t.to_string(), c.to_string())).unwrap_or((body.clone(), String::new()));
            let sender = session.turn_events.lock().unwrap().as_ref().and_then(|w| w.upgrade());
            let Some(tx) = sender else { return Err(ToolError::Failed("nothing is listening for this".into())) };
            let _ = tx.send(("show".to_string(), json!({"type": "show", "title": title, "kind": kind, "content": content, "path": image.map(|p| p.to_string_lossy().to_string())})));
            Ok(format!("Opened \"{title}\" beside the chat."))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allow_all(_: &Path) -> bool {
        true
    }

    #[test]
    fn text_is_shown_as_markdown_unless_told_otherwise() {
        let (k, body, p) = plan(&json!({"title": "Plan", "content": "# Hi"}), allow_all, Path::new("/x")).unwrap();
        assert_eq!((k.as_str(), body.as_str(), p), ("markdown", "Plan\n# Hi", None));
        let (k, _, _) = plan(&json!({"title": "t", "content": "fn x() {}", "kind": "code"}), allow_all, Path::new("/x")).unwrap();
        assert_eq!(k, "code");
    }

    #[test]
    fn mistakes_are_told_to_the_model_plainly() {
        let r = Path::new("/x");
        assert!(plan(&json!({"content": "x"}), allow_all, r).unwrap_err().to_string().contains("title"));
        assert!(plan(&json!({"title": "t"}), allow_all, r).unwrap_err().to_string().contains("content"));
        assert!(plan(&json!({"title": "t", "content": "x", "kind": "video"}), allow_all, r).unwrap_err().to_string().contains("kind"));
        assert!(plan(&json!({"title": "t", "content": "x", "kind": "image"}), allow_all, r).unwrap_err().to_string().contains("path"));
    }

    #[test]
    fn a_file_is_read_by_the_core_only_inside_the_folder_and_images_pass_as_paths() {
        let d = std::env::temp_dir().join(format!("lh-show-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("a.md"), "# Notes").unwrap();
        std::fs::write(d.join("pic.png"), b"png").unwrap();
        std::fs::write(d.join("bin.dat"), [0xff, 0xfe, 0x00]).unwrap();
        let inside = |p: &Path| p.starts_with(&d);
        let (k, body, _) = plan(&json!({"title": "Notes", "path": "a.md"}), inside, &d).unwrap();
        assert_eq!((k.as_str(), body.as_str()), ("markdown", "Notes\n# Notes"));
        let (k, _, img) = plan(&json!({"title": "Pic", "path": "pic.png"}), inside, &d).unwrap();
        assert_eq!(k, "image");
        assert_eq!(img.unwrap(), d.join("pic.png").canonicalize().unwrap());
        assert!(plan(&json!({"title": "x", "path": "/etc/hostname"}), inside, &d).unwrap_err().to_string().contains("outside"));
        assert!(plan(&json!({"title": "x", "path": "bin.dat"}), inside, &d).unwrap_err().to_string().contains("not text"));
        assert!(plan(&json!({"title": "x", "path": "missing.md"}), inside, &d).unwrap_err().to_string().contains("cannot read"));
        assert!(plan(&json!({"title": "x", "path": "."}), inside, &d).unwrap_err().to_string().contains("not a file"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_web_page_is_sent_as_source_with_the_approved_path() {
        let d = std::env::temp_dir().join(format!("lh-show-html-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("index.html"), "<h1>Hi</h1>").unwrap();
        let inside = |p: &Path| p.starts_with(&d);
        let (k, body, page) = plan(&json!({"title": "Page", "path": "index.html"}), inside, &d).unwrap();
        assert_eq!((k.as_str(), body.as_str()), ("html", "Page\n<h1>Hi</h1>"));
        assert_eq!(page.unwrap(), d.join("index.html").canonicalize().unwrap(), "the front end gets the resolved file");
        assert!(plan(&json!({"title": "x", "path": "/etc/hostname", "kind": "html"}), inside, &d).unwrap_err().to_string().contains("outside"), "no path outside the folder, whatever the kind");
        let (k, _, page) = plan(&json!({"title": "Inline", "content": "<p>x</p>", "kind": "html"}), allow_all, &d).unwrap();
        assert_eq!(k, "html");
        assert!(page.is_none(), "text with no file has nothing to open in a browser");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_file_kind_follows_its_name() {
        assert_eq!(kind_of_name("a.html"), "html");
        assert_eq!(kind_of_name("A.HTM"), "html");
        assert_eq!(kind_of_name("a.PNG"), "image");
        assert_eq!(kind_of_name("a.rs"), "code");
        assert_eq!(kind_of_name("README.md"), "markdown");
        assert_eq!(kind_of_name("notes"), "text");
    }

    #[test]
    fn very_long_text_is_cut() {
        let big = "x".repeat(MAX_TEXT + 500);
        let (_, body, _) = plan(&json!({"title": "t", "content": big}), allow_all, Path::new("/x")).unwrap();
        assert_eq!(body.len(), "t\n".len() + MAX_TEXT);
    }
}
