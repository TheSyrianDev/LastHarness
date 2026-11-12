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
        let add_top = self.button("pg-prov-add-top", "+ Add provider", Kind::Primary, cx, |a, _, cx| {
            prov_form_close(a);
            a.pg.open.insert("prov-form".into());
            cx.notify();
        });
        // Refresh sits in the header, next to "+ Add provider"
        let header_actions = div().flex().items_center().gap_2().child(self.pg_refresh(Category::Models, cx)).child(add_top);
        col = col.child(div().flex().flex_wrap().items_center().justify_between().gap_2().child(self.pg_title("Providers")).child(header_actions));
        col = col.child(self.pg_muted("Model services this app can use. The default answers when a role does not name one. A key is typed here only, kept in the core, and never shown again."));
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

        // one full-width row per provider; Manage opens its detail below that row, and only then
        let mut rows = div().flex().flex_col().gap_1p5().w_full();
        for p in &list {
            let manage = !adding && sel.as_deref() == Some(p.id.as_str()) && self.pg.open.contains("prov-manage");
            rows = rows.child(self.prov_row(p, manage, cx));
            if manage {
                rows = rows.child(div().w_full().pl_4().child(self.provider_detail(p.clone(), window, cx)));
            }
        }
        col = col.child(rows);
        if adding {
            col = col.child(self.registry_form(&presets, window, cx));
        }
        col = col.child(self.prov_default_row(&list, cx));
        col.into_any_element()
    }

    /// The default model line: its words and a Change button that opens a picker of tested providers' models.
    fn prov_default_row(&mut self, list: &[m::Provider], cx: &mut Context<Self>) -> AnyElement {
        let pal = self.pal.clone();
        let open = self.pg.open.contains("prov-default-picker");
        let words = m::default_words(list).unwrap_or_else(|| "none set".to_string());
        let mut row = div().flex().flex_wrap().items_center().justify_between().gap_2().pt_2().border_t_1().border_color(pal.border).child(div().text_sm().text_color(pal.text).child(format!("Default model: {words}"))).child(self.button("pg-prov-default-change", if open { "Close" } else { "Change" }, Kind::Normal, cx, |a, _, cx| {
            if !a.pg.open.remove("prov-default-picker") {
                a.pg.open.insert("prov-default-picker".into());
            }
            cx.notify();
        }));
        if open {
            let items = m::pickable(list, &self.pg.prov_test);
            let mut pick = div().flex().flex_col().gap_1().p_2().rounded(pal.inner).bg(pal.sunken);
            if items.is_empty() {
                pick = pick.child(self.pg_muted("Only providers that passed a test are listed. Test one first."));
            }
            for (pid, pname, model_name) in items {
                let (dp, dm) = (pid.clone(), model_name.clone());
                pick = pick.child(self.button(format!("pg-prov-dpick-{pid}-{model_name}"), format!("{model_name} ({pname})"), Kind::Normal, cx, move |a, _, cx| {
                    a.pg.open.remove("prov-default-picker");
                    a.pg_act("providers", &format!("prov-default-model:{dp}"), "POST", format!("/v1/providers/{dp}/default-model"), Some(m::default_model_body(Some(&dm))), cx);
                    a.pg_act("providers", &format!("prov-default:{dp}"), "POST", format!("/v1/providers/{dp}/default"), None, cx);
                }));
            }
            row = row.child(div().w_full().child(pick));
        }
        row.into_any_element()
    }

    fn light_color(&self, l: m::Light) -> Hsla {
        match l {
            m::Light::Ready => self.pal.good,
            m::Light::Attention => self.pal.warn,
            m::Light::Off => self.pal.muted,
        }
    }

    /// One provider as a full-width row: status dot, name, full address, and a Manage button that is never clipped.
    /// `open` tints the row and means its detail is shown below it.
    fn prov_row(&mut self, p: &m::Provider, open: bool, cx: &mut Context<Self>) -> AnyElement {
        let tested = self.pg.prov_test.get(&p.id).copied();
        let (light, words) = m::light(p, tested);
        let color = self.light_color(light);
        let id = p.id.clone();
        let label = if p.name.is_empty() { p.id.clone() } else { p.name.clone() };
        let hover_bg = self.pal.button;
        let tint = self.pal.accent.opacity(0.1);
        // no key yet for a remote service: the button opens the key step; otherwise it selects the provider
        let signin = !p.local && !p.key_saved;
        let (btn_label, btn_id) = if signin { ("Sign in", format!("pg-prov-signin-{}", p.id)) } else { ("Manage", format!("pg-prov-manage-{}", p.id)) };
        let (bid, key_open) = (id.clone(), format!("prov-key:{}", p.id));
        let button = self.button(btn_id, btn_label, Kind::Normal, cx, move |a, _, cx| {
            let same = a.pg.prov_sel.as_deref() == Some(bid.as_str());
            if same && !signin && a.pg.open.contains("prov-manage") {
                a.pg.open.remove("prov-manage");
            } else {
                a.pg.prov_sel = Some(bid.clone());
                a.pg.open.remove("prov-form");
                a.pg.open.insert("prov-manage".into());
                if signin {
                    a.pg.open.insert(key_open.clone());
                }
            }
            cx.notify();
        });
        let muted = self.pal.muted;
        let endpoint = if p.detected { format!("Detected  ·  {}", p.url) } else { format!("{}  ·  {}", p.url, m::short_models(p)) };
        div()
            .id(SharedString::from(format!("pg-prov-row-{}", p.id)))
            .role(Role::Button)
            .aria_label(format!("{label}, {words}"))
            .w_full()
            .flex()
            .items_center()
            .gap_3()
            .px_3()
            .py_2()
            .rounded(self.pal.inner)
            .border_1()
            .border_color(self.pal.border)
            .text_sm()
            .text_color(self.pal.text)
            .bg(if open { tint } else { hsla(0., 0., 0., 0.) })
            .hover(move |s| s.bg(hover_bg))
            .child(div().flex_shrink_0().text_color(color).child(light.glyph()))
            .child(div().flex_1().min_w_0().child(div().child(label)).child(div().text_xs().text_color(muted).child(endpoint)))
            .child(div().flex_shrink_0().child(button))
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
        if p.detected {
            // found on this computer, not saved: Test only, no menu, no edit, remove or make-default
            return div().flex().flex_col().gap_3().p_3().rounded(pal.inner).bg(pal.sunken).text_color(pal.text).min_w_0().child(head).child(self.pg_muted("Found running on this computer. It is not saved, so it cannot be changed here. Add it as a provider to keep it.")).into_any_element();
        }
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
            {
                let (en, on, mk) = (id.clone(), !p.enabled, menu_key.clone());
                let label = if p.enabled { "Turn off" } else { "Turn on" };
                menu = menu.child(self.button(format!("pg-prov-enable-{id}"), label, Kind::Normal, cx, move |a, _, cx| {
                    a.pg.open.remove(&mk);
                    a.pg_act("providers", &format!("prov-enable:{en}"), "PATCH", format!("/v1/providers/{en}"), Some(json!({"enabled": on})), cx)
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
                let on = p.model_on(name);
                let is_default = p.default_model.as_deref() == Some(name.as_str());
                let (mn, mid) = (id.clone(), name.clone());
                let box_glyph = if on { "☑" } else { "☐" };
                let mut row = div().flex().items_center().gap_2().text_sm().child(self.button(format!("pg-prov-model-{id}-{name}"), box_glyph, Kind::Ghost, cx, move |a, _, cx| {
                    a.pg_act("providers", &format!("prov-model:{mn}"), "POST", format!("/v1/providers/{mn}/models/toggle"), Some(m::toggle_body(&mid, !on)), cx)
                })).child(div().flex_1().min_w_0().text_color(if on { pal.text } else { pal.muted }).child(name.clone()));
                if is_default {
                    row = row.child(div().text_xs().text_color(pal.accent).child("Default"));
                } else if on {
                    let (dn, dm) = (id.clone(), name.clone());
                    row = row.child(self.button(format!("pg-prov-mdef-{id}-{name}"), "Make default", Kind::Ghost, cx, move |a, _, cx| {
                        a.pg_act("providers", &format!("prov-default-model:{dn}"), "POST", format!("/v1/providers/{dn}/default-model"), Some(m::default_model_body(Some(&dm))), cx)
                    }));
                }
                models = models.child(row);
            }
        }
        col = col.child(models);

        // footer: Remove provider (secondary, left) and Save (primary, right)
        let save_id = id.clone();
        let save = self.button(format!("pg-prov-save-{id}"), "Save", Kind::Primary, cx, move |a, _, cx| {
            let (name, url) = (a.pg_text(&format!("prov.edit.name.{save_id}"), cx), a.pg_text(&format!("prov.edit.url.{save_id}"), cx));
            let key = if a.pg.open.contains(&format!("prov-key:{save_id}")) { a.pg_text(&format!("prov.edit.key.{save_id}"), cx) } else { String::new() };
            let key = match m::resolve_key(&key, |n| std::env::var(n).ok()) {
                Ok(k) => k,
                Err(e) => {
                    a.pg_note_set("providers", true, e);
                    cx.notify();
                    return;
                }
            };
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
            let body = match m::approve_key(&key, |n| std::env::var(n).ok()) {
                Ok(b) => b,
                Err(e) => {
                    a.pg_note_set("providers", true, &e);
                    cx.notify();
                    return;
                }
            };
            a.pg_clear(&[format!("prov.pkey.{a_id}").as_str()]);
            a.pg_act("providers", &format!("prov-approve:{a_id}"), "POST", format!("/v1/providers/proposals/{a_id}/approve"), Some(body), cx);
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

    /// The add flow, in the right pane. Step 1 picks how to connect, step 2 names it and tests it, step 3 picks
    /// the models to keep and saves. Only a passed test lets the flow reach step 3.
    fn registry_form(&mut self, presets: &[m::Preset], window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let step = self.pg.prov_step.clamp(1, 3);
        let method = self.pg.prov_method.clone();
        let chosen = self.pg.prov_preset.clone();
        let mut rows = vec![self.pg_title("Add a provider"), self.pg_muted(format!("Step {step} of 3"))];
        let cancel = self.button("pg-prov-cancel", "Cancel", Kind::Ghost, cx, |a, _, cx| {
            prov_form_close(a);
            cx.notify();
        });
        match step {
            1 => {
                rows.push(self.pg_muted("How do you connect?"));
                let mut opts = div().flex().flex_col().gap_1();
                for (val, label) in [("key", "API key (a known service)"), ("custom", "Custom endpoint (OpenAI-compatible)")] {
                    let on = method == val;
                    let glyph = if on { "●" } else { "○" };
                    opts = opts.child(self.button(format!("pg-prov-method-{val}"), format!("{glyph}  {label}"), if on { Kind::Primary } else { Kind::Normal }, cx, move |a, _, cx| {
                        a.pg.prov_method = val.to_string();
                        cx.notify();
                    }));
                }
                rows.push(opts.into_any_element());
                let next = self.button("pg-prov-next", "Next", Kind::Primary, cx, |a, _, cx| {
                    if !a.pg.prov_method.is_empty() {
                        a.pg.prov_step = 2;
                    }
                    cx.notify();
                });
                rows.push(div().flex().gap_1().child(next).child(cancel).into_any_element());
            }
            2 => {
                if method == "key" {
                    let mut chips = div().flex().flex_wrap().gap_1();
                    for p in presets.iter().filter(|p| p.supported) {
                        let id = p.id.clone();
                        chips = chips.child(self.button(format!("pg-prov-preset-{}", p.id), p.name.clone(), if p.id == chosen { Kind::Primary } else { Kind::Normal }, cx, move |a, _, cx| {
                            a.pg.prov_preset = id.clone();
                            a.pg.prov_form_found = None;
                            cx.notify();
                        }));
                    }
                    rows.push(chips.into_any_element());
                }
                let preset = presets.iter().find(|p| p.id == chosen && method == "key");
                match preset {
                    Some(p) => {
                        let mut line = p.url.clone();
                        if !p.note.is_empty() {
                            line = format!("{line}. {}", p.note);
                        }
                        rows.push(self.pg_muted(line));
                    }
                    None => rows.push(self.xp_field("prov.url", "Base URL", "http://localhost:8080/v1", window, cx)),
                }
                rows.push(self.xp_field("prov.name", "Name", "", window, cx));
                rows.push(self.xp_secret("prov.key", "API key (type the key, or the name of an environment variable)", window, cx));
                let test = self.button("pg-prov-test-form", "Test connection", Kind::Normal, cx, |a, _, cx| {
                    let presets = a.prov_presets();
                    let chosen = a.pg.prov_preset.clone();
                    let url = match presets.iter().find(|p| p.id == chosen && a.pg.prov_method == "key") {
                        Some(p) => p.url.clone(),
                        None => a.pg_text("prov.url", cx),
                    };
                    let key = match m::resolve_key(&a.pg_text("prov.key", cx), |n| std::env::var(n).ok()) {
                        Ok(k) => k,
                        Err(e) => {
                            a.pg_note_set("providers", true, e);
                            cx.notify();
                            return;
                        }
                    };
                    a.pg_act("providers", "prov-test:form", "POST", "/v1/provider/test", Some(m::test_body(&url, &key, None)), cx);
                });
                rows.push(test);
                if let Some((_, words, _)) = &self.pg.prov_form_found {
                    rows.push(self.pg_muted(words.clone()));
                }
                let back = self.button("pg-prov-back", "Back", Kind::Ghost, cx, |a, _, cx| {
                    a.pg.prov_step = 1;
                    cx.notify();
                });
                let next = self.button("pg-prov-next", "Next", Kind::Primary, cx, |a, _, cx| {
                    if a.pg.prov_form_found.as_ref().is_some_and(|f| f.0) {
                        a.pg.prov_step = 3;
                    } else {
                        a.pg_note_set("providers", true, "Test the connection first. Next opens once it finds models.");
                    }
                    cx.notify();
                });
                rows.push(div().flex().gap_1().child(back).child(next).child(cancel).into_any_element());
            }
            _ => {
                let (models, off) = match &self.pg.prov_form_found {
                    Some((_, _, models)) => (models.clone(), self.pg.prov_form_off.clone()),
                    None => (Vec::new(), Default::default()),
                };
                rows.push(self.pg_muted("Untick a model to leave it out. Models are on by default."));
                if models.is_empty() {
                    rows.push(self.pg_muted("The server lists no models. Save to add the provider anyway."));
                }
                for name in models {
                    let on = !off.contains(&name);
                    let glyph = if on { "☑" } else { "☐" };
                    let nm = name.clone();
                    rows.push(self.button(format!("pg-prov-off-{name}"), format!("{glyph}  {name}"), Kind::Ghost, cx, move |a, _, cx| {
                        if !a.pg.prov_form_off.remove(&nm) {
                            a.pg.prov_form_off.insert(nm.clone());
                        }
                        cx.notify();
                    }));
                }
                let back = self.button("pg-prov-back", "Back", Kind::Ghost, cx, |a, _, cx| {
                    a.pg.prov_step = 2;
                    cx.notify();
                });
                let save = self.button("pg-prov-save", "Save", Kind::Primary, cx, |a, _, cx| {
                    let presets = a.prov_presets();
                    let chosen = a.pg.prov_preset.clone();
                    let key_method = a.pg.prov_method == "key";
                    let (url, name) = (a.pg_text("prov.url", cx), a.pg_text("prov.name", cx));
                    let key = match m::resolve_key(&a.pg_text("prov.key", cx), |n| std::env::var(n).ok()) {
                        Ok(k) => k,
                        Err(e) => {
                            a.pg_note_set("providers", true, e);
                            cx.notify();
                            return;
                        }
                    };
                    let preset = (key_method && !chosen.is_empty()).then_some(chosen.as_str());
                    match m::add_body(preset, &url, &name, &key, &presets) {
                        Ok(body) => {
                            let label = if name.trim().is_empty() { preset.unwrap_or(url.trim()).to_string() } else { name.trim().to_string() };
                            let off: Vec<String> = a.pg.prov_form_off.iter().cloned().collect();
                            a.pg.prov_pending = Some((off, true));
                            prov_form_close(a);
                            a.pg_act("providers", &format!("prov-add:{label}"), "POST", "/v1/providers", Some(body), cx);
                        }
                        Err(e) => {
                            a.pg_note_set("providers", true, e);
                            cx.notify();
                        }
                    }
                });
                rows.push(div().flex().gap_1().child(back).child(save).child(cancel).into_any_element());
            }
        }
        self.xp_card(rows).into_any_element()
    }
}

/// Close the add flow and forget its typed boxes and its test.
fn prov_form_close(a: &mut App) {
    a.pg_clear(&["prov.url", "prov.name", "prov.key"]);
    a.pg.open.remove("prov-form");
    a.pg.prov_step = 1;
    a.pg.prov_method.clear();
    a.pg.prov_preset.clear();
    a.pg.prov_form_found = None;
    a.pg.prov_form_off.clear();
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
