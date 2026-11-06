//! The model's tools for models and providers: `list_models`, `select_model`, `load_model`, `propose_provider`.
//! Their wording lives in the tool descriptions, not in the system prompt. The model can use only providers the
//! user has registered. A new provider is only a proposal until the user approves it, and a key is never part of
//! a tool call: the user types it into the app.

use std::sync::{Arc, OnceLock, Weak};

use lh_tools::{BoxFut, ExtraTool, Model, Risk, ToolError};
use serde_json::{json, Value};

use crate::registry::NewProvider;
use crate::session::Session;
use crate::AppState;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Kind {
    List,
    Select,
    Load,
    Propose,
}

impl Kind {
    pub fn enabled(live: &crate::Live) -> Vec<Kind> {
        let mut v = vec![];
        if live.list_models {
            v.push(Kind::List);
        }
        if live.select_model {
            v.push(Kind::Select);
            v.push(Kind::Load);
        }
        if live.propose_provider {
            v.push(Kind::Propose);
        }
        v
    }
}

pub(crate) struct ModelTool<M> {
    pub st: Weak<AppState<M>>,
    pub session: Arc<OnceLock<Weak<Session>>>,
    pub kind: Kind,
}

impl<M> std::fmt::Debug for ModelTool<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModelTool").field("kind", &self.kind).finish()
    }
}

impl<M: Model + 'static> ExtraTool for ModelTool<M> {
    fn def(&self) -> lh_provider::ToolDef {
        let (name, description, parameters) = match self.kind {
            Kind::List => ("list_models", "List the registered model providers and the models each offers.", json!({"type":"object","properties":{},"additionalProperties":false})),
            Kind::Select => ("select_model", "Use another model from a registered provider for this chat from the next message on. Name it provider:model.", json!({"type":"object","properties":{"model":{"type":"string","description":"provider:model, as list_models shows"}},"required":["model"]})),
            Kind::Load => ("load_model", "Ask a local provider (such as llama-swap) to load a model now. Cloud providers are refused. Unloading is not supported.", json!({"type":"object","properties":{"model":{"type":"string","description":"provider:model"}},"required":["model"]})),
            Kind::Propose => ("propose_provider", "Suggest a new provider by preset id or address. It is only a suggestion: the user decides and enters any key. Never send a key.", json!({"type":"object","properties":{"preset":{"type":"string"},"url":{"type":"string"},"reason":{"type":"string"}}})),
        };
        lh_provider::ToolDef { name: name.into(), description: description.into(), parameters }
    }

    fn risk(&self) -> Risk {
        match self.kind {
            Kind::List => Risk::ReadOnly,
            // allowed by decision, and written to the activity log; they change no files
            _ => Risk::ReadOnly,
        }
    }

    fn describe(&self, a: &Value) -> String {
        format!("{} {}", self.def().name, a["model"].as_str().or(a["preset"].as_str()).or(a["url"].as_str()).unwrap_or(""))
    }

    fn run(&self, a: Value) -> BoxFut<'_, Result<String, ToolError>> {
        Box::pin(async move {
            let st = self.st.upgrade().ok_or(ToolError::Failed("the core is shutting down".into()))?;
            let reg = crate::registry_api::registry(&st);
            let sid = self.session.get().and_then(|w| w.upgrade());
            let log = |kind: &str, d: Value| {
                let _ = st.settings.store.log(kind, "providers", "model", &d);
            };
            let arg = |k: &str| a[k].as_str().map(str::trim).filter(|v| !v.is_empty()).map(str::to_string);
            match self.kind {
                Kind::List => {
                    let default = reg.default_id();
                    let mut out = vec![];
                    for e in reg.list() {
                        let mut models = e.models.clone();
                        if let Some(c) = reg.client(&e.id) {
                            if let Ok(live) = tokio::time::timeout(std::time::Duration::from_secs(5), c.models()).await {
                                if let Ok(m) = live {
                                    reg.set_models(&e.id, m.clone());
                                    models = m;
                                }
                            }
                        }
                        out.push(json!({"provider": e.id, "name": e.name, "default": e.id == default, "local": e.local(), "models": models}));
                    }
                    Ok(serde_json::to_string(&out).unwrap())
                }
                Kind::Select | Kind::Load => {
                    let full = arg("model").ok_or(ToolError::Args("model is missing".into()))?;
                    let (pid, model) = reg.split(&full);
                    let pid = pid.unwrap_or_else(|| reg.default_id());
                    let e = reg.get(&pid).ok_or_else(|| ToolError::Failed(format!("no registered provider called {pid}; use list_models")))?;
                    if self.kind == Kind::Select {
                        let s = sid.ok_or(ToolError::Failed("no chat to switch".into()))?;
                        *s.model_switch.lock().unwrap() = Some(format!("{}:{}", e.id, model));
                        log("model_selected", json!({"provider": e.id, "model": model, "session_known": true}));
                        return Ok(format!("From the next message this chat uses {}:{}.", e.id, model));
                    }
                    if !e.local() {
                        return Err(ToolError::Failed(format!("{} is not a local provider; models are loaded only on local ones", e.name)));
                    }
                    // llama-swap loads a model when it is asked for one; a one-token request does that
                    let req = lh_provider::ChatRequest { model: model.clone(), messages: vec![lh_provider::Message::user("hi")], tools: vec![], max_tokens: Some(1), sampling: Default::default() };
                    let c = reg.client(&e.id).ok_or(ToolError::Failed("provider unavailable".into()))?;
                    match tokio::time::timeout(std::time::Duration::from_secs(300), c.complete(&req, &mut |_| {})).await {
                        Ok(Ok(_)) => {
                            log("model_loaded", json!({"provider": e.id, "model": model}));
                            Ok(format!("{model} is loaded on {}.", e.name))
                        }
                        Ok(Err(err)) => Err(ToolError::Failed(format!("could not load {model}: {err}"))),
                        Err(_) => Err(ToolError::Failed("loading took too long".into())),
                    }
                }
                Kind::Propose => {
                    let reason = arg("reason").unwrap_or_default();
                    let n = NewProvider { preset: arg("preset"), url: arg("url"), ..Default::default() };
                    if n.preset.is_none() && n.url.is_none() {
                        return Err(ToolError::Args("give a preset id or a url".into()));
                    }
                    let p = reg.propose(&n, &reason).map_err(ToolError::Failed)?;
                    log("provider_proposed", json!({"id": p.id, "url": p.url, "preset": p.preset, "reason": p.reason}));
                    let _ = st.events.send(json!({"type": "provider_proposal", "id": p.id, "name": p.name}));
                    Ok(format!("Proposed {} ({}). Nothing is registered until the user approves it and enters any key themselves.", p.name, p.url))
                }
            }
        })
    }
}
