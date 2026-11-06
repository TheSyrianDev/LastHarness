//! The browser's door over real HTTPS: pairing with a PIN, signing in with a key, and what a paired
//! browser may and may not reach.
use std::sync::Arc;
use std::time::Duration;

use base64::Engine as _;
use lh_core::web::{self, Web};
use lh_core::{auth, router_with_web, web_router, AppState, Settings};
use lh_provider::{ChatRequest, Event, Response};
use lh_store::Store;
use lh_tools::Model;
use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};
use serde_json::{json, Value};

struct Models;
impl Model for Models {
    async fn complete(&self, _: &ChatRequest, _: &mut (dyn FnMut(Event) + Send)) -> Result<Response, lh_provider::Error> {
        Ok(Response::default())
    }
    async fn models(&self) -> Result<Vec<String>, lh_provider::Error> {
        Ok(vec!["m1".into()])
    }
}

struct Rig {
    https: String,
    local: String,
    token: String,
    web: Arc<Web>,
    http: reqwest::Client,
    fingerprint: String,
    _dir: tempfile::TempDir,
}

async fn rig() -> Rig {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(Store::open(&dir.path().join("lh.db")).unwrap());
    let settings = Settings {
        config: Some(dir.path().join("config.toml")),
        ask_all: false,
        mcp: None,
        team: None,
        provider_name: None,
        learn: None,
        maintain_when_idle: false,
        memory: None,
        image_model: None,
        web: Default::default(),
        idle_after: Duration::ZERO,
        compact_at: Some(0),
        store: store.clone(),
        spend: lh_spend::SpendMeter::new(store.clone(), lh_spend::Config::default(), "t", true),
        bypass: false,
        sudo_enabled: false,
        allow_rules: vec![],
    };
    let state = AppState::new(Models, settings);
    let tls = web::load_or_create_tls(&dir.path().join("web"), &[]).unwrap();
    let w = Web::new(store, tls.fingerprint.clone());
    w.open_pairing(Duration::from_secs(300));
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let https = format!("https://127.0.0.1:{}", l.local_addr().unwrap().port());
    tokio::spawn(web::serve_tls(l, web_router(state.clone(), w.clone()), tls.acceptor));
    let token = auth::new_token();
    let ll = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let lport = ll.local_addr().unwrap().port();
    let app = router_with_web(state, token.clone(), lport, Some(w.clone()));
    tokio::spawn(async move { axum::serve(ll, app).await.unwrap() });
    Rig { https, local: format!("http://127.0.0.1:{lport}"), token, web: w, http: reqwest::Client::builder().danger_accept_invalid_certs(true).build().unwrap(), fingerprint: tls.fingerprint, _dir: dir }
}

struct Browser {
    key: SigningKey,
    jwk: Value,
}
impl Browser {
    fn new() -> Self {
        let key = SigningKey::random(&mut p256::elliptic_curve::rand_core::OsRng);
        let pt = key.verifying_key().to_encoded_point(false);
        let b64 = |b: &[u8]| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b);
        let jwk = json!({"kty": "EC", "crv": "P-256", "x": b64(pt.x().unwrap()), "y": b64(pt.y().unwrap())});
        Self { key, jwk }
    }
    fn sign(&self, msg: &str) -> String {
        let s: Signature = self.key.sign(msg.as_bytes());
        s.to_bytes().iter().map(|b| format!("{b:02x}")).collect()
    }
}

async fn pair(r: &Rig, b: &Browser, pin: &str) -> String {
    let start: Value = r.http.post(format!("{}/pair/start", r.https)).json(&json!({"name": "Firefox on Linux", "public_key": b.jwk, "proof": web::proof_for(pin, &b.jwk).unwrap()})).send().await.unwrap().json().await.unwrap();
    assert!(start["id"].is_string(), "{start}");
    let accepted: Value = r.http.post(format!("{}/v1/devices/accept", r.local)).bearer_auth(&r.token).json(&json!({"pin": pin})).send().await.unwrap().json().await.unwrap();
    accepted["id"].as_str().unwrap_or_else(|| panic!("{accepted}")).to_string()
}

async fn sign_in(r: &Rig, b: &Browser, dev: &str) -> Option<String> {
    let c: Value = r.http.post(format!("{}/auth/challenge", r.https)).json(&json!({"device_id": dev})).send().await.unwrap().json().await.unwrap();
    let nonce = c["nonce"].as_str()?.to_string();
    let resp = r.http.post(format!("{}/auth/verify", r.https)).json(&json!({"device_id": dev, "nonce": nonce, "signature": b.sign(&nonce)})).send().await.unwrap();
    resp.headers().get("set-cookie").map(|v| v.to_str().unwrap().split(';').next().unwrap().to_string())
}

#[tokio::test]
async fn the_pages_come_over_https_with_strict_headers_and_the_api_needs_a_signed_in_device() {
    let r = rig().await;
    assert!(r.fingerprint.matches(':').count() == 31, "a SHA-256 fingerprint: {}", r.fingerprint);
    let page = r.http.get(format!("{}/", r.https)).send().await.unwrap();
    assert_eq!(page.status(), 200);
    let csp = page.headers()["content-security-policy"].to_str().unwrap().to_string();
    assert!(csp.contains("default-src 'self'") && csp.contains("frame-ancestors 'none'"), "{csp}");
    assert_eq!(page.headers()["cache-control"], "no-store");
    assert_eq!(r.http.get(format!("{}/app.js", r.https)).send().await.unwrap().status(), 200);
    // the page carries its build, and asks for the script and style by it, so a new build is never read from a stale copy
    let id = web::build_id();
    assert!(id.len() == 12 && id.chars().all(|c| c.is_ascii_hexdigit()), "{id}");
    let html = page.text().await.unwrap();
    assert!(html.contains(&format!("<meta name=\"lh-build\" content=\"{id}\">")), "the build stamp is in the page");
    assert!(html.contains(&format!("src=\"/app.js?v={id}\"")) && html.contains(&format!("href=\"/style.css?v={id}\"")), "script and style are asked for by build");
    assert_eq!(r.http.get(format!("{}/app.js?v={id}", r.https)).send().await.unwrap().status(), 200);
    assert!(html.contains(&format!("src=\"/motion.js?v={id}\"")), "motion.js is asked for by build");
    assert_eq!(r.http.get(format!("{}/motion.js", r.https)).send().await.unwrap().status(), 200);
    assert_eq!(r.http.get(format!("{}/style.css?v={id}", r.https)).send().await.unwrap().status(), 200);
    // on a new build the script drops stored keys that were not chosen, and never the device key in IndexedDB
    let js = r.http.get(format!("{}/app.js", r.https)).send().await.unwrap().text().await.unwrap();
    let fresh = &js[js.find("function freshBuild()").expect("the build check is there")..];
    let fresh = &fresh[..fresh.find("\nfreshBuild();").unwrap()];
    assert!(fresh.contains("localStorage.removeItem") && !fresh.contains("indexedDB") && !fresh.contains("cookie"), "{fresh}");
    // the themes the desktop app ships come with the page, so both look the same
    let themes: serde_json::Value = r.http.get(format!("{}/themes.json", r.https)).send().await.unwrap().json().await.unwrap();
    assert!(themes.as_array().map(|a| !a.is_empty()).unwrap_or(false) || themes.as_object().map(|o| !o.is_empty()).unwrap_or(false), "{themes}");
    for p in ["/v1/models", "/v1/sessions/s1", "/v1/memory/pages", "/v1/devices"] {
        assert_eq!(r.http.get(format!("{}{p}", r.https)).send().await.unwrap().status(), 401, "{p} must need a signed-in device");
    }
    // the file token is for the machine, not for the network
    assert_eq!(r.http.get(format!("{}/v1/models", r.https)).bearer_auth(&r.token).send().await.unwrap().status(), 401);
    // plain HTTP to the TLS port gets nothing
    let plain = reqwest::Client::new().get(r.https.replace("https", "http")).timeout(Duration::from_secs(3)).send().await;
    assert!(plain.is_err() || !plain.unwrap().status().is_success());
}

#[tokio::test]
async fn pairing_with_a_pin_then_signing_in_with_the_key() {
    let r = rig().await;
    let b = Browser::new();
    let start: Value = r.http.post(format!("{}/pair/start", r.https)).json(&json!({"name": "Firefox on Linux", "public_key": b.jwk, "proof": web::proof_for("123456", &b.jwk).unwrap()})).send().await.unwrap().json().await.unwrap();
    let pid = start["id"].as_str().unwrap();
    let waiting: Value = r.http.get(format!("{}/pair/status/{pid}", r.https)).send().await.unwrap().json().await.unwrap();
    assert_eq!(waiting["state"], "waiting");
    // the machine sees who is asking and the fingerprint of the certificate to compare
    let pend: Value = r.http.get(format!("{}/v1/devices/pending", r.local)).bearer_auth(&r.token).send().await.unwrap().json().await.unwrap();
    assert_eq!(pend["pending"][0]["name"], "Firefox on Linux");
    assert_eq!(pend["fingerprint"], r.fingerprint.as_str());
    // a wrong PIN pairs nothing
    let wrong = r.http.post(format!("{}/v1/devices/accept", r.local)).bearer_auth(&r.token).json(&json!({"pin": "000000"})).send().await.unwrap();
    assert_eq!(wrong.status(), 404);
    // the browser cannot accept its own pairing
    assert_eq!(r.http.post(format!("{}/v1/devices/accept", r.https)).json(&json!({"pin": "123456"})).send().await.unwrap().status(), 401);
    let ok: Value = r.http.post(format!("{}/v1/devices/accept", r.local)).bearer_auth(&r.token).json(&json!({"pin": "123456"})).send().await.unwrap().json().await.unwrap();
    let dev = ok["id"].as_str().unwrap().to_string();
    let done: Value = r.http.get(format!("{}/pair/status/{pid}", r.https)).send().await.unwrap().json().await.unwrap();
    assert_eq!((done["state"].as_str(), done["device_id"].as_str()), (Some("paired"), Some(dev.as_str())));

    // sign in: a fresh challenge signed with the key
    let cookie = sign_in(&r, &b, &dev).await.expect("signed in");
    let resp = r.http.get(format!("{}/v1/models", r.https)).header("cookie", &cookie).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<Value>().await.unwrap()["models"][0], "m1");
    // a nonce works once
    let c: Value = r.http.post(format!("{}/auth/challenge", r.https)).json(&json!({"device_id": dev})).send().await.unwrap().json().await.unwrap();
    let nonce = c["nonce"].as_str().unwrap();
    let body = json!({"device_id": dev, "nonce": nonce, "signature": b.sign(nonce)});
    assert_eq!(r.http.post(format!("{}/auth/verify", r.https)).json(&body).send().await.unwrap().status(), 200);
    assert_eq!(r.http.post(format!("{}/auth/verify", r.https)).json(&body).send().await.unwrap().status(), 401, "replaying a signed challenge fails");
    // someone else's key cannot sign in as this device
    let other = Browser::new();
    assert!(sign_in(&r, &other, &dev).await.is_none());
    assert!(sign_in(&r, &b, "d_unknown").await.is_none());
    // the cookie is locked down
    let c: Value = r.http.post(format!("{}/auth/challenge", r.https)).json(&json!({"device_id": dev})).send().await.unwrap().json().await.unwrap();
    let n = c["nonce"].as_str().unwrap();
    let v = r.http.post(format!("{}/auth/verify", r.https)).json(&json!({"device_id": dev, "nonce": n, "signature": b.sign(n)})).send().await.unwrap();
    let sc = v.headers()["set-cookie"].to_str().unwrap().to_string();
    assert!(sc.contains("Secure") && sc.contains("HttpOnly") && sc.contains("SameSite=Strict"), "{sc}");
}

#[tokio::test]
async fn a_paired_browser_reaches_the_screens_it_needs_and_nothing_that_runs_programs() {
    let r = rig().await;
    let b = Browser::new();
    let dev = pair(&r, &b, "654321").await;
    let cookie = sign_in(&r, &b, &dev).await.unwrap();
    let origin = r.https.clone();
    let get = |p: &str| r.http.get(format!("{}{p}", r.https)).header("cookie", &cookie).send();
    assert_eq!(get("/v1/status").await.unwrap().status(), 200);
    assert_eq!(get("/v1/devices").await.unwrap().status(), 200);
    // provider registry changes (and keys) stay on the machine
    for (m, p) in [("POST", "/v1/providers"), ("DELETE", "/v1/providers/x"), ("POST", "/v1/providers/x/default"), ("POST", "/v1/providers/proposals/x/approve"), ("POST", "/v1/providers/proposals/x/reject")] {
        let resp = r.http.request(m.parse().unwrap(), format!("{}{p}", r.https)).header("cookie", &cookie).header("origin", &origin).json(&json!({"url": "http://evil/v1", "key": "k"})).send().await.unwrap();
        assert_eq!(resp.status(), 403, "{m} {p} stays on the machine");
    }
    for p in ["/v1/mcp/servers", "/v1/trust?folder=/", "/v1/devices/pending", "/v1/team/plugins/preview"] {
        assert_eq!(get(p).await.unwrap().status(), 403, "{p} stays on the machine");
    }
    // a change must say it comes from this site
    let post = |p: &str, o: Option<&str>| {
        let mut rb = r.http.post(format!("{}{p}", r.https)).header("cookie", &cookie).json(&json!({}));
        if let Some(o) = o {
            rb = rb.header("origin", o);
        }
        rb.send()
    };
    assert_eq!(post("/v1/sessions", None).await.unwrap().status(), 403, "no Origin on a change");
    assert_eq!(post("/v1/sessions", Some("https://evil.example")).await.unwrap().status(), 403, "another site");
    assert_eq!(post("/v1/export", Some(&origin)).await.unwrap().status(), 403);
    assert_ne!(post("/v1/sessions", Some(&origin)).await.unwrap().status(), 403, "from this site it reaches the handler");
    assert_eq!(post("/v1/mcp/servers/x/approve", Some(&origin)).await.unwrap().status(), 403);
    // the browser's chat list deletes and resumes chats; the handler answers (404 for a missing chat), not the device guard
    let del = r.http.delete(format!("{}/v1/chats/s999", r.https)).header("cookie", &cookie).header("origin", &origin).send().await.unwrap();
    assert_eq!(del.status(), 404, "a browser may delete a chat");
    assert_eq!(post("/v1/chats/s999/resume", Some(&origin)).await.unwrap().status(), 404, "a browser may resume a chat");
}

#[tokio::test]
async fn revoking_a_device_ends_its_session_at_once_and_wrong_pins_lock_the_door() {
    let r = rig().await;
    let b = Browser::new();
    let dev = pair(&r, &b, "111222").await;
    let cookie = sign_in(&r, &b, &dev).await.unwrap();
    assert_eq!(r.http.get(format!("{}/v1/status", r.https)).header("cookie", &cookie).send().await.unwrap().status(), 200);
    let list: Value = r.http.get(format!("{}/v1/devices", r.local)).bearer_auth(&r.token).send().await.unwrap().json().await.unwrap();
    assert_eq!(list["devices"][0]["name"], "Firefox on Linux");
    assert_eq!(r.http.delete(format!("{}/v1/devices/{dev}", r.local)).bearer_auth(&r.token).send().await.unwrap().status(), 204);
    assert_eq!(r.http.get(format!("{}/v1/status", r.https)).header("cookie", &cookie).send().await.unwrap().status(), 401, "the session died with the device");
    assert!(sign_in(&r, &b, &dev).await.is_none(), "and it cannot sign in again");
    assert_eq!(r.http.delete(format!("{}/v1/devices/{dev}", r.local)).bearer_auth(&r.token).send().await.unwrap().status(), 404);

    // five wrong PINs drop every waiting request and lock the door for a minute
    let victim = Browser::new();
    r.http.post(format!("{}/pair/start", r.https)).json(&json!({"name": "x", "public_key": victim.jwk, "proof": web::proof_for("999999", &victim.jwk).unwrap()})).send().await.unwrap();
    for i in 0..5 {
        let c = r.web.accept(&format!("00000{i}")).unwrap_err().0.as_u16();
        assert_eq!(c, if i < 4 { 404 } else { 429 });
    }
    assert!(r.web.pending().is_empty());
    assert_eq!(r.web.accept("999999").unwrap_err().0.as_u16(), 429, "even the right PIN waits out the lock");
}

#[tokio::test]
async fn two_requests_that_share_a_pin_pair_neither() {
    let r = rig().await;
    let (a, b) = (Browser::new(), Browser::new());
    for br in [&a, &b] {
        r.http.post(format!("{}/pair/start", r.https)).json(&json!({"name": "dup", "public_key": br.jwk, "proof": web::proof_for("424242", &br.jwk).unwrap()})).send().await.unwrap();
    }
    assert_eq!(r.web.accept("424242").unwrap_err().0.as_u16(), 409);
    assert!(r.web.devices().is_empty() && r.web.pending().is_empty());
}

#[test]
fn a_certificate_is_made_once_and_reused() {
    let d = tempfile::tempdir().unwrap();
    let a = web::load_or_create_tls(d.path(), &["mypc.lan".to_string()]).unwrap().fingerprint;
    let b = web::load_or_create_tls(d.path(), &[]).unwrap().fingerprint;
    assert_eq!(a, b);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(d.path().join("key.der")).unwrap().permissions().mode() & 0o777, 0o600);
    }
}

#[tokio::test]
async fn the_browsers_own_cryptography_is_accepted() {
    // The browser code uses WebCrypto (P-256 keys, HMAC proof, IEEE P1363 signatures). Node has the same
    // API, so this runs those exact calls against the real server. Skipped where Node is not installed.
    if std::process::Command::new("node").arg("--version").output().is_err() {
        eprintln!("node not found; skipping");
        return;
    }
    let r = rig().await;
    let script = format!("{}/tests/interop.mjs", env!("CARGO_MANIFEST_DIR"));
    let out = tokio::process::Command::new("node").arg(script).args([&r.https, &r.local, &r.token]).env("NODE_TLS_REJECT_UNAUTHORIZED", "0").output().await.unwrap();
    assert!(out.status.success() && String::from_utf8_lossy(&out.stdout).contains("interop ok"), "stdout: {}\nstderr: {}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
}

#[tokio::test]
async fn pairing_ids_cannot_be_guessed_logout_ends_the_session_and_floods_are_capped() {
    let r = rig().await;
    let b = Browser::new();
    let start: Value = r.http.post(format!("{}/pair/start", r.https)).json(&json!({"name": "x", "public_key": b.jwk, "proof": web::proof_for("135790", &b.jwk).unwrap()})).send().await.unwrap().json().await.unwrap();
    let id = start["id"].as_str().unwrap();
    assert!(id.len() >= 20 && id != "p1", "{id}");
    // someone who tries the old sequential names learns nothing
    for guess in ["p1", "p2", "p3"] {
        let st: Value = r.http.get(format!("{}/pair/status/{guess}", r.https)).send().await.unwrap().json().await.unwrap();
        assert_eq!(st["state"], "expired", "{guess}");
    }
    // logout: the session stops working even if the cookie is kept
    let dev = pair(&r, &b, "246810").await;
    let cookie = sign_in(&r, &b, &dev).await.unwrap();
    assert_eq!(r.http.get(format!("{}/v1/status", r.https)).header("cookie", &cookie).send().await.unwrap().status(), 200);
    r.http.post(format!("{}/auth/logout", r.https)).header("cookie", &cookie).send().await.unwrap();
    assert_eq!(r.http.get(format!("{}/v1/status", r.https)).header("cookie", &cookie).send().await.unwrap().status(), 401);
    // challenges are capped for one device name; the pile stays bounded
    let mut refused = 0;
    for _ in 0..1100 {
        let c = r.http.post(format!("{}/auth/challenge", r.https)).json(&json!({"device_id": dev})).send().await.unwrap();
        if c.status() == 401 {
            refused += 1;
        }
    }
    assert!(refused > 0, "the 1001st open challenge must be refused");
}

#[tokio::test]
async fn a_browser_cannot_loosen_how_far_the_model_endpoint_is_trusted_with_memory() {
    let r = rig().await;
    let b = Browser::new();
    let dev = pair(&r, &b, "112233").await;
    let cookie = sign_in(&r, &b, &dev).await.unwrap();
    for (key, val) in [("memory.trust", "local"), ("permissions.bypass", "true"), ("provider.url", "http://evil/v1"), ("web.listen", "0.0.0.0:7843"), ("spending.mode", "none")] {
        let resp = r.http.put(format!("{}/v1/settings", r.https)).header("cookie", &cookie).header("origin", &r.https).json(&json!({ key: val })).send().await.unwrap();
        assert_eq!(resp.status(), 403, "{key} must stay on the machine");
    }
}

#[tokio::test]
async fn a_browser_cannot_open_a_project_at_the_root_or_in_a_hidden_folder_and_reads_home_only_with_questions() {
    let r = rig().await;
    let b = Browser::new();
    let dev = pair(&r, &b, "778899").await;
    let cookie = sign_in(&r, &b, &dev).await.unwrap();
    let origin = r.https.clone();
    let make = |body: Value| r.http.post(format!("{}/v1/sessions", r.https)).header("cookie", &cookie).header("origin", &origin).json(&body).send();
    let home = std::env::var("HOME").unwrap();
    for dir in ["/", "/etc", home.as_str(), &format!("{home}/.ssh"), "/tmp"] {
        let resp = make(json!({"model": "m", "project_dir": dir})).await.unwrap();
        assert_eq!(resp.status(), 403, "{dir} must be refused from a browser");
    }
    // the run endpoint has the same rule
    let resp = r.http.post(format!("{}/v1/team/runs", r.https)).header("cookie", &cookie).header("origin", &origin).json(&json!({"department": "x", "goal": "y", "project_dir": "/"})).send().await.unwrap();
    assert_eq!(resp.status(), 403);
    // the machine's own terminal is not limited
    let local = r.http.post(format!("{}/v1/sessions", r.local)).bearer_auth(&r.token).json(&json!({"model": "m", "project_dir": "/tmp"})).send().await.unwrap();
    assert_eq!(local.status(), 200);
}

#[tokio::test]
async fn nobody_can_ask_to_pair_until_the_user_opens_a_window_on_the_machine() {
    let r = rig().await;
    r.web.close_pairing();
    let b = Browser::new();
    let body = json!({"name": "x", "public_key": b.jwk, "proof": web::proof_for("121212", &b.jwk).unwrap()});
    let resp = r.http.post(format!("{}/pair/start", r.https)).json(&body).send().await.unwrap();
    assert_eq!(resp.status(), 403);
    assert!(resp.json::<Value>().await.unwrap()["error"].as_str().unwrap().contains("lh devices pair"));
    assert!(r.web.pending().is_empty(), "nothing waits");
    // the window opens from the machine only
    assert_eq!(r.http.post(format!("{}/v1/devices/open", r.https)).json(&json!({})).send().await.unwrap().status(), 401);
    let open = r.http.post(format!("{}/v1/devices/open", r.local)).bearer_auth(&r.token).json(&json!({"minutes": 2})).send().await.unwrap();
    assert_eq!(open.status(), 200);
    assert_eq!(r.http.post(format!("{}/pair/start", r.https)).json(&body).send().await.unwrap().status(), 201);
}

#[tokio::test]
async fn a_phone_signs_in_once_with_the_login_token_and_wrong_tokens_lock_the_door() {
    let r = rig().await;
    let path = r._dir.path().join("cfg/phone-login-token");
    r.web.enable_login(path.clone(), vec!["https://100.101.102.103:7843".into()]).unwrap();
    let token = std::fs::read_to_string(&path).unwrap();
    assert_eq!(token.len(), 64);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
    }
    // the machine's own apps get the link; a signed-in browser cannot
    let info: Value = r.http.get(format!("{}/v1/devices/phone", r.local)).bearer_auth(&r.token).send().await.unwrap().json().await.unwrap();
    assert_eq!(info["link"], format!("https://100.101.102.103:7843/#login={token}"));
    assert_eq!(r.http.get(format!("{}/v1/devices/phone", r.https)).send().await.unwrap().status(), 401);

    // without a session nothing private answers
    assert_eq!(r.http.get(format!("{}/v1/status", r.https)).send().await.unwrap().status(), 401);
    let ok = r.http.post(format!("{}/auth/token", r.https)).json(&json!({"token": token, "name": "Pixel"})).send().await.unwrap();
    assert_eq!(ok.status(), 200);
    let set = ok.headers().get("set-cookie").unwrap().to_str().unwrap().to_string();
    assert!(set.contains("HttpOnly") && set.contains("Secure") && set.contains("SameSite=Strict"), "{set}");
    let cookie = set.split(';').next().unwrap().to_string();
    assert_eq!(r.http.get(format!("{}/v1/status", r.https)).header("cookie", &cookie).send().await.unwrap().status(), 200);
    assert_eq!(r.http.get(format!("{}/v1/devices/phone", r.https)).header("cookie", &cookie).send().await.unwrap().status(), 403, "the token is never shown to a device");

    // the phone is a device like any other: listed, and removing it ends the session
    let list: Value = r.http.get(format!("{}/v1/devices", r.local)).bearer_auth(&r.token).send().await.unwrap().json().await.unwrap();
    let dev = list["devices"].as_array().unwrap().iter().find(|d| d["name"] == "Pixel").unwrap()["id"].as_str().unwrap().to_string();
    // a restart forgets sessions in memory, but the device keeps the session's hash
    let again = Web::new(Arc::new(Store::open(&r._dir.path().join("lh.db")).unwrap()), String::new());
    assert!(again.session_device(cookie.trim_start_matches("lh_session=")).is_some());
    assert!(again.session_device("nonsense").is_none());
    assert_eq!(r.http.delete(format!("{}/v1/devices/{dev}", r.local)).bearer_auth(&r.token).send().await.unwrap().status(), 204);
    assert_eq!(r.http.get(format!("{}/v1/status", r.https)).header("cookie", &cookie).send().await.unwrap().status(), 401);

    // rotating replaces the link; the old token no longer works
    let rotated: Value = r.http.post(format!("{}/v1/devices/phone/rotate", r.local)).bearer_auth(&r.token).send().await.unwrap().json().await.unwrap();
    assert_ne!(rotated["link"], info["link"]);
    assert_eq!(r.web.login_with_token(&token, "x").unwrap_err().0.as_u16(), 401);

    // five wrong tokens lock the door for a minute, even for the right one
    let fresh = std::fs::read_to_string(&path).unwrap();
    for i in 1..5 {
        assert_eq!(r.web.login_with_token(&format!("wrong{i}"), "x").unwrap_err().0.as_u16(), 401);
    }
    assert_eq!(r.http.post(format!("{}/auth/token", r.https)).json(&json!({"token": fresh})).send().await.unwrap().status(), 429);
}

#[test]
fn only_a_tailscale_address_is_taken_from_tailscale_ip() {
    assert_eq!(web::parse_tailscale_ip("100.101.102.103\n"), Some("100.101.102.103".parse().unwrap()));
    assert_eq!(web::parse_tailscale_ip("192.168.1.5\n100.64.0.1\n"), Some("100.64.0.1".parse().unwrap()));
    assert_eq!(web::parse_tailscale_ip("100.128.0.1\n0.0.0.0\n"), None);
    assert_eq!(web::parse_tailscale_ip(""), None);
}

#[test]
fn the_certificate_is_remade_when_a_new_address_is_needed() {
    let dir = tempfile::tempdir().unwrap();
    let a = web::load_or_create_tls(dir.path(), &[]).unwrap().fingerprint;
    assert_eq!(web::load_or_create_tls(dir.path(), &[]).unwrap().fingerprint, a);
    let b = web::load_or_create_tls(dir.path(), &["100.64.0.1".into()]).unwrap().fingerprint;
    assert_ne!(a, b);
    assert_eq!(web::load_or_create_tls(dir.path(), &["100.64.0.1".into()]).unwrap().fingerprint, b);
}

/// The browser page draws the new-chat screen as the desktop app does: a hello for the time of day with the
/// person's own name (never a built-in one), the Chat, Assistant and Project switch, four suggestions and Private.
#[test]
fn the_new_chat_screen_matches_the_desktop_app() {
    let html = include_str!("../web/index.html");
    let js = include_str!("../web/app.js");
    for id in ["id=\"greeting-sub\"", "id=\"modes\"", "data-mode=\"assistant\"", "data-mode=\"project\"", "id=\"kind-hint\"", "id=\"chips\"", "id=\"private-btn\""] {
        assert!(html.contains(id), "index.html lacks {id}");
    }
    for s in ["Plan a project", "Summarise a file", "Ask about my notes", "Just chat", "Everything stays on this computer"] {
        assert!(html.contains(s), "index.html lacks {s}");
    }
    for s in ["Good morning", "Good afternoon", "Good evening", "Plain conversation, no tools", "incognito: !!chat.incognito", "PAGE_ICON", "Add font…"] {
        assert!(js.contains(s), "app.js lacks {s}");
    }
    assert!(!js.contains("What would you like to explore?"), "the old greeting is gone");
}

#[tokio::test]
async fn hooks_are_read_from_a_browser_and_changed_only_on_the_machine() {
    let r = rig().await;
    assert_eq!(r.http.get(format!("{}/v1/hooks", r.https)).send().await.unwrap().status(), 401, "no session");
    let b = Browser::new();
    let dev = pair(&r, &b, "445566").await;
    let cookie = sign_in(&r, &b, &dev).await.unwrap();
    let v: Value = r.http.get(format!("{}/v1/hooks", r.https)).header("cookie", &cookie).header("origin", &r.https).send().await.unwrap().json().await.unwrap();
    assert_eq!(v["before"], json!([]), "{v}");
    let resp = r.http.post(format!("{}/v1/hooks", r.https)).header("cookie", &cookie).header("origin", &r.https).json(&json!({"when": "after", "run": "cargo fmt"})).send().await.unwrap();
    assert_eq!(resp.status(), 403, "hooks run programs, so a browser cannot add one");
    assert_eq!(r.http.post(format!("{}/v1/hooks", r.local)).json(&json!({"when": "after", "run": "x"})).send().await.unwrap().status(), 401, "no token");
    let call = |m: reqwest::Method, path: &str, body: Option<Value>| {
        let mut q = r.http.request(m, format!("{}{path}", r.local)).bearer_auth(&r.token);
        if let Some(b) = body {
            q = q.json(&b);
        }
        q.send()
    };
    let resp = call(reqwest::Method::POST, "/v1/hooks", Some(json!({"when": "after", "tools": "write_file, edit_file", "run": "cargo fmt", "secs": "5"}))).await.unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(call(reqwest::Method::POST, "/v1/hooks", Some(json!({"when": "before", "run": " "}))).await.unwrap().status(), 400, "an empty command is refused");
    assert_eq!(call(reqwest::Method::POST, "/v1/hooks", Some(json!({"when": "before", "run": "x", "secs": "0"}))).await.unwrap().status(), 400, "the time limit is 1 to 600 seconds");
    assert_eq!(call(reqwest::Method::POST, "/v1/hooks", Some(json!({"when": "during", "run": "x"}))).await.unwrap().status(), 400);
    let file = r._dir.path().join("hooks.toml");
    let h = lh_tools::Hooks::load(&file).unwrap();
    assert_eq!(h.after[0].tools, vec!["write_file", "edit_file"]);
    assert_eq!(h.after[0].timeout.as_secs(), 5);
    let v: Value = call(reqwest::Method::PUT, "/v1/hooks/after/0", Some(json!({"run": "just fmt"}))).await.unwrap().json().await.unwrap();
    assert_eq!(v["after"][0], json!({"tools": [], "run": "just fmt", "timeout_secs": 30}), "{v}");
    assert_eq!(lh_tools::Hooks::load(&file).unwrap().after[0].run, "just fmt");
    assert_eq!(call(reqwest::Method::PUT, "/v1/hooks/after/3", Some(json!({"run": "x"}))).await.unwrap().status(), 404);
    let v: Value = call(reqwest::Method::DELETE, "/v1/hooks/after/0", None).await.unwrap().json().await.unwrap();
    assert_eq!(v["after"], json!([]));
    assert!(lh_tools::Hooks::load(&file).unwrap().is_empty());
    let a: Value = call(reqwest::Method::GET, "/v1/activity?kind=hooks", None).await.unwrap().json().await.unwrap();
    let ops: Vec<&str> = a["entries"].as_array().unwrap().iter().map(|e| e["detail"]["op"].as_str().unwrap_or("")).collect();
    assert_eq!(ops, vec!["removed", "edited", "added"], "{a}");
    assert!(a["entries"].as_array().unwrap().iter().all(|e| e["detail"]["device"] == json!("local")), "{a}");
}

#[test]
fn the_hooks_page_uses_the_core() {
    let js = include_str!("../web/app.js");
    assert!(js.contains("'/v1/hooks'") && js.contains("Before a tool") && js.contains("After a tool"));
}

#[test]
fn the_look_drives_rounding_and_glass_and_the_folded_mark_opens_the_list() {
    let js = include_str!("../web/app.js");
    let css = include_str!("../web/style.css");
    let html = include_str!("../web/index.html");
    assert!(js.contains("'--inner'") && js.contains("'--glass-fill'"));
    assert!(!css.contains("border-radius: 8px;"), "corners follow the look");
    assert!(css.contains("var(--glass-fill"));
    assert!(html.contains("id=\"side-logo\"") && js.contains("#side-logo"));
    assert!(css.contains(".side-foot { margin-top: auto; }"));
}

#[test]
fn motion_is_vendored_and_stills_when_asked() {
    let js = include_str!("../web/app.js");
    let css = include_str!("../web/style.css");
    let html = include_str!("../web/index.html");
    let motion = include_str!("../web/motion.js");
    assert!(motion.contains("Motion"), "motion.js is the motion.dev build");
    assert!(html.contains("<script src=\"/motion.js\"></script>"), "served from lhd, not a CDN");
    assert!(!html.contains("https://"), "nothing loads from the internet");
    assert!(js.contains("prefers-reduced-motion") && css.contains("prefers-reduced-motion"));
    assert!(css.contains("prefers-reduced-transparency"));
    assert!(include_str!("../../../THIRD-PARTY-NOTICES.md").contains("motion.js"));
}
