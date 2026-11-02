//! A local web page, drawn into a pixel buffer, safely.
//!
//! The engine is Blitz (HTML, CSS, layout and a CPU painter; no JavaScript at all). Everything that touches it is in
//! this crate. The rules it keeps:
//!
//! * The engine fetches nothing itself. Every request the page makes goes to [`net::Deny`], which refuses it and
//!   answers with empty data, except `data:` addresses, which are decoded in memory.
//! * A click on a link never navigates. [`PageView::link_at`] only says which address is under the pointer; the
//!   app decides what to do with it.
//! * The engine runs in a helper process of its own (`lh-pageview-helper`, built beside the app). Every call has a
//!   time limit. If the engine panics, overflows its stack, hangs, uses too much memory or is given too much, the
//!   app kills the helper and the caller gets a [`Fallback`] with the reason in plain words and shows something
//!   else. The app itself never loads the engine, so nothing the engine does can crash or stall the app.
//! * Memory: only the visible part of the page is ever painted, so a frame is as big as the panel and no bigger, and
//!   closing the page ends the helper and frees everything it held.

pub mod debounce;
pub mod proto;
pub mod scroll;

use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::thread::JoinHandle;
use std::time::Duration;

pub use proto::{Fault, NetReport};
use proto::{read_reply, write_request, Reply, Request};

/// How much a page may ask of the engine.
#[derive(Clone, Debug)]
pub struct Options {
    /// Most bytes of HTML. Bigger pages are not given to the engine at all.
    pub max_html_bytes: usize,
    /// Longest the first parse and layout may take.
    pub load_timeout: Duration,
    /// Longest any later call (resize, scroll, paint, pointing at a link) may take.
    pub call_timeout: Duration,
    /// Where the helper program is. `None`: next to the running program, or where `LH_PAGEVIEW_HELPER` says.
    pub helper: Option<PathBuf>,
    /// For tests only: make the engine fail on purpose, to prove the fallbacks work.
    #[doc(hidden)]
    pub fault: Fault,
}

impl Default for Options {
    fn default() -> Self {
        Options { max_html_bytes: 2 * 1024 * 1024, load_timeout: Duration::from_secs(5), call_timeout: Duration::from_secs(3), helper: None, fault: Fault::None }
    }
}

/// Where the helper is looked for: the given path, `LH_PAGEVIEW_HELPER`, or next to the running program. Next to the
/// program, the Servo helper (`lh-pageview-servo`, pages may run scripts) is used when it is there, and the Blitz
/// helper (`lh-pageview-helper`, no scripts) otherwise. The crash tests always use Blitz: only it has test faults.
pub fn helper_path(opts: &Options) -> Option<PathBuf> {
    if let Some(p) = &opts.helper {
        return Some(p.clone());
    }
    if let Some(p) = std::env::var_os("LH_PAGEVIEW_HELPER") {
        return Some(PathBuf::from(p));
    }
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    // beside the program, or one folder up (a test program lives in `deps/`)
    let names: &[&str] = if opts.fault == Fault::None { &["lh-pageview-servo", "lh-pageview-helper"] } else { &["lh-pageview-helper"] };
    names.iter().flat_map(|n| [dir.join(n), dir.parent().map(|d| d.join(n)).unwrap_or_default()]).find(|p| p.is_file())
}

/// Why the page cannot be shown inside the app. The app shows its plain card and says this.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fallback {
    TooBig { bytes: usize, cap: usize },
    TimedOut,
    Crashed,
    Engine(String),
}

impl Fallback {
    /// One plain sentence for the person.
    pub fn text(&self) -> String {
        match self {
            Fallback::TooBig { bytes, cap } => format!("This page is {} MB, more than the {} MB the app will draw itself.", mb(*bytes), mb(*cap)),
            Fallback::TimedOut => "Drawing this page took too long, so the app stopped trying.".into(),
            Fallback::Crashed => "The page drawing engine failed on this page.".into(),
            Fallback::Engine(m) => format!("The page could not be drawn: {m}"),
        }
    }
}

fn mb(n: usize) -> String {
    let m = n as f64 / 1_048_576.0;
    if m >= 10. { format!("{m:.0}") } else { format!("{m:.1}") }
}

/// A picture of part of the page: RGBA, row by row, `width * height * 4` bytes.
#[derive(Clone, Debug)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// What the page is.
#[derive(Clone, Debug, PartialEq)]
pub struct Info {
    pub title: String,
    /// The height of the whole page in CSS pixels, and how far down the view is.
    pub content_height: f32,
    pub scroll_y: f32,
    /// A follow-up the page offered with `lh.followUp(text)` since the last answer; empty for none.
    pub follow_up: String,
}

/// Sites an app in the chat may load pictures, map tiles and JSON from, over https only. The helper has no
/// network: it hands such requests to the app, which fetches them (see `fetch_allowed`) and passes the bytes back.
pub const ALLOWED_HOSTS: [&str; 4] = ["tile.openstreetmap.org", "a.tile.openstreetmap.org", "b.tile.openstreetmap.org", "upload.wikimedia.org"];
/// The largest single answer passed to a page, and the most answers one page gets.
pub const FETCH_MAX_BYTES: usize = 5 * 1024 * 1024;
pub const FETCH_MAX_COUNT: usize = 300;

/// Whether a page may have this address fetched for it: https, the default port, an allowed host, no user name.
pub fn fetch_allowed(url: &str) -> bool {
    let Ok(u) = url::Url::parse(url) else { return false };
    u.scheme() == "https" && u.port().is_none() && u.username().is_empty() && u.password().is_none() && u.host_str().is_some_and(|h| ALLOWED_HOSTS.contains(&h))
}

/// Whether an answer of this type may be passed to the page: pictures and JSON only.
pub fn fetch_type_allowed(content_type: &str) -> bool {
    let t = content_type.split(';').next().unwrap_or("").trim().to_ascii_lowercase();
    (t.starts_with("image/") && t != "image/svg+xml") || t == "application/json" || t == "application/geo+json"
}

/// The longest follow-up a page may offer, in characters.
pub const FOLLOW_UP_MAX: usize = 500;

/// A page's follow-up as plain text: no control characters, spaces run together, at most `FOLLOW_UP_MAX` characters.
pub fn clean_follow_up(text: &str) -> String {
    let flat: String = text.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    flat.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(FOLLOW_UP_MAX).collect()
}

/// The place under the pointer is a link to this address (resolved against the page's own address). `same_page` is
/// true for a link that only points inside the page (`#top`), which needs no questions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    pub url: String,
    pub same_page: bool,
}

/// The page address links are resolved against. It is not a place anything is ever fetched from.
pub const BASE: &str = "file:///page/index.html";

/// One open page. Dropping it ends the helper and frees everything the page held.
pub struct PageView {
    child: Child,
    to_helper: Option<Sender<Request>>,
    from_helper: Receiver<Reply>,
    threads: Vec<JoinHandle<()>>,
    opts: Options,
    info: Info,
    net: NetReport,
    /// The last follow-up the page offered, until the app takes it.
    follow_up: Option<String>,
    /// Requests to allowed sites waiting for the app to fetch them, and how many were asked for in all.
    to_fetch: Vec<(u32, String)>,
    fetched: usize,
    /// After a timeout or a crash nothing more is asked of the engine.
    dead: Option<Fallback>,
}

impl PageView {
    /// Start the helper, give it `html` and wait for the first layout of a view `w` by `h` physical pixels at `scale`.
    pub fn open(html: &str, w: u32, h: u32, scale: f32, opts: Options) -> Result<PageView, Fallback> {
        if html.len() > opts.max_html_bytes {
            return Err(Fallback::TooBig { bytes: html.len(), cap: opts.max_html_bytes });
        }
        let helper = helper_path(&opts).ok_or_else(|| Fallback::Engine("the page drawing program is missing".into()))?;
        let mut child = Command::new(&helper)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .env_clear()
            .spawn()
            .map_err(|e| Fallback::Engine(format!("the page drawing program would not start ({e})")))?;
        let (stdin, stdout) = (child.stdin.take().expect("piped"), child.stdout.take().expect("piped"));
        let (req_tx, req_rx) = channel::<Request>();
        let (rep_tx, rep_rx) = channel::<Reply>();
        // two small threads, so the app never waits on a pipe: one writes what the helper is asked, one reads what it says
        let writer = std::thread::spawn(move || {
            let mut out = BufWriter::new(stdin);
            while let Ok(req) = req_rx.recv() {
                if write_request(&mut out, &req).is_err() || out.flush().is_err() {
                    return;
                }
            }
        });
        let reader = std::thread::spawn(move || {
            let mut inp = BufReader::new(stdout);
            while let Ok(rep) = read_reply(&mut inp) {
                if rep_tx.send(rep).is_err() {
                    return;
                }
            }
        });
        let mut view = PageView { child, to_helper: Some(req_tx), from_helper: rep_rx, threads: vec![writer, reader], opts: opts.clone(), info: Info { title: String::new(), content_height: 0., scroll_y: 0., follow_up: String::new() }, net: NetReport::default(), follow_up: None, to_fetch: vec![], fetched: 0, dead: None };
        let load = Request::Load { w, h, scale, fault: opts.fault, html: html.as_bytes().to_vec() };
        match view.call(load, opts.load_timeout)? {
            Reply::Info(i, n) => {
                view.note(i, n);
                Ok(view)
            }
            _ => Err(Fallback::Crashed),
        }
    }

    pub fn info(&self) -> &Info {
        &self.info
    }

    /// What the page asked the network for and was refused, as of the last call. Always the whole story: nothing
    /// else was fetched, because the engine has no other way to.
    pub fn net(&self) -> &NetReport {
        &self.net
    }

    pub fn failed(&self) -> Option<&Fallback> {
        self.dead.as_ref()
    }

    /// The helper's process number (for tests and the activity view).
    pub fn helper_pid(&self) -> u32 {
        self.child.id()
    }

    /// The memory the helper holds now, in megabytes (what an open page costs).
    pub fn helper_rss_mb(&self) -> Option<f64> {
        let s = std::fs::read_to_string(format!("/proc/{}/status", self.child.id())).ok()?;
        let kb: f64 = s.lines().find(|l| l.starts_with("VmRSS:"))?.split_whitespace().nth(1)?.parse().ok()?;
        Some(kb / 1024.)
    }

    fn fail(&mut self, why: Fallback) -> Fallback {
        // a stuck or crashed helper is ended, and nothing more is asked of it
        let _ = self.child.kill();
        self.dead = Some(why.clone());
        why
    }

    fn call(&mut self, req: Request, timeout: Duration) -> Result<Reply, Fallback> {
        if let Some(f) = &self.dead {
            return Err(f.clone());
        }
        let sent = self.to_helper.as_ref().is_some_and(|t| t.send(req).is_ok());
        if !sent {
            return Err(self.fail(Fallback::Crashed));
        }
        match self.from_helper.recv_timeout(timeout) {
            Ok(Reply::Crashed) | Err(RecvTimeoutError::Disconnected) => Err(self.fail(Fallback::Crashed)),
            Ok(r) => Ok(r),
            Err(RecvTimeoutError::Timeout) => Err(self.fail(Fallback::TimedOut)),
        }
    }

    /// The follow-up the page offered, once. Nothing is sent anywhere: the app shows it and waits for a click.
    pub fn take_follow_up(&mut self) -> Option<String> {
        self.follow_up.take()
    }

    /// Requests to allowed sites the app should fetch for the page. Over the per-page count they are dropped,
    /// and the page's request simply never finishes.
    pub fn take_fetches(&mut self) -> Vec<(u32, String)> {
        std::mem::take(&mut self.to_fetch)
    }

    /// Hand the page what was fetched for request `id`. Anything not a picture or JSON, or too big, is refused.
    pub fn deliver(&mut self, id: u32, status: u16, content_type: &str, body: Vec<u8>) -> Result<(), Fallback> {
        let ok = status > 0 && fetch_type_allowed(content_type) && body.len() <= FETCH_MAX_BYTES;
        let (status, content_type, body) = if ok { (status, content_type.to_string(), body) } else { (0, String::new(), vec![]) };
        let t = self.opts.call_timeout;
        if let Reply::Info(i, n) = self.call(Request::Deliver { id, status, content_type, body }, t)? {
            self.note(i, n);
        }
        Ok(())
    }

    fn note(&mut self, i: Info, mut n: NetReport) {
        for (id, url) in std::mem::take(&mut n.fetch) {
            if fetch_allowed(&url) && self.fetched < FETCH_MAX_COUNT {
                self.fetched += 1;
                self.to_fetch.push((id, url));
            }
        }
        let f = clean_follow_up(&i.follow_up);
        if !f.is_empty() {
            self.follow_up = Some(f);
        }
        self.info = i;
        self.net = n;
    }

    /// The view changed size (physical pixels) or the screen scale did. The scroll position is kept, within the new limits.
    pub fn resize(&mut self, w: u32, h: u32, scale: f32) -> Result<(), Fallback> {
        let t = self.opts.call_timeout;
        if let Reply::Info(i, n) = self.call(Request::Resize { w, h, scale }, t)? {
            self.note(i, n);
        }
        Ok(())
    }

    /// Scroll to `y` CSS pixels from the top; the engine side keeps it inside the page.
    pub fn scroll_to(&mut self, y: f32) -> Result<(), Fallback> {
        let t = self.opts.call_timeout;
        if let Reply::Info(i, n) = self.call(Request::ScrollTo { y }, t)? {
            self.note(i, n);
        }
        Ok(())
    }

    /// Paint the part of the page in view.
    pub fn render(&mut self) -> Result<Frame, Fallback> {
        let t = self.opts.call_timeout;
        match self.call(Request::Render, t)? {
            Reply::Frame(f, i, n) => {
                self.note(i, n);
                Ok(f)
            }
            _ => Err(self.fail(Fallback::Crashed)),
        }
    }

    /// Pass the pointer to the page (physical pixels from the picture's top left): 0 moved, 1 pressed, 2 released.
    pub fn pointer(&mut self, action: u8, x: f32, y: f32) -> Result<(), Fallback> {
        let t = self.opts.call_timeout;
        if let Reply::Info(i, n) = self.call(Request::Pointer { action, x, y }, t)? {
            self.note(i, n);
        }
        Ok(())
    }

    /// Pass a key to the page: its name and the text it types (empty for keys that type nothing).
    pub fn key(&mut self, key: &str, text: &str) -> Result<(), Fallback> {
        let t = self.opts.call_timeout;
        if let Reply::Info(i, n) = self.call(Request::Key { key: key.into(), text: text.into() }, t)? {
            self.note(i, n);
        }
        Ok(())
    }

    /// The link under a point of the picture (physical pixels from its top left corner), if any.
    pub fn link_at(&mut self, x: f32, y: f32) -> Result<Option<Link>, Fallback> {
        let t = self.opts.call_timeout;
        match self.call(Request::LinkAt { x, y }, t)? {
            Reply::Link(l) => Ok(l),
            _ => Ok(None),
        }
    }
}

impl Drop for PageView {
    fn drop(&mut self) {
        self.to_helper = None; // the helper sees its input close and ends by itself ...
        let _ = self.child.kill(); // ... and if it is stuck, it is ended
        let _ = self.child.wait();
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
    }
}

/// An `href` made absolute against the page's own address. Empty and unreadable ones are not links.
pub fn resolve_link(href: &str) -> Option<Link> {
    let href = href.trim();
    if href.is_empty() {
        return None;
    }
    let base = url::Url::parse(BASE).ok()?;
    let target = base.join(href).ok()?;
    let mut bare = target.clone();
    bare.set_fragment(None);
    let mut base_bare = base;
    base_bare.set_fragment(None);
    Some(Link { same_page: bare == base_bare, url: target.to_string() })
}

/// Which links may be handed to the desktop to open, after the person has clicked a button for it: only web
/// addresses and mail. Never a file, never anything that starts a program.
pub fn openable(url: &str) -> bool {
    match url::Url::parse(url) {
        Ok(u) => matches!(u.scheme(), "http" | "https" | "mailto") && !url.starts_with('-'),
        Err(_) => false,
    }
}

#[cfg(test)]
mod follow_up_tests {
    use super::*;

    #[test]
    fn a_follow_up_is_plain_short_text() {
        assert_eq!(clean_follow_up("  Show me\n\tmore\u{7}  "), "Show me more");
        assert_eq!(clean_follow_up(&"x".repeat(900)).chars().count(), FOLLOW_UP_MAX);
        assert_eq!(clean_follow_up("<b>hi</b>"), "<b>hi</b>", "kept as text, never drawn as markup");
        assert_eq!(clean_follow_up(" \n "), "");
    }

    #[test]
    fn only_allowed_sites_over_https_are_fetched() {
        assert!(fetch_allowed("https://tile.openstreetmap.org/3/4/2.png"));
        assert!(fetch_allowed("https://upload.wikimedia.org/wikipedia/commons/a/a9/Example.jpg"));
        for bad in ["http://tile.openstreetmap.org/1/0/0.png", "https://tile.openstreetmap.org:8443/x", "https://evil.example/x.png", "https://tile.openstreetmap.org.evil.example/x", "https://u:p@upload.wikimedia.org/x", "file:///etc/passwd", "nonsense"] {
            assert!(!fetch_allowed(bad), "{bad}");
        }
        assert!(fetch_type_allowed("image/png") && fetch_type_allowed("application/json; charset=utf-8"));
        assert!(!fetch_type_allowed("text/html") && !fetch_type_allowed("image/svg+xml") && !fetch_type_allowed("application/javascript"));
    }
}
