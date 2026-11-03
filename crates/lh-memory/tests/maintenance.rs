use std::sync::Arc;

use lh_memory::*;
use lh_store::Store;

fn mem() -> (tempfile::TempDir, Memory) {
    let d = tempfile::tempdir().unwrap();
    let m = Memory::open(Arc::new(Store::open_memory().unwrap()), d.path().join("m")).unwrap();
    (d, m)
}
fn add(m: &Memory, title: &str, body: &str) -> Page {
    m.create(NewPage::new(Layer::Facts, title, body), By::User).unwrap()
}

#[test]
fn duplicates_become_a_suggestion_to_archive_the_older_page_and_nothing_else_changes() {
    let (_d, m) = mem();
    let a = add(&m, "Coffee", "The user takes coffee black without sugar in the morning.");
    let b = add(&m, "Coffee habit", "The user takes coffee black, without sugar, in the morning.");
    add(&m, "Chess", "Plays chess on weekends.");
    let f = m.maintenance().unwrap();
    assert_eq!(f.iter().filter(|x| x.kind == FindingKind::Duplicate).count(), 1);
    assert_eq!(m.list(None, State::Live).unwrap().len(), 3, "the pass itself changes nothing");
    let props = m.list_proposals(Some("pending")).unwrap();
    assert_eq!(props.len(), 1);
    assert_eq!((props[0].kind, props[0].target_page.as_deref()), (Kind::Archive, Some(a.id.as_str())));
    assert!(props[0].evidence.contains("Coffee habit"));
    // running it again does not pile up the same suggestion
    m.maintenance().unwrap();
    assert_eq!(m.list_proposals(Some("pending")).unwrap().len(), 1);
    let _ = b;
}

#[test]
fn pinned_pages_are_never_suggested_for_archiving() {
    let (_d, m) = mem();
    let a = add(&m, "Coffee", "The user takes coffee black without sugar in the morning.");
    add(&m, "Coffee habit", "The user takes coffee black, without sugar, in the morning.");
    m.set_pinned(&a.id, true, By::User).unwrap();
    // the older page is pinned, so no suggestion
    std::thread::sleep(std::time::Duration::from_millis(1));
    m.maintenance().unwrap();
    assert!(m.list_proposals(Some("pending")).unwrap().iter().all(|p| p.target_page.as_deref() != Some(a.id.as_str())));
}

#[test]
fn broken_links_and_unlinked_pages_are_reported() {
    let (_d, m) = mem();
    let hub = add(&m, "Doctor", "Dr. Amal at the clinic. See [[pharmacy]] and [[missing-page]].");
    add(&m, "Pharmacy", "The pharmacy on Main Street.");
    let lone = add(&m, "Gym", "Goes on Tuesdays.");
    let f = m.maintenance().unwrap();
    let broken: Vec<_> = f.iter().filter(|x| x.kind == FindingKind::BrokenLink).collect();
    assert_eq!((broken.len(), broken[0].pages[0].as_str()), (1, hub.id.as_str()));
    assert!(broken[0].detail.contains("missing-page"));
    assert!(f.iter().any(|x| x.kind == FindingKind::Unlinked && x.pages == [lone.id.clone()]));
    assert!(!f.iter().any(|x| x.kind == FindingKind::Unlinked && x.pages[0] != lone.id && x.pages[0] != hub.id), "pharmacy is linked");
}

#[test]
fn old_facts_are_listed_as_possibly_stale_but_preferences_are_not() {
    let (_d, m) = mem();
    let old = add(&m, "Old", "The user rents a flat on Palm Street.");
    let pref = m.create(NewPage::new(Layer::Preferences, "Tone", "Terse."), By::User).unwrap();
    let year_ago = lh_store::now() - (STALE_DAYS + 5) * 86_400;
    m.store_for_tests().with(|c| c.execute("update pages set updated = ?1", [year_ago]).map(|_| ())).unwrap();
    let f = m.maintenance().unwrap();
    assert!(f.iter().any(|x| x.kind == FindingKind::Stale && x.pages == [old.id.clone()]));
    assert!(!f.iter().any(|x| x.kind == FindingKind::Stale && x.pages == [pref.id.clone()]));
}

#[test]
fn contradiction_candidates_share_words_but_are_not_duplicates() {
    let (_d, m) = mem();
    add(&m, "Home", "The user lives in Cairo near the river.");
    add(&m, "Home now", "The user lives in Berlin near the river.");
    add(&m, "Chess", "Plays chess on weekends.");
    let c = m.contradiction_candidates(5).unwrap();
    assert_eq!(c.len(), 1);
    let titles = [c[0].0.title.as_str(), c[0].1.title.as_str()];
    assert!(titles.contains(&"Home") && titles.contains(&"Home now"));
    assert!(contradiction_prompt(&c[0].0, &c[0].1).contains("Cairo"));
}
