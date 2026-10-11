use futures::{channel::mpsc as fmpsc, StreamExt};
use gpui::*;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{button::*, Root};
use gui_common::chat::*;
use gui_common::{load_cfg, Cfg, Wake};
use std::sync::{mpsc, Arc, Mutex};

struct ChatData {
    id: u64,
    title: String,
    shared: Shared,
    model: String,
    text: String,
    turn: Option<Turn>,
}

struct App {
    cfg: Cfg,
    models: Vec<String>,
    chats: Vec<ChatData>,
    active: usize,
    next_id: u64,
    path: String,
    input: Entity<InputState>,
    tx: mpsc::Sender<CEv>,
    wake: Wake,
    scroll: ScrollHandle,
}

impl App {
    fn saved(&self) -> Vec<Saved> {
        self.chats.iter().map(|c| Saved { id: c.id, title: c.title.clone(), session: c.shared.lock().unwrap().id.clone(), model: c.model.clone(), text: c.text.clone() }).collect()
    }
    fn persist(&self) { save(&self.path, &self.saved()); }

    fn new_chat(&mut self) {
        let model = self.chats.get(self.active).map(|c| c.model.clone()).or(self.models.first().cloned()).unwrap_or_default();
        self.chats.push(ChatData { id: self.next_id, title: "New chat".into(), shared: Default::default(), model, text: String::new(), turn: None });
        self.next_id += 1;
        self.active = self.chats.len() - 1;
        self.persist();
    }

    fn delete_chat(&mut self, id: u64) {
        let Some(i) = self.chats.iter().position(|c| c.id == id) else { return };
        let c = self.chats.remove(i);
        if let Some(t) = &c.turn { t.stop(); }
        if let Some(sid) = c.shared.lock().unwrap().id.clone() { delete_session(self.cfg.clone(), sid); }
        if self.chats.is_empty() { self.new_chat(); }
        self.active = self.active.min(self.chats.len() - 1);
        if i < self.active { self.active -= 1; }
        self.persist();
    }

    fn send_or_stop(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(t) = &self.chats[self.active].turn { t.stop(); return; }
        let text = self.input.read(cx).value().to_string();
        if text.trim().is_empty() { return; }
        self.input.update(cx, |s, cx| s.set_value("", window, cx));
        let (cfg, tx, wake) = (self.cfg.clone(), self.tx.clone(), self.wake.clone());
        let c = &mut self.chats[self.active];
        if c.title == "New chat" { c.title = text.chars().take(28).collect(); }
        c.text.push_str(&format!("> {text}\n"));
        c.turn = Some(start_turn(cfg, c.shared.clone(), c.id, c.model.clone(), text, tx, wake));
        self.persist();
        cx.notify();
    }
}

impl Render for App {
    fn render(&mut self, _w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let busy = self.chats[self.active].turn.is_some();
        let mut side = div().flex().flex_col().w(px(240.)).gap_1()
            .child(Button::new("new").label("New chat").on_click(cx.listener(|this, _, _, cx| { this.new_chat(); cx.notify(); })));
        for (i, c) in self.chats.iter().enumerate() {
            let id = c.id;
            let mut pick = Button::new(("c", id)).label(c.title.clone()).w_full().on_click(cx.listener(move |this, _, _, cx| { this.active = i; cx.notify(); }));
            if i == self.active { pick = pick.primary(); }
            side = side.child(div().flex().flex_row().gap_1().child(div().flex_1().child(pick))
                .child(Button::new(("d", id)).label("x").on_click(cx.listener(move |this, _, _, cx| { this.delete_chat(id); cx.notify(); }))));
        }
        side = side.child(div().mt_2().child("Model for this chat"));
        let cur = self.chats[self.active].model.clone();
        for (i, m) in self.models.iter().enumerate() {
            let name = m.clone();
            let mut b = Button::new(("m", i)).label(m.clone()).w_full().on_click(cx.listener(move |this, _, _, cx| {
                this.chats[this.active].model = name.clone();
                this.persist();
                cx.notify();
            }));
            if *m == cur { b = b.primary(); }
            side = side.child(b);
        }
        let c = &self.chats[self.active];
        let mut action = Button::new("act").label(if busy { "Stop" } else { "Send" }).on_click(cx.listener(|this, _, w, cx| this.send_or_stop(w, cx)));
        if busy { action = action.danger(); }
        div().flex().flex_row().size_full().p_1().gap_2().child(side).child(
            div().flex().flex_col().flex_1().gap_1()
                .child(div().child(format!("Chat: {}   Model: {}", c.title, c.model)))
                .child(div().id("chat").flex_1().overflow_y_scroll().track_scroll(&self.scroll).child(c.text.clone()))
                .child(div().flex().flex_row().gap_1().child(div().flex_1().child(Input::new(&self.input))).child(action)),
        )
    }
}

fn main() {
    let cfg = load_cfg();
    Application::new().run(move |cx| {
        gpui_component::init(cx);
        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let (tx, rx) = mpsc::channel::<CEv>();
                let (ftx, mut frx) = fmpsc::unbounded::<CEv>();
                std::thread::spawn(move || { while let Ok(ev) = rx.recv() { if ftx.unbounded_send(ev).is_err() { break; } } });
                let input = cx.new(|cx| InputState::new(window, cx).placeholder("Message"));
                let wake: Wake = Arc::new(|| {});
                let path = state_path();
                let view = cx.new(|cx| {
                    cx.subscribe_in(&input, window, |this: &mut App, _, ev: &InputEvent, w, cx| {
                        if let InputEvent::PressEnter { .. } = ev { this.send_or_stop(w, cx); }
                    }).detach();
                    gui_common::chat::spawn_models(cfg.clone(), tx.clone(), wake.clone());
                    let mut chats: Vec<ChatData> = load_saved(&path).into_iter().map(|s| {
                        if let Some(id) = &s.session { check_session(cfg.clone(), s.id, id.clone(), tx.clone(), wake.clone()); }
                        let shared: Shared = Arc::new(Mutex::new(Sess { id: s.session.clone(), model_id: String::new() }));
                        ChatData { id: s.id, title: s.title, shared, model: s.model, text: s.text, turn: None }
                    }).collect();
                    let next_id = chats.iter().map(|c| c.id).max().unwrap_or(0) + 1;
                    let mut app = App { cfg: cfg.clone(), models: vec![], chats: vec![], active: 0, next_id, path, input: input.clone(), tx: tx.clone(), wake: wake.clone(), scroll: ScrollHandle::new() };
                    app.chats.append(&mut chats);
                    if app.chats.is_empty() { app.new_chat(); }
                    app.active = app.chats.len() - 1;
                    app
                });
                let v = view.clone();
                window.spawn(cx, async move |cx| {
                    while let Some(ev) = frx.next().await {
                        let _ = v.update(cx, |a, cx| {
                            match ev {
                                CEv::Models(m) => {
                                    for c in a.chats.iter_mut() { if c.model.is_empty() || !m.contains(&c.model) { c.model = m.first().cloned().unwrap_or_default(); } }
                                    a.models = m;
                                }
                                CEv::Text { chat, text } => if let Some(c) = a.chats.iter_mut().find(|c| c.id == chat) { c.text.push_str(&text) },
                                CEv::Done { chat } => {
                                    if let Some(c) = a.chats.iter_mut().find(|c| c.id == chat) { c.turn = None; c.text.push('\n'); }
                                    a.persist();
                                }
                                CEv::Failed { chat, err } => {
                                    if let Some(c) = a.chats.iter_mut().find(|c| c.id == chat) { c.turn = None; c.text.push_str(&format!("\n[error] {err}\n")); }
                                }
                                CEv::Alive { chat, alive } => if !alive {
                                    if let Some(c) = a.chats.iter_mut().find(|c| c.id == chat) {
                                        c.shared.lock().unwrap().id = None;
                                        c.text.push_str("[the core restarted and no longer remembers this chat; the text is kept, but the model starts fresh]\n");
                                    }
                                },
                            }
                            let max = a.scroll.max_offset();
                            a.scroll.set_offset(point(px(0.), -max.height));
                            cx.notify();
                        });
                    }
                }).detach();
                input.update(cx, |s, cx| s.focus(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            }).unwrap();
        }).detach();
    });
}
