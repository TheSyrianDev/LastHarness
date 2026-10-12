//! Throwaway: render one local HTML file to a PNG with litehtml (C++ engine) and its CPU pixel-buffer backend
//! (tiny-skia for drawing, cosmic-text for text). The app, not the engine, decides what to fetch: images the page
//! asks for are only listed here and never loaded.
//! Usage: hr-litehtml PAGE.html OUT.png [WIDTH]
use std::time::Instant;

use litehtml::pixbuf::PixbufContainer;
use litehtml::{DrawContext, Position};

fn rss_kb() -> (u64, u64) {
    let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let f = |k: &str| s.lines().find(|l| l.starts_with(k)).and_then(|l| l.split_whitespace().nth(1)).and_then(|v| v.parse().ok()).unwrap_or(0);
    (f("VmRSS:"), f("VmHWM:"))
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (page, out) = (&a[1], &a[2]);
    let w: u32 = a.get(3).and_then(|v| v.parse().ok()).unwrap_or(800);
    let html = std::fs::read_to_string(page).expect("page");
    let t0 = Instant::now();
    // first pass: how tall is the page at this width?
    let mut probe = PixbufContainer::new(w, 100);
    let height = {
        let mut doc = litehtml::Document::from_html(&html, &mut probe, None, None).expect("parse");
        doc.render(w as f32);
        doc.height().ceil() as u32
    };
    let h = height.clamp(200, 6000);
    let laid_out = t0.elapsed();
    let mut c = PixbufContainer::new(w, h);
    let pending;
    {
        let mut doc = litehtml::Document::from_html(&html, &mut c, None, None).expect("parse");
        doc.render(w as f32);
        doc.draw(DrawContext::default(), 0.0, 0.0, Some(Position { x: 0.0, y: 0.0, width: w as f32, height: h as f32 }));
    }
    pending = c.take_pending_images();
    let done = t0.elapsed();
    // the pixmap is premultiplied; the pages here are opaque, so it can be saved as it is
    image::save_buffer(out, c.pixels(), w, h, image::ColorType::Rgba8).expect("png");
    let (rss, hwm) = rss_kb();
    println!(
        "{{\"page\": \"{page}\", \"layout_ms\": {}, \"total_ms\": {}, \"content_height_px\": {height}, \"rss_mb\": {:.0}, \"peak_rss_mb\": {:.0}, \"images_the_page_asked_for_and_were_not_loaded\": {:?}}}",
        laid_out.as_millis(),
        done.as_millis(),
        rss as f64 / 1024.0,
        hwm as f64 / 1024.0,
        pending
    );
}
