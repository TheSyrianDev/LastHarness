//! Channels: Telegram, Matrix, Signal and Slack. Add, edit, switch on and off. A channel is off until its
//! warning is read and acknowledged.

use gpui::prelude::*;
use gpui::*;
use serde_json::json;

use super::model::channels as m;
use crate::app::App;
use crate::ui::Kind;
use lh_gui::setting_names::Category;

impl App {
    pub(super) fn pg_channels(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let mut col = div().flex().flex_col().gap_2();
        if let Some(n) = self.pg_note_for("channels") {
            col = col.child(n);
        }
        col = col.child(self.pg_muted("Messages on a channel pass through that service's servers. Every channel is off until you have read its warning and switched it on, and only the people you list are answered."));
        let (kinds, list) = match self.pg_list("channels") {
            None => return col.child(self.pg_note_line("Loading…")).into_any_element(),
            Some(Err(e)) => return col.child(self.pg_error_line("The channel list", &e)).into_any_element(),
            Some(Ok(v)) => (m::kinds(&v), m::channels(&v)),
        };
        if list.is_empty() {
            col = col.child(self.pg_note_line("No channels yet."));
        }
        for c in list {
            col = col.child(self.channel_card(c, &kinds, cx));
        }
        col = col.child(self.channel_form(&kinds, window, cx));
        col.child(div().flex().child(self.pg_refresh(Category::Channels, cx))).into_any_element()
    }

    fn channel_card(&mut self, c: m::Channel, kinds: &[m::Kind], cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let name = c.name.clone();
        let mut rows: Vec<AnyElement> = vec![
            div().flex().flex_wrap().items_center().gap_2().child(div().flex_1().child(format!("{}  ({})", c.name, c.kind))).child(div().text_sm().text_color(if c.enabled { p.accent } else { p.muted }).child(if c.enabled { "On" } else { "Off" })).into_any_element(),
            self.pg_muted(m::state_words(&c)),
            self.pg_muted(format!("Answers: {}. Model: {}. May talk to it: {}.", if c.mode == "assistant" { "with tools, asking in the chat" } else { "chat only" }, if c.model.is_empty() { "none chosen" } else { &c.model }, if c.allow.is_empty() { "nobody yet".to_string() } else { c.allow.join(", ") })),
        ];
        let warning = kinds.iter().find(|k| k.kind == c.kind).map(|k| k.warning.clone()).unwrap_or_default();
        let asking = self.pg.confirm.as_deref() == Some(format!("ch-on-{name}").as_str());
        if asking {
            rows.push(div().min_w_0().p_2().rounded(p.inner).border_1().border_color(p.danger.opacity(0.55)).text_sm().child(warning).into_any_element());
        }
        let mut buttons = div().flex().flex_wrap().gap_1();
        if c.enabled {
            let n = name.clone();
            buttons = buttons.child(self.button(format!("pg-ch-off-{name}"), "Switch off", Kind::Normal, cx, move |a, _, cx| a.pg_act("channels", &format!("ch-disable:{n}"), "POST", format!("/v1/channels/{n}/disable"), None, cx)));
        } else if asking {
            let n = name.clone();
            buttons = buttons
                .child(self.button(format!("pg-ch-ack-{name}"), "I have read this. Switch it on", Kind::Danger, cx, move |a, _, cx| a.pg_act("channels", &format!("ch-enable:{n}"), "POST", format!("/v1/channels/{n}/enable"), Some(json!({"acknowledge": true})), cx)))
                .child(self.button(format!("pg-ch-nope-{name}"), "Leave it off", Kind::Ghost, cx, |a, _, cx| {
                    a.pg.confirm = None;
                    cx.notify();
                }));
        } else {
            let (n, missing) = (name.clone(), m::missing_for_enable(&c));
            buttons = buttons.child(self.button(format!("pg-ch-on-{name}"), "Switch on…", Kind::Normal, cx, move |a, _, cx| match missing {
                Some(why) => {
                    a.pg_note_set("channels", true, why);
                    cx.notify();
                }
                None => {
                    a.pg.notes.remove("channels");
                    a.pg.confirm = Some(format!("ch-on-{n}"));
                    cx.notify();
                }
            }));
        }
        let edit = c.clone();
        buttons = buttons.child(self.button(format!("pg-ch-edit-{name}"), "Edit", Kind::Ghost, cx, move |a, _, cx| {
            a.pg.editing = Some(edit.name.clone());
            a.pg.channel_kind = edit.kind.clone();
            a.pg.channel_assistant = edit.mode == "assistant";
            a.pg.open.insert("ch-form".into());
            a.pg.set_next.insert("ch.name".into(), edit.name.clone());
            a.pg.set_next.insert("ch.allow".into(), edit.allow.join(", "));
            a.pg.set_next.insert("ch.model".into(), edit.model.clone());
            for (k, v) in &edit.settings {
                a.pg.set_next.insert(format!("ch.set.{k}"), v.clone());
            }
            cx.notify();
        }));
        let n = name.clone();
        buttons = buttons.child(self.pg_confirm_button(&format!("ch-rm-{name}"), "Remove", "Yes, remove it", cx, move |a, _, cx| a.pg_act("channels", &format!("ch-remove:{n}"), "DELETE", format!("/v1/channels/{n}"), None, cx)));
        rows.push(buttons.into_any_element());
        self.xp_card(rows).id(SharedString::from(format!("pg-ch-{name}"))).role(Role::Group).aria_label(format!("Channel {name}")).into_any_element()
    }

    fn channel_form(&mut self, kinds: &[m::Kind], window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        if !self.pg.open.contains("ch-form") {
            return div().flex().child(self.button("pg-ch-new", "Add a channel", Kind::Normal, cx, |a, _, cx| {
                a.pg.editing = None;
                a.pg.channel_kind = String::new();
                a.pg.channel_assistant = false;
                a.pg.open.insert("ch-form".into());
                cx.notify();
            })).into_any_element();
        }
        let editing = self.pg.editing.clone();
        if self.pg.channel_kind.is_empty() {
            self.pg.channel_kind = kinds.first().map(|k| k.kind.clone()).unwrap_or_default();
        }
        let chosen = self.pg.channel_kind.clone();
        let Some(kind) = kinds.iter().find(|k| k.kind == chosen).cloned() else {
            return self.pg_note_line("The core did not list any kinds of channel.");
        };
        let mut kind_row = div().flex().flex_wrap().gap_1();
        for k in kinds {
            let kk = k.kind.clone();
            kind_row = kind_row.child(self.button(format!("pg-ch-kind-{}", k.kind), k.kind.clone(), if k.kind == chosen { Kind::Primary } else { Kind::Normal }, cx, move |a, _, cx| {
                if a.pg.editing.is_none() {
                    a.pg.channel_kind = kk.clone();
                    cx.notify();
                }
            }));
        }
        let name = self.xp_field("ch.name", "Name", "family-chat", window, cx);
        let allow = self.xp_field("ch.allow", "Who may talk to it (ids, separated by commas)", "123456789, 987654321", window, cx);
        let model = self.xp_field("ch.model", "Model that answers", "a model name from your list", window, cx);
        let assistant = self.pg.channel_assistant;
        let mode = div().flex().flex_wrap().gap_1()
            .child(self.button("pg-ch-mode-chat", "Chat only", if assistant { Kind::Normal } else { Kind::Primary }, cx, |a, _, cx| {
                a.pg.channel_assistant = false;
                cx.notify();
            }))
            .child(self.button("pg-ch-mode-asst", "With tools, asking in the chat", if assistant { Kind::Primary } else { Kind::Normal }, cx, |a, _, cx| {
                a.pg.channel_assistant = true;
                cx.notify();
            }));
        let mut fields: Vec<AnyElement> = vec![];
        for f in &kind.fields {
            let label = format!("{}{}: {}", f.name, if f.required { "" } else { " (optional)" }, f.help);
            fields.push(self.xp_field(&format!("ch.set.{}", f.name), &label, if f.name.contains("token") { "env:NAME" } else { "" }, window, cx));
        }
        let kind_for_save = kind.clone();
        let save = self.button("pg-ch-save", if editing.is_some() { "Save" } else { "Add (it starts off)" }, Kind::Primary, cx, move |a, _, cx| {
            let values: Vec<(String, String)> = kind_for_save.fields.iter().map(|f| (f.name.clone(), a.pg_text(&format!("ch.set.{}", f.name), cx))).collect();
            let (name, allow, model) = (a.pg_text("ch.name", cx), a.pg_text("ch.allow", cx), a.pg_text("ch.model", cx));
            match m::form_body(&name, &kind_for_save, &allow, &model, a.pg.channel_assistant, &values) {
                Ok(mut body) => {
                    let n = a.pg.editing.clone().unwrap_or_else(|| name.trim().to_string());
                    if a.pg.editing.is_some() {
                        // the kind and name stay; the core replaces the settings it is sent
                        body.as_object_mut().map(|o| (o.remove("name"), o.remove("kind")));
                        a.pg_act("channels", &format!("ch-edit:{n}"), "PUT", format!("/v1/channels/{n}"), Some(body), cx);
                    } else {
                        a.pg_act("channels", &format!("ch-add:{n}"), "POST", "/v1/channels", Some(body), cx);
                    }
                    a.pg_clear(&["ch.name", "ch.allow", "ch.model"]);
                    for f in &kind_for_save.fields {
                        a.pg_clear(&[format!("ch.set.{}", f.name).as_str()]);
                    }
                    a.pg.open.remove("ch-form");
                    a.pg.editing = None;
                }
                Err(e) => {
                    a.pg_note_set("channels", true, e);
                    cx.notify();
                }
            }
        });
        let cancel = self.button("pg-ch-cancel", "Cancel", Kind::Ghost, cx, |a, _, cx| {
            a.pg.open.remove("ch-form");
            a.pg.editing = None;
            cx.notify();
        });
        let mut rows = vec![self.pg_title(if let Some(n) = &editing { format!("Edit {n}") } else { "Add a channel".to_string() })];
        if editing.is_none() {
            rows.push(kind_row.into_any_element());
        }
        rows.extend([name, allow, model, mode.into_any_element()]);
        rows.extend(fields);
        rows.push(self.pg_muted("Keys are never typed here. Write where the key is kept, as env:NAME (an environment variable) or secret:NAME."));
        rows.push(div().flex().gap_1().child(save).child(cancel).into_any_element());
        self.xp_card(rows).into_any_element()
    }
}
