//! `lhd`: the LastHarness core. Listens on loopback only.
//!
//! lhd --provider http://localhost:9292/v1 [--key secret:NAME] [--listen 127.0.0.1:7842]
//!     [--token-file PATH] [--allow-sudo] [--allow "git status"]... [--dangerously-skip-permissions]

use std::net::SocketAddr;

use lh_core::{auth, router, AppState, Settings};
use lh_provider::Provider;
use lh_secrets::{resolve, OsStore};

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("lhd: {msg}");
    std::process::exit(2)
}

#[tokio::main]
async fn main() {
    let mut a = std::env::args().skip(1);
    let (mut listen, mut provider, mut key, mut token_file) = ("127.0.0.1:7842".to_string(), None, None, None);
    let mut settings = Settings { bypass: false, sudo_enabled: false, allow_rules: vec![] };
    while let Some(arg) = a.next() {
        let mut val = || a.next().unwrap_or_else(|| fail(format!("{arg} needs a value")));
        match arg.as_str() {
            "--listen" => listen = val(),
            "--provider" => provider = Some(val()),
            "--key" => key = Some(val()),
            "--token-file" => token_file = Some(std::path::PathBuf::from(val())),
            "--allow" => settings.allow_rules.push(val()),
            "--allow-sudo" => settings.sudo_enabled = true,
            "--dangerously-skip-permissions" => {
                eprintln!("WARNING: all permission prompts are off. You accept the risk; the project is not responsible for the outcome.");
                settings.bypass = true;
            }
            other => fail(format!("unknown option {other}")),
        }
    }
    let provider = provider.unwrap_or_else(|| fail("--provider URL is required"));
    let addr: SocketAddr = listen.parse().unwrap_or_else(|e| fail(format!("--listen: {e}")));
    if !addr.ip().is_loopback() {
        fail("the local API only listens on loopback until device pairing exists");
    }
    let api_key = key.map(|k| match resolve(&OsStore::new("lastharness"), &k) {
        Ok(Some(v)) => v,
        Ok(None) => fail(format!("no secret stored under {k}")),
        Err(e) => fail(e),
    });

    let token = auth::new_token();
    let path = token_file.unwrap_or_else(auth::default_token_path);
    auth::write_token(&path, &token).unwrap_or_else(|e| fail(format!("cannot write the token file {}: {e}", path.display())));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap_or_else(|e| fail(format!("cannot listen on {addr}: {e}")));
    let port = listener.local_addr().unwrap().port();
    let app = router(AppState::new(Provider::new(provider, api_key), settings), token, port);
    eprintln!("lhd: listening on http://127.0.0.1:{port}, token in {}", path.display());
    axum::serve(listener, app).with_graceful_shutdown(async { let _ = tokio::signal::ctrl_c().await; }).await.unwrap();
}
