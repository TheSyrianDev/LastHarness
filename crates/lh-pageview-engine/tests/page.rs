//! The page viewer against real pages: it draws them, it asks the network for nothing, it falls back when the
//! engine cannot cope, and clicks, scrolling and right-to-left behave. Pages are the ones from the spike
//! (`tests/pages`, copied from the html-render spike).
use std::net::TcpListener;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use lh_pageview::{openable, resolve_link, Fallback, Fault, Frame, Options, PageView};

fn opts() -> Options {
    Options { helper: Some(PathBuf::from(env!("CARGO_BIN_EXE_lh-pageview-helper"))), ..Options::default() }
}

fn page(name: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/pages").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn open(html: &str, w: u32, h: u32) -> PageView {
    PageView::open(html, w, h, 1.0, opts()).expect("the page opens")
}

/// How many pixels differ from the most common colour, and how many distinct colours there are (a blank or
/// one-colour picture has none of either).
fn ink(f: &Frame) -> (usize, usize) {
    let mut counts = std::collections::HashMap::new();
    for px in f.rgba.as_chunks::<4>().0 {
        *counts.entry([px[0], px[1], px[2]]).or_insert(0usize) += 1;
    }
    let most = counts.values().copied().max().unwrap_or(0);
    (f.rgba.len() / 4 - most, counts.len())
}

#[test]
fn the_spike_pages_are_drawn_and_are_not_blank_or_one_colour() {
    for name in ["article.html", "layout.html", "arabic.html"] {
        let mut v = open(&page(name), 900, 1000);
        let f = v.render().expect(name);
        assert_eq!((f.width, f.height), (900, 1000));
        assert_eq!(f.rgba.len(), 900 * 1000 * 4);
        let (different, colours) = ink(&f);
        assert!(different > 5_000 && colours > 8, "{name}: {different} pixels differ, {colours} colours");
        assert!(v.info().content_height > 300., "{name}: {}", v.info().content_height);
    }
}

#[test]
fn the_page_title_and_height_are_known() {
    let v = open(&page("article.html"), 800, 600);
    assert_eq!(v.info().title, "Article test");
    assert!(v.info().content_height > 800., "an article taller than the view: {}", v.info().content_height);
}

#[test]
fn the_picture_is_the_size_of_the_view_at_any_scale_and_memory_does_not_grow_with_the_page() {
    let mut v = PageView::open(&page("article.html"), 900, 700, 2.0, opts()).unwrap();
    let f = v.render().unwrap();
    assert_eq!((f.width, f.height, f.rgba.len()), (900, 700, 900 * 700 * 4), "a frame is the view, not the page");
    v.resize(500, 400, 1.0).unwrap();
    let f = v.render().unwrap();
    assert_eq!((f.width, f.height), (500, 400));
    let (different, _) = ink(&f);
    assert!(different > 2_000, "still drawn after a resize");
}

#[test]
fn scrolling_moves_the_picture_and_stays_inside_the_page() {
    let mut v = open(&page("article.html"), 800, 500);
    let top = v.render().unwrap();
    v.scroll_to(600.).unwrap();
    assert_eq!(v.info().scroll_y, 600.);
    let down = v.render().unwrap();
    assert_ne!(top.rgba, down.rgba, "the picture changed");
    v.scroll_to(1.0e9).unwrap();
    let max = v.info().content_height - 500.;
    assert!((v.info().scroll_y - max).abs() < 2., "stops at the end: {} vs {max}", v.info().scroll_y);
    v.scroll_to(-50.).unwrap();
    assert_eq!(v.info().scroll_y, 0.);
    // a resize keeps the place inside the new limits
    v.scroll_to(900.).unwrap();
    v.resize(800, 4000, 1.0).unwrap();
    assert!(v.info().scroll_y <= (v.info().content_height - 4000.).max(0.) + 1.);
}

#[test]
fn right_to_left_text_sits_to_the_right_and_left_to_right_to_the_left() {
    let rtl = r#"<html dir="rtl"><body style="margin:0;background:#fff"><p style="font-size:28px;margin:20px">هذا نص عربي قصير للتجربة</p></body></html>"#;
    let ltr = r#"<html dir="ltr"><body style="margin:0;background:#fff"><p style="font-size:28px;margin:20px">A short line to try it out</p></body></html>"#;
    let centre = |html: &str| -> f32 {
        let mut v = open(html, 800, 120);
        let f = v.render().unwrap();
        let (mut sum, mut n) = (0f64, 0f64);
        for (i, px) in f.rgba.as_chunks::<4>().0.iter().enumerate() {
            if px[0] < 128 {
                sum += (i as u32 % f.width) as f64;
                n += 1.;
            }
        }
        assert!(n > 100., "the line was drawn");
        (sum / n) as f32 / f.width as f32
    };
    let (r, l) = (centre(rtl), centre(ltr));
    assert!(r > 0.6, "right to left text is on the right: {r}");
    assert!(l < 0.5, "left to right text is on the left: {l}");
}

/// A page that tries every way of reaching out: pictures, scripts, styles, frames, fonts, backgrounds, form
/// targets, and a file. A listener on this machine counts any connection made to it.
#[test]
fn a_hostile_page_makes_the_engine_ask_for_nothing_and_every_ask_is_refused() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    let html = format!(
        r#"<!doctype html><html><head><title>hostile</title>
<style>@import url("http://127.0.0.1:{port}/import.css"); body{{background:url("http://127.0.0.1:{port}/bg.png")}}
@font-face{{font-family:X;src:url("http://127.0.0.1:{port}/f.woff2")}} p{{font-family:X}}</style>
<link rel="stylesheet" href="http://127.0.0.1:{port}/a.css"><link rel="stylesheet" href="file:///etc/passwd">
<link rel="icon" href="http://127.0.0.1:{port}/favicon.ico"></head>
<body><img src="http://127.0.0.1:{port}/i.png"><img src="file:///etc/passwd"><img srcset="http://127.0.0.1:{port}/s.png 2x">
<script src="http://127.0.0.1:{port}/s.js"></script><script>fetch("http://127.0.0.1:{port}/fetch"); new WebSocket("ws://127.0.0.1:{port}/ws");</script>
<iframe src="http://127.0.0.1:{port}/frame.html"></iframe><iframe src="file:///etc/passwd"></iframe>
<object data="http://127.0.0.1:{port}/o.swf"></object><video src="http://127.0.0.1:{port}/v.mp4" poster="http://127.0.0.1:{port}/p.png"></video>
<form action="http://127.0.0.1:{port}/post"><input name=a><button>go</button></form>
<svg width=20 height=20><image href="http://127.0.0.1:{port}/svg.png"/></svg>
<p>text with a custom font</p></body></html>"#
    );
    let mut v = open(&html, 800, 600);
    let f = v.render().expect("a hostile page still draws");
    assert!(ink(&f).0 > 100);
    std::thread::sleep(Duration::from_millis(300));
    assert!(listener.accept().is_err(), "the engine connected to a local listener on its own");
    let refused = v.net().urls.clone();
    assert!(v.net().refused >= 3, "it did ask, and every ask was refused: {refused:?}");
    assert!(refused.iter().all(|u| !u.starts_with("data:")));
    assert_eq!(v.net().data_served, 0);
    // a file address is refused like any other
    assert!(refused.iter().any(|u| u.starts_with("file://")), "{refused:?}");
}

#[test]
fn a_picture_inside_the_page_as_a_data_address_is_served_from_memory() {
    use base64::Engine as _;
    let mut png = vec![];
    let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([220, 20, 20, 255]));
    image::DynamicImage::ImageRgba8(img).write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
    let b64 = base64::engine::general_purpose::STANDARD.encode(&png);
    let html = format!(r#"<html><body style="margin:0;background:#fff"><img style="width:300px;height:300px" src="data:image/png;base64,{b64}"></body></html>"#);
    let mut v = open(&html, 400, 400);
    let f = v.render().unwrap();
    assert_eq!(v.net().refused, 0);
    assert!(v.net().data_served >= 1, "the data address was decoded in memory");
    // the 300 x 300 square is red, the rest white
    let at = |x: usize, y: usize| &f.rgba[(y * 400 + x) * 4..(y * 400 + x) * 4 + 3];
    assert_eq!(at(150, 150), &[220, 20, 20], "the picture is there");
    assert_eq!(at(350, 350), &[255, 255, 255]);
}

#[test]
fn links_are_found_by_pointing_resolved_and_never_followed() {
    let html = r##"<html><body style="margin:0"><a href="https://example.com/x?y=1" style="display:block;height:60px;font-size:30px">web</a><a href="#end" style="display:block;height:60px">inside</a><a href="notes/a.html" style="display:block;height:60px">relative</a><a href="mailto:a@b.c" style="display:block;height:60px">mail</a><p style="height:900px">tall</p><a id="end" href="javascript:alert(1)" style="display:block;height:60px">script</a></body></html>"##;
    let mut v = open(html, 600, 400);
    v.render().unwrap();
    let at = |v: &mut PageView, y: f32| v.link_at(10., y).unwrap();
    let web = at(&mut v, 30.).expect("a link at the top");
    assert_eq!((web.url.as_str(), web.same_page), ("https://example.com/x?y=1", false));
    let inside = at(&mut v, 90.).expect("the in-page link");
    assert!(inside.same_page && inside.url.ends_with("#end"), "{inside:?}");
    let rel = at(&mut v, 150.).expect("the relative link");
    assert!(rel.url.ends_with("/page/notes/a.html") && !rel.same_page, "{rel:?}");
    assert_eq!(at(&mut v, 210.).unwrap().url, "mailto:a@b.c");
    assert_eq!(at(&mut v, 500.), None, "plain text is not a link");
    assert_eq!(v.net().refused, 0, "pointing at links asked the network for nothing");
    // after scrolling, the same point is a different place on the page
    v.scroll_to(60.).unwrap();
    assert!(at(&mut v, 30.).expect("the second link is under the pointer now").same_page);
}

#[test]
fn only_web_addresses_and_mail_may_be_opened_in_the_browser() {
    for ok in ["https://example.com/", "http://localhost:8080/x", "mailto:a@b.c"] {
        assert!(openable(ok), "{ok}");
    }
    for no in ["file:///etc/passwd", "javascript:alert(1)", "data:text/html,x", "ftp://x/", "-flag", "/usr/bin/foo", "", "ssh://host", "chrome://settings"] {
        assert!(!openable(no), "{no}");
    }
    assert_eq!(resolve_link("  "), None);
    assert_eq!(resolve_link("").map(|l| l.url), None);
    let l = resolve_link("#top").unwrap();
    assert!(l.same_page);
}

#[test]
fn a_page_over_the_cap_is_not_given_to_the_engine() {
    let big = format!("<html><body>{}</body></html>", "<p>x</p>".repeat(400_000));
    assert!(big.len() > 2 * 1024 * 1024);
    let e = PageView::open(&big, 400, 400, 1.0, opts()).err().expect("too big");
    assert!(matches!(e, Fallback::TooBig { .. }), "{e:?}");
    let t = e.text();
    assert!(t.contains("MB") && t.contains("2.0 MB") && !t.contains("TooBig"), "{t}");
    // the cap can be raised or lowered by whoever opens it
    let small = Options { max_html_bytes: 100, ..opts() };
    assert!(PageView::open(&"<p>0123456789</p>".repeat(20), 100, 100, 1.0, small).is_err());
}

#[test]
fn an_engine_that_panics_or_hangs_gives_a_fallback_and_is_not_asked_again() {
    let html = "<html><body><p>hello</p></body></html>";
    let quick = |f: Fault| Options { load_timeout: Duration::from_millis(1500), call_timeout: Duration::from_millis(1500), fault: f, ..opts() };
    // while loading
    assert_eq!(PageView::open(html, 200, 200, 1.0, quick(Fault::PanicOnLoad)).err(), Some(Fallback::Crashed));
    let t = Instant::now();
    assert_eq!(PageView::open(html, 200, 200, 1.0, quick(Fault::HangOnLoad)).err(), Some(Fallback::TimedOut));
    assert!(t.elapsed() < Duration::from_secs(3), "the time limit is a limit: {:?}", t.elapsed());
    // while painting
    let mut v = PageView::open(html, 200, 200, 1.0, quick(Fault::PanicOnRender)).unwrap();
    assert_eq!(v.render().err(), Some(Fallback::Crashed));
    assert_eq!(v.failed(), Some(&Fallback::Crashed));
    assert_eq!(v.scroll_to(10.).err(), Some(Fallback::Crashed), "nothing more is asked of a crashed engine");
    let mut v = PageView::open(html, 200, 200, 1.0, quick(Fault::HangOnRender)).unwrap();
    let t = Instant::now();
    assert_eq!(v.render().err(), Some(Fallback::TimedOut));
    assert!(t.elapsed() < Duration::from_secs(2));
    assert_eq!(v.render().err(), Some(Fallback::TimedOut), "and a stuck one is not asked again");
    for f in [Fallback::Crashed, Fallback::TimedOut, Fallback::Engine("x".into())] {
        assert!(!f.text().is_empty() && !f.text().contains("Fallback"), "{f:?}");
    }
}

#[test]
fn broken_and_cut_off_pages_still_draw_something_or_fall_back_without_a_crash() {
    for html in ["", "<", "<html><body><div class='a'><p>Hello <b>wor", "<style>body{color:red} .a{background:linear-gradient(90deg,#f00,", "<div>".repeat(3000).as_str(), "<table><tr><td>a<td>b</table><script>x"] {
        let mut v = match PageView::open(html, 300, 200, 1.0, Options { load_timeout: Duration::from_secs(10), ..opts() }) {
            Ok(v) => v,
            Err(f) => {
                assert!(!f.text().is_empty());
                continue;
            }
        };
        match v.render() {
            Ok(f) => assert_eq!(f.rgba.len(), 300 * 200 * 4),
            Err(f) => assert!(!f.text().is_empty()),
        }
    }
}

#[test]
fn a_stack_overflow_in_the_engine_kills_only_the_helper_not_the_app() {
    let html = "<html><body><p>hello</p></body></html>";
    let mut v = PageView::open(html, 200, 200, 1.0, Options { fault: Fault::OverflowOnRender, ..opts() }).unwrap();
    assert_eq!(v.render().err(), Some(Fallback::Crashed));
    // this test is still running: the process that overflowed was the helper
    assert_eq!(v.failed(), Some(&Fallback::Crashed));
    // and a page nested far deeper than any real page opens, or falls back, but never takes this process down
    let deep = format!("<html><body>{}x</body></html>", "<div>".repeat(60_000));
    match PageView::open(&deep, 300, 200, 1.0, Options { load_timeout: Duration::from_secs(20), ..opts() }) {
        Ok(mut v) => {
            let _ = v.render();
        }
        Err(f) => assert!(!f.text().is_empty()),
    }
}

#[test]
fn a_missing_helper_is_a_fallback_with_a_reason_not_a_crash() {
    let e = PageView::open("<p>x</p>", 100, 100, 1.0, Options { helper: Some(PathBuf::from("/nonexistent/lh-pageview-helper")), ..Options::default() }).err().expect("no helper");
    assert!(matches!(e, Fallback::Engine(_)) && e.text().contains("would not start"), "{e:?}");
}

#[test]
fn dropping_the_page_ends_the_helper_and_frees_its_memory() {
    let mut v = open("<p>x</p>", 100, 100);
    v.render().unwrap();
    let pid = v.helper_pid();
    assert!(std::path::Path::new(&format!("/proc/{pid}")).exists(), "the helper runs while the page is open");
    drop(v);
    assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists(), "the helper is gone, and reaped, as soon as the page is dropped");
}

/// The resident memory of the helper with a page open at 900 x 1000, next to the 20 to 26 MB of the spike (that was
/// the whole test program, in the same process).
#[test]
fn memory_per_open_page_is_reported() {
    let mut rows = vec![];
    for name in ["article.html", "layout.html", "arabic.html"] {
        let mut v = open(&page(name), 900, 1000);
        v.render().unwrap();
        let after_first = v.helper_rss_mb().unwrap();
        for y in [300., 600., 900., 0.] {
            v.scroll_to(y).unwrap();
            v.render().unwrap();
        }
        rows.push(format!("{name}: {after_first:.0} MB after the first picture, {:.0} MB after scrolling", v.helper_rss_mb().unwrap()));
        assert!(v.helper_rss_mb().unwrap() < 400., "{name}");
    }
    eprintln!("MEMORY helper process, 900 x 1000: {}", rows.join("; "));
}
