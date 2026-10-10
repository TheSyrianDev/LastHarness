//! The Milestone 2 exit rule, as a test: a request to a cloud model never carries sensitive memory.
use std::collections::HashSet;
use std::sync::Arc;

use lh_memory::*;
use lh_store::Store;

fn mem() -> (tempfile::TempDir, Memory) {
    let d = tempfile::tempdir().unwrap();
    let m = Memory::open(Arc::new(Store::open_memory().unwrap()), d.path().join("m")).unwrap();
    (d, m)
}
fn dest(trust: Trust) -> Destination {
    Destination { trust, project: None }
}
fn add(m: &Memory, layer: Layer, title: &str, body: &str, privacy: Privacy) -> Page {
    let mut n = NewPage::new(layer, title, body);
    n.privacy = privacy;
    n.gist = format!("gist of {title}");
    m.create(n, By::User).unwrap()
}
fn everything_sent(m: &Memory, d: &Destination, query: &str) -> (String, Vec<PageRef>) {
    let cfg = RecallConfig::default();
    let (about, mut refs) = m.about_you(d, &cfg).unwrap();
    let r = m.recall(query, d, &cfg, &[], &HashSet::new()).unwrap();
    refs.extend(r.refs());
    (format!("{about}\n{}", r.render()), refs)
}

#[test]
fn sensitive_pages_never_reach_a_cloud_destination_by_any_path() {
    let (_d, m) = mem();
    let mut secret_ids = vec![];
    let mut open_ids = vec![];
    for i in 0..30 {
        // every sensitive page has a distinctive marker word that appears nowhere else
        let s = add(&m, [Layer::Facts, Layer::Preferences, Layer::Projects][i % 3], &format!("Private thing {i}"), &format!("zebrasecret{i} is the code for the vault. Remember zebrasecret{i}."), Privacy::Sensitive);
        let n = add(&m, [Layer::Facts, Layer::Preferences, Layer::Projects][i % 3], &format!("Open thing {i}"), &format!("giraffeopen{i} is a harmless note about vault shopping."), Privacy::Normal);
        m.set_pinned(&s.id, i % 2 == 0, By::User).unwrap(); // pins must not lift the rule either
        secret_ids.push(s.id);
        open_ids.push(n.id);
    }
    // a page the user marked sensitive by editing the file
    let hand = add(&m, Layer::Facts, "Hand marked", "handmarker is about vault matters.", Privacy::Normal);
    let path = m.root().join("facts/hand-marked.md");
    std::fs::write(&path, std::fs::read_to_string(&path).unwrap().replace("privacy: normal", "privacy: sensitive")).unwrap();
    m.scan().unwrap();
    secret_ids.push(hand.id);

    let queries: Vec<String> = (0..30).flat_map(|i| [format!("zebrasecret{i}"), format!("vault giraffeopen{i}"), "vault code".to_string(), "handmarker vault".to_string()]).collect();
    for trust in [Trust::OwnCloud, Trust::AnyCloud] {
        for q in &queries {
            let (text, refs) = everything_sent(&m, &dest(trust), q);
            assert!(!text.contains("zebrasecret") && !text.contains("handmarker") && !text.contains("Private thing") && !text.contains("Hand marked") && !text.contains("gist of Private"), "{trust:?} {q}: {text}");
            assert!(refs.iter().all(|r| !secret_ids.contains(&r.id)), "{trust:?} {q}");
        }
    }
    // not vacuous: the same questions to a local model do find the secrets, and the cloud-safe pages are used
    let (text, refs) = everything_sent(&m, &dest(Trust::Local), "zebrasecret4");
    assert!(text.contains("zebrasecret4") && refs.iter().any(|r| secret_ids.contains(&r.id)));
    let (text, _) = everything_sent(&m, &dest(Trust::AnyCloud), "giraffeopen7");
    assert!(text.contains("giraffeopen7"));
    // and the count of what was held back is reported, without the text
    let held = m.recall("zebrasecret4 vault", &dest(Trust::AnyCloud), &RecallConfig::default(), &[], &HashSet::new()).unwrap();
    assert!(held.withheld >= 1 && !held.render().contains("zebrasecret"));
}

#[test]
fn semantic_hits_go_through_the_same_gate() {
    let (_d, m) = mem();
    let s = add(&m, Layer::Facts, "Medication", "Takes metformin daily.", Privacy::Sensitive);
    let n = add(&m, Layer::Facts, "Hobby", "Likes chess.", Privacy::Normal);
    // pretend the embedding search ranked the secret page first
    let sem = vec![(s.id.clone(), 0.99), (n.id.clone(), 0.5)];
    let r = m.recall("what medicine", &dest(Trust::AnyCloud), &RecallConfig::default(), &sem, &HashSet::new()).unwrap();
    assert_eq!(r.notes.iter().map(|x| x.title.as_str()).collect::<Vec<_>>(), ["Hobby"]);
    assert_eq!(r.withheld, 1);
    let r = m.recall("what medicine", &dest(Trust::Local), &RecallConfig::default(), &sem, &HashSet::new()).unwrap();
    assert_eq!(r.notes[0].title, "Medication");
}

#[test]
fn archived_other_project_and_repeated_pages_are_left_out_and_requests_stay_bounded() {
    let (_d, m) = mem();
    let old = add(&m, Layer::Facts, "Old city", "Lived in Cairo before.", Privacy::Normal);
    m.archive(&old.id, By::User, "moved").unwrap();
    let mut other = NewPage::new(Layer::Projects, "Other project", "Cairo migration plan.");
    other.project = Some("/work/other".into());
    m.create(other, By::User).unwrap();
    let mut mine = NewPage::new(Layer::Projects, "This project", "Cairo office layout.");
    mine.project = Some("/work/mine".into());
    let mine = m.create(mine, By::User).unwrap();
    let d = Destination { trust: Trust::AnyCloud, project: Some("/work/mine".into()) };
    let r = m.recall("Cairo", &d, &RecallConfig::default(), &[], &HashSet::new()).unwrap();
    assert_eq!(r.notes.iter().map(|n| n.title.as_str()).collect::<Vec<_>>(), ["This project"]);
    // a page already in the conversation is not added again, but a new version is
    let seen: HashSet<_> = [(mine.id.clone(), 1)].into();
    assert!(m.recall("Cairo", &d, &RecallConfig::default(), &[], &seen).unwrap().notes.is_empty());
    m.update(&mine.id, Edit { body: Some("Cairo office moved to floor 3.".into()), ..Default::default() }, By::User).unwrap();
    assert_eq!(m.recall("Cairo", &d, &RecallConfig::default(), &[], &seen).unwrap().notes.len(), 1);

    // bounds
    for i in 0..40 {
        add(&m, Layer::Facts, &format!("Long note {i}"), &format!("lighthouse {}", "word ".repeat(400)), Privacy::Normal);
    }
    let cfg = RecallConfig::default();
    let r = m.recall("lighthouse", &d, &cfg, &[], &HashSet::new()).unwrap();
    assert!(r.notes.len() <= cfg.max_items && r.notes.iter().map(|n| n.text.chars().count()).sum::<usize>() <= cfg.budget_chars);
    assert!(r.notes.iter().all(|n| n.text.chars().count() <= cfg.excerpt_chars + 1));
    let (about, _) = m.about_you(&d, &cfg).unwrap();
    assert!(about.chars().count() <= cfg.about_chars);
}

#[test]
fn the_about_you_block_uses_gists_pins_first_and_never_a_model() {
    let (_d, m) = mem();
    let a = add(&m, Layer::Facts, "Lives", "The user lives in Berlin.", Privacy::Normal);
    let p = add(&m, Layer::Preferences, "Style", "Short answers.", Privacy::Normal);
    m.set_pinned(&a.id, true, By::User).unwrap();
    let mut nogist = NewPage::new(Layer::Facts, "No gist", "Plays the oud. Started in 2019.");
    nogist.gist = String::new();
    m.create(nogist, By::User).unwrap();
    let (text, refs) = m.about_you(&dest(Trust::AnyCloud), &RecallConfig::default()).unwrap();
    let lines: Vec<_> = text.lines().skip(1).collect();
    assert_eq!(lines[0], "- gist of Lives", "pinned first");
    assert_eq!(lines[1], "- gist of Style");
    assert_eq!(lines[2], "- Plays the oud.", "no gist: the first sentence, cut, not generated");
    assert_eq!(refs.iter().map(|r| r.id.clone()).collect::<Vec<_>>()[..2], [a.id, p.id]);
    assert_eq!(m.about_you(&dest(Trust::Local), &RecallConfig::default()).unwrap().0, text);
    let (_d2, empty) = mem();
    assert_eq!(empty.about_you(&dest(Trust::Local), &RecallConfig::default()).unwrap().0, "");
}
