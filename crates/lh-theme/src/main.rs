//! `lastai-theme`: a tiny loopback service that tells the Last AI website which
//! Omarchy theme is in use, so the site wears your desktop's colours and changes
//! the moment you switch themes.
//!
//!   GET /theme.json   the current theme: name, mode, colours
//!   GET /events       server-sent events, one per theme change
//!   OPTIONS *         preflight, including Chrome's private-network request
//!
//! It binds 127.0.0.1 only, reads two files, and never writes anything. Off
//! Omarchy it answers `/theme.json` with 404 and the website falls back to its
//! own picker. Run it once by hand to check: `lastai-theme --print`.

mod theme;

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const DEFAULT_PORT: u16 = 8787;

type Clients = Arc<Mutex<Vec<Sender<String>>>>;

fn main() {
    let mut port = std::env::var("LASTAI_THEME_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    let mut print_once = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--print" => print_once = true,
            "--port" => {
                if let Some(value) = args.next().and_then(|p| p.parse().ok()) {
                    port = value;
                }
            }
            "-h" | "--help" => {
                println!("lastai-theme [--port N] [--print]\n\nHands the Omarchy theme in use to the Last AI website.");
                return;
            }
            other => {
                eprintln!("lastai-theme: unknown option {other}");
                std::process::exit(2);
            }
        }
    }

    let Some(files) = locate() else {
        if print_once {
            println!("{{\"error\":\"no Omarchy theme on this machine\"}}");
            return;
        }
        eprintln!("lastai-theme: no Omarchy theme found; nothing to serve");
        std::process::exit(1);
    };

    let Some((theme, stamp)) = theme::snapshot(&files) else {
        if print_once {
            println!("{{\"error\":\"the current theme has no readable colors.toml\"}}");
            return;
        }
        eprintln!("lastai-theme: could not read {}", files.theme_dir.display());
        std::process::exit(1);
    };

    if print_once {
        println!("{}", theme.json());
        return;
    }

    let json = Arc::new(Mutex::new(theme.json()));
    let clients: Clients = Arc::new(Mutex::new(Vec::new()));

    watch(files, stamp, json.clone(), clients.clone());

    let bind = format!("127.0.0.1:{port}");
    let listener = match TcpListener::bind(&bind) {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("lastai-theme: cannot listen on {bind}: {error}");
            std::process::exit(1);
        }
    };
    eprintln!("lastai-theme: {} on http://{bind}/theme.json", theme.name);

    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let json = json.clone();
        let clients = clients.clone();
        std::thread::spawn(move || serve(stream, json, clients));
    }
}

/// The real folders, from the environment; `None` when neither has a theme.
fn locate() -> Option<theme::Files> {
    let state = match std::env::var_os("XDG_STATE_HOME") {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from(std::env::var_os("HOME")?).join(".local/state"),
    };
    let config = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from(std::env::var_os("HOME")?).join(".config"),
    };
    theme::current_dir(Some(state), Some(config))
}

/// Notice a theme change and push it to everyone listening. A small poll is
/// enough: `omarchy theme set` copies a folder, it does not fire an event the
/// service could subscribe to.
fn watch(files: theme::Files, mut stamp: theme::Stamp, json: Arc<Mutex<String>>, clients: Clients) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(700));
        let Some((theme, next)) = theme::snapshot(&files) else {
            continue;
        };
        if next == stamp {
            continue;
        }
        stamp = next;
        eprintln!("lastai-theme: theme now {}", theme.name);
        let body = theme.json();
        *json.lock().unwrap() = body.clone();

        let mut live = clients.lock().unwrap();
        live.retain(|client| client.send(body.clone()).is_ok());
    });
}

fn serve(mut stream: TcpStream, json: Arc<Mutex<String>>, clients: Clients) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(10)));

    let mut buffer = [0u8; 8192];
    let mut read = 0;
    while read < buffer.len() {
        match stream.read(&mut buffer[read..]) {
            Ok(0) => return,
            Ok(n) => {
                read += n;
                if buffer[..read].windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            Err(_) => return,
        }
    }

    let head = String::from_utf8_lossy(&buffer[..read]).into_owned();
    let mut lines = head.lines();
    let Some(request) = lines.next() else { return };
    let mut parts = request.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let path = parts.next().unwrap_or("/");
    let path = path.split('?').next().unwrap_or("/");
    let origin = lines
        .take(40)
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("origin")
                .then(|| value.trim().to_string())
        })
        .unwrap_or_else(|| "*".to_string());

    if method == "OPTIONS" {
        let _ = stream.write_all(
            format!(
                "HTTP/1.1 204 No Content\r\n{}Access-Control-Allow-Methods: GET, OPTIONS\r\n\
                 Access-Control-Allow-Headers: *\r\nAccess-Control-Max-Age: 600\r\n\
                 Content-Length: 0\r\nConnection: close\r\n\r\n",
                cors(&origin)
            )
            .as_bytes(),
        );
        return;
    }

    match (method, path) {
        ("GET", "/theme.json") => {
            let body = json.lock().unwrap().clone();
            let _ = stream.write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\n\
                     Cache-Control: no-store\r\n{}Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                    cors(&origin),
                    body.len(),
                    body
                )
                .as_bytes(),
            );
        }
        ("GET", "/events") => {
            let (tx, rx) = mpsc::channel();
            clients.lock().unwrap().push(tx);
            let _ = stream.write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\n\
                     {}X-Accel-Buffering: no\r\nConnection: keep-alive\r\n\r\n",
                    cors(&origin)
                )
                .as_bytes(),
            );
            // send what is true now, then every change as it lands
            let first = json.lock().unwrap().clone();
            if write_event(&mut stream, &first).is_err() {
                return;
            }
            loop {
                match rx.recv_timeout(Duration::from_secs(15)) {
                    Ok(body) => {
                        if write_event(&mut stream, &body).is_err() {
                            return;
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if stream.write_all(b": keep-alive\n\n").is_err() {
                            return;
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }
        }
        ("GET", "/") => {
            let body = "lastai-theme: GET /theme.json for the Omarchy theme in use\n";
            let _ = stream.write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\n{}Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                    cors(&origin),
                    body.len(),
                    body
                )
                .as_bytes(),
            );
        }
        _ => {
            let _ = stream.write_all(
                format!(
                    "HTTP/1.1 404 Not Found\r\n{}Content-Length: 0\r\nConnection: close\r\n\r\n",
                    cors(&origin)
                )
                .as_bytes(),
            );
        }
    }
}

fn write_event(stream: &mut TcpStream, body: &str) -> std::io::Result<()> {
    stream.write_all(format!("data: {body}\n\n").as_bytes())?;
    stream.flush()
}

/// CORS for the website: echo the origin that asked, and allow the private
/// network request Chrome makes when a public page reaches a local address.
fn cors(origin: &str) -> String {
    format!(
        "Access-Control-Allow-Origin: {origin}\r\nVary: Origin\r\n\
         Access-Control-Allow-Private-Network: true\r\n"
    )
}
