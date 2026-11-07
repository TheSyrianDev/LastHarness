//! Notes: secret screening, the index cap, department ownership and sharing, and the conflict and stale flags at recall.
use std::collections::HashSet;
use std::sync::Arc;

use lh_memory::*;
use lh_store::Store;

fn mem() -> (tempfile::TempDir, Memory) {
    let d = tempfile::tempdir().unwrap();
    let m = Memory::open(Arc::new(Store::open_memory().unwrap()), d.path().join("m")).unwrap();
    (d, m)
}
fn dest(department: Option<&str>) -> Destination {
    Destination { trust: Trust::Local, project: None, department: department.map(str::to_string) }
}
fn note(m: &Memory, dept: Option<&str>, shared: bool, title: &str, body: &str) -> Page {
    let mut n = NewPage::new(Layer::Facts, title, body);
    n.department = dept.map(str::to_string);
    n.shared = shared;
    m.create(n, By::User).unwrap()
}
fn recall(m: &Memory, q: &str, d: &Destination, cfg: &RecallConfig) -> Recalled {
    m.recall(q, d, cfg, &[], &HashSet::new()).unwrap()
}

#[test]
fn redact_replaces_keys_tokens_and_home_paths_and_keeps_the_rest() {
    let s = redact("Deploy with key sk-abcdefghijklmnop and read /home/fixture-user/project/notes.md today.");
    assert!(s.redacted);
    assert!(!s.text.contains("sk-abcdef") && !s.text.contains("/home/fixture-user"), "{}", s.text);
    assert!(s.text.contains("[redacted]") && s.text.contains("[private path]"), "{}", s.text);
    assert!(s.text.contains("Deploy with key") && s.text.contains("today."), "rest of the note stays: {}", s.text);
    let clean = redact("Prefers short answers in English.");
    assert!(!clean.redacted);
    assert_eq!(clean.text, "Prefers short answers in English.");
}

#[test]
fn index_is_capped_by_about_chars() {
    let (_d, m) = mem();
    for i in 0..60 {
        note(&m, None, false, &format!("Fact number {i}"), &format!("The user has a long fact about topic {i} that goes on for a while."));
    }
    let cfg = RecallConfig { about_chars: 600, ..Default::default() };
    let (text, refs) = m.about_you(&dest(None), &cfg).unwrap();
    assert!(text.chars().count() <= 600, "index is {} chars", text.chars().count());
    assert!(!refs.is_empty() && refs.len() < 60, "some notes fit, not all: {}", refs.len());
}

#[test]
fn a_department_sees_its_own_notes_and_the_users_not_other_departments() {
    let (_d, m) = mem();
    let own = note(&m, Some("sales"), false, "Pricing tier", "Sales quotes the pro tier at 40 euros.");
    let personal = note(&m, None, false, "Pricing preference", "The user likes pricing in euros.");
    let other = note(&m, Some("legal"), false, "Pricing contract", "Legal keeps pricing terms private.");
    let shared = note(&m, Some("legal"), true, "Pricing shared", "Legal shares its pricing review calendar.");
    let cfg = RecallConfig::default();
    let r = recall(&m, "pricing", &dest(Some("sales")), &cfg);
    let ids: Vec<&str> = r.notes.iter().map(|n| n.page.id.as_str()).collect();
    assert!(ids.contains(&own.id.as_str()) && ids.contains(&personal.id.as_str()) && ids.contains(&shared.id.as_str()));
    assert!(!ids.contains(&other.id.as_str()), "legal's private note stays with legal");
    let (about, refs) = m.about_you(&dest(Some("sales")), &cfg).unwrap();
    assert!(!about.contains("Legal keeps") && !refs.iter().any(|r| r.id == other.id), "{about}");
    let r = recall(&m, "pricing", &dest(None), &cfg);
    assert!(!r.notes.iter().any(|n| n.page.id == own.id), "the main chat does not see a department's own note");
}

#[test]
fn a_note_from_another_owner_on_the_same_topic_is_flagged_as_a_possible_conflict() {
    let (_d, m) = mem();
    note(&m, Some("sales"), true, "Office hours", "The office opens at nine every weekday morning.");
    note(&m, Some("ops"), true, "Office hours", "The office opens at ten every weekday morning now.");
    let r = recall(&m, "office opens weekday morning", &dest(None), &RecallConfig::default());
    let flagged: Vec<&Note> = r.notes.iter().filter(|n| !n.conflicts.is_empty()).collect();
    assert!(flagged.len() >= 2, "both notes are flagged: {:?}", r.notes.iter().map(|n| (&n.owner, &n.conflicts)).collect::<Vec<_>>());
    let text = r.render();
    assert!(text.contains("may conflict with") && text.contains("check which is current"), "{text}");
}

#[test]
fn a_note_not_changed_for_longer_than_the_limit_is_flagged_stale() {
    let (_d, m) = mem();
    note(&m, None, false, "Old desk", "The user works at the desk by the window.");
    let fresh = recall(&m, "desk window", &dest(None), &RecallConfig::default());
    assert!(!fresh.notes[0].stale, "a new note is not stale");
    let cfg = RecallConfig { stale_days: -1, ..Default::default() };
    let r = recall(&m, "desk window", &dest(None), &cfg);
    assert!(r.notes[0].stale);
    assert!(r.render().contains("may be stale: last changed"), "{}", r.render());
}
