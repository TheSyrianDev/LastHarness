//! `lhd`: the LastHarness core. Listens on loopback only.
//!
//! lhd --provider http://localhost:9292/v1 [--key secret:NAME] [--listen 127.0.0.1:7842]
//!     [--token-file PATH] [--store PATH] [--spend-config PATH] [--compact-at TOKENS] [--idle-after SECONDS] [--searxng URL] [--image-model NAME]
//!     [--no-memory] [--maintain-when-idle] [--memory-dir PATH] [--trust local|own|any]
//!     [--embed-server PATH --embed-model FILE.gguf [--embed-libs DIRS] [--embed-name NAME]]
//!     [--embed-url URL --embed-name NAME [--embed-allow-cloud]] [--allow-sudo] [--allow "git status"]... [--dangerously-skip-permissions]

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
    let mut maintain_idle = false;
    let (mut no_memory, mut memory_dir, mut trust) = (false, None::<std::path::PathBuf>, None::<String>);
    let (mut embed_server, mut embed_model, mut embed_libs, mut embed_name, mut embed_url, mut embed_cloud) = (None::<String>, None::<String>, None::<String>, None::<String>, None::<String>, false);
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
            "--no-memory" => no_memory = true,
            "--maintain-when-idle" => maintain_idle = true,
            "--memory-dir" => memory_dir = Some(std::path::PathBuf::from(val())),
            "--trust" => trust = Some(val()),
            "--embed-server" => embed_server = Some(val()),
            "--embed-model" => embed_model = Some(val()),
            "--embed-libs" => embed_libs = Some(val()),
            "--embed-name" => embed_name = Some(val()),
            "--embed-url" => embed_url = Some(val()),
            "--embed-allow-cloud" => embed_cloud = true,
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
    let _ = store.log("startup", "-", "lhd", &serde_json::json!({"provider": provider, "local": lh_provider::is_local_url(&provider), "bypass": flags.0, "sudo_enabled": flags.1, "allow_rules": flags.2, "memory": !no_memory}));
    let memory = if no_memory {
        None
    } else {
        let provider_trust = match trust.as_deref() {
            Some(t) => lh_memory::Trust::parse(t).unwrap_or_else(|| fail("--trust must be local, own or any")),
            None if lh_provider::is_local_url(&provider) => lh_memory::Trust::Local,
            None => lh_memory::Trust::AnyCloud,
        };
        let dir = memory_dir.unwrap_or_else(default_memory_dir);
        let memory = Arc::new(lh_memory::Memory::open(store.clone(), &dir).unwrap_or_else(|e| fail(format!("{}: {e}", dir.display()))));
        let _ = memory.scan();
        use lh_memory::embed::*;
        let embedder = match (embed_server, embed_model, embed_url) {
            (Some(server), Some(model), None) => AnyEmbedder::Managed(Box::new(ManagedEmbedder::new(ManagedConfig {
                server: server.into(),
                library_path: embed_libs,
                model_file: model.into(),
                model_name: embed_name.clone().unwrap_or_else(|| "embeddinggemma-2".into()),
                prefixes: Prefixes::embedding_gemma(),
                threads: 6,
            }))),
            (None, None, Some(url)) => AnyEmbedder::Http(
                HttpEmbedder::new(&url, &embed_name.clone().unwrap_or_else(|| fail("--embed-url needs --embed-name")), None, Prefixes::embedding_gemma(), embed_cloud).unwrap_or_else(|e| fail(e)),
            ),
            (None, None, None) => AnyEmbedder::Off,
            _ => fail("use either --embed-server with --embed-model, or --embed-url, not a mix"),
        };
        Some(lh_core::MemoryRuntime { memory, embedder: Arc::new(embedder), trust: provider_trust, cfg: lh_memory::RecallConfig::default() })
    };
    let settings = Settings { maintain_when_idle: maintain_idle, memory, image_model, web: lh_tools::WebConfig { searxng_url: searxng, allow_private: false }, idle_after: std::time::Duration::from_secs(idle_after), compact_at, store, spend, bypass: flags.0, sudo_enabled: flags.1, allow_rules: flags.2 };

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
fn default_memory_dir() -> std::path::PathBuf {
    default_store_path().parent().map(|p| p.join("memory")).unwrap_or_else(|| std::path::PathBuf::from("memory"))
}
