//! The application state and what happens when something changes: a key is pressed, the core says something,
//! a theme file is edited. Drawing is in `ui.rs`.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use gpui::*;
use gpui_component::input::{InputEvent, TextareaState};
use lh_client::ServerEvent;
use serde_json::Value;

use lh_gui::api::{futures_channel, Backend, Cmd, Ev, StoredChat};
use lh_gui::config::Config;
use lh_gui::layout::{Layout, Page, Panel};
use lh_gui::model::{Block, Chats, NEW_TITLE};
use lh_gui::models::ModelList;
use lh_gui::prefs::Prefs;
use lh_gui::rtl::{self, Direction};
use lh_gui::runs::{Question, Run};
use crate::style::{apply_component_theme, Pal};
use lh_gui::theme::{self, ThemeSet};

/// A question about continuing a chat that the user has to answer: the warning the core gave.
#[derive(Clone, Debug)]
pub struct Pending {
    pub chat: u64,
    pub model: String,
    pub warning: Value,
}

/// Departments, memory and settings, as the right panel and the department pages show them.
#[derive(Default)]
pub struct PanelData {
    pub departments: Vec<Department>,
    pub runs: HashMap<String, Run>,
    pub run_order: Vec<String>,
    pub pages: Vec<MemoryPage>,
    pub settings: Vec<Setting>,
    pub memory_error: Option<String>,
    pub settings_error: Option<String>,
    pub departments_error: Option<String>,
    pub goal_note: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Department {
    pub name: String,
    pub description: String,
    pub lead: String,
    pub members: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemoryPage {
    pub id: String,
    pub layer: String,
    pub title: String,
    pub gist: String,
    pub body: String,
    pub privacy: String,
    pub pinned: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Setting {
    pub key: String,
    pub group: String,
    pub value: String,
    pub help: String,
    pub kind: lh_gui::settings::Kind,
}

pub struct App {
    pub cfg: Config,
    pub backend: Backend,
    pub connected: bool,
    pub banner: Option<String>,
    pub models: ModelList,
    pub chats: Chats,
    pub composer: Entity<TextareaState>,
    pub themes: ThemeSet,
    pub active_theme: usize,
    pub prefs: Prefs,
    pub window_radius: Option<f32>,
    pub pal: Pal,
    pub scroll: ScrollHandle,
    pub model_menu: bool,
    pub theme_menu: bool,
    pub pending: Option<Pending>,
    pub confirm_delete: Option<u64>,
    pub open_blocks: HashSet<(u64, usize)>,
    pub hover: Option<SharedString>,
    pub hover_ever: bool,
    pub rtl: bool,
    pub locale: String,
    pub panels: PanelData,
    pub goal: Entity<TextareaState>,
    pub dept_pick: Option<String>,
    /// The model name each session was last told to use (to know when a send must switch it).
    pub core_models: HashMap<u64, String>,
    pub content: Vec<crate::panels::Content>,
    pub content_index: usize,
}

fn stored_to_chat(chats: &mut Chats, models: &ModelList, s: &StoredChat) {
    let model = models.name_of(&s.model);
    match chats.by_session(&s.id) {
        Some(c) => {
            if !s.title.is_empty() && c.title == NEW_TITLE {
                c.title = s.title.clone();
            }
        }
        None => {
            if s.messages > 0 {
                chats.add_stored(&s.id, &s.title, &model);
            }
        }
    }
}

impl App {
    pub fn new(cfg: Config, window: &mut Window, cx: &mut Context<Self>) -> App {
        let prefs = Prefs::load(&cfg.prefs);
        let themes = ThemeSet::load(&cfg.themes);
        let active_theme = themes.themes.iter().position(|t| t.label() == prefs.theme).unwrap_or(0);
        let window_radius = theme::window_radius();
        let pal = Pal::from(&themes.themes[active_theme], window_radius);
        apply_component_theme(&pal, cx);

        let (tx, mut rx) = futures_channel::unbounded::<Ev>();
        let backend = Backend::start(cfg.clone(), tx);
        backend.send(Cmd::Refresh);
        cx.spawn(async move |this: WeakEntity<App>, cx: &mut AsyncApp| {
            loop {
                let ev = rx.recv().await;
                if this.update(cx, |app, cx| app.on_event(ev, cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        // theme files edited while the app runs, and the core appearing or disappearing
        cx.spawn(async move |this: WeakEntity<App>, cx: &mut AsyncApp| {
            let mut tick = 0u64;
            loop {
                cx.background_executor().timer(Duration::from_millis(400)).await;
                tick += 1;
                let alive = this.update(cx, |app, cx| {
                    if app.themes.changed() {
                        let label = app.themes.themes[app.active_theme].label();
                        app.themes.reload();
                        app.active_theme = app.themes.themes.iter().position(|t| t.label() == label).unwrap_or(0);
                        app.refresh_pal(cx);
                        cx.notify();
                    }
                    // look for the core again every 3 s while it is away, and refresh the lists every 20 s
                    if (!app.connected && tick.is_multiple_of(8)) || tick.is_multiple_of(50) {
                        app.backend.send(Cmd::Refresh);
                    }
                });
                if alive.is_err() {
                    break;
                }
            }
        })
        .detach();

        let composer = cx.new(|cx| TextareaState::new(window, cx).auto_grow(1, 8).placeholder("Message"));
        composer.update(cx, |s, cx| s.focus(window, cx));
        let goal = cx.new(|cx| TextareaState::new(window, cx).auto_grow(2, 6).placeholder("What should the department do?"));
        cx.subscribe_in(&composer, window, |this: &mut App, _, ev: &InputEvent, window, cx| {
            // Enter sends; Shift+Enter makes a new line (the editor has already handled the newline case)
            if let InputEvent::PressEnter { shift: false, secondary: false } = ev {
                this.send(window, cx);
            }
        })
        .detach();
        let locale = rtl::desktop_locale();
        let is_rtl = prefs.direction.is_rtl(&locale);
        let mut chats = Chats::default();
        chats.new_chat(&prefs.model);
        let layout_model = prefs.model.clone();
        let _ = layout_model;
        App {
            cfg,
            backend,
            connected: false,
            banner: None,
            models: ModelList::default(),
            chats,
            composer,
            themes,
            active_theme,
            prefs,
            window_radius,
            pal,
            scroll: ScrollHandle::new(),
            model_menu: false,
            theme_menu: false,
            pending: None,
            confirm_delete: None,
            open_blocks: HashSet::new(),
            hover: None,
            hover_ever: false,
            rtl: is_rtl,
            locale,
            panels: PanelData::default(),
            goal,
            dept_pick: None,
            core_models: HashMap::new(),
            content: vec![],
            content_index: 0,
        }
    }

    pub fn layout(&self) -> &Layout {
        &self.prefs.layout
    }

    pub fn persist(&mut self) {
        self.prefs.theme = self.themes.themes[self.active_theme].label();
        if let Some(c) = self.chats.current() {
            if !c.model.is_empty() {
                self.prefs.model = c.model.clone();
            }
        }
        let _ = self.prefs.save(&self.cfg.prefs);
    }

    pub fn refresh_pal(&mut self, cx: &mut Context<Self>) {
        self.window_radius = theme::window_radius();
        self.pal = Pal::from(&self.themes.themes[self.active_theme], self.window_radius);
        apply_component_theme(&self.pal, cx);
    }

    /// Switch the look. The window itself must be transparent when the look lets the desktop show through.
    pub fn set_theme(&mut self, idx: usize, window: &mut Window, cx: &mut Context<Self>) {
        if idx >= self.themes.themes.len() {
            return;
        }
        self.active_theme = idx;
        self.refresh_pal(cx);
        self.apply_window(window);
        self.persist();
        cx.notify();
    }

    pub fn apply_window(&self, window: &mut Window) {
        let t = &self.themes.themes[self.active_theme];
        window.set_background_appearance(if t.blur { WindowBackgroundAppearance::Blurred } else if t.is_translucent() { WindowBackgroundAppearance::Transparent } else { WindowBackgroundAppearance::Opaque });
    }

    pub fn set_direction(&mut self, d: Direction) {
        self.prefs.direction = d;
        self.rtl = d.is_rtl(&self.locale);
        self.persist();
    }

    // ---- chat actions -------------------------------------------------------------------------

    pub fn new_chat(&mut self, cx: &mut Context<Self>) {
        let model = self.chats.current().map(|c| c.model.clone()).filter(|m| !m.is_empty()).or_else(|| self.models.names().first().cloned()).unwrap_or_default();
        self.chats.new_chat(&model);
        self.pending = None;
        self.layout_mut().back_to_chat();
        cx.notify();
    }

    pub fn layout_mut(&mut self) -> &mut Layout {
        &mut self.prefs.layout
    }

    pub fn select_chat(&mut self, key: u64, cx: &mut Context<Self>) {
        self.chats.select(key);
        self.pending = None;
        self.model_menu = false;
        self.layout_mut().back_to_chat();
        let (dormant, sid, loaded, model) = match self.chats.current() {
            Some(c) => (c.dormant, c.session.clone(), !c.blocks.is_empty(), c.model.clone()),
            None => return,
        };
        if dormant {
            if let Some(id) = sid {
                if !loaded {
                    self.backend.send(Cmd::LoadHistory { chat: key, id: id.clone() });
                }
                self.backend.send(Cmd::Resume { chat: key, id, model: Some(self.models.id_for(&model)).filter(|m| !m.is_empty()), confirm: false });
            }
        }
        self.scroll_to_end();
        cx.notify();
    }

    pub fn confirm_resume(&mut self, cx: &mut Context<Self>) {
        let Some(p) = self.pending.take() else { return };
        let sid = self.chats.list.iter().find(|c| c.key == p.chat).and_then(|c| c.session.clone());
        if let Some(id) = sid {
            self.backend.send(Cmd::Resume { chat: p.chat, id, model: Some(self.models.id_for(&p.model)).filter(|m| !m.is_empty()), confirm: true });
        }
        cx.notify();
    }

    pub fn cancel_resume(&mut self, cx: &mut Context<Self>) {
        self.pending = None;
        cx.notify();
    }

    /// The delete button asks twice: the first press arms it, the second deletes.
    pub fn delete_chat(&mut self, key: u64, cx: &mut Context<Self>) {
        if self.confirm_delete != Some(key) {
            self.confirm_delete = Some(key);
            cx.notify();
            return;
        }
        self.confirm_delete = None;
        let Some(c) = self.chats.list.iter().find(|c| c.key == key) else { return };
        let (sid, stored) = (c.session.clone(), c.dormant || c.session.is_some());
        self.backend.send(Cmd::Delete { chat: key, id: sid, stored });
        cx.notify();
    }

    pub fn choose_model(&mut self, name: &str, cx: &mut Context<Self>) {
        self.model_menu = false;
        let Some(c) = self.chats.current_mut() else { return };
        if c.model == name {
            return;
        }
        let (key, sid) = (c.key, c.session.clone());
        match sid {
            // a chat the core already knows: the core says what changing the model costs
            Some(id) if !c.blocks.is_empty() => {
                self.pending = Some(Pending { chat: key, model: name.to_string(), warning: Value::Null });
                self.backend.send(Cmd::Resume { chat: key, id, model: Some(self.models.id_for(name)), confirm: false });
            }
            _ => c.model = name.to_string(),
        }
        self.persist();
        cx.notify();
    }

    pub fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.composer.read(cx).value().to_string();
        let text = text.trim().to_string();
        let Some(c) = self.chats.current() else { return };
        if c.busy {
            return self.stop(cx);
        }
        if text.is_empty() || c.dormant || self.pending.is_some() {
            return;
        }
        if c.model.is_empty() {
            self.banner = Some("There is no model to talk to yet. Is the core running and does it have a model?".into());
            return cx.notify();
        }
        self.composer.update(cx, |s, cx| s.set_value("", window, cx));
        let (key, session, model) = (c.key, c.session.clone(), self.models.id_for(&c.model));
        // the model the core last had for this session, as an id; unknown after a restart (then nothing is switched)
        let session_model = c.session.as_ref().and_then(|_| self.core_models.get(&key)).filter(|m| !m.is_empty()).map(|m| self.models.id_for(m));
        if let Some(c) = self.chats.by_key(key) {
            c.push_user(&text);
        }
        self.backend.send(Cmd::Send { chat: key, session, session_model, model, text });
        self.scroll_to_end();
        self.persist();
        cx.notify();
    }

    pub fn stop(&mut self, cx: &mut Context<Self>) {
        if let Some(c) = self.chats.current_mut() {
            let (key, session) = (c.key, c.session.clone());
            self.backend.send(Cmd::Stop { chat: key, session });
            if let Some(c) = self.chats.by_key(key) {
                c.apply(&ServerEvent::Error("stopped".into()));
            }
        }
        cx.notify();
    }

    pub fn answer(&mut self, ask: &str, yes: bool, cx: &mut Context<Self>) {
        if let Some(c) = self.chats.current_mut() {
            if let Some(s) = c.session.clone() {
                self.backend.send(Cmd::Answer { session: s, ask: ask.into(), yes });
            }
            c.answer(ask);
        }
        cx.notify();
    }

    pub fn scroll_to_end(&mut self) {
        let max = self.scroll.max_offset();
        self.scroll.set_offset(point(px(0.), -max.y));
    }

    // ---- events from the core -----------------------------------------------------------------

    pub fn on_event(&mut self, ev: Ev, cx: &mut Context<Self>) {
        match ev {
            Ev::Connected(ok) => {
                if ok && !self.connected {
                    self.banner = None;
                    self.load_panels();
                }
                self.connected = ok;
            }
            Ev::Models(ids) => {
                self.models = ModelList::new(ids);
                let first = self.models.names().first().cloned().unwrap_or_default();
                let preferred = self.prefs.model.clone();
                for c in self.chats.list.iter_mut() {
                    if c.model.is_empty() || (!self.models.has(&c.model) && c.blocks.is_empty() && c.session.is_none()) {
                        c.model = if self.models.has(&preferred) { preferred.clone() } else { first.clone() };
                    }
                }
            }
            Ev::Chats(list) => {
                for s in &list {
                    stored_to_chat(&mut self.chats, &self.models, s);
                }
            }
            Ev::SessionReady { chat, session, model } => {
                if let Some(c) = self.chats.by_key(chat) {
                    c.session = Some(session);
                }
                let name = self.models.name_of(&model);
                let _ = name;
                self.core_models.insert(chat, self.models.name_of(&model));
            }
            Ev::Server { chat, event } => {
                let current = self.chats.current().map(|c| c.key);
                if let Some(c) = self.chats.by_key(chat) {
                    c.apply(&event);
                }
                if current == Some(chat) {
                    self.scroll_to_end();
                }
            }
            Ev::StreamEnded { chat } => {
                if let Some(c) = self.chats.by_key(chat) {
                    c.end_turn();
                }
                self.persist();
            }
            Ev::Failed { chat, message } => {
                if chat == 0 {
                    self.banner = Some(message);
                } else {
                    if let Some(c) = self.chats.by_key(chat) {
                        c.end_turn();
                        c.blocks.push(Block::Error(message));
                    }
                    if self.pending.as_ref().is_some_and(|p| p.chat == chat) {
                        self.pending = None;
                    }
                }
            }
            Ev::Deleted { chat } => {
                let model = self.chats.current().map(|c| c.model.clone()).unwrap_or_default();
                self.chats.remove(chat, &model);
                self.core_models.remove(&chat);
                self.pending = None;
            }
            Ev::History { chat, messages, context_tokens } => {
                if let Some(c) = self.chats.by_key(chat) {
                    c.load_history(&messages);
                    c.context_tokens = Some(context_tokens);
                }
                self.scroll_to_end();
            }
            Ev::ResumeWarning { chat, warning, model } => {
                let name = self.models.name_of(&model);
                self.pending = Some(Pending { chat, model: name, warning });
            }
            Ev::Resumed { chat, session, model, context_tokens } => {
                let name = self.models.name_of(&model);
                if let Some(c) = self.chats.by_key(chat) {
                    c.session = Some(session);
                    c.dormant = false;
                    c.context_tokens = Some(context_tokens);
                    if !name.is_empty() {
                        c.model = name.clone();
                    }
                }
                self.core_models.insert(chat, name);
                self.pending = None;
                self.persist();
            }
            Ev::Reply { tag, result } => self.on_reply(&tag, result),
            Ev::RunStarted { run, department, goal } => {
                if !self.panels.run_order.contains(&run) {
                    self.panels.run_order.push(run.clone());
                }
                self.panels.runs.entry(run.clone()).or_insert_with(|| Run::new(&run, &department, &goal));
            }
            Ev::RunEvent { run, event } => {
                if let Some(r) = self.panels.runs.get_mut(&run) {
                    r.apply(&event);
                }
            }
            Ev::RunEnded { run } => {
                if let Some(r) = self.panels.runs.get_mut(&run) {
                    r.finished = true;
                }
            }
        }
        cx.notify();
    }

    // ---- side panels --------------------------------------------------------------------------

    pub fn load_panels(&mut self) {
        for (tag, path) in [("departments", "/v1/team/departments"), ("memory", "/v1/memory/pages"), ("settings", "/v1/settings"), ("runs", "/v1/team/runs")] {
            self.backend.send(Cmd::Call { tag: tag.into(), method: "GET".into(), path: path.into(), body: None });
        }
    }

    fn on_reply(&mut self, tag: &str, result: Result<Value, String>) {
        let p = &mut self.panels;
        match (tag, result) {
            ("departments", Ok(v)) => {
                p.departments_error = None;
                p.departments = v["departments"].as_array().map(|a| a.iter().filter(|d| d["error"].is_null()).map(|d| Department { name: d["name"].as_str().unwrap_or("").into(), description: d["description"].as_str().unwrap_or("").into(), lead: d["lead"].as_str().unwrap_or("").into(), members: d["members"].as_array().map(|m| m.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default() }).collect()).unwrap_or_default();
            }
            ("departments", Err(e)) => p.departments_error = Some(e),
            ("memory", Ok(v)) => {
                p.memory_error = None;
                p.pages = v["pages"].as_array().map(|a| a.iter().filter(|x| x["archived"] != true).map(|x| MemoryPage { id: x["id"].as_str().unwrap_or("").into(), layer: x["layer"].as_str().unwrap_or("").into(), title: x["title"].as_str().unwrap_or("").into(), gist: x["gist"].as_str().unwrap_or("").into(), body: x["body"].as_str().unwrap_or("").into(), privacy: x["privacy"].as_str().unwrap_or("").into(), pinned: x["pinned"] == true }).collect()).unwrap_or_default();
            }
            ("memory", Err(e)) => p.memory_error = Some(e),
            ("settings", Ok(v)) => {
                p.settings_error = None;
                p.settings = v["settings"].as_array().map(|a| a.iter().map(|x| Setting { key: x["key"].as_str().unwrap_or("").into(), group: x["group"].as_str().unwrap_or("").into(), value: match &x["value"] { Value::String(s) => s.clone(), other => other.to_string() }, help: x["help"].as_str().unwrap_or("").into(), kind: lh_gui::settings::Kind::from_json(&x["kind"]) }).collect()).unwrap_or_default();
            }
            ("settings", Err(e)) => p.settings_error = Some(e),
            ("runs", Ok(v)) => {
                // pick up runs that are still going from before the window opened
                let runs: Vec<(String, String, String, bool)> = v["runs"].as_array().map(|a| a.iter().map(|r| (r["id"].as_str().unwrap_or("").to_string(), r["department"].as_str().unwrap_or("").to_string(), r["goal"].as_str().unwrap_or("").to_string(), r["finished"] == true)).collect()).unwrap_or_default();
                for (id, dept, goal, finished) in runs {
                    if !p.runs.contains_key(&id) && !finished {
                        self.backend.send(Cmd::FollowRun { run: id, department: dept, goal });
                    }
                }
            }
            ("run", Err(e)) => p.goal_note = Some(e),
            (t, Ok(_)) if t.starts_with("setting:") => {
                p.settings_error = None;
                self.backend.send(Cmd::Call { tag: "settings".into(), method: "GET".into(), path: "/v1/settings".into(), body: None });
            }
            (t, Err(e)) if t.starts_with("setting:") => p.settings_error = Some(e),
            _ => {}
        }
    }

    pub fn open_panel(&mut self, panel: Panel, cx: &mut Context<Self>) {
        self.layout_mut().toggle_panel(panel);
        if self.connected {
            self.load_panels();
        }
        self.persist();
        cx.notify();
    }

    pub fn open_department(&mut self, name: &str, cx: &mut Context<Self>) {
        self.layout_mut().open_department(name);
        self.dept_pick = Some(name.to_string());
        cx.notify();
    }

    pub fn start_run(&mut self, dept: &str, window: &mut Window, cx: &mut Context<Self>) {
        let goal = self.goal.read(cx).value().trim().to_string();
        if goal.is_empty() {
            self.panels.goal_note = Some("Say what the department should do first.".into());
            return cx.notify();
        }
        self.goal.update(cx, |s, cx| s.set_value("", window, cx));
        self.panels.goal_note = None;
        self.backend.send(Cmd::StartRun { department: dept.into(), goal, project_dir: None });
        cx.notify();
    }

    pub fn answer_run(&mut self, run: &str, q: &Question, yes: bool, cx: &mut Context<Self>) {
        let body = match q {
            Question::Plan { .. } => if yes { serde_json::json!({"decision": "approve"}) } else { serde_json::json!({"decision": "reject", "note": "not approved"}) },
            _ => serde_json::json!({"answer": if yes { "yes" } else { "no" }}),
        };
        self.backend.send(Cmd::AnswerRun { run: run.into(), question: q.id().into(), body });
        if let Some(r) = self.panels.runs.get_mut(run) {
            r.answered(q.id());
        }
        cx.notify();
    }

    pub fn change_setting(&mut self, key: &str, value: String, cx: &mut Context<Self>) {
        self.backend.send(Cmd::Call { tag: format!("setting:{key}"), method: "PUT".into(), path: "/v1/settings".into(), body: Some(serde_json::json!({ key: value })) });
        cx.notify();
    }

    pub fn page(&self) -> &Page {
        &self.prefs.layout.page
    }
}
