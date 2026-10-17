//! Activity: what the assistant did (newest first, with Undo for file changes) and what it cost.

use gpui::prelude::*;
use gpui::*;

use super::model::activity as m;
use crate::app::App;
use crate::ui::Kind;
use lh_gui::setting_names::Category;

impl App {
    pub(super) fn pg_activity(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let mut col = div().flex().flex_col().gap_2();
        if let Some(n) = self.pg_note_for("activity") {
            col = col.child(n);
        }
        col = col.child(self.spend_card());

        col = col.child(self.pg_title("What was done"));
        col = col.child(self.pg_muted("Every tool the assistant used, every permission you answered, and every request that left this computer. It is only ever added to. Undo is offered for file changes that are still as the assistant left them."));
        let mut filter = div().flex().flex_wrap().gap_1();
        for (kind, label) in m::KINDS {
            let chosen = self.pg.activity_kind == *kind;
            let k = (*kind).to_string();
            filter = filter.child(self.button(format!("pg-act-kind-{kind}"), *label, if chosen { Kind::Primary } else { Kind::Ghost }, cx, move |a, _, cx| {
                a.pg.activity_kind = k.clone();
                a.pg_load(Category::Activity);
                cx.notify();
            }));
        }
        col = col.child(filter);
        match self.pg_list("activity") {
            None => col = col.child(self.pg_note_line("Loading…")),
            Some(Err(e)) => col = col.child(self.pg_error_line("The activity log", &e)),
            Some(Ok(v)) => {
                let list = m::entries(&v);
                if list.is_empty() {
                    col = col.child(self.pg_note_line("Nothing here yet."));
                }
                for e in list {
                    let seq = e.seq;
                    let undo = e.undoable.then(|| {
                        self.button(format!("pg-undo-{seq}"), "Undo", Kind::Normal, cx, move |a, _, cx| a.pg_act("activity", "undo:", "POST", format!("/v1/activity/{seq}/undo"), None, cx))
                    });
                    col = col.child(
                        div().flex().flex_wrap().items_center().gap_2().p_2().rounded(p.inner).bg(p.sunken).text_color(p.text).min_w_0()
                            .id(SharedString::from(format!("pg-entry-{seq}"))).role(Role::ListItem)
                            .child(div().flex().flex_col().flex_1().min_w(px(200.)).child(div().min_w_0().text_sm().child(if e.summary.is_empty() { m::kind_label(&e.kind) } else { e.summary.clone() })).child(div().text_xs().text_color(p.muted).child(format!("{} · {} · {}", e.when, m::kind_label(&e.kind), e.actor))))
                            .children(undo),
                    );
                }
            }
        }
        col.child(div().flex().child(self.pg_refresh(Category::Activity, cx))).into_any_element()
    }

    fn spend_card(&self) -> AnyElement {
        let mut rows = vec![self.pg_title("Spending")];
        match self.pg_list("spend") {
            None => rows.push(self.pg_note_line("Loading…")),
            Some(Err(e)) => rows.push(self.pg_error_line("The spending summary", &e)),
            Some(Ok(v)) => {
                let s = m::spend(&v);
                rows.push(self.pg_muted("Counted from the ledger of model calls, by UTC day and month. Models on this computer cost nothing."));
                rows.push(div().text_sm().child(format!("Today: {} ({})", s.today, m::calls_words(s.today_calls))).into_any_element());
                rows.push(div().text_sm().child(format!("This month: {} ({})", s.month, m::calls_words(s.month_calls))).into_any_element());
                if s.unpriced > 0 {
                    rows.push(self.pg_muted(format!("{} this month had no known price, so their cost is not counted. Add a price in the spend settings.", m::calls_words(s.unpriced))));
                }
                if s.providers.is_empty() {
                    rows.push(self.pg_muted("Nothing recorded this month."));
                } else {
                    rows.push(self.pg_muted("This month, by service:"));
                    for (name, amount, calls) in s.providers {
                        rows.push(div().text_sm().child(format!("{name}: {amount} ({})", m::calls_words(calls))).into_any_element());
                    }
                }
            }
        }
        self.xp_card(rows).into_any_element()
    }
}
