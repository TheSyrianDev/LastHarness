//! `lh -p "question"`: one turn without the full-screen app, for scripts and other programs.
//!
//! The answer goes to standard output; notes and errors go to standard error. With `--json` every event goes to
//! standard output as one JSON line. The chat works in the current folder with the usual tools. Nobody is there
//! to answer a permission question, so each one is answered no and named on standard error, unless the
//! permission settings already allow it. With `--bypass-permissions` every question is answered yes, after a
//! warning on standard error.

use std::io::{Read, Write};

use serde_json::{json, Value};

use crate::client::{Client, ServerEvent};

/// How the turn ended, as the exit code: 0 answered, 1 the turn failed or was refused, 3 a tool was refused for
/// lack of anyone to ask (the answer may be incomplete).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    Answered,
    Failed,
    AskedAndRefused,
}

impl End {
    pub fn code(self) -> i32 {
        match self {
            End::Answered => 0,
            End::Failed => 1,
            End::AskedAndRefused => 3,
        }
    }
}

/// Printed to standard error when a run starts with `--bypass-permissions`.
pub const BYPASS_WARNING: &str = "Warning: --bypass-permissions lets this run write files and run commands without asking.\nEvery permission question in this run is answered yes. The sandbox stays on (--no-sandbox turns it off).";

/// Printed to standard error when a run starts with `--no-sandbox`.
pub const NO_SANDBOX_WARNING: &str = "Warning: --no-sandbox turns the sandbox off for this run.\nCommands can change any file your user can, not only this folder, and reach the network. Permission questions still apply.";

/// The prompt: the text given, or standard input when it is `-`. Standard input that is not a terminal is added
/// after the given text, so `cat notes.txt | lh -p "summarise this"` works.
pub fn prompt(given: &str, stdin: &mut dyn Read, stdin_is_terminal: bool) -> String {
    let mut piped = String::new();
    if given == "-" || !stdin_is_terminal {
        let _ = stdin.read_to_string(&mut piped);
    }
    match (given == "-", piped.trim().is_empty()) {
        (true, _) => piped.trim().to_string(),
        (false, true) => given.to_string(),
        (false, false) => format!("{given}\n\n{}", piped.trim_end()),
    }
}

/// The JSON line for one event, for `--json`.
/// `yes` is the answer given to permission questions in this run.
pub fn event_json(e: &ServerEvent, yes: bool) -> Value {
    match e {
        ServerEvent::Reasoning(t) => json!({"type": "reasoning", "text": t}),
        ServerEvent::Text(t) => json!({"type": "text", "text": t}),
        ServerEvent::ToolStart { id, name, arguments } => json!({"type": "tool_start", "id": id, "name": name, "arguments": arguments}),
        ServerEvent::ToolResult { id, text } => json!({"type": "tool_result", "id": id, "text": text}),
        ServerEvent::Ask { id, tool, what, .. } => json!({"type": "ask", "id": id, "tool": tool, "what": what, "answer": if yes { "yes" } else { "no" }}),
        ServerEvent::Usage { prompt_tokens, completion_tokens, cached_tokens } => json!({"type": "usage", "prompt_tokens": prompt_tokens, "completion_tokens": completion_tokens, "cached_tokens": cached_tokens}),
        ServerEvent::Finish(r) => json!({"type": "finish", "reason": r}),
        ServerEvent::Done { empty } => json!({"type": "done", "empty": empty}),
        ServerEvent::StepLimit => json!({"type": "step_limit"}),
        ServerEvent::Refused(m) => json!({"type": "refused", "message": m}),
        ServerEvent::Compacted { messages_summarized } => json!({"type": "compacted", "messages_summarized": messages_summarized}),
        ServerEvent::Notice(m) => json!({"type": "notice", "message": m}),
        ServerEvent::Error(m) => json!({"type": "error", "message": m}),
        ServerEvent::Show { title, kind, .. } => json!({"type": "show", "title": title, "kind": kind}),
        ServerEvent::Diff { id, path, diff } => json!({"type": "diff", "id": id, "path": path, "diff": diff}),
        ServerEvent::Other => json!({"type": "other"}),
    }
}

/// Run one turn and write it out. `bypass` answers every permission question yes.
pub async fn run(client: &Client, session: &str, text: &str, as_json: bool, bypass: bool, out: &mut dyn Write, err: &mut dyn Write) -> End {
    let mut stream = match client.turn(session, text).await {
        Ok(s) => s,
        Err(e) => {
            let _ = writeln!(err, "lh: {e}");
            return End::Failed;
        }
    };
    let (mut end, mut refused_ask, mut ended_with_newline) = (End::Failed, false, true);
    loop {
        let ev = match stream.next().await {
            Ok(Some(ev)) => ev,
            Ok(None) => break,
            Err(e) => {
                let _ = writeln!(err, "lh: {e}");
                end = End::Failed;
                break;
            }
        };
        if as_json {
            let _ = writeln!(out, "{}", event_json(&ev, bypass));
        }
        match &ev {
            ServerEvent::Text(t) if !as_json => {
                let _ = write!(out, "{t}");
                let _ = out.flush();
                ended_with_newline = t.ends_with('\n');
            }
            ServerEvent::Ask { id, tool, what, .. } if bypass => {
                let _ = writeln!(err, "lh: said yes to {tool} ({what}): --bypass-permissions");
                let _ = client.answer(session, id, true).await;
            }
            ServerEvent::Ask { id, tool, what, .. } => {
                refused_ask = true;
                let _ = writeln!(err, "lh: said no to {tool} ({what}): nobody is here to ask. Allow it in the permission settings, or run lh without -p.");
                let _ = client.answer(session, id, false).await;
            }
            ServerEvent::ToolStart { name, .. } if !as_json => {
                let _ = writeln!(err, "lh: using {name}");
            }
            ServerEvent::Notice(m) if !as_json => {
                let _ = writeln!(err, "lh: {m}");
            }
            ServerEvent::Done { .. } => {
                end = End::Answered;
                break;
            }
            ServerEvent::StepLimit => {
                let _ = writeln!(err, "lh: stopped, this took more steps than one turn allows");
                end = End::Failed;
                break;
            }
            ServerEvent::Refused(m) | ServerEvent::Error(m) => {
                let _ = writeln!(err, "lh: {m}");
                end = End::Failed;
                break;
            }
            _ => {}
        }
    }
    if !as_json && !ended_with_newline {
        let _ = writeln!(out);
    }
    if end == End::Answered && refused_ask {
        End::AskedAndRefused
    } else {
        end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bypass_keeps_the_sandbox_and_no_sandbox_warns_on_its_own() {
        assert!(BYPASS_WARNING.contains("sandbox stays on"));
        assert!(NO_SANDBOX_WARNING.contains("turns the sandbox off"));
        assert!(!NO_SANDBOX_WARNING.contains("answered yes"), "a separate warning, not the bypass one");
    }

    #[test]
    fn the_prompt_comes_from_the_words_given_or_from_a_pipe() {
        let mut none: &[u8] = b"";
        assert_eq!(prompt("hi", &mut none, true), "hi");
        let mut piped: &[u8] = b"line one\nline two\n";
        assert_eq!(prompt("summarise", &mut piped, false), "summarise\n\nline one\nline two");
        let mut piped: &[u8] = b"  only this \n";
        assert_eq!(prompt("-", &mut piped, false), "only this");
        let mut keyboard: &[u8] = b"ignored";
        assert_eq!(prompt("hi", &mut keyboard, true), "hi", "a terminal is not read");
    }

    #[test]
    fn exit_codes() {
        assert_eq!((End::Answered.code(), End::Failed.code(), End::AskedAndRefused.code()), (0, 1, 3));
    }

    #[test]
    fn json_lines_name_the_event() {
        assert_eq!(event_json(&ServerEvent::Text("a".into()), false), json!({"type": "text", "text": "a"}));
        assert_eq!(event_json(&ServerEvent::Done { empty: false }, false)["type"], "done");
        let ask = ServerEvent::Ask { id: "1".into(), tool: "write_file".into(), what: "w".into(), reason: String::new(), worker: String::new(), detail: String::new() };
        assert_eq!((event_json(&ask, false)["answer"].as_str(), event_json(&ask, true)["answer"].as_str()), (Some("no"), Some("yes")));
    }
}
