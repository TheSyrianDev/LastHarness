//! The Servo helper, driven as the app drives it: load a page, check its script ran, the outside was refused,
//! links are found, and a frame comes back the right size.
use std::io::{BufReader, BufWriter};
use std::process::{Command, Stdio};

use lh_pageview::proto::{read_reply, write_request, Fault, Reply, Request};

const PAGE: &str = r#"<!doctype html><html><head><title>Servo test</title>
<link rel="stylesheet" href="https://example.com/a.css"></head>
<body style="margin:0;background:#fff">
<div id="box" style="width:300px;height:3000px;background:#fff"></div>
<a id="l" href="https://example.org/" style="position:absolute;left:0;top:0;width:100px;height:40px;display:block">link</a>
<img src="http://example.net/i.png">
<script>document.title = "ran"; document.getElementById("box").style.background = "rgb(0, 0, 255)";</script>
</body></html>"#;

#[test]
fn a_page_runs_its_script_and_reaches_nothing_outside() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_lh-pageview-servo")).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    let mut to = BufWriter::new(child.stdin.take().unwrap());
    let mut from = BufReader::new(child.stdout.take().unwrap());

    write_request(&mut to, &Request::Load { w: 400, h: 300, scale: 1.0, fault: Fault::None, html: PAGE.as_bytes().to_vec() }).unwrap();
    let Reply::Info(info, net) = read_reply(&mut from).unwrap() else { panic!("no info") };
    assert_eq!(info.title, "ran", "the script changed the title");
    assert!(info.content_height >= 3000.0, "{}", info.content_height);
    assert!(net.refused >= 2, "{net:?}");
    assert!(net.urls.iter().all(|u| u.contains("example.")), "{net:?}");

    write_request(&mut to, &Request::LinkAt { x: 20.0, y: 20.0 }).unwrap();
    let Reply::Link(Some(l)) = read_reply(&mut from).unwrap() else { panic!("no link") };
    assert_eq!(l.url, "https://example.org/");

    write_request(&mut to, &Request::ScrollTo { y: 500.0 }).unwrap();
    let Reply::Info(info, _) = read_reply(&mut from).unwrap() else { panic!("no info") };
    assert_eq!(info.scroll_y, 500.0);

    write_request(&mut to, &Request::Render).unwrap();
    let Reply::Frame(f, _, _) = read_reply(&mut from).unwrap() else { panic!("no frame") };
    assert_eq!((f.width, f.height, f.rgba.len()), (400, 300, 400 * 300 * 4));
    // the script painted the box blue: a pixel inside it, below the link
    let px = &f.rgba[((100 * 400) + 50) * 4..][..4];
    assert!(px[2] > 200 && px[0] < 60, "{px:?}");
    drop(to);
    let _ = child.wait();
}
