//! Your data: export, import with a summary first, and starting with the computer.

use gpui::prelude::*;
use gpui::*;
use gpui_component::input::Input;

use super::model::{data as m, service};
use crate::app::App;
use crate::ui::Kind;

/// Runs a closure that may take a moment off the window's thread, then hands its result to the window.
fn background<T: Send + 'static>(cx: &mut Context<App>, work: impl FnOnce() -> T + Send + 'static, done: impl FnOnce(&mut App, T, &mut Context<App>) + 'static) {
    cx.spawn(async move |this: WeakEntity<App>, cx: &mut AsyncApp| {
        let r = cx.background_executor().spawn(async move { work() }).await;
        let _ = this.update(cx, |a, cx| done(a, r, cx));
    })
    .detach();
}

impl App {
    pub(super) fn pg_data(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let mut col = div().flex().flex_col().gap_2();
        if let Some(n) = self.pg_note_for("data") {
            col = col.child(n);
        }
        if self.connected {
            col = col.child(self.export_card(window, cx)).child(self.import_card(window, cx));
        } else {
            col = col.child(self.pg_note_line("The core is not running, so export and import are not available."));
        }
        col.child(self.service_card(cx)).into_any_element()
    }

    fn export_card(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let input = self.xp_input("data.export", "/path/to/a/new/folder", window, cx);
        let choose = self.button("pg-export-choose", "Choose folder", Kind::Normal, cx, |a, _, cx| a.pg_pick_folder("data.export", "LastHarness-export", cx));
        let go = self.button("pg-export-go", "Save a copy", Kind::Primary, cx, |a, _, cx| {
            let dir = a.pg_text("data.export", cx);
            if dir.trim().is_empty() {
                a.pg_note_set("data", true, "Choose a folder for the copy first.");
                cx.notify();
            } else {
                a.pg_act("data", "export:", "POST", "/v1/export", Some(serde_json::json!({"dir": dir.trim()})), cx);
            }
        });
        self.xp_card(vec![
            self.pg_title("Save a copy of your data"),
            self.pg_muted("Writes your memory, rules, writing style, roles and departments to a new folder. The folder must be new or empty. The copy is not encrypted, so keep it somewhere private."),
            div().w_full().flex().items_center().gap_1().child(div().flex_1().min_w_0().child(Input::new(&input).aria_label("Folder for the copy"))).child(choose).into_any_element(),
            div().flex().child(go).into_any_element(),
        ])
        .into_any_element()
    }

    fn import_card(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let input = self.xp_input("data.import", "/path/to/a/saved/copy", window, cx);
        let choose = self.button("pg-import-choose", "Choose folder", Kind::Normal, cx, |a, _, cx| a.pg_pick_folder("data.import", "", cx));
        let look = self.button("pg-import-look", "Look inside", Kind::Normal, cx, |a, _, cx| {
            let dir = a.pg_text("data.import", cx);
            if dir.trim().is_empty() {
                a.pg_note_set("data", true, "Choose the folder that holds a saved copy first.");
                cx.notify();
            } else {
                a.pg.import = None;
                a.pg_act("data", "import-look:", "POST", "/v1/import", Some(m::import_body(&dir, false, &[], false)), cx);
            }
        });
        let mut rows = vec![
            self.pg_title("Bring in a saved copy"),
            self.pg_muted("First you see what is in the folder. Nothing changes until you press Bring it in. Your own roles, departments and writing style stay unless you choose to replace them."),
            div().w_full().flex().items_center().gap_1().child(div().flex_1().min_w_0().child(Input::new(&input).aria_label("Folder with a saved copy"))).child(choose).into_any_element(),
            div().flex().child(look).into_any_element(),
        ];
        if let Some(pv) = self.pg.import.clone() {
            let p = self.pal.clone();
            let mut sum = div().flex().flex_col().gap_1().p_2().rounded(p.inner).bg(p.button).child(div().text_sm().child("In that folder:"));
            for line in m::summary(&pv) {
                sum = sum.child(div().min_w_0().text_sm().child(line));
            }
            rows.push(sum.into_any_element());
            let clashes = m::clashes(&pv);
            if !clashes.is_empty() || pv.style {
                rows.push(self.pg_muted("You already have some of these. Pick any you want the copy to replace:"));
            }
            let mut picks = div().flex().flex_wrap().gap_1();
            for name in clashes {
                let on = self.pg.replace.contains(&name);
                let n = name.clone();
                picks = picks.child(self.button(format!("pg-replace-{name}"), format!("{} {name}", if on { "Replace" } else { "Keep mine:" }), if on { Kind::Primary } else { Kind::Normal }, cx, move |a, _, cx| {
                    if !a.pg.replace.remove(&n) {
                        a.pg.replace.insert(n.clone());
                    }
                    cx.notify();
                }));
            }
            if pv.style {
                let on = self.pg.replace_style;
                picks = picks.child(self.button("pg-replace-style", if on { "Replace my writing style" } else { "Keep my writing style" }, if on { Kind::Primary } else { Kind::Normal }, cx, |a, _, cx| {
                    a.pg.replace_style = !a.pg.replace_style;
                    cx.notify();
                }));
            }
            rows.push(picks.into_any_element());
            if m::has_content(&pv) {
                rows.push(div().flex().child(self.button("pg-import-apply", "Bring it in", Kind::Primary, cx, |a, _, cx| {
                    let dir = a.pg_text("data.import", cx);
                    let mut replace: Vec<String> = a.pg.replace.iter().cloned().collect();
                    replace.sort();
                    a.pg_act("data", "import-apply:", "POST", "/v1/import", Some(m::import_body(&dir, true, &replace, a.pg.replace_style)), cx);
                })).into_any_element());
            }
        }
        self.xp_card(rows).into_any_element()
    }

    fn service_card(&mut self, cx: &mut Context<Self>) -> AnyElement {
        if self.pg.service.is_none() && !self.pg.open.contains("svc-asked") {
            self.pg.open.insert("svc-asked".into());
            self.pg_service_check(cx);
        }
        let status = self.pg.service.clone();
        let mut rows = vec![
            self.pg_title("Start with the computer"),
            self.pg_muted("Starts the core when you log in, so the app, your browser and your phone can reach it without a terminal. It writes one file in your own folder and asks your own user's systemd to run it. Nothing needs root."),
            self.pg_muted(status.as_ref().map(service::status_words).unwrap_or_else(|| "Checking…".into())),
        ];
        if let Some(text) = self.pg.service_preview.clone() {
            rows.push(div().min_w_0().p_2().rounded(self.pal.inner).bg(self.pal.button).text_xs().child(text).into_any_element());
        }
        let mut buttons = div().flex().flex_wrap().gap_1();
        let installed = status.as_ref().is_some_and(|s| s.installed);
        if self.pg.service_preview.is_some() {
            buttons = buttons
                .child(self.button("pg-svc-go", "Go ahead", Kind::Primary, cx, |a, _, cx| {
                    let Some(lhd) = service::find_lhd() else { return };
                    a.pg.service_preview = None;
                    background(cx, move || service::install(&lhd, &service::unit_path(), &mut service::real_systemctl), |a, r: Result<String, String>, cx| {
                        match r {
                            Ok(t) => a.pg_note_set("data", false, t),
                            Err(e) => a.pg_note_set("data", true, e),
                        }
                        a.pg_service_check(cx);
                        cx.notify();
                    });
                    cx.notify();
                }))
                .child(self.button("pg-svc-cancel", "Cancel", Kind::Ghost, cx, |a, _, cx| {
                    a.pg.service_preview = None;
                    cx.notify();
                }));
        } else if installed {
            buttons = buttons.child(self.button("pg-svc-remove", "Stop starting with the computer", Kind::Normal, cx, |a, _, cx| {
                background(cx, || service::remove(&service::unit_path(), &mut service::real_systemctl), |a, r: Result<String, String>, cx| {
                    match r {
                        Ok(t) => a.pg_note_set("data", false, t),
                        Err(e) => a.pg_note_set("data", true, e),
                    }
                    a.pg_service_check(cx);
                    cx.notify();
                });
                a.pg.notes.remove("data");
                cx.notify();
            }));
        } else {
            buttons = buttons.child(self.button("pg-svc-install", "Start with the computer…", Kind::Normal, cx, |a, _, cx| {
                match service::find_lhd() {
                    Some(lhd) => a.pg.service_preview = Some(service::preview(&lhd, &service::unit_path())),
                    None => a.pg_note_set("data", true, "Cannot find the core program `lhd` next to this app or on the path. Build it with: cargo build --release"),
                }
                cx.notify();
            }));
        }
        rows.push(buttons.into_any_element());
        self.xp_card(rows).into_any_element()
    }

    fn pg_service_check(&mut self, cx: &mut Context<Self>) {
        background(cx, || service::status(&service::unit_path(), &mut service::real_systemctl), |a, s, cx| {
            a.pg.service = Some(s);
            cx.notify();
        });
    }
}
