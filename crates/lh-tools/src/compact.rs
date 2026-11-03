//! Auto-compaction. When a conversation nears the context limit, older turns are replaced by a
//! summary the model writes itself.
//!
//! The summary request is the whole conversation plus one more user message, with the tool list
//! unchanged. Everything before that last message is byte-identical to the previous request, so a
//! server with prefix caching reads the long history from its cache and only the new instruction
//! is fresh work (spike 5: 4x to 20x cheaper than reading it cold).
//!
//! After that the history becomes: the original system text with the summary added to it, then the
//! most recent user turn(s) untouched. Cuts always fall on a user message, so a tool call is never
//! separated from its result.

use lh_provider::{ChatRequest, Error, Message, Role};

use crate::{Meter, MeterVerdict, Model};

pub struct CompactOptions {
    /// Start compacting when the conversation is estimated to reach this many tokens. 0 turns it off.
    pub threshold_tokens: u64,
    /// How many of the latest user turns stay word for word.
    pub keep_last_turns: usize,
}

impl Default for CompactOptions {
    fn default() -> Self {
        Self { threshold_tokens: 24_000, keep_last_turns: 1 }
    }
}

#[derive(Debug, PartialEq)]
pub struct Compacted {
    pub tokens_before: u64,
    pub tokens_after: u64,
    pub messages_summarized: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum CompactError {
    #[error("the summary was not made: {0}")]
    Skipped(String),
    #[error(transparent)]
    Model(#[from] Error),
}

/// Where a summary starts inside the system message.
pub const SUMMARY_MARKER: &str = "\n\n[Summary of the earlier conversation]\n";
const ASK: &str = "Pause the task. Write a summary of this conversation so far, for yourself to continue from. Keep every fact, decision, file name, \
command result and open question that matters for the next steps, and say what was being done last. Plain text, no tool calls, under 400 words.";

/// Rough size of the request in tokens. Counts three bytes per token, so it errs high.
pub fn estimate_tokens(req: &ChatRequest) -> u64 {
    let msgs: usize = req.messages.iter().map(|m| m.content.as_deref().map_or(0, str::len) + m.tool_calls.iter().map(|c| c.name.len() + c.arguments.len()).sum::<usize>()).sum();
    let tools: usize = req.tools.iter().map(|t| t.description.len() + t.parameters.to_string().len()).sum();
    ((msgs + tools) / 3) as u64
}

/// Compact `req` if it is big enough. `known_tokens` is the server-reported size of the last
/// request, which is more accurate than the estimate; the larger of the two is used.
pub async fn compact_if_needed<M: Model, T: Meter>(
    model: &M,
    meter: &T,
    req: &mut ChatRequest,
    known_tokens: Option<u64>,
    opts: &CompactOptions,
) -> Result<Option<Compacted>, CompactError> {
    let before = estimate_tokens(req).max(known_tokens.unwrap_or(0));
    if opts.threshold_tokens == 0 || before < opts.threshold_tokens {
        return Ok(None);
    }
    let lead = req.messages.iter().take_while(|m| m.role == Role::System).count();
    let users: Vec<usize> = req.messages.iter().enumerate().filter(|(_, m)| m.role == Role::User).map(|(i, _)| i).collect();
    if opts.keep_last_turns == 0 || users.len() <= opts.keep_last_turns {
        return Ok(None); // nothing older than the turns we keep
    }
    let cut = users[users.len() - opts.keep_last_turns];
    if cut <= lead {
        return Ok(None);
    }

    let mut ask = req.clone();
    ask.messages.push(Message::user(ASK));
    ask.max_tokens = Some(3000); // room for a thinking model to think and still write the summary
    match meter.before(&ask).await {
        MeterVerdict::Allow => {}
        MeterVerdict::Ask(m) | MeterVerdict::Refuse(m) => return Err(CompactError::Skipped(m)),
    }
    let resp = model.complete(&ask, &mut |_| {}).await?;
    meter.after(&ask, resp.usage).await;
    let summary = resp.text.trim();
    if !resp.tool_calls.is_empty() || summary.is_empty() {
        return Err(CompactError::Skipped("the model did not write a summary".into()));
    }

    let mut system: String = req.messages[..lead].iter().filter_map(|m| m.content.as_deref()).collect::<Vec<_>>().join("\n\n");
    if let Some(i) = system.find(SUMMARY_MARKER) {
        system.truncate(i); // the new summary already includes the old one
    }
    system.push_str(SUMMARY_MARKER);
    system.push_str(summary);
    let summarized = cut - lead;
    let mut messages = vec![Message::system(system)];
    messages.extend(req.messages.drain(cut..));
    req.messages = messages;
    Ok(Some(Compacted { tokens_before: before, tokens_after: estimate_tokens(req), messages_summarized: summarized }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NoMeter;
    use lh_provider::{Event, Response, ToolCall};
    use std::sync::Mutex;

    struct Summarizer {
        reply: Mutex<Option<Response>>,
        seen: Mutex<Vec<ChatRequest>>,
    }
    impl Summarizer {
        fn new(text: &str) -> Self {
            Self { reply: Mutex::new(Some(Response { text: text.into(), ..Default::default() })), seen: Mutex::new(vec![]) }
        }
    }
    impl Model for Summarizer {
        async fn complete(&self, req: &ChatRequest, _: &mut (dyn FnMut(Event) + Send)) -> Result<Response, Error> {
            self.seen.lock().unwrap().push(req.clone());
            Ok(self.reply.lock().unwrap().take().expect("asked twice"))
        }
    }

    fn long_chat() -> ChatRequest {
        let call = ToolCall { id: "c".into(), name: "read_file".into(), arguments: "{}".into(), repaired: false };
        let mut r = ChatRequest { model: "m".into(), ..Default::default() };
        r.messages = vec![
            Message::system("You are helpful."),
            Message::user("first question ".repeat(100)),
            Message::assistant("first answer ".repeat(100), vec![]),
            Message::user("second question"),
            Message::assistant("", vec![call]),
            Message::tool_result("c", "file text"),
            Message::assistant("second answer", vec![]),
            Message::user("latest question"),
        ];
        r
    }
    fn opts(t: u64) -> CompactOptions {
        CompactOptions { threshold_tokens: t, keep_last_turns: 1 }
    }

    #[tokio::test]
    async fn small_chats_and_off_are_left_alone() {
        let m = Summarizer::new("x");
        let mut r = long_chat();
        assert_eq!(compact_if_needed(&m, &NoMeter, &mut r, None, &opts(1_000_000)).await.unwrap(), None);
        assert_eq!(compact_if_needed(&m, &NoMeter, &mut r, None, &opts(0)).await.unwrap(), None);
        assert_eq!(r.messages.len(), 8);
        assert!(m.seen.lock().unwrap().is_empty(), "the model was not called");
    }

    #[tokio::test]
    async fn old_turns_become_a_summary_and_the_latest_turn_stays() {
        let m = Summarizer::new("They discussed two things.");
        let mut r = long_chat();
        let before = r.clone();
        let c = compact_if_needed(&m, &NoMeter, &mut r, None, &opts(100)).await.unwrap().unwrap();
        assert_eq!(c.messages_summarized, 6);
        assert!(c.tokens_after < c.tokens_before);
        assert_eq!(r.messages.len(), 2);
        assert_eq!(r.messages[0].role, Role::System);
        let sys = r.messages[0].content.as_deref().unwrap();
        assert!(sys.starts_with("You are helpful.") && sys.contains("They discussed two things."));
        assert_eq!(r.messages[1], Message::user("latest question"));
        // the summary request is the old history untouched, plus one message at the end
        let sent = &m.seen.lock().unwrap()[0];
        assert_eq!(&sent.messages[..8], &before.messages[..]);
        assert_eq!(sent.messages.len(), 9);
    }

    #[tokio::test]
    async fn a_second_compaction_replaces_the_old_summary_instead_of_stacking() {
        let mut r = long_chat();
        compact_if_needed(&Summarizer::new("FIRST SUMMARY"), &NoMeter, &mut r, None, &opts(100)).await.unwrap();
        r.messages.push(Message::assistant("answer ".repeat(100), vec![]));
        r.messages.push(Message::user("one more"));
        compact_if_needed(&Summarizer::new("SECOND SUMMARY"), &NoMeter, &mut r, Some(500), &opts(100)).await.unwrap().unwrap();
        let sys = r.messages[0].content.as_deref().unwrap();
        assert!(sys.contains("SECOND SUMMARY") && !sys.contains("FIRST SUMMARY"));
        assert_eq!(sys.matches("[Summary of the earlier conversation]").count(), 1);
    }

    #[tokio::test]
    async fn a_bad_summary_changes_nothing() {
        let call = ToolCall { id: "x".into(), name: "read_file".into(), arguments: "{}".into(), repaired: false };
        let bad = Summarizer { reply: Mutex::new(Some(Response { tool_calls: vec![call], ..Default::default() })), seen: Mutex::new(vec![]) };
        let mut r = long_chat();
        let before = r.clone();
        assert!(matches!(compact_if_needed(&bad, &NoMeter, &mut r, None, &opts(100)).await, Err(CompactError::Skipped(_))));
        assert_eq!(r.messages, before.messages);
        let blank = Summarizer::new("   ");
        assert!(compact_if_needed(&blank, &NoMeter, &mut r, None, &opts(100)).await.is_err());
        assert_eq!(r.messages, before.messages);
    }

    #[tokio::test]
    async fn a_single_turn_cannot_be_compacted_and_known_size_counts() {
        let m = Summarizer::new("x");
        let mut one = ChatRequest { model: "m".into(), messages: vec![Message::system("s"), Message::user("only turn")], ..Default::default() };
        assert_eq!(compact_if_needed(&m, &NoMeter, &mut one, Some(999_999), &opts(100)).await.unwrap(), None);
        let mut r = long_chat();
        // tiny by estimate, but the server said the last request was huge
        for msg in r.messages.iter_mut() {
            msg.content = msg.content.as_ref().map(|c| c.chars().take(5).collect());
        }
        assert!(compact_if_needed(&Summarizer::new("S"), &NoMeter, &mut r, Some(30_000), &opts(24_000)).await.unwrap().is_some());
    }
}
