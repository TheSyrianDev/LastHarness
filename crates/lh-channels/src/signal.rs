//! Signal through `signal-cli` running as a daemon (`signal-cli daemon --http HOST:PORT`). Incoming
//! messages arrive on its event stream, replies go through its JSON-RPC endpoint. The account must
//! already be linked in signal-cli; nothing here handles Signal's keys. The sender is a phone number.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use lh_provider::SseParser;
use lh_tools::BoxFut;
use serde_json::{json, Value};

use crate::{msg, Channel, Message, Result};

pub struct Signal {
    base: String,
    account: String,
    http: reqwest::Client,
    stream: tokio::sync::Mutex<Option<(reqwest::Response, SseParser)>>,
    id: AtomicU64,
}

impl Signal {
    pub fn new(daemon_url: &str, account: &str) -> Self {
        Self { base: daemon_url.trim_end_matches('/').to_string(), account: account.to_string(), http: reqwest::Client::new(), stream: tokio::sync::Mutex::new(None), id: AtomicU64::new(1) }
    }
}

/// Pull messages out of one event: signal-cli sends the envelope directly or wrapped as a JSON-RPC notification.
fn from_event(v: &Value, me: &str) -> Option<Message> {
    let env = v.get("envelope").or_else(|| v["params"].get("envelope"))?;
    let text = env["dataMessage"]["message"].as_str()?;
    let sender = env["sourceNumber"].as_str().or_else(|| env["source"].as_str())?;
    if sender == me || env["dataMessage"].get("groupInfo").is_some() || env["dataMessage"].get("groupV2").is_some() {
        return None; // our own message, or one in a group (a reply there would reach everyone in it)
    }
    Some(Message { chat: sender.to_string(), sender: sender.to_string(), text: text.to_string() })
}

impl Channel for Signal {
    fn kind(&self) -> &'static str {
        "signal"
    }

    fn next(&self) -> BoxFut<'_, Result<Vec<Message>>> {
        Box::pin(async move {
            let mut guard = self.stream.lock().await;
            if guard.is_none() {
                let r = self.http.get(format!("{}/api/v1/events", self.base)).header("Accept", "text/event-stream").send().await.map_err(|e| msg(format!("signal-cli is not reachable at {}: {}", self.base, crate::scrub(e))))?;
                if !r.status().is_success() {
                    return Err(msg(format!("signal-cli answered {} at {}", r.status(), self.base)));
                }
                *guard = Some((r, SseParser::new()));
            }
            let (resp, parser) = guard.as_mut().unwrap();
            // wait up to a while for something; an empty return just means a quiet spell
            match tokio::time::timeout(Duration::from_secs(25), resp.chunk()).await {
                Err(_) => Ok(vec![]),
                Ok(Err(e)) => {
                    *guard = None;
                    Err(msg(format!("the connection to signal-cli broke: {}", crate::scrub(e))))
                }
                Ok(Ok(None)) => {
                    *guard = None;
                    Err(msg("signal-cli closed the connection"))
                }
                Ok(Ok(Some(bytes))) => Ok(parser.push(&bytes).iter().filter_map(|d| serde_json::from_str::<Value>(d).ok()).filter_map(|v| from_event(&v, &self.account)).collect()),
            }
        })
    }

    fn send(&self, chat: &str, text: &str) -> BoxFut<'_, Result<()>> {
        let (chat, text) = (chat.to_string(), text.to_string());
        Box::pin(async move {
            let body = json!({"jsonrpc": "2.0", "id": self.id.fetch_add(1, Ordering::SeqCst), "method": "send", "params": {"account": self.account, "recipient": [chat], "message": text}});
            let r = self.http.post(format!("{}/api/v1/rpc", self.base)).json(&body).send().await.map_err(|e| msg(format!("signal-cli is not reachable at {}: {}", self.base, crate::scrub(e))))?;
            let v: Value = r.json().await.map_err(|e| msg(format!("signal-cli sent something unreadable: {}", e.without_url())))?;
            if let Some(e) = v.get("error") {
                return Err(msg(format!("signal-cli said: {}", e["message"].as_str().unwrap_or("error"))));
            }
            Ok(())
        })
    }
}
