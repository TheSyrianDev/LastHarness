//! Throwaway client shared by the four toolkit test apps. Plain HTTP to lhd on loopback, SSE parsed by hand.
use std::io::{BufRead, BufReader};
use std::sync::{mpsc::Sender, Arc, Mutex};

#[derive(Debug)]
pub enum Ev {
    Models(Vec<String>),
    Text(String),
    Done,
    Error(String),
}

#[derive(Clone)]
pub struct Cfg {
    pub base: String,
    pub token: String,
}

static IDS: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());
const TIERS: [&str; 8] = ["fast", "balanced", "max", "stock", "long", "low", "native", "high"];

/// Model name without its tier words ("Qwen-fast-stock" -> "Qwen").
pub fn display_name(id: &str) -> String {
    let mut s = id;
    while let Some((head, tail)) = s.rsplit_once('-') {
        if TIERS.contains(&tail) { s = head } else { break }
    }
    s.to_string()
}

/// The real model id for a display name; the "balanced" one if there are several.
pub fn id_for(name: &str) -> String {
    let ids = IDS.lock().unwrap();
    let all: Vec<&(String, String)> = ids.iter().filter(|(d, _)| d == name).collect();
    all.iter().find(|(_, i)| i.ends_with("-balanced")).or(all.first()).map(|(_, i)| i.clone()).unwrap_or_else(|| name.to_string())
}

pub type Wake = Arc<dyn Fn() + Send + Sync>;

/// `--base URL` (default http://127.0.0.1:7842) and `--token-file PATH`.
pub fn load_cfg() -> Cfg {
    let mut base = "http://127.0.0.1:7842".to_string();
    let mut tf = None;
    let mut a = std::env::args().skip(1);
    while let Some(x) = a.next() {
        match x.as_str() {
            "--base" => base = a.next().unwrap_or_default(),
            "--token-file" => tf = a.next(),
            _ => {}
        }
    }
    let token = tf.and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default().trim().to_string();
    Cfg { base, token }
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder().http_status_as_error(false).build().into()
}

pub fn spawn_models(cfg: Cfg, tx: Sender<Ev>, wake: Wake) {
    std::thread::spawn(move || {
        let r = agent().get(&format!("{}/v1/models", cfg.base)).header("Authorization", &format!("Bearer {}", cfg.token)).call();
        let ev = match r.and_then(|mut r| r.body_mut().read_json::<serde_json::Value>()) {
            Ok(v) => {
                let arr = v.get("models").or(v.get("data")).and_then(|x| x.as_array()).cloned().unwrap_or_default();
                let names: Vec<String> = arr.iter().filter_map(|m| m.as_str().map(String::from).or_else(|| m.get("id").and_then(|i| i.as_str()).map(String::from))).collect();
                let mut pairs: Vec<(String, String)> = names.into_iter().map(|i: String| (display_name(&i), i)).collect();
                let mut shown: Vec<String> = vec![];
                for (d, _) in &pairs { if !shown.contains(d) { shown.push(d.clone()) } }
                *IDS.lock().unwrap() = std::mem::take(&mut pairs);
                Ev::Models(shown)
            }
            Err(e) => Ev::Error(e.to_string()),
        };
        let _ = tx.send(ev);
        wake();
    });
}

/// Creates the session on first use, posts the turn and forwards each `text` event.
pub fn spawn_turn(cfg: Cfg, session: Arc<Mutex<Option<String>>>, model: String, text: String, tx: Sender<Ev>, wake: Wake) {
    std::thread::spawn(move || {
        let res = (|| -> Result<(), String> {
            let ag = agent();
            let auth = format!("Bearer {}", cfg.token);
            let mut sid = session.lock().unwrap().clone();
            if sid.is_none() {
                let mut r = ag.post(&format!("{}/v1/sessions", cfg.base)).header("Authorization", &auth)
                    .send_json(serde_json::json!({"model": id_for(&model), "mode": "chat", "max_tokens": 600})).map_err(|e| e.to_string())?;
                let v: serde_json::Value = r.body_mut().read_json().map_err(|e| e.to_string())?;
                let id = v["id"].as_str().ok_or("no session id")?.to_string();
                *session.lock().unwrap() = Some(id.clone());
                sid = Some(id);
            }
            let mut r = ag.post(&format!("{}/v1/sessions/{}/turns", cfg.base, sid.unwrap())).header("Authorization", &auth)
                .send_json(serde_json::json!({"text": text})).map_err(|e| e.to_string())?;
            if r.status().as_u16() >= 400 {
                return Err(format!("turn refused: {}", r.status()));
            }
            let rd = BufReader::new(r.body_mut().as_reader());
            let mut event = String::new();
            for line in rd.lines() {
                let line = line.map_err(|e| e.to_string())?;
                if let Some(e) = line.strip_prefix("event: ") {
                    event = e.to_string();
                } else if let Some(d) = line.strip_prefix("data: ") {
                    if event == "text" {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(d) {
                            if let Some(t) = v["text"].as_str() {
                                let _ = tx.send(Ev::Text(t.to_string()));
                                wake();
                            }
                        }
                    }
                }
            }
            Ok(())
        })();
        let _ = tx.send(match res { Ok(()) => Ev::Done, Err(e) => Ev::Error(e) });
        wake();
    });
}

pub mod chat {
    //! Several chats, a stop button, a model switch inside one chat, and chats that survive an app restart
    //! (while lhd keeps running). The turn call is written by hand over a plain socket so Stop can close the
    //! connection, which is how lhd is told to stop (it ends the turn when the stream goes away).
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::io::{BufRead, BufReader, Write};
    use std::net::{Shutdown, TcpStream};

    #[derive(Debug)]
    pub enum CEv {
        Models(Vec<String>),
        Text { chat: u64, text: String },
        Done { chat: u64 },
        Failed { chat: u64, err: String },
        /// Answer to `check_session`: does lhd still hold this chat's session?
        Alive { chat: u64, alive: bool },
    }

    #[derive(Default)]
    pub struct Sess {
        pub id: Option<String>,
        pub model_id: String,
    }
    pub type Shared = Arc<Mutex<Sess>>;

    #[derive(Serialize, Deserialize, Clone, Default)]
    pub struct Saved {
        pub id: u64,
        pub title: String,
        pub session: Option<String>,
        pub model: String,
        pub text: String,
    }
    pub fn load_saved(path: &str) -> Vec<Saved> {
        std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }
    pub fn save(path: &str, chats: &[Saved]) {
        if let Some(d) = std::path::Path::new(path).parent() { let _ = std::fs::create_dir_all(d); }
        let _ = std::fs::write(path, serde_json::to_string(chats).unwrap_or_default());
    }
    /// `--state PATH` (default ~/.local/state/gt-gui/chats.json).
    pub fn state_path() -> String {
        let mut a = std::env::args().skip(1);
        while let Some(x) = a.next() { if x == "--state" { return a.next().unwrap_or_default(); } }
        format!("{}/.local/state/gt-gui/chats.json", std::env::var("HOME").unwrap_or_default())
    }

    fn host(base: &str) -> String { base.trim_start_matches("http://").trim_end_matches('/').to_string() }
    fn auth(cfg: &Cfg) -> String { format!("Bearer {}", cfg.token) }

    pub struct Turn {
        sock: Arc<Mutex<Option<TcpStream>>>,
        cfg: Cfg,
        shared: Shared,
    }
    impl Turn {
        /// Close the connection (lhd stops the work) and free any waiting question.
        pub fn stop(&self) {
            if let Some(s) = self.sock.lock().unwrap().take() { let _ = s.shutdown(Shutdown::Both); }
            if let Some(id) = self.shared.lock().unwrap().id.clone() {
                let cfg = self.cfg.clone();
                std::thread::spawn(move || { let _ = agent().post(&format!("{}/v1/sessions/{id}/cancel", cfg.base)).header("Authorization", &auth(&cfg)).send_empty(); });
            }
        }
    }

    pub fn start_turn(cfg: Cfg, shared: Shared, chat: u64, model_display: String, text: String, tx: Sender<CEv>, wake: Wake) -> Turn {
        let sock: Arc<Mutex<Option<TcpStream>>> = Arc::new(Mutex::new(None));
        let turn = Turn { sock: sock.clone(), cfg: cfg.clone(), shared: shared.clone() };
        std::thread::spawn(move || {
            let res = (|| -> Result<(), String> {
                let ag = agent();
                let want = id_for(&model_display);
                let (sid, have) = { let s = shared.lock().unwrap(); (s.id.clone(), s.model_id.clone()) };
                let sid = match sid {
                    Some(id) => {
                        if have != want {
                            let r = ag.post(&format!("{}/v1/sessions/{id}/model", cfg.base)).header("Authorization", &auth(&cfg)).send_json(serde_json::json!({"model": want})).map_err(|e| e.to_string())?;
                            if r.status().as_u16() >= 400 { return Err(format!("model switch refused: {}", r.status())); }
                            shared.lock().unwrap().model_id = want.clone();
                        }
                        id
                    }
                    None => {
                        let mut r = ag.post(&format!("{}/v1/sessions", cfg.base)).header("Authorization", &auth(&cfg)).send_json(serde_json::json!({"model": want, "mode": "chat", "max_tokens": 600})).map_err(|e| e.to_string())?;
                        let v: serde_json::Value = r.body_mut().read_json().map_err(|e| e.to_string())?;
                        let id = v["id"].as_str().ok_or("no session id")?.to_string();
                        let mut s = shared.lock().unwrap();
                        s.id = Some(id.clone());
                        s.model_id = want;
                        id
                    }
                };
                let body = serde_json::json!({"text": text}).to_string();
                let mut s = TcpStream::connect(host(&cfg.base)).map_err(|e| e.to_string())?;
                *sock.lock().unwrap() = s.try_clone().ok();
                // HTTP/1.0: the answer is not chunked and ends when the server closes the connection
                write!(s, "POST /v1/sessions/{sid}/turns HTTP/1.0\r\nHost: {}\r\nAuthorization: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", host(&cfg.base), auth(&cfg), body.len()).map_err(|e| e.to_string())?;
                let mut rd = BufReader::new(s);
                let mut line = String::new();
                rd.read_line(&mut line).map_err(|e| e.to_string())?;
                let ok = line.contains(" 200");
                let status = line.trim().to_string();
                loop { line.clear(); if rd.read_line(&mut line).map_err(|e| e.to_string())? == 0 || line.trim().is_empty() { break; } }
                if !ok {
                    let mut rest = String::new();
                    let _ = std::io::Read::read_to_string(&mut rd, &mut rest);
                    return Err(format!("{status} {rest}"));
                }
                let mut event = String::new();
                loop {
                    line.clear();
                    match rd.read_line(&mut line) { Ok(0) | Err(_) => break, Ok(_) => {} }
                    let l = line.trim_end();
                    if let Some(e) = l.strip_prefix("event: ") { event = e.to_string(); }
                    else if let Some(d) = l.strip_prefix("data: ") {
                        let v: serde_json::Value = serde_json::from_str(d).unwrap_or_default();
                        let t = match event.as_str() {
                            "text" => v["text"].as_str().map(String::from),
                            "error" | "notice" => v["message"].as_str().map(|m| format!("\n[{event}] {m}\n")),
                            _ => None,
                        };
                        if let Some(text) = t { let _ = tx.send(CEv::Text { chat, text }); wake(); }
                    }
                }
                Ok(())
            })();
            *sock.lock().unwrap() = None;
            let _ = tx.send(match res { Ok(()) => CEv::Done { chat }, Err(err) => CEv::Failed { chat, err } });
            wake();
        });
        turn
    }

    pub fn delete_session(cfg: Cfg, id: String) {
        std::thread::spawn(move || { let _ = agent().delete(&format!("{}/v1/sessions/{id}", cfg.base)).header("Authorization", &auth(&cfg)).call(); });
    }

    pub fn check_session(cfg: Cfg, chat: u64, id: String, tx: Sender<CEv>, wake: Wake) {
        std::thread::spawn(move || {
            let alive = agent().get(&format!("{}/v1/sessions/{id}", cfg.base)).header("Authorization", &auth(&cfg)).call().map(|r| r.status().as_u16() == 200).unwrap_or(false);
            let _ = tx.send(CEv::Alive { chat, alive });
            wake();
        });
    }

    pub fn spawn_models(cfg: Cfg, tx: Sender<CEv>, wake: Wake) {
        let (t2, rx) = std::sync::mpsc::channel::<Ev>();
        super::spawn_models(cfg, t2, Arc::new(|| {}));
        std::thread::spawn(move || { if let Ok(Ev::Models(m)) = rx.recv() { let _ = tx.send(CEv::Models(m)); wake(); } });
    }
}
