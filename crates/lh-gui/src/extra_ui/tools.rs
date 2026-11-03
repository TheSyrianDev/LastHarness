//! Tools and MCP: servers (add, approve, remove, their tools) and the folders that may bring their own.

use gpui::prelude::*;
use gpui::*;

use super::model::tools as m;
use crate::app::App;
use crate::ui::Kind;
use lh_gui::setting_names::Category;

impl App {
    pub(super) fn pg_tools(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let mut col = div().flex().flex_col().gap_2();
        if let Some(n) = self.pg_note_for("tools") {
            col = col.child(n);
        }
        col = col.child(self.pg_title("MCP servers")).child(self.pg_muted("An MCP server gives the assistant more tools. Nothing runs until you approve it, and you see the exact command or link first."));
        match self.pg_list("mcp") {
            None => col = col.child(self.pg_note_line("Loading…")),
            Some(Err(e)) => col = col.child(self.pg_error_line("The server list", &e)),
            Some(Ok(v)) => {
                let list = m::servers(&v);
                if list.is_empty() {
                    col = col.child(self.pg_note_line("No MCP servers yet."));
                }
                for s in list {
                    col = col.child(self.server_card(s, cx));
                }
            }
        }
        col = col.child(self.add_server_form(window, cx));

        col = col.child(div().pt_2().child(self.pg_title("Folders that bring their own tools")));
        col = col.child(self.pg_muted("A project folder can list servers of its own. They load only after you trust that folder. When you open a chat in such a folder you are asked; your answers are listed here."));
        match self.pg_list("trust") {
            None => {}
            Some(Err(e)) => col = col.child(self.pg_error_line("The folder list", &e)),
            Some(Ok(v)) => {
                let list = m::folders(&v);
                if list.is_empty() {
                    col = col.child(self.pg_note_line("No folder has a decision yet."));
                }
                for f in list {
                    col = col.child(self.folder_card(f, cx));
                }
            }
        }
        let _ = p;
        col.child(div().flex().child(self.pg_refresh(Category::Tools, cx))).into_any_element()
    }

    fn server_card(&mut self, s: m::Server, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let name = s.name.clone();
        let state = if s.approved { "Approved" } else { "Waiting for your approval" };
        let mut rows: Vec<AnyElement> = vec![
            div().flex().flex_wrap().items_center().gap_2().child(div().flex_1().child(format!("{}  ({})", s.name, if s.kind == "remote" { "a link" } else { "a program on this computer" }))).child(div().text_sm().text_color(if s.approved { p.muted } else { p.accent }).child(state)).into_any_element(),
            div().min_w_0().text_sm().text_color(p.text).child(format!("{}  {}", if s.kind == "remote" { "Link:" } else { "Runs:" }, s.runs)).into_any_element(),
        ];
        if let Some(e) = &s.error {
            rows.push(div().text_xs().text_color(p.danger).child(format!("Cannot start: {e}")).into_any_element());
        }
        if s.approved && s.error.is_none() {
            rows.push(self.pg_muted(if s.tools.is_empty() { "It offers no tools.".to_string() } else { format!("{} tools:", s.tools.len()) }));
            for t in &s.tools {
                rows.push(div().min_w_0().text_xs().text_color(p.muted).child(if t.description.is_empty() { t.name.clone() } else { format!("{}  {}", t.name, t.description) }).into_any_element());
            }
        }
        let mut buttons = div().flex().flex_wrap().gap_1();
        if !s.approved {
            let n = name.clone();
            buttons = buttons.child(self.button(format!("pg-mcp-approve-{name}"), "Approve and start", Kind::Primary, cx, move |a, _, cx| a.pg_act("tools", &format!("mcp-approve:{n}"), "POST", format!("/v1/mcp/servers/{n}/approve"), None, cx)));
        }
        let n = name.clone();
        buttons = buttons.child(self.pg_confirm_button(&format!("mcp-rm-{name}"), "Remove", "Yes, remove it", cx, move |a, _, cx| a.pg_act("tools", &format!("mcp-remove:{n}"), "DELETE", format!("/v1/mcp/servers/{n}"), None, cx)));
        rows.push(buttons.into_any_element());
        self.xp_card(rows).id(SharedString::from(format!("pg-mcp-{name}"))).role(Role::Group).aria_label(format!("MCP server {name}")).into_any_element()
    }

    fn add_server_form(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let open = self.pg.open.contains("mcp-form");
        if !open {
            return div().flex().child(self.button("pg-mcp-new", "Add a server", Kind::Normal, cx, |a, _, cx| {
                a.pg_toggle("mcp-form");
                cx.notify();
            })).into_any_element();
        }
        let link = self.pg.open.contains("mcp-link");
        let mode = div().flex().flex_wrap().gap_1()
            .child(self.button("pg-mcp-mode-cmd", "A program on this computer", if link { Kind::Normal } else { Kind::Primary }, cx, |a, _, cx| {
                a.pg.open.remove("mcp-link");
                cx.notify();
            }))
            .child(self.button("pg-mcp-mode-url", "A link", if link { Kind::Primary } else { Kind::Normal }, cx, |a, _, cx| {
                a.pg.open.insert("mcp-link".into());
                cx.notify();
            }));
        let name = self.xp_field("mcp.name", "Name", "files", window, cx);
        let (label, hint) = if link { ("Link", "https://mcp.example.org/mcp") } else { ("Command (the program and what follows it)", "npx -y @modelcontextprotocol/server-filesystem /home/me/docs") };
        let target = self.xp_field(if link { "mcp.url" } else { "mcp.cmd" }, label, hint, window, cx);
        let bearer = link.then(|| self.xp_field("mcp.bearer", "Name of the environment variable that holds the key (leave empty if none)", "MY_SERVER_KEY", window, cx));
        // what will run, shown exactly, before it is added
        let typed = self.pg_text(if link { "mcp.url" } else { "mcp.cmd" }, cx);
        let shown = m::preview(link, &typed);
        let will_run = (!shown.is_empty()).then(|| div().min_w_0().text_sm().text_color(self.pal.text).child(format!("It will {} {}", if link { "connect to" } else { "run:" }, shown)).into_any_element());
        let add = self.button("pg-mcp-add", "Add (it will not run yet)", Kind::Primary, cx, move |a, _, cx| {
            let (n, t, b) = (a.pg_text("mcp.name", cx), a.pg_text(if link { "mcp.url" } else { "mcp.cmd" }, cx), a.pg_text("mcp.bearer", cx));
            match m::add_body(&n, link, &t, &b) {
                Ok(body) => {
                    a.pg_act("tools", &format!("mcp-add:{}", n.trim()), "POST", "/v1/mcp/servers", Some(body), cx);
                    a.pg_clear(&["mcp.name", "mcp.url", "mcp.cmd", "mcp.bearer"]);
                    a.pg.open.remove("mcp-form");
                }
                Err(e) => {
                    a.pg_note_set("tools", true, e);
                    cx.notify();
                }
            }
        });
        let cancel = self.button("pg-mcp-cancel", "Cancel", Kind::Ghost, cx, |a, _, cx| {
            a.pg.open.remove("mcp-form");
            cx.notify();
        });
        self.xp_card(vec![self.pg_title("Add an MCP server"), mode.into_any_element(), name, target]
            .into_iter()
            .chain(bearer)
            .chain(will_run)
            .chain([div().flex().gap_1().child(add).child(cancel).into_any_element()])
            .collect())
            .into_any_element()
    }

    fn folder_card(&mut self, f: m::Folder, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let mut rows: Vec<AnyElement> = vec![
            div().min_w_0().child(f.folder.clone()).into_any_element(),
            div().text_sm().text_color(if f.decision == "changed" { p.accent } else { p.muted }).child(m::decision_words(&f.decision)).into_any_element(),
        ];
        if !f.exists {
            rows.push(self.pg_muted("This folder is not there any more."));
        }
        for (name, runs) in &f.asks {
            rows.push(div().min_w_0().text_xs().text_color(p.muted).child(format!("It lists {name}: {runs}")).into_any_element());
        }
        let folder = f.folder.clone();
        let id = format!("trust-{}", f.folder);
        rows.push(div().flex().child(self.pg_confirm_button(&id, "Revoke", "Yes, forget this folder", cx, move |a, _, cx| {
            a.pg_act("tools", "trust-forget:", "POST", "/v1/trust", Some(serde_json::json!({"folder": folder, "decision": "forget"})), cx)
        })).into_any_element());
        self.xp_card(rows).into_any_element()
    }
}
