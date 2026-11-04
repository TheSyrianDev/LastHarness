//! The desktop app: a thin front end for the core, drawn with GPUI.

mod app;
mod page_ui;
mod extra_ui;
mod pages_ui;
mod panels;
mod style;
mod tray;
mod ui;
mod width;

use lh_gui::config;
use gpui::prelude::*;
use gpui::*;
use gpui_component::Root;

fn main() {
    lh_config::platform_dirs();
    let cfg = match config::Config::from_args(std::env::args().skip(1)) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("lastharness-gui: {e}");
            std::process::exit(2);
        }
    };
    lh_gui::config::start_core_if_missing(&cfg.base, &cfg.token_file);
    tray::spawn();
    gpui_platform::application().run(move |cx| {
        gpui_component::init(cx);
        app::App::load_user_fonts(cx);
        ui::bind_keys(cx);
        let opts = WindowOptions {
            window_background: WindowBackgroundAppearance::Transparent,
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, size(px(1180.), px(760.)), cx))),
            titlebar: Some(TitlebarOptions { title: Some("LastHarness".into()), ..Default::default() }),
            app_id: Some("lastharness-gui".into()),
            ..Default::default()
        };
        cx.open_window(opts, move |window, cx| {
            let view = cx.new(|cx| app::App::new(cfg, window, cx));
            view.update(cx, |a, _| a.apply_window(window));
            cx.new(|cx| Root::new(view, window, cx))
        })
        .expect("could not open the window");
        cx.activate(true);
    });
}
