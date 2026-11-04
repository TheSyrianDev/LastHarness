//! `lhd`: the LastHarness core. Listens on loopback only.
//!
//! lhd [--provider http://localhost:9292/v1] [--config PATH] [--key secret:NAME] [--listen 127.0.0.1:7842]
//!     Settings come from the settings file (`lh setup` writes it); a flag beats the file.
//!     [--token-file PATH] [--store PATH] [--spend-config PATH] [--compact-at TOKENS] [--idle-after SECONDS] [--searxng URL] [--image-model NAME]
//!     [--provider-name NAME] [--no-memory] [--maintain-when-idle]
//!     [--learn propose|silent|off] [--learn-threshold N] [--skills-dir PATH] [--skill-min-calls N] [--memory-dir PATH] [--trust local|own|any]
//!     [--embed-server PATH --embed-model FILE.gguf [--embed-libs DIRS] [--embed-name NAME]]
//!     [--embed-url URL --embed-name NAME [--embed-allow-cloud]] [--allow-sudo] [--allow "git status"]... [--dangerously-skip-permissions] [--no-sandbox]

use std::net::SocketAddr;

use std::sync::Arc;

use lh_core::{auth, AppState, Settings};
use lh_provider::Provider;
use lh_secrets::{resolve, OsStore};

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("lhd: {msg}");
    std::process::exit(2)
}

#[tokio::main]
async fn main() {
    lh_config::platform_dirs();
    let mut a = std::env::args().skip(1);
    let (mut listen, mut provider, mut key, mut token_file) = ("127.0.0.1:7842".to_string(), None, None, None);
    let (mut store_path, mut spend_path, mut compact_at, mut idle_after) = (None, None, None::<u64>, None::<u64>);
    let (mut searxng, mut image_model) = (None, None);
    let mut maintain_idle = false;
    let mut web_listen = None::<String>;
    let mut config_path = None::<std::path::PathBuf>;
    let mut provider_name = None::<String>;
    let (mut no_team, mut team_dir, mut team_tokens) = (false, None::<std::path::PathBuf>, 4000u32);
    let (mut learn_mode, mut learn_threshold, mut skills_dir, mut skill_calls) = (None::<String>, None::<usize>, None::<std::path::PathBuf>, 6usize);
    let (mut no_memory, mut memory_dir, mut trust) = (false, None::<std::path::PathBuf>, None::<String>);
    let (mut embed_server, mut embed_model, mut embed_libs, mut embed_name, mut embed_url, mut embed_cloud) = (None::<String>, None::<String>, None::<String>, None::<String>, None::<String>, false);
    let mut no_sandbox = false;
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
            "--idle-after" => idle_after = val().parse().map(Some).unwrap_or_else(|_| fail("--idle-after needs a number of seconds (0 means never)")),
            "--provider-name" => provider_name = Some(val()),
            "--learn" => learn_mode = Some(val()),
            "--learn-threshold" => learn_threshold = Some(val().parse().unwrap_or_else(|_| fail("--learn-threshold needs a number"))),
            "--skills-dir" => skills_dir = Some(std::path::PathBuf::from(val())),
            "--skill-min-calls" => skill_calls = val().parse().unwrap_or_else(|_| fail("--skill-min-calls needs a number")),
            "--no-memory" => no_memory = true,
            "--web-listen" => web_listen = Some(val()),
            "--config" => config_path = Some(std::path::PathBuf::from(val())),
            "--no-team" => no_team = true,
            "--team-dir" => team_dir = Some(std::path::PathBuf::from(val())),
            "--team-max-tokens" => team_tokens = val().parse().unwrap_or_else(|_| fail("--team-max-tokens needs a number")),
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
            "--no-sandbox" => {
                no_sandbox = true;
                eprintln!("{}", NO_SANDBOX_WARNING);
            }
            "--dangerously-skip-permissions" => {
                eprintln!("WARNING: all permission prompts are off. You accept the risk; the project is not responsible for the outcome.");
                flags.0 = true;
            }
            other => fail(format!("unknown option {other}")),
        }
    }
    let config_path = config_path.unwrap_or_else(lh_config::ConfigFile::default_path);
    let cfg = lh_config::ConfigFile::load(&config_path).unwrap_or_else(|e| fail(e));
    // a flag beats the settings file, and the file beats the default
    let provider = provider.or_else(|| cfg.get_opt("provider.url")).unwrap_or_else(|| fail(format!("no provider: give --provider URL, or run `lh setup` (settings are read from {})", config_path.display())));
    // A crash report is only prepared, never sent, and only when the user has not switched them off.
    // It holds the version, the system and the place in the code: no message text, paths or names.
    // The hook is always in place and looks at the setting when something goes wrong, so switching reports
    // on or off takes effect without a restart.
    {
        let file = config_path.parent().map(|p| p.join("crash-pending.json"));
        let cfg_file = config_path.clone();
        std::panic::set_hook(Box::new(move |info| {
            if lh_config::ConfigFile::load(&cfg_file).map(|c| c.get("reports.crash")).unwrap_or_else(|_| "off".into()) == "off" {
                return;
            }
            let at = info.location().map(|l| format!("{}:{}", l.file().rsplit('/').next().unwrap_or(""), l.line())).unwrap_or_default();
            let report = serde_json::json!({"version": env!("CARGO_PKG_VERSION"), "os": std::env::consts::OS, "arch": std::env::consts::ARCH, "where": at});
            if let Some(f) = &file {
                let _ = std::fs::write(f, report.to_string());
            }
            eprintln!("lhd stopped unexpectedly at {at}. A report was prepared; `lh settings reports` says what it holds.");
        }));
    }
    let key = key.or_else(|| cfg.get_opt("provider.key"));
    let provider_name = provider_name.or_else(|| cfg.get_opt("provider.name"));
    let idle_after = idle_after.or_else(|| cfg.get_num("core.idle_after")).unwrap_or(600);
    // Flags only: the file's values for web search, the proxy, images, compaction, spending and permissions are
    // applied (and re-applied on every change) by `AppState::apply_settings`, with a flag still winning.
    let no_memory = no_memory || !cfg.get_bool("memory.enabled");
    let trust = trust.or_else(|| Some(cfg.get("memory.trust")).filter(|t| t != "auto"));
    let learn_mode = learn_mode.unwrap_or_else(|| cfg.get("learning.mode"));
    let learn_threshold = learn_threshold.or_else(|| cfg.get_num("learning.threshold")).unwrap_or(3);
    let (bypass_now, sudo_now) = (flags.0 || cfg.get_bool("permissions.bypass"), flags.1 || cfg.get_bool("permissions.sudo"));
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
    let _ = store.log("startup", "-", "lhd", &serde_json::json!({"provider": provider, "local": lh_provider::is_local_url(&provider), "bypass": bypass_now, "sudo_enabled": sudo_now, "allow_rules": flags.2, "memory": !no_memory, "learn": learn_mode}));
    let provider_trust = match trust.as_deref() {
        Some(t) => lh_memory::Trust::parse(t).unwrap_or_else(|| fail("--trust must be local, own or any")),
        None if lh_provider::is_local_url(&provider) => lh_memory::Trust::Local,
        None => lh_memory::Trust::AnyCloud,
    };
    let learn = match learn_mode.as_str() {
        "off" => None,
        "propose" | "silent" => {
            let dir = skills_dir.unwrap_or_else(default_skills_dir);
            let l = lh_learn::Learn::new(store.clone()).with_skills_dir(dir);
            Some(lh_core::LearnRuntime { learn: Arc::new(l), trust: provider_trust, mode: if learn_mode == "silent" { lh_core::LearnMode::Silent } else { lh_core::LearnMode::Propose }, threshold: learn_threshold.max(2), skill_min_calls: skill_calls })
        }
        _ => fail("--learn must be propose, silent or off"),
    };
    let memory = if no_memory {
        None
    } else {
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
    let team = if no_team {
        None
    } else {
        let dir = team_dir.unwrap_or_else(default_team_dir);
        match lh_core::TeamRuntime::open(&dir, std::sync::Arc::new(lh_learn::Learn::new(store.clone())), team_tokens) {
            Ok(t) => Some(t),
            Err(e) => fail(format!("cannot open the roles and departments in {}: {e}", dir.display())),
        }
    };
    if team.is_some() {
        use lh_team::{Autonomy, TeamSettings, UnavailablePolicy};
        let t = TeamSettings::new(&store);
        let apply: lh_team::Result<()> = (|| {
            if cfg.is_set("team.autonomy") {
                t.set_autonomy(Autonomy::parse(&cfg.get("team.autonomy")).unwrap_or(Autonomy::PlanFirst))?;
            }
            if let (true, Some(n)) = (cfg.is_set("team.parallel"), cfg.get_num("team.parallel")) {
                t.set_parallel(n)?;
            }
            if let (true, Some(n)) = (cfg.is_set("team.attempts"), cfg.get_num("team.attempts")) {
                t.set_attempts(n)?;
            }
            if cfg.is_set("team.unavailable") {
                t.set_unavailable(if cfg.get("team.unavailable") == "fallback" { UnavailablePolicy::Fallback } else { UnavailablePolicy::Stop })?;
            }
            Ok(())
        })();
        if let Err(e) = apply {
            fail(format!("the team settings in {} are not usable: {e}", config_path.display()));
        }
    }
    let mcp = (!no_team).then(|| lh_core::McpRuntime::open(default_store_path().parent().map(|p| p.join("mcp.json")).unwrap_or_else(|| std::path::PathBuf::from("mcp.json"))));
    let web_store = store.clone();
    let channel_store = store.clone();
    let schedule_store = store.clone();
    let settings = Settings { provider_name, learn, maintain_when_idle: maintain_idle, memory, image_model, web: lh_tools::WebConfig { searxng_url: searxng, allow_private: false, use_proxy: false }, idle_after: std::time::Duration::from_secs(idle_after), compact_at, store, spend, bypass: flags.0, sudo_enabled: flags.1, allow_rules: flags.2, team, mcp, ask_all: false, config: Some(config_path.clone()) };

    let token = auth::new_token();
    let path = token_file.unwrap_or_else(auth::default_token_path);
    // take the port first: a second lhd that cannot listen must not replace the running core's token
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap_or_else(|e| fail(format!("cannot listen on {addr}: {e}")));
    auth::write_token(&path, &token).unwrap_or_else(|e| fail(format!("cannot write the token file {}: {e}", path.display())));
    let port = listener.local_addr().unwrap().port();
    let (provider_url, api_key_for_kv) = (provider.clone(), api_key.clone());
    let state = AppState::new(Provider::new(provider, api_key), settings);
    state.set_kv_endpoint(&provider_url, api_key_for_kv);
    state.apply_settings(&cfg, &[]);
    if !no_sandbox && !cfg!(target_os = "linux") {
        eprintln!("lhd: WARNING: the sandbox needs bubblewrap, which only Linux has. The model's commands run without it here: they can change any file your user can. Permission prompts still apply.");
    }
    if no_sandbox {
        state.turn_sandbox_off();
    }
    // the browser's door: off unless an address is given
    let web_listen = web_listen.or_else(|| cfg.get_opt("web.listen"));
    let tailscale = cfg.get("web.tailscale") == "true";
    let web = if web_listen.is_none() && !tailscale {
        None
    } else {
        let waddr: Option<SocketAddr> = web_listen.as_ref().map(|w| w.parse().unwrap_or_else(|e| fail(format!("web.listen {w}: {e}"))));
        // the Tailscale door: opt-in, on the tailnet address only (never 0.0.0.0), same port as web.listen
        let ts = tailscale.then(|| match lh_core::web::tailscale_ipv4() {
            Ok(ip) => Some(SocketAddr::new(ip.into(), waddr.map(|a| a.port()).unwrap_or(7843))),
            Err(e) => {
                eprintln!("lhd: web.tailscale is on but {e}; the phone access stays closed");
                None
            }
        }).flatten();
        let mut extra: Vec<String> = waddr.iter().chain(ts.iter()).filter(|a| !a.ip().is_unspecified() && !a.ip().is_loopback()).map(|a| a.ip().to_string()).collect();
        extra.dedup();
        let dir = default_store_path().parent().map(|p| p.join("web")).unwrap_or_else(|| std::path::PathBuf::from("web"));
        let tls = lh_core::web::load_or_create_tls(&dir, &extra).unwrap_or_else(|e| fail(e));
        let w = lh_core::web::Web::new(web_store, tls.fingerprint.clone());
        let router = lh_core::web_router(state.clone(), w.clone());
        let mut phone_urls = vec![];
        for a in waddr.iter().chain(ts.iter()) {
            let wl = tokio::net::TcpListener::bind(a).await.unwrap_or_else(|e| fail(format!("cannot listen on {a}: {e}")));
            let at = wl.local_addr().unwrap();
            if !at.ip().is_loopback() {
                phone_urls.push(format!("https://{at}"));
            }
            eprintln!("lhd: browser UI at https://{at}  (certificate fingerprint {})", tls.fingerprint);
            tokio::spawn(lh_core::web::serve_tls(wl, router.clone(), tls.acceptor.clone()));
        }
        // tailscale first: it is the address a phone can reach from anywhere
        phone_urls.sort_by_key(|u| !ts.is_some_and(|t| u.contains(&t.ip().to_string())));
        let login_path = config_path.parent().map(|p| p.join("phone-login-token")).unwrap_or_else(|| std::path::PathBuf::from("phone-login-token"));
        w.enable_login(login_path, phone_urls).unwrap_or_else(|e| fail(e));
        eprintln!("lhd: a new browser pairs with a PIN (`lh devices`); a phone signs in with `lh devices phone`");
        Some(w)
    };
    // messaging channels: the runner talks to this core through its own local API, like any other client
    lh_core::channels::spawn_runner(channel_store, format!("http://127.0.0.1:{port}"), token.clone());
    // scheduled tasks start the same way, as chats through the local API
    lh_core::schedules_api::spawn_runner(schedule_store, format!("http://127.0.0.1:{port}"), token.clone());
    let app = lh_core::router_with_web(state, token, port, web);
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
fn default_team_dir() -> std::path::PathBuf {
    default_store_path().parent().map(|p| p.join("team")).unwrap_or_else(|| std::path::PathBuf::from("team"))
}
fn default_skills_dir() -> std::path::PathBuf {
    default_store_path().parent().map(|p| p.join("skills")).unwrap_or_else(|| std::path::PathBuf::from("skills"))
}

/// Printed when lhd starts with `--no-sandbox`.
const NO_SANDBOX_WARNING: &str = "WARNING: --no-sandbox turns the sandbox off. The model's commands can change any file your user can, not only the chat's folder, and reach the network freely. Permission prompts still apply.";
