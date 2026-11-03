//! Hooks and Scheduled tasks. Hooks are read from and written to `hooks.toml` directly; scheduled tasks go through
//! the core's `/v1/schedules`.

use gpui::prelude::*;
use gpui::*;

use super::model::{hooks as h, schedules as sm};
use crate::app::App;
use crate::ui::Kind;
use lh_gui::setting_names::Category;

impl App {
    pub(super) fn pg_hooks(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let path = h::path();
        if self.pg.hooks.is_none() {
            self.pg.hooks = Some(h::load(&path));
        }
        let mut col = div().flex().flex_col().gap_2();
        if let Some(n) = self.pg_note_for("hooks") {
            col = col.child(n);
        }
        col = col.child(self.pg_muted(format!("Commands run before or after a tool call, kept in {}. A command before a tool that fails stops the tool. New chats use the changes.", path.display())));
        let hooks = match self.pg.hooks.clone() {
            Some(Ok(x)) => x,
            Some(Err(e)) => return col.child(self.pg_error_line("hooks.toml", &e)).into_any_element(),
            None => h::Hooks::default(),
        };
        for (after, list) in [(false, &hooks.before), (true, &hooks.after)] {
            col = col.child(self.pg_title(if after { "After a tool" } else { "Before a tool" }));
            if list.is_empty() {
                col = col.child(self.pg_note_line("None."));
            }
            for (i, k) in list.iter().enumerate() {
                let tools = if k.tools.is_empty() { "every tool".to_string() } else { k.tools.join(", ") };
                let remove = self.button(format!("pg-hook-rm-{after}-{i}"), "Remove", Kind::Ghost, cx, move |a, _, cx| {
                    if let Some(Ok(mut x)) = a.pg.hooks.clone() {
                        let l = if after { &mut x.after } else { &mut x.before };
                        if i < l.len() {
                            l.remove(i);
                        }
                        a.hooks_save(x);
                    }
                    cx.notify();
                });
                col = col.child(self.xp_card(vec![
                    div().text_sm().font_family(self.pal.mono.clone()).child(k.run.clone()).into_any_element(),
                    self.pg_muted(format!("{tools} · stops after {} s", k.timeout.as_secs())),
                    div().flex().child(remove).into_any_element(),
                ]));
            }
        }
        col = col.child(self.pg_title("Add a hook"));
        col = col.child(self.xp_field("hook-run", "Command (run by sh in the chat's folder)", "cargo fmt", window, cx));
        col = col.child(self.xp_field("hook-tools", "Tools, separated by commas (empty for every tool)", "write_file", window, cx));
        col = col.child(self.xp_field("hook-secs", "Time limit in seconds", "30", window, cx));
        let add = |after: bool| move |a: &mut App, _: &mut Window, cx: &mut Context<App>| {
            let (tools, run, secs) = (a.pg_text("hook-tools", cx), a.pg_text("hook-run", cx), a.pg_text("hook-secs", cx));
            match (h::from_form(&tools, &run, &secs), a.pg.hooks.clone()) {
                (Ok(k), Some(Ok(mut x))) => {
                    if after { x.after.push(k) } else { x.before.push(k) }
                    if a.hooks_save(x) {
                        a.pg_clear(&["hook-tools", "hook-run", "hook-secs"]);
                    }
                }
                (Err(e), _) => a.pg_note_set("hooks", true, e),
                _ => {}
            }
            cx.notify();
        };
        col = col.child(div().flex().gap_1()
            .child(self.button("pg-hook-add-before", "Add before a tool", Kind::Normal, cx, add(false)))
            .child(self.button("pg-hook-add-after", "Add after a tool", Kind::Normal, cx, add(true))));
        col.into_any_element()
    }

    /// Write the hooks and say what happened. True when the file was written.
    fn hooks_save(&mut self, x: h::Hooks) -> bool {
        match h::save(&h::path(), &x) {
            Ok(()) => {
                self.pg.hooks = Some(Ok(x));
                self.pg_note_set("hooks", false, "Saved. New chats use these hooks.");
                true
            }
            Err(e) => {
                self.pg_note_set("hooks", true, e);
                false
            }
        }
    }

    pub(super) fn pg_schedules(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let mut col = div().flex().flex_col().gap_2();
        if let Some(n) = self.pg_note_for("schedules") {
            col = col.child(n);
        }
        col = col.child(self.pg_muted("Tasks that start a chat on their own at set times. Nobody is there to answer, so every permission question is answered no."));
        match self.pg_list("schedules") {
            None => col = col.child(self.pg_note_line("Loading…")),
            Some(Err(e)) => col = col.child(self.pg_error_line("The task list", &e)),
            Some(Ok(v)) => {
                let list = sm::tasks(&v);
                if list.is_empty() {
                    col = col.child(self.pg_note_line("No tasks yet."));
                }
                for t in list {
                    let id = t.id;
                    let remove = self.pg_confirm_button(&format!("sch-rm-{id}"), "Delete", "Delete this task", cx, move |a, _, cx| a.pg_act("schedules", "sch-remove:", "DELETE", format!("/v1/schedules/{id}"), None, cx));
                    let mut rows = vec![
                        div().text_sm().child(t.prompt.clone()).into_any_element(),
                        self.pg_muted(format!("{} · {}{}", t.when, t.model, if t.enabled { "" } else { " · off" })),
                    ];
                    if !t.last_result.is_empty() {
                        rows.push(self.pg_muted(format!("Last time: {}", t.last_result)));
                    }
                    rows.push(div().flex().child(remove).into_any_element());
                    col = col.child(self.xp_card(rows));
                }
            }
        }
        // what scheduled tasks changed on their own, newest first
        if let Some(Ok(v)) = self.pg_list("sch-files") {
            let changed = super::model::activity::entries(&v);
            if !changed.is_empty() {
                col = col.child(self.pg_title("Files changed by tasks"));
                for e in changed {
                    col = col.child(self.pg_muted(format!("{} · {}", e.when, e.summary)));
                }
            }
        }
        col = col.child(self.pg_title("Add a task"));
        col = col.child(self.xp_field("sch-when", "When: every 30m, every 2h, every 1d or daily 08:00", "every 2h", window, cx));
        col = col.child(self.xp_field("sch-task", "What to do", "Summarise new files in my notes folder", window, cx));
        col = col.child(self.xp_field("sch-model", "Model", "Qwen3.6-35B-A3B-balanced", window, cx));
        col = col.child(self.xp_field("sch-folder", "Folder (empty for your home folder)", "", window, cx));
        let add = self.button("pg-sch-add", "Add the task", Kind::Primary, cx, |a, _, cx| {
            let (w, t, m, f) = (a.pg_text("sch-when", cx), a.pg_text("sch-task", cx), a.pg_text("sch-model", cx), a.pg_text("sch-folder", cx));
            match sm::new_task(&w, &t, &m, &f) {
                Ok(body) => {
                    a.pg_act("schedules", "sch-add:", "POST", "/v1/schedules", Some(body), cx);
                    a.pg_clear(&["sch-when", "sch-task", "sch-folder"]);
                }
                Err(e) => a.pg_note_set("schedules", true, e),
            }
            cx.notify();
        });
        col.child(div().flex().gap_1().child(add).child(self.pg_refresh(Category::Schedules, cx))).into_any_element()
    }
}
