//! The Models and keys, Learning and Departments pages of Settings. Each page keeps what the core last said
//! in `PageState`, asks for it again after every change, and shows a plain sentence when something fails.
//! The pieces that decide things live in `lh_gui::pages_logic`, where they are tested.

mod learning;
mod models_keys;
mod team;

use std::collections::{HashMap, HashSet};

use gpui::prelude::*;
use gpui::*;
use gpui_component::input::{Input, InputState, TextareaState};
use serde_json::Value;

use crate::app::App;
use crate::ui::Kind;
use lh_gui::api::Cmd;
use lh_gui::setting_names::Category;

/// Everything these pages remember between draws.
#[derive(Default)]
pub struct PageState {
    /// The last good answer of each call, by name ("rules", "rule:r3", "roles"...).
    pub data: HashMap<String, Value>,
    /// Why a call failed, by name.
    pub errors: HashMap<String, String>,
    /// A line under a section: what was done, or why it was refused.
    pub notes: HashMap<String, (bool, String)>,
    pub inputs: HashMap<String, Entity<InputState>>,
    pub areas: HashMap<String, Entity<TextareaState>>,
    /// Words to put into a box at the next draw (the box needs the window to be filled).
    pub fill: Vec<(String, String)>,
    /// Boxes to empty at the next draw.
    pub clear: Vec<String>,
    /// The pages already asked for since Settings opened.
    pub loaded: HashSet<&'static str>,
    /// The drop-down list that is open.
    pub list: Option<String>,
    /// A destructive action waiting for "Yes".
    pub confirm: Option<String>,
    /// The first-run check has been made.
    pub first_run_done: bool,
    pub models_keys: models_keys::State,
    pub learning: learning::State,
    pub team: team::State,
}

impl App {
    // ---- talking to the core ----------------------------------------------------------------

    /// Ask for something and keep the answer under `name`.
    pub fn pg_get(&mut self, name: &str) {
        if let Some(path) = pg_path(name) {
            self.backend.send(Cmd::Call { tag: format!("pg|{name}"), method: "GET".into(), path, body: None });
        }
    }

    /// Do something, say `ok` under `section` when it worked (or the core's own words when it did not),
    /// and ask again for the lists in `refetch`.
    #[allow(clippy::too_many_arguments)]
    pub fn pg_do(&mut self, section: &str, method: &str, path: String, body: Option<Value>, refetch: &[&str], ok: &str, cx: &mut Context<Self>) {
        self.backend.send(Cmd::Call { tag: format!("pg-do|{section}|{}|{ok}", refetch.join(",")), method: method.into(), path, body });
        self.sp.notes.remove(section);
        cx.notify();
    }

    /// Forget what these pages knew and ask for the provider again (Settings opened, or the core came back).
    pub fn pg_reload(&mut self) {
        self.sp.loaded.clear();
        self.sp.confirm = None;
        self.sp.list = None;
        self.pg_get("provider");
    }

    /// An answer for these pages came back.
    pub fn on_page_reply(&mut self, tag: &str, result: Result<Value, String>) {
        let mut parts = tag.splitn(4, '|');
        match (parts.next(), parts.next()) {
            (Some("pg"), Some(name)) => match result {
                Ok(v) => {
                    self.sp.errors.remove(name);
                    self.after_get(name, &v);
                    self.sp.data.insert(name.to_string(), v);
                }
                Err(e) => {
                    if name == "provtest" {
                        self.sp.models_keys.testing = false;
                    }
                    self.sp.errors.insert(name.to_string(), e);
                }
            },
            (Some("pg-do"), Some(section)) => {
                let (refetch, ok) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
                match result {
                    Ok(v) => {
                        if !ok.is_empty() {
                            self.sp.notes.insert(section.to_string(), (true, ok.to_string()));
                        }
                        for n in refetch.split(',').filter(|n| !n.is_empty()) {
                            self.pg_get(n);
                        }
                        self.after_do(section, &v);
                    }
                    Err(e) => {
                        self.sp.notes.insert(section.to_string(), (false, e));
                    }
                }
            }
            _ => {}
        }
    }

    /// Anything that follows a fresh answer: the first-run page, boxes that fill from it.
    fn after_get(&mut self, name: &str, v: &Value) {
        if name == "provider" && !self.sp.first_run_done {
            self.sp.first_run_done = true;
            if lh_gui::pages_logic::models_keys::ProviderInfo::from_json(v).needs_first_run() {
                self.settings_open = true;
                self.settings_page = Category::Models;
            }
        }
        if name == "provtest" {
            self.sp.models_keys.testing = false;
            self.sp.models_keys.result = Some(lh_gui::pages_logic::models_keys::TestResult::from_json(v));
        }
        if name == "style" {
            if let Some(t) = v["style"]["text"].as_str() {
                self.sp.fill.push(("style".into(), t.to_string()));
            }
        }
    }

    /// Anything that follows an action: boxes that empty, forms that close.
    fn after_do(&mut self, section: &str, _v: &Value) {
        match section {
            "rules" => self.sp.clear.push("rule.new".into()),
            "skills" => self.sp.learning.preview = None,
            "provider" => self.sp.clear.push("prov.key".into()),
            _ => {}
        }
    }

    /// The part of a Settings page that these files draw, if the page has one. Models and keys comes first on
    /// its page; Learning and Departments bring their editors.
    pub fn extra_page(&mut self, page: Category, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        self.pg_apply_queues(window, cx);
        let id = page.id();
        let names: &[&str] = match page {
            Category::Models => &["provider"],
            Category::Learning => &["rules", "style", "skills", "proposals", "habits"],
            Category::Departments => &["roles", "types", "depts", "tsettings", "away"],
            _ => return None,
        };
        if self.connected && !self.sp.loaded.contains(id) {
            self.sp.loaded.insert(id);
            for n in names {
                self.pg_get(n);
            }
        }
        Some(match page {
            Category::Models => {
                let rank = self.button("rank-models", "Rank my models from benchmarks", Kind::Normal, cx, |a, w, cx| {
                    a.close_settings(w, cx);
                    a.rank_models(w, cx);
                });
                let n = self.stop_count();
                let count_row = div().flex().items_center().gap_2()
                    .child(self.button("stops-fewer", "Fewer", crate::ui::Kind::Normal, cx, move |a, _, cx| a.set_stop_count(n.saturating_sub(1), cx)))
                    .child(div().text_sm().child(format!("{n} stops")))
                    .child(self.button("stops-more", "More", crate::ui::Kind::Normal, cx, move |a, _, cx| a.set_stop_count(n + 1, cx)));
                let mut titles = div().flex().flex_col().gap_1p5();
                for i in 0..n {
                    let key = format!("stop-title-{i}");
                    let fresh = !self.sp.inputs.contains_key(&key);
                    let placeholder = lh_gui::tiers::title_of(&[], i, self.rtl);
                    let input = self.pg_input(&key, &placeholder, false, window, cx);
                    if fresh {
                        if let Some(t) = self.prefs.stop_titles.get(i).filter(|t| !t.is_empty()).cloned() {
                            self.pg_set(&key, &t, window, cx);
                        }
                    }
                    titles = titles.child(Input::new(&input).aria_label(format!("Title of stop {}", i + 1)));
                }
                let save = self.button("stops-save-titles", "Save stop titles", Kind::Normal, cx, |a, _, cx| a.save_stop_titles(cx));
                div().flex().flex_col().gap_3()
                    .child(div().flex().flex_col().gap_1()
                        .child(div().text_sm().font_weight(gpui::FontWeight::MEDIUM).child("Model slider"))
                        .child(div().text_xs().text_color(self.pal.muted).child("The stops run from the lowest to the highest. Choose how many (2 to 7) and what each is called. The assistant can look up benchmarks and suggest which model goes where; nothing changes until you set it. Set a stop yourself in the model list."))
                        .child(count_row)
                        .child(titles)
                        .child(div().flex().child(save))
                        .child(div().flex().child(rank)))
                    .child(self.models_keys_block(window, cx))
                    .into_any_element()
            }
            Category::Learning => self.learning_block(window, cx),
            _ => self.team_block(window, cx),
        })
    }

    fn pg_apply_queues(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for key in std::mem::take(&mut self.sp.clear) {
            self.pg_set(&key, "", window, cx);
        }
        for (key, value) in std::mem::take(&mut self.sp.fill) {
            self.pg_set(&key, &value, window, cx);
        }
    }

    // ---- boxes ---------------------------------------------------------------------------------

    /// A one-line box, made the first time it is asked for.
    pub fn pg_input(&mut self, key: &str, placeholder: &str, masked: bool, window: &mut Window, cx: &mut Context<Self>) -> Entity<InputState> {
        if let Some(i) = self.sp.inputs.get(key) {
            return i.clone();
        }
        let ph = placeholder.to_string();
        let st = cx.new(|cx| InputState::new(window, cx).placeholder(ph).masked(masked));
        self.sp.inputs.insert(key.to_string(), st.clone());
        st
    }

    /// A box for several lines.
    pub fn pg_area(&mut self, key: &str, placeholder: &str, rows: usize, window: &mut Window, cx: &mut Context<Self>) -> Entity<TextareaState> {
        if let Some(i) = self.sp.areas.get(key) {
            return i.clone();
        }
        let ph = placeholder.to_string();
        let st = cx.new(|cx| TextareaState::new(window, cx).auto_grow(rows, rows + 8).placeholder(ph));
        self.sp.areas.insert(key.to_string(), st.clone());
        st
    }

    /// What is typed in a box (one line or several), or empty if the box was never drawn.
    pub fn pg_val(&self, key: &str, cx: &Context<Self>) -> String {
        if let Some(i) = self.sp.inputs.get(key) {
            return i.read(cx).value().to_string();
        }
        if let Some(a) = self.sp.areas.get(key) {
            return a.read(cx).value().to_string();
        }
        String::new()
    }

    /// Put words into a box. A box that is not drawn yet gets them when it is made.
    pub fn pg_set(&mut self, key: &str, value: &str, window: &mut Window, cx: &mut Context<Self>) {
        let v = value.to_string();
        if let Some(i) = self.sp.inputs.get(key).cloned() {
            i.update(cx, |s, cx| s.set_value(v, window, cx));
        } else if let Some(a) = self.sp.areas.get(key).cloned() {
            a.update(cx, |s, cx| s.set_value(v, window, cx));
        } else {
            self.sp.fill.push((key.to_string(), v));
        }
    }

    // ---- parts ---------------------------------------------------------------------------------

    pub fn pg_card(&self) -> Div {
        let p = &self.pal;
        div().flex().flex_col().gap_2().p_3().rounded(p.inner).bg(p.sunken).text_color(p.text)
    }

    pub fn pg_heading(&self, t: impl Into<SharedString>) -> Div {
        div().text_color(self.pal.text).font_weight(gpui::FontWeight::MEDIUM).pt_3().child(t.into())
    }

    pub fn pg_small(&self, t: impl Into<SharedString>) -> Div {
        div().min_w_0().text_xs().text_color(self.pal.muted).child(t.into())
    }

    /// The line under a section: the good colour when it worked, the danger colour when it did not.
    pub fn pg_note(&self, section: &str) -> Option<AnyElement> {
        let p = &self.pal;
        self.sp.notes.get(section).map(|(ok, t)| div().min_w_0().text_sm().text_color(if *ok { p.good } else { p.danger }).child(t.clone()).into_any_element())
    }

    pub fn pg_error(&self, name: &str) -> Option<AnyElement> {
        let p = &self.pal;
        // the page's banner already says it; repeating it under a section adds nothing
        let banner = self.panels.settings_error.as_deref();
        self.sp.errors.get(name).filter(|e| Some(e.as_str()) != banner).map(|e| div().min_w_0().text_sm().text_color(p.danger).child(e.clone()).into_any_element())
    }

    pub fn pg_field(&mut self, label: &str, key: &str, placeholder: &str, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let st = self.pg_input(key, placeholder, false, window, cx);
        div().flex().flex_col().gap_1().child(div().text_sm().child(label.to_string())).child(Input::new(&st).aria_label(label.to_string())).into_any_element()
    }

    pub fn pg_text_area(&mut self, label: &str, key: &str, placeholder: &str, rows: usize, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let st = self.pg_area(key, placeholder, rows, window, cx);
        div().flex().flex_col().gap_1().child(div().text_sm().child(label.to_string())).child(gpui_component::input::Textarea::new(&st).aria_label(label.to_string())).into_any_element()
    }

    /// A button that asks again before it does something that cannot be undone.
    pub fn pg_confirm(&mut self, id: &str, label: &str, question: &str, cx: &mut Context<Self>, act: impl Fn(&mut App, &mut Window, &mut Context<App>) + 'static) -> AnyElement {
        if self.sp.confirm.as_deref() == Some(id) {
            let yes = self.button(format!("{id}-yes"), "Yes, do it", Kind::Danger, cx, move |a, w, cx| {
                a.sp.confirm = None;
                act(a, w, cx);
                cx.notify();
            });
            let no = self.button(format!("{id}-no"), "Cancel", Kind::Ghost, cx, |a, _, cx| {
                a.sp.confirm = None;
                cx.notify();
            });
            return div().flex().flex_wrap().items_center().gap_1().child(div().text_sm().text_color(self.pal.danger).child(question.to_string())).child(yes).child(no).into_any_element();
        }
        let key = id.to_string();
        self.button(id.to_string(), label.to_string(), Kind::Ghost, cx, move |a, _, cx| {
            a.sp.confirm = Some(key.clone());
            cx.notify();
        })
    }

    /// A drop-down: a button with the current choice that opens a short list. `options` are (words, value).
    pub fn pg_dropdown(&mut self, id: &str, shown: String, options: Vec<(String, String)>, selected: &str, cx: &mut Context<Self>, pick: impl Fn(&mut App, String, &mut Window, &mut Context<App>) + Clone + 'static) -> AnyElement {
        let p = self.pal.clone();
        let open = self.sp.list.as_deref() == Some(id);
        let key = id.to_string();
        let toggle = self.button(format!("{id}-toggle"), format!("{} {}", lh_gui::model::truncate_title(&shown, 34), if open { "▴" } else { "▾" }), Kind::Normal, cx, move |a, _, cx| {
            a.sp.list = if a.sp.list.as_deref() == Some(key.as_str()) { None } else { Some(key.clone()) };
            cx.notify();
        });
        let mut col = div().flex().flex_col().gap_1().child(div().flex().child(toggle));
        if open {
            let rows: Vec<AnyElement> = options
                .into_iter()
                .enumerate()
                .map(|(n, (label, value))| {
                    let kind = if value == selected { Kind::Primary } else { Kind::Ghost };
                    let pick = pick.clone();
                    self.button(format!("{id}-opt-{n}"), label, kind, cx, move |a, w, cx| {
                        a.sp.list = None;
                        pick(a, value.clone(), w, cx);
                        cx.notify();
                    })
                })
                .collect();
            col = col.child(div().on_scroll_wheel(|_, _, cx| cx.stop_propagation()).child(div().id(SharedString::from(format!("{id}-options"))).role(Role::List).max_h(px(220.)).overflow_y_scroll().flex().flex_col().gap_1().p_1().rounded(p.inner).bg(p.button).children(rows)));
        }
        col.into_any_element()
    }

    /// The names the model pickers offer: what the core's model list holds, once each.
    pub fn pg_model_options(&self) -> Vec<(String, String)> {
        self.models.names().iter().map(|n| (n.clone(), self.models.id_for(n))).collect()
    }
}

/// The address behind a name.
fn pg_path(name: &str) -> Option<String> {
    Some(match name {
        "provider" => "/v1/provider".into(),
        "rules" => "/v1/learn/rules?all=true".into(),
        "style" => "/v1/learn/style".into(),
        "skills" => "/v1/learn/skills".into(),
        "proposals" => "/v1/learn/proposals".into(),
        "habits" => "/v1/learn/habits?min=2".into(),
        "roles" => "/v1/team/roles".into(),
        "types" => "/v1/team/types".into(),
        "depts" => "/v1/team/departments".into(),
        "tsettings" => "/v1/team/settings".into(),
        "away" => "/v1/team/away".into(),
        other => {
            let (kind, id) = other.split_once(':')?;
            match kind {
                "rule" => format!("/v1/learn/rules/{id}"),
                "skill" => format!("/v1/learn/skills/{id}"),
                "role" => format!("/v1/team/roles/{id}"),
                "rolehist" => format!("/v1/team/roles/{id}/history"),
                "depthist" => format!("/v1/team/departments/{id}/history"),
                _ => return None,
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::pg_path;

    #[test]
    fn every_name_has_an_address() {
        for n in ["provider", "rules", "style", "skills", "proposals", "habits", "roles", "types", "depts", "tsettings", "away"] {
            assert!(pg_path(n).unwrap().starts_with("/v1/"), "{n}");
        }
        assert_eq!(pg_path("rule:r3").unwrap(), "/v1/learn/rules/r3");
        assert_eq!(pg_path("depthist:web").unwrap(), "/v1/team/departments/web/history");
        assert!(pg_path("nonsense").is_none() && pg_path("zzz:1").is_none());
    }
}
