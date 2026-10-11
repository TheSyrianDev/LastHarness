use gui_common::*;
use iced::futures::{channel::mpsc as fmpsc, SinkExt, Stream};
use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length, Subscription, Task};
use std::sync::{mpsc, Arc, Mutex, OnceLock};

static BRIDGE: OnceLock<Mutex<Option<fmpsc::UnboundedReceiver<Ev>>>> = OnceLock::new();
static CFG: OnceLock<Cfg> = OnceLock::new();
const INPUT: &str = "input";

#[derive(Debug, Clone)]
enum Msg {
    Ev(Arc<Ev>),
    Input(String),
    Send,
    Pick(usize),
}

struct App {
    models: Vec<String>,
    selected: usize,
    chat: String,
    input: String,
    tx: mpsc::Sender<Ev>,
    wake: Wake,
    session: Arc<Mutex<Option<String>>>,
}

fn events() -> impl Stream<Item = Msg> {
    iced::stream::channel(100, |mut out: fmpsc::Sender<Msg>| async move {
        let mut rx = BRIDGE.get().unwrap().lock().unwrap().take().unwrap();
        use iced::futures::StreamExt;
        while let Some(ev) = rx.next().await {
            let _ = out.send(Msg::Ev(Arc::new(ev))).await;
        }
    })
}

impl App {
    fn new() -> (Self, Task<Msg>) {
        let cfg = CFG.get().unwrap().clone();
        let (tx, rx) = mpsc::channel::<Ev>();
        let (ftx, frx) = fmpsc::unbounded::<Ev>();
        BRIDGE.set(Mutex::new(Some(frx))).ok();
        // std channel -> async channel, so the UI sleeps until there is something to show
        std::thread::spawn(move || { while let Ok(ev) = rx.recv() { if ftx.unbounded_send(ev).is_err() { break; } } });
        let wake: Wake = Arc::new(|| {});
        spawn_models(cfg, tx.clone(), wake.clone());
        (App { models: vec![], selected: 0, chat: String::new(), input: String::new(), tx, wake, session: Arc::new(Mutex::new(None)) }, iced::widget::operation::focus(INPUT))
    }

    fn update(&mut self, m: Msg) -> Task<Msg> {
        match m {
            Msg::Ev(ev) => match &*ev {
                Ev::Models(v) => self.models = v.clone(),
                Ev::Text(t) => self.chat.push_str(t),
                Ev::Done => self.chat.push('\n'),
                Ev::Error(e) => self.chat.push_str(&format!("\n[error] {e}\n")),
            },
            Msg::Input(s) => self.input = s,
            Msg::Pick(i) => self.selected = i,
            Msg::Send => {
                let t = std::mem::take(&mut self.input);
                if !t.trim().is_empty() {
                    self.chat.push_str(&format!("> {t}\n"));
                    let model = self.models.get(self.selected).cloned().unwrap_or_default();
                    spawn_turn(CFG.get().unwrap().clone(), self.session.clone(), model, t, self.tx.clone(), self.wake.clone());
                }
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Msg> {
        let side = self.models.iter().enumerate().fold(column![text("Models")].spacing(4), |c, (i, m)| {
            c.push(button(text(m.as_str())).on_press(Msg::Pick(i)).width(Length::Fill).style(if i == self.selected { button::primary } else { button::text }))
        });
        let chat = scrollable(text(self.chat.as_str()).width(Length::Fill)).anchor_bottom().height(Length::Fill).width(Length::Fill);
        let input = row![text_input("Message", &self.input).id(INPUT).on_input(Msg::Input).on_submit(Msg::Send), button("Send").on_press(Msg::Send)].spacing(4);
        row![container(side).width(200), column![chat, input].spacing(4)].spacing(8).padding(4).into()
    }

    fn subscription(&self) -> Subscription<Msg> {
        Subscription::run(events)
    }
}

fn main() -> iced::Result {
    CFG.set(load_cfg()).ok();
    iced::application(App::new, App::update, App::view)
        .subscription(App::subscription)
        .title("gt-iced")
        .window_size((900.0, 600.0))
        .run()
}
