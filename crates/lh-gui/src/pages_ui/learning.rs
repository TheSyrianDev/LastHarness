//! Learning: suggestions and habits waiting for an answer, the rules in force, the style text, and skills.
//! The same things `lh learn` does, with the history of each and a way back.

use std::collections::{BTreeMap, HashSet};

use gpui::prelude::*;
use gpui::*;
use serde_json::{json, Value};

use crate::app::App;
use crate::ui::Kind;
use lh_gui::pages_logic::arr;
use lh_gui::pages_logic::learning::{self as logic, HabitRow, ProposalRow, RuleRow, SkillPreview, SkillRow};
use lh_gui::pages_logic::{versions, Version};

#[derive(Default)]
pub struct State {
    pub edit_rule: Option<String>,
    pub history_rule: Option<String>,
    pub style_history: bool,
    pub open_skill: Option<String>,
    pub edit_proposal: Option<i64>,
    pub dismissed_habits: HashSet<String>,
    pub preview: Option<(BTreeMap<String, String>, SkillPreview)>,
}

impl App {
    pub(super) fn learning_block(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let mut col = div().flex().flex_col().gap_2();
        if !self.connected {
            return col.child(self.pg_small("The core is not running, so learning cannot be shown.")).into_any_element();
        }
        if let Some(e) = ["rules", "style", "skills", "proposals", "habits"].iter().find_map(|n| self.pg_error(n)) {
            col = col.child(e);
        }
        let proposals = self.sp.data.get("proposals").map(logic::proposals).unwrap_or_default();
        let habits: Vec<HabitRow> = self.sp.data.get("habits").map(logic::habits).unwrap_or_default().into_iter().filter(|h| !self.sp.learning.dismissed_habits.contains(&h.example)).collect();
        if !proposals.is_empty() || !habits.is_empty() {
            col = col.child(self.pg_heading("Waiting for you"));
            for pr in proposals {
                col = col.child(self.proposal_card(pr, window, cx));
            }
            for (i, h) in habits.into_iter().enumerate() {
                col = col.child(self.habit_card(i, h, cx));
            }
        }
        col = col.child(self.rules_section(window, cx)).child(self.style_section(window, cx)).child(self.skills_section(window, cx));
        col.into_any_element()
    }

    // ---- suggestions and habits ------------------------------------------------------------

    fn proposal_card(&mut self, pr: ProposalRow, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let id = pr.id;
        let editing = self.sp.learning.edit_proposal == Some(id);
        let mut card = self.pg_card().border_1().border_color(p.accent).id(SharedString::from(format!("proposal-{id}"))).role(Role::Group).aria_label(pr.headline.clone());
        card = card.child(div().min_w_0().child(pr.headline.clone()));
        if !pr.editable.is_empty() && !editing {
            card = card.child(self.pg_small(format!("It would say: {}", pr.editable)));
        }
        if !pr.because.is_empty() {
            card = card.child(self.pg_small(format!("Because you said: \"{}\"", pr.because)));
        }
        if pr.sensitive {
            card = card.child(self.pg_small("Sensitive: it stays on this computer."));
        }
        let key = format!("prop.{id}");
        if editing {
            card = card.child(self.pg_text_area("Your wording", &key, "", 2, window, cx));
        }
        let row = pr.clone();
        let accept = self.button(format!("prop-accept-{id}"), if editing { "Accept my wording" } else { "Accept" }, Kind::Primary, cx, {
            let key = key.clone();
            move |a, _, cx| {
                let typed = if a.sp.learning.edit_proposal == Some(row.id) { a.pg_val(&key, cx) } else { String::new() };
                let body = logic::accept_body(&row, &typed);
                a.sp.learning.edit_proposal = None;
                a.pg_do("proposals", "POST", format!("/v1/learn/proposals/{}/accept", row.id), Some(body), &["proposals", "rules", "style", "skills"], "Done. It is part of what the assistant knows now.", cx);
            }
        });
        let edit = (!pr.editable.is_empty()).then(|| {
            let (text, key) = (pr.editable.clone(), key.clone());
            self.button(format!("prop-edit-{id}"), if editing { "Cancel" } else { "Edit" }, Kind::Normal, cx, move |a, w, cx| {
                if a.sp.learning.edit_proposal == Some(id) {
                    a.sp.learning.edit_proposal = None;
                } else {
                    a.sp.learning.edit_proposal = Some(id);
                    a.pg_set(&key, &text, w, cx);
                }
                cx.notify();
            })
        });
        let reject = self.button(format!("prop-reject-{id}"), "Reject", Kind::Ghost, cx, move |a, _, cx| {
            a.pg_do("proposals", "POST", format!("/v1/learn/proposals/{id}/reject"), Some(json!({})), &["proposals"], "Dropped.", cx);
        });
        card.child(div().flex().flex_wrap().gap_1().child(accept).children(edit).child(reject)).into_any_element()
    }

    fn habit_card(&mut self, i: usize, h: HabitRow, cx: &mut Context<Self>) -> AnyElement {
        let text = h.example.clone();
        let make = self.button(format!("habit-make-{i}"), "Make it a rule", Kind::Primary, cx, {
            let t = text.clone();
            move |a, _, cx| {
                a.sp.learning.dismissed_habits.insert(t.clone());
                a.pg_do("rules", "POST", "/v1/learn/rules".into(), Some(json!({"text": t})), &["rules", "habits"], "Added as a rule.", cx);
            }
        });
        let edit = self.button(format!("habit-edit-{i}"), "Edit first", Kind::Normal, cx, {
            let t = text.clone();
            move |a, w, cx| {
                a.pg_set("rule.new", &t, w, cx);
                cx.notify();
            }
        });
        let no = self.button(format!("habit-no-{i}"), "Not this", Kind::Ghost, cx, {
            let t = text.clone();
            move |a, _, cx| {
                a.sp.learning.dismissed_habits.insert(t.clone());
                cx.notify();
            }
        });
        self.pg_card()
            .id(SharedString::from(format!("habit-{i}")))
            .role(Role::Group)
            .aria_label("A habit")
            .child(div().min_w_0().child(format!("You have asked for this in {} chats: {}", h.sessions, text)))
            .child(div().flex().flex_wrap().gap_1().child(make).child(edit).child(no))
            .into_any_element()
    }

    // ---- rules ---------------------------------------------------------------------------------

    fn rules_section(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let rules: Vec<RuleRow> = self.sp.data.get("rules").map(logic::rules).unwrap_or_default();
        let mut col = div().flex().flex_col().gap_2().child(self.pg_heading("Rules")).child(self.pg_small("Things the assistant always keeps to. Switch one off to stop it without losing it."));
        let new_box = self.pg_input("rule.new", "Write a rule, like: Always give the source for a fact", false, window, cx);
        let add = self.button("rule-add", "Add rule", Kind::Primary, cx, |a, _, cx| {
            let text = a.pg_val("rule.new", cx);
            if text.trim().is_empty() {
                a.sp.notes.insert("rules".into(), (false, "Write the rule first.".into()));
                cx.notify();
                return;
            }
            a.pg_do("rules", "POST", "/v1/learn/rules".into(), Some(json!({"text": text.trim()})), &["rules"], "Rule added.", cx);
        });
        col = col.child(div().flex().items_center().gap_1().child(div().flex_1().min_w_0().child(gpui_component::input::Input::new(&new_box).aria_label("New rule"))).child(add));
        if let Some(n) = self.pg_note("rules") {
            col = col.child(n);
        }
        if rules.is_empty() {
            col = col.child(self.pg_small("No rules yet."));
        }
        for r in rules {
            col = col.child(self.rule_card(r, window, cx));
        }
        col.into_any_element()
    }

    fn rule_card(&mut self, r: RuleRow, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let id = r.id.clone();
        let editing = self.sp.learning.edit_rule.as_deref() == Some(id.as_str());
        let history_open = self.sp.learning.history_rule.as_deref() == Some(id.as_str());
        let switch = {
            let id = id.clone();
            self.glass_switch(format!("rule-on-{id}"), r.active, true, "Rule on", false, cx, move |a, on, _, cx| {
                a.pg_do("rules", "POST", format!("/v1/learn/rules/{id}/status"), Some(json!({"active": on})), &["rules"], "", cx);
            })
        };
        let mut tags: Vec<String> = r.scope.clone();
        if r.sensitive {
            tags.push("sensitive, stays on this computer".into());
        }
        if !r.active {
            tags.push("off".into());
        }
        let mut card = self.pg_card().id(SharedString::from(format!("rule-{id}"))).role(Role::Group).aria_label(r.text.clone());
        let head = div().flex().items_start().gap_2().child(switch).child(div().flex_1().min_w_0().text_color(if r.active { p.text } else { p.muted }).child(r.text.clone()));
        card = card.child(head);
        if !tags.is_empty() || !r.source_words().is_empty() {
            let line = [r.source_words().to_string(), tags.join(", ")].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(". ");
            card = card.child(self.pg_small(line));
        }
        if editing {
            let key = format!("rule.edit.{id}");
            card = card.child(self.pg_text_area("New wording", &key, "", 2, window, cx));
            let save = self.button(format!("rule-save-{id}"), "Save", Kind::Primary, cx, {
                let (id, key) = (id.clone(), key.clone());
                move |a, _, cx| {
                    let text = a.pg_val(&key, cx);
                    if text.trim().is_empty() {
                        a.sp.notes.insert("rules".into(), (false, "A rule cannot be empty. Delete it instead.".into()));
                        cx.notify();
                        return;
                    }
                    a.sp.learning.edit_rule = None;
                    a.pg_do("rules", "PATCH", format!("/v1/learn/rules/{id}"), Some(json!({"text": text.trim()})), &["rules", &format!("rule:{id}")], "Rule changed. The old wording stays in its history.", cx);
                }
            });
            let cancel = self.button(format!("rule-cancel-{id}"), "Cancel", Kind::Ghost, cx, |a, _, cx| {
                a.sp.learning.edit_rule = None;
                cx.notify();
            });
            card = card.child(div().flex().gap_1().child(save).child(cancel));
        } else {
            let edit = self.button(format!("rule-edit-{id}"), "Edit", Kind::Normal, cx, {
                let (id, text) = (id.clone(), r.text.clone());
                move |a, w, cx| {
                    a.sp.learning.edit_rule = Some(id.clone());
                    a.pg_set(&format!("rule.edit.{id}"), &text, w, cx);
                    cx.notify();
                }
            });
            let hist = self.button(format!("rule-history-{id}"), if history_open { "Hide history" } else { "History" }, Kind::Ghost, cx, {
                let id = id.clone();
                move |a, _, cx| {
                    if a.sp.learning.history_rule.as_deref() == Some(id.as_str()) {
                        a.sp.learning.history_rule = None;
                    } else {
                        a.sp.learning.history_rule = Some(id.clone());
                        a.pg_get(&format!("rule:{id}"));
                    }
                    cx.notify();
                }
            });
            let del = self.pg_confirm(&format!("rule-delete-{id}"), "Delete", "Delete this rule and its history for good?", cx, {
                let id = id.clone();
                move |a, _, cx| a.pg_do("rules", "DELETE", format!("/v1/learn/rules/{id}"), None, &["rules"], "Deleted for good.", cx)
            });
            card = card.child(div().flex().flex_wrap().items_center().gap_1().child(edit).child(hist).child(del));
        }
        if history_open {
            let list = self.sp.data.get(&format!("rule:{id}")).map(|v| versions(arr(v, "history"), |h| h["content"]["text"].as_str().unwrap_or("").to_string())).unwrap_or_default();
            card = card.child(self.history_list(&format!("rule-v-{id}"), list, cx, {
                let id = id.clone();
                move |a, ver, cx| a.pg_do("rules", "POST", format!("/v1/learn/rules/{id}/rollback"), Some(json!({"version": ver})), &["rules", &format!("rule:{id}")], &format!("Went back to version {ver}."), cx)
            }));
        }
        card.into_any_element()
    }

    /// A list of versions, newest first, each older one with a button to go back to it.
    fn history_list(&mut self, id: &str, list: Vec<Version>, cx: &mut Context<Self>, back: impl Fn(&mut App, i64, &mut Context<App>) + Clone + 'static) -> AnyElement {
        let p = self.pal.clone();
        if list.is_empty() {
            return self.pg_small("Loading the history…").into_any_element();
        }
        let rows: Vec<AnyElement> = list
            .into_iter()
            .enumerate()
            .map(|(n, v)| {
                let go = (n > 0).then(|| {
                    let back = back.clone();
                    let num = v.number;
                    self.button(format!("{id}-back-{num}"), "Go back to this", Kind::Ghost, cx, move |a, _, cx| back(a, num, cx))
                });
                div().flex().flex_wrap().items_center().gap_2().child(div().flex_1().min_w_0().text_sm().child(format!("Version {}, {}{}", v.number, v.line, if n == 0 { " (now)" } else { "" }))).children(go).into_any_element()
            })
            .collect();
        div().flex().flex_col().gap_1().p_1().rounded(p.inner).bg(p.button).id(SharedString::from(format!("{id}-list"))).role(Role::List).children(rows).into_any_element()
    }

    // ---- style ---------------------------------------------------------------------------------

    fn style_section(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let mut col = div().flex().flex_col().gap_2().child(self.pg_heading("How I talk to you")).child(self.pg_small("A short note on tone and length. It goes with every chat, so keep it short."));
        col = col.child(self.pg_text_area("Style", "style", "For example: Short answers. Plain words. No filler.", 3, window, cx));
        let save = self.button("style-save", "Save", Kind::Primary, cx, |a, _, cx| {
            let t = a.pg_val("style", cx);
            a.pg_do("style", "PUT", "/v1/learn/style".into(), Some(json!({"text": t})), &["style"], "Saved. The old text stays in the history.", cx);
        });
        let hist = self.button("style-history", if self.sp.learning.style_history { "Hide history" } else { "History" }, Kind::Ghost, cx, |a, _, cx| {
            a.sp.learning.style_history = !a.sp.learning.style_history;
            cx.notify();
        });
        col = col.child(div().flex().gap_1().child(save).child(hist));
        if self.sp.learning.style_history {
            let list = self.sp.data.get("style").map(|v| versions(arr(v, "history"), |h| lh_gui::model::truncate_title(h["content"]["text"].as_str().unwrap_or(""), 80))).unwrap_or_default();
            col = col.child(self.history_list("style-v", list, cx, |a, ver, cx| a.pg_do("style", "POST", "/v1/learn/style/rollback".into(), Some(json!({"version": ver})), &["style"], &format!("Went back to the text of version {ver}."), cx)));
        }
        if let Some(n) = self.pg_note("style") {
            col = col.child(n);
        }
        col.into_any_element()
    }

    // ---- skills --------------------------------------------------------------------------------

    fn skills_section(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let skills: Vec<SkillRow> = self.sp.data.get("skills").map(logic::skills).unwrap_or_default();
        let mut col = div().flex().flex_col().gap_2().child(self.pg_heading("Skills")).child(self.pg_small("Ready-made know-how the assistant reads when a task needs it."));
        if skills.is_empty() {
            col = col.child(self.pg_small("No skills yet."));
        }
        for s in skills {
            col = col.child(self.skill_card(s, cx));
        }
        col = col.child(self.install_skill(window, cx));
        if let Some(n) = self.pg_note("skills") {
            col = col.child(n);
        }
        col.into_any_element()
    }

    fn skill_card(&mut self, s: SkillRow, cx: &mut Context<Self>) -> AnyElement {
        let name = s.name.clone();
        let open = self.sp.learning.open_skill.as_deref() == Some(name.as_str());
        let mut card = self.pg_card().id(SharedString::from(format!("skill-{name}"))).role(Role::Group).aria_label(name.clone());
        card = card.child(div().min_w_0().child(name.clone()));
        match &s.broken {
            Some(e) => card = card.child(div().text_sm().text_color(self.pal.danger).child(format!("This skill is broken: {e}"))),
            None => card = card.child(self.pg_small(s.description.clone())),
        }
        let show = self.button(format!("skill-show-{name}"), if open { "Hide" } else { "Show contents" }, Kind::Normal, cx, {
            let name = name.clone();
            move |a, _, cx| {
                if a.sp.learning.open_skill.as_deref() == Some(name.as_str()) {
                    a.sp.learning.open_skill = None;
                } else {
                    a.sp.learning.open_skill = Some(name.clone());
                    a.pg_get(&format!("skill:{name}"));
                }
                cx.notify();
            }
        });
        let del = self.pg_confirm(&format!("skill-delete-{name}"), "Delete", "Delete this skill and its history for good?", cx, {
            let name = name.clone();
            move |a, _, cx| {
                a.sp.learning.open_skill = None;
                a.pg_do("skills", "DELETE", format!("/v1/learn/skills/{name}"), None, &["skills"], "Deleted for good.", cx)
            }
        });
        card = card.child(div().flex().flex_wrap().items_center().gap_1().child(show).child(del));
        if open {
            match self.sp.data.get(&format!("skill:{name}")).cloned() {
                None => card = card.child(self.pg_small("Loading…")),
                Some(v) => {
                    let files: Vec<&str> = arr(&v, "files").iter().filter_map(Value::as_str).collect();
                    if !files.is_empty() {
                        card = card.child(self.pg_small(format!("Other files: {}", files.join(", "))));
                    }
                    card = card.child(div().id(SharedString::from(format!("skill-body-{name}"))).max_h(px(260.)).overflow_y_scroll().p_2().rounded(self.pal.inner).bg(self.pal.button).text_sm().child(v["body"].as_str().unwrap_or("").to_string()));
                    let list = versions(arr(&v, "history"), |h| h["note"].as_str().unwrap_or("").to_string());
                    card = card.child(self.pg_small("Versions")).child(self.history_list(&format!("skill-v-{name}"), list, cx, {
                        let name = name.clone();
                        move |a, ver, cx| a.pg_do("skills", "POST", format!("/v1/learn/skills/{name}/rollback"), Some(json!({"version": ver})), &["skills", &format!("skill:{name}")], &format!("Went back to version {ver}."), cx)
                    }));
                }
            }
        }
        card.into_any_element()
    }

    /// Install from a folder: choose it, look at what it holds, then say yes.
    fn install_skill(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let folder = self.pg_input("skill.folder", "/path/to/a/skill/folder", false, window, cx);
        let choose = self.button("skill-choose", "Choose folder", Kind::Normal, cx, |_, _, cx| {
            let rx = cx.prompt_for_paths(PathPromptOptions { files: false, directories: true, multiple: false, prompt: Some("Choose".into()) });
            cx.spawn(async move |this: WeakEntity<App>, cx: &mut AsyncApp| {
                if let Ok(Ok(Some(paths))) = rx.await {
                    if let Some(path) = paths.first() {
                        let path = path.to_string_lossy().to_string();
                        let _ = this.update(cx, |a, cx| {
                            a.sp.fill.push(("skill.folder".into(), path));
                            cx.notify();
                        });
                    }
                }
            })
            .detach();
        });
        let look = self.button("skill-preview", "Look inside", Kind::Primary, cx, |a, _, cx| {
            let dir = a.pg_val("skill.folder", cx);
            a.sp.notes.remove("skills");
            match logic::read_skill_folder(std::path::Path::new(dir.trim())) {
                Ok(files) => {
                    let pv = logic::preview(&files);
                    a.sp.learning.preview = Some((files, pv));
                }
                Err(e) => {
                    a.sp.learning.preview = None;
                    a.sp.notes.insert("skills".into(), (false, if dir.trim().is_empty() { "Choose a folder first.".to_string() } else { e }));
                }
            }
            cx.notify();
        });
        let mut col = self.pg_card().child(div().child("Add a skill from a folder")).child(self.pg_small("The folder needs a SKILL.md file. Nothing is installed until you say so.")).child(div().flex().items_center().gap_1().child(div().flex_1().min_w_0().child(gpui_component::input::Input::new(&folder).aria_label("Skill folder"))).child(choose).child(look));
        if let Some((files, pv)) = self.sp.learning.preview.clone() {
            let mut pc = div().flex().flex_col().gap_1().p_2().rounded(self.pal.inner).bg(self.pal.button).id("skill-preview-card").role(Role::Group).aria_label("What the folder holds");
            pc = pc.child(div().child(if pv.name.is_empty() { "(no name)".to_string() } else { pv.name.clone() })).child(self.pg_small(pv.description.clone()));
            pc = pc.child(self.pg_small(format!("{} file{}, {} bytes: {}", pv.files.len(), if pv.files.len() == 1 { "" } else { "s" }, pv.bytes, pv.files.join(", "))));
            match &pv.problem {
                Some(m) => pc = pc.child(div().text_sm().text_color(self.pal.danger).child(m.clone())),
                None => {
                    let go = self.button("skill-install", "Install", Kind::Primary, cx, move |a, _, cx| {
                        a.pg_do("skills", "POST", "/v1/learn/skills".into(), Some(json!({"files": files.clone()})), &["skills"], "Installed.", cx);
                    });
                    pc = pc.child(div().flex().child(go));
                }
            }
            col = col.child(pc);
        }
        col.into_any_element()
    }
}
