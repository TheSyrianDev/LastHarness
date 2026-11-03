//! The page drawing helper: a separate process that holds the engine, so a crash, a stack overflow, a runaway loop or
//! a memory blow-up in it can never take the app down; the app just kills it. It reads requests on standard input and
//! answers on standard output (see `proto`). It limits itself before it reads anything: no new privileges, a ceiling
//! on memory, no core dumps, and, where the system allows it without any privilege, a network namespace with no
//! network in it. The engine also refuses every request by itself; this is a second wall.

use std::io::{BufReader, BufWriter, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::time::Duration;

use lh_pageview::proto::{read_request, write_reply, Fault, NetReport, Reply, Request};
use lh_pageview_engine::engine::Engine;

const MEMORY_CEILING: libc::rlim_t = 3 * 1024 * 1024 * 1024;

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
        // no network: a new network namespace holds nothing but a loopback that is down. Needs no privilege where user
        // namespaces are allowed; where they are not, this quietly does nothing and the engine's own refusal remains.
        libc::unshare(libc::CLONE_NEWUSER | libc::CLONE_NEWNET);
    }
}

fn report(e: &Engine) -> NetReport {
    NetReport { refused: e.stats.refused() as u32, data_served: e.stats.data_served() as u32, urls: e.stats.refused_urls(), fetch: vec![] }
}

fn hang() -> ! {
    loop {
        std::thread::sleep(Duration::from_secs(3600));
    }
}

#[allow(unconditional_recursion)]
fn overflow(n: u64) -> u64 {
    let pad = [n as u8; 256];
    std::hint::black_box(&pad);
    overflow(n + 1) + pad[0] as u64
}

fn run() {
    let mut input = BufReader::new(std::io::stdin().lock());
    let mut output = BufWriter::new(std::io::stdout().lock());
    let mut engine: Option<Engine> = None;
    let mut fault = Fault::None;
    while let Ok(Some(req)) = read_request(&mut input) {
        let result = catch_unwind(AssertUnwindSafe(|| -> Option<Reply> {
            match req {
                Request::Load { w, h, scale, fault: f, html } => {
                    fault = f;
                    match fault {
                        Fault::PanicOnLoad => panic!("deliberate failure while loading (test)"),
                        Fault::HangOnLoad => hang(),
                        _ => {}
                    }
                    let html = String::from_utf8_lossy(&html).into_owned();
                    let e = Engine::load(&html, w, h, scale);
                    let reply = Reply::Info(e.info(), report(&e));
                    engine = Some(e);
                    Some(reply)
                }
                Request::Resize { w, h, scale } => engine.as_mut().map(|e| Reply::Info(e.resize(w, h, scale), report(e))),
                Request::ScrollTo { y } => engine.as_mut().map(|e| Reply::Info(e.scroll_to(y), report(e))),
                Request::Render => {
                    match fault {
                        Fault::PanicOnRender => panic!("deliberate failure while painting (test)"),
                        Fault::HangOnRender => hang(),
                        Fault::OverflowOnRender => {
                            std::hint::black_box(overflow(0));
                        }
                        _ => {}
                    }
                    engine.as_mut().map(|e| {
                        let (f, i) = e.render();
                        Reply::Frame(f, i, report(e))
                    })
                }
                Request::LinkAt { x, y } => engine.as_ref().map(|e| Reply::Link(e.link_at(x, y))),
                // Blitz runs no scripts, so input has nothing to drive: answer with the page as it is
                Request::Pointer { .. } | Request::Key { .. } | Request::Deliver { .. } => engine.as_mut().map(|e| Reply::Info(e.scroll_to(e.info().scroll_y), report(e))),
            }
        }));
        let reply = match result {
            Ok(Some(r)) => r,
            Ok(None) => Reply::Crashed,
            Err(_) => {
                let _ = write_reply(&mut output, &Reply::Crashed);
                let _ = output.flush();
                return; // a document that panicked is not touched again
            }
        };
        if write_reply(&mut output, &reply).is_err() {
            return;
        }
    }
}

fn main() {
    limit_myself();
    // a thread with a generous stack of its own: deep pages recurse, and the main thread's stack is small
    let t = std::thread::Builder::new().name("engine".into()).stack_size(256 * 1024 * 1024).spawn(run);
    if let Ok(t) = t {
        let _ = t.join();
    }
}
