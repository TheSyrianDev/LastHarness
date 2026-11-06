//! Providers, under Models and keys: the registered model services, adding one from a preset or an address, the
//! default, removing, and the providers the assistant proposed. A key is typed here by the person, in a masked
//! box, sent once, and never shown again.

use gpui::prelude::*;
use gpui::*;
use gpui_component::input::{Input, InputState};

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
        if list.is_empty() {
            col = col.child(self.pg_note_line("No providers registered yet."));
        }
        for p in list {
            col = col.child(self.provider_card(p, cx));
        }
        for q in proposals {
            col = col.child(self.registry_proposal_card(q, &presets, window, cx));
        }
        col = col.child(self.registry_form(&presets, window, cx));
        col.child(div().flex().child(self.pg_refresh(Category::Models, cx))).into_any_element()
    }

    fn provider_card(&mut self, p: m::Provider, cx: &mut Context<Self>) -> AnyElement {
        let id = p.id.clone();
        let pal = self.pal.clone();
        let mut head = div().flex().flex_wrap().items_center().gap_2().child(div().flex_1().child(if p.name.is_empty() { p.id.clone() } else { p.name.clone() }));
        if p.default {
            head = head.child(div().text_sm().text_color(pal.accent).child("Default"));
        }
        let mut buttons = div().flex().flex_wrap().gap_1();
        if !p.default {
            let n = id.clone();
            buttons = buttons.child(self.button(format!("pg-prov-def-{id}"), "Make default", Kind::Normal, cx, move |a, _, cx| a.pg_act("providers", &format!("prov-default:{n}"), "POST", format!("/v1/providers/{n}/default"), None, cx)));
        }
        let n = id.clone();
        buttons = buttons.child(self.pg_confirm_button(&format!("prov-rm-{id}"), "Remove", "Yes, remove it", cx, move |a, _, cx| a.pg_act("providers", &format!("prov-remove:{n}"), "DELETE", format!("/v1/providers/{n}"), None, cx)));
        self.xp_card(vec![head.into_any_element(), self.pg_muted(m::state_words(&p)), buttons.into_any_element()]).id(SharedString::from(format!("pg-prov-{id}"))).role(Role::Group).aria_label(format!("Provider {id}")).into_any_element()
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

    fn registry_form(&mut self, presets: &[m::Preset], window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        if !self.pg.open.contains("prov-form") {
            return div().flex().child(self.button("pg-prov-new", "Add a provider", Kind::Normal, cx, |a, _, cx| {
                a.pg.prov_preset = String::new();
                a.pg.open.insert("prov-form".into());
                cx.notify();
            })).into_any_element();
        }
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
        let cancel = self.button("pg-prov-cancel", "Cancel", Kind::Ghost, cx, |a, _, cx| {
            a.pg_clear(&["prov.url", "prov.name", "prov.key"]);
            a.pg.open.remove("prov-form");
            cx.notify();
        });
        rows.push(div().flex().gap_1().child(save).child(cancel).into_any_element());
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
