//! The browser front end's door: HTTPS with a certificate the core makes for itself, pairing with a PIN,
//! and sign-in by a key the browser keeps and cannot export.
//!
//! Pairing (spec 12.1): the browser makes a key pair and shows a PIN. It sends its public key and a proof
//! (an HMAC of the key with the PIN). The user types the PIN on this machine, which finds the request the
//! proof belongs to and pairs it. From then on the browser signs a fresh challenge with its key and gets a
//! session cookie; no password. The list of devices can be seen and any device revoked.
//!
//! A browser cannot read the certificate it is talking to, so the fingerprint is shown on this machine
//! and the user compares it with the one in the browser's certificate details.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::{Path, Request, State};
use axum::http::{header, HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use hmac::{Hmac, Mac};
use lh_store::Store;
use p256::ecdsa::signature::Verifier;
use p256::ecdsa::{Signature, VerifyingKey};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const PENDING_TTL: Duration = Duration::from_secs(300);
const MAX_PENDING: usize = 5;
const CHALLENGE_TTL: Duration = Duration::from_secs(60);
const SESSION_TTL: Duration = Duration::from_secs(12 * 3600);
const MAX_FAILED_PINS: u32 = 5;
const LOCKOUT: Duration = Duration::from_secs(60);

/// Which door a request came through. Added to the request so a handler can be stricter for devices.
#[derive(Clone, Debug, PartialEq)]
pub enum Via {
    Local,
    Device(String),
}

struct Pending {
    id: String,
    name: String,
    public_key: Value,
    proof: String,
    at: Instant,
}

#[derive(Default)]
struct Pairing {
    pending: Vec<Pending>,
    failed: u32,
    locked_until: Option<Instant>,
}

pub struct Web {
    store: Arc<Store>,
    pairing: Mutex<Pairing>,
    challenges: Mutex<HashMap<String, (String, Instant)>>,
    sessions: Mutex<HashMap<String, (String, Instant)>>,
    pub fingerprint: String,
    next: AtomicU64,
}

fn rand_hex(n: usize) -> String {
    let mut b = vec![0u8; n];
    getrandom::fill(&mut b).expect("the OS random source is unavailable");
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn same(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Spelled the way browsers show it: upper case pairs with colons.
pub fn format_fingerprint(der: &[u8]) -> String {
    Sha256::digest(der).iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(":")
}

/// The one string both sides sign and hash: the key's two coordinates.
fn canonical_key(jwk: &Value) -> Option<String> {
    let (x, y) = (jwk["x"].as_str()?, jwk["y"].as_str()?);
    (jwk["kty"] == "EC" && jwk["crv"] == "P-256").then(|| format!("{x}.{y}"))
}

fn verifying_key(jwk: &Value) -> Option<VerifyingKey> {
    use base64::Engine as _;
    let dec = |s: &str| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(s).ok();
    let (x, y) = (dec(jwk["x"].as_str()?)?, dec(jwk["y"].as_str()?)?);
    if x.len() != 32 || y.len() != 32 {
        return None;
    }
    let mut sec1 = vec![0x04];
    sec1.extend(x);
    sec1.extend(y);
    VerifyingKey::from_sec1_bytes(&sec1).ok()
}

pub fn proof_for(pin: &str, jwk: &Value) -> Option<String> {
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(pin.as_bytes()).ok()?;
    mac.update(canonical_key(jwk)?.as_bytes());
    Some(mac.finalize().into_bytes().iter().map(|b| format!("{b:02x}")).collect())
}

impl Web {
    pub fn new(store: Arc<Store>, fingerprint: String) -> Arc<Self> {
        Arc::new(Self { store, pairing: Mutex::default(), challenges: Mutex::default(), sessions: Mutex::default(), fingerprint, next: AtomicU64::new(1) })
    }

    fn device_key(&self, id: &str) -> Option<Value> {
        use lh_store::rusqlite::OptionalExtension;
        let (k, revoked): (String, i64) = self.store.with(|c| c.query_row("select public_key, revoked from devices where id = ?1", [id], |r| Ok((r.get(0)?, r.get(1)?))).optional()).ok()??;
        (revoked == 0).then(|| serde_json::from_str(&k).ok()).flatten()
    }

    /// Who a session cookie belongs to, if it is good.
    pub fn session_device(&self, token: &str) -> Option<String> {
        let mut s = self.sessions.lock().unwrap();
        let (dev, exp) = s.get(token)?.clone();
        if exp < Instant::now() || self.device_key(&dev).is_none() {
            s.remove(token);
            return None;
        }
        Some(dev)
    }

    pub fn devices(&self) -> Vec<Value> {
        self.store
            .with(|c| {
                let mut q = c.prepare("select id, name, created, last_seen, revoked from devices order by created")?;
                let rows = q.query_map([], |r| Ok(json!({"id": r.get::<_, String>(0)?, "name": r.get::<_, String>(1)?, "created": r.get::<_, i64>(2)?, "last_seen": r.get::<_, i64>(3)?, "revoked": r.get::<_, i64>(4)? != 0})))?;
                rows.collect()
            })
            .unwrap_or_default()
    }

    pub fn revoke(&self, id: &str) -> bool {
        let n = self.store.with(|c| c.execute("update devices set revoked = 1 where id = ?1 and revoked = 0", [id])).unwrap_or(0);
        self.sessions.lock().unwrap().retain(|_, (d, _)| d != id);
        if n > 0 {
            let _ = self.store.log("device", id, "user", &json!({"op": "revoked"}));
        }
        n > 0
    }

    fn start(&self, name: &str, jwk: Value, proof: &str) -> Result<String, (StatusCode, &'static str)> {
        if canonical_key(&jwk).is_none() || verifying_key(&jwk).is_none() {
            return Err((StatusCode::BAD_REQUEST, "the key must be a P-256 public key"));
        }
        if proof.len() != 64 || !proof.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err((StatusCode::BAD_REQUEST, "the proof is malformed"));
        }
        let mut p = self.pairing.lock().unwrap();
        p.pending.retain(|x| x.at.elapsed() < PENDING_TTL);
        if p.pending.len() >= MAX_PENDING {
            return Err((StatusCode::TOO_MANY_REQUESTS, "too many pairing requests are waiting; try again in a few minutes"));
        }
        let id = format!("p{}", self.next.fetch_add(1, Ordering::SeqCst));
        let name: String = name.chars().filter(|c| !c.is_control()).take(60).collect();
        p.pending.push(Pending { id: id.clone(), name: if name.is_empty() { "browser".into() } else { name }, public_key: jwk, proof: proof.to_lowercase(), at: Instant::now() });
        Ok(id)
    }

    pub fn pending(&self) -> Vec<Value> {
        let mut p = self.pairing.lock().unwrap();
        p.pending.retain(|x| x.at.elapsed() < PENDING_TTL);
        p.pending.iter().map(|x| json!({"id": x.id, "name": x.name, "key_fingerprint": canonical_key(&x.public_key).map(|k| format_fingerprint(k.as_bytes())), "waiting_secs": x.at.elapsed().as_secs()})).collect()
    }

    /// The PIN typed on this machine. Returns the device that was paired.
    pub fn accept(&self, pin: &str) -> Result<Value, (StatusCode, String)> {
        let mut p = self.pairing.lock().unwrap();
        if p.locked_until.is_some_and(|t| t > Instant::now()) {
            return Err((StatusCode::TOO_MANY_REQUESTS, "too many wrong PINs; wait a minute and ask the device to start again".into()));
        }
        p.pending.retain(|x| x.at.elapsed() < PENDING_TTL);
        let hits: Vec<usize> = p.pending.iter().enumerate().filter(|(_, x)| proof_for(pin, &x.public_key).is_some_and(|f| same(f.as_bytes(), x.proof.as_bytes()))).map(|(i, _)| i).collect();
        match hits.len() {
            0 => {
                p.failed += 1;
                if p.failed >= MAX_FAILED_PINS {
                    p.pending.clear();
                    p.failed = 0;
                    p.locked_until = Some(Instant::now() + LOCKOUT);
                    return Err((StatusCode::TOO_MANY_REQUESTS, "too many wrong PINs; every waiting request was dropped. Ask the device to start again".into()));
                }
                Err((StatusCode::NOT_FOUND, "no waiting device matches that PIN".into()))
            }
            1 => {
                p.failed = 0;
                let x = p.pending.remove(hits[0]);
                let id = format!("d_{}", rand_hex(6));
                let key = x.public_key.to_string();
                self.store.with(|c| c.execute("insert into devices(id, name, public_key, created) values (?1, ?2, ?3, ?4)", (&id, &x.name, &key, now()))).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
                let _ = self.store.log("device", &id, "user", &json!({"op": "paired", "name": x.name}));
                // tell the waiting browser which device it now is
                self.challenges.lock().unwrap().insert(format!("paired:{}", x.id), (id.clone(), Instant::now()));
                Ok(json!({"id": id, "name": x.name}))
            }
            _ => {
                // two requests with the same proof PIN: someone is racing the real device
                p.pending.clear();
                Err((StatusCode::CONFLICT, "two waiting requests matched that PIN, so none was paired. Ask the device to start again".into()))
            }
        }
    }

    fn status(&self, pending_id: &str) -> Value {
        if let Some((dev, at)) = self.challenges.lock().unwrap().get(&format!("paired:{pending_id}")).cloned() {
            if at.elapsed() < PENDING_TTL {
                return json!({"state": "paired", "device_id": dev});
            }
        }
        if self.pairing.lock().unwrap().pending.iter().any(|x| x.id == pending_id && x.at.elapsed() < PENDING_TTL) {
            json!({"state": "waiting"})
        } else {
            json!({"state": "expired"})
        }
    }

    fn challenge(&self, device_id: &str) -> Option<String> {
        self.device_key(device_id)?;
        let mut c = self.challenges.lock().unwrap();
        c.retain(|_, (_, at)| at.elapsed() < CHALLENGE_TTL.max(PENDING_TTL));
        let nonce = rand_hex(32);
        c.insert(format!("nonce:{nonce}"), (device_id.to_string(), Instant::now()));
        Some(nonce)
    }

    /// Check the signature on a challenge; on success make a session.
    fn verify(&self, device_id: &str, nonce: &str, signature_hex: &str) -> Option<String> {
        let (dev, at) = self.challenges.lock().unwrap().remove(&format!("nonce:{nonce}"))?; // one use
        if dev != device_id || at.elapsed() > CHALLENGE_TTL {
            return None;
        }
        let key = verifying_key(&self.device_key(device_id)?)?;
        let sig_bytes: Vec<u8> = (0..signature_hex.len() / 2).filter_map(|i| u8::from_str_radix(signature_hex.get(i * 2..i * 2 + 2)?, 16).ok()).collect();
        let sig = Signature::from_slice(&sig_bytes).ok()?;
        key.verify(nonce.as_bytes(), &sig).ok()?;
        let token = rand_hex(32);
        self.sessions.lock().unwrap().insert(token.clone(), (device_id.to_string(), Instant::now() + SESSION_TTL));
        let _ = self.store.with(|c| c.execute("update devices set last_seen = ?1 where id = ?2", (now(), device_id)));
        Some(token)
    }
}

// ---- the pages ----

const INDEX: &str = include_str!("../web/index.html");
const APP: &str = include_str!("../web/app.js");

fn page(body: &'static str, content_type: &'static str) -> Response {
    let mut r = Response::new(Body::from(body));
    let h = r.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    r
}

/// Headers on everything the web door sends.
pub async fn headers(req: Request, next: Next) -> Response {
    let mut r = next.run(req).await;
    let h = r.headers_mut();
    h.insert("content-security-policy", HeaderValue::from_static("default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'"));
    h.insert("x-content-type-options", HeaderValue::from_static("nosniff"));
    h.insert("referrer-policy", HeaderValue::from_static("same-origin"));
    h.insert("cache-control", HeaderValue::from_static("no-store"));
    h.insert("strict-transport-security", HeaderValue::from_static("max-age=31536000"));
    r
}

#[derive(Deserialize)]
struct StartBody {
    name: String,
    public_key: Value,
    proof: String,
}

#[derive(Deserialize)]
struct ChallengeBody {
    device_id: String,
}

#[derive(Deserialize)]
struct VerifyBody {
    device_id: String,
    nonce: String,
    signature: String,
}

/// The public part: pages, pairing and sign-in. Nothing here reveals anything private.
pub fn public_routes(web: Arc<Web>) -> Router {
    Router::new()
        .route("/", get(|| async { page(INDEX, "text/html; charset=utf-8") }))
        .route("/app.js", get(|| async { page(APP, "text/javascript; charset=utf-8") }))
        .route(
            "/pair/start",
            post(|State(w): State<Arc<Web>>, Json(b): Json<StartBody>| async move {
                match w.start(&b.name, b.public_key, &b.proof) {
                    Ok(id) => (StatusCode::CREATED, Json(json!({"id": id, "expires_in": PENDING_TTL.as_secs()}))).into_response(),
                    Err((c, m)) => (c, Json(json!({"error": m}))).into_response(),
                }
            }),
        )
        .route("/pair/status/{id}", get(|State(w): State<Arc<Web>>, Path(id): Path<String>| async move { Json(w.status(&id)) }))
        .route(
            "/auth/challenge",
            post(|State(w): State<Arc<Web>>, Json(b): Json<ChallengeBody>| async move {
                match w.challenge(&b.device_id) {
                    Some(n) => Json(json!({"nonce": n})).into_response(),
                    None => (StatusCode::UNAUTHORIZED, Json(json!({"error": "this device is not paired (or was revoked)"}))).into_response(),
                }
            }),
        )
        .route(
            "/auth/verify",
            post(|State(w): State<Arc<Web>>, Json(b): Json<VerifyBody>| async move {
                match w.verify(&b.device_id, &b.nonce, &b.signature) {
                    Some(t) => {
                        let mut r = Json(json!({"ok": true})).into_response();
                        r.headers_mut().insert(header::SET_COOKIE, HeaderValue::from_str(&format!("lh_session={t}; Secure; HttpOnly; SameSite=Strict; Path=/; Max-Age={}", SESSION_TTL.as_secs())).unwrap());
                        r
                    }
                    None => (StatusCode::UNAUTHORIZED, Json(json!({"error": "the signature did not match"}))).into_response(),
                }
            }),
        )
        .route(
            "/auth/logout",
            post(|| async {
                let mut r = StatusCode::NO_CONTENT.into_response();
                r.headers_mut().insert(header::SET_COOKIE, HeaderValue::from_static("lh_session=; Secure; HttpOnly; SameSite=Strict; Path=/; Max-Age=0"));
                r
            }),
        )
        .with_state(web)
}

/// What a paired browser may reach. Everything that can run programs, move files out of the machine or
/// change who is trusted stays on the machine's own terminal.
pub fn device_may(method: &Method, path: &str) -> bool {
    const DENIED: &[&str] = &["/v1/mcp", "/v1/export", "/v1/import", "/v1/trust", "/v1/team/plugins", "/v1/memory/export", "/v1/memory/import", "/v1/devices/pending", "/v1/devices/accept", "/v1/devices/fingerprint"];
    if DENIED.iter().any(|d| path == *d || path.starts_with(&format!("{d}/"))) {
        return false;
    }
    // reading anything else is fine; changes only in the areas the browser UI has screens for
    if method == Method::GET || method == Method::HEAD {
        return path.starts_with("/v1/");
    }
    ["/v1/sessions", "/v1/memory", "/v1/learn", "/v1/team", "/v1/settings", "/v1/devices", "/v1/activity"].iter().any(|d| path == *d || path.starts_with(&format!("{d}/")))
}

fn cookie(req: &Request, name: &str) -> Option<String> {
    req.headers().get_all(header::COOKIE).iter().filter_map(|v| v.to_str().ok()).flat_map(|v| v.split(';')).filter_map(|kv| kv.trim().split_once('=')).find(|(k, _)| *k == name).map(|(_, v)| v.to_string())
}

/// Guard for the API over HTTPS: a valid device session, a same-site request, an allowed path.
pub async fn require_device(State(w): State<Arc<Web>>, mut req: Request, next: Next) -> Response {
    let Some(dev) = cookie(&req, "lh_session").and_then(|t| w.session_device(&t)) else {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "not signed in"}))).into_response();
    };
    // over HTTP/2 the name is in the request's authority, not in a Host header
    let host = req.headers().get(header::HOST).and_then(|v| v.to_str().ok()).map(str::to_string).or_else(|| req.uri().authority().map(|a| a.to_string())).unwrap_or_default();
    if let Some(o) = req.headers().get(header::ORIGIN).and_then(|v| v.to_str().ok()) {
        if o != format!("https://{host}") {
            return (StatusCode::FORBIDDEN, "cross-site requests are not accepted").into_response();
        }
    } else if !matches!(*req.method(), Method::GET | Method::HEAD) {
        return (StatusCode::FORBIDDEN, "this request must say where it comes from").into_response();
    }
    if !device_may(req.method(), req.uri().path()) {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "this is not available from a browser; use the terminal on the machine"}))).into_response();
    }
    req.extensions_mut().insert(Via::Device(dev));
    next.run(req).await
}

// ---- for the machine's own terminal (behind the local token) ----

pub fn local_routes(web: Arc<Web>) -> Router {
    Router::new()
        .route("/v1/devices", get(|State(w): State<Arc<Web>>| async move { Json(json!({"devices": w.devices()})) }))
        .route("/v1/devices/pending", get(|State(w): State<Arc<Web>>| async move { Json(json!({"pending": w.pending(), "fingerprint": w.fingerprint})) }))
        .route("/v1/devices/fingerprint", get(|State(w): State<Arc<Web>>| async move { Json(json!({"fingerprint": w.fingerprint})) }))
        .route(
            "/v1/devices/accept",
            post(|State(w): State<Arc<Web>>, Json(b): Json<Value>| async move {
                match w.accept(b["pin"].as_str().unwrap_or("").trim()) {
                    Ok(d) => Json(d).into_response(),
                    Err((c, m)) => (c, Json(json!({"error": m}))).into_response(),
                }
            }),
        )
        .route(
            "/v1/devices/{id}",
            axum::routing::delete(|State(w): State<Arc<Web>>, Path(id): Path<String>| async move {
                if w.revoke(&id) { StatusCode::NO_CONTENT } else { StatusCode::NOT_FOUND }
            }),
        )
        .with_state(web)
}

/// Devices and the fingerprint, for the browser's settings screen (list and revoke only).
pub fn device_routes(web: Arc<Web>) -> Router {
    Router::new()
        .route("/v1/devices", get(|State(w): State<Arc<Web>>| async move { Json(json!({"devices": w.devices()})) }))
        .route(
            "/v1/devices/{id}",
            axum::routing::delete(|State(w): State<Arc<Web>>, Path(id): Path<String>| async move {
                if w.revoke(&id) { StatusCode::NO_CONTENT } else { StatusCode::NOT_FOUND }
            }),
        )
        .with_state(web)
}

// ---- the certificate and the HTTPS server ----

pub struct Tls {
    pub acceptor: tokio_rustls::TlsAcceptor,
    pub fingerprint: String,
}

fn tls_err(e: impl std::fmt::Display) -> String {
    format!("cannot set up the certificate: {e}")
}

/// Load the core's own certificate from `dir`, or make one the first time (and again when it is old).
/// The key stays in that folder, readable only by this user.
pub fn load_or_create_tls(dir: &std::path::Path, extra_names: &[String]) -> Result<Tls, String> {
    std::fs::create_dir_all(dir).map_err(tls_err)?;
    let (cert_path, key_path) = (dir.join("cert.der"), dir.join("key.der"));
    let fresh = std::fs::metadata(&cert_path).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_some_and(|age| age < Duration::from_secs(300 * 86_400));
    let (cert_der, key_der) = match (fresh, std::fs::read(&cert_path), std::fs::read(&key_path)) {
        (true, Ok(c), Ok(k)) => (c, k),
        _ => {
            let mut names: Vec<String> = vec!["localhost".into(), "127.0.0.1".into(), "::1".into()];
            if let Ok(h) = std::fs::read_to_string("/etc/hostname") {
                let h = h.trim();
                if !h.is_empty() {
                    names.push(h.to_string());
                }
            }
            names.extend(extra_names.iter().cloned());
            names.dedup();
            let mut params = rcgen::CertificateParams::new(names).map_err(tls_err)?;
            params.distinguished_name.push(rcgen::DnType::CommonName, "LastHarness");
            let today: Vec<i64> = lh_tools::today().split('-').filter_map(|p| p.parse().ok()).collect();
            let (y, m, d) = (today[0] as i32, today[1] as u8, today[2] as u8);
            params.not_before = rcgen::date_time_ymd(y, m, d);
            params.not_after = rcgen::date_time_ymd(y + 2, m, d.min(28));
            let key = rcgen::KeyPair::generate().map_err(tls_err)?;
            let cert = params.self_signed(&key).map_err(tls_err)?;
            let (c, k) = (cert.der().to_vec(), key.serialize_der());
            std::fs::write(&cert_path, &c).map_err(tls_err)?;
            let mut opts = std::fs::OpenOptions::new();
            opts.write(true).create(true).truncate(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                opts.mode(0o600);
            }
            std::io::Write::write_all(&mut opts.open(&key_path).map_err(tls_err)?, &k).map_err(tls_err)?;
            (c, k)
        }
    };
    let fingerprint = format_fingerprint(&cert_der);
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut cfg = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(tls_err)?
        .with_no_client_auth()
        .with_single_cert(vec![rustls::pki_types::CertificateDer::from(cert_der)], rustls::pki_types::PrivateKeyDer::try_from(key_der).map_err(tls_err)?)
        .map_err(tls_err)?;
    cfg.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(Tls { acceptor: tokio_rustls::TlsAcceptor::from(Arc::new(cfg)), fingerprint })
}

/// Serve `app` over HTTPS on an already bound listener until the process ends.
pub async fn serve_tls(listener: tokio::net::TcpListener, app: Router, acceptor: tokio_rustls::TlsAcceptor) {
    use tower::Service;
    let mut make = app.into_make_service();
    loop {
        let Ok((tcp, _)) = listener.accept().await else { continue };
        let svc = match make.call(&tcp).await {
            Ok(s) => s,
            Err(never) => match never {},
        };
        let acceptor = acceptor.clone();
        tokio::spawn(async move {
            // a handshake that does not finish quickly is dropped, so a stranger cannot hold the door open
            let Ok(Ok(tls)) = tokio::time::timeout(Duration::from_secs(10), acceptor.accept(tcp)).await else { return };
            let io = hyper_util::rt::TokioIo::new(tls);
            let hyper_svc = hyper::service::service_fn(move |req: hyper::Request<hyper::body::Incoming>| svc.clone().call(req));
            let _ = hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new()).serve_connection_with_upgrades(io, hyper_svc).await;
        });
    }
}
