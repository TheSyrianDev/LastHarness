//! The application state and what happens when something changes: a key is pressed, the core says something,
//! a theme file is edited. Drawing is in `ui.rs`.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use gpui::*;
use gpui_component::input::{InputEvent, InputState, TextareaState};
use lh_client::ServerEvent;
use serde_json::Value;

use lh_gui::api::{futures_channel, Backend, Cmd, Ev, SessionStart, StoredChat};
use lh_gui::config::Config;
use lh_gui::layout::{Layout, Page, Panel};
use lh_gui::model::{Block, Chats, Mode, NEW_TITLE};
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
    pub proposals: Vec<Proposal>,
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

/// Something the assistant suggests remembering, waiting for the user.
#[derive(Clone, Debug, PartialEq)]
pub struct Proposal {
    pub id: i64,
    pub kind: String,
    pub title: String,
    pub text: String,
    pub evidence: String,
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
    /// What the desktop last said about dark or light, when it says.
    pub system_dark: Option<bool>,
    pub prefs: Prefs,
    pub window_radius: Option<f32>,
    pub pal: Pal,
    pub scroll: ScrollHandle,
    /// Whether the conversation follows new text, or the person scrolled up and is reading.
    pub follow: lh_gui::follow::Follow,
    /// The thinking text of a running turn, when it is opened: its own scroll, and the same follow rule as the chat.
    pub think_scroll: ScrollHandle,
    pub think_follow: lh_gui::follow::Follow,
    pub model_menu: bool,
    pub pending: Option<Pending>,
    pub confirm_delete: Option<u64>,
    pub open_blocks: HashSet<(u64, usize)>,
    pub hover: Option<SharedString>,
    pub hover_ever: bool,
    pub rtl: bool,
    pub locale: String,
    pub panels: PanelData,
    pub goal: Entity<TextareaState>,
    /// The folder for a chat in project mode.
    pub project_input: Entity<InputState>,
    pub dept_pick: Option<String>,
    /// The model name each session was last told to use (to know when a send must switch it).
    pub core_models: HashMap<u64, String>,
    pub content: Vec<crate::panels::Content>,
    pub content_index: usize,
    /// The panel the person last used.
    pub focus: Focus,
    /// Seconds the bubbles have drifted; only moves while the look has bubbles.
    /// A problem to show on the opened-content panel, such as a browser that would not start.
    pub content_note: Option<String>,
    /// The chat tree panel: whose tree it shows, the chosen line, a question waiting.
    pub tree: lh_gui::treeview::TreeState,
    /// A name being typed for a line of the tree.
    pub tree_label: Entity<InputState>,
    /// Words to put in the message box at the next draw (choosing something said earlier brings it back).
    pub pending_input: Option<String>,
    /// Questions the person answered "Always for this chat" to.
    pub always: lh_gui::permissions::Always,
    /// The command palette, while it is open.
    pub palette: Option<lh_gui::palette::Palette>,
    pub palette_input: Entity<InputState>,
    /// The chat list and the side panel sliding open and shut.
    pub left_reveal: lh_gui::motion::Reveal,
    pub right_reveal: lh_gui::motion::Reveal,
    /// The Settings window, while it is up, and where keyboard focus lives inside it.
    pub settings_open: bool,
    pub settings_focus: FocusHandle,
    /// While the Settings window is drawn its buttons can take keyboard focus (elsewhere they would steal it from
    /// the message box when clicked).
    pub tab_buttons: bool,
    /// The chat width slider, number box and presets in Settings.
    pub width: crate::width::WidthControls,
    /// Said once, under Appearance: the look that was saved does not exist any more.
    pub look_notice: Option<String>,
    /// The web page drawn inside the Opened tab, while it is shown.
    pub page: Option<crate::page_ui::PageState>,
    /// The page shown in Settings, the box that searches all pages, and keyboard focus for the list of pages.
    pub settings_page: lh_gui::setting_names::Category,
    pub settings_search: Entity<InputState>,
    pub settings_nav_focus: FocusHandle,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Focus {
    Sidebar,
    #[default]
    Chat,
    Side,
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
                if let Some(c) = chats.by_session(&s.id) {
                    c.mode = Mode::parse(&s.mode);
                }
            }
        }
    }
}

impl App {
    pub fn new(cfg: Config, window: &mut Window, cx: &mut Context<Self>) -> App {
        let mut prefs = Prefs::load(&cfg.prefs);
        let themes = ThemeSet::load(&cfg.themes);
        // a saved look that no longer exists falls back to the first look, once, with a line saying so under Appearance
        let look_notice = lh_gui::theme::missing_look_notice(&prefs.theme, &themes.families());
        if look_notice.is_some() {
            prefs.theme = String::new();
        }
        let system_dark = lh_gui::appearance::system_dark();
        let active_theme = themes.resolve(&prefs.theme, lh_gui::appearance::is_dark(prefs.mode, system_dark));
        let window_radius = theme::window_radius();
        let pal = Pal::from(&themes.themes[active_theme], window_radius, lh_gui::config::reduce_motion());
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
                        app.themes.reload();
                        app.active_theme = app.themes.resolve(&app.prefs.theme, app.dark());
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

        // "follow the system": look at the desktop's setting every 3 s while that is chosen, off the screen's thread
        cx.spawn_in(window, async move |this: WeakEntity<App>, cx: &mut AsyncWindowContext| loop {
            cx.background_executor().timer(Duration::from_secs(3)).await;
            let follows = this.update(cx, |app, _| app.prefs.mode == lh_gui::appearance::ModeChoice::System);
            match follows {
                Err(_) => break,
                Ok(false) => continue,
                Ok(true) => {}
            }
            let now = cx.background_executor().spawn(async { lh_gui::appearance::system_dark() }).await;
            let alive = this.update_in(cx, |app, window, cx| {
                if app.system_dark != now {
                    app.system_dark = now;
                    app.reapply_look(window, cx);
                }
            });
            if alive.is_err() {
                break;
            }
        })
        .detach();

        // a web page in the Opened tab opens once the panel has a size and follows a resize once it stops changing
        cx.spawn_in(window, async move |this: WeakEntity<App>, cx: &mut AsyncWindowContext| loop {
            cx.background_executor().timer(Duration::from_millis(120)).await;
            let alive = this.update_in(cx, |app, window, cx| {
                if app.page.is_some() {
                    app.page_poll(window, cx);
                }
            });
            if alive.is_err() {
                break;
            }
        })
        .detach();

        // while the model thinks, the progress line (count, speed, time) is redrawn once a second, not for every token
        cx.spawn_in(window, async move |this: WeakEntity<App>, cx: &mut AsyncWindowContext| loop {
            cx.background_executor().timer(Duration::from_secs(1)).await;
            let alive = this.update_in(cx, |app, _, cx| {
                if app.chats.current().is_some_and(|c| c.busy && c.think.active()) {
                    cx.notify();
                }
            });
            if alive.is_err() {
                break;
            }
        })
        .detach();


        let composer = cx.new(|cx| TextareaState::new(window, cx).auto_grow(1, 8).placeholder("Message"));
        composer.update(cx, |s, cx| s.focus(window, cx));
        let project_input = cx.new(|cx| InputState::new(window, cx).placeholder("/path/to/the/project folder"));
        let goal = cx.new(|cx| TextareaState::new(window, cx).auto_grow(2, 6).placeholder("What should the department do?"));
        cx.subscribe_in(&composer, window, |this: &mut App, _, ev: &InputEvent, window, cx| {
            // Enter sends; Shift+Enter makes a new line (the editor has already handled the newline case)
            if let InputEvent::PressEnter { shift: false, secondary: false } = ev {
                this.send(window, cx);
            }
        })
        .detach();
        let palette_input = cx.new(|cx| InputState::new(window, cx).placeholder("Type a command, a chat, a model or a look"));
        cx.subscribe_in(&palette_input, window, |this: &mut App, input, ev: &InputEvent, window, cx| match ev {
            InputEvent::Change => {
                let q = input.read(cx).value().to_string();
                if let Some(p) = this.palette.as_mut() {
                    p.set_query(&q);
                }
                cx.notify();
            }
            InputEvent::PressEnter { .. } => this.run_selected(window, cx),
            _ => {}
        })
        .detach();
        // the Settings window keeps focus to itself: if it leaves (Tab past the last button), it is brought back
        let settings_focus = cx.focus_handle();
        cx.on_focus_out(&settings_focus, window, |this: &mut App, _, window, cx| {
            if this.settings_open && !this.settings_focus.contains_focused(window, cx) {
                this.settings_focus.focus(window, cx);
            }
        })
        .detach();
        let locale = rtl::desktop_locale();
        let is_rtl = prefs.direction.is_rtl(&locale);
        let prefs_column = prefs.layout.column;
        let settings_page = lh_gui::setting_names::Category::from_id(&prefs.settings_page);
        let settings_search = cx.new(|cx| InputState::new(window, cx).placeholder("Search the settings"));
        cx.subscribe_in(&settings_search, window, |_, _, ev: &InputEvent, _, cx| {
            if matches!(ev, InputEvent::Change) {
                cx.notify();
            }
        })
        .detach();
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
            system_dark,
            prefs,
            window_radius,
            pal,
            scroll: ScrollHandle::new(),
            follow: Default::default(),
            think_scroll: ScrollHandle::new(),
            think_follow: Default::default(),
            model_menu: false,
            pending: None,
            confirm_delete: None,
            open_blocks: HashSet::new(),
            hover: None,
            hover_ever: false,
            rtl: is_rtl,
            locale,
            panels: PanelData::default(),
            goal,
            project_input,
            dept_pick: None,
            core_models: HashMap::new(),
            content: vec![],
            content_index: 0,
            focus: Focus::default(),
            content_note: None,
            tree: Default::default(),
            tree_label: cx.new(|cx| InputState::new(window, cx).placeholder("A name for this line")),
            pending_input: None,
            always: Default::default(),
            palette: None,
            palette_input,
            left_reveal: Default::default(),
            right_reveal: Default::default(),
            settings_open: false,
            settings_focus,
            tab_buttons: false,
            width: crate::width::WidthControls::new(window, cx, prefs_column, is_rtl),
            look_notice,
            page: None,
            settings_page,
            settings_search,
            settings_nav_focus: cx.focus_handle(),
        }
    }

    pub fn layout(&self) -> &Layout {
        &self.prefs.layout
    }

    pub fn persist(&mut self) {
        if let Some(c) = self.chats.current() {
            if !c.model.is_empty() {
                self.prefs.model = c.model.clone();
            }
        }
        let _ = self.prefs.save(&self.cfg.prefs);
    }

    pub fn focus_on(&mut self, f: Focus, cx: &mut Context<Self>) {
        if self.focus != f {
            self.focus = f;
            cx.notify();
        }
    }

    pub fn refresh_pal(&mut self, cx: &mut Context<Self>) {
        self.window_radius = theme::window_radius();
        self.pal = Pal::from(&self.themes.themes[self.active_theme], self.window_radius, lh_gui::config::reduce_motion());
        apply_component_theme(&self.pal, cx);
    }

    /// Is the dark variant of the looks in use?
    pub fn dark(&self) -> bool {
        lh_gui::appearance::is_dark(self.prefs.mode, self.system_dark)
    }

    /// The family of looks in use ("Calm"), whatever mode it is showing in.
    pub fn family(&self) -> String {
        self.themes.themes[self.active_theme].name.clone()
    }

    /// Pick the look for the saved family and mode and show it. The window itself must be transparent when the look
    /// lets the desktop show through.
    pub fn reapply_look(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.active_theme = self.themes.resolve(&self.prefs.theme, self.dark());
        self.refresh_pal(cx);
        self.apply_window(window);
        self.persist();
        cx.notify();
    }

    /// Choose a family of looks (Calm, Matte, Glass, or one of the person's own).
    pub fn set_family(&mut self, family: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.themes.families().iter().any(|f| f == family) {
            self.prefs.theme = family.to_string();
            self.reapply_look(window, cx);
        }
    }

    /// Light, dark, or follow the desktop.
    pub fn set_appearance(&mut self, mode: lh_gui::appearance::ModeChoice, window: &mut Window, cx: &mut Context<Self>) {
        if mode == lh_gui::appearance::ModeChoice::System {
            self.system_dark = lh_gui::appearance::system_dark();
        }
        self.prefs.mode = mode;
        self.reapply_look(window, cx);
    }

    /// The light/dark key.
    pub fn toggle_mode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.system_dark = lh_gui::appearance::system_dark();
        let next = lh_gui::appearance::toggled(self.prefs.mode, self.system_dark);
        self.set_appearance(next, window, cx);
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

    pub fn set_mode(&mut self, mode: Mode, cx: &mut Context<Self>) {
        if let Some(c) = self.chats.current_mut() {
            if c.session.is_none() && c.blocks.is_empty() {
                c.mode = mode;
                if mode != Mode::Chat {
                    c.incognito = false; // a chat that works on the computer is kept, so it can be resumed
                }
            }
        }
        cx.notify();
    }

    pub fn toggle_private(&mut self, cx: &mut Context<Self>) {
        if let Some(c) = self.chats.current_mut() {
            if c.session.is_none() && c.blocks.is_empty() && c.mode == Mode::Chat {
                c.incognito = !c.incognito;
            }
        }
        cx.notify();
    }

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
        self.refresh_tree();
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
        let folder = self.project_input.read(cx).value().trim().to_string();
        let start = SessionStart { mode: c.mode.as_str().into(), project_dir: (c.mode == Mode::Project).then_some(folder.clone()), incognito: c.incognito };
        if c.session.is_none() && c.mode == Mode::Project && !std::path::Path::new(&folder).is_dir() {
            self.banner = Some("A project chat needs a folder that exists. Type its path above the message box.".into());
            return cx.notify();
        }
        self.banner = None;
        self.composer.update(cx, |s, cx| s.set_value("", window, cx));
        let (key, session, model) = (c.key, c.session.clone(), self.models.id_for(&c.model));
        // the model the core last had for this session, as an id; unknown after a restart (then nothing is switched)
        let session_model = c.session.as_ref().and_then(|_| self.core_models.get(&key)).filter(|m| !m.is_empty()).map(|m| self.models.id_for(m));
        if let Some(c) = self.chats.by_key(key) {
            c.push_user(&text);
        }
        self.backend.send(Cmd::Send { chat: key, session, session_model, model, text, start });
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

    /// Yes, and the same question from the same tool in this chat is answered yes from now on (until the app closes).
    pub fn answer_always(&mut self, ask: &str, tool: &str, reason: &str, cx: &mut Context<Self>) {
        if let Some(key) = self.chats.current().map(|c| c.key) {
            self.always.grant(key, tool, reason);
        }
        self.answer(ask, true, cx);
    }

    /// Answer the oldest open question of the chat that is shown (the Y and N keys).
    pub fn answer_first(&mut self, yes: bool, cx: &mut Context<Self>) {
        if let Some(id) = self.chats.current().and_then(|c| c.asks.first().map(|a| a.id.clone())) {
            self.answer(&id, yes, cx);
        }
    }

    /// Do the Y and N keys answer a question now? Only when one is open on the chat shown, and nothing that takes
    /// typing could be using the keys: the message box is empty and no name is being typed.
    pub fn asking(&self, cx: &Context<Self>) -> bool {
        self.page() == &Page::Chat
            && self.chats.current().is_some_and(|c| !c.asks.is_empty())
            && self.palette.is_none()
            && !self.settings_open
            && self.composer.read(cx).value().is_empty()
            && self.tree.prompt != lh_gui::treeview::Prompt::Label
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

    /// Go to the newest line and follow from there: sending a message, opening a chat, the jump button.
    pub fn scroll_to_end(&mut self) {
        let max = self.scroll.max_offset();
        self.scroll.set_offset(point(px(0.), -max.y));
        self.follow.placed((-max.y).into());
    }

    /// New thinking arrived in the open chat: the opened thinking text follows it only while it is at the bottom.
    pub fn follow_think(&mut self) {
        if self.chats.current().is_some_and(|c| c.think.snapshot(std::time::Instant::now()).is_some_and(|s| s.tokens <= 1)) {
            self.think_follow = Default::default(); // a new stretch of thinking starts at the bottom
        }
        let (offset, max) = (self.think_scroll.offset().y, self.think_scroll.max_offset().y);
        if self.think_follow.on_new_content(offset.into(), max.into()) {
            let m = self.think_scroll.max_offset();
            self.think_scroll.set_offset(point(px(0.), -m.y));
            self.think_follow.placed((-m.y).into());
        }
    }

    /// New text arrived in the open chat: follow it only if the view is at the bottom. If the person scrolled up
    /// their place is left alone and the jump button is shown.
    pub fn follow_stream(&mut self) {
        let (offset, max) = (self.scroll.offset().y, self.scroll.max_offset().y);
        if self.follow.on_new_content(offset.into(), max.into()) {
            self.scroll_to_end();
        }
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
                // a question the person already said "always" to is answered without asking again
                if let ServerEvent::Ask { id, tool, reason, .. } = &event {
                    if self.always.covers(chat, tool, reason) {
                        if let Some(session) = self.chats.by_key(chat).and_then(|c| c.session.clone()) {
                            self.backend.send(Cmd::Answer { session, ask: id.clone(), yes: true });
                            return cx.notify();
                        }
                    }
                }
                // the assistant opened something: it goes to the side panel (of whatever chat is shown)
                if let ServerEvent::Show { title, kind, content, path } = &event {
                    let c = lh_gui::content::Content::from_show(title, kind, content, path.as_deref());
                    self.open_content(c, cx);
                }
                let current = self.chats.current().map(|c| c.key);
                if let Some(c) = self.chats.by_key(chat) {
                    c.apply(&event);
                }
                if current == Some(chat) {
                    self.follow_stream();
                    self.follow_think();
                }
            }
            Ev::StreamEnded { chat } => {
                if let Some(c) = self.chats.by_key(chat) {
                    c.end_turn();
                }
                self.persist();
                self.refresh_tree();
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
                self.always.forget_chat(chat);
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
        for (tag, path) in [("departments", "/v1/team/departments"), ("memory", "/v1/memory/pages"), ("proposals", "/v1/memory/proposals"), ("settings", "/v1/settings"), ("runs", "/v1/team/runs")] {
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
            ("proposals", Ok(v)) => {
                p.proposals = v["proposals"].as_array().map(|a| a.iter().filter(|x| x["status"].as_str().unwrap_or("pending") == "pending").map(|x| Proposal { id: x["id"].as_i64().unwrap_or(0), kind: x["kind"].as_str().unwrap_or("").into(), title: x["title"].as_str().unwrap_or("").into(), text: x["text"].as_str().unwrap_or("").into(), evidence: x["evidence"].as_str().unwrap_or("").into() }).collect()).unwrap_or_default();
            }
            ("proposals", Err(_)) => p.proposals.clear(),
            ("memory-action", Ok(_)) => {
                for (tag, path) in [("memory", "/v1/memory/pages"), ("proposals", "/v1/memory/proposals")] {
                    self.backend.send(Cmd::Call { tag: tag.into(), method: "GET".into(), path: path.into(), body: None });
                }
            }
            ("memory-action", Err(e)) => p.memory_error = Some(e),
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
            (t, r) if t.starts_with("tree") => self.on_tree_reply(t, r),
            (t, Ok(_)) if t.starts_with("setting:") => {
                p.settings_error = None;
                self.backend.send(Cmd::Call { tag: "settings".into(), method: "GET".into(), path: "/v1/settings".into(), body: None });
            }
            (t, Err(e)) if t.starts_with("setting:") => p.settings_error = Some(e),
            _ => {}
        }
    }

    // ---- the chat tree ------------------------------------------------------------------------

    /// The core's id of the chat that is open, if it has one.
    fn tree_session(&self) -> Option<(u64, String)> {
        self.chats.current().and_then(|c| c.session.clone().map(|s| (c.key, s)))
    }

    /// Ask for the tree of the open chat.
    pub fn load_tree(&mut self) {
        match self.tree_session() {
            Some((key, sid)) => self.backend.send(Cmd::Call { tag: format!("tree:{key}"), method: "GET".into(), path: format!("/v1/chats/{sid}/tree"), body: None }),
            None => {
                self.tree = Default::default();
                self.tree.note = "Nothing to show yet: the tree starts with the first message.".into();
            }
        }
    }

    /// Read the tree of the open chat again. The title bar needs it for the branch it names, so it is read whether
    /// or not the branches tab is shown; a chat with no stored messages yet has none.
    pub fn refresh_tree(&mut self) {
        if self.connected && self.tree_session().is_some() {
            self.load_tree();
        }
    }

    /// The branch the title bar names for the chat that is open: "branch 2 of 3", or nothing for a single line.
    pub fn branches(&self) -> Option<(usize, Option<usize>)> {
        let key = self.chats.current()?.key;
        if self.tree.chat != Some(key) {
            return None;
        }
        self.tree.tree.as_ref()?.branches()
    }

    /// Go to a line of the tree. The core asks about a summary when the branch being left has messages.
    pub fn tree_go(&mut self, node: i64, summarize: Option<bool>) {
        if let Some((key, sid)) = self.tree_session() {
            let mut body = serde_json::json!({"node": node});
            if let Some(s) = summarize {
                body["summarize"] = s.into();
            }
            self.backend.send(Cmd::Call { tag: format!("tree-move:{key}:{node}"), method: "POST".into(), path: format!("/v1/chats/{sid}/tree/move"), body: Some(body) });
        }
    }

    pub fn tree_label(&mut self, node: i64, label: String) {
        if let Some((key, sid)) = self.tree_session() {
            self.backend.send(Cmd::Call { tag: format!("tree-label:{key}"), method: "POST".into(), path: format!("/v1/chats/{sid}/tree/label"), body: Some(serde_json::json!({"node": node, "label": label})) });
        }
    }

    pub fn tree_fork(&mut self, node: i64) {
        if let Some((key, sid)) = self.tree_session() {
            self.backend.send(Cmd::Call { tag: format!("tree-fork:{key}"), method: "POST".into(), path: format!("/v1/chats/{sid}/fork"), body: Some(serde_json::json!({"node": node})) });
        }
    }

    pub fn tree_clone(&mut self) {
        if let Some((key, sid)) = self.tree_session() {
            self.backend.send(Cmd::Call { tag: format!("tree-clone:{key}"), method: "POST".into(), path: format!("/v1/chats/{sid}/clone"), body: Some(serde_json::json!({})) });
        }
    }

    fn on_tree_reply(&mut self, tag: &str, result: Result<Value, String>) {
        use lh_gui::treeview::Outcome;
        let mut parts = tag.split(':');
        let (kind, key) = (parts.next().unwrap_or(""), parts.next().and_then(|k| k.parse::<u64>().ok()).unwrap_or(0));
        let node = parts.next().and_then(|n| n.parse::<i64>().ok()).unwrap_or(0);
        match (kind, result) {
            ("tree", Ok(v)) => match lh_client::tree::ChatTree::from_json(&v) {
                Ok(t) => self.tree.load(key, t),
                Err(e) => self.tree.failed(key, e),
            },
            ("tree", Err(e)) => self.tree.failed(key, e),
            ("tree-move", Ok(v)) => {
                if let Outcome::Moved { input, .. } = self.tree.moved(node, lh_client::tree::Moved::from_json(&v)) {
                    // the chat now shows its new branch, and the words of the message chosen go back in the box
                    if let Some(sid) = self.chats.by_key(key).and_then(|c| c.session.clone()) {
                        self.backend.send(Cmd::LoadHistory { chat: key, id: sid });
                    }
                    if input.is_some() {
                        self.pending_input = input;
                    }
                    self.load_tree();
                }
            }
            ("tree-move", Err(e)) | ("tree-label", Err(e)) | ("tree-fork", Err(e)) | ("tree-clone", Err(e)) => self.tree.note = e,
            ("tree-label", Ok(_)) => {
                self.tree.prompt = lh_gui::treeview::Prompt::None;
                self.load_tree();
            }
            ("tree-fork", Ok(_)) | ("tree-clone", Ok(_)) => {
                self.tree.note = "Made a new chat. It is in the chat list.".into();
                self.backend.send(Cmd::Refresh);
            }
            _ => {}
        }
    }

    pub fn open_panel(&mut self, panel: Panel, cx: &mut Context<Self>) {
        self.layout_mut().toggle_panel(panel);
        if self.connected {
            self.load_panels();
            self.refresh_tree();
        }
        self.persist();
        cx.notify();
    }

    /// Switch the side panel to another tab and keep it shown.
    pub fn show_in_dock(&mut self, panel: Panel, cx: &mut Context<Self>) {
        self.layout_mut().show_panel(panel);
        if self.connected {
            self.load_panels();
            self.refresh_tree();
        }
        self.persist();
        cx.notify();
    }

    // ---- the settings window ------------------------------------------------------------------

    /// Show Settings as a window over the app. Opening it puts the palette away; focus goes inside it.
    pub fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.palette = None;
        self.model_menu = false;
        self.settings_open = true;
        if self.connected {
            self.load_panels();
        }
        self.width.note = None;
        self.sync_width_widgets(window, cx);
        self.settings_focus.focus(window, cx);
        cx.notify();
    }

    /// Put Settings away and give focus back to the message box.
    /// Show another page of Settings. The page is remembered, and a search in progress is cleared.
    pub fn set_settings_page(&mut self, page: lh_gui::setting_names::Category, window: &mut Window, cx: &mut Context<Self>) {
        self.settings_page = page;
        self.prefs.settings_page = page.id().to_string();
        self.settings_search.update(cx, |s, cx| s.set_value("", window, cx));
        self.settings_nav_focus.focus(window, cx);
        self.persist();
        cx.notify();
    }

    pub fn close_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.settings_open {
            self.settings_open = false;
            self.composer.update(cx, |s, cx| s.focus(window, cx));
            cx.notify();
        }
    }

    pub fn toggle_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.settings_open {
            self.close_settings(window, cx);
        } else {
            self.open_settings(window, cx);
        }
    }

    // ---- the command palette ------------------------------------------------------------------

    /// Everything the palette lists, from what the app knows now.
    pub fn palette_snapshot(&self) -> lh_gui::palette::Snapshot {
        lh_gui::palette::Snapshot {
            rtl: self.rtl,
            chats: self.chats.list.iter().map(|c| (c.key, c.title.clone())).collect(),
            current_chat: self.chats.current().map(|c| c.key),
            models: self.models.names().to_vec(),
            current_model: self.chats.current().map(|c| c.model.clone()).unwrap_or_default(),
            looks: self.themes.families(),
            current_look: self.themes.families().iter().position(|f| *f == self.family()).unwrap_or(0),
            mode: self.prefs.mode,
            dark: self.dark(),
            system_available: self.system_dark.is_some() || lh_gui::appearance::system_dark().is_some(),
            direction: self.prefs.direction,
            column: self.layout().column,
            presets: self.layout().presets.clone(),
        }
    }

    /// The rows for what is typed now: every entry, and the indexes of those that match, best first.
    pub fn palette_rows(&self) -> (Vec<lh_gui::palette::Entry>, Vec<usize>) {
        let entries = lh_gui::palette::entries(&self.palette_snapshot());
        let query = self.palette.as_ref().map(|p| p.query.clone()).unwrap_or_default();
        let idx = lh_gui::palette::filter(&entries, &query);
        (entries, idx)
    }

    pub fn toggle_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.palette.is_some() {
            return self.close_palette(window, cx);
        }
        self.palette = Some(Default::default());
        self.settings_open = false;
        self.model_menu = false;
        self.palette_input.update(cx, |s, cx| {
            s.set_value("", window, cx);
            s.focus(window, cx);
        });
        cx.notify();
    }

    pub fn close_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.palette.take().is_some() {
            self.composer.update(cx, |s, cx| s.focus(window, cx));
            cx.notify();
        }
    }

    /// Move the chosen row (the Up and Down keys).
    pub fn palette_step(&mut self, down: bool) {
        let len = self.palette_rows().1.len();
        if let Some(p) = self.palette.as_mut() {
            p.step(down, len);
        }
    }

    pub fn run_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (entries, idx) = self.palette_rows();
        let Some(selected) = self.palette.as_ref().map(|p| p.selected) else { return };
        if let Some(e) = idx.get(selected).map(|&i| entries[i].clone()) {
            self.run_command(e.command, window, cx);
        }
    }

    pub fn run_command(&mut self, command: lh_gui::palette::Command, window: &mut Window, cx: &mut Context<Self>) {
        use lh_gui::palette::Command;
        self.close_palette(window, cx);
        match command {
            Command::NewChat => self.new_chat(cx),
            Command::Settings => self.open_settings(window, cx),
            Command::ToggleChatList => {
                self.layout_mut().toggle_left();
                self.persist();
            }
            Command::OpenTab(p) => self.show_in_dock(p, cx),
            Command::SwitchChat(k) => self.select_chat(k, cx),
            Command::Model(m) => self.choose_model(&m, cx),
            Command::Look(i) => {
                if let Some(f) = self.themes.families().get(i).cloned() {
                    self.set_family(&f, window, cx);
                }
            }
            Command::Mode(m) => self.set_appearance(m, window, cx),
            Command::ToggleMode => self.toggle_mode(window, cx),
            Command::Direction(d) => self.set_direction(d),
            Command::Width(w) => self.set_column(w, window, cx),
        }
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

    /// A change to memory (accept, reject, archive, pin, privacy); the lists are read again afterwards.
    pub fn memory_action(&mut self, path: String, body: Option<Value>, cx: &mut Context<Self>) {
        self.backend.send(Cmd::Call { tag: "memory-action".into(), method: "POST".into(), path, body });
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
