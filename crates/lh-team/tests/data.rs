use std::collections::BTreeMap;
use std::sync::Arc;

use lh_learn::Learn;
use lh_store::Store;
use lh_team::*;

struct Fx {
    _d: tempfile::TempDir,
    store: Arc<Store>,
    roles: RoleBook,
    depts: DeptBook,
}
fn fx() -> Fx {
    let d = tempfile::tempdir().unwrap();
    let store = Arc::new(Store::open_memory().unwrap());
    let learn = Arc::new(Learn::new(store.clone()));
    let roles = RoleBook::open(d.path().join("roles"), learn.clone()).unwrap();
    let depts = DeptBook::open(d.path().join("departments"), learn).unwrap();
    Fx { _d: d, store, roles, depts }
}
fn role(name: &str, target: Target) -> Role {
    Role { name: name.into(), description: format!("The {name}."), target, tools: vec!["read_file".into()], memory: MemorySlice::None, sampling: RoleSampling::default(), max_steps: 8, thinking: None, instructions: format!("Be the {name}.") }
}
fn dept(name: &str, lead: &str, members: &[Member]) -> Department {
    Department { name: name.into(), description: String::new(), lead: lead.into(), members: members.to_vec(), parallel: None, attempts: None, instructions: "Work together.".into() }
}
fn avail(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn the_starter_roles_are_written_once_and_deleted_ones_stay_deleted() {
    let f = fx();
    assert_eq!(f.roles.install_starters().unwrap(), 5);
    let names: Vec<_> = f.roles.list().unwrap().into_iter().map(|e| e.name).collect();
    assert_eq!(names, ["coder", "designer", "planner", "researcher", "writer"]);
    assert!(f.roles.list().unwrap().iter().all(|e| e.role.is_ok()));
    assert_eq!(f.roles.get("planner").unwrap().target, Target::Type("frontier".into()));
    f.roles.delete("designer", "user").unwrap();
    assert_eq!(f.roles.install_starters().unwrap(), 0, "the starter set is only written the first time");
    assert!(f.roles.get("designer").is_err());
}

#[test]
fn roles_are_versioned_readable_files_that_can_be_rolled_back() {
    let f = fx();
    let mut r = role("scout", Target::Type("fastest".into()));
    assert_eq!(f.roles.save(&r, "user").unwrap(), 1);
    r.instructions = "Look around and report in one line.".into();
    r.sampling.temperature = Some(0.1);
    assert_eq!(f.roles.save(&r, "user").unwrap(), 2);
    let file = std::fs::read_to_string(f.roles.dir().join("scout.md")).unwrap();
    assert!(file.starts_with("---\nname: scout\n") && file.contains("temperature: 0.1") && file.ends_with("Look around and report in one line.\n"), "{file}");
    // a hand edit of the file is read like any other change
    std::fs::write(f.roles.dir().join("scout.md"), file.replace("max_steps: 8", "max_steps: 3")).unwrap();
    assert_eq!(f.roles.get("scout").unwrap().max_steps, 3);
    assert_eq!(f.roles.rollback("scout", 1, "user").unwrap(), 3);
    assert_eq!(f.roles.get("scout").unwrap().instructions, "Be the scout.");
    assert_eq!(f.roles.history("scout").unwrap().iter().map(|h| h.op.as_str()).collect::<Vec<_>>(), ["create", "edit", "rollback"]);
    assert!(f.roles.rollback("scout", 9, "user").is_err());
    // a broken file is listed with its reason and never used
    std::fs::write(f.roles.dir().join("broken.md"), "no header").unwrap();
    let e = f.roles.list().unwrap();
    assert!(e.iter().find(|x| x.name == "broken").unwrap().role.as_ref().unwrap_err().contains("header"));
    assert!(f.roles.get("broken").is_err());
    f.roles.delete("scout", "user").unwrap();
    assert!(f.roles.history("scout").unwrap().iter().all(|h| h.content.as_object().is_some_and(|o| o.is_empty())));
}

#[test]
fn model_types_start_empty_and_resolve_to_the_model_the_user_chose() {
    let f = fx();
    let t = ModelTypes::new(&f.store);
    assert_eq!(t.list().unwrap().iter().map(|x| (x.name.as_str(), x.model.clone(), x.stock)).collect::<Vec<_>>(), [("frontier", None, true), ("mid-tier", None, true), ("fastest", None, true)]);
    let e = t.resolve(&Target::Type("frontier".into()), &[], UnavailablePolicy::Stop, &|_| true).unwrap_err();
    assert!(e.to_string().contains("has no model yet"), "{e}");
    t.set_model("frontier", Some("big-model")).unwrap();
    t.add("coder", Some("code-model")).unwrap();
    assert!(t.add("Coder", None).is_err() && t.add("bad name", None).is_err());
    let r = t.resolve(&Target::Type("coder".into()), &avail(&["code-model", "big-model"]), UnavailablePolicy::Stop, &|_| true).unwrap();
    assert_eq!((r.model.as_str(), r.note), ("code-model", None));
    // an exact model and an unknown server list ("cannot say") are both fine
    assert_eq!(t.resolve(&Target::Model("anything".into()), &[], UnavailablePolicy::Stop, &|_| true).unwrap().model, "anything");
    assert!(t.resolve(&Target::Model("gone".into()), &avail(&["x"]), UnavailablePolicy::Fallback, &|_| true).is_err());
    assert!(t.resolve(&Target::Type("nope".into()), &[], UnavailablePolicy::Stop, &|_| true).is_err());
}

#[test]
fn a_missing_model_stops_with_a_clear_message_or_falls_back_down_the_list_and_says_so() {
    let f = fx();
    let t = ModelTypes::new(&f.store);
    t.set_model("fastest", Some("tiny")).unwrap();
    t.set_fallbacks("fastest", &avail(&["small", "forbidden", "medium"])).unwrap();
    let up = avail(&["small", "forbidden", "medium"]);
    let stop = t.resolve(&Target::Type("fastest".into()), &up, UnavailablePolicy::Stop, &|_| true).unwrap_err().to_string();
    assert!(stop.contains("tiny") && stop.contains("Pick another model now"), "{stop}");
    let fb = t.resolve(&Target::Type("fastest".into()), &up, UnavailablePolicy::Fallback, &|_| true).unwrap();
    assert_eq!(fb.model, "small");
    assert!(fb.note.unwrap().contains("tiny is not available"));
    // a fallback the sensitive-data rule forbids is skipped
    assert_eq!(t.resolve(&Target::Type("fastest".into()), &up, UnavailablePolicy::Fallback, &|m| m != "small" && m != "forbidden").unwrap().model, "medium");
    let none = t.resolve(&Target::Type("fastest".into()), &up, UnavailablePolicy::Fallback, &|_| false).unwrap_err().to_string();
    assert!(none.contains("none of its fallbacks"), "{none}");
}

#[test]
fn deleting_a_type_that_roles_use_needs_each_role_moved_first() {
    let f = fx();
    let t = ModelTypes::new(&f.store);
    t.add("coder", Some("code-model")).unwrap();
    f.roles.save(&role("a", Target::Type("coder".into())), "u").unwrap();
    f.roles.save(&role("b", Target::Type("coder".into())), "u").unwrap();
    f.roles.save(&role("c", Target::Type("fastest".into())), "u").unwrap();
    match t.delete("coder", &f.roles, &BTreeMap::new()) {
        Err(Error::NeedsReassign(v)) => assert_eq!(v, ["a", "b"]),
        other => panic!("{other:?}"),
    }
    let partial = BTreeMap::from([("a".to_string(), Target::Type("mid-tier".into()))]);
    assert!(matches!(t.delete("coder", &f.roles, &partial), Err(Error::NeedsReassign(v)) if v == ["b"]));
    let to_gone = BTreeMap::from([("a".to_string(), Target::Type("gone".into())), ("b".to_string(), Target::Type("fastest".into()))]);
    assert!(t.delete("coder", &f.roles, &to_gone).is_err());
    assert!(t.get("coder").is_ok(), "nothing changed after a refused delete");
    let ok = BTreeMap::from([("a".to_string(), Target::Type("mid-tier".into())), ("b".to_string(), Target::Model("exact-model".into()))]);
    t.delete("coder", &f.roles, &ok).unwrap();
    assert!(t.get("coder").is_err());
    assert_eq!((f.roles.get("a").unwrap().target, f.roles.get("b").unwrap().target), (Target::Type("mid-tier".into()), Target::Model("exact-model".into())));
    // stock types can be deleted too, when nothing uses them
    t.delete("frontier", &f.roles, &BTreeMap::new()).unwrap();
    assert_eq!(t.list().unwrap().len(), 2);
}

#[test]
fn departments_are_files_with_a_lead_and_members_and_can_nest() {
    let f = fx();
    f.roles.install_starters().unwrap();
    let design = dept("design", "planner", &[Member::Role("designer".into()), Member::Role("writer".into())]);
    f.depts.save(&design, &f.roles, "user").unwrap();
    let mut web = dept("web", "planner", &[Member::Role("coder".into()), Member::Dept("design".into())]);
    web.parallel = Some(2);
    web.attempts = Some(4);
    web.description = "Builds websites.".into();
    f.depts.save(&web, &f.roles, "user").unwrap();
    let back = f.depts.get("web").unwrap();
    assert_eq!(back, web);
    assert!(std::fs::read_to_string(f._d.path().join("departments/web.md")).unwrap().contains("members: coder dept:design"));
    // bad departments are refused with a reason
    let why = |d: Department| f.depts.save(&d, &f.roles, "u").unwrap_err().to_string();
    assert!(why(dept("x", "nobody", &[Member::Role("coder".into())])).contains("lead"));
    assert!(why(dept("x", "planner", &[Member::Role("ghost".into())])).contains("not a role"));
    assert!(why(dept("x", "planner", &[Member::Dept("ghost".into())])).contains("not a department"));
    assert!(why(dept("x", "planner", &[Member::Role("coder".into()), Member::Role("coder".into())])).contains("listed twice"));
    // a department cannot contain itself, directly or through another
    let mut design2 = f.depts.get("design").unwrap();
    design2.members.push(Member::Dept("web".into()));
    assert!(f.depts.save(&design2, &f.roles, "u").unwrap_err().to_string().contains("contain each other"));
    // and one that another contains cannot be deleted
    assert!(f.depts.delete("design", "u").unwrap_err().to_string().contains("contains design"));
    f.depts.delete("web", "u").unwrap();
    f.depts.delete("design", "u").unwrap();
    // versions
    f.depts.save(&design, &f.roles, "u").unwrap();
    let mut d2 = design.clone();
    d2.instructions = "Changed.".into();
    f.depts.save(&d2, &f.roles, "u").unwrap();
    f.depts.rollback("design", 3, "u").unwrap();
    assert_eq!(f.depts.get("design").unwrap().instructions, "Work together.");
}

#[test]
fn settings_have_safe_defaults_limits_and_survive_a_reopen() {
    let f = fx();
    let s = TeamSettings::new(&f.store);
    assert_eq!((s.unavailable().unwrap(), s.parallel().unwrap(), s.autonomy().unwrap(), s.attempts().unwrap(), s.away_since().unwrap()), (UnavailablePolicy::Stop, 1, Autonomy::PlanFirst, 3, None));
    s.set_parallel(3).unwrap();
    s.set_autonomy(Autonomy::CheckIns).unwrap();
    s.set_unavailable(UnavailablePolicy::Fallback).unwrap();
    s.set_attempts(5).unwrap();
    s.set_away(true).unwrap();
    assert_eq!((s.parallel().unwrap(), s.autonomy().unwrap(), s.unavailable().unwrap(), s.attempts().unwrap()), (3, Autonomy::CheckIns, UnavailablePolicy::Fallback, 5));
    assert!(s.away_since().unwrap().is_some());
    s.set_away(false).unwrap();
    assert!(s.away_since().unwrap().is_none());
    assert!(s.set_parallel(0).is_err() && s.set_parallel(99).is_err() && s.set_attempts(0).is_err());
    assert_eq!(f.store.recent(50, Some("setting_changed")).unwrap().len(), 6, "every change is logged");
}

#[test]
fn a_plugin_folder_is_previewed_then_installed_without_replacing_what_the_user_has() {
    use lh_team::plugin;
    let d = tempfile::tempdir().unwrap();
    let store = std::sync::Arc::new(lh_store::Store::open_memory().unwrap());
    let learn = std::sync::Arc::new(lh_learn::Learn::new(store));
    let roles = RoleBook::open(d.path().join("roles"), learn.clone()).unwrap();
    roles.install_starters().unwrap();
    let depts = DeptBook::open(d.path().join("departments"), learn).unwrap();
    let p = d.path().join("legal-kit");
    for sub in ["roles", "departments", "themes"] {
        std::fs::create_dir_all(p.join(sub)).unwrap();
    }
    std::fs::write(p.join("plugin.json"), r#"{"name": "legal-kit", "description": "Contract help."}"#).unwrap();
    std::fs::write(p.join("roles/clause-checker.md"), "---\nname: clause-checker\ndescription: Checks clauses.\ntype: mid-tier\ntools: read_file\n---\nCheck clauses.\n").unwrap();
    std::fs::write(p.join("roles/writer.md"), "---\nname: writer\ndescription: My own writer.\ntype: mid-tier\ntools: read_file\n---\nReplaced writer.\n").unwrap();
    std::fs::write(p.join("roles/broken.md"), "no header").unwrap();
    std::fs::write(p.join("departments/contracts.md"), "---\nname: contracts\nlead: planner\nmembers: clause-checker\n---\n").unwrap();
    std::fs::write(p.join("departments/needs-ghost.md"), "---\nname: needs-ghost\nlead: planner\nmembers: ghost\n---\n").unwrap();
    std::fs::write(p.join("themes/dusk.json"), "{}").unwrap();

    let pv = plugin::preview(&p, &roles, &depts).unwrap();
    assert_eq!((pv.name.as_str(), pv.description.as_str()), ("legal-kit", "Contract help."));
    assert_eq!(pv.roles.iter().map(|r| (r.name.as_str(), r.exists)).collect::<Vec<_>>(), [("clause-checker", false), ("writer", true)]);
    assert_eq!(pv.departments.len(), 2);
    assert_eq!(pv.themes, ["dusk.json"]);
    assert!(pv.problems.len() == 1 && pv.problems[0].starts_with("roles/broken.md"), "{:?}", pv.problems);
    assert!(roles.get("clause-checker").is_err(), "a preview installs nothing");

    let before = roles.get("writer").unwrap().instructions;
    let rep = plugin::install(&p, &roles, &depts, &d.path().join("themes"), &[], "plugin").unwrap();
    assert!(rep.installed.contains(&"role clause-checker".to_string()) && rep.installed.contains(&"department contracts".to_string()) && rep.installed.contains(&"theme dusk.json".to_string()), "{rep:?}");
    assert!(rep.skipped.iter().any(|s| s.starts_with("role writer")) && rep.skipped.iter().any(|s| s.starts_with("department needs-ghost")), "{rep:?}");
    assert_eq!(roles.get("writer").unwrap().instructions, before, "the user's own writer stays");
    assert!(d.path().join("themes/legal-kit/dusk.json").is_file());

    // replacing is explicit, and it is a version that can be rolled back
    plugin::install(&p, &roles, &depts, &d.path().join("themes"), &["writer".to_string()], "plugin").unwrap();
    assert_eq!(roles.get("writer").unwrap().instructions, "Replaced writer.");
    roles.rollback("writer", 1, "user").unwrap();
    assert_eq!(roles.get("writer").unwrap().instructions, before);
    assert!(plugin::preview(&d.path().join("nope"), &roles, &depts).is_err());
}

#[test]
fn the_leads_prompts_stay_small() {
    use lh_team::{check_prompt, failure_prompt, final_prompt, plan_prompt, PlanStep};
    let est = |s: &str| lh_tools::estimate_text_tokens(s);
    let dept = Department { name: "web".into(), description: "Builds websites.".into(), lead: "planner".into(), members: vec![], parallel: None, attempts: None, instructions: "Work together and keep it simple.".into() };
    let members: Vec<(String, String)> = ["researcher", "writer", "coder", "designer", "dept:design"].iter().map(|n| (n.to_string(), "Finds, writes, builds or designs one thing at a time.".to_string())).collect();
    let plan = plan_prompt("Make a landing page for a bakery with a menu and opening hours.", &dept, &members, "");
    let step = PlanStep { id: "1".into(), title: "Write the copy".into(), assignee: "writer".into(), brief: "Write the headline and three short paragraphs.".into(), depends_on: vec![], milestone: None };
    let check = check_prompt("Make a landing page for a bakery.", &step, "Wrote the copy.", &"x".repeat(2500));
    let fail = failure_prompt(&step, "writer", "the build fails", 1, 3, &["researcher".to_string(), "writer".to_string(), "coder".to_string()]);
    let fin = final_prompt("Make a landing page.", &[("Write the copy".to_string(), "x".repeat(800)), ("Build the page".to_string(), "x".repeat(800))], &[]);
    eprintln!("prompt sizes (tokens, chars/3.5): plan {} check {} failure {} final {}", est(&plan), est(&check), est(&fail), est(&fin));
    assert!(est(&plan) < 400, "plan {}", est(&plan));
    assert!(est(&fail) < 300, "failure {}", est(&fail));
    assert!(est(&check) < 1100 && est(&fin) < 1100, "check {} final {}", est(&check), est(&fin));
}
