//! Drawing for the extra Settings pages: Tools and MCP, Channels, the devices block of Web and phone, Your data and
//! Activity. What each page reads and sends is in `lh_gui::pages`; this file holds the shared state and the small
//! building blocks, and each page has its own file.

mod activity;
mod channels;
mod data;
mod hooks;
mod tools;
mod web;

use std::collections::{HashMap, HashSet};

use gpui::prelude::*;
use gpui::*;
use gpui_component::input::{Input, InputState};
use serde_json::Value;

use crate::app::App;
use crate::ui::Kind;
use lh_gui::api::Cmd;
use lh_gui::pages as model;
use lh_gui::setting_names::Category;

/// Everything the extra pages remember between redraws.
#[derive(Default)]
pub struct PgState {
    /// What each list last answered, by name (`mcp`, `trust`, `channels`, `devices`, `pending`, `activity`, `spend`).
    pub data: HashMap<String, Result<Value, String>>,
    /// The page that was last drawn, so a page asks for its lists when it is opened.
    pub last_page: Option<Category>,
    /// One sentence per page about what just happened: true when it is a problem.
    pub notes: HashMap<String, (bool, String)>,
    pub inputs: HashMap<String, Entity<InputState>>,
    /// Text to put into a box the next time it is drawn (a folder that was picked, a channel being edited).
    pub set_next: HashMap<String, String>,
    /// Forms and sections that are open.
    pub open: HashSet<String>,
    /// A step that waits for a second click (removing, switching on), by name.
    pub confirm: Option<String>,
    pub activity_kind: String,
    pub channel_kind: String,
    pub channel_assistant: bool,
    pub editing: Option<String>,
    pub import: Option<model::data::Preview>,
    pub replace: HashSet<String>,
    pub replace_style: bool,
    pub service: Option<model::service::Status>,
    /// The file the service step would write, shown before the person agrees.
    pub service_preview: Option<String>,
    /// hooks.toml as last read or written.
    pub hooks: Option<Result<model::hooks::Hooks, String>>,
}

impl App {
    fn pg_fetch(&mut self, key: &str, path: impl Into<String>) {
        self.backend.send(Cmd::Call { tag: format!("pg:{key}"), method: "GET".into(), path: path.into(), body: None });
    }

    /// Ask the core for something, and tell the page what happened. `what` is `kind:argument`.
    fn pg_act(&mut self, page: &str, what: &str, method: &str, path: impl Into<String>, body: Option<Value>, cx: &mut Context<Self>) {
        self.pg.notes.remove(page);
        self.pg.confirm = None;
        self.backend.send(Cmd::Call { tag: format!("pg-act:{page}:{what}"), method: method.into(), path: path.into(), body });
        cx.notify();
    }

    fn pg_note_set(&mut self, page: &str, problem: bool, text: impl Into<String>) {
        self.pg.notes.insert(page.to_string(), (problem, text.into()));
    }

    /// Read the lists a page shows.
    pub fn pg_load(&mut self, page: Category) {
        match page {
            Category::Tools => {
                self.pg_fetch("mcp", "/v1/mcp/servers");
                self.pg_fetch("trust", "/v1/trust");
            }
            Category::Channels => self.pg_fetch("channels", "/v1/channels"),
            Category::Schedules => {
                self.pg_fetch("schedules", "/v1/schedules");
                self.pg_fetch("sch-files", model::activity::path("schedule_changed_files", 5));
            }
            Category::Hooks => self.pg.hooks = None,
            Category::Web => {
                self.pg_fetch("devices", "/v1/devices");
                self.pg_fetch("pending", "/v1/devices/pending");
                self.pg_fetch("phone", "/v1/devices/phone");
            }
            Category::Activity => {
                let path = model::activity::path(&self.pg.activity_kind, 100);
                self.pg_fetch("activity", path);
                self.pg_fetch("spend", "/v1/spend");
            }
            _ => {}
        }
    }

    /// The body of a page this file draws, or `None` for a page the settings list draws.
    pub fn pg_page(&mut self, page: Category, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !matches!(page, Category::Tools | Category::Channels | Category::Data | Category::Activity | Category::Hooks | Category::Schedules) {
            return None;
        }
        self.pg_opened(page);
        // the Data page works without the core for its last part, so it decides for itself
        if !self.connected && !matches!(page, Category::Data | Category::Hooks) {
            return Some(self.pg_note_line("The core is not running, so this page cannot be shown."));
        }
        Some(match page {
            Category::Tools => self.pg_tools(window, cx),
            Category::Channels => self.pg_channels(window, cx),
            Category::Data => self.pg_data(window, cx),
            Category::Hooks => self.pg_hooks(window, cx),
            Category::Schedules => self.pg_schedules(window, cx),
            _ => self.pg_activity(window, cx),
        })
    }

    /// The devices block under the web settings.
    pub fn pg_devices(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        self.pg_opened(Category::Web);
        self.pg_web(window, cx)
    }

    fn pg_opened(&mut self, page: Category) {
        if self.pg.last_page != Some(page) {
            self.pg.last_page = Some(page);
            if self.connected {
                self.pg_load(page);
            }
        }
    }

    pub fn on_pg_reply(&mut self, tag: &str, result: Result<Value, String>) {
        if let Some(key) = tag.strip_prefix("pg:") {
            self.pg.data.insert(key.to_string(), result);
            return;
        }
        let mut parts = tag.trim_start_matches("pg-act:").splitn(3, ':');
        let (page, kind, arg) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""), parts.next().unwrap_or("").to_string());
        let page_cat = match page {
            "tools" => Category::Tools,
            "channels" => Category::Channels,
            "web" => Category::Web,
            "data" => Category::Data,
            "schedules" => Category::Schedules,
            _ => Category::Activity,
        };
        match result {
            Err(e) => {
                let e = e.trim().to_string();
                self.pg_note_set(page, true, if e.is_empty() { "The core did not accept that.".to_string() } else { e });
            }
            Ok(v) => {
                self.pg_done(page, kind, &arg, &v);
                // lists are read again so the page shows what is true now
                if page_cat != Category::Data {
                    self.pg_load(page_cat);
                }
            }
        }
    }

    fn pg_done(&mut self, page: &str, kind: &str, arg: &str, v: &Value) {
        let note = match kind {
            "mcp-add" => model::tools::after_add(v, arg),
            "mcp-approve" => format!("Approved {arg}. It is running now."),
            "mcp-remove" => format!("Removed {arg}."),
            "trust-forget" => "That folder's decision was forgotten. You are asked again the next time.".to_string(),
            "ch-add" => format!("Added {arg}. It is OFF. Read the warning, then switch it on when you are ready."),
            "ch-edit" => format!("Saved {arg}."),
            "ch-enable" => format!("{arg} is switched on."),
            "ch-disable" => format!("{arg} is switched off."),
            "ch-remove" => format!("Removed {arg}."),
            "dev-open" => format!("Pairing is open for {} minutes. On the other device, open this computer's address and press Pair. It shows a PIN: type it below.", v["open_for_minutes"]),
            "dev-accept" => format!("Paired {}. It can sign in now.", model::s(v, "name")),
            "phone-new" => "Made a new sign-in link. The old one no longer works; phones already signed in stay in.".to_string(),
            "dev-remove" => "Removed. That device has to pair again to come back.".to_string(),
            "export" => model::data::export_note(v),
            "import-look" => {
                let p = model::data::preview(v);
                self.pg.replace.clear();
                self.pg.replace_style = false;
                let found = model::data::has_content(&p);
                self.pg.import = Some(p);
                if !found {
                    self.pg_note_set(page, false, "There is nothing in that folder to bring in.");
                }
                return;
            }
            "import-apply" => {
                let p = model::data::preview(v);
                self.pg.import = None;
                let done = p.done.map(|d| d.iter().map(|(k, val)| format!("{k}: {val}")).collect::<Vec<_>>().join(", ")).unwrap_or_default();
                format!("Brought in. {done}")
            }
            "undo" => model::activity::undo_note(v),
            "sch-add" => "Added. It runs at the next time it is due.".to_string(),
            "sch-remove" => "Deleted.".to_string(),
            _ => String::new(),
        };
        if !note.is_empty() {
            self.pg_note_set(page, false, note);
        }
    }

    // ---- building blocks ------------------------------------------------------------------------

    fn pg_note_line(&self, t: impl Into<SharedString>) -> AnyElement {
        div().text_sm().text_color(self.pal.muted).child(t.into()).into_any_element()
    }

    /// A raised box for one thing: a server, a channel, a device.
    fn xp_card(&self, children: Vec<AnyElement>) -> Div {
        let p = &self.pal;
        div().flex().flex_col().gap_1p5().p_3().rounded(p.inner).bg(p.sunken).text_color(p.text).min_w_0().children(children)
    }

    fn pg_title(&self, t: impl Into<SharedString>) -> AnyElement {
        div().text_color(self.pal.text).font_weight(gpui::FontWeight::MEDIUM).child(t.into()).into_any_element()
    }

    fn pg_muted(&self, t: impl Into<SharedString>) -> AnyElement {
        div().min_w_0().text_xs().text_color(self.pal.muted).child(t.into()).into_any_element()
    }

    /// What just happened on this page, red when it is a problem.
    fn pg_note_for(&self, page: &str) -> Option<AnyElement> {
        let (problem, text) = self.pg.notes.get(page)?.clone();
        let p = &self.pal;
        Some(div().min_w_0().px_2().py_1().rounded(p.inner).border_1().border_color(if problem { p.danger } else { p.accent }).text_sm().text_color(if problem { p.danger } else { p.text }).child(text).into_any_element())
    }

    /// A line that says the list is not available.
    fn pg_error_line(&self, what: &str, e: &str) -> AnyElement {
        div().text_sm().text_color(self.pal.danger).child(format!("{what} could not be read: {e}")).into_any_element()
    }

    /// The text box with this name, made the first time it is drawn. A value waiting in `set_next` is put in first.
    fn xp_input(&mut self, key: &str, hint: &str, window: &mut Window, cx: &mut Context<Self>) -> Entity<InputState> {
        let st = match self.pg.inputs.get(key).cloned() {
            Some(s) => s,
            None => {
                let hint = hint.to_string();
                let s = cx.new(|cx| InputState::new(window, cx).placeholder(hint));
                self.pg.inputs.insert(key.to_string(), s.clone());
                s
            }
        };
        if let Some(v) = self.pg.set_next.remove(key) {
            st.update(cx, |i, cx| i.set_value(v, window, cx));
        }
        st
    }

    fn xp_field(&mut self, key: &str, label: &str, hint: &str, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let input = self.xp_input(key, hint, window, cx);
        div().flex().flex_col().gap_1().child(div().text_sm().text_color(self.pal.muted).child(label.to_string())).child(Input::new(&input).aria_label(label.to_string())).into_any_element()
    }

    fn pg_text(&self, key: &str, cx: &mut Context<Self>) -> String {
        self.pg.inputs.get(key).map(|i| i.read(cx).value().to_string()).unwrap_or_default()
    }

    fn pg_clear(&mut self, keys: &[&str]) {
        for k in keys {
            if self.pg.inputs.contains_key(*k) {
                self.pg.set_next.insert((*k).to_string(), String::new());
            }
        }
    }

    /// Open the folder picker; the chosen folder goes into the box named `key` (with `suffix` added to it).
    fn pg_pick_folder(&mut self, key: &str, suffix: &'static str, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions { files: false, directories: true, multiple: false, prompt: Some("Choose".into()) });
        let key = key.to_string();
        cx.spawn(async move |this: WeakEntity<App>, cx: &mut AsyncApp| {
            if let Ok(Ok(Some(paths))) = rx.await {
                if let Some(path) = paths.first() {
                    let mut p = path.to_string_lossy().to_string();
                    if !suffix.is_empty() {
                        p = format!("{}/{suffix}", p.trim_end_matches('/'));
                    }
                    let _ = this.update(cx, |a, cx| {
                        a.pg.set_next.insert(key, p);
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    fn pg_toggle(&mut self, name: &str) {
        if !self.pg.open.remove(name) {
            self.pg.open.insert(name.to_string());
        }
    }

    /// Two clicks for something that cannot be taken back: the first asks, the second does it.
    fn pg_confirm_button(&mut self, id: &str, label: &str, sure: &str, cx: &mut Context<Self>, go: impl Fn(&mut App, &mut Window, &mut Context<App>) + 'static) -> AnyElement {
        let waiting = self.pg.confirm.as_deref() == Some(id);
        let key = id.to_string();
        if waiting {
            self.button(format!("pg-{id}-sure"), sure.to_string(), Kind::Danger, cx, go)
        } else {
            self.button(format!("pg-{id}"), label.to_string(), Kind::Ghost, cx, move |a, _, cx| {
                a.pg.confirm = Some(key.clone());
                cx.notify();
            })
        }
    }

    fn pg_refresh(&mut self, page: Category, cx: &mut Context<Self>) -> AnyElement {
        self.button(format!("pg-refresh-{}", page.id()), "Refresh", Kind::Ghost, cx, move |a, _, cx| {
            a.pg_load(page);
            cx.notify();
        })
    }

    /// The core's answer for a list, as JSON or an error line.
    fn pg_list(&self, key: &str) -> Option<Result<Value, String>> {
        self.pg.data.get(key).cloned()
    }
}
