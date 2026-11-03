use std::sync::Arc;

use lh_memory::*;
use lh_store::Store;

fn mem() -> (tempfile::TempDir, Memory) {
    let d = tempfile::tempdir().unwrap();
    let m = Memory::open(Arc::new(Store::open_memory().unwrap()), d.path().join("m")).unwrap();
    (d, m)
}
fn prop(kind: Kind, title: &str, body: &str, target: Option<&str>) -> NewProposal {
    NewProposal { kind, layer: Layer::Facts, title: title.into(), body: body.into(), gist: "a gist".into(), target_page: target.map(str::to_string), privacy: Privacy::Normal, privacy_reason: String::new(), evidence: "they said so".into(), conv: Some("s1".into()), project: None }
}

#[test]
fn conversations_are_kept_as_said_and_searchable() {
    let (d, m) = mem();
    m.start_conversation("s1", Some("/work/a")).unwrap();
    m.append_message("s1", "user", "I am flying to Berlin next week.").unwrap();
    m.append_message("s1", "assistant", "Nice. Do you need a hotel?").unwrap();
    m.append_message("s1", "user", "رحلتي إلى برلين الأسبوع القادم").unwrap();
    m.end_conversation("s1").unwrap();
    assert_eq!(m.messages("s1").unwrap().len(), 3);
    let hits = m.search_conversations("Berlin", 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!((hits[0].conv.as_str(), hits[0].role.as_str()), ("s1", "user"));
    assert_eq!(m.search_conversations("برلين", 10).unwrap().len(), 1);
    assert!(m.search_conversations("hotel", 10).unwrap().iter().all(|h| h.role == "assistant"));
    // never edited
    assert!(m.search_conversations("zzz", 10).unwrap().is_empty());
    // exported to Markdown
    m.export(&d.path().join("out")).unwrap();
    let md = std::fs::read_to_string(d.path().join("out/conversations/s1.md")).unwrap();
    assert!(md.contains("# Conversation s1") && md.contains("**user:**") && md.contains("flying to Berlin") && md.contains("**assistant:**"));
}

#[test]
fn accepting_a_new_suggestion_makes_a_page_with_the_users_edits() {
    let (_d, m) = mem();
    let id = m.add_proposal(prop(Kind::New, "Home city", "Lives in Berlin.", None)).unwrap().unwrap();
    assert_eq!(m.list_proposals(Some("pending")).unwrap().len(), 1);
    assert!(m.list(None, State::Live).unwrap().is_empty(), "nothing is remembered before the user accepts");
    let page = m.accept_proposal(id, ProposalEdit { body: Some("Lives in Berlin, Kreuzberg.".into()), privacy: Some(Privacy::Sensitive), ..Default::default() }, Some("/work/a".into())).unwrap();
    assert_eq!((page.body.as_str(), page.gist.as_str(), page.privacy, page.project.as_deref(), page.source.as_str()), ("Lives in Berlin, Kreuzberg.", "a gist", Privacy::Sensitive, Some("/work/a"), "s1"));
    assert_eq!(m.list_proposals(Some("accepted")).unwrap().len(), 1);
    assert!(m.accept_proposal(id, ProposalEdit::default(), None).is_err(), "decided once");
}

#[test]
fn a_changed_fact_archives_the_old_version() {
    let (_d, m) = mem();
    let old = m.create(NewPage::new(Layer::Facts, "Home city", "Lives in Cairo."), By::User).unwrap();
    let id = m.add_proposal(prop(Kind::Update, "Home city", "Lives in Berlin.", Some(&old.id))).unwrap().unwrap();
    let page = m.accept_proposal(id, ProposalEdit::default(), None).unwrap();
    assert_eq!((page.id.as_str(), page.version, page.body.as_str()), (old.id.as_str(), 2, "Lives in Berlin."));
    assert!(std::fs::read_to_string(m.root().join("_archive/facts/home-city-v1.md")).unwrap().contains("Cairo"));
    assert_eq!(m.history(&old.id).unwrap().len(), 2);
    assert!(m.search("Cairo", State::Live, 5).unwrap().is_empty());
}

#[test]
fn an_archive_suggestion_archives_and_a_rejection_changes_nothing() {
    let (_d, m) = mem();
    let p = m.create(NewPage::new(Layer::Facts, "Old job", "Worked at a bakery."), By::User).unwrap();
    let a = m.add_proposal(prop(Kind::Archive, "", "", Some(&p.id))).unwrap().unwrap();
    m.accept_proposal(a, ProposalEdit::default(), None).unwrap();
    assert!(m.get(&p.id).unwrap().archived_at.is_some());
    let r = m.add_proposal(prop(Kind::New, "Hobby", "Plays chess.", None)).unwrap().unwrap();
    m.reject_proposal(r).unwrap();
    assert!(m.list(None, State::Live).unwrap().is_empty());
    assert!(m.reject_proposal(r).is_err());
    // a rejected idea may be suggested again later
    assert!(m.add_proposal(prop(Kind::New, "Hobby", "Plays chess.", None)).unwrap().is_some());
}

#[test]
fn repeats_and_things_already_known_are_not_suggested_twice() {
    let (_d, m) = mem();
    assert!(m.add_proposal(prop(Kind::New, "Hobby", "Plays chess.", None)).unwrap().is_some());
    assert!(m.add_proposal(prop(Kind::New, "hobby", "plays CHESS", None)).unwrap().is_none(), "same words, still waiting");
    m.create(NewPage::new(Layer::Facts, "Coffee", "Takes it black."), By::User).unwrap();
    assert!(m.add_proposal(prop(Kind::New, "Coffee", "Takes it black.", None)).unwrap().is_none(), "already a page");
    assert!(m.add_proposal(prop(Kind::New, "", "text", None)).is_err());
}
