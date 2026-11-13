//! Chat slash commands that call the core and show the answer as a notice: `/usage`, `/insights`,
//! `/rollback [N]`, `/budget <tokens>|off`, `/help`. Same endpoints the GUI uses.

use serde_json::{json, Value};

use crate::Client;

/// What a slash line asks for. `None` means the line is not one of these commands.
#[derive(Debug, PartialEq)]
pub enum Slash {
    Usage,
    Insights,
    Rollback(String),
    Budget(Option<u64>),
    Help,
}

/// The one-line list `/help` shows. Keep it in step with `parse`.
pub const HELP: &str = "Commands: /usage (tokens used in this chat), /insights (what the core has learned), /rollback [N] (undo the last changes), /budget <tokens>|off (token limit for this chat), /help (this list). /attach and /detach add or remove files; /review asks for suggestions.";

/// Parse a chat line. `/budget` takes a token count or `off`; anything else after the command is an error text.
pub fn parse(line: &str) -> Option<Result<Slash, String>> {
    let t = line.trim();
    let (cmd, rest) = t.split_once(' ').map(|(c, r)| (c, r.trim())).unwrap_or((t, ""));
    match cmd {
        "/usage" => Some(Ok(Slash::Usage)),
        "/insights" => Some(Ok(Slash::Insights)),
        "/help" if rest.is_empty() => Some(Ok(Slash::Help)),
        "/rollback" => Some(Ok(Slash::Rollback(t.to_string()))),
        "/budget" => Some(match rest {
            "off" => Ok(Slash::Budget(None)),
            n => n.parse::<u64>().map(|v| Slash::Budget(Some(v))).map_err(|_| "usage: /budget <tokens> or /budget off".to_string()),
        }),
        _ => None,
    }
}

/// Run one parsed command against the core and return the text to show.
pub async fn run(c: &Client, session: &str, cmd: Slash) -> Result<String, String> {
    let text = |v: Value| v["text"].as_str().or(v["message"].as_str()).unwrap_or("done").to_string();
    match cmd {
        Slash::Help => Ok(HELP.to_string()),
        Slash::Usage => c.call("GET", &format!("/v1/sessions/{session}/usage"), None).await.map(text).map_err(|e| e.to_string()),
        Slash::Insights => c.call("GET", "/v1/insights", None).await.map(text).map_err(|e| e.to_string()),
        Slash::Rollback(cmd) => c.call("POST", &format!("/v1/sessions/{session}/rollback"), Some(json!({ "command": cmd }))).await.map(text).map_err(|e| e.to_string()),
        Slash::Budget(tokens) => c
            .call("POST", &format!("/v1/sessions/{session}/budget"), Some(json!({ "tokens": tokens })))
            .await
            .map(|_| match tokens {
                Some(n) => format!("Token budget for this chat: {n}. A turn is refused once it is used up."),
                None => "Token budget removed for this chat.".to_string(),
            })
            .map_err(|e| e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_is_local_and_lists_every_command() {
        assert_eq!(parse("/help"), Some(Ok(Slash::Help)));
        assert!(matches!(parse("/help me"), None));
        for c in ["/usage", "/insights", "/rollback", "/budget", "/help", "/attach", "/review"] {
            assert!(HELP.contains(c), "help misses {c}");
        }
    }

    #[test]
    fn parses_the_four_commands_and_rejects_bad_budgets() {
        assert_eq!(parse("/usage"), Some(Ok(Slash::Usage)));
        assert_eq!(parse("  /insights "), Some(Ok(Slash::Insights)));
        assert_eq!(parse("/rollback 3"), Some(Ok(Slash::Rollback("/rollback 3".into()))));
        assert_eq!(parse("/budget 5000"), Some(Ok(Slash::Budget(Some(5000)))));
        assert_eq!(parse("/budget off"), Some(Ok(Slash::Budget(None))));
        assert!(matches!(parse("/budget lots"), Some(Err(_))));
        assert_eq!(parse("/attach x"), None);
        assert_eq!(parse("hello"), None);
    }
}
