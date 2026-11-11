//! Several registered model providers. The list lives in `providers.json` beside the settings file. It holds
//! addresses, names and a *reference* to each key (`secret:NAME`); the key itself is only in the secret store
//! and no call, log or file here ever carries it. One entry is the default. The old single-provider settings
//! (`provider.url`, `provider.api`, `provider.key`) become the first entry, `main`, when the file does not exist yet.
//! Models are named `provider:model`; a bare model name goes to the default provider.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use lh_provider::{ChatRequest, Provider};
use lh_secrets::SecretStore;
use lh_tools::Model;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Entry {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub preset: Option<String>,
    pub url: String,
    /// "openai", "anthropic" or "ollama".
    pub api: String,
    /// `secret:NAME`, never the key.
    #[serde(default)]
    pub key_ref: Option<String>,
    /// "local", "own" (a cloud under the user's account) or "any".
    pub trust: String,
    #[serde(default)]
    pub models: Vec<String>,
    /// Off: its models are not routed. Missing in old files: on.
    #[serde(default = "enabled_true")]
    pub enabled: bool,
    /// Models the person turned off. A model not listed here is on.
    #[serde(default)]
    pub disabled_models: Vec<String>,
    /// The model used when a role does not name one. Must be on and listed.
    #[serde(default)]
    pub default_model: Option<String>,
}

fn enabled_true() -> bool {
    true
}

impl Entry {
    pub fn local(&self) -> bool {
        self.trust == "local"
    }
    /// What is safe to show: everything but the key reference.
    pub fn public(&self, default: bool) -> Value {
        json!({"id": self.id, "name": self.name, "preset": self.preset, "url": self.url, "api": self.api, "trust": self.trust, "local": self.local(), "key_saved": self.key_ref.is_some(), "default": default, "models": self.models, "enabled": self.enabled, "disabled_models": self.disabled_models, "default_model": self.default_model})
    }
}

/// Refuse a call that the entry's settings turn off: the provider itself, or a model it has switched off.
/// An empty model name is a listing call and only checks the provider.
pub fn check_route(e: &Entry, model: &str) -> Result<(), String> {
    if !e.enabled {
        return Err(format!("{} is turned off. Turn it on in Providers.", e.name));
    }
    if !model.is_empty() && e.disabled_models.iter().any(|m| m == model) {
        return Err(format!("{model} is turned off for {}.", e.name));
    }
    Ok(())
}

/// Check that `model` may be the default: listed by the server and not turned off.
pub fn check_default_model(e: &Entry, model: &str) -> Result<(), String> {
    if e.models.is_empty() {
        return Err("Refresh the model list first, then pick a default.".into());
    }
    if !e.models.iter().any(|m| m == model) || e.disabled_models.iter().any(|m| m == model) {
        return Err("Default model must be one of the enabled models.".into());
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Proposal {
    pub id: String,
    pub name: String,
    pub preset: Option<String>,
    pub url: String,
    pub api: String,
    pub trust: String,
    pub reason: String,
}

#[derive(Default, Serialize, Deserialize)]
struct Data {
    default: String,
    entries: Vec<Entry>,
    #[serde(default)]
    proposals: Vec<Proposal>,
    #[serde(default)]
    next: u64,
}

pub struct Registry {
    path: Option<PathBuf>,
    data: Mutex<Data>,
    secrets: Arc<dyn SecretStore>,
    clients: Mutex<HashMap<String, Arc<Provider>>>,
}

/// What to do with the saved key when an entry is edited.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum KeyChange {
    #[default]
    Keep,
    Replace(String),
    Remove,
}

/// The fields an edit may change. `None` keeps the current value.
#[derive(Default)]
pub struct Update {
    pub name: Option<String>,
    pub url: Option<String>,
    pub api: Option<String>,
    pub trust: Option<String>,
    pub key: Option<KeyChange>,
    pub enabled: Option<bool>,
}

/// What adding a provider needs. A preset fills what is missing.
#[derive(Default, Deserialize)]
pub struct NewProvider {
    pub preset: Option<String>,
    pub url: Option<String>,
    pub name: Option<String>,
    pub api: Option<String>,
    pub trust: Option<String>,
}

fn slug(s: &str) -> String {
    let t: String = s.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    let t = t.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-");
    if t.is_empty() { "provider".into() } else { t }
}

pub fn api_name(k: lh_provider::ApiKind) -> &'static str {
    match k {
        lh_provider::ApiKind::Anthropic => "anthropic",
        lh_provider::ApiKind::Ollama => "ollama",
        lh_provider::ApiKind::OpenAiCompatible => "openai",
    }
}

/// Fill an entry from a preset and/or typed values. The result has no key.
fn resolve_new(n: &NewProvider) -> Result<(String, String, Option<String>, String, String), String> {
    let p = match &n.preset {
        Some(id) => Some(lh_provider::find_preset(id).filter(|p| p.supported).ok_or_else(|| format!("No supported provider preset called {id}."))?),
        None => None,
    };
    let url = n.url.clone().or_else(|| p.map(|p| p.base_url.to_string())).unwrap_or_default();
    if let Some(m) = crate::provider_api::check_url(&url) {
        return Err(m);
    }
    let url = url.trim().to_string();
    let name = n.name.clone().filter(|s| !s.trim().is_empty()).or_else(|| p.map(|p| p.name.to_string())).unwrap_or_else(|| url.clone());
    let api = n.api.clone().or_else(|| p.map(|p| api_name(p.api).to_string())).unwrap_or_else(|| "openai".into());
    if !["openai", "anthropic", "ollama"].contains(&api.as_str()) {
        return Err(format!("api must be openai, anthropic or ollama, not {api}"));
    }
    let local = lh_provider::is_local_url(&url);
    let trust = n.trust.clone().unwrap_or_else(|| if local { "local".into() } else { "any".into() });
    if !["local", "own", "any"].contains(&trust.as_str()) {
        return Err("trust must be local, own or any".into());
    }
    Ok((name, url, n.preset.clone(), api, trust))
}

impl Registry {
    /// Open the registry kept at `path` (`None`: in memory only). When there is no file yet, `legacy`
    /// (the old single provider) becomes the first entry.
    pub fn open(path: Option<PathBuf>, legacy: Option<Entry>, secrets: Arc<dyn SecretStore>) -> Self {
        let mut data: Data = path.as_ref().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        if data.entries.is_empty() {
            if let Some(e) = legacy {
                data.default = e.id.clone();
                data.entries.push(e);
            }
        }
        let r = Self { path, data: Mutex::new(data), secrets, clients: Default::default() };
        r.save();
        r
    }

    /// Open from the settings file: the registry file beside it, migrating `provider.*` when needed.
    pub fn from_config(config: &Path, secrets: Arc<dyn SecretStore>) -> Self {
        let cfg = lh_config::ConfigFile::load(config).ok();
        let legacy = cfg.as_ref().and_then(|c| {
            let url = c.get_opt("provider.url")?;
            let api = c.get("provider.api");
            Some(Entry { id: "main".into(), name: c.get_opt("provider.name").unwrap_or_else(|| "Main".into()), preset: None, trust: if lh_provider::is_local_url(&url) { "local".into() } else { "any".into() }, url, api, key_ref: c.get_opt("provider.key"), models: vec![], enabled: true, disabled_models: vec![], default_model: None })
        });
        Self::open(Some(config.with_file_name("providers.json")), legacy, secrets)
    }

    fn save(&self) {
        if let Some(p) = &self.path {
            let d = self.data.lock().unwrap();
            if let Ok(t) = serde_json::to_string_pretty(&*d) {
                let _ = std::fs::write(p, t);
            }
        }
    }

    pub fn list(&self) -> Vec<Entry> {
        self.data.lock().unwrap().entries.clone()
    }
    pub fn default_id(&self) -> String {
        self.data.lock().unwrap().default.clone()
    }
    pub fn get(&self, id: &str) -> Option<Entry> {
        self.data.lock().unwrap().entries.iter().find(|e| e.id == id).cloned()
    }
    pub fn is_empty(&self) -> bool {
        self.data.lock().unwrap().entries.is_empty()
    }

    fn key_of(&self, e: &Entry) -> Option<String> {
        e.key_ref.as_deref().and_then(|r| lh_secrets::resolve(&*self.secrets, r).ok().flatten())
    }

    /// A client for the entry; the key is read from the secret store when the client is built.
    pub fn client(&self, id: &str) -> Option<Arc<Provider>> {
        if let Some(c) = self.clients.lock().unwrap().get(id) {
            return Some(c.clone());
        }
        let e = self.get(id)?;
        let key = self.key_of(&e);
        let p = Arc::new(match e.api.as_str() {
            "anthropic" => Provider::anthropic(&e.url, key),
            "ollama" => Provider::ollama(&e.url, key),
            _ => Provider::new(&e.url, key),
        });
        self.clients.lock().unwrap().insert(id.to_string(), p.clone());
        Some(p)
    }

    fn unique_id(d: &Data, name: &str) -> String {
        let base = slug(name);
        let mut id = base.clone();
        let mut n = 1;
        while d.entries.iter().any(|e| e.id == id) {
            n += 1;
            id = format!("{base}-{n}");
        }
        id
    }

    /// Register a provider. `key` goes to the secret store; the entry keeps a reference.
    pub fn add(&self, n: &NewProvider, key: Option<&str>) -> Result<Entry, String> {
        let (name, url, preset, api, trust) = resolve_new(n)?;
        let key = key.map(str::trim).filter(|k| !k.is_empty());
        if let Some(p) = n.preset.as_deref().and_then(lh_provider::find_preset) {
            if p.key_required && key.is_none() {
                return Err(format!("{} needs a key.", p.name));
            }
        }
        let e = {
            let mut d = self.data.lock().unwrap();
            let id = Self::unique_id(&d, &name);
            let key_ref = match key {
                Some(k) => {
                    let secret = format!("provider-{id}");
                    self.secrets.set(&secret, k).map_err(|e| e.to_string())?;
                    Some(format!("secret:{secret}"))
                }
                None => None,
            };
            let e = Entry { id, name, preset, url, api, key_ref, trust, models: vec![], enabled: true, disabled_models: vec![], default_model: None };
            if d.entries.is_empty() {
                d.default = e.id.clone();
            }
            d.entries.push(e.clone());
            e
        };
        self.save();
        Ok(e)
    }

    /// The saved key of an entry, read from the secret store. Never sent to the GUI.
    pub fn key_for(&self, id: &str) -> Option<String> {
        self.get(id).and_then(|e| self.key_of(&e))
    }

    /// Change an entry. Changing the address or kind clears its model list. A cloud preset cannot lose its key.
    pub fn update(&self, id: &str, u: &Update) -> Result<Entry, String> {
        let e = {
            let mut d = self.data.lock().unwrap();
            let i = d.entries.iter().position(|e| e.id == id).ok_or("No such provider.")?;
            let cur = d.entries[i].clone();
            let url = match &u.url {
                Some(v) => {
                    if let Some(m) = crate::provider_api::check_url(v) {
                        return Err(m);
                    }
                    v.trim().to_string()
                }
                None => cur.url.clone(),
            };
            let api = u.api.clone().unwrap_or_else(|| cur.api.clone());
            if !["openai", "anthropic", "ollama"].contains(&api.as_str()) {
                return Err(format!("api must be openai, anthropic or ollama, not {api}"));
            }
            let trust = u.trust.clone().unwrap_or_else(|| cur.trust.clone());
            if !["local", "own", "any"].contains(&trust.as_str()) {
                return Err("trust must be local, own or any".into());
            }
            let name = match &u.name {
                Some(v) if !v.trim().is_empty() => v.trim().to_string(),
                _ => cur.name.clone(),
            };
            let needs_key = cur.preset.as_deref().and_then(lh_provider::find_preset).map(|p| p.key_required).unwrap_or(false);
            let secret = cur.key_ref.as_deref().and_then(|r| r.strip_prefix("secret:")).map(str::to_string).unwrap_or_else(|| format!("provider-{id}"));
            let mut key_ref = cur.key_ref.clone();
            match u.key.clone().unwrap_or(KeyChange::Keep) {
                KeyChange::Keep => {}
                KeyChange::Replace(k) => {
                    let k = k.trim().to_string();
                    if k.is_empty() {
                        return Err("Type a key, or keep the saved one.".into());
                    }
                    self.secrets.set(&secret, &k).map_err(|e| e.to_string())?;
                    key_ref = Some(format!("secret:{secret}"));
                }
                KeyChange::Remove => {
                    if needs_key {
                        return Err(format!("{} needs a key.", cur.name));
                    }
                    if secret.starts_with("provider-") && secret != "provider-key" {
                        let _ = self.secrets.delete(&secret);
                    }
                    key_ref = None;
                }
            }
            let moved = url != cur.url || api != cur.api;
            let enabled = u.enabled.unwrap_or(cur.enabled);
            if !enabled && d.default == id {
                return Err("This is the default provider; make another one the default first.".into());
            }
            // a moved address has other models, so the model settings start again
            let (models, disabled_models, default_model) = if moved { (vec![], vec![], None) } else { (cur.models.clone(), cur.disabled_models.clone(), cur.default_model.clone()) };
            let e = Entry { id: cur.id.clone(), name, preset: cur.preset.clone(), url, api, key_ref, trust, models, enabled, disabled_models, default_model };
            d.entries[i] = e.clone();
            e
        };
        self.clients.lock().unwrap().remove(id);
        self.save();
        Ok(e)
    }

    pub fn remove(&self, id: &str) -> Result<(), String> {
        let e = {
            let mut d = self.data.lock().unwrap();
            if d.default == id && d.entries.len() > 1 {
                return Err("This is the default provider; make another one the default first.".into());
            }
            let i = d.entries.iter().position(|e| e.id == id).ok_or("No such provider.")?;
            let e = d.entries.remove(i);
            if d.entries.is_empty() {
                d.default.clear();
            }
            e
        };
        if let Some(r) = e.key_ref.as_deref().and_then(|r| r.strip_prefix("secret:")) {
            if r.starts_with("provider-") && r != "provider-key" {
                let _ = self.secrets.delete(r);
            }
        }
        self.clients.lock().unwrap().remove(id);
        self.save();
        Ok(())
    }

    pub fn set_default(&self, id: &str) -> Result<(), String> {
        {
            let mut d = self.data.lock().unwrap();
            if !d.entries.iter().any(|e| e.id == id) {
                return Err("No such provider.".into());
            }
            d.default = id.to_string();
        }
        self.save();
        Ok(())
    }

    pub fn set_models(&self, id: &str, models: Vec<String>) {
        if let Some(e) = self.data.lock().unwrap().entries.iter_mut().find(|e| e.id == id) {
            // a default that the server no longer lists is dropped
            if e.default_model.as_ref().is_some_and(|m| !models.contains(m)) {
                e.default_model = None;
            }
            e.models = models;
        }
        self.save();
    }

    /// Turn one model on or off for a provider. Turning off the default model is refused.
    pub fn set_model_enabled(&self, id: &str, model: &str, on: bool) -> Result<Entry, String> {
        let e = {
            let mut d = self.data.lock().unwrap();
            let e = d.entries.iter_mut().find(|e| e.id == id).ok_or("No such provider.")?;
            if model.trim().is_empty() {
                return Err("Name the model to turn on or off.".into());
            }
            if !on && e.default_model.as_deref() == Some(model) {
                return Err("This is the default model; make another model the default first.".into());
            }
            e.disabled_models.retain(|m| m != model);
            if !on {
                e.disabled_models.push(model.to_string());
            }
            e.clone()
        };
        self.save();
        Ok(e)
    }

    /// Set (or clear, with `None`) the provider's default model. It must be listed and on.
    pub fn set_default_model(&self, id: &str, model: Option<&str>) -> Result<Entry, String> {
        let e = {
            let mut d = self.data.lock().unwrap();
            let e = d.entries.iter_mut().find(|e| e.id == id).ok_or("No such provider.")?;
            match model {
                Some(m) => {
                    check_default_model(e, m)?;
                    e.default_model = Some(m.to_string());
                }
                None => e.default_model = None,
            }
            e.clone()
        };
        self.save();
        Ok(e)
    }

    /// The old single-provider settings (or the address the core was started with) changed: keep the `main`
    /// entry in step, creating it when there is none.
    pub fn sync_main(&self, url: &str, api: &str, name: &str, key_ref: Option<String>) {
        if url.is_empty() {
            return;
        }
        {
            let mut d = self.data.lock().unwrap();
            let trust = if lh_provider::is_local_url(url) { "local" } else { "any" }.to_string();
            match d.entries.iter_mut().find(|e| e.id == "main") {
                Some(e) => {
                    (e.url, e.trust, e.api, e.key_ref) = (url.to_string(), trust, api.to_string(), key_ref);
                    if !name.is_empty() {
                        e.name = name.to_string();
                    }
                }
                None => {
                    d.entries.push(Entry { id: "main".into(), name: if name.is_empty() { "Main".into() } else { name.into() }, preset: None, url: url.into(), api: api.into(), key_ref, trust, models: vec![], enabled: true, disabled_models: vec![], default_model: None });
                    if d.default.is_empty() {
                        d.default = "main".into();
                    }
                }
            }
        }
        self.clients.lock().unwrap().remove("main");
        self.save();
    }

    /// A proposal waits here until the user approves it. It never holds a key.
    pub fn propose(&self, n: &NewProvider, reason: &str) -> Result<Proposal, String> {
        let (name, url, preset, api, trust) = resolve_new(n)?;
        let p = {
            let mut d = self.data.lock().unwrap();
            if d.entries.iter().any(|e| e.url == url) {
                return Err("A provider with that address is already registered.".into());
            }
            d.next += 1;
            let p = Proposal { id: format!("p{}", d.next), name, preset, url, api, trust, reason: reason.chars().take(300).collect() };
            d.proposals.push(p.clone());
            p
        };
        self.save();
        Ok(p)
    }
    pub fn proposals(&self) -> Vec<Proposal> {
        self.data.lock().unwrap().proposals.clone()
    }
    pub fn reject(&self, pid: &str) -> bool {
        let found = {
            let mut d = self.data.lock().unwrap();
            let n = d.proposals.len();
            d.proposals.retain(|p| p.id != pid);
            d.proposals.len() != n
        };
        self.save();
        found
    }
    /// The user approved: register it. The key, if any, is typed by the user into this call.
    pub fn approve(&self, pid: &str, key: Option<&str>) -> Result<Entry, String> {
        let p = self.proposals().into_iter().find(|p| p.id == pid).ok_or("No such proposal.")?;
        let e = self.add(&NewProvider { preset: p.preset.clone(), url: Some(p.url.clone()), name: Some(p.name.clone()), api: Some(p.api.clone()), trust: Some(p.trust.clone()) }, key)?;
        self.reject(pid);
        Ok(e)
    }

    /// `provider:model` to (entry id, model). A prefix that is not a registered id is part of the model name.
    pub fn split(&self, full: &str) -> (Option<String>, String) {
        if let Some((a, b)) = full.split_once(':') {
            if self.get(a).is_some() {
                return (Some(a.to_string()), b.to_string());
            }
        }
        (None, full.to_string())
    }
}

/// The model the core talks through: routes `provider:model` to the registered provider, everything else
/// to the default (the registry's, or the one the core was started with).
pub struct Fleet {
    pub base: Provider,
    pub reg: Arc<Registry>,
}

impl Fleet {
    pub fn new(base: Provider, reg: Arc<Registry>) -> Self {
        Self { base, reg }
    }
    fn pick(&self, full: &str) -> (Option<Arc<Provider>>, String) {
        let (id, model) = self.reg.split(full);
        let id = id.or_else(|| Some(self.reg.default_id()).filter(|d| !d.is_empty()));
        (id.and_then(|i| self.reg.client(&i)), model)
    }
    /// Like `pick`, but refuses a provider or model that the settings turn off.
    fn route(&self, full: &str) -> Result<(Option<Arc<Provider>>, String), lh_provider::Error> {
        let (id, model) = self.reg.split(full);
        let id = id.or_else(|| Some(self.reg.default_id()).filter(|d| !d.is_empty()));
        if let Some(e) = id.as_deref().and_then(|i| self.reg.get(i)) {
            check_route(&e, &model).map_err(lh_provider::Error::Refused)?;
        }
        Ok((id.and_then(|i| self.reg.client(&i)), model))
    }
}

impl Model for Fleet {
    async fn complete(&self, req: &ChatRequest, on_event: &mut (dyn FnMut(lh_provider::Event) + Send)) -> Result<lh_provider::Response, lh_provider::Error> {
        let (p, model) = self.route(&req.model)?;
        let mut r = req.clone();
        r.model = model;
        Model::complete(p.as_deref().unwrap_or(&self.base), &r, on_event).await
    }
    async fn models(&self) -> Result<Vec<String>, lh_provider::Error> {
        let (p, _) = self.pick("");
        Model::models(p.as_deref().unwrap_or(&self.base)).await
    }
    async fn model_details(&self) -> Result<Vec<Value>, lh_provider::Error> {
        let (p, _) = self.pick("");
        Model::model_details(p.as_deref().unwrap_or(&self.base)).await
    }
    async fn capabilities(&self, model: &str) -> lh_provider::Capabilities {
        let (p, m) = self.pick(model);
        Model::capabilities(p.as_deref().unwrap_or(&self.base), &m).await
    }
    async fn media_support(&self, model: &str) -> lh_provider::Support {
        let (p, m) = self.pick(model);
        Model::media_support(p.as_deref().unwrap_or(&self.base), &m).await
    }
    async fn provider_name(&self) -> Option<String> {
        let (p, _) = self.pick("");
        Model::provider_name(p.as_deref().unwrap_or(&self.base)).await
    }
    async fn generate_images(&self, model: &str, prompt: &str, size: Option<&str>, n: u32) -> Result<Vec<lh_provider::GeneratedImage>, lh_provider::Error> {
        let (p, m) = self.route(model)?;
        Model::generate_images(p.as_deref().unwrap_or(&self.base), &m, prompt, size, n).await
    }
    fn release_idle(&self) {
        self.base.release_connections();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lh_secrets::MemoryStore;

    fn reg() -> (Registry, Arc<MemoryStore>) {
        let s = Arc::new(MemoryStore::default());
        (Registry::open(None, None, s.clone()), s)
    }

    #[test]
    fn first_is_default_keys_go_to_the_secret_store_only() {
        let (r, s) = reg();
        let a = r.add(&NewProvider { url: Some("http://localhost:9292/v1".into()), name: Some("Local".into()), ..Default::default() }, None).unwrap();
        let b = r.add(&NewProvider { url: Some("https://api.example.com/v1".into()), name: Some("Cloud".into()), ..Default::default() }, Some("sk-topsecret")).unwrap();
        assert_eq!((r.default_id(), a.trust.as_str(), b.trust.as_str()), ("local".into(), "local", "any"));
        assert_eq!(s.get("provider-cloud").unwrap().as_deref(), Some("sk-topsecret"));
        let shown = serde_json::to_string(&b.public(false)).unwrap();
        assert!(!shown.contains("sk-topsecret") && !shown.contains("secret:"));
        assert!(r.remove("local").is_err());
        r.set_default("cloud").unwrap();
        r.remove("local").unwrap();
    }

    #[test]
    fn provider_prefix_routing_and_proposals() {
        let (r, _) = reg();
        r.add(&NewProvider { url: Some("http://localhost:1/v1".into()), name: Some("Box".into()), ..Default::default() }, None).unwrap();
        assert_eq!(r.split("box:llama3:8b"), (Some("box".into()), "llama3:8b".into()));
        assert_eq!(r.split("llama3:8b"), (None, "llama3:8b".into()));
        let p = r.propose(&NewProvider { preset: Some("openrouter".into()), ..Default::default() }, "cheap").unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(r.list().len(), 1);
        let e = r.approve(&p.id, Some("k")).unwrap();
        assert_eq!(r.list().len(), 2);
        assert!(r.proposals().is_empty() && e.key_ref.is_some());
        assert!(r.propose(&NewProvider { url: Some("http://localhost:1/v1".into()), ..Default::default() }, "").is_err());
    }

    #[test]
    fn cloud_preset_without_key_is_refused_and_local_is_not() {
        let (r, _) = reg();
        let e = r.add(&NewProvider { preset: Some("openrouter".into()), ..Default::default() }, None).unwrap_err();
        assert_eq!(e, "OpenRouter needs a key.");
        assert!(r.add(&NewProvider { preset: Some("openrouter".into()), ..Default::default() }, Some("   ")).is_err());
        assert!(r.is_empty());
        let l = r.add(&NewProvider { preset: Some("llama-swap".into()), ..Default::default() }, None).unwrap();
        assert_eq!(l.key_ref, None);
        assert!(r.add(&NewProvider { preset: Some("openrouter".into()), ..Default::default() }, Some("sk-1")).is_ok());
    }

    #[test]
    fn update_replaces_and_removes_the_key_and_clears_models_on_move() {
        let (r, s) = reg();
        r.add(&NewProvider { url: Some("https://api.example.com/v1".into()), name: Some("Cloud".into()), ..Default::default() }, Some("sk-one")).unwrap();
        r.set_models("cloud", vec!["a".into()]);
        // rename only: key and models kept
        let e = r.update("cloud", &Update { name: Some("Cloud 2".into()), ..Default::default() }).unwrap();
        assert_eq!((e.name.as_str(), e.models.len(), r.key_for("cloud").as_deref()), ("Cloud 2", 1, Some("sk-one")));
        // replace key
        r.update("cloud", &Update { key: Some(KeyChange::Replace(" sk-two ".into())), ..Default::default() }).unwrap();
        assert_eq!(s.get("provider-cloud").unwrap().as_deref(), Some("sk-two"));
        // move address: models cleared
        let e = r.update("cloud", &Update { url: Some("https://other.example.com/v1".into()), ..Default::default() }).unwrap();
        assert!(e.models.is_empty());
        // remove key
        let e = r.update("cloud", &Update { key: Some(KeyChange::Remove), ..Default::default() }).unwrap();
        assert_eq!((e.key_ref, s.get("provider-cloud").unwrap()), (None, None));
        // bad values refused, nothing changed
        assert!(r.update("cloud", &Update { url: Some("ftp://x".into()), ..Default::default() }).is_err());
        assert!(r.update("cloud", &Update { api: Some("nope".into()), ..Default::default() }).is_err());
        assert!(r.update("missing", &Update::default()).is_err());
        assert_eq!(r.get("cloud").unwrap().url, "https://other.example.com/v1");
    }

    #[test]
    fn cloud_preset_cannot_lose_its_key_and_per_entry_key_is_read() {
        let (r, s) = reg();
        r.add(&NewProvider { preset: Some("openrouter".into()), ..Default::default() }, Some("sk-or")).unwrap();
        assert!(r.update("openrouter", &Update { key: Some(KeyChange::Remove), ..Default::default() }).is_err());
        assert_eq!(r.key_for("openrouter").as_deref(), Some("sk-or"));
        assert_eq!(r.key_for("missing"), None);
        assert_eq!(s.get("provider-openrouter").unwrap().as_deref(), Some("sk-or"));
    }

    #[test]
    fn old_single_provider_becomes_main() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("config.toml");
        let mut c = lh_config::ConfigFile::load(&cfg).unwrap();
        c.set("provider.url", "http://localhost:9292/v1").unwrap();
        c.set("provider.key", "secret:provider-key").unwrap();
        c.save().unwrap();
        let r = Registry::from_config(&cfg, Arc::new(MemoryStore::default()));
        assert_eq!((r.default_id(), r.list()[0].url.clone()), ("main".into(), "http://localhost:9292/v1".into()));
        assert!(dir.path().join("providers.json").exists());
    }

    #[test]
    fn legacy_entries_load_enabled_with_no_model_settings() {
        let old = r#"{"id":"cloud","name":"Cloud","url":"https://x/v1","api":"openai","trust":"any","models":["a"]}"#;
        let e: Entry = serde_json::from_str(old).unwrap();
        assert!(e.enabled);
        assert!(e.disabled_models.is_empty());
        assert_eq!(e.default_model, None);
    }

    #[test]
    fn disabled_provider_and_model_are_not_routed() {
        let mut e: Entry = serde_json::from_str(r#"{"id":"c","name":"C","url":"u","api":"openai","trust":"any"}"#).unwrap();
        assert!(check_route(&e, "m").is_ok());
        e.disabled_models.push("m".into());
        assert!(check_route(&e, "m").unwrap_err().contains("turned off for C"));
        assert!(check_route(&e, "other").is_ok());
        e.enabled = false;
        assert!(check_route(&e, "other").unwrap_err().contains("turned off"));
    }

    #[test]
    fn default_model_must_be_listed_and_enabled() {
        let mut e: Entry = serde_json::from_str(r#"{"id":"c","name":"C","url":"u","api":"openai","trust":"any"}"#).unwrap();
        assert!(check_default_model(&e, "a").unwrap_err().contains("Refresh"));
        e.models = vec!["a".into(), "b".into()];
        e.disabled_models = vec!["b".into()];
        assert!(check_default_model(&e, "a").is_ok());
        assert_eq!(check_default_model(&e, "b").unwrap_err(), "Default model must be one of the enabled models.");
        assert!(check_default_model(&e, "zz").is_err());
    }

    #[test]
    fn model_toggle_and_default_model_persist_and_validate() {
        let (r, _s) = reg();
        r.add(&NewProvider { url: Some("http://h:1/v1".into()), name: Some("H".into()), ..Default::default() }, None).unwrap();
        let id = r.list()[0].id.clone();
        r.set_models(&id, vec!["a".into(), "b".into()]);
        assert!(r.set_default_model(&id, Some("zz")).is_err());
        r.set_default_model(&id, Some("a")).unwrap();
        // turning off the default model is refused
        assert!(r.set_model_enabled(&id, "a", false).is_err());
        let e = r.set_model_enabled(&id, "b", false).unwrap();
        assert_eq!(e.disabled_models, vec!["b".to_string()]);
        assert!(r.set_default_model(&id, Some("b")).is_err());
        // a model the server stops listing drops the default
        r.set_models(&id, vec!["b".into()]);
        assert_eq!(r.get(&id).unwrap().default_model, None);
        let back = r.set_model_enabled(&id, "b", true).unwrap();
        assert!(back.disabled_models.is_empty());
    }

    #[test]
    fn default_provider_cannot_be_turned_off() {
        let (r, _s) = reg();
        r.add(&NewProvider { url: Some("http://h:1/v1".into()), name: Some("H".into()), ..Default::default() }, None).unwrap();
        let id = r.list()[0].id.clone();
        assert!(r.update(&id, &Update { enabled: Some(false), ..Default::default() }).is_err());
        assert!(r.get(&id).unwrap().enabled);
    }
}
