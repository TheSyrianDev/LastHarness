//! The app side of the page viewer, without the engine: what it refuses before starting anything, and how it
//! handles addresses.
use lh_pageview::{openable, resolve_link, Fallback, Options, PageView};

#[test]
fn a_page_over_the_cap_is_refused_before_anything_starts() {
    let big = "x".repeat(2 * 1024 * 1024 + 1);
    let e = PageView::open(&big, 400, 400, 1.0, Options::default()).err().expect("too big");
    assert!(matches!(e, Fallback::TooBig { .. }), "{e:?}");
    let t = e.text();
    assert!(t.contains("2.0 MB") && t.contains("more than"), "{t}");
    assert!(!t.contains("TooBig"), "plain words, not the name of the variant: {t}");
}

#[test]
fn a_missing_helper_is_a_fallback_with_a_reason_not_a_crash() {
    let o = Options { helper: Some("/nonexistent/lh-pageview-helper".into()), ..Options::default() };
    let e = PageView::open("<p>x</p>", 100, 100, 1.0, o).err().expect("no helper");
    assert!(matches!(e, Fallback::Engine(_)) && e.text().contains("would not start"), "{e:?}");
}

#[test]
fn links_are_resolved_against_the_pages_own_address_and_same_page_ones_are_marked() {
    let l = resolve_link("https://example.com/x?y=1").unwrap();
    assert_eq!((l.url.as_str(), l.same_page), ("https://example.com/x?y=1", false));
    let l = resolve_link("#end").unwrap();
    assert!(l.same_page && l.url.ends_with("#end"));
    let l = resolve_link("notes/a.html").unwrap();
    assert!(!l.same_page && l.url.ends_with("/page/notes/a.html"));
    assert_eq!(resolve_link("  "), None);
    assert_eq!(resolve_link("").map(|l| l.url), None);
    assert_eq!(resolve_link("mailto:a@b.c").unwrap().url, "mailto:a@b.c");
}

#[test]
fn only_web_addresses_and_mail_may_be_opened_in_the_browser() {
    for ok in ["https://example.com/", "http://localhost:8080/x", "mailto:a@b.c"] {
        assert!(openable(ok), "{ok}");
    }
    for no in ["file:///etc/passwd", "javascript:alert(1)", "data:text/html,x", "ftp://x/", "-flag", "/usr/bin/foo", "", "ssh://host", "chrome://settings", "file:///home/me/page.html#top"] {
        assert!(!openable(no), "{no}");
    }
}
