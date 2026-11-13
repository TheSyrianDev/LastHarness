//! Snapshots before file changes. Every write or edit saves the file's previous bytes under
//! `<root>/.lastharness/snapshots/` first, so `rollback` can put the newest changes back.
//!
//! Each snapshot is one file named `<sequence>-<hex path>`, where the sequence is the save order.
//! The previous content is stored as-is; a file that did not exist is recorded as an empty marker.

use std::path::{Path, PathBuf};

const DIR: &str = ".lastharness/snapshots";
const NEW_FILE: &str = "new";

fn dir(root: &Path) -> PathBuf {
    root.join(DIR)
}

fn hex(s: &str) -> String {
    s.bytes().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Option<String> {
    let bytes: Option<Vec<u8>> = (0..s.len()).step_by(2).map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok()).collect();
    String::from_utf8(bytes?).ok()
}

/// Save the state of `file` (relative to `root`) before it changes. `previous` is None for a new file.
pub fn save(root: &Path, file: &str, previous: Option<&[u8]>) -> std::io::Result<()> {
    let d = dir(root);
    std::fs::create_dir_all(&d)?;
    let seq = std::fs::read_dir(&d)?.count() as u64 + 1;
    let name = format!("{seq:010}-{}", hex(file));
    match previous {
        Some(bytes) => std::fs::write(d.join(name), bytes),
        None => std::fs::write(d.join(format!("{name}.{NEW_FILE}")), b""),
    }
}

/// Restore the newest `count` snapshots, newest first. Returns the files put back.
/// A snapshot of a new file removes that file. Each restored snapshot is deleted.
pub fn rollback(root: &Path, count: usize) -> std::io::Result<Vec<String>> {
    let d = dir(root);
    let mut entries: Vec<PathBuf> = match std::fs::read_dir(&d) {
        Ok(rd) => rd.filter_map(|e| e.ok().map(|e| e.path())).collect(),
        Err(_) => return Ok(vec![]),
    };
    entries.sort();
    let mut restored = Vec::new();
    for snap in entries.into_iter().rev().take(count) {
        let name = snap.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
        let is_new = name.ends_with(&format!(".{NEW_FILE}"));
        let stem = name.trim_end_matches(&format!(".{NEW_FILE}"));
        let Some(file) = stem.split_once('-').and_then(|(_, h)| unhex(h)) else { continue };
        let target = root.join(&file);
        if is_new {
            let _ = std::fs::remove_file(&target);
        } else {
            std::fs::write(&target, std::fs::read(&snap)?)?;
        }
        std::fs::remove_file(&snap)?;
        restored.push(file);
    }
    Ok(restored)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollback_restores_previous_bytes_and_removes_new_files() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        std::fs::write(root.join("a.txt"), "one").unwrap();
        save(root, "a.txt", Some(b"one")).unwrap();
        std::fs::write(root.join("a.txt"), "two").unwrap();
        save(root, "new.txt", None).unwrap();
        std::fs::write(root.join("new.txt"), "fresh").unwrap();

        let back = rollback(root, 2).unwrap();
        assert_eq!(back, vec!["new.txt".to_string(), "a.txt".to_string()]);
        assert!(!root.join("new.txt").exists());
        assert_eq!(std::fs::read_to_string(root.join("a.txt")).unwrap(), "one");
        assert!(rollback(root, 5).unwrap().is_empty());
    }

    #[test]
    fn rollback_with_count_one_undoes_only_the_newest_change() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        std::fs::write(root.join("f.txt"), "v1").unwrap();
        save(root, "f.txt", Some(b"v1")).unwrap();
        std::fs::write(root.join("f.txt"), "v2").unwrap();
        save(root, "f.txt", Some(b"v2")).unwrap();
        std::fs::write(root.join("f.txt"), "v3").unwrap();
        rollback(root, 1).unwrap();
        assert_eq!(std::fs::read_to_string(root.join("f.txt")).unwrap(), "v2");
    }

    #[tokio::test]
    async fn a_large_existing_file_is_restored_not_deleted() {
        let d = tempfile::tempdir().unwrap();
        let set = crate::ToolSet::new([crate::Tool::WriteFile]);
        let big = "x".repeat(2 * 1024 * 1024);
        std::fs::write(d.path().join("big.txt"), &big).unwrap();
        crate::Tool::WriteFile.run(&serde_json::json!({"path":"big.txt","content":"small"}), d.path(), &set).await.unwrap();
        rollback(d.path(), 1).unwrap();
        assert_eq!(std::fs::read_to_string(d.path().join("big.txt")).unwrap(), big);
    }

    #[test]
    fn file_names_with_spaces_and_unicode_round_trip() {
        assert_eq!(unhex(&hex("dir/a b é.md")).as_deref(), Some("dir/a b é.md"));
    }
}
