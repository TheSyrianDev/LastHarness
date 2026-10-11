//! Headless Servo in its own process. Renders one URL into a software buffer and writes it as a PNG, once
//! or again every --every-ms milliseconds (written to a temp file and renamed, so a reader never sees a half file).
//! Usage: servo-helper --url URL --out FILE.png [--size 900x600] [--every-ms 500] [--wait-ms 1500] [--max-secs 40]
use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use dpi::PhysicalSize;
use servo::{EventLoopWaker, LoadStatus, RenderingContext, Servo, ServoBuilder, SoftwareRenderingContext, WebView, WebViewBuilder, WebViewDelegate};
use url::Url;

#[derive(Clone)]
struct Waker(Arc<AtomicBool>);
impl EventLoopWaker for Waker {
    fn clone_box(&self) -> Box<dyn EventLoopWaker> { Box::new(self.clone()) }
    fn wake(&self) { self.0.store(true, Ordering::Relaxed); }
}

struct Delegate { status: Cell<Option<LoadStatus>>, frames: Cell<u32> }
impl WebViewDelegate for Delegate {
    fn notify_load_status_changed(&self, _: WebView, s: LoadStatus) { self.status.set(Some(s)); }
    fn notify_new_frame_ready(&self, _: WebView) { self.frames.set(self.frames.get() + 1); }
}

fn arg(name: &str) -> Option<String> {
    let mut a = std::env::args().skip(1);
    while let Some(x) = a.next() { if x == name { return a.next(); } }
    None
}

fn main() {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    let url = Url::parse(&arg("--url").expect("--url")).expect("url");
    let out = arg("--out").expect("--out");
    let (w, h) = arg("--size").and_then(|s| s.split_once('x').map(|(a, b)| (a.parse().unwrap_or(900), b.parse().unwrap_or(600)))).unwrap_or((900u32, 600u32));
    let every = arg("--every-ms").and_then(|s| s.parse::<u64>().ok());
    let wait = Duration::from_millis(arg("--wait-ms").and_then(|s| s.parse().ok()).unwrap_or(1500));
    let max = Duration::from_secs(arg("--max-secs").and_then(|s| s.parse().ok()).unwrap_or(40));

    let rc = Rc::new(SoftwareRenderingContext::new(PhysicalSize { width: w, height: h }).expect("software rendering context"));
    rc.make_current().expect("make current");
    let woke = Arc::new(AtomicBool::new(false));
    let servo: Servo = ServoBuilder::default().event_loop_waker(Box::new(Waker(woke.clone()))).build();
    let delegate = Rc::new(Delegate { status: Cell::new(None), frames: Cell::new(0) });
    let webview = WebViewBuilder::new(&servo, rc.clone()).url(url).delegate(delegate.clone()).build();

    let t0 = Instant::now();
    let mut complete_at = None;
    let mut painted = 0;
    while t0.elapsed() < max {
        servo.spin_event_loop();
        if delegate.frames.get() != painted { painted = delegate.frames.get(); webview.paint(); }
        let ours = webview.url().map_or(false, |u| u.scheme().starts_with("http") || u.scheme() == "file");
        if ours && webview.load_status() == LoadStatus::Complete && complete_at.is_none() {
            complete_at = Some(t0.elapsed());
            eprintln!("loaded in {:?}", complete_at.unwrap());
        }
        if let Some(c) = complete_at { if t0.elapsed() > c + wait { break; } }
        std::thread::sleep(Duration::from_millis(5));
    }
    if complete_at.is_none() { eprintln!("not loaded within {:?}", max); }

    let shoot = |webview: &WebView, servo: &Servo| {
        servo.spin_event_loop();
        webview.paint();
        let rect = servo::DeviceIntRect::from_size(euclid::Size2D::new(w as i32, h as i32));
        match rc.read_to_image(rect) {
            Some(img) => { let tmp = format!("{out}.tmp"); if img.save_with_format(&tmp, image::ImageFormat::Png).is_ok() { let _ = std::fs::rename(&tmp, &out); } }
            None => eprintln!("read_to_image returned nothing"),
        }
    };
    shoot(&webview, &servo);
    eprintln!("frames painted: {}", delegate.frames.get());
    if let Some(ms) = every {
        eprintln!("ready");
        loop {
            let t = Instant::now();
            while t.elapsed() < Duration::from_millis(ms) { servo.spin_event_loop(); std::thread::sleep(Duration::from_millis(10)); }
            shoot(&webview, &servo);
        }
    }
}

