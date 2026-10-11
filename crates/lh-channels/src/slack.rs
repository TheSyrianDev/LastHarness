//! Slack through Socket Mode: the app opens a WebSocket to Slack, so no public address is needed and
//! every user runs their own app. Direct messages only. The sender is the Slack user id.

use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use lh_tools::BoxFut;
use serde_json::{json, Value};
use tokio_tungstenite::tungstenite::Message as Ws;

use crate::{msg, Channel, Error, Message, Result};

type Socket = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

pub struct Slack {
    api: String,
    app_token: String,
    bot_token: String,
    http: reqwest::Client,
    ws: tokio::sync::Mutex<Option<Socket>>,
}

impl Slack {
    pub fn new(api_base: &str, app_token: &str, bot_token: &str) -> Self {
        let api = if api_base.is_empty() { "https://slack.com/api" } else { api_base.trim_end_matches('/') };
        Self { api: api.to_string(), app_token: app_token.to_string(), bot_token: bot_token.to_string(), http: reqwest::Client::builder().timeout(Duration::from_secs(30)).build().unwrap_or_default(), ws: tokio::sync::Mutex::new(None) }
    }

    async fn connect(&self) -> Result<Socket> {
        let r = self.http.post(format!("{}/apps.connections.open", self.api)).bearer_auth(&self.app_token).send().await.map_err(|e| msg(format!("Slack is not reachable: {}", e.without_url())))?;
        let v: Value = r.json().await.map_err(|e| msg(format!("Slack sent something unreadable: {}", e.without_url())))?;
        if v["ok"] != true {
            return Err(match v["error"].as_str() {
                Some("invalid_auth" | "not_authed" | "token_revoked" | "account_inactive") => Error::Auth("Slack".into()),
                other => msg(format!("Slack said: {}", other.unwrap_or("not ok"))),
            });
        }
        let url = v["url"].as_str().ok_or_else(|| msg("Slack gave no socket address"))?;
        let connector = if url.starts_with("wss://") { Some(tls_connector()) } else { None };
        let (ws, _) = tokio_tungstenite::connect_async_tls_with_config(url, None, false, connector).await.map_err(|e| msg(format!("cannot open the Slack socket: {}", crate::scrub(e))))?;
        Ok(ws)
    }
}

fn tls_connector() -> tokio_tungstenite::Connector {
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let cfg = rustls::ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider())).with_safe_default_protocol_versions().expect("TLS versions").with_root_certificates(roots).with_no_client_auth();
    tokio_tungstenite::Connector::Rustls(Arc::new(cfg))
}

impl Channel for Slack {
    fn kind(&self) -> &'static str {
        "slack"
    }

    fn next(&self) -> BoxFut<'_, Result<Vec<Message>>> {
        Box::pin(async move {
            let mut guard = self.ws.lock().await;
            if guard.is_none() {
                *guard = Some(self.connect().await?);
            }
            let ws = guard.as_mut().unwrap();
            let frame = match tokio::time::timeout(Duration::from_secs(25), ws.next()).await {
                Err(_) => return Ok(vec![]),
                Ok(None) => {
                    *guard = None;
                    return Err(msg("the Slack socket closed"));
                }
                Ok(Some(Err(e))) => {
                    *guard = None;
                    return Err(msg(format!("the Slack socket broke: {}", crate::scrub(e))));
                }
                Ok(Some(Ok(f))) => f,
            };
            let Ws::Text(text) = frame else { return Ok(vec![]) };
            let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
            match v["type"].as_str() {
                Some("disconnect") => {
                    *guard = None; // Slack asks us to reconnect, which the next call does
                    Ok(vec![])
                }
                Some("events_api") => {
                    // acknowledge at once, or Slack sends the event again
                    if let Some(id) = v["envelope_id"].as_str() {
                        let _ = ws.send(Ws::text(json!({"envelope_id": id}).to_string())).await;
                    }
                    let e = &v["payload"]["event"];
                    let dm = e["type"] == "message" && e["channel_type"] == "im" && e.get("subtype").is_none() && e.get("bot_id").is_none();
                    match (dm, e["user"].as_str(), e["channel"].as_str(), e["text"].as_str()) {
                        (true, Some(u), Some(c), Some(t)) => Ok(vec![Message { chat: c.to_string(), sender: u.to_string(), text: t.to_string() }]),
                        _ => Ok(vec![]),
                    }
                }
                _ => Ok(vec![]),
            }
        })
    }

    fn send(&self, chat: &str, text: &str) -> BoxFut<'_, Result<()>> {
        let (chat, text) = (chat.to_string(), text.to_string());
        Box::pin(async move {
            let r = self.http.post(format!("{}/chat.postMessage", self.api)).bearer_auth(&self.bot_token).json(&json!({"channel": chat, "text": text})).send().await.map_err(|e| msg(format!("Slack is not reachable: {}", e.without_url())))?;
            let v: Value = r.json().await.map_err(|e| msg(format!("Slack sent something unreadable: {}", e.without_url())))?;
            if v["ok"] != true {
                return Err(msg(format!("Slack refused the message: {}", v["error"].as_str().unwrap_or("not ok"))));
            }
            Ok(())
        })
    }
}
