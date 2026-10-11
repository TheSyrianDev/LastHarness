use eframe::egui;
use gui_common::*;
use std::sync::{mpsc, Arc, Mutex};

struct App {
    cfg: Cfg,
    rx: mpsc::Receiver<Ev>,
    tx: mpsc::Sender<Ev>,
    wake: Wake,
    models: Vec<String>,
    selected: usize,
    chat: String,
    input: String,
    busy: bool,
    session: Arc<Mutex<Option<String>>>,
    first: bool,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        while let Ok(ev) = self.rx.try_recv() {
            match ev {
                Ev::Models(m) => self.models = m,
                Ev::Text(t) => self.chat.push_str(&t),
                Ev::Done => { self.busy = false; self.chat.push('\n'); }
                Ev::Error(e) => { self.busy = false; self.chat.push_str(&format!("\n[error] {e}\n")); }
            }
        }
        egui::Panel::left("side").show(ui, |ui| {
            ui.heading("Models");
            for (i, m) in self.models.iter().enumerate() {
                if ui.selectable_label(self.selected == i, m).clicked() { self.selected = i; }
            }
        });
        egui::Panel::bottom("input").show(ui, |ui| {
            ui.horizontal(|ui| {
                let r = ui.add(egui::TextEdit::singleline(&mut self.input).desired_width(ui.available_width() - 70.0).hint_text("Message"));
                if std::mem::take(&mut self.first) { r.request_focus(); }
                if std::mem::take(&mut self.first) { r.request_focus(); }
                let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if (ui.button("Send").clicked() || enter) && !self.busy && !self.input.trim().is_empty() {
                    let text = std::mem::take(&mut self.input);
                    self.chat.push_str(&format!("> {text}\n"));
                    self.busy = true;
                    let model = self.models.get(self.selected).cloned().unwrap_or_default();
                    spawn_turn(self.cfg.clone(), self.session.clone(), model, text, self.tx.clone(), self.wake.clone());
                    r.request_focus();
                }
            });
        });
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().stick_to_bottom(true).auto_shrink([false, false]).show(ui, |ui| {
                ui.add(egui::Label::new(&self.chat).wrap());
            });
        });
    }
}

fn main() -> eframe::Result {
    let cfg = load_cfg();
    let opts = eframe::NativeOptions { viewport: egui::ViewportBuilder::default().with_inner_size([900.0, 600.0]).with_app_id("gt-egui"), ..Default::default() };
    eframe::run_native("gt-egui", opts, Box::new(move |cc| {
        let mut fonts = egui::FontDefinitions::default();
        if let Ok(b) = std::fs::read("/usr/share/fonts/noto/NotoSansArabic-Regular.ttf") {
            fonts.font_data.insert("arabic".into(), Arc::new(egui::FontData::from_owned(b)));
            fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().push("arabic".into());
        }
        cc.egui_ctx.set_fonts(fonts);
        let (tx, rx) = mpsc::channel();
        let ctx = cc.egui_ctx.clone();
        let wake: Wake = Arc::new(move || ctx.request_repaint());
        spawn_models(cfg.clone(), tx.clone(), wake.clone());
        Ok(Box::new(App { cfg, rx, tx, wake, models: vec![], selected: 0, chat: String::new(), input: String::new(), busy: false, session: Arc::new(Mutex::new(None)), first: true }))
    }))
}
