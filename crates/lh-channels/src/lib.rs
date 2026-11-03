//! Messaging channels: talk to the assistant from a chat app on your phone.
//!
//! A channel has two jobs: wait for messages and send replies. Everything else (who is allowed, which
//! conversation a message belongs to, permission questions, the sensitive-data rule) is decided by the
//! core, the same for every channel. Every channel here is a client that the core runs on your own
//! machine; there is no server in between that holds your credentials.
//!
//! Channels carry no security promise: the service they run through can read the messages.

mod matrix;
mod signal;
mod slack;
mod telegram;

use lh_tools::BoxFut;
use serde::{Deserialize, Serialize};

pub use matrix::Matrix;
pub use signal::Signal;
pub use slack::Slack;
pub use telegram::Telegram;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Msg(String),
    /// The service refused our credentials.
    #[error("{0} refused the credentials; check the token")]
    Auth(String),
}
pub type Result<T> = std::result::Result<T, Error>;

/// An error's text with any web address taken out, since an address can carry a token.
pub(crate) fn scrub(e: impl std::fmt::Display) -> String {
    e.to_string().split_whitespace().map(|w| if w.contains("://") { "[address]" } else { w }).collect::<Vec<_>>().join(" ")
}

pub(crate) fn msg(s: impl Into<String>) -> Error {
    Error::Msg(s.into())
}

/// One incoming message.
#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    /// Where to answer: a chat, room or conversation id on that service.
    pub chat: String,
    /// Who wrote it, as that service names people. The allow-list is checked against this.
    pub sender: String,
    pub text: String,
}

pub trait Channel: Send + Sync {
    fn kind(&self) -> &'static str;
    /// Wait for new messages. Returns when there are some or after a quiet spell (then with none).
    /// An error means the connection broke; the caller waits and tries again.
    fn next(&self) -> BoxFut<'_, Result<Vec<Message>>>;
    fn send(&self, chat: &str, text: &str) -> BoxFut<'_, Result<()>>;
}

/// What the user sets up for one channel. Secrets are references (`env:NAME` or `secret:NAME`), never values.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChannelConfig {
    pub name: String,
    /// telegram, matrix, slack or signal
    pub kind: String,
    #[serde(default)]
    pub enabled: bool,
    /// Who may talk to the assistant, as the service names them. Nobody else is answered or even logged.
    #[serde(default)]
    pub allow: Vec<String>,
    /// `chat` (no tools) or `assistant` (tools, with permission questions asked in the chat).
    #[serde(default = "default_mode")]
    pub mode: String,
    /// The model to talk to. Required to enable.
    #[serde(default)]
    pub model: String,
    /// Kind-specific settings; see `required_settings`.
    #[serde(default)]
    pub settings: std::collections::BTreeMap<String, String>,
}

fn default_mode() -> String {
    "chat".into()
}

pub const KINDS: &[&str] = &["telegram", "matrix", "slack", "signal"];

/// Setting names with what they mean.
pub type SettingHelp = &'static [(&'static str, &'static str)];

/// The settings each kind needs, with what they mean. The first group must be given; the second may be.
pub fn required_settings(kind: &str) -> Option<(SettingHelp, SettingHelp)> {
    Some(match kind {
        "telegram" => (&[("token", "the bot token from BotFather, as env:NAME or secret:NAME")], &[("api_base", "default https://api.telegram.org")]),
        "matrix" => (&[("homeserver", "for example https://matrix.example.org"), ("token", "an access token for the assistant's account, as env:NAME or secret:NAME")], &[]),
        "slack" => (&[("app_token", "the app-level token (xapp-...) for Socket Mode, as env:NAME or secret:NAME"), ("bot_token", "the bot token (xoxb-...), as env:NAME or secret:NAME")], &[("api_base", "default https://slack.com/api")]),
        "signal" => (&[("daemon_url", "where signal-cli runs in daemon mode, for example http://127.0.0.1:8080"), ("account", "the phone number signal-cli is linked to")], &[]),
        _ => return None,
    })
}

/// What the user must be told before the channel is switched on.
pub fn warning(kind: &str) -> String {
    let common = "Messages on this channel pass through the service's servers, which can read them. LastHarness cannot protect them and makes no promise about their security. Notes marked sensitive are never sent here, and a reply that looks like it holds sensitive information is held back. Only the people you list can talk to the assistant; everyone else is ignored.";
    match kind {
        "slack" => format!("{common} Slack keeps message history, and the workspace owner and Slack itself can read messages in the workspace. Use a workspace you control and create your own Slack app (this avoids Slack's Marketplace limits)."),
        "telegram" => format!("{common} Telegram bot chats are not end-to-end encrypted."),
        "matrix" => format!("{common} Only rooms without end-to-end encryption work; messages in encrypted rooms cannot be read."),
        "signal" => format!("{common} Signal protects messages on the way, but this machine decrypts them to read them, and the assistant's replies are written to the chat as plain text."),
        _ => common.to_string(),
    }
}

/// Look up a secret reference. `env:NAME` reads an environment variable; `secret:NAME` the system store.
pub fn resolve_secret(reference: &str) -> Result<String> {
    if let Some(name) = reference.strip_prefix("env:") {
        return std::env::var(name).map_err(|_| msg(format!("the environment variable {name} is not set")));
    }
    Err(msg("secrets must be written as env:NAME (an environment variable) or secret:NAME (the system secret store)"))
}

/// Split a long reply into pieces a chat app accepts, at paragraph or line breaks where possible.
pub fn split_message(text: &str, max: usize) -> Vec<String> {
    let mut out = vec![];
    let mut rest = text.trim();
    while rest.chars().count() > max {
        let cut_byte = rest.char_indices().nth(max).map(|(i, _)| i).unwrap_or(rest.len());
        let window = &rest[..cut_byte];
        let at = window.rfind("\n\n").or_else(|| window.rfind('\n')).or_else(|| window.rfind(". ").map(|i| i + 1)).or_else(|| window.rfind(' ')).filter(|i| *i > max / 3).unwrap_or(cut_byte);
        out.push(rest[..at].trim().to_string());
        rest = rest[at..].trim_start();
    }
    if !rest.is_empty() {
        out.push(rest.to_string());
    }
    out
}

/// Build the channel client for a config. Secrets are read now.
pub fn build(c: &ChannelConfig) -> Result<Box<dyn Channel>> {
    let (need, _) = required_settings(&c.kind).ok_or_else(|| msg(format!("unknown channel kind {}; the kinds are {}", c.kind, KINDS.join(", "))))?;
    for (k, _) in need {
        if c.settings.get(*k).is_none_or(|v| v.is_empty()) {
            return Err(msg(format!("the {} channel needs the setting {k}", c.kind)));
        }
    }
    let s = |k: &str| c.settings.get(k).cloned().unwrap_or_default();
    Ok(match c.kind.as_str() {
        "telegram" => Box::new(Telegram::new(&s("api_base"), &resolve_secret(&s("token"))?)),
        "matrix" => Box::new(Matrix::new(&s("homeserver"), &resolve_secret(&s("token"))?, c.allow.clone())),
        "slack" => Box::new(Slack::new(&s("api_base"), &resolve_secret(&s("app_token"))?, &resolve_secret(&s("bot_token"))?)),
        "signal" => Box::new(Signal::new(&s("daemon_url"), &s("account"))),
        _ => unreachable!(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_replies_split_at_natural_breaks() {
        let p = "First paragraph here.\n\nSecond paragraph that is a bit longer than the first one.";
        assert_eq!(split_message(p, 40), ["First paragraph here.", "Second paragraph that is a bit longer", "than the first one."]);
        assert_eq!(split_message("short", 100), ["short"]);
        assert!(split_message(&"é".repeat(25), 10).iter().all(|c| c.chars().count() <= 10), "multi-byte text is cut on character edges");
        assert_eq!(split_message(&"x".repeat(25), 10).concat().len(), 25, "an unbroken run is still cut");
        assert!(split_message("   ", 10).is_empty());
    }

    #[test]
    fn secrets_must_be_references_and_every_kind_has_a_warning() {
        assert!(resolve_secret("plain-token-text").unwrap_err().to_string().contains("env:NAME"));
        assert!(resolve_secret("env:LH_DEFINITELY_NOT_SET_9").unwrap_err().to_string().contains("not set"));
        for k in KINDS {
            assert!(warning(k).contains("cannot protect them"), "{k}");
            assert!(required_settings(k).is_some());
        }
        assert!(warning("slack").contains("Marketplace"));
        let c = ChannelConfig { name: "x".into(), kind: "irc".into(), enabled: false, allow: vec![], mode: "chat".into(), model: String::new(), settings: Default::default() };
        assert!(build(&c).err().unwrap().to_string().contains("unknown channel kind irc"));
    }
}
