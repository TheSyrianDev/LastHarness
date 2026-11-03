//! Matrix client (client-server API, long-polling sync). Rooms without end-to-end encryption only: an
//! encrypted room's messages cannot be read here. The sender is the full Matrix id (@user:server).
//! Invitations are accepted only from people on the allow-list.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use lh_tools::BoxFut;
use serde_json::{json, Value};

use crate::{msg, Channel, Error, Message, Result};

pub struct Matrix {
    base: String,
    token: String,
    http: reqwest::Client,
    allow: Vec<String>,
    since: Mutex<Option<String>>,
    me: Mutex<Option<String>>,
    txn: AtomicU64,
}

fn enc(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

impl Matrix {
    pub fn new(homeserver: &str, token: &str, allow: Vec<String>) -> Self {
        Self { base: homeserver.trim_end_matches('/').to_string(), token: token.to_string(), http: reqwest::Client::builder().timeout(Duration::from_secs(60)).build().unwrap_or_default(), allow: allow.into_iter().map(|a| a.to_lowercase()).collect(), since: Mutex::new(None), me: Mutex::new(None), txn: AtomicU64::new(1) }
    }

    /// How many people are in the room; `None` if it cannot be found out (then the room is not served).
    async fn member_count(&self, room: &str) -> Option<usize> {
        let v = self.get(&format!("rooms/{}/joined_members", enc(room)), &[]).await.ok()?;
        v["joined"].as_object().map(|m| m.len())
    }

    async fn get(&self, path: &str, query: &[(&str, String)]) -> Result<Value> {
        let r = self.http.get(format!("{}/_matrix/client/v3/{path}", self.base)).bearer_auth(&self.token).query(query).send().await.map_err(|e| msg(format!("the Matrix server is not reachable: {}", e.without_url())))?;
        if r.status() == 401 || r.status() == 403 {
            return Err(Error::Auth("the Matrix server".into()));
        }
        r.json().await.map_err(|e| msg(format!("the Matrix server sent something unreadable: {}", e.without_url())))
    }
}

impl Channel for Matrix {
    fn kind(&self) -> &'static str {
        "matrix"
    }

    fn next(&self) -> BoxFut<'_, Result<Vec<Message>>> {
        Box::pin(async move {
            if self.me.lock().unwrap().is_none() {
                let who = self.get("account/whoami", &[]).await?;
                *self.me.lock().unwrap() = Some(who["user_id"].as_str().ok_or_else(|| msg("the Matrix server did not say who this account is"))?.to_string());
            }
            let since = self.since.lock().unwrap().clone();
            // the first sync only finds where "now" is: old messages are history, not requests
            let first = since.is_none();
            let mut q = vec![("timeout", if first { "0".to_string() } else { "25000".to_string() })];
            if let Some(s) = &since {
                q.push(("since", s.clone()));
            }
            let v = self.get("sync", &q).await?;
            *self.since.lock().unwrap() = v["next_batch"].as_str().map(str::to_string);
            if first {
                return Ok(vec![]);
            }
            // join rooms the allow-listed people invited us to
            for (room, inv) in v["rooms"]["invite"].as_object().into_iter().flatten() {
                let from = inv["invite_state"]["events"].as_array().into_iter().flatten().find(|e| e["type"] == "m.room.member" && e["content"]["membership"] == "invite").and_then(|e| e["sender"].as_str());
                if from.is_some_and(|f| self.allow.contains(&f.to_lowercase())) {
                    let _ = self.http.post(format!("{}/_matrix/client/v3/join/{}", self.base, enc(room))).bearer_auth(&self.token).json(&json!({})).send().await;
                }
            }
            let me = self.me.lock().unwrap().clone().unwrap_or_default();
            let mut out = vec![];
            for (room, r) in v["rooms"]["join"].as_object().into_iter().flatten() {
                // a reply goes to everyone in the room, so only one-to-one rooms are served
                let wrote = r["timeline"]["events"].as_array().into_iter().flatten().any(|e| e["type"] == "m.room.message" && e["sender"].as_str() != Some(&me));
                if wrote && self.member_count(room).await.is_none_or(|n| n > 2) {
                    continue;
                }
                for e in r["timeline"]["events"].as_array().into_iter().flatten() {
                    if e["type"] == "m.room.message" && e["content"]["msgtype"] == "m.text" && e["sender"].as_str() != Some(&me) {
                        if let (Some(s), Some(b)) = (e["sender"].as_str(), e["content"]["body"].as_str()) {
                            out.push(Message { chat: room.clone(), sender: s.to_string(), text: b.to_string() });
                        }
                    }
                }
            }
            Ok(out)
        })
    }

    fn send(&self, chat: &str, text: &str) -> BoxFut<'_, Result<()>> {
        let (chat, text) = (chat.to_string(), text.to_string());
        Box::pin(async move {
            let txn = format!("lh{}-{}", std::process::id(), self.txn.fetch_add(1, Ordering::SeqCst));
            let r = self.http.put(format!("{}/_matrix/client/v3/rooms/{}/send/m.room.message/{txn}", self.base, enc(&chat))).bearer_auth(&self.token).json(&json!({"msgtype": "m.text", "body": text})).send().await.map_err(|e| msg(format!("the Matrix server is not reachable: {}", e.without_url())))?;
            if !r.status().is_success() {
                return Err(msg(format!("the Matrix server refused the message ({})", r.status())));
            }
            Ok(())
        })
    }
}
