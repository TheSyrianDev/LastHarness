use gui_common::*;
use slint::{ComponentHandle, Model, ModelRc, SharedString, StandardListViewItem, VecModel};
use std::sync::{mpsc, Arc, Mutex};

slint::include_modules!();

fn main() {
    let cfg = load_cfg();
    let ui = App::new().unwrap();
    let (tx, rx) = mpsc::channel::<Ev>();
    let rx = Arc::new(Mutex::new(rx));
    let weak = ui.as_weak();
    let wake: Wake = {
        let rx = rx.clone();
        Arc::new(move || {
            let (rx, weak) = (rx.clone(), weak.clone());
            let _ = slint::invoke_from_event_loop(move || {
                let Some(ui) = weak.upgrade() else { return };
                while let Ok(ev) = rx.lock().unwrap().try_recv() {
                    match ev {
                        Ev::Models(m) => ui.set_models(ModelRc::new(VecModel::from(m.into_iter().map(|s| StandardListViewItem::from(SharedString::from(s))).collect::<Vec<_>>()))),
                        Ev::Text(t) => ui.set_chat(format!("{}{}", ui.get_chat(), t).into()),
                        Ev::Done => ui.set_chat(format!("{}\n", ui.get_chat()).into()),
                        Ev::Error(e) => ui.set_chat(format!("{}\n[error] {e}\n", ui.get_chat()).into()),
                    }
                }
            });
        })
    };
    spawn_models(cfg.clone(), tx.clone(), wake.clone());
    let session = Arc::new(Mutex::new(None));
    let w = ui.as_weak();
    ui.on_send(move |text| {
        let text = text.to_string();
        if text.trim().is_empty() { return; }
        let ui = w.upgrade().unwrap();
        ui.set_chat(format!("{}> {text}\n", ui.get_chat()).into());
        let model = ui.get_models().row_data(ui.get_selected().max(0) as usize).map(|i| i.text.to_string()).unwrap_or_default();
        spawn_turn(cfg.clone(), session.clone(), model, text, tx.clone(), wake.clone());
    });
    ui.run().unwrap();
}
