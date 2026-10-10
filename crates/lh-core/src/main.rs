//! `lhd`: the LastHarness core. Listens on loopback only.
//!
//! lhd --provider http://localhost:9292/v1 [--key secret:NAME] [--listen 127.0.0.1:7842]
//!     [--token-file PATH] [--store PATH] [--spend-config PATH] [--compact-at TOKENS] [--idle-after SECONDS] [--searxng URL] [--image-model NAME] [--allow-sudo] [--allow "git status"]... [--dangerously-skip-permissions]

use std::net::SocketAddr;

use std::sync::Arc;

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
    let (mut store_path, mut spend_path, mut compact_at, mut idle_after) = (None, None, None::<u64>, 600u64);
    let (mut searxng, mut image_model) = (None, None);
    let mut flags = (false, false, Vec::<String>::new()); // bypass, sudo, allow rules
    while let Some(arg) = a.next() {
        let mut val = || a.next().unwrap_or_else(|| fail(format!("{arg} needs a value")));
        match arg.as_str() {
            "--listen" => listen = val(),
            "--provider" => provider = Some(val()),
            "--key" => key = Some(val()),
            "--token-file" => token_file = Some(std::path::PathBuf::from(val())),
            "--store" => store_path = Some(std::path::PathBuf::from(val())),
            "--spend-config" => spend_path = Some(std::path::PathBuf::from(val())),
            "--compact-at" => compact_at = Some(val().parse().unwrap_or_else(|_| fail("--compact-at needs a number of tokens (0 turns it off)"))),
            "--idle-after" => idle_after = val().parse().unwrap_or_else(|_| fail("--idle-after needs a number of seconds (0 means never)")),
            "--searxng" => searxng = Some(val()),
            "--image-model" => image_model = Some(val()),
            "--allow" => flags.2.push(val()),
            "--allow-sudo" => flags.1 = true,
            "--dangerously-skip-permissions" => {
                eprintln!("WARNING: all permission prompts are off. You accept the risk; the project is not responsible for the outcome.");
                flags.0 = true;
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

    let store_path = store_path.unwrap_or_else(default_store_path);
    let store = Arc::new(lh_store::Store::open(&store_path).unwrap_or_else(|e| fail(format!("{}: {e}", store_path.display()))));
    let spend_cfg = match spend_path.or_else(|| Some(config_dir().join("spend.toml")).filter(|p| p.exists())) {
        Some(p) => lh_spend::Config::from_toml(&std::fs::read_to_string(&p).unwrap_or_else(|e| fail(format!("{}: {e}", p.display())))).unwrap_or_else(|e| fail(format!("{e}"))),
        None => lh_spend::Config::default(),
    };
    let spend = lh_spend::SpendMeter::new(store.clone(), spend_cfg, provider.clone(), lh_provider::is_local_url(&provider));
    let _ = store.log("startup", "-", "lhd", &serde_json::json!({"provider": provider, "local": lh_provider::is_local_url(&provider), "bypass": flags.0, "sudo_enabled": flags.1, "allow_rules": flags.2}));
    let settings = Settings { image_model, web: lh_tools::WebConfig { searxng_url: searxng, allow_private: false }, idle_after: std::time::Duration::from_secs(idle_after), compact_at, store, spend, bypass: flags.0, sudo_enabled: flags.1, allow_rules: flags.2 };

    let token = auth::new_token();
    let path = token_file.unwrap_or_else(auth::default_token_path);
    auth::write_token(&path, &token).unwrap_or_else(|e| fail(format!("cannot write the token file {}: {e}", path.display())));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap_or_else(|e| fail(format!("cannot listen on {addr}: {e}")));
    let port = listener.local_addr().unwrap().port();
    let app = router(AppState::new(Provider::new(provider, api_key), settings), token, port);
    eprintln!("lhd: listening on http://127.0.0.1:{port}, token in {}", path.display());
    axum::serve(listener, app).with_graceful_shutdown(async { let _ = tokio::signal::ctrl_c().await; }).await.unwrap();
}

fn home() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
}
fn config_dir() -> std::path::PathBuf {
    std::env::var_os("XDG_CONFIG_HOME").map(std::path::PathBuf::from).unwrap_or_else(|| home().join(".config")).join("lastharness")
}
fn default_store_path() -> std::path::PathBuf {
    std::env::var_os("XDG_DATA_HOME").map(std::path::PathBuf::from).unwrap_or_else(|| home().join(".local/share")).join("lastharness/lh.db")
}
