//! The page drawing helper with Servo. Same requests and replies as the Blitz helper (see `lh_pageview::proto`), so
//! the app does not care which one it runs. Pages may run JavaScript here, which is why it is walled in twice:
//!
//! * The process limits itself before it reads anything: no new privileges, a memory ceiling, no core dumps, and a
//!   network namespace with no network in it (where the system allows that without privilege).
//! * Servo asks this program about every request a page makes. The page itself is served from memory; everything
//!   else is answered with an empty "403" and counted, and every navigation away from the page is refused. The one
//!   exception is a picture, map tile or JSON from a short list of sites (`lh_pageview::ALLOWED_HOSTS`, https
//!   only): that request is held, named in the reply, fetched by the app outside this sandbox, and its bytes come
//!   back with `Request::Deliver`. This process still never touches the network itself.
//!
//! Scrolling, the page height and the link under the pointer are asked of the page with small scripts, because
//! those are the parts of Servo's interface that stay the same from release to release.

use std::cell::{Cell, RefCell};
use std::io::{BufReader, BufWriter, Write};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use dpi::PhysicalSize;
use lh_pageview::proto::{read_request, write_reply, NetReport, Reply, Request};
use lh_pageview::{resolve_link, scroll, Frame, Info, BASE};
use servo::{
    EventLoopWaker, JSValue, LoadStatus, NavigationRequest, RenderingContext, Servo, ServoBuilder, SoftwareRenderingContext, WebResourceLoad, WebResourceResponse, WebView, WebViewBuilder,
    WebViewDelegate,
};
use url::Url;

/// Servo 0.7 draws `<input type=range>` but does not move it with the pointer or the arrow keys
/// (servo-script `range_input_type.rs` has no input handling), so each page gets this small script that does,
/// and sends the same `input` and `change` events a browser would. It also gives the page
/// `lh.followUp(text)`: the text is kept until the app asks for it after the next press or key, and the
/// app shows it as a suggestion that is sent only if the person clicks it. The script runs after the page's own,
/// so a page calls it from a click or key handler.
const RANGE_FIX: &str = r#"<script>(function(){
var held=null;
function set(r,x){var b=r.getBoundingClientRect();var min=+r.min||0,max=r.max===''?100:+r.max,st=r.step==='any'?0:(+r.step||1);
var f=Math.min(1,Math.max(0,(x-b.left)/Math.max(1,b.width)));var v=min+f*(max-min);if(st)v=Math.round((v-min)/st)*st+min;
if(String(v)!==r.value){r.value=v;r.dispatchEvent(new Event('input',{bubbles:true}));}}
document.addEventListener('mousedown',function(e){var r=e.target;if(r&&r.type==='range'&&!r.disabled){held=r;r.focus();set(r,e.clientX);e.preventDefault();}},true);
document.addEventListener('mousemove',function(e){if(held)set(held,e.clientX);},true);
document.addEventListener('mouseup',function(){if(held){held.dispatchEvent(new Event('change',{bubbles:true}));held=null;}},true);
document.addEventListener('keydown',function(e){var r=document.activeElement;if(!r||r.type!=='range')return;
var d={ArrowLeft:-1,ArrowDown:-1,ArrowRight:1,ArrowUp:1}[e.key];if(!d)return;var st=+r.step||1;
var v=Math.min(r.max===''?100:+r.max,Math.max(+r.min||0,+r.value+d*st));if(String(v)!==r.value){r.value=v;r.dispatchEvent(new Event('input',{bubbles:true}));r.dispatchEvent(new Event('change',{bubbles:true}));}e.preventDefault();},true);
})();
window.lh={followUp:function(t){window.__lhFollow=String(t).slice(0,500);}};
</script>"#;

const MEMORY_CEILING: libc::rlim_t = 4 * 1024 * 1024 * 1024;
/// Longest wait for the page to finish loading, and for one script answer.
const LOAD_WAIT: Duration = Duration::from_secs(3);
const SCRIPT_WAIT: Duration = Duration::from_secs(2);

fn limit_myself() {
    // SAFETY: plain system calls with constant arguments.
    unsafe {
        libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0);
        let lim = |r, v| {
            let l = libc::rlimit { rlim_cur: v, rlim_max: v };
            libc::setrlimit(r, &l);
        };
        lim(libc::RLIMIT_AS, MEMORY_CEILING);
        lim(libc::RLIMIT_CORE, 0);
        libc::unshare(libc::CLONE_NEWUSER | libc::CLONE_NEWNET);
    }
}

#[derive(Clone)]
struct Waker(Arc<AtomicBool>);
impl EventLoopWaker for Waker {
    fn clone_box(&self) -> Box<dyn EventLoopWaker> {
        Box::new(self.clone())
    }
    fn wake(&self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

/// The page's own HTML, the counts of what it asked for, and frame bookkeeping.
struct Delegate {
    html: RefCell<Vec<u8>>,
    refused: Cell<u32>,
    urls: RefCell<Vec<String>>,
    frames: Cell<u32>,
    status: Cell<Option<LoadStatus>>,
    /// Requests to allowed sites, held until the app delivers what it fetched, and the ones not yet named to it.
    held: RefCell<std::collections::HashMap<u32, WebResourceLoad>>,
    asks: RefCell<Vec<(u32, String)>>,
    next: Cell<u32>,
}

impl WebViewDelegate for Delegate {
    fn notify_load_status_changed(&self, _: WebView, s: LoadStatus) {
        self.status.set(Some(s));
    }
    fn notify_new_frame_ready(&self, _: WebView) {
        self.frames.set(self.frames.get() + 1);
    }
    fn request_navigation(&self, _: WebView, n: NavigationRequest) {
        // the page itself is the only place it may be; links are handed to the app through `link_at`
        if n.url.as_str() == BASE {
            n.allow();
        } else {
            n.deny();
        }
    }
    fn load_web_resource(&self, _: WebView, load: WebResourceLoad) {
        let url = load.request().url.clone();
        if url.as_str() == BASE {
            let mut headers = http::HeaderMap::new();
            headers.insert(http::header::CONTENT_TYPE, http::HeaderValue::from_static("text/html; charset=utf-8"));
            let mut l = load.intercept(WebResourceResponse::new(url).headers(headers).status_code(http::StatusCode::OK));
            let mut body = self.html.borrow().clone();
            body.extend_from_slice(RANGE_FIX.as_bytes());
            l.send_body_data(body);
            l.finish();
            return;
        }
        if lh_pageview::fetch_allowed(url.as_str()) && self.held.borrow().len() < 64 {
            let id = self.next.get() + 1;
            self.next.set(id);
            self.asks.borrow_mut().push((id, url.as_str().to_string()));
            self.held.borrow_mut().insert(id, load);
            return;
        }
        if url.scheme() == "data" {
            // inside the page already; nothing leaves the process
            return;
        }
        self.refused.set(self.refused.get() + 1);
        if self.urls.borrow().len() < 50 {
            self.urls.borrow_mut().push(url.as_str().chars().take(200).collect());
        }
        // empty, not silence, so the page does not wait for it
        load.intercept(WebResourceResponse::new(url).status_code(http::StatusCode::FORBIDDEN)).finish();
    }
}

struct Page {
    servo: Servo,
    rc: Rc<SoftwareRenderingContext>,
    webview: WebView,
    delegate: Rc<Delegate>,
    w: u32,
    h: u32,
    scale: f32,
    painted: u32,
}

impl Page {
    fn new(html: Vec<u8>, w: u32, h: u32, scale: f32) -> Result<Page, String> {
        let (w, h, scale) = (w.max(1), h.max(1), scale.clamp(0.5, 4.0));
        let rc = Rc::new(SoftwareRenderingContext::new(PhysicalSize { width: w, height: h }).map_err(|e| format!("{e:?}"))?);
        rc.make_current().map_err(|e| format!("{e:?}"))?;
        let servo = ServoBuilder::default().event_loop_waker(Box::new(Waker(Arc::new(AtomicBool::new(false))))).build();
        let delegate = Rc::new(Delegate { html: RefCell::new(html), refused: Cell::new(0), urls: RefCell::new(vec![]), frames: Cell::new(0), status: Cell::new(None), held: RefCell::default(), asks: RefCell::default(), next: Cell::new(0) });
        let webview = WebViewBuilder::new(&servo, rc.clone()).url(Url::parse(BASE).expect("BASE is a valid address")).delegate(delegate.clone()).build();
        webview.set_hidpi_scale_factor(euclid::Scale::new(scale));
        let mut p = Page { servo, rc, webview, delegate, w, h, scale, painted: 0 };
        let t0 = Instant::now();
        while t0.elapsed() < LOAD_WAIT && p.delegate.status.get() != Some(LoadStatus::Complete) {
            p.spin();
        }
        Ok(p)
    }

    fn spin(&mut self) {
        self.servo.spin_event_loop();
        if self.delegate.frames.get() != self.painted {
            self.painted = self.delegate.frames.get();
            self.webview.paint();
        }
        std::thread::sleep(Duration::from_millis(2));
    }

    /// Run a short script in the page and wait for its answer.
    fn ask(&mut self, script: &str) -> Option<JSValue> {
        let out: Rc<RefCell<Option<Option<JSValue>>>> = Rc::default();
        let o = out.clone();
        self.webview.evaluate_javascript(script, move |r| *o.borrow_mut() = Some(r.ok()));
        let t0 = Instant::now();
        while out.borrow().is_none() && t0.elapsed() < SCRIPT_WAIT {
            self.spin();
        }
        let answer = out.borrow_mut().take().flatten();
        answer
    }

    fn number(&mut self, script: &str) -> f32 {
        match self.ask(script) {
            Some(JSValue::Number(n)) => n as f32,
            _ => 0.0,
        }
    }

    fn info(&mut self) -> Info {
        let title = self.webview.page_title().unwrap_or_default().split_whitespace().collect::<Vec<_>>().join(" ");
        Info { title, content_height: self.number("document.documentElement.scrollHeight"), scroll_y: self.number("window.scrollY"), follow_up: String::new() }
    }

    /// The page's info, with any follow-up it offered since it was last asked.
    fn info_after_input(&mut self) -> Info {
        let follow_up = match self.ask("(function(){var f=window.__lhFollow||'';window.__lhFollow='';return String(f);})()") {
            Some(JSValue::String(s)) => s.chars().take(lh_pageview::FOLLOW_UP_MAX).collect(),
            _ => String::new(),
        };
        Info { follow_up, ..self.info() }
    }

    fn report(&self) -> NetReport {
        NetReport { refused: self.delegate.refused.get(), data_served: 0, urls: self.delegate.urls.borrow().clone(), fetch: std::mem::take(&mut *self.delegate.asks.borrow_mut()) }
    }

    /// The app fetched a held request: answer the page with it, or with an error when it could not.
    fn deliver(&mut self, id: u32, status: u16, content_type: &str, body: Vec<u8>) -> Info {
        if let Some(load) = self.delegate.held.borrow_mut().remove(&id) {
            let url = load.request().url.clone();
            let code = http::StatusCode::from_u16(status).ok().filter(|_| status > 0).unwrap_or(http::StatusCode::BAD_GATEWAY);
            let mut headers = http::HeaderMap::new();
            if let Ok(v) = http::HeaderValue::from_str(content_type) {
                headers.insert(http::header::CONTENT_TYPE, v);
            }
            let mut l = load.intercept(WebResourceResponse::new(url).headers(headers).status_code(code));
            if status > 0 {
                l.send_body_data(body);
            }
            l.finish();
        }
        self.spin();
        self.info()
    }

    fn resize(&mut self, w: u32, h: u32, scale: f32) -> Info {
        (self.w, self.h, self.scale) = (w.max(1), h.max(1), scale.clamp(0.5, 4.0));
        self.webview.resize(PhysicalSize { width: self.w, height: self.h });
        self.webview.set_hidpi_scale_factor(euclid::Scale::new(self.scale));
        self.spin();
        self.info()
    }

    fn scroll_to(&mut self, y: f32) -> Info {
        let height = self.number("document.documentElement.scrollHeight");
        let y = scroll::clamp(y, height, self.h as f32 / self.scale);
        self.ask(&format!("window.scrollTo(0, {y})"));
        self.spin();
        self.info()
    }

    fn render(&mut self) -> (Frame, Info) {
        // let the page settle a moment: scripts and timers may still be drawing
        for _ in 0..5 {
            self.spin();
        }
        self.webview.paint();
        let rect = servo::DeviceIntRect::from_size(euclid::Size2D::new(self.w as i32, self.h as i32));
        let rgba = self.rc.read_to_image(rect).map(|img| img.into_raw()).unwrap_or_else(|| vec![255; (self.w * self.h * 4) as usize]);
        (Frame { width: self.w, height: self.h, rgba }, self.info())
    }

    /// The pointer moved, was pressed or was released (physical pixels of the picture). Real input events, so the
    /// page's own handlers, sliders and text fields work as in a browser.
    fn pointer(&mut self, action: u8, x: f32, y: f32) -> Info {
        let point = servo::WebViewPoint::Device(servo::DevicePoint::new(x, y));
        let ev = match action {
            1 => servo::InputEvent::MouseButton(servo::MouseButtonEvent::new(servo::MouseButtonAction::Down, servo::MouseButton::Primary, point)),
            2 => servo::InputEvent::MouseButton(servo::MouseButtonEvent::new(servo::MouseButtonAction::Up, servo::MouseButton::Primary, point)),
            _ => servo::InputEvent::MouseMove(servo::MouseMoveEvent::new(point)),
        };
        self.webview.notify_input_event(ev);
        self.spin();
        if action == 0 { self.info() } else { self.info_after_input() }
    }

    /// A key, pressed and released. `text` is what it types; named keys (Enter, arrows, Backspace) type nothing.
    fn key(&mut self, key: &str, text: &str) -> Info {
        use servo::{Key, KeyState, NamedKey};
        let k = match key {
            "enter" => Key::Named(NamedKey::Enter),
            "backspace" => Key::Named(NamedKey::Backspace),
            "delete" => Key::Named(NamedKey::Delete),
            "tab" => Key::Named(NamedKey::Tab),
            "escape" => Key::Named(NamedKey::Escape),
            "left" => Key::Named(NamedKey::ArrowLeft),
            "right" => Key::Named(NamedKey::ArrowRight),
            "up" => Key::Named(NamedKey::ArrowUp),
            "down" => Key::Named(NamedKey::ArrowDown),
            "home" => Key::Named(NamedKey::Home),
            "end" => Key::Named(NamedKey::End),
            "space" => Key::Character(" ".into()),
            _ if !text.is_empty() => Key::Character(text.into()),
            _ => return self.info(),
        };
        for state in [KeyState::Down, KeyState::Up] {
            self.webview.notify_input_event(servo::InputEvent::Keyboard(servo::KeyboardEvent::from_state_and_key(state, k.clone())));
        }
        self.spin();
        self.info_after_input()
    }

    /// The link at a point of the picture (physical pixels): the nearest `<a href>` around the element there.
    fn link_at(&mut self, x: f32, y: f32) -> Option<lh_pageview::Link> {
        let (x, y) = (x / self.scale, y / self.scale);
        let js = format!("(function(){{var e=document.elementFromPoint({x},{y});var a=e&&e.closest('a[href]');return a?a.getAttribute('href'):null;}})()");
        match self.ask(&js) {
            Some(JSValue::String(href)) => resolve_link(&href),
            _ => None,
        }
    }
}

fn run() {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    let mut input = BufReader::new(std::io::stdin().lock());
    let mut output = BufWriter::new(std::io::stdout().lock());
    let mut page: Option<Page> = None;
    while let Ok(Some(req)) = read_request(&mut input) {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Option<Reply> {
            match req {
                // the fault switches exist for the Blitz helper's crash tests; this helper has no test faults
                Request::Load { w, h, scale, html, .. } => {
                    let mut p = Page::new(html, w, h, scale).ok()?;
                    let reply = Reply::Info(p.info(), p.report());
                    page = Some(p);
                    Some(reply)
                }
                Request::Resize { w, h, scale } => page.as_mut().map(|p| Reply::Info(p.resize(w, h, scale), p.report())),
                Request::ScrollTo { y } => page.as_mut().map(|p| Reply::Info(p.scroll_to(y), p.report())),
                Request::Render => page.as_mut().map(|p| {
                    let (f, i) = p.render();
                    Reply::Frame(f, i, p.report())
                }),
                Request::LinkAt { x, y } => page.as_mut().map(|p| Reply::Link(p.link_at(x, y))),
                Request::Pointer { action, x, y } => page.as_mut().map(|p| Reply::Info(p.pointer(action, x, y), p.report())),
                Request::Key { key, text } => page.as_mut().map(|p| Reply::Info(p.key(&key, &text), p.report())),
                Request::Deliver { id, status, content_type, body } => page.as_mut().map(|p| Reply::Info(p.deliver(id, status, &content_type, body), p.report())),
            }
        }));
        let reply = match result {
            Ok(Some(r)) => r,
            Ok(None) => Reply::Crashed,
            Err(_) => {
                let _ = write_reply(&mut output, &Reply::Crashed);
                let _ = output.flush();
                return;
            }
        };
        if write_reply(&mut output, &reply).is_err() {
            return;
        }
    }
}

fn main() {
    limit_myself();
    // Servo keeps its state on the thread that built it, so everything happens on this one thread
    let t = std::thread::Builder::new().name("servo".into()).stack_size(256 * 1024 * 1024).spawn(run);
    if let Ok(t) = t {
        let _ = t.join();
    }
}
