//! Files sent with a message. They arrive as base64 inside the JSON of the turn request, are checked and
//! adapted to the model (see `lh_provider::prepare`), and are kept on disk next to the database, one file
//! per id. The database holds only a marker with the ids, never the bytes. The activity log holds names,
//! types and sizes, never contents. File contents are untrusted data: they are framed as such for the model.

use std::path::{Path, PathBuf};

use base64::Engine;
use lh_provider::{kind_of, AttachKind, Attachment};
use serde::Deserialize;
use serde_json::{json, Value};

/// Marker appended to the stored text of a message that had files. `resume` finds it and reloads the files.
const MARK: &str = "[[lh-attach:";

#[derive(Deserialize)]
pub struct NewAttachment {
    pub name: String,
    #[serde(default)]
    pub mime: String,
    /// Base64 of the file.
    pub data: String,
}

fn dir_of(store: &lh_store::Store) -> Option<PathBuf> {
    store.dir().map(|d| d.join("attachments"))
}

fn safe_name(n: &str) -> String {
    let base = n.rsplit(['/', '\\']).next().unwrap_or("file");
    let s: String = base.chars().filter(|c| !c.is_control()).take(120).collect();
    if s.trim().is_empty() { "file".into() } else { s }
}

fn new_id() -> String {
    let mut b = [0u8; 8];
    getrandom::fill(&mut b).expect("the OS random source is unavailable");
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Decode what the client sent. Errors are plain sentences for the user.
pub fn decode(files: &[NewAttachment]) -> Result<Vec<Attachment>, String> {
    files
        .iter()
        .map(|f| {
            let name = safe_name(&f.name);
            let bytes = base64::engine::general_purpose::STANDARD.decode(f.data.trim()).map_err(|_| format!("\"{name}\" is not valid base64."))?;
            let kind = kind_of(&name, &f.mime, &bytes).ok_or_else(|| format!("\"{name}\" is a binary file of a type that cannot be sent (images, video, audio, PDF and text files can)."))?;
            let mime = if f.mime.is_empty() { default_mime(kind) } else { f.mime.clone() };
            Ok(Attachment::new(new_id(), name, mime, kind, bytes))
        })
        .collect()
}

fn default_mime(k: AttachKind) -> String {
    match k {
        AttachKind::Image => "image/png",
        AttachKind::Video => "video/mp4",
        AttachKind::Audio => "audio/wav",
        AttachKind::Pdf => "application/pdf",
        _ => "text/plain",
    }
    .into()
}

/// Write the files (as sent, before any adaptation) under `attachments/<id>`, with a small meta file.
pub fn save(store: &lh_store::Store, files: &[Attachment]) {
    let Some(dir) = dir_of(store) else { return };
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    for f in files {
        let _ = std::fs::write(dir.join(&f.id), &*f.data);
        let meta = json!({"name": f.name, "mime": f.mime, "kind": kind_name(f.kind)});
        let _ = std::fs::write(dir.join(format!("{}.json", f.id)), meta.to_string());
    }
}

fn kind_name(k: AttachKind) -> &'static str {
    match k {
        AttachKind::Image => "image",
        AttachKind::Video => "video",
        AttachKind::Audio => "audio",
        AttachKind::Pdf => "pdf",
        AttachKind::Text => "text",
        AttachKind::Note => "note",
    }
}

/// What the activity log gets: no bytes.
pub fn describe(files: &[Attachment]) -> Value {
    Value::Array(files.iter().map(|f| json!({"id": f.id, "name": f.name, "mime": f.mime, "kind": kind_name(f.kind), "bytes": f.data.len()})).collect())
}

/// The stored form of a message: its text plus a marker with the file ids.
pub fn stored_text(text: &str, files: &[Attachment]) -> String {
    if files.is_empty() {
        return text.to_string();
    }
    let ids: Vec<&str> = files.iter().map(|f| f.id.as_str()).collect();
    format!("{text}\n{MARK}{}]]", ids.join(","))
}

/// Split a stored text into the plain text and the file ids it carried.
pub fn split_marker(stored: &str) -> (String, Vec<String>) {
    if let Some(i) = stored.rfind(MARK) {
        if let Some(ids) = stored[i + MARK.len()..].strip_suffix("]]") {
            let ids: Vec<String> = ids.split(',').filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_hexdigit())).map(String::from).collect();
            return (stored[..i].trim_end_matches('\n').to_string(), ids);
        }
    }
    (stored.to_string(), vec![])
}

/// Load saved files by id. Missing ones come back as a note, so the model is not told they exist.
pub fn load(dir: &Path, ids: &[String]) -> Vec<Attachment> {
    let dir = dir.join("attachments");
    ids.iter()
        .map(|id| {
            let meta: Option<Value> = std::fs::read_to_string(dir.join(format!("{id}.json"))).ok().and_then(|s| serde_json::from_str(&s).ok());
            let name = meta.as_ref().and_then(|m| m["name"].as_str()).unwrap_or("file").to_string();
            let mime = meta.as_ref().and_then(|m| m["mime"].as_str()).unwrap_or("").to_string();
            match std::fs::read(dir.join(id)) {
                Ok(bytes) => match kind_of(&name, &mime, &bytes) {
                    Some(k) => Attachment::new(id.clone(), name, mime, k, bytes),
                    None => gone(id, &name),
                },
                Err(_) => gone(id, &name),
            }
        })
        .collect()
}

fn gone(id: &str, name: &str) -> Attachment {
    Attachment::new(id, name, "text/plain", AttachKind::Note, format!("[A file named \"{name}\" was attached earlier, but it is no longer on disk.]").into_bytes())
}
