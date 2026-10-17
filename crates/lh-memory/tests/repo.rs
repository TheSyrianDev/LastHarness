use std::sync::Arc;

use lh_memory::*;
use lh_store::Store;

fn mem() -> (tempfile::TempDir, Memory) {
    let d = tempfile::tempdir().unwrap();
    let m = Memory::open(Arc::new(Store::open_memory().unwrap()), d.path().join("memory")).unwrap();
    (d, m)
}
fn page(m: &Memory, layer: Layer, title: &str, body: &str) -> Page {
    m.create(NewPage::new(layer, title, body), By::User).unwrap()
}
fn ids(hits: &[Hit]) -> Vec<String> {
    hits.iter().map(|h| h.page.id.clone()).collect()
}

#[test]
fn a_page_is_a_readable_file_an_index_row_and_a_history_entry() {
    let (_d, m) = mem();
    let p = page(&m, Layer::Facts, "Penicillin allergy", "The user is allergic to penicillin. See [[doctor]].");
    let file = std::fs::read_to_string(m.root().join("facts/penicillin-allergy.md")).unwrap();
    assert!(file.starts_with("---\nid: p_") && file.contains("layer: facts") && file.contains("# Penicillin allergy"), "{file}");
    assert!(file.contains("allergic to penicillin"));
    let index = std::fs::read_to_string(m.root().join("facts/index.md")).unwrap();
    assert!(index.contains("[[penicillin-allergy]] Penicillin allergy"));
    let h = m.history(&p.id).unwrap();
    assert_eq!((h.len(), h[0].op.as_str(), h[0].by.as_str(), h[0].version), (1, "create", "user", 1));
    assert_eq!(m.get(&p.id).unwrap(), p);
    // the same title again gets its own file
    let q = page(&m, Layer::Facts, "Penicillin allergy", "Second page with the same title.");
    assert_eq!(q.slug, "penicillin-allergy-2");
    assert!(page_err(&m, "", "x"));
    assert!(page_err(&m, "T", "   "));
}
fn page_err(m: &Memory, t: &str, b: &str) -> bool {
    m.create(NewPage::new(Layer::Facts, t, b), By::User).is_err()
}

#[test]
fn keyword_search_works_in_english_and_arabic_with_word_forms() {
    let (_d, m) = mem();
    let en = page(&m, Layer::Facts, "Trip", "Flight to Berlin on 14 June with Lufthansa.");
    let ar = page(&m, Layer::Facts, "الإيجار", "الراتب ٤٢٠٠ شهريا والإيجار ١١٠٠.");
    let other = page(&m, Layer::Preferences, "Style", "Short answers, plain words.");
    assert_eq!(ids(&m.search("Berlin", State::Live, 5).unwrap()), vec![en.id.clone()]);
    assert_eq!(ids(&m.search("flights", State::Live, 5).unwrap()), vec![en.id.clone()], "prefix match finds word forms");
    assert_eq!(ids(&m.search("ايجار", State::Live, 5).unwrap()), vec![ar.id.clone()], "alef forms and the article are folded");
    assert_eq!(ids(&m.search("٤٢٠٠", State::Live, 5).unwrap()), vec![ar.id.clone()], "Arabic digits match ASCII digits");
    assert!(m.search("the of and", State::Live, 5).unwrap().is_empty(), "only stop words");
    assert_eq!(ids(&m.search("answers", State::Live, 5).unwrap()), vec![other.id]);
}

#[test]
fn editing_keeps_the_old_version_and_the_search_follows() {
    let (_d, m) = mem();
    let p = page(&m, Layer::Facts, "Home city", "The user lives in Cairo.");
    let q = m.update(&p.id, Edit { body: Some("The user lives in Berlin.".into()), gist: Some("Lives in Berlin".into()), ..Default::default() }, By::User).unwrap();
    assert_eq!((q.version, q.gist.as_str()), (2, "Lives in Berlin"));
    let old = std::fs::read_to_string(m.root().join("_archive/facts/home-city-v1.md")).unwrap();
    assert!(old.contains("lives in Cairo"));
    assert!(m.search("Cairo", State::Live, 5).unwrap().is_empty());
    assert_eq!(m.search("Berlin", State::Live, 5).unwrap().len(), 1);
    let ops: Vec<_> = m.history(&p.id).unwrap().iter().map(|h| (h.op.clone(), h.version, h.body.clone())).collect();
    assert_eq!(ops, [("create".into(), 1, "The user lives in Cairo.".into()), ("edit".into(), 2, "The user lives in Berlin.".into())]);
    // no change, no new version
    assert_eq!(m.update(&p.id, Edit { body: Some("The user lives in Berlin.".into()), ..Default::default() }, By::User).unwrap().version, 2);
}

#[test]
fn archived_pages_leave_normal_recall_but_stay_searchable_and_restorable() {
    let (_d, m) = mem();
    let p = page(&m, Layer::Facts, "Old job", "Worked at a bakery.");
    m.archive(&p.id, By::User, "changed jobs").unwrap();
    assert!(!m.root().join("facts/old-job.md").exists());
    assert!(m.root().join("_archive/facts/old-job-archived.md").exists());
    assert!(m.search("bakery", State::Live, 5).unwrap().is_empty());
    assert_eq!(m.search("bakery", State::Archived, 5).unwrap().len(), 1);
    assert_eq!(m.search("bakery", State::Any, 5).unwrap().len(), 1);
    assert!(m.update(&p.id, Edit { body: Some("x".into()), ..Default::default() }, By::User).is_err());
    assert!(!std::fs::read_to_string(m.root().join("facts/index.md")).unwrap().contains("Old job"));
    let back = m.restore(&p.id, By::User).unwrap();
    assert!(back.archived_at.is_none() && m.root().join("facts/old-job.md").exists());
    assert_eq!(m.search("bakery", State::Live, 5).unwrap().len(), 1);
    let ops: Vec<_> = m.history(&p.id).unwrap().iter().map(|h| h.op.clone()).collect();
    assert_eq!(ops, ["create", "archive", "restore"]);
}

#[test]
fn deleting_removes_the_text_everywhere_but_keeps_the_fact_that_it_happened() {
    let (_d, m) = mem();
    let p = page(&m, Layer::Facts, "Secret", "Tell nobody about the red door.");
    m.update(&p.id, Edit { body: Some("Tell nobody about the blue door.".into()), ..Default::default() }, By::User).unwrap();
    m.delete(&p.id, By::User).unwrap();
    assert!(m.get(&p.id).is_err());
    assert!(m.search("door", State::Any, 5).unwrap().is_empty());
    assert!(!m.root().join("facts/secret.md").exists());
    assert!(std::fs::read_dir(m.root().join("_archive/facts")).map(|mut d| d.next().is_none()).unwrap_or(true), "archived copies are gone too");
    let h = m.history(&p.id).unwrap();
    assert!(h.iter().all(|r| r.body.is_empty() && r.title == "[deleted]"), "no text survives in the history");
    assert_eq!(h.last().unwrap().op, "delete");
}

#[test]
fn the_history_cannot_be_edited_or_removed() {
    let (_d, m) = mem();
    let p = page(&m, Layer::Facts, "T", "body");
    let r = m.history(&p.id).unwrap();
    assert_eq!(r.len(), 1);
    let s = lh_store::Store::open_memory().unwrap();
    s.with(|c| c.execute("insert into page_history(page_id, version, op, by, at, title, body, privacy) values ('x',1,'create','user',1,'t','b','normal')", [])).unwrap();
    assert!(s.with(|c| c.execute("delete from page_history", [])).is_err());
    assert!(s.with(|c| c.execute("update page_history set body = 'changed'", [])).is_err());
    assert!(s.with(|c| c.execute("update page_history set body = '', title = '[deleted]'", [])).is_ok(), "blanking a deleted page is the one allowed change");
}

#[test]
fn hand_edits_are_applied_logged_and_the_old_version_kept() {
    let (_d, m) = mem();
    let p = page(&m, Layer::Preferences, "Tone", "Be terse.");
    assert_eq!(m.scan().unwrap(), ScanReport::default(), "nothing changed, nothing reported");

    // edit the body by hand
    let path = m.root().join("preferences/tone.md");
    let text = std::fs::read_to_string(&path).unwrap().replace("Be terse.", "Be terse, but explain risks.");
    std::fs::write(&path, text).unwrap();
    let rep = m.scan().unwrap();
    assert_eq!(rep.edited, vec![p.id.clone()]);
    let now = m.get(&p.id).unwrap();
    assert_eq!((now.version, now.body.as_str()), (2, "Be terse, but explain risks."));
    assert!(std::fs::read_to_string(m.root().join("_archive/preferences/tone-v1.md")).unwrap().contains("Be terse."));
    assert_eq!(m.history(&p.id).unwrap().last().map(|h| (h.op.clone(), h.by.clone())), Some(("hand_edit".into(), "hand_edit".into())));
    assert_eq!(m.scan().unwrap(), ScanReport::default());

    // mark it sensitive by hand
    let text = std::fs::read_to_string(&path).unwrap().replace("privacy: normal", "privacy: sensitive");
    std::fs::write(&path, text).unwrap();
    m.scan().unwrap();
    assert_eq!(m.get(&p.id).unwrap().privacy, Privacy::Sensitive);

    // a brand-new note dropped into the folder, with no header at all
    std::fs::write(m.root().join("facts/my-note.md"), "# Coffee\n\nI take it black.").unwrap();
    let rep = m.scan().unwrap();
    assert_eq!(rep.created.len(), 1);
    let c = m.get(&rep.created[0]).unwrap();
    assert_eq!((c.layer, c.title.as_str(), c.body.as_str(), c.slug.as_str()), (Layer::Facts, "Coffee", "I take it black.", "my-note"));
    assert!(std::fs::read_to_string(m.root().join("facts/my-note.md")).unwrap().contains(&format!("id: {}", c.id)), "the file now carries an id");
    assert_eq!(m.search("black", State::Live, 5).unwrap().len(), 1);

    // deleting a file archives the page, it does not erase it
    std::fs::remove_file(m.root().join("facts/my-note.md")).unwrap();
    assert_eq!(m.scan().unwrap().archived, vec![c.id.clone()]);
    assert!(m.get(&c.id).unwrap().archived_at.is_some());
}

#[test]
fn a_hand_edit_can_raise_privacy_but_never_lower_it() {
    let (_d, m) = mem();
    let mut np = NewPage::new(Layer::Facts, "Doctor", "Sees Dr. Amal on Tuesdays.");
    np.privacy = Privacy::Sensitive;
    let p = m.create(np, By::User).unwrap();
    let path = m.root().join("facts/doctor.md");

    // the header line flipped to normal: ignored, and the file is put right
    let text = std::fs::read_to_string(&path).unwrap().replace("privacy: sensitive", "privacy: normal");
    std::fs::write(&path, text).unwrap();
    m.scan().unwrap();
    assert_eq!(m.get(&p.id).unwrap().privacy, Privacy::Sensitive);
    assert!(std::fs::read_to_string(&path).unwrap().contains("privacy: sensitive"), "the file says what is true");

    // the same with a text change: the text is kept, the privacy is not lowered, and the history says why
    let text = std::fs::read_to_string(&path).unwrap().replace("privacy: sensitive", "privacy: normal").replace("Tuesdays", "Mondays");
    std::fs::write(&path, text).unwrap();
    m.scan().unwrap();
    let now = m.get(&p.id).unwrap();
    assert_eq!((now.privacy, now.body.as_str()), (Privacy::Sensitive, "Sees Dr. Amal on Mondays."));
    assert!(m.history(&p.id).unwrap().last().unwrap().note.contains("cannot lower"));

    // a normal page edited to hold a password becomes sensitive, header or not
    let q = page(&m, Layer::Facts, "Wifi", "The router is in the hall.");
    let path = m.root().join("facts/wifi.md");
    let text = std::fs::read_to_string(&path).unwrap().replace("in the hall.", "in the hall. password: hunter22");
    std::fs::write(&path, text).unwrap();
    m.scan().unwrap();
    assert_eq!(m.get(&q.id).unwrap().privacy, Privacy::Sensitive);

    // and so does a new file dropped in with no header
    std::fs::write(m.root().join("facts/bank.md"), "# Bank\n\npassword: hunter22").unwrap();
    let rep = m.scan().unwrap();
    assert_eq!(m.get(&rep.created[0]).unwrap().privacy, Privacy::Sensitive);
}

#[test]
fn an_unchanged_folder_is_not_read_again() {
    let (_d, m) = mem();
    let p = page(&m, Layer::Facts, "Tea", "Green.");
    assert_eq!(m.scan_if_changed().unwrap(), ScanReport::default());
    let path = m.root().join("facts/tea.md");
    std::thread::sleep(std::time::Duration::from_millis(20));
    let text = std::fs::read_to_string(&path).unwrap().replace("Green.", "Green, no sugar.");
    std::fs::write(&path, text).unwrap();
    assert_eq!(m.scan_if_changed().unwrap().edited, vec![p.id.clone()], "an edit is seen");
    assert_eq!(m.scan_if_changed().unwrap(), ScanReport::default(), "and only once");
}

#[test]
fn an_emptied_file_is_left_alone() {
    let (_d, m) = mem();
    let p = page(&m, Layer::Facts, "Keep me", "Important.");
    let path = m.root().join("facts/keep-me.md");
    std::fs::write(&path, "---\nid: ".to_string() + &p.id + "\n---\n# Keep me\n\n").unwrap();
    assert!(m.scan().unwrap().edited.is_empty());
    assert_eq!(m.get(&p.id).unwrap().body, "Important.");
}

#[test]
fn privacy_and_pins_are_written_to_the_file_and_history() {
    let (_d, m) = mem();
    let p = page(&m, Layer::Facts, "Bank", "Account at the corner bank.");
    m.set_privacy(&p.id, Privacy::Sensitive, "finance", By::User).unwrap();
    m.set_pinned(&p.id, true, By::User).unwrap();
    let f = std::fs::read_to_string(m.root().join("facts/bank.md")).unwrap();
    assert!(f.contains("privacy: sensitive") && f.contains("pinned: true") && f.contains("privacy_reason: finance"));
    assert_eq!(m.scan().unwrap(), ScanReport::default());
    let ops: Vec<_> = m.history(&p.id).unwrap().iter().map(|h| h.op.clone()).collect();
    assert_eq!(ops, ["create", "reclassify", "pin"]);
    let q = m.get(&p.id).unwrap();
    assert!(q.pinned && q.privacy == Privacy::Sensitive);
}

#[test]
fn export_and_import_round_trip_between_two_installs() {
    let (_d, m) = mem();
    let a = page(&m, Layer::Facts, "Trip", "Berlin in June.");
    let b = page(&m, Layer::Projects, "Decision", "We use SQLite.");
    m.archive(&b.id, By::User, "").unwrap();
    let out = tempfile::tempdir().unwrap();
    m.export(&out.path().join("backup")).unwrap();
    assert!(out.path().join("backup/facts/trip.md").exists());

    let (_d2, m2) = mem();
    let rep = m2.import(&out.path().join("backup")).unwrap();
    assert_eq!((rep.imported, rep.already_present), (1, 0), "live pages are imported; the archive is not");
    assert_eq!(m2.get(&a.id).unwrap().body, "Berlin in June.");
    assert_eq!(m2.history(&a.id).unwrap()[0].by, "import");
    assert_eq!(m2.search("Berlin", State::Live, 5).unwrap().len(), 1);
    let again = m2.import(&out.path().join("backup")).unwrap();
    assert_eq!((again.imported, again.already_present), (0, 1));
}
