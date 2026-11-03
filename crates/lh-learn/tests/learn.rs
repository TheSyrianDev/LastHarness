use std::collections::BTreeMap;
use std::sync::Arc;

use lh_learn::*;
use lh_store::Store;
use serde_json::json;

fn learn() -> (tempfile::TempDir, Learn) {
    let d = tempfile::tempdir().unwrap();
    let l = Learn::new(Arc::new(Store::open_memory().unwrap())).with_skills_dir(d.path().join("skills"));
    (d, l)
}

#[test]
fn a_habit_needs_several_sessions_and_the_same_wish_in_different_words_counts() {
    let (_d, l) = learn();
    l.scan_messages("s1", None, &["Please never use bullet points".into(), "What time is it?".into()]).unwrap();
    l.scan_messages("s2", None, &["Don't give me bullet points, ever".into()]).unwrap();
    l.scan_messages("s2", None, &["Don't give me bullet points, ever".into()]).unwrap();
    assert_eq!(l.signals(None).unwrap().len(), 2, "a repeat inside one session is not counted twice");
    assert!(l.habits(3).unwrap().is_empty(), "two sessions are not yet a habit");
    l.scan_messages("s3", None, &["no bullet points please, I said that before".into(), "Always add links after research".into()]).unwrap();
    l.scan_messages("s4", None, &["Always add source links after you research".into()]).unwrap();
    let h = l.habits(3).unwrap();
    assert_eq!(h.len(), 1);
    assert_eq!((h[0].sessions, h[0].signals.len()), (3, 3));
    assert!(h[0].signals.iter().all(|s| s.text.to_lowercase().contains("bullet")));
    assert_eq!(l.habits(2).unwrap().len(), 2, "the links wish is seen in two sessions");
}

#[test]
fn sensitive_things_said_are_flagged_so_rules_from_them_stay_local() {
    let (_d, l) = learn();
    l.scan_messages("s1", None, &["Always remember that my doctor prescribed metformin".into()]).unwrap();
    let s = &l.signals(None).unwrap()[0];
    assert_eq!(s.privacy, lh_memory::Privacy::Sensitive);
}

#[test]
fn the_models_answer_becomes_suggestions_that_wait_for_the_user() {
    let (_d, l) = learn();
    let reply = "Sure:\n```json\n[{\"kind\":\"rule\",\"topic\":\"Bullets\",\"text\":\"Do not use bullet points; write short paragraphs.\",\"scope\":\"global\",\"evidence\":\"never use bullet points\"},\
        {\"kind\":\"style\",\"text\":\"Be terse.\",\"evidence\":\"short answers\"},\
        {\"kind\":\"rule\",\"topic\":\"tests\",\"text\":\"Run the tests before you finish.\",\"scope\":\"project\"},\
        {\"kind\":\"nonsense\",\"text\":\"x\"}]\n```";
    let drafts = parse_drafts(reply);
    assert_eq!(drafts.len(), 3);
    let ids = l.propose_from_drafts(drafts, "stated", Some("s1"), Some("/work/a"), &["quote".into()]).unwrap();
    assert_eq!(ids.len(), 3);
    assert!(l.list_rules(true).unwrap().is_empty() && l.style().unwrap().is_none(), "nothing applies before the user accepts");
    let p = l.list_proposals(Some("pending")).unwrap();
    assert_eq!(p.iter().map(|x| x.kind).collect::<Vec<_>>(), [Kind::Rule, Kind::Style, Kind::Rule]);
    assert_eq!(p[2].payload["project"], "/work/a", "a project-only rule is tied to the project");

    // accept the first as it is, the third with an edit
    let Applied::Rule(r) = l.accept_proposal(p[0].id, None, "user").unwrap() else { panic!() };
    assert_eq!((r.topic.as_str(), r.source.as_str(), r.evidence.clone()), ("bullets", "proposal", vec!["never use bullet points".to_string()]));
    let Applied::Rule(r3) = l.accept_proposal(p[2].id, Some(&json!({"text": "Run the full test suite before you finish."})), "user").unwrap() else { panic!() };
    assert_eq!((r3.text.as_str(), r3.project.as_deref()), ("Run the full test suite before you finish.", Some("/work/a")));
    let Applied::Style(s) = l.accept_proposal(p[1].id, None, "user").unwrap() else { panic!() };
    assert_eq!(s.text, "Be terse.");
    assert!(l.accept_proposal(p[0].id, None, "user").is_err(), "decided once");
    l.reject_proposal(p[1].id).unwrap_err();
}

#[test]
fn what_is_already_known_is_not_suggested_and_a_new_wording_changes_the_old_rule() {
    let (_d, l) = learn();
    let r = l.add_rule(NewRule { topic: "length".into(), ..NewRule::new("Reply in one line.") }, "user").unwrap();
    l.set_style("Terse. Plain words.", "user").unwrap();
    let d = |kind: &str, topic: &str, text: &str| parse_drafts(&format!("[{{\"kind\":\"{kind}\",\"topic\":\"{topic}\",\"text\":\"{text}\"}}]"));
    assert!(l.propose_from_drafts(d("rule", "length", "Reply in one line."), "repeat", None, None, &[]).unwrap().is_empty(), "same words as an existing rule");
    assert!(l.propose_from_drafts(d("style", "", "Terse."), "repeat", None, None, &[]).unwrap().is_empty(), "already in the style profile");
    let ids = l.propose_from_drafts(d("rule", "length", "Reply in two lines at most."), "repeat", None, None, &[]).unwrap();
    let p = l.get_proposal(ids[0]).unwrap();
    assert_eq!((p.kind, p.payload["target"].as_str(), p.payload["was"].as_str()), (Kind::RuleUpdate, Some(r.id.as_str()), Some("Reply in one line.")));
    // the same suggestion again is not added a second time
    assert!(l.propose_from_drafts(d("rule", "length", "Reply in two lines at most."), "repeat", None, None, &[]).unwrap().is_empty());
    let Applied::Rule(updated) = l.accept_proposal(p.id, None, "user").unwrap() else { panic!() };
    assert_eq!((updated.id.as_str(), updated.version, updated.text.as_str()), (r.id.as_str(), 2, "Reply in two lines at most."));
    // and it can be undone
    assert_eq!(l.rollback_rule(&r.id, 1, "user").unwrap().text, "Reply in one line.");
}

#[test]
fn a_secret_in_a_habit_makes_the_rule_sensitive_even_if_the_model_forgot() {
    let (_d, l) = learn();
    let ids = l.propose_from_drafts(parse_drafts("[{\"kind\":\"rule\",\"topic\":\"wifi\",\"text\":\"The wifi password is hunter2; use it when asked to connect.\"}]"), "stated", None, None, &[]).unwrap();
    let Applied::Rule(r) = l.accept_proposal(ids[0], None, "user").unwrap() else { panic!() };
    assert_eq!(r.privacy, lh_memory::Privacy::Sensitive);
}

fn skill_files(name: &str, body: &str) -> BTreeMap<String, String> {
    let meta = SkillMeta { name: name.into(), description: "Do the thing. Use when asked to do the thing.".into(), license: None, compatibility: None, allowed_tools: vec![], metadata: Default::default() };
    BTreeMap::from([("SKILL.md".to_string(), render_skill_md(&meta, body)), ("references/notes.md".to_string(), "Extra notes.".to_string()), ("scripts/run.sh".to_string(), "echo hi\n".to_string())])
}

#[test]
fn a_skill_is_a_normal_skill_folder_with_versions_and_rollback() {
    let (d, l) = learn();
    l.install_skill(&skill_files("do-thing", "1. Step one."), "create", "user", "").unwrap();
    let s = l.load_skill("do-thing").unwrap();
    assert_eq!((s.meta.name.as_str(), s.body.as_str(), s.files.as_slice()), ("do-thing", "1. Step one.", &["references/notes.md".to_string(), "scripts/run.sh".to_string()][..]));
    assert!(d.path().join("skills/do-thing/SKILL.md").exists());
    assert_eq!(l.read_skill_file("do-thing", "references/notes.md").unwrap(), "Extra notes.");
    for bad in ["../outside", "/etc/passwd", "SKILL.md.bak", "scripts"] {
        assert!(l.read_skill_file("do-thing", bad).is_err(), "{bad}");
    }
    let mut v2 = skill_files("do-thing", "1. Step one.\n2. Step two.");
    v2.remove("scripts/run.sh");
    l.install_skill(&v2, "edit", "user", "added step two").unwrap();
    assert!(!d.path().join("skills/do-thing/scripts/run.sh").exists(), "files no longer in the skill are removed");
    assert_eq!(l.history("skill", "do-thing").unwrap().iter().map(|h| (h.version, h.op.clone())).collect::<Vec<_>>(), [(1, "create".into()), (2, "edit".into())]);
    l.rollback_skill("do-thing", 1, "user").unwrap();
    let back = l.load_skill("do-thing").unwrap();
    assert_eq!(back.body, "1. Step one.");
    assert!(d.path().join("skills/do-thing/scripts/run.sh").exists());
    assert_eq!(l.history("skill", "do-thing").unwrap().last().map(|h| (h.version, h.op.clone())), Some((3, "rollback".into())));

    l.delete_skill("do-thing", "user").unwrap();
    assert!(l.load_skill("do-thing").is_err() && !d.path().join("skills/do-thing").exists());
    assert!(l.history("skill", "do-thing").unwrap().iter().all(|h| h.content.as_object().is_some_and(|o| o.is_empty())));
}

#[test]
fn the_list_shows_valid_skills_and_explains_the_broken_ones_without_offering_them() {
    let (d, l) = learn();
    l.install_skill(&skill_files("good-one", "Steps."), "create", "user", "").unwrap();
    std::fs::create_dir_all(d.path().join("skills/bad-one")).unwrap();
    std::fs::write(d.path().join("skills/bad-one/SKILL.md"), "no header at all").unwrap();
    std::fs::create_dir_all(d.path().join("skills/wrong-name")).unwrap();
    std::fs::write(d.path().join("skills/wrong-name/SKILL.md"), "---\nname: other\ndescription: x\n---\nb").unwrap();
    let list = l.list_skills().unwrap();
    assert_eq!(list.len(), 3);
    assert!(list.iter().find(|e| e.name == "good-one").unwrap().error.is_none());
    assert!(list.iter().find(|e| e.name == "bad-one").unwrap().error.as_deref().unwrap().contains("header"));
    assert!(list.iter().find(|e| e.name == "wrong-name").unwrap().error.as_deref().unwrap().contains("must match"));
    assert!(l.load_skill("bad-one").is_err());
}

#[test]
fn skill_suggestions_create_and_change_skills_only_when_accepted() {
    let (_d, l) = learn();
    let draft = parse_skill_draft("```json\n{\"worth\":true,\"name\":\"release-notes\",\"description\":\"Write release notes from git history. Use before a release.\",\"body\":\"1. Run git log.\\n2. Group by type.\"}\n```").unwrap();
    let id = l.add_proposal(Kind::SkillNew, json!({"name": draft.name, "description": draft.description, "body": draft.body}), vec!["after 9 tool calls".into()], "skill", Some("s1")).unwrap().unwrap();
    assert!(l.list_skills().unwrap().is_empty(), "not a skill yet");
    l.accept_proposal(id, None, "user").unwrap();
    assert_eq!(l.load_skill("release-notes").unwrap().body, "1. Run git log.\n2. Group by type.");
    assert!(l.add_proposal(Kind::SkillNew, json!({"name": "release-notes", "description": "x", "body": "1. Run git log.\n2. Group by type."}), vec![], "skill", None).unwrap().is_none(), "same suggestion");

    let (body, note) = parse_skill_change("{\"change\":true,\"body\":\"1. Run git log since the last tag.\\n2. Group by type.\",\"note\":\"use the last tag\"}").unwrap();
    let id = l.add_proposal(Kind::SkillUpdate, json!({"name": "release-notes", "body": body, "note": note}), vec![], "skill", None).unwrap().unwrap();
    l.accept_proposal(id, None, "user").unwrap();
    let s = l.load_skill("release-notes").unwrap();
    assert!(s.body.contains("since the last tag") && s.meta.description.starts_with("Write release notes"), "the description is kept");
    assert_eq!(l.history("skill", "release-notes").unwrap().last().unwrap().note, "use the last tag");
    // a new skill cannot overwrite an existing one
    let id = l.add_proposal(Kind::SkillNew, json!({"name": "release-notes", "description": "d", "body": "other"}), vec![], "skill", None).unwrap().unwrap();
    assert!(l.accept_proposal(id, None, "user").is_err());
    assert!(parse_skill_draft("{\"worth\":false}").is_none() && parse_skill_draft("{\"worth\":true,\"name\":\"Bad Name\",\"description\":\"d\",\"body\":\"b\"}").is_none());
    assert!(parse_skill_change("{\"change\":false}").is_none());
}

/// The skills that ship with Omarchy are real files made by someone else; ours must read them.
#[test]
fn real_skills_from_other_tools_are_read() {
    let root = std::path::Path::new("/usr/share/omarchy/default/agents/skills");
    let Ok(rd) = std::fs::read_dir(root) else { return eprintln!("skipped: no Omarchy skills on this machine") };
    let mut seen = 0;
    for e in rd.flatten().filter(|e| e.path().is_dir()) {
        let folder = e.file_name().to_string_lossy().to_string();
        let text = std::fs::read_to_string(e.path().join("SKILL.md")).unwrap();
        let (meta, body) = parse_skill_md(&text, Some(&folder)).unwrap_or_else(|err| panic!("{folder}: {err}"));
        assert!(meta.description.len() > 40 && !meta.description.contains('\n') && !body.is_empty(), "{folder}");
        seen += 1;
    }
    assert!(seen >= 1);
    // and a skills folder pointed straight at them lists them without error
    let l = Learn::new(Arc::new(Store::open_memory().unwrap())).with_skills_dir(root);
    assert!(l.list_skills().unwrap().iter().all(|s| s.error.is_none()));
    let d = l.load_skill("diagnose-crash").unwrap();
    assert!(d.files.contains(&"reporting.md".to_string()));
}

#[test]
fn the_same_wish_in_new_words_adds_evidence_to_the_waiting_suggestion_instead_of_a_second_one() {
    let (_d, l) = learn();
    let first = l.propose_from_drafts(parse_drafts(r#"[{"kind":"style","text":"Never use bullet points; write in short paragraphs instead.","evidence":"never use bullet points"}]"#), "stated", Some("s1"), None, &[]).unwrap();
    assert_eq!(first.len(), 1);
    // other sessions say it again, once as a style note and once as a rule
    for (kind, text, ev) in [("style", "Use paragraphs only, never bullet points.", "paragraphs only"), ("rule", "No bullet points, write paragraphs.", "no bullet points please")] {
        let again = l.propose_from_drafts(parse_drafts(&format!("[{{\"kind\":\"{kind}\",\"topic\":\"bullets\",\"text\":\"{text}\",\"evidence\":\"{ev}\"}}]")), "repeat", Some("s2"), None, &[]).unwrap();
        assert!(again.is_empty(), "{text}");
    }
    let p = l.list_proposals(Some("pending")).unwrap();
    assert_eq!(p.len(), 1);
    assert_eq!(p[0].source, "repeat", "seen in other sessions makes it a repeat");
    assert_eq!(p[0].evidence, ["never use bullet points", "paragraphs only", "no bullet points please"]);
    // a different wish is still its own suggestion
    assert_eq!(l.propose_from_drafts(parse_drafts(r#"[{"kind":"rule","topic":"links","text":"Always add source links after research."}]"#), "stated", None, None, &[]).unwrap().len(), 1);
    // once decided, the same wish is not suggested again either
    l.accept_proposal(first[0], None, "user").unwrap();
    assert!(l.propose_from_drafts(parse_drafts(r#"[{"kind":"style","text":"Never use bullet points, only paragraphs."}]"#), "repeat", None, None, &[]).unwrap().is_empty());
}
