//! Signals: things the user said or did that may mean "do it this way". A stated rule ("always...",
//! "never..."), a correction ("no, I said..."), or a thumbs up or down. They are stored with the session
//! they came from; a habit is a group of similar signals from several different sessions.

use std::collections::HashSet;
use std::sync::LazyLock;

use lh_memory::{detect, Privacy};
use lh_store::now;
use regex::Regex;

use crate::{Learn, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignalKind {
    /// "Always do X", "never do Y", "from now on...".
    Stated,
    /// "No, I said...", "that is wrong", "stop doing...".
    Correction,
    ThumbsUp,
    ThumbsDown,
}

impl SignalKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SignalKind::Stated => "stated",
            SignalKind::Correction => "correction",
            SignalKind::ThumbsUp => "thumbs_up",
            SignalKind::ThumbsDown => "thumbs_down",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        [SignalKind::Stated, SignalKind::Correction, SignalKind::ThumbsUp, SignalKind::ThumbsDown].into_iter().find(|k| k.as_str() == s)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Signal {
    pub id: i64,
    pub at: i64,
    pub session: String,
    pub kind: SignalKind,
    pub text: String,
    pub norm: String,
    pub project: Option<String>,
    pub privacy: Privacy,
    /// 0 = not looked at, 1 = put to the model with its own session, 2 = put to the model as part of a habit.
    pub asked: i64,
}

static STATED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(?:always|never|from now on|from here on|going forward|in future|in the future|every time|each time|whenever|make sure (?:you|to)|don'?t ever|do not ever|remember to)\b|دائم[اً]|دائماً|أبد[اً]|ابدا|من الآن|من الان|في كل مرة|كلما|تأكد (?:من|أن)|لا تفعل|ممنوع",
    )
    .expect("built-in pattern")
});
static CORRECTION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^\s*(?:no[,.! ]|nope|wrong|that'?s (?:not|wrong|incorrect)|not what i|not like that|i (?:said|asked|meant|told you|wanted)|actually[, ]|stop |don'?t |please don'?t|why did you|instead[, ])|^\s*(?:لا[،, .!]|خطأ|ليس هذا|قلت لك|أنا قلت|توقف|بدلا من)")
        .expect("built-in pattern")
});

/// Does this user message look like a rule or a correction? Cheap and runs on every message; the
/// model decides later whether it really is one.
pub fn classify_message(text: &str) -> Option<SignalKind> {
    let t = text.trim();
    if t.chars().count() < 6 || t.chars().count() > 600 {
        return None;
    }
    if STATED.is_match(t) {
        Some(SignalKind::Stated)
    } else if CORRECTION.is_match(t) {
        Some(SignalKind::Correction)
    } else {
        None
    }
}

/// The part of a message that states a suggestion: its questions are the task, not the suggestion. A message made
/// only of questions is kept whole.
pub fn wish_part(text: &str) -> String {
    let mut parts: Vec<String> = vec![];
    let mut cur = String::new();
    for c in text.chars() {
        cur.push(c);
        if matches!(c, '.' | '!' | '?' | '؟' | '\n') {
            parts.push(std::mem::take(&mut cur));
        }
    }
    if !cur.trim().is_empty() {
        parts.push(cur);
    }
    let kept: Vec<&str> = parts.iter().map(|p| p.trim()).filter(|p| !p.is_empty() && !p.ends_with('?') && !p.ends_with('؟')).collect();
    if kept.is_empty() { text.trim().to_string() } else { kept.join(" ") }
}

/// Words that say what a signal is about, for comparing signals with each other.
pub fn norm_words(text: &str) -> Vec<String> {
    const FILLER: &[&str] = &["use", "using", "please", "make", "sure", "just", "like", "want", "need", "should", "must", "t", "s", "ll", "re", "ve", "d", "m", "dont", "didn", "doesn", "stop", "always", "never", "every", "time", "now", "going", "forward", "answer", "reply", "response", "write", "give", "say", "tell", "said", "told"];
    let cleaned = text.replace(['\'', '’'], "");
    let mut out: Vec<String> = lh_memory::text::tokens(&cleaned).into_iter().filter(|w| w.chars().count() > 1 && !FILLER.contains(&w.as_str())).collect();
    out.sort();
    out.dedup();
    out
}

fn similar(a: &HashSet<String>, b: &HashSet<String>) -> bool {
    let common = a.intersection(b).count();
    if common == 0 || a.is_empty() || b.is_empty() {
        return false;
    }
    // a short signal ("no bullet lists") is covered by a longer one that says the same thing and more
    common as f64 / a.len().min(b.len()) as f64 >= 0.6 && (common >= 2 || a.len().min(b.len()) == 1)
}

/// A group of similar signals.
#[derive(Clone, Debug)]
pub struct Cluster {
    pub signals: Vec<Signal>,
    /// How many different sessions they come from.
    pub sessions: usize,
}

impl Learn {
    /// Remember a signal. Returns its id, or `None` if the same session already has the same words.
    pub fn record_signal(&self, session: &str, kind: SignalKind, text: &str, project: Option<&str>) -> Result<Option<i64>> {
        let text = &wish_part(text);
        let norm = norm_words(text).join(" ");
        if norm.is_empty() && !matches!(kind, SignalKind::ThumbsUp | SignalKind::ThumbsDown) {
            return Ok(None);
        }
        let dup: bool = self.store.with(|c| c.query_row("select exists(select 1 from signals where session = ?1 and kind = ?2 and norm = ?3)", (session, kind.as_str(), &norm), |r| r.get(0)))?;
        if dup {
            return Ok(None);
        }
        let privacy = detect(text).privacy;
        Ok(Some(self.store.with(|c| {
            c.execute("insert into signals(at, session, kind, text, norm, project, privacy) values (?1,?2,?3,?4,?5,?6,?7)", (now(), session, kind.as_str(), text, &norm, project, privacy.as_str()))?;
            Ok(c.last_insert_rowid())
        })?))
    }

    /// Look at what the user said in a session and record the messages that look like rules or corrections.
    pub fn scan_messages(&self, session: &str, project: Option<&str>, user_messages: &[String]) -> Result<usize> {
        let mut n = 0;
        for m in user_messages {
            if let Some(kind) = classify_message(m) {
                if self.record_signal(session, kind, m, project)?.is_some() {
                    n += 1;
                }
            }
        }
        Ok(n)
    }

    pub fn signals(&self, session: Option<&str>) -> Result<Vec<Signal>> {
        Ok(self.store.with(|c| {
            let mut q = c.prepare("select id, at, session, kind, text, norm, project, privacy, asked from signals where (?1 is null or session = ?1) order by id")?;
            let rows = q.query_map([session], |r| {
                Ok(Signal {
                    id: r.get(0)?,
                    at: r.get(1)?,
                    session: r.get(2)?,
                    kind: SignalKind::parse(&r.get::<_, String>(3)?).unwrap_or(SignalKind::Correction),
                    text: r.get(4)?,
                    norm: r.get(5)?,
                    project: r.get(6)?,
                    privacy: Privacy::parse(&r.get::<_, String>(7)?).unwrap_or(Privacy::Sensitive),
                    asked: r.get(8)?,
                })
            })?;
            rows.collect()
        })?)
    }

    /// Mark signals as put to the model (`level` 1: with their own session; 2: as part of a habit), so
    /// the same words are not asked about again.
    pub fn mark_asked(&self, ids: &[i64], level: i64) -> Result<()> {
        for id in ids {
            self.store.with(|c| c.execute("update signals set asked = max(asked, ?2) where id = ?1", (id, level)).map(|_| ()))?;
        }
        Ok(())
    }

    /// Habits that have at least one signal not yet put to the model.
    pub fn new_habits(&self, min_sessions: usize) -> Result<Vec<Cluster>> {
        let asked: HashSet<i64> = self.store.with(|c| {
            let mut q = c.prepare("select id from signals where asked >= 2")?;
            let rows = q.query_map([], |r| r.get(0))?;
            rows.collect()
        })?;
        Ok(self.habits(min_sessions)?.into_iter().filter(|c| c.signals.iter().any(|s| !asked.contains(&s.id))).collect())
    }

    /// Groups of similar signals that come from at least `min_sessions` different sessions: habits.
    /// Thumbs signals carry no words and are not grouped.
    pub fn habits(&self, min_sessions: usize) -> Result<Vec<Cluster>> {
        let all: Vec<Signal> = self.signals(None)?.into_iter().filter(|s| !s.norm.is_empty()).collect();
        let sets: Vec<HashSet<String>> = all.iter().map(|s| s.norm.split(' ').map(str::to_string).collect()).collect();
        let mut parent: Vec<usize> = (0..all.len()).collect();
        fn find(p: &mut [usize], x: usize) -> usize {
            if p[x] != x {
                let r = find(p, p[x]);
                p[x] = r;
            }
            p[x]
        }
        for i in 0..all.len() {
            for j in i + 1..all.len() {
                if similar(&sets[i], &sets[j]) {
                    let (a, b) = (find(&mut parent, i), find(&mut parent, j));
                    parent[a] = b;
                }
            }
        }
        let mut groups: std::collections::BTreeMap<usize, Vec<Signal>> = Default::default();
        for (i, s) in all.into_iter().enumerate() {
            groups.entry(find(&mut parent, i)).or_default().push(s);
        }
        Ok(groups
            .into_values()
            .map(|signals| {
                let sessions = signals.iter().map(|s| s.session.as_str()).collect::<HashSet<_>>().len();
                Cluster { signals, sessions }
            })
            .filter(|c| c.sessions >= min_sessions)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_and_corrections_are_told_apart_from_ordinary_messages() {
        for (t, k) in [
            ("Always give me links after research", Some(SignalKind::Stated)),
            ("From now on answer in one line.", Some(SignalKind::Stated)),
            ("never use bullet points please", Some(SignalKind::Stated)),
            ("Make sure to run the tests before you finish", Some(SignalKind::Stated)),
            ("No, I said short answers", Some(SignalKind::Correction)),
            ("That's not what I asked for", Some(SignalKind::Correction)),
            ("Stop adding emojis", Some(SignalKind::Correction)),
            ("Don't summarize the file, just fix it", Some(SignalKind::Correction)),
            ("دائما اعطني الروابط بعد البحث", Some(SignalKind::Stated)),
            ("لا، قلت لك إجابة قصيرة", Some(SignalKind::Correction)),
            ("What is the capital of France?", None),
            ("Can you open the file", None),
            ("thanks", None),
            ("", None),
        ] {
            assert_eq!(classify_message(t), k, "{t}");
        }
    }

    #[test]
    fn only_the_wish_is_kept_not_the_question_asked_with_it() {
        assert_eq!(wish_part("Please never use bullet points. Write short paragraphs. What is a good name for a cat?"), "Please never use bullet points. Write short paragraphs.");
        assert_eq!(wish_part("No bullet points please, I said that before. Why is the sky blue?"), "No bullet points please, I said that before.");
        assert_eq!(wish_part("Why did you use bullets again?"), "Why did you use bullets again?");
        assert_eq!(wish_part("Always add links"), "Always add links");
        assert_eq!(wish_part("دائما اعطني الروابط. ما هي عاصمة فرنسا؟"), "دائما اعطني الروابط.");
    }

    #[test]
    fn similar_wishes_in_different_words_group_together() {
        let sets = |a: &str, b: &str| (norm_words(a).into_iter().collect::<HashSet<_>>(), norm_words(b).into_iter().collect::<HashSet<_>>());
        let (a, b) = sets("don't use bullet points", "No bullet lists please");
        assert!(!similar(&a, &b), "points and lists are different words, so this pair alone is not linked");
        let (a, b) = sets("never use bullet points", "Don't give me bullet points, ever");
        assert!(similar(&a, &b));
        let (a, b) = sets("always add links after research", "add source links after you research");
        assert!(similar(&a, &b), "{a:?} {b:?}");
        let (a, b) = sets("always add links after research", "never use bullet points");
        assert!(!similar(&a, &b));
    }
}
