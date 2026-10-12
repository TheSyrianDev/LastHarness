//! Throwaway: render one local HTML file to a PNG with Blitz (Stylo + Taffy + Parley + Vello CPU), no window.
//! Every request the page makes for another file or address goes to a net provider that refuses it and says so.
//! Usage: hr-blitz PAGE.html OUT.png [WIDTH] [HEIGHT]
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyrender::render_to_buffer;
use anyrender_vello_cpu::VelloCpuImageRenderer;
use blitz_dom::DocumentConfig;
use blitz_html::HtmlDocument;
use blitz_traits::navigation::{NavigationOptions, NavigationProvider};
use blitz_traits::net::{NetHandler, NetProvider, Request};
use blitz_traits::shell::{ColorScheme, Viewport};

/// Refuses everything and keeps a list of what was asked for.
#[derive(Default)]
struct Deny {
    asked: Mutex<Vec<String>>,
}
impl NetProvider for Deny {
    fn fetch(&self, _doc_id: usize, request: Request, handler: Box<dyn NetHandler>) {
        self.asked.lock().unwrap().push(request.url.to_string());
        // answer with nothing, so the page does not wait for a resource that will never come
        handler.bytes(request.url.to_string(), blitz_traits::net::Bytes::new());
    }
}

#[derive(Default)]
struct Nav {
    clicked: Mutex<Vec<String>>,
}
impl NavigationProvider for Nav {
    fn navigate_to(&self, o: NavigationOptions) {
        self.clicked.lock().unwrap().push(o.url.to_string());
    }
}

fn rss_kb() -> (u64, u64) {
    let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let f = |k: &str| s.lines().find(|l| l.starts_with(k)).and_then(|l| l.split_whitespace().nth(1)).and_then(|v| v.parse().ok()).unwrap_or(0);
    (f("VmRSS:"), f("VmHWM:"))
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.get(1).map(String::as_str) == Some("--interact") {
        return interact(&a[2], &a[3]);
    }
    let (page, out) = (&a[1], &a[2]);
    let w: u32 = a.get(3).and_then(|v| v.parse().ok()).unwrap_or(800);
    let h: u32 = a.get(4).and_then(|v| v.parse().ok()).unwrap_or(1000);
    let html = std::fs::read_to_string(page).expect("page");
    let deny = Arc::new(Deny::default());
    let nav = Arc::new(Nav::default());
    let t0 = Instant::now();
    let cfg = DocumentConfig {
        viewport: Some(Viewport::new(w, h, 1.0, ColorScheme::Light)),
        // the base is the folder of the one approved file; every fetch is still refused by `Deny`
        base_url: Some(format!("file://{}/", std::path::Path::new(page).canonicalize().unwrap().parent().unwrap().display())),
        net_provider: Some(deny.clone()),
        navigation_provider: Some(nav.clone()),
        ..Default::default()
    };
    let mut doc = HtmlDocument::from_html(&html, cfg);
    let parsed = t0.elapsed();
    doc.resolve(0.0);
    let laid_out = t0.elapsed();
    let content_h = doc.root_element().final_layout().size.height.ceil() as u32;
    let full_h = content_h.clamp(h, 6000);
    doc.set_viewport(Viewport::new(w, full_h, 1.0, ColorScheme::Light));
    doc.resolve(0.0);
    let buf = render_to_buffer::<VelloCpuImageRenderer, _>(|scene| blitz_paint::paint_scene(scene, &mut doc, 1.0, w, full_h, 0, 0), w, full_h);
    let done = t0.elapsed();
    image::save_buffer(out, &buf, w, full_h, image::ColorType::Rgba8).expect("png");
    let (rss, hwm) = rss_kb();
    println!(
        "{}",
        serde_json_lite(&[
            ("page", page.to_string()),
            ("parse_ms", format!("{}", parsed.as_millis())),
            ("layout_ms", format!("{}", laid_out.as_millis())),
            ("total_ms", format!("{}", done.as_millis())),
            ("content_height_px", content_h.to_string()),
            ("rss_mb", format!("{:.0}", rss as f64 / 1024.0)),
            ("peak_rss_mb", format!("{:.0}", hwm as f64 / 1024.0)),
            ("refused_requests", format!("{:?}", deny.asked.lock().unwrap())),
        ])
    );
}

fn serde_json_lite(kv: &[(&str, String)]) -> String {
    format!("{{{}}}", kv.iter().map(|(k, v)| format!("\"{k}\": \"{}\"", v.replace('"', "'"))).collect::<Vec<_>>().join(", "))
}

/// Scrolling and links in a short viewport: scroll, repaint, time the repaint, and find a link by pointing at it.
fn interact(page: &str, out_prefix: &str) {
    let (w, h) = (800u32, 500u32);
    let html = std::fs::read_to_string(page).expect("page");
    let deny = Arc::new(Deny::default());
    let nav = Arc::new(Nav::default());
    let cfg = DocumentConfig { viewport: Some(Viewport::new(w, h, 1.0, ColorScheme::Light)), net_provider: Some(deny), navigation_provider: Some(nav), base_url: Some("file:///approved/page.html".into()), ..Default::default() };
    let mut doc = HtmlDocument::from_html(&html, cfg);
    doc.resolve(0.0);
    let paint = |doc: &mut HtmlDocument| render_to_buffer::<VelloCpuImageRenderer, _>(|scene| blitz_paint::paint_scene(scene, doc, 1.0, w, h, 0, 0), w, h);
    let t = Instant::now();
    let buf = paint(&mut doc);
    println!("first frame {}x{}: {} ms", w, h, t.elapsed().as_millis());
    image::save_buffer(format!("{out_prefix}-top.png"), &buf, w, h, image::ColorType::Rgba8).unwrap();
    // find a link by pointing at the page: which href is under each probe point?
    let mut found = vec![];
    for y in (0..h).step_by(6) {
        for x in (0..w).step_by(8) {
            if let Some(hit) = doc.hit(x as f32, y as f32) {
                let mut id = Some(hit.node_id);
                while let Some(n) = id.and_then(|i| doc.get_node(i)) {
                    if let Some(href) = n.attr(blitz_dom::local_name!("href")) {
                        if !found.iter().any(|(_, _, hh): &(u32, u32, String)| hh == href) {
                            found.push((x, y, href.to_string()));
                        }
                    }
                    id = n.parent;
                }
            }
        }
    }
    println!("links found by pointing (top frame): {found:?}");
    // scroll down and repaint, timing the repaints
    doc.scroll_viewport_by(0.0, -700.0);
    doc.resolve(0.0);
    let t = Instant::now();
    let mut buf = vec![];
    for _ in 0..30 {
        buf = paint(&mut doc);
    }
    println!("scrolled to y={:.0}; repaint of a {}x{} frame: {:.1} ms each (30 runs)", doc.viewport_scroll().y, w, h, t.elapsed().as_secs_f64() * 1000.0 / 30.0);
    let mut found2: Vec<(u32, u32, String)> = vec![];
    let sy = doc.viewport_scroll().y as u32;
    for y in (0..h).step_by(6) {
        for x in (0..w).step_by(8) {
            // pointer positions are in the frame; the host adds the scroll offset to get a position in the page
            if let Some(hit) = doc.hit(x as f32, (y + sy) as f32) {
                let mut id = Some(hit.node_id);
                while let Some(n) = id.and_then(|i| doc.get_node(i)) {
                    if let Some(href) = n.attr(blitz_dom::local_name!("href")) {
                        if !found2.iter().any(|(_, _, hh)| hh == href) {
                            found2.push((x, y, href.to_string()));
                        }
                    }
                    id = n.parent;
                }
            }
        }
    }
    println!("links found by pointing (after scrolling): {found2:?}");
    for (x, y) in [(110u32, 400u32), (110, 380), (150, 420)] {
        match doc.hit(x as f32, y as f32) {
            Some(hit) => {
                let n = doc.get_node(hit.node_id).unwrap();
                println!("hit ({x},{y}) -> node {} text={} tag={:?} parent_tag={:?}", hit.node_id, hit.is_text, n.element_data().map(|e| e.name.local.to_string()), n.parent.and_then(|p| doc.get_node(p)).and_then(|p| p.element_data().map(|e| e.name.local.to_string())));
            }
            None => println!("hit ({x},{y}) -> nothing"),
        }
    }
    image::save_buffer(format!("{out_prefix}-scrolled.png"), &buf, w, h, image::ColorType::Rgba8).unwrap();
    let (rss, hwm) = rss_kb();
    println!("rss {} MB, peak {} MB", rss / 1024, hwm / 1024);
}
