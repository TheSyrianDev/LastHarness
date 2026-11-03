//! Departments: the departments and roles editor, model types, team settings, away mode, and the history of
//! each role and department with a way back. The same things `lh team` does.

use std::collections::BTreeMap;

use gpui::prelude::*;
use gpui::*;
use serde_json::{json, Value};

use crate::app::App;
use crate::ui::Kind;
use lh_gui::pages_logic::arr;
use lh_gui::pages_logic::team::{self as logic, AwayView, DeptForm, Home, RoleForm, RoleTarget, TeamSettingsView, TypeRow, AUTONOMY, UNAVAILABLE};
use lh_gui::pages_logic::versions;

/// A type being deleted, and where the roles that use it go.
pub struct TypeDelete {
    pub name: String,
    pub affected: Vec<String>,
    pub homes: BTreeMap<String, Home>,
}

#[derive(Default)]
pub struct State {
    /// The role form: whether the role is new, and what is in it besides the text boxes.
    pub role: Option<(bool, RoleForm)>,
    pub dept: Option<(bool, DeptForm)>,
    pub role_history: Option<String>,
    pub dept_history: Option<String>,
    /// A model type being edited (its name), or `Some("")` for a new one.
    pub type_edit: Option<String>,
    pub type_delete: Option<TypeDelete>,
}

/// The model types as drop-down options, plus the words for "no model yet".
fn type_options(types: &[TypeRow]) -> Vec<(String, String)> {
    types.iter().map(|t| (t.name.clone(), t.name.clone())).collect()
}

impl App {
    pub(super) fn team_block(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let mut col = div().flex().flex_col().gap_2();
        if !self.connected {
            return col.child(self.pg_small("The core is not running, so departments cannot be shown.")).into_any_element();
        }
        if let Some(e) = ["roles", "types", "depts", "tsettings", "away"].iter().find_map(|n| self.pg_error(n)) {
            col = col.child(e);
        }
        col = col.child(self.team_settings_section(cx)).child(self.away_section(cx));
        col = col.child(self.depts_section(window, cx)).child(self.roles_section(window, cx)).child(self.types_section(window, cx));
        col.into_any_element()
    }

    // ---- team settings and away ----------------------------------------------------------

    fn team_settings_section(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let Some(t) = self.sp.data.get("tsettings").map(TeamSettingsView::from_json) else { return div().into_any_element() };
        let save = |field: &'static str| move |a: &mut App, v: Value, cx: &mut Context<App>| a.pg_do("tsettings", "PUT", "/v1/team/settings".into(), Some(json!({ field: v })), &["tsettings"], "Saved.", cx);
        let autonomy = {
            let s = save("autonomy");
            self.pg_dropdown("ts-autonomy", logic::words(&AUTONOMY, &t.autonomy), AUTONOMY.iter().map(|(k, w)| (w.to_string(), k.to_string())).collect(), &t.autonomy, cx, move |a, v, _, cx| s(a, json!(v), cx))
        };
        let unavailable = {
            let s = save("unavailable");
            self.pg_dropdown("ts-unavailable", logic::words(&UNAVAILABLE, &t.unavailable), UNAVAILABLE.iter().map(|(k, w)| (w.to_string(), k.to_string())).collect(), &t.unavailable, cx, move |a, v, _, cx| s(a, json!(v), cx))
        };
        let stepper = |this: &mut App, id: &str, value: u64, min: u64, max: u64, field: &'static str, cx: &mut Context<App>| {
            let s = save(field);
            let s2 = save(field);
            let minus = this.button(format!("{id}-minus"), "−", Kind::Ghost, cx, move |a, _, cx| s(a, json!(value.saturating_sub(1).max(min)), cx));
            let plus = this.button(format!("{id}-plus"), "+", Kind::Ghost, cx, move |a, _, cx| s2(a, json!((value + 1).min(max)), cx));
            div().flex().items_center().gap_1().child(minus).child(div().w(px(36.)).flex().justify_center().child(value.to_string())).child(plus).into_any_element()
        };
        let parallel = stepper(self, "ts-parallel", t.parallel, 1, t.parallel_max, "parallel", cx);
        let attempts = stepper(self, "ts-attempts", t.attempts, 1, t.attempts_max, "attempts", cx);
        let row = |label: &str, c: AnyElement| div().flex().flex_wrap().items_center().gap_2().child(div().flex_1().min_w(px(180.)).child(label.to_string())).child(c);
        let mut card = self.pg_card().id("team-settings").role(Role::Group).aria_label("Team settings");
        card = card
            .child(row("Workers at the same time", parallel))
            .child(row("Tries for each step", attempts))
            .child(row("How much departments ask you", autonomy))
            .child(row("When a model is missing", unavailable));
        if let Some(n) = self.pg_note("tsettings") {
            card = card.child(n);
        }
        div().flex().flex_col().gap_2().child(self.pg_heading("Team settings")).child(card).into_any_element()
    }

    fn away_section(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let v = self.sp.data.get("away").map(AwayView::from_json).unwrap_or_default();
        let switch = self.glass_switch("away-switch", v.away, true, "Away mode", true, cx, |a, on, _, cx| {
            a.pg_do("away", "POST", "/v1/team/away".into(), Some(json!({"on": on})), &["away", "tsettings"], if on { "Away mode is on." } else { "Away mode is off." }, cx);
        });
        let mut card = self.pg_card().id("away-card").role(Role::Group).aria_label("Away mode").child(switch);
        card = card.child(self.pg_small("While you are away, departments keep working but say no to anything that needs permission, and set those steps aside for you."));
        if !v.decisions.is_empty() {
            card = card.child(self.pg_small("Decided without you:")).children(v.decisions.iter().map(|d| div().text_sm().min_w_0().child(d.clone())));
        }
        if !v.parked.is_empty() {
            card = card.child(self.pg_small("Set aside for you:")).children(v.parked.iter().map(|d| div().text_sm().min_w_0().child(d.clone())));
        }
        if let Some(n) = self.pg_note("away") {
            card = card.child(n);
        }
        card.into_any_element()
    }

    // ---- departments -----------------------------------------------------------------------

    fn depts_section(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let list: Vec<Value> = self.sp.data.get("depts").map(|v| arr(v, "departments").to_vec()).unwrap_or_default();
        let mut col = div().flex().flex_col().gap_2();
        let new = self.button("dept-new", "New department", Kind::Ghost, cx, |a, w, cx| {
            let f = DeptForm::default();
            a.open_dept_form(true, f, w, cx);
        });
        col = col.child(div().flex().items_center().gap_2().child(div().flex_1().child(self.pg_heading("Departments"))).child(new));
        if list.is_empty() {
            col = col.child(self.pg_small("No departments yet"));
        }
        for d in list {
            col = col.child(self.dept_card(d, cx));
        }
        if self.sp.team.dept.is_some() {
            col = col.child(self.dept_form(window, cx));
        }
        if let Some(n) = self.pg_note("depts") {
            col = col.child(n);
        }
        col.into_any_element()
    }

    fn dept_card(&mut self, d: Value, cx: &mut Context<Self>) -> AnyElement {
        let name = d["name"].as_str().unwrap_or("").to_string();
        let mut card = self.pg_card().id(SharedString::from(format!("dept-card-{name}"))).role(Role::Group).aria_label(name.clone());
        card = card.child(div().min_w_0().child(name.clone()));
        if let Some(e) = d["error"].as_str() {
            card = card.child(div().text_sm().text_color(self.pal.danger).child(format!("This department file has a problem: {e}")));
            let del = self.pg_confirm(&format!("dept-del-{name}"), "Delete", "Delete this department?", cx, {
                let n = name.clone();
                move |a, _, cx| a.pg_do("depts", "DELETE", format!("/v1/team/departments/{n}"), None, &["depts"], "Deleted.", cx)
            });
            return card.child(del).into_any_element();
        }
        let form = DeptForm::from_json(&d);
        if !form.description.is_empty() {
            card = card.child(self.pg_small(form.description.clone()));
        }
        card = card.child(self.pg_small(format!("Lead: {}.   Members: {}.", form.lead, form.members.join(", "))));
        let open_hist = self.sp.team.dept_history.as_deref() == Some(name.as_str());
        let edit = self.button(format!("dept-edit-{name}"), "Edit", Kind::Normal, cx, {
            let f = form.clone();
            move |a, w, cx| a.open_dept_form(false, f.clone(), w, cx)
        });
        let hist = self.button(format!("dept-hist-{name}"), if open_hist { "Hide history" } else { "History" }, Kind::Ghost, cx, {
            let n = name.clone();
            move |a, _, cx| {
                if a.sp.team.dept_history.as_deref() == Some(n.as_str()) {
                    a.sp.team.dept_history = None;
                } else {
                    a.sp.team.dept_history = Some(n.clone());
                    a.pg_get(&format!("depthist:{n}"));
                }
                cx.notify();
            }
        });
        let del = self.pg_confirm(&format!("dept-del-{name}"), "Delete", "Delete this department?", cx, {
            let n = name.clone();
            move |a, _, cx| a.pg_do("depts", "DELETE", format!("/v1/team/departments/{n}"), None, &["depts"], "Deleted.", cx)
        });
        let open = self.button(format!("dept-open-{name}"), "Open page", Kind::Ghost, cx, {
            let n = name.clone();
            move |a, _, cx| a.open_department(&n, cx)
        });
        card = card.child(div().flex().flex_wrap().items_center().gap_1().child(edit).child(hist).child(open).child(del));
        if open_hist {
            card = card.child(self.version_rows(&format!("depthist:{name}"), &format!("dept-v-{name}"), cx, {
                let n = name.clone();
                move |a, ver, cx| a.pg_do("depts", "POST", format!("/v1/team/departments/{n}/rollback"), Some(json!({"version": ver})), &["depts", &format!("depthist:{n}")], &format!("Went back to version {ver}."), cx)
            }));
        }
        card.into_any_element()
    }

    /// The history of a role or department: newest first, with a way back to each older version.
    fn version_rows(&mut self, data: &str, id: &str, cx: &mut Context<Self>, back: impl Fn(&mut App, i64, &mut Context<App>) + Clone + 'static) -> AnyElement {
        let p = self.pal.clone();
        let list = match self.sp.data.get(data) {
            Some(v) => versions(arr(v, "versions"), |h| h["note"].as_str().unwrap_or("").to_string()),
            None => return self.pg_small("Loading the history…").into_any_element(),
        };
        let rows: Vec<AnyElement> = list
            .into_iter()
            .enumerate()
            .map(|(n, v)| {
                let go = (n > 0).then(|| {
                    let (back, num) = (back.clone(), v.number);
                    self.button(format!("{id}-back-{num}"), "Go back to this", Kind::Ghost, cx, move |a, _, cx| back(a, num, cx))
                });
                div().flex().flex_wrap().items_center().gap_2().child(div().flex_1().min_w_0().text_sm().child(format!("Version {}, {}{}", v.number, v.line, if n == 0 { " (now)" } else { "" }))).children(go).into_any_element()
            })
            .collect();
        div().flex().flex_col().gap_1().p_1().rounded(p.inner).bg(p.button).id(SharedString::from(format!("{id}-list"))).role(Role::List).children(rows).into_any_element()
    }

    fn open_dept_form(&mut self, is_new: bool, f: DeptForm, w: &mut Window, cx: &mut Context<Self>) {
        for (k, v) in [("dept.name", &f.name), ("dept.desc", &f.description), ("dept.parallel", &f.parallel), ("dept.attempts", &f.attempts), ("dept.ins", &f.instructions)] {
            self.pg_set(k, v, w, cx);
        }
        self.sp.notes.remove("deptform");
        self.sp.team.dept = Some((is_new, f));
        cx.notify();
    }

    fn dept_form(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let Some((is_new, form)) = self.sp.team.dept.clone() else { return div().into_any_element() };
        let roles = self.role_names();
        let depts: Vec<String> = self.sp.data.get("depts").map(|v| arr(v, "departments").iter().filter_map(|d| d["name"].as_str()).filter(|n| *n != form.name).map(|n| format!("dept:{n}")).collect()).unwrap_or_default();
        let mut card = self.pg_card().border_1().border_color(self.pal.accent).id("dept-form").role(Role::Group).aria_label(if is_new { "New department" } else { "Edit department" });
        card = card.child(div().child(if is_new { "New department".to_string() } else { format!("Edit {}", form.name) }));
        if is_new {
            card = card.child(self.pg_field("Name", "dept.name", "web (lower case letters, digits, hyphens)", window, cx));
        }
        card = card.child(self.pg_field("What it is for", "dept.desc", "One line", window, cx));
        let lead = self.pg_dropdown("dept-lead", if form.lead.is_empty() { "Pick a lead".to_string() } else { form.lead.clone() }, roles.iter().map(|r| (r.clone(), r.clone())).collect(), &form.lead, cx, |a, v, _, _| {
            if let Some((_, f)) = a.sp.team.dept.as_mut() {
                f.lead = v;
            }
        });
        card = card.child(div().flex().flex_col().gap_1().child(div().text_sm().child("Lead")).child(lead)).child(self.pg_small("The lead plans the work, hands out steps and checks the results."));
        let chips: Vec<AnyElement> = roles
            .iter()
            .chain(depts.iter())
            .enumerate()
            .map(|(i, m)| {
                let on = form.members.contains(m);
                let m2 = m.clone();
                self.button(format!("dept-member-{i}"), if on { format!("✓ {m}") } else { m.clone() }, if on { Kind::Primary } else { Kind::Normal }, cx, move |a, _, cx| {
                    if let Some((_, f)) = a.sp.team.dept.as_mut() {
                        match f.members.iter().position(|x| *x == m2) {
                            Some(p) => {
                                f.members.remove(p);
                            }
                            None => f.members.push(m2.clone()),
                        }
                    }
                    cx.notify();
                })
            })
            .collect();
        card = card.child(div().flex().flex_col().gap_1().child(div().text_sm().child("Members")).child(div().flex().flex_wrap().gap_1().children(chips)));
        card = card.child(self.pg_field("Workers at once for this department (empty: use the team setting)", "dept.parallel", "", window, cx));
        card = card.child(self.pg_field("Tries per step for this department (empty: use the team setting)", "dept.attempts", "", window, cx));
        card = card.child(self.pg_text_area("Instructions for the whole department", "dept.ins", "Optional", 3, window, cx));
        let save = self.button("dept-save", "Save", Kind::Primary, cx, move |a, _, cx| {
            let Some((is_new, mut f)) = a.sp.team.dept.clone() else { return };
            if is_new {
                f.name = a.pg_val("dept.name", cx).trim().to_string();
            }
            f.description = a.pg_val("dept.desc", cx);
            f.parallel = a.pg_val("dept.parallel", cx);
            f.attempts = a.pg_val("dept.attempts", cx);
            f.instructions = a.pg_val("dept.ins", cx);
            match f.compose() {
                Ok(text) => {
                    a.sp.team.dept = None;
                    a.pg_do("depts", "PUT", format!("/v1/team/departments/{}", f.name), Some(json!({"text": text})), &["depts"], "Saved.", cx);
                }
                Err(e) => {
                    a.sp.notes.insert("deptform".into(), (false, e));
                    cx.notify();
                }
            }
        });
        let cancel = self.button("dept-cancel", "Cancel", Kind::Ghost, cx, |a, _, cx| {
            a.sp.team.dept = None;
            cx.notify();
        });
        card = card.child(div().flex().gap_1().child(save).child(cancel));
        if let Some(n) = self.pg_note("deptform") {
            card = card.child(n);
        }
        card.into_any_element()
    }

    // ---- roles -----------------------------------------------------------------------------

    fn role_names(&self) -> Vec<String> {
        self.sp.data.get("roles").map(|v| logic::role_targets(v).into_iter().map(|r| r.name).collect()).unwrap_or_default()
    }

    fn roles_section(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let list: Vec<Value> = self.sp.data.get("roles").map(|v| arr(v, "roles").to_vec()).unwrap_or_default();
        let mut col = div().flex().flex_col().gap_2();
        let new = self.button("role-new", "New role", Kind::Ghost, cx, |a, w, cx| a.open_role_form(true, RoleForm::blank(), w, cx));
        col = col.child(div().flex().items_center().gap_2().child(div().flex_1().child(self.pg_heading("Roles"))).child(new));
        col = col.child(self.pg_small("A role is a job a worker can do: its instructions, the tools it may use and the kind of model it runs on."));
        for r in list {
            col = col.child(self.role_card(r, cx));
        }
        if self.sp.team.role.is_some() {
            col = col.child(self.role_form(window, cx));
        }
        if let Some(n) = self.pg_note("roles") {
            col = col.child(n);
        }
        col.into_any_element()
    }

    fn role_card(&mut self, r: Value, cx: &mut Context<Self>) -> AnyElement {
        let name = r["name"].as_str().unwrap_or("").to_string();
        let mut card = self.pg_card().id(SharedString::from(format!("role-card-{name}"))).role(Role::Group).aria_label(name.clone());
        card = card.child(div().min_w_0().child(name.clone()));
        if let Some(e) = r["error"].as_str() {
            card = card.child(div().text_sm().text_color(self.pal.danger).child(format!("This role file has a problem: {e}")));
        } else {
            let f = RoleForm::from_json(&r);
            card = card.child(self.pg_small(f.description.clone()));
            let model = if f.kind == "model" { format!("the model {}", f.target) } else { format!("the {} model type", f.target) };
            card = card.child(self.pg_small(format!("Runs on {model}. Tools: {}.", if f.tools.is_empty() { "none".to_string() } else { f.tools.join(", ") })));
        }
        let open_hist = self.sp.team.role_history.as_deref() == Some(name.as_str());
        let mut row = div().flex().flex_wrap().items_center().gap_1();
        if r["error"].is_null() {
            let f = RoleForm::from_json(&r);
            row = row.child(self.button(format!("role-edit-{name}"), "Edit", Kind::Normal, cx, move |a, w, cx| a.open_role_form(false, f.clone(), w, cx)));
        }
        row = row.child(self.button(format!("role-hist-{name}"), if open_hist { "Hide history" } else { "History" }, Kind::Ghost, cx, {
            let n = name.clone();
            move |a, _, cx| {
                if a.sp.team.role_history.as_deref() == Some(n.as_str()) {
                    a.sp.team.role_history = None;
                } else {
                    a.sp.team.role_history = Some(n.clone());
                    a.pg_get(&format!("rolehist:{n}"));
                }
                cx.notify();
            }
        }));
        row = row.child(self.pg_confirm(&format!("role-del-{name}"), "Delete", "Delete this role?", cx, {
            let n = name.clone();
            move |a, _, cx| a.pg_do("roles", "DELETE", format!("/v1/team/roles/{n}"), None, &["roles"], "Deleted.", cx)
        }));
        card = card.child(row);
        if open_hist {
            card = card.child(self.version_rows(&format!("rolehist:{name}"), &format!("role-v-{name}"), cx, {
                let n = name.clone();
                move |a, ver, cx| a.pg_do("roles", "POST", format!("/v1/team/roles/{n}/rollback"), Some(json!({"version": ver})), &["roles", &format!("rolehist:{n}")], &format!("Went back to version {ver}."), cx)
            }));
        }
        card.into_any_element()
    }

    fn open_role_form(&mut self, is_new: bool, f: RoleForm, w: &mut Window, cx: &mut Context<Self>) {
        for (k, v) in [("role.name", &f.name), ("role.desc", &f.description), ("role.model", &f.target), ("role.memory", &f.memory), ("role.temp", &f.temperature), ("role.top_p", &f.top_p), ("role.top_k", &f.top_k), ("role.min_p", &f.min_p), ("role.repeat", &f.repeat_penalty), ("role.steps", &f.max_steps), ("role.ins", &f.instructions)] {
            self.pg_set(k, v, w, cx);
        }
        self.sp.notes.remove("roleform");
        self.sp.team.role = Some((is_new, f));
        cx.notify();
    }

    fn role_form(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let Some((is_new, form)) = self.sp.team.role.clone() else { return div().into_any_element() };
        let types: Vec<TypeRow> = self.sp.data.get("types").map(logic::types).unwrap_or_default();
        let known: Vec<String> = logic::KNOWN_TOOLS.iter().map(|t| t.to_string()).collect();
        let mut card = self.pg_card().border_1().border_color(self.pal.accent).id("role-form").role(Role::Group).aria_label(if is_new { "New role" } else { "Edit role" });
        card = card.child(div().child(if is_new { "New role".to_string() } else { format!("Edit {}", form.name) }));
        if is_new {
            card = card.child(self.pg_field("Name", "role.name", "coder (lower case letters, digits, hyphens)", window, cx));
        }
        card = card.child(self.pg_field("What it does, in one line", "role.desc", "The lead reads this when it chooses who gets a step", window, cx));
        // which model: a kind of model, or one exact model
        let by_type = self.button("role-by-type", "A model type", if form.kind != "model" { Kind::Primary } else { Kind::Normal }, cx, |a, _, cx| {
            if let Some((_, f)) = a.sp.team.role.as_mut() {
                f.kind = "type".into();
            }
            cx.notify();
        });
        let by_model = self.button("role-by-model", "One exact model", if form.kind == "model" { Kind::Primary } else { Kind::Normal }, cx, |a, _, cx| {
            if let Some((_, f)) = a.sp.team.role.as_mut() {
                f.kind = "model".into();
            }
            cx.notify();
        });
        let target: AnyElement = if form.kind == "model" {
            let picker = self.pg_dropdown("role-model", if form.target.is_empty() { "Pick a model".to_string() } else { form.target.clone() }, self.pg_model_options(), &form.target, cx, |a, v, w, cx| {
                a.pg_set("role.model", &v, w, cx);
                if let Some((_, f)) = a.sp.team.role.as_mut() {
                    f.target = v;
                }
            });
            let typed = self.pg_field("or type its name", "role.model", "", window, cx);
            div().flex().flex_col().gap_1().child(picker).child(typed).into_any_element()
        } else {
            self.pg_dropdown("role-type", if form.target.is_empty() { "Pick a type".to_string() } else { form.target.clone() }, type_options(&types), &form.target, cx, |a, v, _, _| {
                if let Some((_, f)) = a.sp.team.role.as_mut() {
                    f.target = v;
                }
            })
        };
        card = card.child(div().flex().flex_col().gap_1().child(div().text_sm().child("Model")).child(div().flex().gap_1().child(by_type).child(by_model)).child(target));
        // tools
        let chips: Vec<AnyElement> = known
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let on = form.tools.contains(t);
                let t2 = t.clone();
                self.button(format!("role-tool-{i}"), if on { format!("✓ {t}") } else { t.clone() }, if on { Kind::Primary } else { Kind::Normal }, cx, move |a, _, cx| {
                    if let Some((_, f)) = a.sp.team.role.as_mut() {
                        match f.tools.iter().position(|x| *x == t2) {
                            Some(p) => {
                                f.tools.remove(p);
                            }
                            None => f.tools.push(t2.clone()),
                        }
                    }
                    cx.notify();
                })
            })
            .collect();
        card = card.child(div().flex().flex_col().gap_1().child(div().text_sm().child("Tools it may use")).child(div().flex().flex_wrap().gap_1().children(chips)));
        card = card.child(self.pg_field("Memory it may see (none, all, or preferences facts projects)", "role.memory", "none", window, cx));
        // sampling
        card = card.child(div().text_sm().child("Sampling (leave empty to let the model decide)"));
        let samp = ["role.temp:Temperature", "role.top_p:Top P", "role.top_k:Top K", "role.min_p:Min P", "role.repeat:Repeat penalty"];
        let mut grid = div().flex().flex_wrap().gap_2();
        for s in samp {
            let (key, label) = s.split_once(':').unwrap_or((s, s));
            let f = self.pg_field(label, key, "", window, cx);
            grid = grid.child(div().w(px(150.)).child(f));
        }
        card = card.child(grid);
        card = card.child(self.pg_field("Most model calls in one step (1 to 100)", "role.steps", "12", window, cx));
        let thinking = self.pg_dropdown("role-thinking", match form.thinking.as_str() { "on" => "Thinks first", "off" => "Answers without thinking", _ => "Let the model decide" }.to_string(), vec![("Let the model decide".into(), String::new()), ("Answers without thinking".into(), "off".into()), ("Thinks first".into(), "on".into())], &form.thinking, cx, |a, v, _, _| {
            if let Some((_, f)) = a.sp.team.role.as_mut() {
                f.thinking = v;
            }
        });
        card = card.child(div().flex().flex_col().gap_1().child(div().text_sm().child("Thinking")).child(thinking));
        let used = self.pg_val("role.ins", cx).trim().chars().count();
        card = card.child(self.pg_text_area("Instructions", "role.ins", "Who the role is and how it works. Keep it short: it goes with every step.", 4, window, cx));
        card = card.child(div().text_xs().text_color(if used > logic::INSTRUCTIONS_MAX { self.pal.danger } else { self.pal.muted }).child(format!("{used} of {} characters", logic::INSTRUCTIONS_MAX)));
        let save = self.button("role-save", "Save", Kind::Primary, cx, move |a, _, cx| {
            let Some((is_new, mut f)) = a.sp.team.role.clone() else { return };
            if is_new {
                f.name = a.pg_val("role.name", cx).trim().to_string();
            }
            f.description = a.pg_val("role.desc", cx);
            if f.kind == "model" {
                f.target = a.pg_val("role.model", cx).trim().to_string();
            }
            f.memory = a.pg_val("role.memory", cx);
            f.temperature = a.pg_val("role.temp", cx);
            f.top_p = a.pg_val("role.top_p", cx);
            f.top_k = a.pg_val("role.top_k", cx);
            f.min_p = a.pg_val("role.min_p", cx);
            f.repeat_penalty = a.pg_val("role.repeat", cx);
            f.max_steps = a.pg_val("role.steps", cx);
            f.instructions = a.pg_val("role.ins", cx);
            match f.compose() {
                Ok(text) => {
                    a.sp.team.role = None;
                    a.pg_do("roles", "PUT", format!("/v1/team/roles/{}", f.name), Some(json!({"text": text})), &["roles"], "Saved.", cx);
                }
                Err(e) => {
                    a.sp.notes.insert("roleform".into(), (false, e));
                    cx.notify();
                }
            }
        });
        let cancel = self.button("role-cancel", "Cancel", Kind::Ghost, cx, |a, _, cx| {
            a.sp.team.role = None;
            cx.notify();
        });
        card = card.child(div().flex().gap_1().child(save).child(cancel));
        if let Some(n) = self.pg_note("roleform") {
            card = card.child(n);
        }
        card.into_any_element()
    }

    // ---- model types -------------------------------------------------------------------------

    fn types_section(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let types: Vec<TypeRow> = self.sp.data.get("types").map(logic::types).unwrap_or_default();
        let mut col = div().flex().flex_col().gap_2();
        let new = self.button("type-new", "New type", Kind::Ghost, cx, |a, w, cx| {
            a.sp.team.type_edit = Some(String::new());
            for k in ["type.name", "type.model", "type.fallbacks"] {
                a.pg_set(k, "", w, cx);
            }
            cx.notify();
        });
        col = col.child(div().flex().items_center().gap_2().child(div().flex_1().child(self.pg_heading("Model types"))).child(new));
        col = col.child(self.pg_small("A model type is a kind of job for a model, like frontier or mid-tier. Roles name a type, and the type names the model, so one change here moves every role that uses it."));
        for t in types.iter().cloned() {
            col = col.child(self.type_card(t, cx));
        }
        if self.sp.team.type_edit.is_some() {
            col = col.child(self.type_form(window, cx));
        }
        if self.sp.team.type_delete.is_some() {
            col = col.child(self.type_delete_card(&types, cx));
        }
        if let Some(n) = self.pg_note("types") {
            col = col.child(n);
        }
        col.into_any_element()
    }

    fn type_card(&mut self, t: TypeRow, cx: &mut Context<Self>) -> AnyElement {
        let name = t.name.clone();
        let model = t.model.clone().unwrap_or_default();
        let mut card = self.pg_card().id(SharedString::from(format!("type-card-{name}"))).role(Role::Group).aria_label(name.clone());
        card = card.child(div().flex().items_center().gap_2().child(div().child(name.clone())).when(t.stock, |d| d.child(self.pg_small("built in"))));
        card = card.child(self.pg_small(match &t.model {
            Some(m) => format!("Uses {}{}", self.models.name_of(m), if t.fallbacks.is_empty() { String::new() } else { format!(", then {}", t.fallbacks.join(", ")) }),
            None => "No model chosen yet.".to_string(),
        }));
        let edit = self.button(format!("type-edit-{name}"), "Edit", Kind::Normal, cx, {
            let (n, m, fb) = (name.clone(), model.clone(), t.fallbacks.join(", "));
            move |a, w, cx| {
                a.sp.team.type_edit = Some(n.clone());
                a.pg_set("type.model", &m, w, cx);
                a.pg_set("type.fallbacks", &fb, w, cx);
                cx.notify();
            }
        });
        let del = self.button(format!("type-del-{name}"), "Delete", Kind::Ghost, cx, {
            let n = name.clone();
            move |a, _, cx| a.start_type_delete(&n, cx)
        });
        card.child(div().flex().gap_1().child(edit).child(del)).into_any_element()
    }

    fn type_form(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let editing = self.sp.team.type_edit.clone().unwrap_or_default();
        let is_new = editing.is_empty();
        let mut card = self.pg_card().border_1().border_color(self.pal.accent).id("type-form").role(Role::Group).aria_label("Model type");
        card = card.child(div().child(if is_new { "New model type".to_string() } else { format!("Edit {editing}") }));
        if is_new {
            card = card.child(self.pg_field("Name", "type.name", "cheap (lower case letters, digits, hyphens)", window, cx));
        }
        let current = self.pg_val("type.model", cx);
        let picker = self.pg_dropdown("type-model", if current.is_empty() { "No model yet".to_string() } else { self.models.name_of(&current) }, std::iter::once(("No model yet".to_string(), String::new())).chain(self.pg_model_options()).collect(), &current, cx, |a, v, w, cx| a.pg_set("type.model", &v, w, cx));
        card = card.child(div().flex().flex_col().gap_1().child(div().text_sm().child("Model")).child(picker));
        card = card.child(self.pg_field("Models to try, in order, when the first is missing (separate with commas)", "type.fallbacks", "", window, cx));
        let save = self.button("type-save", "Save", Kind::Primary, cx, move |a, _, cx| {
            let Some(editing) = a.sp.team.type_edit.clone() else { return };
            let model = a.pg_val("type.model", cx).trim().to_string();
            let fallbacks: Vec<String> = a.pg_val("type.fallbacks", cx).split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            let model_json = if model.is_empty() { Value::Null } else { json!(model) };
            if editing.is_empty() {
                let name = a.pg_val("type.name", cx).trim().to_lowercase();
                if !logic::valid_name(&name) {
                    a.sp.notes.insert("typeform".into(), (false, "The name can use lower case letters, digits and single hyphens, like cheap.".into()));
                    cx.notify();
                    return;
                }
                a.sp.team.type_edit = None;
                a.pg_do("types", "POST", "/v1/team/types".into(), Some(json!({"name": name, "model": model_json})), &["types"], "Added.", cx);
                if !fallbacks.is_empty() {
                    a.pg_do("types", "PUT", format!("/v1/team/types/{name}"), Some(json!({"fallbacks": fallbacks})), &["types"], "Added.", cx);
                }
            } else {
                a.sp.team.type_edit = None;
                a.pg_do("types", "PUT", format!("/v1/team/types/{editing}"), Some(json!({"model": model_json, "fallbacks": fallbacks})), &["types"], "Saved.", cx);
            }
        });
        let cancel = self.button("type-cancel", "Cancel", Kind::Ghost, cx, |a, _, cx| {
            a.sp.team.type_edit = None;
            cx.notify();
        });
        card = card.child(div().flex().gap_1().child(save).child(cancel));
        if let Some(n) = self.pg_note("typeform") {
            card = card.child(n);
        }
        card.into_any_element()
    }

    /// Deleting a type: if no role uses it, ask once; if roles do, they each need a new home first.
    fn start_type_delete(&mut self, name: &str, cx: &mut Context<Self>) {
        let roles: Vec<RoleTarget> = self.sp.data.get("roles").map(logic::role_targets).unwrap_or_default();
        let types: Vec<TypeRow> = self.sp.data.get("types").map(logic::types).unwrap_or_default();
        let affected = logic::roles_using_type(&roles, name);
        let homes = logic::default_homes(&affected, &types, name);
        self.sp.team.type_delete = Some(TypeDelete { name: name.to_string(), affected, homes });
        self.sp.notes.remove("types");
        cx.notify();
    }

    fn type_delete_card(&mut self, types: &[TypeRow], cx: &mut Context<Self>) -> AnyElement {
        let Some(del) = self.sp.team.type_delete.as_ref() else { return div().into_any_element() };
        let (name, affected, homes) = (del.name.clone(), del.affected.clone(), del.homes.clone());
        let mut card = self.pg_card().border_1().border_color(self.pal.danger).id("type-delete").role(Role::Group).aria_label("Delete a model type");
        card = card.child(div().child(format!("Delete the type {name}?")));
        if affected.is_empty() {
            card = card.child(self.pg_small("No role uses it."));
        } else {
            card = card.child(self.pg_small("These roles use it. Choose where each one goes before it is deleted."));
        }
        let others: Vec<(String, String)> = types.iter().filter(|t| t.name != name).map(|t| (format!("Type {}", t.name), t.name.clone())).collect();
        for role in affected.iter() {
            let home = homes.get(role).cloned();
            let shown = match &home {
                Some(Home::Type(t)) => format!("Type {t}"),
                Some(Home::Model(m)) => format!("Model {m}"),
                None => "Choose…".into(),
            };
            let mut options: Vec<(String, String)> = others.iter().map(|(l, v)| (l.clone(), format!("type:{v}"))).collect();
            options.extend(self.pg_model_options().into_iter().map(|(l, v)| (format!("Model {l}"), format!("model:{v}"))));
            let selected = match &home {
                Some(Home::Type(t)) => format!("type:{t}"),
                Some(Home::Model(m)) => format!("model:{m}"),
                None => String::new(),
            };
            let r = role.clone();
            let picker = self.pg_dropdown(&format!("type-home-{role}"), shown, options, &selected, cx, move |a, v, _, _| {
                if let Some(d) = a.sp.team.type_delete.as_mut() {
                    let h = match v.split_once(':') {
                        Some(("type", t)) => Home::Type(t.to_string()),
                        Some((_, m)) => Home::Model(m.to_string()),
                        None => return,
                    };
                    d.homes.insert(r.clone(), h);
                }
            });
            card = card.child(div().flex().flex_wrap().items_center().gap_2().child(div().w(px(120.)).child(role.clone())).child(picker));
        }
        let go = self.button("type-delete-go", "Delete the type", Kind::Danger, cx, {
            let (name, affected) = (name.clone(), affected.clone());
            move |a, _, cx| {
                let homes = a.sp.team.type_delete.as_ref().map(|d| d.homes.clone()).unwrap_or_default();
                match logic::reassign_body(&affected, &homes) {
                    Ok(body) => {
                        a.sp.team.type_delete = None;
                        a.pg_do("types", "DELETE", format!("/v1/team/types/{name}"), Some(body), &["types", "roles"], "Deleted.", cx);
                    }
                    Err(e) => {
                        a.sp.notes.insert("types".into(), (false, e));
                        cx.notify();
                    }
                }
            }
        });
        let cancel = self.button("type-delete-cancel", "Cancel", Kind::Ghost, cx, |a, _, cx| {
            a.sp.team.type_delete = None;
            cx.notify();
        });
        card.child(div().flex().gap_1().child(go).child(cancel)).into_any_element()
    }
}
