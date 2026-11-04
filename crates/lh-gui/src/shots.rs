//! Offscreen pictures of whole screens, for design review. Nothing is shown on screen and the mouse is not touched:
//! the window is a test window drawn by the offscreen renderer. The core is not running, so each screen is set up by
//! hand. Run with `LH_SCREEN_SHOTS=<dir> cargo test -p lh-gui --bin lastharness-gui screen_shots -- --ignored`.

use super::*;
use std::sync::Arc;
use gpui_component::Root;
use lh_gui::content::{Content, ContentKind};
use lh_gui::setting_names::Category;

fn cfg(dir: &std::path::Path) -> Config {
    Config { base: "http://127.0.0.1:9".into(), token_file: dir.join("token"), prefs: dir.join("prefs.toml"), themes: dir.join("themes") }
}

fn shown(title: &str, kind: ContentKind, body: &str) -> Block {
    Block::Shown(Content { title: title.into(), kind, body: body.into(), path: None })
}

fn fill_chat(app: &mut App) {
    app.connected = true;
    app.banner = None;
    app.models = ModelList::new(vec!["Qwen3.6-35B-A3B".into(), "Tiel-Coder-7B".into(), "Ornith-14B".into()]);
    // the second look runs this again; the chats are already there
    if app.chats.list.iter().any(|c| c.session.as_deref() == Some("s0")) {
        return;
    }
    for (i, t) in ["Plan the garden beds", "Tax letter draft", "Why is the build slow", "Weekend trip ideas"].iter().enumerate() {
        app.chats.add_stored(&format!("s{i}"), t, "Qwen3.6-35B-A3B");
        if let Some(c) = app.chats.list.last_mut() {
            c.last = lh_gui::model::unix_now() - (i as i64 + 1) * 5000;
        }
    }
}

fn messages(app: &mut App) {
    app.banner = None;
    // open the stored garden chat rather than a second one with the same name
    let key = app.chats.by_session("s0").map(|c| c.key).unwrap();
    app.chats.select(key);
    let c = app.chats.current_mut().unwrap();
    c.dormant = false;
    c.blocks = vec![
        Block::User("How much did each bed yield last year? A table and a chart please.".into()),
        Block::Assistant { text: "Here is what the notes say. The **tomato bed** did best by far; the herbs were steady.\n\n- Tomatoes: 42 kg\n- Beans: 18 kg\n- Herbs: 6 kg".into(), reasoning: "Looking at the notes.".into(), worked: Some(7) },
        Block::Tool { id: "t1".into(), name: "read_file".into(), arguments: "{\"path\":\"~/garden/notes.md\"}".into(), result: Some("ok".into()) },
        shown("Yield by bed", ContentKind::Table, r#"{"columns":["Bed","Crop","Kg","Hours"],"rows":[["North","Tomatoes",42,30],["East","Beans",18,12],["South","Herbs",6,5]],"caption":"Season 2026"}"#),
        shown("Kilograms by bed", ContentKind::Chart, r#"{"type":"bar","labels":["Tomatoes","Beans","Herbs"],"series":[{"name":"Kg","values":[42,18,6]}]}"#),
        Block::User("Thanks. Which bed should get more sun?".into()),
        Block::Assistant { text: "The east bed. Beans like sun and it is shaded after noon.".into(), reasoning: String::new(), worked: None },
    ];
    c.last = lh_gui::model::unix_now();
}

#[::core::prelude::v1::test]
#[ignore]
fn screen_shots() {
    let Some(out) = std::env::var_os("LH_SCREEN_SHOTS").map(std::path::PathBuf::from) else { return };
    std::fs::create_dir_all(&out).unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let text = Arc::new(gpui_wgpu::CosmicTextSystem::new("IBM Plex Sans"));
    let mut cx = gpui::HeadlessAppContext::with_platform(text, Arc::new(()), gpui_platform::current_headless_renderer);
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::ui::bind_keys(cx);
    });
    let c = cfg(tmp.path());
    let mut app_entity = None;
    let win = cx
        .open_window(size(px(1180.), px(760.)), |window, cx| {
            let view = cx.new(|cx| App::new(c, window, cx));
            app_entity = Some(view.clone());
            cx.new(|cx| Root::new(view, window, cx))
        })
        .unwrap();
    let app = app_entity.unwrap();
    let any: AnyWindowHandle = win.into();

    let mut snap = |cx: &mut gpui::HeadlessAppContext, name: &str, set: &dyn Fn(&mut App, &mut Window, &mut Context<App>)| {
        cx.update_window(any, |_, window, cx| app.update(cx, |a, cx| set(a, window, cx))).unwrap();
        // two draws: the first lays out, the second has the measured sizes; reveals are let finish in between
        for _ in 0..2 {
            cx.run_until_parked();
            std::thread::sleep(Duration::from_millis(300));
            cx.update_window(any, |_, window, cx| { let t = window.draw(cx); t.clear(cx) }).unwrap();
        }
        let img = cx.capture_screenshot(any).unwrap();
        img.save(out.join(format!("{name}.png"))).unwrap();
    };

    let look = |name: &'static str| move |a: &mut App, w: &mut Window, cx: &mut Context<App>| {
        a.prefs.theme = name.into();
        a.reapply_look(w, cx);
    };
    for theme in ["Calm", "Glass"] {
        let t = theme.to_lowercase();
        snap(&mut cx, &format!("{t}-01-away"), &|a, w, cx| { look(theme)(a, w, cx); });
        snap(&mut cx, &format!("{t}-02-greeting"), &|a, _, _| fill_chat(a));
        snap(&mut cx, &format!("{t}-03-chat"), &|a, _, _| messages(a));
        snap(&mut cx, &format!("{t}-04-panel-work"), &|a, _, _| {
            a.layout_mut().right_visible = true;
            a.layout_mut().panel = Panel::Work;
        });
        snap(&mut cx, &format!("{t}-05-panel-depts"), &|a, _, _| a.layout_mut().panel = Panel::Departments);
        snap(&mut cx, &format!("{t}-06-model-menu"), &|a, _, _| {
            a.layout_mut().right_visible = false;
            a.model_menu = true;
        });
        snap(&mut cx, &format!("{t}-07-slider"), &|a, _, _| {
            a.model_menu = false;
            a.slider_open = true;
        });
        snap(&mut cx, &format!("{t}-08-folded"), &|a, _, _| {
            a.slider_open = false;
            a.layout_mut().left_visible = false;
        });
        for (i, cat) in Category::ALL.iter().enumerate() {
            let cat = *cat;
            snap(&mut cx, &format!("{t}-s{i:02}-{cat:?}"), &move |a, w, cx| {
                a.layout_mut().left_visible = true;
                a.open_settings(w, cx);
                a.settings_page = cat;
            });
        }
        snap(&mut cx, &format!("{t}-09-back"), &|a, _, _| {
            a.settings_open = false;
            a.chats.new_chat("Qwen3.6-35B-A3B");
        });
    }
    // the Glass look tinted around two picked colours, in a chat and on the Appearance page
    snap(&mut cx, "glass-10-tinted-chat", &|a, w, cx| {
        a.settings_open = false;
        messages(a);
        a.prefs.glass_colours = vec![lh_gui::theme::field_colour(14, 2), lh_gui::theme::field_colour(20, 3)];
        a.prefs.glass_opacity = Some(0.8);
        look("Glass")(a, w, cx);
    });
    snap(&mut cx, "glass-11-tinted-appearance", &|a, w, cx| {
        a.open_settings(w, cx);
        a.settings_page = Category::Appearance;
    });
}
