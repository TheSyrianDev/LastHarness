//! Files a person attaches to a message, shared by the terminal and the desktop app. Files are only
//! checked (name, size) when attached; the bytes are read and encoded when the message is sent.
//! The caps match the core's: 10 files, 96 MB together.

use std::path::{Path, PathBuf};

use base64::Engine;
use serde_json::{json, Value};

pub const MAX_FILES: usize = 10;
pub const MAX_TOTAL: u64 = 96 * 1024 * 1024;
/// Starts the marker the core appends to the stored text of a message that had files.
const MARK: &str = "[[lh-attach:";

/// One file waiting to be sent.
#[derive(Clone, Debug, PartialEq)]
pub struct Staged {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
}

impl Staged {
    pub fn from_path(path: &Path) -> Result<Staged, String> {
        let shown = path.display();
        let meta = std::fs::metadata(path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => format!("There is no file at {shown}."),
            _ => format!("Cannot read {shown}: {e}."),
        })?;
        if !meta.is_file() {
            return Err(format!("{shown} is not a file."));
        }
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "file".into());
        Ok(Staged { path: path.to_path_buf(), name, size: meta.len() })
    }
}

/// The files chosen for the next message.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stage {
    pub files: Vec<Staged>,
}

impl Stage {
    /// Add a file; the error is a plain sentence. Adding the same path twice does nothing.
    pub fn add(&mut self, path: &Path) -> Result<(), String> {
        let f = Staged::from_path(path)?;
        if self.files.iter().any(|x| x.path == f.path) {
            return Ok(());
        }
        if self.files.len() >= MAX_FILES {
            return Err(format!("A message can carry {MAX_FILES} files at most. Remove one first."));
        }
        if self.total() + f.size > MAX_TOTAL {
            return Err(format!("\"{}\" would take the files past {} MB together. Remove one first.", f.name, MAX_TOTAL / 1024 / 1024));
        }
        self.files.push(f);
        Ok(())
    }

    pub fn remove(&mut self, i: usize) {
        if i < self.files.len() {
            self.files.remove(i);
        }
    }

    pub fn total(&self) -> u64 {
        self.files.iter().map(|f| f.size).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    pub fn take(&mut self) -> Vec<Staged> {
        std::mem::take(&mut self.files)
    }
}

/// A guess from the extension. Empty means "let the core work it out from the bytes".
pub fn mime_for(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "wav" => "audio/wav",
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        "flac" => "audio/flac",
        "pdf" => "application/pdf",
        "txt" | "md" | "rs" | "py" | "json" | "toml" | "csv" | "log" => "text/plain",
        _ => "",
    }
}

/// The `attachments` array for the turn request. Reads the files now.
pub fn payload(files: &[Staged]) -> Result<Vec<Value>, String> {
    let b64 = base64::engine::general_purpose::STANDARD;
    files
        .iter()
        .map(|f| {
            let bytes = std::fs::read(&f.path).map_err(|e| format!("Cannot read {}: {e}.", f.path.display()))?;
            Ok(json!({"name": f.name, "mime": mime_for(&f.name), "data": b64.encode(bytes)}))
        })
        .collect()
}

/// Remove the visible `[[lh-attach:...]]` marker from stored user text. Returns the plain text and how many
/// files the message carried.
pub fn strip_marker(stored: &str) -> (String, usize) {
    if let Some(i) = stored.rfind(MARK) {
        if let Some(ids) = stored[i + MARK.len()..].strip_suffix("]]") {
            let n = ids.split(',').count();
            if ids.split(',').all(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_hexdigit())) {
                return (stored[..i].trim_end_matches('\n').to_string(), n);
            }
        }
    }
    (stored.to_string(), 0)
}

/// "1 file" / "3 files".
pub fn count_label(n: usize) -> String {
    if n == 1 { "1 file".into() } else { format!("{n} files") }
}

/// "12 B", "3.4 KB", "5.0 MB".
pub fn size_label(b: u64) -> String {
    match b {
        0..=1023 => format!("{b} B"),
        1024..=1048575 => format!("{:.1} KB", b as f64 / 1024.0),
        _ => format!("{:.1} MB", b as f64 / 1048576.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(dir: &Path, name: &str, n: usize) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, vec![b'a'; n]).unwrap();
        p
    }

    #[test]
    fn marker_is_stripped() {
        assert_eq!(strip_marker("look\n[[lh-attach:0a1b,ff00]]"), ("look".into(), 2));
        assert_eq!(strip_marker("plain"), ("plain".into(), 0));
        assert_eq!(strip_marker("odd [[lh-attach:zz]]"), ("odd [[lh-attach:zz]]".into(), 0));
    }

    #[test]
    fn caps_and_errors() {
        let d = std::env::temp_dir().join(format!("lh-attach-test-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let mut s = Stage::default();
        for i in 0..MAX_FILES {
            s.add(&file(&d, &format!("f{i}.txt"), 3)).unwrap();
        }
        assert!(s.add(&file(&d, "extra.txt", 3)).unwrap_err().contains("10 files"));
        s.add(&d.join("f0.txt")).unwrap();
        assert_eq!(s.files.len(), MAX_FILES, "same path twice adds nothing");
        s.remove(0);
        assert_eq!(s.files.len(), MAX_FILES - 1);
        assert!(s.add(&d.join("missing.png")).unwrap_err().contains("no file"));
        assert!(s.add(&d).unwrap_err().contains("not a file"));
        let p = payload(&s.files[..1]).unwrap();
        assert_eq!(p[0]["name"], "f1.txt");
        assert_eq!(p[0]["mime"], "text/plain");
        assert_eq!(p[0]["data"], "YWFh");
        let mut t = Stage::default();
        t.files.push(Staged { path: "x".into(), name: "x".into(), size: MAX_TOTAL });
        assert!(t.add(&d.join("f1.txt")).unwrap_err().contains("96 MB"));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn labels() {
        assert_eq!(count_label(1), "1 file");
        assert_eq!(size_label(2048), "2.0 KB");
    }
}
