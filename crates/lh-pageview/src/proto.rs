//! The few messages between the app and the helper process. Plain length-prefixed little-endian bytes on the
//! helper's standard input and output: no serialisation library, nothing the helper could be tricked into running.

use std::io::{self, Read, Write};

use crate::{Frame, Info, Link};

/// Sanity limits on what is read, so a broken helper cannot make the app allocate without end.
pub const MAX_STRING: usize = 16 * 1024 * 1024;
pub const MAX_FRAME_BYTES: usize = 128 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    None = 0,
    PanicOnLoad = 1,
    HangOnLoad = 2,
    PanicOnRender = 3,
    HangOnRender = 4,
    /// Recursion with no end, to prove a stack overflow in the helper leaves the app alone.
    OverflowOnRender = 5,
}

impl Fault {
    pub fn from_u8(b: u8) -> Fault {
        match b {
            1 => Fault::PanicOnLoad,
            2 => Fault::HangOnLoad,
            3 => Fault::PanicOnRender,
            4 => Fault::HangOnRender,
            5 => Fault::OverflowOnRender,
            _ => Fault::None,
        }
    }
}

pub enum Request {
    Load { w: u32, h: u32, scale: f32, fault: Fault, html: Vec<u8> },
    Resize { w: u32, h: u32, scale: f32 },
    ScrollTo { y: f32 },
    Render,
    LinkAt { x: f32, y: f32 },
    /// The pointer, in physical pixels of the picture: 0 moved, 1 pressed, 2 released. A helper that cannot
    /// pass input on (Blitz) just answers with the page info.
    Pointer { action: u8, x: f32, y: f32 },
    /// A key pressed: its name as the toolkit gives it ("a", "backspace", "enter", "left"), and the text it types.
    Key { key: String, text: String },
    /// What the app fetched for a request the page made to an allowed site (see `NetReport::fetch`). A status of 0
    /// means it could not be fetched; the page gets an error.
    Deliver { id: u32, status: u16, content_type: String, body: Vec<u8> },
}

/// What the page asked the network for: counts, and the first few addresses.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NetReport {
    pub refused: u32,
    pub data_served: u32,
    pub urls: Vec<String>,
    /// Requests to allowed sites the page made since the last answer, by number: the helper has no network, so
    /// the app fetches them and hands the bytes back with `Request::Deliver`.
    pub fetch: Vec<(u32, String)>,
}

pub enum Reply {
    Info(Info, NetReport),
    Frame(Frame, Info, NetReport),
    Link(Option<Link>),
    Crashed,
}

fn w32(w: &mut impl Write, v: u32) -> io::Result<()> {
    w.write_all(&v.to_le_bytes())
}
fn wf(w: &mut impl Write, v: f32) -> io::Result<()> {
    w.write_all(&v.to_le_bytes())
}
fn r32(r: &mut impl Read) -> io::Result<u32> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}
fn rf(r: &mut impl Read) -> io::Result<f32> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b)?;
    Ok(f32::from_le_bytes(b))
}
fn r8(r: &mut impl Read) -> io::Result<u8> {
    let mut b = [0u8; 1];
    r.read_exact(&mut b)?;
    Ok(b[0])
}
fn wbytes(w: &mut impl Write, b: &[u8]) -> io::Result<()> {
    w32(w, b.len() as u32)?;
    w.write_all(b)
}
fn rbytes(r: &mut impl Read, max: usize) -> io::Result<Vec<u8>> {
    let n = r32(r)? as usize;
    if n > max {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "message too long"));
    }
    let mut v = vec![0u8; n];
    r.read_exact(&mut v)?;
    Ok(v)
}
fn rstring(r: &mut impl Read) -> io::Result<String> {
    Ok(String::from_utf8_lossy(&rbytes(r, MAX_STRING)?).into_owned())
}

pub fn write_request(w: &mut impl Write, q: &Request) -> io::Result<()> {
    match q {
        Request::Load { w: pw, h, scale, fault, html } => {
            w.write_all(&[1])?;
            w32(w, *pw)?;
            w32(w, *h)?;
            wf(w, *scale)?;
            w.write_all(&[*fault as u8])?;
            wbytes(w, html)?;
        }
        Request::Resize { w: pw, h, scale } => {
            w.write_all(&[2])?;
            w32(w, *pw)?;
            w32(w, *h)?;
            wf(w, *scale)?;
        }
        Request::ScrollTo { y } => {
            w.write_all(&[3])?;
            wf(w, *y)?;
        }
        Request::Render => w.write_all(&[4])?,
        Request::LinkAt { x, y } => {
            w.write_all(&[5])?;
            wf(w, *x)?;
            wf(w, *y)?;
        }
        Request::Pointer { action, x, y } => {
            w.write_all(&[6, *action])?;
            wf(w, *x)?;
            wf(w, *y)?;
        }
        Request::Key { key, text } => {
            w.write_all(&[7])?;
            wbytes(w, key.as_bytes())?;
            wbytes(w, text.as_bytes())?;
        }
        Request::Deliver { id, status, content_type, body } => {
            w.write_all(&[8])?;
            w32(w, *id)?;
            w32(w, *status as u32)?;
            wbytes(w, content_type.as_bytes())?;
            wbytes(w, body)?;
        }
    }
    w.flush()
}

/// `Ok(None)` when the other side closed the pipe.
pub fn read_request(r: &mut impl Read) -> io::Result<Option<Request>> {
    let tag = match r8(r) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    };
    Ok(Some(match tag {
        1 => {
            let (w, h, scale) = (r32(r)?, r32(r)?, rf(r)?);
            let fault = Fault::from_u8(r8(r)?);
            Request::Load { w, h, scale, fault, html: rbytes(r, 64 * 1024 * 1024)? }
        }
        2 => Request::Resize { w: r32(r)?, h: r32(r)?, scale: rf(r)? },
        3 => Request::ScrollTo { y: rf(r)? },
        4 => Request::Render,
        5 => Request::LinkAt { x: rf(r)?, y: rf(r)? },
        6 => Request::Pointer { action: r8(r)?, x: rf(r)?, y: rf(r)? },
        7 => Request::Key { key: rstring(r)?, text: rstring(r)? },
        8 => Request::Deliver { id: r32(r)?, status: r32(r)?.min(999) as u16, content_type: rstring(r)?, body: rbytes(r, crate::FETCH_MAX_BYTES)? },
        _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "unknown request")),
    }))
}

fn write_info(w: &mut impl Write, i: &Info, n: &NetReport) -> io::Result<()> {
    wbytes(w, i.title.as_bytes())?;
    wf(w, i.content_height)?;
    wf(w, i.scroll_y)?;
    wbytes(w, i.follow_up.as_bytes())?;
    w32(w, n.refused)?;
    w32(w, n.data_served)?;
    w32(w, n.urls.len() as u32)?;
    for u in &n.urls {
        wbytes(w, u.as_bytes())?;
    }
    w32(w, n.fetch.len() as u32)?;
    for (id, u) in &n.fetch {
        w32(w, *id)?;
        wbytes(w, u.as_bytes())?;
    }
    Ok(())
}

fn read_info(r: &mut impl Read) -> io::Result<(Info, NetReport)> {
    let title = rstring(r)?;
    let (content_height, scroll_y) = (rf(r)?, rf(r)?);
    let follow_up = rstring(r)?;
    let (refused, data_served, n) = (r32(r)?, r32(r)?, r32(r)?);
    if n > 1000 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "too many addresses"));
    }
    let urls = (0..n).map(|_| rstring(r)).collect::<io::Result<Vec<_>>>()?;
    let k = r32(r)?;
    if k > 1000 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "too many fetches"));
    }
    let fetch = (0..k).map(|_| Ok((r32(r)?, rstring(r)?))).collect::<io::Result<Vec<_>>>()?;
    Ok((Info { title, content_height, scroll_y, follow_up }, NetReport { refused, data_served, urls, fetch }))
}

pub fn write_reply(w: &mut impl Write, p: &Reply) -> io::Result<()> {
    match p {
        Reply::Info(i, n) => {
            w.write_all(&[0])?;
            write_info(w, i, n)?;
        }
        Reply::Frame(f, i, n) => {
            w.write_all(&[1])?;
            w32(w, f.width)?;
            w32(w, f.height)?;
            write_info(w, i, n)?;
            wbytes(w, &f.rgba)?;
        }
        Reply::Link(l) => {
            w.write_all(&[2])?;
            match l {
                Some(l) => {
                    w.write_all(&[1, l.same_page as u8])?;
                    wbytes(w, l.url.as_bytes())?;
                }
                None => w.write_all(&[0])?,
            }
        }
        Reply::Crashed => w.write_all(&[3])?,
    }
    w.flush()
}

pub fn read_reply(r: &mut impl Read) -> io::Result<Reply> {
    Ok(match r8(r)? {
        0 => {
            let (i, n) = read_info(r)?;
            Reply::Info(i, n)
        }
        1 => {
            let (width, height) = (r32(r)?, r32(r)?);
            let (i, n) = read_info(r)?;
            let rgba = rbytes(r, MAX_FRAME_BYTES)?;
            if rgba.len() as u64 != width as u64 * height as u64 * 4 {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "frame size does not match"));
            }
            Reply::Frame(Frame { width, height, rgba }, i, n)
        }
        2 => {
            if r8(r)? == 1 {
                let same_page = r8(r)? == 1;
                Reply::Link(Some(Link { same_page, url: rstring(r)? }))
            } else {
                Reply::Link(None)
            }
        }
        3 => Reply::Crashed,
        _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "unknown reply")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_and_replies_survive_the_wire() {
        let mut buf = vec![];
        write_request(&mut buf, &Request::Load { w: 900, h: 1000, scale: 1.5, fault: Fault::HangOnRender, html: b"<p>x".to_vec() }).unwrap();
        write_request(&mut buf, &Request::LinkAt { x: 1.5, y: 2.5 }).unwrap();
        write_request(&mut buf, &Request::Render).unwrap();
        let mut r = buf.as_slice();
        match read_request(&mut r).unwrap().unwrap() {
            Request::Load { w, h, scale, fault, html } => assert_eq!((w, h, scale, fault, html), (900, 1000, 1.5, Fault::HangOnRender, b"<p>x".to_vec())),
            _ => panic!("load"),
        }
        assert!(matches!(read_request(&mut r).unwrap().unwrap(), Request::LinkAt { x, y } if x == 1.5 && y == 2.5));
        assert!(matches!(read_request(&mut r).unwrap().unwrap(), Request::Render));
        assert!(read_request(&mut r).unwrap().is_none(), "a closed pipe is the end, not an error");

        let info = Info { title: "عنوان".into(), content_height: 1234.5, scroll_y: 6.0, follow_up: "Show more".into() };
        let net = NetReport { refused: 3, data_served: 1, urls: vec!["http://a/".into(), "file:///x".into()], fetch: vec![(4, "https://tile.openstreetmap.org/1/0/0.png".into())] };
        let frame = Frame { width: 2, height: 1, rgba: vec![1, 2, 3, 4, 5, 6, 7, 8] };
        let mut buf = vec![];
        write_reply(&mut buf, &Reply::Info(info.clone(), net.clone())).unwrap();
        write_reply(&mut buf, &Reply::Frame(frame, info.clone(), net.clone())).unwrap();
        write_reply(&mut buf, &Reply::Link(Some(Link { url: "https://x/".into(), same_page: false }))).unwrap();
        write_reply(&mut buf, &Reply::Link(None)).unwrap();
        write_reply(&mut buf, &Reply::Crashed).unwrap();
        let mut r = buf.as_slice();
        assert!(matches!(read_reply(&mut r).unwrap(), Reply::Info(i, n) if i == info && n == net));
        assert!(matches!(read_reply(&mut r).unwrap(), Reply::Frame(f, i, n) if f.rgba.len() == 8 && i == info && n == net));
        assert!(matches!(read_reply(&mut r).unwrap(), Reply::Link(Some(l)) if l.url == "https://x/" && !l.same_page));
        assert!(matches!(read_reply(&mut r).unwrap(), Reply::Link(None)));
        assert!(matches!(read_reply(&mut r).unwrap(), Reply::Crashed));
    }

    #[test]
    fn pointer_and_keys_survive_the_wire() {
        let mut buf = vec![];
        write_request(&mut buf, &Request::Pointer { action: 1, x: 12.5, y: 40. }).unwrap();
        write_request(&mut buf, &Request::Key { key: "a".into(), text: "a".into() }).unwrap();
        let mut r = &buf[..];
        assert!(matches!(read_request(&mut r).unwrap(), Some(Request::Pointer { action: 1, x, y }) if x == 12.5 && y == 40.));
        assert!(matches!(read_request(&mut r).unwrap(), Some(Request::Key { key, text }) if key == "a" && text == "a"));
        let mut buf = vec![];
        write_request(&mut buf, &Request::Deliver { id: 9, status: 200, content_type: "image/png".into(), body: vec![1, 2] }).unwrap();
        assert!(matches!(read_request(&mut &buf[..]).unwrap(), Some(Request::Deliver { id: 9, status: 200, content_type, body }) if content_type == "image/png" && body == [1, 2]));
    }

    #[test]
    fn a_broken_or_greedy_message_is_an_error_not_an_allocation() {
        // a string that claims to be 4 GB
        let bad = [0u8, 0xff, 0xff, 0xff, 0xff];
        assert!(read_reply(&mut &bad[..]).is_err());
        // a frame whose size does not match its bytes
        let mut buf = vec![];
        write_reply(&mut buf, &Reply::Frame(Frame { width: 10, height: 10, rgba: vec![0; 8] }, Info { title: String::new(), content_height: 0., scroll_y: 0., follow_up: String::new() }, NetReport::default())).unwrap();
        assert!(read_reply(&mut buf.as_slice()).is_err());
        assert!(read_request(&mut &[9u8][..]).is_err(), "an unknown request is refused");
    }
}
