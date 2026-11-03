//! The local API is only for this machine and only for clients that hold the token.
//! Two checks on every request: the `Host` header must be a loopback name with our port (stops a
//! web page from reaching us through DNS rebinding), and the bearer token must match. Browsers
//! send an `Origin` header on cross-site calls; none is accepted until the browser UI exists.

use std::path::Path;
use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{header, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

pub fn new_token() -> String {
    let mut b = [0u8; 32];
    getrandom::fill(&mut b).expect("the OS random source is unavailable");
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Write the token where only this user can read it.
pub fn write_token(path: &Path, token: &str) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        f.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    f.write_all(token.as_bytes())
}

/// Where the token lives by default: the runtime folder if there is one, else the state folder.
pub fn default_token_path() -> std::path::PathBuf {
    if let Some(d) = std::env::var_os("XDG_RUNTIME_DIR") {
        return Path::new(&d).join("lastharness").join("token");
    }
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from).unwrap_or_default();
    home.join(".local/state/lastharness/token")
}

fn same(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

pub struct Guard {
    pub token: String,
    pub hosts: Vec<String>,
}

pub async fn require(State(g): State<Arc<Guard>>, req: Request, next: Next) -> Response {
    let h = req.headers();
    let host_ok = h.get(header::HOST).and_then(|v| v.to_str().ok()).is_some_and(|v| g.hosts.iter().any(|x| x == v));
    if !host_ok {
        return (StatusCode::FORBIDDEN, "wrong Host header").into_response();
    }
    if h.contains_key(header::ORIGIN) {
        return (StatusCode::FORBIDDEN, "cross-site requests are not accepted").into_response();
    }
    let token_ok = h
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .is_some_and(|t| same(t.as_bytes(), g.token.as_bytes()));
    if !token_ok {
        return (StatusCode::UNAUTHORIZED, "missing or wrong token").into_response();
    }
    next.run(req).await
}
