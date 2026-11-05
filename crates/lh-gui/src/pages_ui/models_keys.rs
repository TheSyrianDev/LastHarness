//! Models and keys: the model provider, its key, a connection test. The image model pickers stay below as
//! ordinary setting cards.

use gpui::prelude::*;
use gpui::*;
use serde_json::json;

use crate::app::App;
use crate::ui::Kind;
use lh_gui::api::Cmd;
use lh_gui::pages_logic::models_keys::{check_url, looks_like_ollama, save_body, test_body, ProviderInfo, Status, TestResult};

#[derive(Default)]
pub struct State {
    /// The edit form is open.
    pub editing: bool,
    /// A test is running.
    pub testing: bool,
    /// What the last test found, for the address in the form.
    pub result: Option<TestResult>,
}

impl App {
    pub(super) fn models_keys_block(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let info = self.sp.data.get("provider").map(ProviderInfo::from_json);
        let mut col = div().flex().flex_col().gap_2();
        let failed = self.sp.errors.contains_key("provider");
        if let Some(e) = self.pg_error("provider") {
            col = col.child(e);
        }
        let Some(info) = info else {
            // Core unreachable: do not show the "asking" text.
            if failed {
                return col.into_any_element();
            }
            return col.child(self.pg_small("Asking the core about your model service…")).into_any_element();
        };
        let status = info.status();
        let dot = match status {
            Status::Working => p.good,
            Status::Down => p.danger,
            Status::NotSet => p.muted,
        };
        // an unset provider shows the form straight away
        let editing = self.sp.models_keys.editing || status == Status::NotSet;
        col = col.child(self.pg_heading("Model service"));
        if status != Status::NotSet {
            let edit = self.button("prov-edit", if editing { "Close" } else { "Edit" }, Kind::Normal, cx, {
                let (url, name) = (info.url.clone(), info.name.clone());
                move |a, w, cx| {
                    let open = !a.sp.models_keys.editing;
                    a.sp.models_keys.editing = open;
                    a.sp.models_keys.result = None;
                    if open {
                        a.pg_set("prov.url", &url, w, cx);
                        a.pg_set("prov.name", &name, w, cx);
                    }
                    a.pg_set("prov.key", "", w, cx);
                    cx.notify();
                }
            });
            let test = self.test_button("prov-test-saved", info.url.clone(), String::new(), cx);
            let key_words = if info.key_saved { "A key is saved." } else { "No key." };
            col = col.child(
                self.pg_card()
                    .id("provider-row")
                    .role(Role::Group)
                    .aria_label(format!("{}: {}", info.title(), status.words()))
                    .child(div().flex().flex_wrap().items_center().gap_2().child(div().w(px(10.)).h(px(10.)).rounded_full().bg(dot)).child(div().flex_1().min_w_0().child(info.title())).child(div().text_sm().text_color(p.muted).child(status.words())).child(test).child(edit))
                    .child(self.pg_small(format!("{}   {}   {}", info.url, if info.local { "On this computer." } else { "Another computer or a cloud service." }, key_words)))
                    .when(status == Status::Working, |c| c.child(self.pg_small(format!("It offers {} model{}.", info.models, if info.models == 1 { "" } else { "s" }))))
                    .when(info.from_start, |c| c.child(self.pg_small("This address comes from how the core was started. Save it here to keep it in your settings."))),
            );
            if !editing {
                if let Some(r) = self.sp.models_keys.result.clone() {
                    col = col.child(self.test_result_view(&r));
                }
            }
        } else {
            col = col.child(self.pg_small("No model service is set up yet. Type where your models are served from. A key is only needed for services that ask for one."));
        }
        if editing {
            col = col.child(self.provider_form(&info, window, cx));
        }
        if let Some(n) = self.pg_note("provider") {
            col = col.child(n);
        }
        col.into_any_element()
    }

    fn provider_form(&mut self, info: &ProviderInfo, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let url_box = self.pg_field("Address", "prov.url", "http://localhost:9292/v1", window, cx);
        let name_box = self.pg_field("Name (optional)", "prov.name", "Told to the model so it knows what serves it", window, cx);
        let key_ph = if info.key_saved { "Saved. Type a new key to replace it." } else { "Only if the service asks for one" };
        let key_in = self.pg_input("prov.key", key_ph, true, window, cx);
        let key_box = div().flex().flex_col().gap_1().child(div().text_sm().child("Key")).child(gpui_component::input::Input::new(&key_in).aria_label("Key")).child(self.pg_small("Kept in your system's secret store. It is not shown again.")).into_any_element();
        let url = self.pg_val("prov.url", cx);
        let typed_key = self.pg_val("prov.key", cx);
        let test = self.test_button("prov-test", url.clone(), typed_key.clone(), cx);
        let ollama = looks_like_ollama(&url).then(|| {
            let (u, k) = (url.clone(), typed_key.clone());
            self.button("prov-ollama", "Look for Ollama", Kind::Normal, cx, move |a, _, cx| a.run_provider_test(u.clone(), k.clone(), cx))
        });
        let save = self.button("prov-save", "Save", Kind::Primary, cx, {
            let had_url = !info.url.is_empty();
            move |a, _, cx| {
                let (url, name, key) = (a.pg_val("prov.url", cx), a.pg_val("prov.name", cx), a.pg_val("prov.key", cx));
                if let Some(m) = check_url(&url) {
                    a.sp.notes.insert("provider".into(), (false, m));
                    cx.notify();
                    return;
                }
                a.pg_do("provider", "PUT", "/v1/provider".into(), Some(save_body(&url, &name, &key)), &["provider"], "Saved. Restart the core (lhd) so it uses the new address and key.", cx);
                if had_url {
                    a.sp.models_keys.editing = false;
                }
            }
        });
        let remove_key = info.key_saved.then(|| {
            self.pg_confirm("prov-remove-key", "Remove the saved key", "Remove the saved key?", cx, |a, _, cx| {
                a.pg_do("provider", "PUT", "/v1/provider".into(), Some(json!({"key": ""})), &["provider"], "The key is removed. Restart the core (lhd) to stop using it.", cx);
            })
        });
        let cancel = (!info.url.is_empty()).then(|| {
            self.button("prov-cancel", "Cancel", Kind::Ghost, cx, |a, _, cx| {
                a.sp.models_keys.editing = false;
                a.sp.models_keys.result = None;
                cx.notify();
            })
        });
        let mut card = self.pg_card().id("provider-form").role(Role::Group).aria_label("Edit the model service").child(url_box).child(name_box).child(key_box);
        card = card.child(div().flex().flex_wrap().items_center().gap_1().child(save).child(test).children(ollama).children(cancel).children(remove_key));
        if let Some(r) = self.sp.models_keys.result.clone() {
            card = card.child(self.test_result_view(&r));
        }
        if self.sp.models_keys.testing {
            card = card.child(self.pg_small("Trying the address…"));
        }
        if let Some(e) = self.pg_error("provtest") {
            card = card.child(e);
        }
        card.into_any_element()
    }

    /// "Test connection": try an address with the key typed so far (or the saved one).
    fn test_button(&mut self, id: &str, url: String, key: String, cx: &mut Context<Self>) -> AnyElement {
        let label = if self.sp.models_keys.testing { "Testing…" } else { "Test connection" };
        self.button(id.to_string(), label, Kind::Normal, cx, move |a, _, cx| {
            // from the saved row the form's boxes may be empty: use the address on the row
            let (u, k) = if form_is_open(a) { (a.pg_val("prov.url", cx), a.pg_val("prov.key", cx)) } else { (url.clone(), key.clone()) };
            a.run_provider_test(u, k, cx);
        })
    }

    fn run_provider_test(&mut self, url: String, key: String, cx: &mut Context<Self>) {
        self.sp.models_keys.testing = true;
        self.sp.models_keys.result = None;
        self.sp.errors.remove("provtest");
        self.backend.send(Cmd::Call { tag: "pg|provtest".into(), method: "POST".into(), path: "/v1/provider/test".into(), body: Some(test_body(&url, &key)) });
        cx.notify();
    }

    fn test_result_view(&self, r: &TestResult) -> AnyElement {
        let p = &self.pal;
        let mut c = self.pg_card().id("provider-test-result").role(Role::Group).aria_label("Connection test");
        c = c.child(div().flex().items_center().gap_2().child(div().w(px(10.)).h(px(10.)).rounded_full().bg(if r.ok { p.good } else { p.danger })).child(div().flex_1().min_w_0().child(r.message.clone())));
        if r.ok && !r.models.is_empty() {
            c = c.child(self.pg_small(format!("Models: {}", r.model_sample(8))));
        }
        if !r.ok && r.ollama.is_some() {
            c = c.child(self.pg_small("This looks like Ollama, but its model list could not be read."));
        }
        c.into_any_element()
    }
}

/// Is the form open, so its boxes (not the row's address) are what to test?
fn form_is_open(a: &App) -> bool {
    a.sp.models_keys.editing || a.sp.data.get("provider").map(ProviderInfo::from_json).is_none_or(|i| i.url.is_empty())
}
