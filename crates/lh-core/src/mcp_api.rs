//! MCP servers and folder trust over the local API.
//!
//! A server is added in two steps: the user (or an agent on their behalf) names it, and the exact command
//! or link is shown; nothing runs until the user approves. Approved servers are started on first use and
//! kept for the session of the core. A project folder may ship its own servers in `.lastharness/mcp.json`;
//! they load only after the user trusts that folder, and "use my own" leaves them off.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path as FsPath, PathBuf};
use std::sync::{Arc, Mutex};

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use lh_mcp::{Connection, Transport};
use lh_tools::{ExtraTool, Model};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{ApiError, AppState};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Entry {
    name: String,
    transport: Transport,
    approved: bool,
}

#[derive(Default, Serialize, Deserialize)]
struct Saved {
    servers: Vec<Entry>,
    /// Folder to the user's decision: `trust` or `own` (use my own servers and skills instead).
    trust: BTreeMap<String, String>,
    /// What the folder's server list looked like when it was trusted. A changed list is not trusted.
    #[serde(default)]
    trust_hash: BTreeMap<String, String>,
}

pub struct McpRuntime {
    path: PathBuf,
    saved: Mutex<Saved>,
    conns: tokio::sync::Mutex<HashMap<String, Arc<Connection>>>,
}

fn folder_file(folder: &str) -> PathBuf {
    FsPath::new(folder).join(".lastharness").join("mcp.json")
}

/// Fingerprint of the folder's server list, so a list that changes after trust (a `git pull` that adds a
/// server) has to be looked at again.
fn folder_hash(folder: &str) -> String {
    use sha2::{Digest, Sha256};
    std::fs::read(folder_file(folder)).map(|b| Sha256::digest(b).iter().map(|x| format!("{x:02x}")).collect()).unwrap_or_default()
}

/// The servers a folder asks for. Names and transports only; whether they run is up to the user.
fn folder_servers(folder: &str) -> Vec<(String, Transport)> {
    let Ok(text) = std::fs::read_to_string(folder_file(folder)) else { return vec![] };
    let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    v["servers"].as_array().into_iter().flatten().filter_map(|s| Some((s["name"].as_str()?.to_string(), serde_json::from_value::<Transport>(s["transport"].clone()).ok()?))).collect()
}

impl McpRuntime {
    pub fn open(path: PathBuf) -> Self {
        let saved = std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        Self { path, saved: Mutex::new(saved), conns: Default::default() }
    }

    fn save(&self) -> Result<(), ApiError> {
        let text = serde_json::to_string_pretty(&*self.saved.lock().unwrap()).unwrap_or_default();
        if let Some(d) = self.path.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        std::fs::write(&self.path, text).map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, format!("cannot save {}: {e}", self.path.display())))
    }

    /// `trust`, `own`, `changed` (trusted once, but the list is different now) or nothing yet.
    pub fn decision(&self, folder: &str) -> Option<String> {
        let s = self.saved.lock().unwrap();
        let d = s.trust.get(folder).cloned()?;
        if d == "trust" && s.trust_hash.get(folder).map(String::as_str) != Some(folder_hash(folder).as_str()) {
            return Some("changed".into());
        }
        Some(d)
    }

    /// What the folder wants that the user has not yet decided about.
    pub fn pending_trust(&self, folder: &str) -> Option<Value> {
        let servers = folder_servers(folder);
        (matches!(self.decision(folder).as_deref(), None | Some("changed")) && !servers.is_empty()).then(|| json!({"folder": folder, "mcp": servers.iter().map(|(n, t)| json!({"name": n, "runs": t.display()})).collect::<Vec<_>>()}))
    }

    async fn connection(&self, name: &str, t: &Transport) -> Result<Arc<Connection>, lh_mcp::Error> {
        if let Some(c) = self.conns.lock().await.get(name) {
            return Ok(c.clone());
        }
        let c = Connection::connect(name, t.clone()).await?;
        self.conns.lock().await.insert(name.to_string(), c.clone());
        Ok(c)
    }

    /// Tools of every approved server, plus those of a trusted folder. A server that will not start is
    /// skipped; `list` shows why.
    pub async fn tools_for(&self, folder: Option<&str>) -> Vec<Arc<dyn ExtraTool>> {
        let mut want: Vec<(String, Transport)> = self.saved.lock().unwrap().servers.iter().filter(|e| e.approved).map(|e| (e.name.clone(), e.transport.clone())).collect();
        if let Some(f) = folder.filter(|f| self.decision(f).as_deref() == Some("trust")) {
            want.extend(folder_servers(f).into_iter().map(|(n, t)| (format!("{}-{n}", FsPath::new(f).file_name().and_then(|x| x.to_str()).unwrap_or("folder")), t)));
        }
        let mut out = vec![];
        for (name, t) in want {
            if let Ok(c) = self.connection(&name, &t).await {
                out.extend(lh_mcp::tools(&c));
            }
        }
        out
    }
}

pub(crate) fn routes<M: Model + 'static>() -> Router<Arc<AppState<M>>> {
    Router::new()
        .route("/v1/mcp/servers", get(list::<M>).post(add::<M>))
        .route("/v1/mcp/servers/{name}", axum::routing::delete(remove::<M>))
        .route("/v1/mcp/servers/{name}/approve", post(approve::<M>))
        .route("/v1/trust", get(get_trust::<M>).post(set_trust::<M>))
}

fn rt<M>(st: &AppState<M>) -> Result<&McpRuntime, ApiError> {
    st.settings.mcp.as_ref().ok_or(ApiError(StatusCode::NOT_FOUND, "MCP is switched off".into()))
}

async fn list<M: Model + 'static>(State(st): State<Arc<AppState<M>>>) -> Result<Json<Value>, ApiError> {
    let r = rt(&st)?;
    let entries = r.saved.lock().unwrap().servers.clone();
    let mut out = vec![];
    for e in entries {
        let mut j = json!({"name": e.name, "kind": if e.transport.is_remote() { "remote" } else { "local" }, "runs": e.transport.display(), "approved": e.approved});
        if e.approved {
            match r.connection(&e.name, &e.transport).await {
                Ok(c) => j["tools"] = json!(c.tools.iter().map(|t| json!({"name": lh_mcp::qualified_name(&e.name, &t.name), "description": t.description})).collect::<Vec<_>>()),
                Err(err) => j["error"] = json!(err.to_string()),
            }
        }
        out.push(j);
    }
    Ok(Json(json!({"servers": out})))
}

#[derive(Deserialize)]
struct NewServer {
    name: String,
    transport: Transport,
}

async fn add<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Json(b): Json<NewServer>) -> Result<(StatusCode, Json<Value>), ApiError> {
    let r = rt(&st)?;
    if b.name.is_empty() || !b.name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(ApiError(StatusCode::BAD_REQUEST, "the name may use letters, digits, - and _".into()));
    }
    {
        let mut s = r.saved.lock().unwrap();
        if s.servers.iter().any(|e| e.name == b.name) {
            return Err(ApiError(StatusCode::CONFLICT, format!("there is already a server called {}", b.name)));
        }
        s.servers.push(Entry { name: b.name.clone(), transport: b.transport.clone(), approved: false });
    }
    r.save()?;
    let _ = st.settings.store.log("mcp_server", &b.name, "user", &json!({"op": "added, waiting for approval", "runs": b.transport.display()}));
    Ok((StatusCode::CREATED, Json(json!({"name": b.name, "approved": false, "runs": b.transport.display(), "note": "nothing runs until you approve it"}))))
}

async fn approve<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(name): Path<String>) -> Result<Json<Value>, ApiError> {
    let r = rt(&st)?;
    let t = r.saved.lock().unwrap().servers.iter().find(|e| e.name == name).map(|e| e.transport.clone()).ok_or(ApiError(StatusCode::NOT_FOUND, "no such server".into()))?;
    let c = r.connection(&name, &t).await.map_err(|e| ApiError(StatusCode::BAD_GATEWAY, e.to_string()))?;
    if let Some(e) = r.saved.lock().unwrap().servers.iter_mut().find(|e| e.name == name) {
        e.approved = true;
    }
    r.save()?;
    let _ = st.settings.store.log("mcp_server", &name, "user", &json!({"op": "approved", "runs": t.display(), "tools": c.tools.len()}));
    Ok(Json(json!({"name": name, "approved": true, "tools": c.tools.iter().map(|t| t.name.clone()).collect::<Vec<_>>()})))
}

async fn remove<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Path(name): Path<String>) -> Result<StatusCode, ApiError> {
    let r = rt(&st)?;
    let before = r.saved.lock().unwrap().servers.len();
    r.saved.lock().unwrap().servers.retain(|e| e.name != name);
    if r.saved.lock().unwrap().servers.len() == before {
        return Err(ApiError(StatusCode::NOT_FOUND, "no such server".into()));
    }
    r.conns.lock().await.remove(&name);
    r.save()?;
    let _ = st.settings.store.log("mcp_server", &name, "user", &json!({"op": "removed"}));
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct FolderQuery {
    /// Left out, the answer is the list of every folder that has a decision.
    folder: Option<String>,
}

async fn get_trust<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Query(q): Query<FolderQuery>) -> Result<Json<Value>, ApiError> {
    let r = rt(&st)?;
    let Some(asked) = q.folder else {
        let saved: Vec<String> = r.saved.lock().unwrap().trust.keys().cloned().collect();
        let folders: Vec<Value> = saved
            .into_iter()
            .map(|f| json!({"folder": f, "decision": r.decision(&f), "exists": FsPath::new(&f).is_dir(), "mcp": folder_servers(&f).iter().map(|(n, t)| json!({"name": n, "runs": t.display()})).collect::<Vec<_>>()}))
            .collect();
        return Ok(Json(json!({"folders": folders})));
    };
    let folder = std::fs::canonicalize(&asked).map(|p| p.to_string_lossy().to_string()).unwrap_or(asked);
    let servers = folder_servers(&folder);
    Ok(Json(json!({"folder": folder, "decision": r.decision(&folder), "mcp": servers.iter().map(|(n, t)| json!({"name": n, "runs": t.display()})).collect::<Vec<_>>()})))
}

#[derive(Deserialize)]
struct SetTrust {
    folder: String,
    /// `trust`, `own` (disable the folder's skills and servers, use mine) or `forget`.
    decision: String,
}

async fn set_trust<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Json(b): Json<SetTrust>) -> Result<Json<Value>, ApiError> {
    let r = rt(&st)?;
    // a folder that is gone can still be forgotten, by the name it was saved under
    let folder = match std::fs::canonicalize(&b.folder) {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) if b.decision == "forget" => b.folder.clone(),
        Err(e) => return Err(ApiError(StatusCode::BAD_REQUEST, format!("{}: {e}", b.folder))),
    };
    match b.decision.as_str() {
        "trust" | "own" => {
            let h = folder_hash(&folder);
            let mut s = r.saved.lock().unwrap();
            s.trust.insert(folder.clone(), b.decision.clone());
            s.trust_hash.insert(folder.clone(), h);
        }
        "forget" => {
            let mut s = r.saved.lock().unwrap();
            s.trust.remove(&folder);
            s.trust_hash.remove(&folder);
        }
        _ => return Err(ApiError(StatusCode::BAD_REQUEST, "decision must be trust, own or forget".into())),
    }
    r.save()?;
    let _ = st.settings.store.log("folder_trust", &folder, "user", &json!({"decision": b.decision}));
    Ok(Json(json!({"folder": folder, "decision": r.decision(&folder)})))
}
