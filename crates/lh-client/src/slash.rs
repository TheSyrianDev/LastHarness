//! Chat slash commands that call the core and show the answer as a notice: `/usage`, `/insights`,
//! `/rollback [N]`, `/budget <tokens>|off`, `/help`. Same endpoints the GUI uses. `/attach`, `/detach` and
//! `/review` are parsed here too, but the screen runs them (see `Slash::is_screen`), not the core.

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
    /// `/attach [path]`: the screen stages a file. The text is what follows the command, may be empty.
    Attach(String),
    /// `/detach <n>`: the screen removes a staged file. The text is what follows the command.
    Detach(String),
    /// `/review`: the screen asks the core for end-of-session suggestions, from the project's git diff when
    /// there is one, else from the conversation.
    Review,
    /// `/review transcript`: as `Review`, but always from the conversation.
    ReviewTranscript,
}

impl Slash {
    /// True for commands the screen handles itself (staging files, showing suggestions). The core never sees them.
    pub fn is_screen(&self) -> bool {
        matches!(self, Slash::Attach(_) | Slash::Detach(_) | Slash::Review | Slash::ReviewTranscript)
    }
}

/// The one-line list `/help` shows. Keep it in step with `parse`.
pub const HELP: &str = "Commands: /usage (tokens used in this chat), /insights (what the core has learned), /rollback [N] (undo the last changes), /budget <tokens>|off (token limit for this chat), /help (this list). /attach [path] adds a file, /detach <n> removes one, /review asks for suggestions (from the git diff, or the chat when there is none), /review transcript forces the chat.";

/// Parse a chat line. `/budget` takes a token count or `off`; anything else after the command is an error text.
pub fn parse(line: &str) -> Option<Result<Slash, String>> {
    let t = line.trim();
    let (cmd, rest) = t.split_once(' ').map(|(c, r)| (c, r.trim())).unwrap_or((t, ""));
    match cmd {
        "/usage" => Some(Ok(Slash::Usage)),
        "/insights" => Some(Ok(Slash::Insights)),
        "/help" if rest.is_empty() => Some(Ok(Slash::Help)),
        "/attach" => Some(Ok(Slash::Attach(rest.to_string()))),
        "/detach" => Some(Ok(Slash::Detach(rest.to_string()))),
        "/review" if rest.is_empty() => Some(Ok(Slash::Review)),
        "/review" if rest == "transcript" => Some(Ok(Slash::ReviewTranscript)),
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
        Slash::Attach(_) | Slash::Detach(_) | Slash::Review | Slash::ReviewTranscript => Err(format!("{} is handled by the screen, not the core", cmd_name(&cmd))),
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

fn cmd_name(cmd: &Slash) -> &'static str {
    match cmd {
        Slash::Attach(_) => "/attach",
        Slash::Detach(_) => "/detach",
        Slash::ReviewTranscript => "/review transcript",
        _ => "/review",
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
        assert_eq!(parse("hello"), None);
    }

    #[test]
    fn attach_detach_review_parse_for_the_screen() {
        assert_eq!(parse("/attach /tmp/a.txt"), Some(Ok(Slash::Attach("/tmp/a.txt".into()))));
        assert_eq!(parse("  /attach  "), Some(Ok(Slash::Attach(String::new()))));
        assert_eq!(parse("/attachment x"), None);
        assert_eq!(parse("/detach 2"), Some(Ok(Slash::Detach("2".into()))));
        assert_eq!(parse("/detach"), Some(Ok(Slash::Detach(String::new()))));
        assert_eq!(parse("/review"), Some(Ok(Slash::Review)));
        assert_eq!(parse("/review now"), None);
        assert_eq!(parse("/review transcript"), Some(Ok(Slash::ReviewTranscript)));
        for c in [Slash::Attach(String::new()), Slash::Detach("1".into()), Slash::Review, Slash::ReviewTranscript] {
            assert!(c.is_screen());
        }
        for c in [Slash::Usage, Slash::Insights, Slash::Help, Slash::Rollback("/rollback".into()), Slash::Budget(None)] {
            assert!(!c.is_screen());
        }
    }
}
