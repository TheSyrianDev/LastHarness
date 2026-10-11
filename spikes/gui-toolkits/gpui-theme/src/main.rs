//! Throwaway theming spike for GPUI (gpui-pre 0.3.8): blur and translucency, gradients, shadows, rounded
//! layered surfaces, animated buttons, TOML themes loaded and swapped live, a background image, a tray
//! icon, AccessKit roles, and a mirrored right-to-left layout. Plain sample content, no real data.
use gpui::prelude::*;
use gpui::*;
use gpui_component::input::{Input, InputState};
use gpui_component::Root;
use serde::Deserialize;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

#[derive(Deserialize, Clone, Default)]
#[serde(default)]
struct Theme {
    name: String,
    mode: String,
    colors: Colors,
    background: Bg,
    surface: Surface,
    animation: Anim,
}
#[derive(Deserialize, Clone, Default)]
#[serde(default)]
struct Colors { panel: String, text: String, accent: String, button: String, button_hover: String }
#[derive(Deserialize, Clone, Default)]
#[serde(default)]
struct Bg { kind: String, from: String, to: String, angle: f32, image: Option<String>, image_opacity: f32 }
#[derive(Deserialize, Clone, Default)]
#[serde(default)]
struct Surface { roundness: f32, match_window: bool, blur: bool, shadow_blur: f32, shadow_y: f32, shadow_color: String }
#[derive(Deserialize, Clone, Default)]
#[serde(default)]
struct Anim { duration_ms: u64 }

/// "#rrggbb" or "#rrggbbaa"
fn hex(s: &str) -> Hsla {
    let s = s.trim_start_matches('#');
    let v = u32::from_str_radix(s, 16).unwrap_or(0);
    let rgba = if s.len() == 8 { v } else { (v << 8) | 0xff };
    Rgba { r: ((rgba >> 24) & 0xff) as f32 / 255., g: ((rgba >> 16) & 0xff) as f32 / 255., b: ((rgba >> 8) & 0xff) as f32 / 255., a: (rgba & 0xff) as f32 / 255. }.into()
}
fn mix(a: Hsla, b: Hsla, t: f32) -> Hsla {
    let (a, b): (Rgba, Rgba) = (a.into(), b.into());
    Rgba { r: a.r + (b.r - a.r) * t, g: a.g + (b.g - a.g) * t, b: a.b + (b.b - a.b) * t, a: a.a + (b.a - a.a) * t }.into()
}

/// The compositor's window corner radius, where it can be read (Hyprland: decoration:rounding).
fn window_radius() -> Option<f32> {
    let o = std::process::Command::new("hyprctl").args(["getoption", "decoration:rounding", "-j"]).output().ok()?;
    let t = String::from_utf8_lossy(&o.stdout).to_string();
    t.split("\"int\":").nth(1)?.trim().split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()
}

/// A web page rendered by servo-helper in its own process; its PNG frames are shown as an image.
struct Embed { helper: PathBuf, url: String, out: PathBuf, child: Option<std::process::Child>, frame: Option<PathBuf>, n: u32, stamp: Option<SystemTime>, status: String, restarts: u32 }
impl Embed {
    fn spawn(&mut self) {
        let _ = std::fs::remove_file(&self.out);
        self.child = std::process::Command::new(&self.helper).args(["--url", &self.url, "--out", &self.out.to_string_lossy(), "--size", "900x500", "--every-ms", "500"]).stderr(std::process::Stdio::null()).spawn().ok();
        self.status = if self.child.is_some() { "helper started".into() } else { "helper failed to start".into() };
    }
    /// called from the poll loop: restart a dead helper, pick up a new frame
    fn tick(&mut self) -> bool {
        let mut changed = false;
        if let Some(c) = &mut self.child {
            if let Ok(Some(st)) = c.try_wait() { self.status = format!("helper exited ({st}); restarting"); self.child = None; self.restarts += 1; changed = true; }
        }
        if self.child.is_none() { self.spawn(); changed = true; }
        let m = std::fs::metadata(&self.out).and_then(|m| m.modified()).ok();
        if m.is_some() && m != self.stamp {
            self.stamp = m; self.n += 1;
            let copy = self.out.with_file_name(format!("embed-frame-{}.png", self.n));
            if std::fs::copy(&self.out, &copy).is_ok() { if let Some(old) = self.frame.replace(copy) { let _ = std::fs::remove_file(old); } self.status = format!("frame {} (restarts: {})", self.n, self.restarts); changed = true; }
        }
        changed
    }
}

struct Spike {
    embed: Option<Embed>,
    dir: PathBuf,
    themes: Vec<Theme>,
    stamp: Option<SystemTime>,
    active: usize,
    rtl: bool,
    hover: bool,
    clicks: u32,
    last_reload: String,
    input: Entity<InputState>,
}

impl Spike {
    fn load(dir: &PathBuf) -> (Vec<Theme>, Option<SystemTime>) {
        let mut files: Vec<_> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.extension().map_or(false, |x| x == "toml")).collect();
        files.sort();
        let mut newest = None;
        let themes = files.iter().filter_map(|p| {
            let m = std::fs::metadata(p).and_then(|m| m.modified()).ok();
            if m > newest { newest = m; }
            let mut t: Theme = toml::from_str(&std::fs::read_to_string(p).ok()?).ok()?;
            if let Some(i) = &t.background.image { t.background.image = Some(dir.parent().unwrap_or(dir).join(i).to_string_lossy().into()); }
            Some(t)
        }).collect();
        (themes, newest)
    }
    fn theme(&self) -> Theme { self.themes.get(self.active).cloned().unwrap_or_default() }
    fn apply_window(&self, window: &mut Window) {
        let t = self.theme();
        window.set_background_appearance(if t.surface.blur { WindowBackgroundAppearance::Blurred } else if t.background.kind == "gradient" && t.background.from.len() == 9 { WindowBackgroundAppearance::Transparent } else { WindowBackgroundAppearance::Opaque });
    }
}

impl Render for Spike {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = self.theme();
        let (text, accent) = (hex(&t.colors.text), hex(&t.colors.accent));
        let (panel, button, button_hover) = (hex(&t.colors.panel), hex(&t.colors.button), hex(&t.colors.button_hover));
        let outer = if t.surface.match_window { window_radius().unwrap_or(t.surface.roundness) } else { t.surface.roundness };
        let round = px(outer);
        // a control inside a panel gets the panel's radius minus its padding, so the curves stay parallel
        let inner = px((outer - 8.).max(3.));
        let shadow = vec![BoxShadow { color: hex(&t.surface.shadow_color), offset: point(px(0.), px(t.surface.shadow_y)), blur_radius: px(t.surface.shadow_blur), spread_radius: px(0.), inset: false }];
        let dur = Duration::from_millis(t.animation.duration_ms.max(1));
        let rtl = self.rtl;

        // sidebar: a list of themes, each a list item for the screen reader
        let mut side = div().id("themes").role(Role::List).aria_label("Themes").flex().flex_col().gap_1().w(px(210.)).p_2().bg(panel).rounded(round).shadow(shadow.clone())
            .child(div().text_color(text).child(if rtl { "السمات" } else { "Themes" }));
        for (i, th) in self.themes.iter().enumerate() {
            let on = i == self.active;
            side = side.child(div().id(("theme", i)).role(Role::ListItem).aria_label(th.name.clone()).aria_selected(on)
                .px_2().py_1().rounded(inner).text_color(text).bg(if on { accent.opacity(0.35) } else { button })
                .hover(move |s| s.bg(button_hover)).cursor_pointer()
                .on_click(cx.listener(move |this, _, window, cx| { this.active = i; this.apply_window(window); cx.notify(); }))
                .child(th.name.clone()));
        }

        // chat area: layered surfaces. Arabic paragraphs are right aligned, others left (start of the line).
        let bubble = |id: &'static str, s: &'static str, user: bool| {
            let is_rtl = s.chars().find(|c| c.is_alphabetic()).map_or(false, |c| (c as u32) >= 0x0590);
            let mut d = div().id(id).role(Role::Label).aria_label(s).max_w(px(520.)).px_3().py_2().rounded(round).text_color(text)
                .bg(if user { accent.opacity(0.4) } else { panel }).shadow(shadow.clone()).child(s);
            if is_rtl { d = d.text_right(); }
            div().flex().w_full().when(user != rtl, |d| d.justify_end()).child(d)
        };
        let chat = div().flex().flex_col().gap_2().flex_1().p_2().child(bubble("m1", "Hello. This is sample text.", true))
            .child(bubble("m2", "مرحبا بالعالم! هذا نص تجريبي بالعربية.", false))
            .child(bubble("m3", "Mixed: النص العربي مع English في نفس السطر.", false));

        // the animated button: fades between two fills on hover, flashes on click (animation restarts when its id changes)
        let hover = self.hover;
        let clicks = self.clicks;
        let (from, to) = if hover { (button, button_hover) } else { (button_hover, button) };
        let send = div().id("send").role(Role::Button).aria_label("Send").px_4().py_2().rounded(inner).text_color(text).cursor_pointer()
            .bg(if hover { button_hover } else { button })
            .on_hover(cx.listener(|this, h: &bool, _, cx| { this.hover = *h; cx.notify(); }))
            .on_click(cx.listener(|this, _, _, cx| { this.clicks += 1; cx.notify(); }))
            .child(if rtl { "إرسال" } else { "Send" })
            .with_animation(("send-fade", (hover as u32) + clicks * 2), Animation::new(dur).with_easing(ease_in_out), move |el, d| {
                let flash = if clicks > 0 { mix(accent, mix(from, to, d), d) } else { mix(from, to, d) };
                el.bg(flash).shadow(vec![BoxShadow { color: accent.opacity(0.6 * (1. - d)), offset: point(px(0.), px(0.)), blur_radius: px(14. * (1. - d)), spread_radius: px(2. * (1. - d)), inset: false }])
            });
        let toggle = div().id("rtl").role(Role::Button).aria_label("Mirror layout").px_3().py_2().rounded(inner).text_color(text).bg(button).cursor_pointer()
            .hover(move |s| s.bg(button_hover))
            .on_click(cx.listener(|this, _, _, cx| { this.rtl = !this.rtl; cx.notify(); }))
            .child(if rtl { "LTR" } else { "RTL" });

        let bar = div().flex().when(rtl, |d| d.flex_row_reverse()).gap_2().items_center().p_2().bg(panel).rounded(round).shadow(shadow.clone())
            .child(div().flex_1().child(Input::new(&self.input))).child(send).child(toggle);
        let pane = match &self.embed {
            Some(e) => div().flex_1().flex().flex_col().gap_1().child(div().text_sm().text_color(text.opacity(0.8)).child(format!("web page in a separate process: {}", e.status)))
                .child(match &e.frame { Some(f) => div().flex_1().child(img(f.clone()).w_full().h_full().object_fit(ObjectFit::Contain)), None => div().flex_1() })
                .into_any_element(),
            None => chat.into_any_element(),
        };
        let main = div().flex().flex_col().flex_1().gap_2().child(pane).child(bar)
            .child(div().text_sm().text_color(text.opacity(0.7)).child(format!("theme: {}   {}   reloaded: {}", t.name, if t.surface.blur { "blur requested" } else { "no blur" }, self.last_reload)));

        // background: image and/or gradient, behind everything
        let mut root = div().size_full().relative().overflow_hidden().text_color(text);
        root = match t.background.kind.as_str() {
            "gradient" => root.bg(linear_gradient(t.background.angle, linear_color_stop(hex(&t.background.from), 0.), linear_color_stop(hex(&t.background.to), 1.))),
            _ => root.bg(hex(&t.background.from)),
        };
        if let Some(path) = &t.background.image {
            root = root.child(img(PathBuf::from(path)).absolute().top_0().left_0().size_full().object_fit(ObjectFit::Cover).opacity(t.background.image_opacity));
        }
        let _ = window;
        root.child(div().size_full().flex().when(rtl, |d| d.flex_row_reverse()).gap_3().p_3().child(side).child(main))
    }
}

struct TrayIcon;
impl ksni::Tray for TrayIcon {
    fn id(&self) -> String { "gt-gpui-theme".into() }
    fn title(&self) -> String { "gt-gpui-theme".into() }
    fn icon_name(&self) -> String { "applications-system".into() }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        vec![StandardItem { label: "Show".into(), ..Default::default() }.into(), StandardItem { label: "Quit".into(), activate: Box::new(|_| std::process::exit(0)), ..Default::default() }.into()]
    }
}

fn arg(name: &str) -> Option<String> {
    let mut a = std::env::args().skip(1);
    while let Some(x) = a.next() { if x == name { return a.next(); } }
    None
}

fn main() {
    let dir = PathBuf::from(arg("--themes").unwrap_or_else(|| "themes".into()));
    let start = arg("--theme").and_then(|n| n.parse::<usize>().ok()).unwrap_or(0);
    if arg("--tray").as_deref() == Some("1") {
        use ksni::blocking::TrayMethods;
        match TrayIcon.spawn() {
            Ok(h) => { eprintln!("tray: registered"); std::mem::forget(h); }
            Err(e) => eprintln!("tray: failed: {e}"),
        }
    }
    gpui_platform::application().run(move |cx| {
        gpui_component::init(cx);
        let opts = WindowOptions { window_background: WindowBackgroundAppearance::Transparent, ..Default::default() };
        cx.open_window(opts, |window, cx| {
            let input = cx.new(|cx| InputState::new(window, cx).placeholder("Message"));
            let (themes, stamp) = Spike::load(&dir);
            let view = cx.new(|cx| {
                // poll the theme folder; a changed file is read again and shown at once
                cx.spawn(async move |this: WeakEntity<Spike>, cx: &mut AsyncApp| loop {
                    cx.background_executor().timer(Duration::from_millis(400)).await;
                    let ok = this.update(cx, |s, cx| {
                        if let Some(e) = &mut s.embed { if e.tick() { cx.notify(); } }
                        let (themes, stamp) = Spike::load(&s.dir);
                        if stamp != s.stamp && !themes.is_empty() {
                            let name = s.theme().name;
                            s.active = themes.iter().position(|t| t.name == name).unwrap_or(0).min(themes.len() - 1);
                            s.themes = themes;
                            s.stamp = stamp;
                            s.last_reload = format!("{:?}", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_secs() % 100000).unwrap_or(0));
                            cx.notify();
                        }
                    });
                    if ok.is_err() { break; }
                }).detach();
                Spike { embed: arg("--embed").map(|url| Embed { helper: PathBuf::from(arg("--helper").unwrap_or_else(|| "../servo/target/release/servo-helper".into())), url, out: std::env::temp_dir().join("gt-embed.png"), child: None, frame: None, n: 0, stamp: None, status: "starting".into(), restarts: 0 }), dir: dir.clone(), active: start.min(themes.len().saturating_sub(1)), themes, stamp, rtl: arg("--rtl").as_deref() == Some("1"), hover: false, clicks: 0, last_reload: "-".into(), input: input.clone() }
            });
            view.update(cx, |s, _| s.apply_window(window));
            input.update(cx, |s, cx| s.focus(window, cx));
            cx.new(|cx| Root::new(view, window, cx))
        }).unwrap();
        cx.activate(true);
    });
}
