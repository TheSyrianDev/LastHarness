//! Naming a chat with a small model after its first reply (`chats.title_model`). It runs in the background and
//! never fails a turn: on any problem the first-words title stays.
use lh_memory::{detect, Privacy, Trust};
use lh_provider::{ChatRequest, Message};
use lh_tools::{Meter, MeterVerdict, Model};
use serde_json::json;

use crate::Settings;

pub(crate) struct Job {
    pub model: String,
    pub conv: String,
    /// The first-words title now stored; the new one replaces it only while it is unchanged.
    pub fallback: String,
    pub user: String,
    pub reply: String,
    pub session: String,
}

const ASK: &str = "Write a title for this chat: at most 6 words, in the same language as the chat, no quotes, no ending punctuation. Answer with the title only.";

/// Clean a model's answer into a title: first line, no quotes or markup, no trailing dot, at most 60 characters.
pub(crate) fn clean(raw: &str) -> Option<String> {
    let line = raw.lines().map(str::trim).find(|l| !l.is_empty())?;
    let quotes = |c: char| matches!(c, '"' | '\'' | '`' | '*' | '#' | '“' | '”' | '«' | '»' | ' ');
    let end = |c: char| quotes(c) || matches!(c, '.' | '。' | '!');
    let t = line.trim_matches(end);
    let t = t.strip_prefix("Title:").map(|r| r.trim_matches(end)).unwrap_or(t);
    let t: String = t.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(60).collect();
    (!t.is_empty()).then_some(t)
}

/// Returns the new title when one was stored.
pub(crate) async fn run<M: Model>(model: &M, s: &Settings, job: &Job) -> Option<String> {
    let rt = s.memory.as_ref()?;
    let clip = |t: &str, n: usize| t.chars().take(n).collect::<String>();
    let (user, reply) = (clip(&job.user, 600), clip(&job.reply, 600));
    // a cloud model gets nothing the memory rules would keep from it
    if rt.trust != Trust::Local && detect(&format!("{user}\n{reply}")).privacy == Privacy::Sensitive {
        let _ = s.store.log("chat_title_skipped", &job.session, "assistant", &json!({"why": "sensitive"}));
        return None;
    }
    let mut req = ChatRequest { model: job.model.clone(), messages: vec![Message::user(format!("{ASK}\n\nUser: {user}\n\nAssistant: {reply}"))], max_tokens: Some(40), ..Default::default() };
    req.sampling.extra.insert("chat_template_kwargs".into(), json!({"enable_thinking": false}));
    let meter = s.spend.for_session(&job.session);
    if !matches!(meter.before(&req).await, MeterVerdict::Allow) {
        return None;
    }
    let _ = s.store.log("chat_title_request", &job.session, "assistant", &json!({"model": job.model, "trust": format!("{:?}", rt.trust), "chars": user.chars().count() + reply.chars().count()}));
    let resp = tokio::time::timeout(std::time::Duration::from_secs(60), model.complete(&req, &mut |_| {})).await.ok()?.ok()?;
    meter.after(&req, resp.usage).await;
    let title = clean(&resp.text)?;
    rt.memory.replace_title(&job.conv, &job.fallback, &title).ok()?.then_some(title)
}

#[cfg(test)]
mod tests {
    use super::clean;
    #[test]
    fn cleans() {
        assert_eq!(clean("\"Trip to Rome\"\n").as_deref(), Some("Trip to Rome"));
        assert_eq!(clean("Title: **Plan the week**.").as_deref(), Some("Plan the week"));
        assert_eq!(clean("  \n").as_deref(), None);
        assert_eq!(clean("\"\"").as_deref(), None);
    }
}
