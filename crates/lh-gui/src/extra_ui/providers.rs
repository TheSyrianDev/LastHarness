//! Providers, under Models and keys, in a list-detail layout: the registered model services on the left, the
//! selected one on the right, with its key, API style, address and models. Adding opens a form in the right pane,
//! and the assistant's proposals sit above. A key is typed here by the person, in a masked box, sent once, and
//! never shown again.

use gpui::prelude::*;
use gpui::*;
use gpui_component::input::{Input, InputState};
use serde_json::json;

use super::model::providers as m;
use crate::app::App;
use crate::ui::Kind;
use lh_gui::setting_names::Category;

impl App {
    /// A masked box for a key.
    fn xp_secret(&mut self, key: &str, label: &str, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let st = match self.pg.inputs.get(key).cloned() {
            Some(s) => s,
            None => {
                let s = cx.new(|cx| InputState::new(window, cx).masked(true));
                self.pg.inputs.insert(key.to_string(), s.clone());
                s
            }
        };
        if let Some(v) = self.pg.set_next.remove(key) {
            st.update(cx, |i, cx| i.set_value(v, window, cx));
        }
        div().flex().flex_col().gap_1().child(div().text_sm().text_color(self.pal.muted).child(label.to_string())).child(Input::new(&st).aria_label(label.to_string())).into_any_element()
    }

    pub fn pg_providers(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        self.pg_opened(Category::Models);
        let mut col = div().flex().flex_col().gap_2().pt_2();
        col = col.child(self.pg_title("Providers")).child(self.pg_muted("Model services this app can use. The default answers when a role does not name one. A key is typed here only, kept in the core, and never shown again."));
        if let Some(n) = self.pg_note_for("providers") {
            col = col.child(n);
        }
        let list = match self.pg_list("providers") {
            None => return col.child(self.pg_note_line("Loading…")).into_any_element(),
            Some(Err(e)) => return col.child(self.pg_error_line("The provider list", &e)).into_any_element(),
            Some(Ok(v)) => m::providers(&v),
        };
        let presets = self.pg_list("prov-presets").and_then(|r| r.ok()).map(|v| m::presets(&v)).unwrap_or_default();
        let proposals = self.pg_list("prov-proposals").and_then(|r| r.ok()).map(|v| m::proposals(&v)).unwrap_or_default();
        for q in proposals {
            col = col.child(self.registry_proposal_card(q, &presets, window, cx));
        }
        let adding = self.pg.open.contains("prov-form");
        if list.is_empty() && !adding {
            col = col.child(self.pg_note_line("No providers registered yet. Add one to start."));
        }
        // the selection must be one that is still listed; otherwise the first one is shown
        let sel = self.pg.prov_sel.clone().filter(|s| list.iter().any(|p| &p.id == s)).or_else(|| list.first().map(|p| p.id.clone()));

        let mut left = div().flex().flex_col().gap_0p5().p_1().w(px(230.)).flex_shrink_0().rounded(self.pal.inner).bg(self.pal.sunken);
        for p in &list {
            let selected = !adding && sel.as_deref() == Some(p.id.as_str());
            left = left.child(self.prov_row(p, selected, cx));
        }
        left = left.child(self.prov_add_row(adding, cx));

        let right = if adding {
            div().flex_1().min_w(px(280.)).min_w_0().child(self.registry_form(&presets, window, cx)).into_any_element()
        } else if let Some(p) = sel.as_ref().and_then(|s| list.iter().find(|p| &p.id == s)) {
            div().flex_1().min_w(px(280.)).min_w_0().child(self.provider_detail(p.clone(), window, cx)).into_any_element()
        } else {
            div().flex_1().min_w(px(280.)).min_w_0().child(self.pg_note_line("Pick a provider on the left, or add one.")).into_any_element()
        };
        col = col.child(div().flex().flex_wrap().gap_3().items_start().child(left).child(right));
        col.child(div().flex().child(self.pg_refresh(Category::Models, cx))).into_any_element()
    }

    fn light_color(&self, l: m::Light) -> Hsla {
        match l {
            m::Light::Ready => self.pal.good,
            m::Light::Attention => self.pal.warn,
            m::Light::Off => self.pal.muted,
        }
    }

    /// One provider in the left column: its light and its name. The selected one is tinted.
    fn prov_row(&mut self, p: &m::Provider, selected: bool, cx: &mut Context<Self>) -> AnyElement {
        let tested = self.pg.prov_test.get(&p.id).copied();
        let (light, words) = m::light(p, tested);
        let color = self.light_color(light);
        let id = p.id.clone();
        let label = if p.name.is_empty() { p.id.clone() } else { p.name.clone() };
        let hover_bg = self.pal.button;
        let tint = self.pal.accent.opacity(0.1);
        div()
            .id(SharedString::from(format!("pg-prov-row-{}", p.id)))
            .role(Role::Button)
            .aria_label(format!("{label}, {words}"))
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .py_1p5()
            .rounded(self.pal.inner)
            .cursor_pointer()
            .text_sm()
            .text_color(self.pal.text)
            .bg(if selected { tint } else { hsla(0., 0., 0., 0.) })
            .hover(move |s| s.bg(hover_bg))
            .on_click(cx.listener(move |a, _, _, cx| {
                a.pg.prov_sel = Some(id.clone());
                a.pg.open.remove("prov-form");
                cx.notify();
            }))
            .child(div().text_color(color).child(light.glyph()))
            .child(div().flex_1().min_w_0().overflow_hidden().child(label))
            .into_any_element()
    }

    /// The "+ Add" row at the bottom of the list. It opens the add flow in the right pane.
    fn prov_add_row(&mut self, adding: bool, cx: &mut Context<Self>) -> AnyElement {
        let hover_bg = self.pal.button;
        let accent = self.pal.accent;
        div()
            .id("pg-prov-add")
            .role(Role::Button)
            .aria_label("Add a provider")
            .flex()
            .items_center()
            .px_2()
            .py_1p5()
            .rounded(self.pal.inner)
            .cursor_pointer()
            .text_sm()
            .text_color(accent)
            .bg(if adding { self.pal.accent.opacity(0.1) } else { hsla(0., 0., 0., 0.) })
            .hover(move |s| s.bg(hover_bg))
            .on_click(cx.listener(|a, _, _, cx| {
                a.pg.prov_preset = String::new();
                a.pg.open.insert("prov-form".into());
                cx.notify();
            }))
            .child("+ Add")
            .into_any_element()
    }

    /// Fill the name and address boxes from the core, once per load. Typing is kept until the next save or reload.
    fn prov_fill(&mut self, p: &m::Provider) {
        if self.pg.prov_filled.insert(p.id.clone()) {
            self.pg.set_next.insert(format!("prov.edit.name.{}", p.id), p.name.clone());
            self.pg.set_next.insert(format!("prov.edit.url.{}", p.id), p.url.clone());
        }
    }

    /// The right pane for one saved provider: header, connection, models, and the footer.
    fn provider_detail(&mut self, p: m::Provider, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let id = p.id.clone();
        let pal = self.pal.clone();
        self.prov_fill(&p);
        let tested = self.pg.prov_test.get(&id).copied();
        let (light, words) = m::light(&p, tested);
        let title = if p.name.is_empty() { p.id.clone() } else { p.name.clone() };

        // header: name, status, Test, and the menu
        let mut head = div().flex().items_center().gap_2().child(
            div().flex_1().min_w_0().flex().flex_col().child(div().text_color(pal.text).font_weight(gpui::FontWeight::MEDIUM).child(title.clone())).child(
                div().flex().gap_1().text_xs().text_color(self.light_color(light)).child(format!("{}  {}", light.glyph(), words)).children(p.default.then(|| div().text_color(pal.accent).child("·  Default"))),
            ),
        );
        let n = id.clone();
        head = head.child(self.button(format!("pg-prov-test-{id}"), "Test", Kind::Normal, cx, move |a, _, cx| a.pg_act("providers", &format!("prov-test:{n}"), "POST", format!("/v1/providers/{n}/test"), Some(json!({})), cx)));
        let menu_key = format!("prov-menu:{id}");
        let menu_open = self.pg.open.contains(&menu_key);
        let mk = menu_key.clone();
        head = head.child(self.button(format!("pg-prov-menu-{id}"), "⋯", Kind::Ghost, cx, move |a, _, cx| {
            if !a.pg.open.remove(&mk) {
                a.pg.open.insert(mk.clone());
            }
            cx.notify();
        }));
        let mut col = div().flex().flex_col().gap_3().p_3().rounded(pal.inner).bg(pal.sunken).text_color(pal.text).min_w_0().child(head);

        if menu_open {
            let mut menu = div().flex().flex_wrap().gap_1().p_2().rounded(pal.inner).border_1().border_color(pal.border);
            if !p.default {
                let n = id.clone();
                let mk = menu_key.clone();
                menu = menu.child(self.button(format!("pg-prov-def-{id}"), "Make default", Kind::Normal, cx, move |a, _, cx| {
                    a.pg.open.remove(&mk);
                    a.pg_act("providers", &format!("prov-default:{n}"), "POST", format!("/v1/providers/{n}/default"), None, cx)
                }));
            }
            menu = menu.child(self.pg_confirm_button(&format!("prov-rm-menu-{id}"), "Remove", "Yes, remove it", cx, {
                let n = id.clone();
                move |a, _, cx| a.pg_act("providers", &format!("prov-remove:{n}"), "DELETE", format!("/v1/providers/{n}"), None, cx)
            }));
            col = col.child(menu);
        }

        // connection group: key (masked), API style (picker), name and address
        let mut conn = div().flex().flex_col().gap_2();
        let key_open = self.pg.open.contains(&format!("prov-key:{id}"));
        let kk = format!("prov.edit.key.{id}");
        let key_row_label = if key_open { "Cancel" } else { "Change" };
        let kid = id.clone();
        let mut key_row = div().flex().items_center().gap_2().child(div().flex_1().min_w_0().text_sm().text_color(pal.muted).child("API key")).child(div().text_sm().child(m::key_words(&p)));
        key_row = key_row.child(self.button(format!("pg-prov-key-{id}"), key_row_label, Kind::Normal, cx, move |a, _, cx| {
            let k = format!("prov-key:{kid}");
            if !a.pg.open.remove(&k) {
                a.pg.open.insert(k);
            }
            cx.notify();
        }));
        conn = conn.child(key_row);
        if key_open {
            conn = conn.child(self.xp_secret(&kk, "New key", window, cx));
        }
        if p.key_saved {
            let rm = id.clone();
            conn = conn.child(div().flex().child(self.pg_confirm_button(&format!("prov-key-rm-{id}"), "Remove the saved key", "Yes, remove the key", cx, move |a, _, cx| a.pg_act("providers", &format!("prov-edit:{rm}"), "PATCH", format!("/v1/providers/{rm}"), Some(json!({"remove_key": true})), cx))));
        }

        let api_open = self.pg.open.contains(&format!("prov-api:{id}"));
        let shown_api = match self.pg.prov_api.get(&id) {
            Some(a) => m::api_label(a),
            None => m::api_label(&p.api),
        };
        let aid = id.clone();
        let mut api_row = div().flex().flex_col().gap_1().child(
            div().flex().items_center().gap_2().child(div().flex_1().min_w_0().text_sm().text_color(pal.muted).child("API style")).child(self.button(format!("pg-prov-apipick-{id}"), format!("{shown_api}  ▾"), Kind::Normal, cx, move |a, _, cx| {
                let k = format!("prov-api:{aid}");
                if !a.pg.open.remove(&k) {
                    a.pg.open.insert(k);
                }
                cx.notify();
            })),
        );
        if api_open {
            let mut opts = div().flex().flex_wrap().gap_1();
            for (val, label) in [("openai", "OpenAI style"), ("anthropic", "Anthropic")] {
                let sid = id.clone();
                opts = opts.child(self.button(format!("pg-prov-api-{val}-{id}"), label, if shown_api == label { Kind::Primary } else { Kind::Normal }, cx, move |a, _, cx| {
                    a.pg.prov_api.insert(sid.clone(), val.to_string());
                    a.pg.open.remove(&format!("prov-api:{sid}"));
                    cx.notify();
                }));
            }
            api_row = api_row.child(opts);
        }
        conn = conn.child(api_row);
        conn = conn.child(self.xp_field(&format!("prov.edit.name.{id}"), "Name", &p.name, window, cx));
        conn = conn.child(self.xp_field(&format!("prov.edit.url.{id}"), "Base URL", &p.url, window, cx));
        col = col.child(conn);

        // models group: a checklist of what the provider lists, and a refresh
        let mut models = div().flex().flex_col().gap_1();
        let n = id.clone();
        models = models.child(div().flex().items_center().gap_2().child(div().flex_1().min_w_0().text_sm().text_color(pal.muted).child("Models")).child(self.button(format!("pg-prov-models-{id}"), "Refresh models", Kind::Normal, cx, move |a, _, cx| a.pg_act("providers", &format!("prov-models:{n}"), "POST", format!("/v1/providers/{n}/models"), None, cx))));
        if p.models.is_empty() {
            models = models.child(self.pg_muted(m::models_words(&p)));
        } else {
            for name in &p.models {
                models = models.child(div().flex().items_center().gap_2().text_sm().child(div().text_color(pal.accent).child("☑")).child(name.clone()));
            }
        }
        col = col.child(models);

        // footer: Remove provider (secondary, left) and Save (primary, right)
        let save_id = id.clone();
        let save = self.button(format!("pg-prov-save-{id}"), "Save", Kind::Primary, cx, move |a, _, cx| {
            let (name, url) = (a.pg_text(&format!("prov.edit.name.{save_id}"), cx), a.pg_text(&format!("prov.edit.url.{save_id}"), cx));
            let key = if a.pg.open.contains(&format!("prov-key:{save_id}")) { a.pg_text(&format!("prov.edit.key.{save_id}"), cx) } else { String::new() };
            let api = a.pg.prov_api.get(&save_id).cloned().unwrap_or_default();
            match m::edit_body(&name, &url, &key, &api) {
                Ok(body) => {
                    a.pg.prov_api.remove(&save_id);
                    a.pg.open.remove(&format!("prov-key:{save_id}"));
                    a.pg.open.remove(&format!("prov-api:{save_id}"));
                    a.pg_clear(&[format!("prov.edit.key.{save_id}").as_str()]);
                    a.pg_act("providers", &format!("prov-edit:{save_id}"), "PATCH", format!("/v1/providers/{save_id}"), Some(body), cx);
                }
                Err(e) => {
                    a.pg_note_set("providers", true, e);
                    cx.notify();
                }
            }
        });
        let footer = div()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap_2()
            .pt_2()
            .border_t_1()
            .border_color(pal.border)
            .child(self.pg_confirm_button(&format!("prov-rm-{id}"), "Remove provider", "Yes, remove it", cx, {
                let n = id.clone();
                move |a, _, cx| a.pg_act("providers", &format!("prov-remove:{n}"), "DELETE", format!("/v1/providers/{n}"), None, cx)
            }))
            .child(save);
        col.child(footer).into_any_element()
    }

    fn registry_proposal_card(&mut self, q: m::Proposal, presets: &[m::Preset], window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let id = q.id.clone();
        let needs = m::key_required(&q.preset, presets);
        let key_key = format!("prov.pkey.{id}");
        let label = if needs { "Key (this service needs one)" } else { "Key (leave empty if none is needed)" };
        let key_box = self.xp_secret(&key_key, label, window, cx);
        let (a_id, r_id) = (id.clone(), id.clone());
        let approve = self.button(format!("pg-prov-ok-{id}"), "Approve", Kind::Primary, cx, move |a, _, cx| {
            let key = a.pg_text(&format!("prov.pkey.{a_id}"), cx);
            if m::key_required(&q_preset(a, &a_id), &a.prov_presets()) && key.trim().is_empty() {
                a.pg_note_set("providers", true, "This service needs a key. Type it in the box, then approve.");
                cx.notify();
                return;
            }
            a.pg_clear(&[format!("prov.pkey.{a_id}").as_str()]);
            a.pg_act("providers", &format!("prov-approve:{a_id}"), "POST", format!("/v1/providers/proposals/{a_id}/approve"), Some(m::approve_body(&key)), cx);
        });
        let reject = self.button(format!("pg-prov-no-{id}"), "Reject", Kind::Ghost, cx, move |a, _, cx| {
            a.pg_clear(&[format!("prov.pkey.{r_id}").as_str()]);
            a.pg_act("providers", &format!("prov-reject:{r_id}"), "POST", format!("/v1/providers/proposals/{r_id}/reject"), None, cx)
        });
        let rows = vec![
            self.pg_title(format!("The assistant proposes: {}", if q.name.is_empty() { &q.id } else { &q.name })),
            self.pg_muted(q.url.clone()),
            self.pg_muted(if q.reason.is_empty() { "No reason given.".to_string() } else { format!("Why: {}", q.reason) }),
            key_box,
            div().flex().flex_wrap().gap_1().child(approve).child(reject).into_any_element(),
        ];
        self.xp_card(rows).id(SharedString::from(format!("pg-prov-prop-{id}"))).role(Role::Group).aria_label(format!("Proposed provider {id}")).into_any_element()
    }

    fn prov_presets(&self) -> Vec<m::Preset> {
        self.pg_list("prov-presets").and_then(|r| r.ok()).map(|v| m::presets(&v)).unwrap_or_default()
    }

    /// The add flow, in the right pane: a preset or an address, a name, a key, then Add, Test or Cancel.
    fn registry_form(&mut self, presets: &[m::Preset], window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let chosen = self.pg.prov_preset.clone();
        let mut chips = div().flex().flex_wrap().gap_1();
        for p in presets.iter().filter(|p| p.supported) {
            let id = p.id.clone();
            chips = chips.child(self.button(format!("pg-prov-preset-{}", p.id), p.name.clone(), if p.id == chosen { Kind::Primary } else { Kind::Normal }, cx, move |a, _, cx| {
                a.pg.prov_preset = id.clone();
                cx.notify();
            }));
        }
        chips = chips.child(self.button("pg-prov-preset-custom", "Another address", if chosen.is_empty() { Kind::Primary } else { Kind::Normal }, cx, |a, _, cx| {
            a.pg.prov_preset = String::new();
            cx.notify();
        }));
        let mut rows = vec![self.pg_title("Add a provider"), chips.into_any_element()];
        if let Some(p) = presets.iter().find(|p| p.id == chosen) {
            let mut line = p.url.clone();
            if !p.note.is_empty() {
                line = format!("{line}. {}", p.note);
            }
            rows.push(self.pg_muted(line));
        } else {
            rows.push(self.xp_field("prov.url", "Address", "http://localhost:8080/v1", window, cx));
        }
        rows.push(self.xp_field("prov.name", "Name (optional)", "", window, cx));
        let key_label = match presets.iter().find(|p| p.id == chosen) {
            Some(p) if p.key_required => "Key (needed)",
            Some(_) => "Key (leave empty if none is needed)",
            None => "Key (leave empty if none is needed)",
        };
        rows.push(self.xp_secret("prov.key", key_label, window, cx));
        let save = self.button("pg-prov-save", "Add", Kind::Primary, cx, |a, _, cx| {
            let presets = a.prov_presets();
            let chosen = a.pg.prov_preset.clone();
            let (url, name, key) = (a.pg_text("prov.url", cx), a.pg_text("prov.name", cx), a.pg_text("prov.key", cx));
            let preset = (!chosen.is_empty()).then_some(chosen.as_str());
            match m::add_body(preset, &url, &name, &key, &presets) {
                Ok(body) => {
                    let label = if name.trim().is_empty() { preset.unwrap_or(url.trim()).to_string() } else { name.trim().to_string() };
                    a.pg_clear(&["prov.url", "prov.name", "prov.key"]);
                    a.pg.open.remove("prov-form");
                    a.pg_act("providers", &format!("prov-add:{label}"), "POST", "/v1/providers", Some(body), cx);
                }
                Err(e) => {
                    a.pg_note_set("providers", true, e);
                    cx.notify();
                }
            }
        });
        let test = self.button("pg-prov-test-form", "Test", Kind::Normal, cx, |a, _, cx| {
            let presets = a.prov_presets();
            let chosen = a.pg.prov_preset.clone();
            let url = match presets.iter().find(|p| p.id == chosen) {
                Some(p) => p.url.clone(),
                None => a.pg_text("prov.url", cx),
            };
            let key = a.pg_text("prov.key", cx);
            a.pg_act("providers", "prov-test:form", "POST", "/v1/provider/test", Some(m::test_body(&url, &key, None)), cx);
        });
        let cancel = self.button("pg-prov-cancel", "Cancel", Kind::Ghost, cx, |a, _, cx| {
            a.pg_clear(&["prov.url", "prov.name", "prov.key"]);
            a.pg.open.remove("prov-form");
            cx.notify();
        });
        rows.push(div().flex().gap_1().child(save).child(test).child(cancel).into_any_element());
        self.xp_card(rows).into_any_element()
    }
}

/// The preset a proposal names, looked up again when Approve is pressed.
fn q_preset(a: &App, id: &str) -> String {
    a.pg_list("prov-proposals")
        .and_then(|r| r.ok())
        .map(|v| m::proposals(&v))
        .and_then(|l| l.into_iter().find(|q| q.id == id))
        .map(|q| q.preset)
        .unwrap_or_default()
}
