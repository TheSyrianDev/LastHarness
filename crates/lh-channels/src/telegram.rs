//! Telegram bot (Bot API, long polling). Messages from anyone are returned; the core decides who is
//! allowed. The sender is the numeric user id.

use std::sync::Mutex;
use std::time::Duration;

use lh_tools::BoxFut;
use serde_json::{json, Value};

use crate::{msg, Channel, Error, Message, Result};

pub struct Telegram {
    base: String,
    http: reqwest::Client,
    offset: Mutex<i64>,
}

impl Telegram {
    pub fn new(api_base: &str, token: &str) -> Self {
        let base = if api_base.is_empty() { "https://api.telegram.org" } else { api_base.trim_end_matches('/') };
        Self { base: format!("{base}/bot{token}"), http: reqwest::Client::builder().timeout(Duration::from_secs(45)).build().unwrap_or_default(), offset: Mutex::new(0) }
    }
}

impl Channel for Telegram {
    fn kind(&self) -> &'static str {
        "telegram"
    }

    fn next(&self) -> BoxFut<'_, Result<Vec<Message>>> {
        Box::pin(async move {
            let offset = *self.offset.lock().unwrap();
            let r = self.http.get(format!("{}/getUpdates", self.base)).query(&[("timeout", "25".to_string()), ("offset", offset.to_string()), ("allowed_updates", "[\"message\"]".to_string())]).send().await.map_err(|e| msg(format!("Telegram is not reachable: {}", e.without_url())))?;
            if r.status() == 401 || r.status() == 404 {
                return Err(Error::Auth("Telegram".into()));
            }
            let v: Value = r.json().await.map_err(|e| msg(format!("Telegram sent something unreadable: {}", e.without_url())))?;
            if v["ok"] != true {
                return Err(msg(format!("Telegram said: {}", v["description"].as_str().unwrap_or("not ok"))));
            }
            let mut out = vec![];
            for u in v["result"].as_array().into_iter().flatten() {
                if let Some(id) = u["update_id"].as_i64() {
                    let mut o = self.offset.lock().unwrap();
                    *o = (*o).max(id + 1);
                }
                let m = &u["message"];
                if let (Some(text), Some(from), Some(chat)) = (m["text"].as_str(), m["from"]["id"].as_i64(), m["chat"]["id"].as_i64()) {
                    // only one-to-one chats: a reply in a group would be read by everyone in it
                    if m["from"]["is_bot"] != true && m["chat"]["type"].as_str().is_none_or(|t| t == "private") {
                        out.push(Message { chat: chat.to_string(), sender: from.to_string(), text: text.to_string() });
                    }
                }
            }
            Ok(out)
        })
    }

    fn send(&self, chat: &str, text: &str) -> BoxFut<'_, Result<()>> {
        let (chat, text) = (chat.to_string(), text.to_string());
        Box::pin(async move {
            let r = self.http.post(format!("{}/sendMessage", self.base)).json(&json!({"chat_id": chat, "text": text})).send().await.map_err(|e| msg(format!("Telegram is not reachable: {}", e.without_url())))?;
            if !r.status().is_success() {
                return Err(msg(format!("Telegram refused the message ({})", r.status())));
            }
            Ok(())
        })
    }
}
