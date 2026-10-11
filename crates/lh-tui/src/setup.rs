//! First run: `lh setup`. Plain questions, one at a time, each with a safe default that Enter accepts.
//! It writes the settings file and nothing else; the same settings can be changed later with
//! `lh settings`. (How first-run choices are best presented is still open; this is the step-by-step form.)

use lh_config::ConfigFile;

/// Where answers come from. The terminal reads lines; tests give a script.
pub trait Prompter {
    fn say(&mut self, text: &str);
    /// Ask a question; an empty answer means the default.
    fn ask(&mut self, question: &str, default: &str) -> String;
    /// Ask for a secret. The terminal cannot hide typing portably, so it says so first.
    fn secret(&mut self, question: &str) -> String;
}

pub struct Terminal;
impl Prompter for Terminal {
    fn say(&mut self, text: &str) {
        println!("{text}");
    }
    fn ask(&mut self, question: &str, default: &str) -> String {
        use std::io::Write;
        print!("{question} [{default}] ");
        let _ = std::io::stdout().flush();
        let mut l = String::new();
        let _ = std::io::stdin().read_line(&mut l);
        let l = l.trim();
        if l.is_empty() { default.to_string() } else { l.to_string() }
    }
    fn secret(&mut self, question: &str) -> String {
        use std::io::Write;
        print!("{question} (what you type is shown on screen) ");
        let _ = std::io::stdout().flush();
        let mut l = String::new();
        let _ = std::io::stdin().read_line(&mut l);
        l.trim().to_string()
    }
}

/// Ask until the answer is a valid value for the key.
fn ask_valid(p: &mut dyn Prompter, cfg: &mut ConfigFile, key: &str, question: &str) {
    let default = cfg.get(key);
    loop {
        let a = p.ask(question, &default);
        match cfg.set(key, &a) {
            Ok(_) => return,
            Err(e) => p.say(&format!("  {e}")),
        }
    }
}

fn yes_no(p: &mut dyn Prompter, cfg: &mut ConfigFile, key: &str, question: &str) {
    let default = if cfg.get_bool(key) { "yes" } else { "no" };
    loop {
        let a = p.ask(&format!("{question} (yes/no)"), default);
        match cfg.set(key, &a) {
            Ok(_) => return,
            Err(_) => p.say("  Please answer yes or no."),
        }
    }
}

/// What the provider offers, or `None` if it could not be reached. Injected so tests need no network.
pub type Probe<'a> = &'a dyn Fn(&str) -> Option<Vec<String>>;
/// Stores a secret and returns the reference to write in the settings file.
pub type SaveSecret<'a> = &'a dyn Fn(&str, &str) -> Result<String, String>;

pub fn run(p: &mut dyn Prompter, cfg: &mut ConfigFile, probe: Probe, save_secret: SaveSecret) -> Result<(), String> {
    p.say("LastHarness setup. Press Enter to accept the answer in brackets. You can change any of this later with `lh settings`.\n");
    // 1. where the models are
    loop {
        let url = p.ask("Where are your models served? (an OpenAI-compatible address)", &cfg.get_opt("provider.url").unwrap_or_else(|| "http://localhost:9292/v1".into()));
        match probe(&url) {
            Some(models) => {
                p.say(&format!("  Reached it. It offers {} model(s){}.", models.len(), if models.is_empty() { String::new() } else { format!(", for example {}", models.iter().take(3).cloned().collect::<Vec<_>>().join(", ")) }));
                cfg.set("provider.url", &url).map_err(|e| e.to_string())?;
                break;
            }
            None => {
                let keep = p.ask("  Could not reach it. Use this address anyway? (yes/no)", "no");
                if keep.to_lowercase().starts_with('y') {
                    cfg.set("provider.url", &url).map_err(|e| e.to_string())?;
                    break;
                }
            }
        }
    }
    let needs_key = p.ask("Does this service need an API key? (yes/no)", "no");
    if needs_key.to_lowercase().starts_with('y') {
        let key = p.secret("API key:");
        if key.is_empty() {
            p.say("  No key entered; skipped.");
        } else {
            let r = save_secret("provider-key", &key)?;
            cfg.set("provider.key", &r).map_err(|e| e.to_string())?;
            p.say("  Stored in your system's secret store. It is not written to the settings file.");
        }
    }
    // 2. memory and learning
    yes_no(p, cfg, "memory.enabled", "Remember things between sessions?");
    p.say("  Learning: propose = suggestions wait for you, silent = applied and logged, off = none.");
    ask_valid(p, cfg, "learning.mode", "How should it learn from you?");
    // 3. how much you are asked
    p.say("  Autonomy on big goals: plan-first = approve the plan first, check-ins = also pause at milestones, fire-and-forget = no questions.");
    ask_valid(p, cfg, "team.autonomy", "How much should it ask you?");
    // 4. media, search
    let media = p.ask("Model for making images (leave empty for none)", &cfg.get("media.image_model"));
    cfg.set("media.image_model", &media).map_err(|e| e.to_string())?;
    let web = p.ask("SearXNG address for web search (leave empty for no search)", &cfg.get("web.searxng"));
    cfg.set("web.searxng", &web).map_err(|e| e.to_string())?;
    // 5. money
    p.say("  Spending: none = only record, caps = refuse past a limit, estimate = ask before an expensive call.");
    ask_valid(p, cfg, "spending.mode", "How should spending be handled?");
    if cfg.get("spending.mode") != "none" {
        ask_valid(p, cfg, "spending.daily_cap_usd", "Daily limit in US dollars (empty for none)");
    }
    // 6. safety
    yes_no(p, cfg, "permissions.sudo", "Let agents use sudo? It still asks each time");
    // 7. reports and updates
    p.say("  Anonymous crash reports would contain only: the app version, your operating system, and the place in the code where it stopped. Never your messages, notes, file names, models or keys.");
    ask_valid(p, cfg, "reports.crash", "Crash reports: off, ask (show it and ask each time) or auto?");
    yes_no(p, cfg, "updates.enabled", "Check for signed updates?");
    cfg.save().map_err(|e| e.to_string())?;
    p.say(&format!("\nSaved to {}.\nStart the core with `lhd`, then run `lh`.", cfg.path().display()));
    Ok(())
}

/// Look at the provider's model list. Blocking, only used by the setup command.
pub fn probe_http(url: &str) -> Option<Vec<String>> {
    let url = format!("{}/models", url.trim_end_matches('/'));
    let client = reqwest::blocking::Client::builder().timeout(std::time::Duration::from_secs(5)).build().ok()?;
    let v: serde_json::Value = client.get(url).send().ok()?.json().ok()?;
    Some(v["data"].as_array()?.iter().filter_map(|m| m["id"].as_str().map(str::to_string)).collect())
}

pub fn save_secret_os(name: &str, value: &str) -> Result<String, String> {
    use lh_secrets::SecretStore;
    lh_secrets::OsStore::new("lastharness").set(name, value).map_err(|e| format!("cannot store the key: {e}"))?;
    Ok(format!("secret:{name}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Script {
        answers: Vec<String>,
        said: Vec<String>,
        asked: Vec<String>,
    }
    impl Prompter for Script {
        fn say(&mut self, t: &str) {
            self.said.push(t.to_string());
        }
        fn ask(&mut self, q: &str, d: &str) -> String {
            self.asked.push(q.to_string());
            let a = if self.answers.is_empty() { String::new() } else { self.answers.remove(0) };
            if a.is_empty() { d.to_string() } else { a }
        }
        fn secret(&mut self, _: &str) -> String {
            self.answers.remove(0)
        }
    }

    fn go(answers: &[&str], reachable: bool) -> (ConfigFile, Script, Result<(), String>, tempfile::TempDir) {
        let d = tempfile::tempdir().unwrap();
        let mut cfg = ConfigFile::load(d.path().join("config.toml")).unwrap();
        let mut p = Script { answers: answers.iter().map(|s| s.to_string()).collect(), said: vec![], asked: vec![] };
        let r = run(&mut p, &mut cfg, &|_| reachable.then(|| vec!["m1".into(), "m2".into()]), &|n, v| {
            assert_eq!((n, v), ("provider-key", "sk-test"));
            Ok("secret:provider-key".into())
        });
        (cfg, p, r, d)
    }

    #[test]
    fn pressing_enter_all_the_way_gives_safe_defaults() {
        let (cfg, p, r, _d) = go(&[], true);
        r.unwrap();
        assert_eq!(cfg.get("provider.url"), "http://localhost:9292/v1");
        assert_eq!((cfg.get("memory.enabled").as_str(), cfg.get("learning.mode").as_str(), cfg.get("team.autonomy").as_str(), cfg.get("permissions.sudo").as_str(), cfg.get("reports.crash").as_str()), ("true", "propose", "plan-first", "false", "off"));
        assert!(cfg.exists(), "the file was written");
        assert!(p.said.iter().any(|s| s.contains("Reached it. It offers 2 model(s), for example m1, m2")));
    }

    #[test]
    fn answers_are_checked_and_asked_again_and_a_key_goes_to_the_secret_store() {
        // url, key? yes, key, memory no, learning "loud" (bad) then silent, autonomy, media, web, spending caps, daily cap, sudo yes, crash ask, updates no
        let (cfg, p, r, _d) = go(&["http://cloud.example/v1", "yes", "sk-test", "no", "loud", "silent", "fire-and-forget", "", "http://localhost:8080", "caps", "5", "yes", "ask", "no"], true);
        r.unwrap();
        assert_eq!(cfg.get("provider.key"), "secret:provider-key");
        assert_eq!((cfg.get("memory.enabled").as_str(), cfg.get("learning.mode").as_str(), cfg.get("team.autonomy").as_str()), ("false", "silent", "fire-and-forget"));
        assert_eq!((cfg.get("web.searxng").as_str(), cfg.get("spending.mode").as_str(), cfg.get("spending.daily_cap_usd").as_str(), cfg.get("permissions.sudo").as_str(), cfg.get("reports.crash").as_str(), cfg.get("updates.enabled").as_str()), ("http://localhost:8080", "caps", "5", "true", "ask", "false"));
        assert!(p.said.iter().any(|s| s.contains("must be one of: propose, silent, off")), "a wrong answer is explained: {:?}", p.said);
        let file = std::fs::read_to_string(cfg.path()).unwrap();
        assert!(!file.contains("sk-test"), "the key itself is never in the file");
    }

    #[test]
    fn an_unreachable_address_is_not_taken_unless_you_insist() {
        let (cfg, p, r, _d) = go(&["http://nowhere/v1", "no", "http://later/v1", "yes"], false);
        r.unwrap();
        assert_eq!(cfg.get("provider.url"), "http://later/v1");
        assert!(p.asked.iter().filter(|q| q.contains("Could not reach it")).count() == 2);
    }
}
