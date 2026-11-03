//! Web pages inside the Opened tab. The page is drawn by the `lh-pageview` helper process into a picture the size
//! of the panel; this file puts that picture on the screen, turns the wheel, the keys and clicks into calls on it,
//! and shows the address of a clicked link in a small bar. Nothing here fetches anything and nothing is opened
//! without a click on a button: see `lh-pageview` for the rules the engine keeps.
//!
//! Calls to the helper can take up to their time limit, so they never run on the window's thread: one task at a time
//! takes the page out of its slot, does everything that was asked for since (resize, scroll, paint, point at a link)
//! and puts it back with the answers.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use gpui::prelude::*;
use gpui::*;
use lh_pageview::debounce::Debounce;
use lh_pageview::scroll::{self, Step};
use lh_pageview::{openable, Fallback, Frame, Info, Link, Options, PageView};

use crate::app::App;
use lh_gui::content::{Content, ContentKind};
use crate::ui::Kind;

/// What was asked of the page since the helper was last busy.
#[derive(Default)]
struct Want {
    resize: Option<(u32, u32, f32)>,
    scroll_to: Option<f32>,
    render: bool,
    link: Option<(f32, f32)>,
    /// Pointer and keys for the page, in order: (action, x, y) or a key and its text.
    input: Vec<Input>,
    /// What the app fetched for the page from allowed sites, to hand over: (request, status, type, body).
    deliver: Vec<(u32, lh_gui::page_fetch::Fetched)>,
}

enum Input {
    Pointer(u8, f32, f32),
    Key(String, String),
}

impl Want {
    fn any(&self) -> bool {
        self.resize.is_some() || self.scroll_to.is_some() || self.render || self.link.is_some() || !self.input.is_empty() || !self.deliver.is_empty()
    }
}

/// The box the picture goes in, written by the panel as it is laid out: size and top left corner in window
/// coordinates, both in logical pixels.
#[derive(Clone, Copy, Default)]
struct Measured {
    w: f32,
    h: f32,
    x: f32,
    y: f32,
}

#[derive(Default)]
struct Answers {
    frame: Option<Frame>,
    info: Option<Info>,
    link: Option<Option<Link>>,
    refused: u32,
    failed: Option<Fallback>,
    /// Pointer or keys were passed on: the page's scripts may still be changing it.
    had_input: bool,
    follow_up: Option<String>,
    fetches: Vec<(u32, String)>,
}

pub struct PageState {
    key: u64,
    /// The page's source, and where it is on screen: "panel", or the chat message it belongs to.
    html: String,
    pub place: String,
    /// The pointer is held down inside the page, so moves are passed on (a slider being dragged).
    pressed: bool,
    view: Option<PageView>,
    opening: bool,
    busy: bool,
    want: Want,
    debounce: Debounce,
    measured: Arc<Mutex<Measured>>,
    scale: f32,
    image: Option<Arc<RenderImage>>,
    info: Option<Info>,
    focus: FocusHandle,
    /// Why the page is not drawn inside the app, when it is not.
    pub fallback: Option<Fallback>,
    /// The address of the link that was clicked last, shown in the bar under the page.
    link: Option<Link>,
    refused: u32,
    /// A follow-up the page offered, shown as a chip under it until it is clicked or dismissed.
    follow_up: Option<String>,
}

/// A number that says which content this is, so a different page starts a new view and the same one keeps its own.
fn key_of(item: &Content) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    item.title.hash(&mut h);
    item.body.hash(&mut h);
    item.path.hash(&mut h);
    h.finish()
}

/// The picture for the toolkit: blue, green, red, alpha.
fn to_image(f: Frame) -> Option<Arc<RenderImage>> {
    let mut px = f.rgba;
    for p in px.as_chunks_mut::<4>().0 {
        p.swap(0, 2);
    }
    let buf = image::RgbaImage::from_raw(f.width, f.height, px)?;
    Some(Arc::new(RenderImage::new(vec![image::Frame::new(buf)])))
}

impl App {
    /// Should this content be drawn by the engine? Only a web page, and only if the person has not turned it off.
    pub fn pages_inside(&self, item: &Content) -> bool {
        !self.prefs.pages_off && item.kind == ContentKind::Html
    }

    /// The reason the page is shown as a card, in words, or nothing if it is (or is about to be) drawn.
    pub fn page_reason(&self, item: &Content) -> Option<String> {
        if self.prefs.pages_off {
            return Some("Showing web pages inside the app is switched off in Settings.".into());
        }
        let st = self.page.as_ref().filter(|p| p.key == key_of(item))?;
        st.fallback.as_ref().map(|f| f.text())
    }

    /// Free the page and its picture: the tab was closed or hidden, or another page replaced it.
    pub fn page_close(&mut self, window: &mut Window) {
        if let Some(st) = self.page.take() {
            if let Some(img) = st.image {
                let _ = window.drop_image(img);
            }
            // the view drops here: its helper process ends and its memory goes back
        }
    }

    /// The page is not on screen now (another tab, another chat, hidden): its helper ends. An app in the chat
    /// keeps a still picture and waits for a Run click.
    pub fn page_release_if_hidden(&mut self, showing: bool, window: &mut Window) {
        if !showing && self.page.is_some() {
            self.page_park(window);
        }
    }

    /// End the helper. For an app in the chat, keep its last picture as a still and mark it stopped.
    pub fn page_park(&mut self, window: &mut Window) {
        let Some(mut st) = self.page.take() else { return };
        if st.place == "panel" {
            self.page = Some(st);
            return self.page_close(window);
        }
        if let Some(img) = st.image.take() {
            let h = st.measured.lock().map(|m| m.h).unwrap_or(0.);
            if let Some(i) = self.page_stills.iter().position(|s| s.0 == st.place) {
                let old = self.page_stills.remove(i);
                let _ = window.drop_image(old.1);
            }
            self.page_stills.push((st.place.clone(), img, h));
            while self.page_stills.len() > 3 {
                let old = self.page_stills.remove(0);
                let _ = window.drop_image(old.1);
            }
        }
        self.page_parked.insert(st.place.clone());
        // the view drops here: its helper process ends and its memory goes back
    }

    /// The still picture of a stopped app, and its height.
    pub fn page_still(&self, place: &str) -> Option<(std::sync::Arc<RenderImage>, f32)> {
        self.page_stills.iter().find(|s| s.0 == place).map(|s| (s.1.clone(), s.2))
    }

    /// Is this page the one running now, at this place?
    pub fn page_live_at(&self, item: &Content, place: &str) -> bool {
        self.page.as_ref().is_some_and(|p| p.key == key_of(item) && p.place == place)
    }

    /// The viewer for a web page, or `None` for the card (the card says why). `place` says where it is drawn; only
    /// one page runs at a time, so drawing it somewhere else moves it there. In the chat the box takes the page's
    /// own height (within limits), so a small app sits in the conversation like any answer.
    pub fn page_element(&mut self, item: &Content, place: &str, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.pages_inside(item) {
            return None;
        }
        let key = key_of(item);
        if self.page.as_ref().is_none_or(|p| p.key != key || p.place != place) {
            self.page_park(window);
            self.page_parked.remove(place);
            self.page = Some(PageState { key, html: item.body.clone(), place: place.to_string(), pressed: false, view: None, opening: false, busy: false, want: Want::default(), debounce: Debounce::default(), measured: Arc::new(Mutex::new(Measured::default())), scale: window.scale_factor(), image: None, info: None, focus: cx.focus_handle(), fallback: None, link: None, refused: 0, follow_up: None });
        }
        let win_h: f32 = window.viewport_size().height.into();
        let in_chat = place != "panel";
        // an app scrolled more than a screen away stops; its still picture stands in until Run is clicked
        let m = self.page.as_ref().filter(|p| p.place == place).and_then(|p| p.measured.lock().ok().map(|m| *m)).unwrap_or_default();
        if in_chat && m.h > 0. && (m.y > win_h * 2. || m.y + m.h < -win_h) {
            self.page_park(window);
            return None;
        }
        let own = self.page.as_ref().filter(|p| p.key == key).and_then(|p| p.info.as_ref()).map(|i| i.content_height).filter(|h| *h > 0.);
        let height = if in_chat { own.unwrap_or(320.).clamp(120., (win_h - 220.).clamp(240., 640.)) } else { (win_h - 300.).clamp(240., 1600.) };
        let p = self.pal.clone();
        let rtl = self.rtl;
        let st = self.page.as_mut()?;
        if st.fallback.is_some() {
            return None;
        }
        st.scale = window.scale_factor();
        let title = st.info.as_ref().map(|i| i.title.clone()).filter(|t| !t.is_empty()).unwrap_or_else(|| if item.title.is_empty() { "Web page".into() } else { item.title.clone() });
        let measured = st.measured.clone();
        let measure = canvas(
            move |b, _, _| {
                if let Ok(mut m) = measured.lock() {
                    *m = Measured { w: b.size.width.into(), h: b.size.height.into(), x: b.origin.x.into(), y: b.origin.y.into() };
                }
            },
            |_, _, _, _| (),
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        let ring = BoxShadow { color: p.accent, offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: px(2.), inset: false };
        let focus = st.focus.clone();
        let picture = st.image.clone().map(|i| img(i).absolute().top_0().left_0().size_full().object_fit(ObjectFit::Fill));
        let waiting = st.image.is_none().then(|| div().absolute().top_0().left_0().size_full().flex().items_center().justify_center().text_sm().text_color(gpui::hsla(0., 0., 0.35, 1.)).child("Drawing the page…"));
        let area = div()
            .id(SharedString::from(format!("page-view-{place}")))
            .role(Role::Document)
            .aria_label(title)
            .track_focus(&focus)
            .tab_stop(true)
            .focus_visible(move |s| s.shadow(vec![ring]))
            .relative()
            .w_full()
            .h(px(height))
            .overflow_hidden()
            .rounded(p.inner)
            .border_1()
            .border_color(p.border)
            // the page keeps its own colours; the picture sits on white so a page with no background looks as it would
            // in a browser, and the frame around it is the app's
            .bg(gpui::hsla(0., 0., 1., 1.))
            .child(measure)
            .children(picture)
            .children(waiting)
            .on_scroll_wheel(cx.listener(|a, ev: &ScrollWheelEvent, w, cx| {
                let dy: f32 = ev.delta.pixel_delta(px(24.)).y.into();
                // a page that fits its box leaves the wheel to the chat around it
                if a.page_can_scroll() {
                    // the wheel reports "up" as positive; the page goes down when the offset grows
                    a.page_step(Step::Pixels(-dy), w, cx);
                    cx.stop_propagation();
                }
            }))
            .on_key_down(cx.listener(|a, ev: &KeyDownEvent, w, cx| {
                let k = &ev.keystroke;
                if k.modifiers.control || k.modifiers.alt || k.modifiers.platform {
                    return; // the app's own shortcuts still work
                }
                if matches!(k.key.as_str(), "pageup" | "pagedown") {
                    if let Some(step) = scroll::key_step(k.key.as_str(), k.modifiers.shift) {
                        a.page_step(step, w, cx);
                    }
                } else {
                    // typing goes to the page: text fields, sliders and buttons there take keys as in a browser
                    a.page_input(Input::Key(k.key.clone(), k.key_char.clone().unwrap_or_default()), w, cx);
                }
                cx.stop_propagation();
            }))
            .on_mouse_down(MouseButton::Left, cx.listener(|a, ev: &MouseDownEvent, w, cx| a.page_click(ev.position, w, cx)))
            .on_mouse_move(cx.listener(|a, ev: &MouseMoveEvent, w, cx| a.page_pointer(0, ev.position, w, cx)))
            .on_mouse_up(MouseButton::Left, cx.listener(|a, ev: &MouseUpEvent, w, cx| a.page_pointer(2, ev.position, w, cx)));
        let bar = self.page_link_bar(rtl, cx);
        let chip = self.page_follow_up_chip(rtl, cx);
        Some(div().flex().flex_col().gap_2().child(area).children(bar).children(chip).into_any_element())
    }

    /// The follow-up the page offered, as a chip under it. Nothing is sent until it is clicked.
    fn page_follow_up_chip(&mut self, rtl: bool, cx: &mut Context<Self>) -> Option<AnyElement> {
        let text = self.page.as_ref()?.follow_up.clone()?;
        let p = self.pal.clone();
        let send_text = text.clone();
        let chip = div().id("page-follow-up").role(Role::Button).aria_label(SharedString::from(format!("{}: {text}", if rtl { "أرسل" } else { "Send" })))
            .flex_1().min_w_0().px_3().py_1p5().rounded(p.inner).border_1().border_color(p.accent.opacity(0.45)).bg(p.accent.opacity(0.08))
            .text_sm().text_color(p.text).cursor_pointer().hover(|s| s.bg(p.accent.opacity(0.16)))
            .on_click(cx.listener(move |a, _, w, cx| {
                if let Some(st) = a.page.as_mut() {
                    st.follow_up = None;
                }
                let t = send_text.clone();
                a.composer.update(cx, |s, cx| s.set_value(t, w, cx));
                a.send(w, cx);
            }))
            .child(text);
        let dismiss = self.icon_button("page-follow-up-dismiss", crate::ui::icons::close(), if rtl { "تجاهل الاقتراح" } else { "Dismiss suggestion" }, cx, |a, _, cx| {
            if let Some(st) = a.page.as_mut() {
                st.follow_up = None;
            }
            cx.notify();
        });
        Some(div().flex().when(rtl, |d| d.flex_row_reverse()).items_center().gap_1().child(chip).child(dismiss).into_any_element())
    }

    /// The bar under the page that names a clicked link and offers to open it. Never shown for a link inside the page.
    fn page_link_bar(&mut self, rtl: bool, cx: &mut Context<Self>) -> Option<AnyElement> {
        let link = self.page.as_ref()?.link.clone()?;
        let p = self.pal.clone();
        let can_open = openable(&link.url);
        let (open_url, copy_url) = (link.url.clone(), link.url.clone());
        let open = can_open.then(|| self.button("page-link-open", if rtl { "فتح في المتصفح" } else { "Open in browser" }, Kind::Primary, cx, move |a, _, cx| a.page_open_link(&open_url, cx)));
        let copy = self.button("page-link-copy", if rtl { "نسخ" } else { "Copy" }, Kind::Normal, cx, move |_, _, cx| cx.write_to_clipboard(ClipboardItem::new_string(copy_url.clone())));
        let close = self.button("page-link-close", if rtl { "إخفاء" } else { "Hide" }, Kind::Ghost, cx, |a, _, cx| {
            if let Some(st) = a.page.as_mut() {
                st.link = None;
            }
            cx.notify();
        });
        let note = (!can_open).then(|| div().text_xs().text_color(p.muted).child(if rtl { "هذا النوع من العناوين لا يُفتح من هنا." } else { "This kind of address is never opened from here." }));
        Some(
            div()
                .id("page-link-bar")
                .role(Role::Group)
                .aria_label(if rtl { "رابط في الصفحة" } else { "Link in the page" })
                .flex()
                .flex_col()
                .gap_1()
                .p_2()
                .rounded(p.inner)
                .bg(p.sunken)
                .text_color(p.text)
                .child(div().text_sm().when(rtl, |d| d.text_right()).overflow_hidden().child(link.url))
                .children(note)
                .child(div().flex().when(rtl, |d| d.flex_row_reverse()).flex_wrap().gap_2().children(open).child(copy).child(close))
                .into_any_element(),
        )
    }

    /// Hand a web address or mail link to the desktop, after the person pressed the button for it.
    pub fn page_open_link(&mut self, url: &str, cx: &mut Context<Self>) {
        if !openable(url) {
            return;
        }
        let spawned = std::process::Command::new("xdg-open").arg(url).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn();
        match spawned {
            Ok(mut child) => {
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
            }
            Err(e) => self.content_note = Some(format!("Could not start the browser (xdg-open): {e}")),
        }
        cx.notify();
    }

    fn page_click(&mut self, at: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(st) = self.page.as_mut() else { return };
        st.focus.focus(window, cx);
        let m = st.measured.lock().map(|m| *m).unwrap_or_default();
        let (x, y): (f32, f32) = (f32::from(at.x) - m.x, f32::from(at.y) - m.y);
        if x < 0. || y < 0. || x > m.w || y > m.h {
            return;
        }
        st.pressed = true;
        st.want.input.push(Input::Pointer(1, x * st.scale, y * st.scale));
        st.want.link = Some((x * st.scale, y * st.scale));
        st.want.render = true;
        self.page_pump(window, cx);
    }

    /// The pointer moved (passed on only while held, to drag a slider) or was let go.
    fn page_pointer(&mut self, action: u8, at: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(st) = self.page.as_mut() else { return };
        if !st.pressed {
            return;
        }
        let m = st.measured.lock().map(|m| *m).unwrap_or_default();
        let (x, y) = ((f32::from(at.x) - m.x).clamp(0., m.w) * st.scale, (f32::from(at.y) - m.y).clamp(0., m.h) * st.scale);
        if action == 2 {
            st.pressed = false;
        }
        // while the helper is busy, only the latest move is worth sending
        if action == 0 && matches!(st.want.input.last(), Some(Input::Pointer(0, ..))) {
            st.want.input.pop();
        }
        st.want.input.push(Input::Pointer(action, x, y));
        st.want.render = true;
        self.page_pump(window, cx);
    }

    fn page_input(&mut self, i: Input, window: &mut Window, cx: &mut Context<Self>) {
        let Some(st) = self.page.as_mut() else { return };
        st.want.input.push(i);
        st.want.render = true;
        self.page_pump(window, cx);
    }

    /// Is the page taller than its box?
    fn page_can_scroll(&self) -> bool {
        let Some(st) = self.page.as_ref() else { return false };
        let h = st.measured.lock().map(|m| m.h).unwrap_or(0.);
        st.info.as_ref().is_some_and(|i| i.content_height > h + 1.)
    }

    /// The wheel, an arrow, a Page key, Home or End: move the view, inside the page.
    fn page_step(&mut self, step: Step, window: &mut Window, cx: &mut Context<Self>) {
        let Some(st) = self.page.as_mut() else { return };
        let (Some(info), Some(m)) = (st.info.clone(), st.measured.lock().ok().map(|m| *m)) else { return };
        let from = st.want.scroll_to.unwrap_or(info.scroll_y);
        let to = scroll::apply(from, step, info.content_height, m.h);
        if (to - from).abs() > 0.01 {
            st.want.scroll_to = Some(to);
            st.want.render = true;
        }
        self.page_pump(window, cx);
    }

    /// Called often while a page is shown: open it once the panel has a size, and follow a changed size once it has
    /// stopped changing.
    pub fn page_poll(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(st) = self.page.as_mut() else { return };
        if st.fallback.is_some() {
            return;
        }
        let m = st.measured.lock().map(|m| *m).unwrap_or_default();
        let size = ((m.w * st.scale).round() as u32, (m.h * st.scale).round() as u32, st.scale);
        if let Some(applied) = st.debounce.poll(size, Instant::now()) {
            // a resize that comes while the page is being opened waits and is applied right after
            if st.view.is_some() || st.busy || st.opening {
                st.want.resize = Some(applied);
                st.want.render = true;
            }
        }
        self.page_pump(window, cx);
    }

    /// Start whatever is wanted, if the helper is free.
    fn page_pump(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(st) = self.page.as_mut() else { return };
        let (title_html, key) = (st.html.clone(), st.key);
        if st.busy || st.opening || st.fallback.is_some() {
            return;
        }
        if st.view.is_none() {
            // first the page has to be opened, at the size the panel has now
            let m = st.measured.lock().map(|m| *m).unwrap_or_default();
            let (w, h) = ((m.w * st.scale).round() as u32, (m.h * st.scale).round() as u32);
            if w < 40 || h < 40 {
                return;
            }
            st.opening = true;
            let scale = st.scale;
            let _ = st.debounce.poll((w, h, scale), Instant::now());
            cx.spawn_in(window, async move |this: WeakEntity<App>, cx: &mut AsyncWindowContext| {
                let opened = cx.background_executor().spawn(async move { PageView::open(&title_html, w, h, scale, Options::default()) }).await;
                let _ = this.update_in(cx, |app, window, cx| app.page_opened(key, opened, window, cx));
            })
            .detach();
            return;
        }
        if !st.want.any() {
            return;
        }
        let want = std::mem::take(&mut st.want);
        let Some(mut view) = st.view.take() else { return };
        st.busy = true;
        cx.spawn_in(window, async move |this: WeakEntity<App>, cx: &mut AsyncWindowContext| {
            let (view, ans) = cx
                .background_executor()
                .spawn(async move {
                    let mut ans = Answers { had_input: !want.input.is_empty(), ..Answers::default() };
                    let run = |view: &mut PageView, ans: &mut Answers| -> Result<(), Fallback> {
                        if let Some((w, h, s)) = want.resize {
                            view.resize(w, h, s)?;
                        }
                        for (id, (status, ctype, body)) in want.deliver {
                            view.deliver(id, status, &ctype, body)?;
                        }
                        for i in &want.input {
                            match i {
                                Input::Pointer(a, x, y) => view.pointer(*a, *x, *y)?,
                                Input::Key(k, t) => view.key(k, t)?,
                            }
                        }
                        if let Some(y) = want.scroll_to {
                            view.scroll_to(y)?;
                        }
                        if want.render {
                            ans.frame = Some(view.render()?);
                        }
                        if let Some((x, y)) = want.link {
                            ans.link = Some(view.link_at(x, y)?);
                        }
                        Ok(())
                    };
                    if let Err(f) = run(&mut view, &mut ans) {
                        ans.failed = Some(f);
                    }
                    ans.info = Some(view.info().clone());
                    ans.follow_up = view.take_follow_up();
                    ans.fetches = view.take_fetches();
                    ans.refused = view.net().refused;
                    (view, ans)
                })
                .await;
            let _ = this.update_in(cx, |app, window, cx| app.page_done(key, view, ans, window, cx));
        })
        .detach();
    }

    /// Fetch what the page asked of allowed sites, off the UI thread, then hand it over and draw again.
    fn page_fetch(&mut self, key: u64, urls: Vec<(u32, String)>, window: &mut Window, cx: &mut Context<Self>) {
        for (id, url) in urls {
            cx.spawn_in(window, async move |this: WeakEntity<App>, cx: &mut AsyncWindowContext| {
                let got = cx.background_executor().spawn(async move { lh_gui::page_fetch::fetch(&url) }).await;
                let _ = this.update_in(cx, |app, window, cx| {
                    if let Some(st) = app.page.as_mut().filter(|p| p.key == key) {
                        st.want.deliver.push((id, got));
                        st.want.render = true;
                    }
                    app.page_pump(window, cx);
                });
            })
            .detach();
        }
    }

    fn page_opened(&mut self, key: u64, opened: Result<PageView, Fallback>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(st) = self.page.as_mut().filter(|p| p.key == key) else { return }; // replaced meanwhile: the new page is dropped
        st.opening = false;
        match opened {
            Ok(mut view) => {
                let fetches = view.take_fetches();
                st.info = Some(view.info().clone());
                st.refused = view.net().refused;
                st.view = Some(view);
                st.want.render = true;
                self.page_fetch(key, fetches, window, cx);
                self.page_pump(window, cx);
            }
            Err(f) => st.fallback = Some(f),
        }
        cx.notify();
    }

    fn page_done(&mut self, key: u64, view: PageView, ans: Answers, window: &mut Window, cx: &mut Context<Self>) {
        let Some(st) = self.page.as_mut().filter(|p| p.key == key) else { return };
        st.busy = false;
        if let Some(i) = ans.info {
            st.info = Some(i);
        }
        st.refused = ans.refused;
        if ans.follow_up.is_some() {
            st.follow_up = ans.follow_up;
        }
        if let Some(f) = ans.failed {
            st.fallback = Some(f);
            if let Some(old) = st.image.take() {
                let _ = window.drop_image(old);
            }
            drop(view);
            cx.notify();
            return;
        }
        st.view = Some(view);
        if let Some(frame) = ans.frame {
            if let Some(new) = to_image(frame) {
                if let Some(old) = st.image.replace(new) {
                    let _ = window.drop_image(old);
                }
            }
        }
        if ans.had_input {
            // scripts answer input a moment later (an `input` handler, a timer): draw again shortly to catch it
            cx.spawn_in(window, async move |this: WeakEntity<App>, cx: &mut AsyncWindowContext| {
                for wait in [120u64, 400] {
                    cx.background_executor().timer(std::time::Duration::from_millis(wait)).await;
                    let _ = this.update_in(cx, |app, window, cx| {
                        if let Some(st) = app.page.as_mut().filter(|p| p.key == key) {
                            st.want.render = true;
                        }
                        app.page_pump(window, cx);
                    });
                }
            })
            .detach();
        }
        if let Some(l) = ans.link {
            // a link inside the page needs nothing; any other is named in the bar and waits for a button
            st.link = l.filter(|l| !l.same_page);
        }
        self.page_fetch(key, ans.fetches, window, cx);
        self.page_pump(window, cx);
        cx.notify();
    }
}
