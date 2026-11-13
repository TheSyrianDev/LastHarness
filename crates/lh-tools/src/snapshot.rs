//! Snapshots before file changes. Every write or edit saves the file's previous bytes under
//! `<root>/.lastharness/snapshots/` first, so a rollback can put them back.
//!
//! Snapshots are grouped by turn. `begin_turn` starts a new turn; `/rollback` undoes the newest
//! turn(s), and `rollback_changes` undoes the newest N file changes. Only the newest `KEEP_TURNS`
//! turns are kept; older snapshots are deleted when a turn begins.
//!
//! Name: `<turn:010>-<seq:010>-<hex path>[.new]`. A file that did not exist is an empty `.new` marker.

use std::path::{Path, PathBuf};

const DIR: &str = ".lastharness/snapshots";
const NEW_FILE: &str = "new";
/// How many turns of snapshots are kept.
pub const KEEP_TURNS: u64 = 20;

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

/// One saved snapshot, parsed from its file name.
struct Snap {
    path: PathBuf,
    turn: u64,
    seq: u64,
    file: String,
    is_new: bool,
}

fn parse(path: PathBuf) -> Option<Snap> {
    let name = path.file_name()?.to_str()?.to_string();
    let is_new = name.ends_with(&format!(".{NEW_FILE}"));
    let stem = name.trim_end_matches(&format!(".{NEW_FILE}")).to_string();
    let mut parts = stem.splitn(3, '-');
    let turn = parts.next()?.parse().ok()?;
    let seq = parts.next()?.parse().ok()?;
    let file = unhex(parts.next()?)?;
    Some(Snap { path, turn, seq, file, is_new })
}

fn all(root: &Path) -> Vec<Snap> {
    let mut v: Vec<Snap> = match std::fs::read_dir(dir(root)) {
        Ok(rd) => rd.filter_map(|e| e.ok()).filter_map(|e| parse(e.path())).collect(),
        Err(_) => vec![],
    };
    v.sort_by_key(|s| (s.turn, s.seq));
    v
}

fn current_turn(root: &Path) -> u64 {
    std::fs::read_to_string(dir(root).join("turn")).ok().and_then(|t| t.trim().parse().ok()).unwrap_or(0)
}

/// Start a new turn. Prunes snapshots of turns older than the newest `KEEP_TURNS`.
/// Also writes `.lastharness/.gitignore` (ignores the whole folder) so it stays out of version control.
pub fn begin_turn(root: &Path) -> std::io::Result<u64> {
    let d = dir(root);
    std::fs::create_dir_all(&d)?;
    // ignore the whole .lastharness folder in git, not just the snapshots
    std::fs::write(root.join(".lastharness").join(".gitignore"), "*\n")?;
    let turn = current_turn(root) + 1;
    std::fs::write(d.join("turn"), turn.to_string())?;
    let oldest_kept = turn.saturating_sub(KEEP_TURNS - 1);
    for s in all(root).into_iter().filter(|s| s.turn < oldest_kept) {
        std::fs::remove_file(&s.path)?;
    }
    Ok(turn)
}

/// Save the state of `file` (relative to `root`) before it changes, in the current turn.
/// `previous` is None for a new file.
pub fn save(root: &Path, file: &str, previous: Option<&[u8]>) -> std::io::Result<()> {
    let d = dir(root);
    std::fs::create_dir_all(&d)?;
    let turn = match current_turn(root) {
        0 => begin_turn(root)?,
        t => t,
    };
    let seq = all(root).last().map_or(0, |s| s.seq) + 1;
    let name = format!("{turn:010}-{seq:010}-{}", hex(file));
    match previous {
        Some(bytes) => std::fs::write(d.join(name), bytes),
        None => std::fs::write(d.join(format!("{name}.{NEW_FILE}")), b""),
    }
}

fn restore(root: &Path, snaps: Vec<Snap>) -> std::io::Result<Vec<String>> {
    let mut restored = Vec::new();
    for snap in snaps.into_iter().rev() {
        let target = root.join(&snap.file);
        if snap.is_new {
            let _ = std::fs::remove_file(&target);
        } else {
            std::fs::write(&target, std::fs::read(&snap.path)?)?;
        }
        std::fs::remove_file(&snap.path)?;
        restored.push(snap.file);
    }
    Ok(restored)
}

/// Undo the newest `count` changes, newest first. Returns the files put back (a file may repeat).
pub fn rollback_changes(root: &Path, count: usize) -> std::io::Result<Vec<String>> {
    let mut v = all(root);
    let keep_from = v.len().saturating_sub(count);
    let picked = v.split_off(keep_from);
    restore(root, picked)
}

/// Undo the newest `count` turns, all their changes. Returns the files put back.
pub fn rollback_turns(root: &Path, count: usize) -> std::io::Result<Vec<String>> {
    let v = all(root);
    let mut turns: Vec<u64> = v.iter().map(|s| s.turn).collect();
    turns.dedup();
    let cutoff = turns.len().saturating_sub(count);
    let oldest = turns.get(cutoff).copied().unwrap_or(u64::MAX);
    restore(root, v.into_iter().filter(|s| s.turn >= oldest).collect())
}

/// The chat command. `/rollback` undoes the last turn, `/rollback N` the last N changes.
#[derive(Debug, PartialEq, Eq)]
pub enum RollbackCommand {
    Turn,
    Changes(usize),
}

/// Parse `/rollback` or `/rollback N`. Returns None for any other text.
pub fn parse_command(text: &str) -> Option<Result<RollbackCommand, String>> {
    let rest = text.trim().strip_prefix("/rollback")?;
    if rest.is_empty() {
        return Some(Ok(RollbackCommand::Turn));
    }
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    Some(match rest.trim().parse::<usize>() {
        Ok(n) if n >= 1 => Ok(RollbackCommand::Changes(n)),
        _ => Err("usage: /rollback or /rollback N, N a number of changes, 1 or more".into()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollback_restores_previous_bytes_and_removes_new_files() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        std::fs::write(root.join("a.txt"), "one").unwrap();
        begin_turn(root).unwrap();
        save(root, "a.txt", Some(b"one")).unwrap();
        std::fs::write(root.join("a.txt"), "two").unwrap();
        save(root, "new.txt", None).unwrap();
        std::fs::write(root.join("new.txt"), "fresh").unwrap();

        let back = rollback_changes(root, 2).unwrap();
        assert_eq!(back, vec!["new.txt".to_string(), "a.txt".to_string()]);
        assert!(!root.join("new.txt").exists());
        assert_eq!(std::fs::read_to_string(root.join("a.txt")).unwrap(), "one");
        assert!(rollback_changes(root, 5).unwrap().is_empty());
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
        rollback_changes(root, 1).unwrap();
        assert_eq!(std::fs::read_to_string(root.join("f.txt")).unwrap(), "v2");
    }

    #[tokio::test]
    async fn a_large_existing_file_is_restored_not_deleted() {
        let d = tempfile::tempdir().unwrap();
        let set = crate::ToolSet::new([crate::Tool::WriteFile]);
        let big = "x".repeat(2 * 1024 * 1024);
        std::fs::write(d.path().join("big.txt"), &big).unwrap();
        crate::Tool::WriteFile.run(&serde_json::json!({"path":"big.txt","content":"small"}), d.path(), &set).await.unwrap();
        rollback_changes(d.path(), 1).unwrap();
        assert_eq!(std::fs::read_to_string(d.path().join("big.txt")).unwrap(), big);
    }

    #[test]
    fn file_names_with_spaces_and_unicode_round_trip() {
        assert_eq!(unhex(&hex("dir/a b é.md")).as_deref(), Some("dir/a b é.md"));
    }

    #[test]
    fn rollback_turns_undoes_a_whole_turn_at_once() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        std::fs::write(root.join("a.txt"), "a0").unwrap();
        std::fs::write(root.join("b.txt"), "b0").unwrap();
        begin_turn(root).unwrap();
        save(root, "a.txt", Some(b"a0")).unwrap();
        std::fs::write(root.join("a.txt"), "a1").unwrap();
        save(root, "b.txt", Some(b"b0")).unwrap();
        std::fs::write(root.join("b.txt"), "b1").unwrap();
        begin_turn(root).unwrap();
        save(root, "a.txt", Some(b"a1")).unwrap();
        std::fs::write(root.join("a.txt"), "a2").unwrap();

        let back = rollback_turns(root, 1).unwrap();
        assert_eq!(back, vec!["a.txt".to_string()]);
        assert_eq!(std::fs::read_to_string(root.join("a.txt")).unwrap(), "a1");
        assert_eq!(std::fs::read_to_string(root.join("b.txt")).unwrap(), "b1");
        let back = rollback_turns(root, 1).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(std::fs::read_to_string(root.join("a.txt")).unwrap(), "a0");
        assert_eq!(std::fs::read_to_string(root.join("b.txt")).unwrap(), "b0");
    }

    #[test]
    fn only_the_newest_twenty_turns_are_kept() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        for i in 0..(KEEP_TURNS + 5) {
            begin_turn(root).unwrap();
            save(root, &format!("f{i}.txt"), None).unwrap();
        }
        let kept: Vec<u64> = all(root).iter().map(|s| s.turn).collect();
        assert_eq!(kept.len() as u64, KEEP_TURNS);
        assert_eq!(*kept.first().unwrap(), 6);
        assert!(root.join(".lastharness").join(".gitignore").exists());
    }

    #[test]
    fn parses_the_rollback_command() {
        assert_eq!(parse_command("/rollback"), Some(Ok(RollbackCommand::Turn)));
        assert_eq!(parse_command("  /rollback 3 "), Some(Ok(RollbackCommand::Changes(3))));
        assert!(parse_command("/rollback 0").unwrap().is_err());
        assert!(parse_command("/rollbackx").is_none());
        assert!(parse_command("hello").is_none());
    }
}
