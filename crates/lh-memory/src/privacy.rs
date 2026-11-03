//! Privacy classes. Text is Sensitive if it matches a pattern (credentials, health, finance,
//! identity documents, private marker) or looks like a personal identifier we cannot be sure about.
//! A local model may add its judgment; when it is unsure the text counts as sensitive. Sensitive
//! pages go only to destinations whose trust level is `Local`.

use regex::Regex;
use std::sync::LazyLock;

use crate::Privacy;

/// How far a destination is trusted with the user's memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Trust {
    /// Runs on this machine.
    Local,
    /// A cloud service under the user's own account.
    OwnCloud,
    /// Any cloud service, and messaging channels.
    AnyCloud,
}

impl Trust {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "local" => Some(Trust::Local),
            "own" | "own-cloud" => Some(Trust::OwnCloud),
            "any" | "any-cloud" => Some(Trust::AnyCloud),
            _ => None,
        }
    }
}

impl Privacy {
    /// May a page of this class be sent to a destination with this trust level?
    pub fn allowed_at(self, trust: Trust) -> bool {
        match self {
            Privacy::Normal => true,
            Privacy::Sensitive => trust == Trust::Local,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Classification {
    pub privacy: Privacy,
    /// Why, in a few words, for the user to read. Empty for Normal.
    pub reason: String,
}

impl Classification {
    fn sensitive(reason: impl Into<String>) -> Self {
        Self { privacy: Privacy::Sensitive, reason: reason.into() }
    }
    fn normal() -> Self {
        Self { privacy: Privacy::Normal, reason: String::new() }
    }
}

fn re(p: &str) -> Regex {
    Regex::new(p).expect("built-in pattern")
}

static KEY_SHAPES: LazyLock<Regex> = LazyLock::new(|| {
    re(r"(?x)
        \bsk-[A-Za-z0-9_-]{16,}            # API secret keys
      | \bAKIA[0-9A-Z]{16}\b               # AWS access key id
      | \bgh[pousr]_[A-Za-z0-9]{20,}       # GitHub tokens
      | \bxox[abps]-[A-Za-z0-9-]{10,}      # Slack tokens
      | -----BEGIN\ [A-Z\ ]*PRIVATE\ KEY-----
      | \beyJ[A-Za-z0-9_-]{8,}\.eyJ[A-Za-z0-9_-]{8,}\.   # JSON web tokens
    ")
});
static CREDENTIAL_WORDS: LazyLock<Regex> = LazyLock::new(|| {
    re(r"(?i)\b(?:password|passwd|passphrase|passcode|api[ _-]?key|secret[ _-]?key|access[ _-]?token|auth(?:entication)?[ _-]?token|pin[ _-]?code|private[ _-]?key|2fa|recovery[ _-]?code)\b|كلمة\s+(?:المرور|السر)|رمز\s+(?:الدخول|التحقق|الأمان)|مفتاح\s+(?:الـ?\s*)?(?:API|سري|خاص)")
});
static FINANCE_WORDS: LazyLock<Regex> = LazyLock::new(|| {
    re(r"(?i)\b(?:salary|salaries|income|wages?|bank\s+account|credit\s+card|debit\s+card|iban|swift|mortgage|loans?|debts?|savings|net\s+worth|tax(?:es)?\s+return|payslip|bank\s+balance|rent)\b|راتب|الراتب|دخل|حساب\s+بنكي|بطاقة\s+(?:ائتمان|الائتمان|بنكية)|قرض|دين|ديون|مدخرات|إيجار|الإيجار|ايجار")
});
static HEALTH_WORDS: LazyLock<Regex> = LazyLock::new(|| {
    re(r"(?i)\b(?:allerg\w*|medication\w*|medicine|diagnos\w*|symptoms?|therapy|therapist|prescri\w+|blood\s+(?:pressure|sugar|type)|diabet\w*|cancer|depress\w*|anxiety|pregnan\w*|surgery|mental\s+health|hiv|asthma|epilep\w*|migraine|disorder|illness|disease|chronic)\b|حساسية|دواء|الدواء|أدوية|مرض|المرض|تشخيص|علاج|العلاج|ضغط\s+الدم|سكري|السكري|اكتئاب|حمل|جراحة|عملية\s+جراحية|صحة\s+نفسية")
});
static IDENTITY_WORDS: LazyLock<Regex> = LazyLock::new(|| {
    re(r"(?i)\b(?:passport|driver'?s?\s+licen[cs]e|national\s+id|id\s+(?:card|number)|social\s+security|ssn|tax\s+id|residence\s+permit|visa\s+number)\b|جواز\s+(?:السفر|سفر)|رخصة\s+(?:القيادة|قيادة)|بطاقة\s+(?:الهوية|هوية)|رقم\s+(?:قومي|وطني|الهوية)|الرقم\s+(?:القومي|الوطني)|إقامة")
});
static PRIVATE_MARK: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)#private\b|\[private\]|(?m)^\s*private\s*:|#سري"));
static SSN: LazyLock<Regex> = LazyLock::new(|| re(r"\b\d{3}-\d{2}-\d{4}\b"));
static IBAN: LazyLock<Regex> = LazyLock::new(|| re(r"\b[A-Z]{2}\d{2}[ ]?(?:[A-Z0-9]{4}[ ]?){2,7}[A-Z0-9]{1,4}\b"));
static CARD: LazyLock<Regex> = LazyLock::new(|| re(r"\b(?:\d[ -]?){13,19}\b"));
static EMAIL: LazyLock<Regex> = LazyLock::new(|| re(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}"));
static PHONE: LazyLock<Regex> = LazyLock::new(|| re(r"(?:\+|\b00)?\d[\d ()-]{7,}\d"));
static LONG_TOKEN: LazyLock<Regex> = LazyLock::new(|| re(r"\b[A-Za-z0-9+/_=-]{32,}\b"));
static ADDRESS: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)\b(?:home\s+address|my\s+address|lives?\s+at)\b|عنوان(?:ي|ه)?\s+(?:المنزل|البيت)?"));

fn luhn(digits: &str) -> bool {
    let d: Vec<u32> = digits.chars().filter_map(|c| c.to_digit(10)).collect();
    if d.len() < 13 {
        return false;
    }
    d.iter().rev().enumerate().map(|(i, &x)| if i % 2 == 1 { let y = x * 2; if y > 9 { y - 9 } else { y } } else { x }).sum::<u32>() % 10 == 0
}

/// Western digits, so the patterns catch Arabic-Indic numbers too.
fn ascii_digits(t: &str) -> String {
    t.chars().map(|c| if ('٠'..='٩').contains(&c) { char::from_digit(c as u32 - '٠' as u32, 10).unwrap() } else { c }).collect()
}

/// Pattern-based classification. Anything it flags is Sensitive; the reason says which rule fired.
pub fn detect(text: &str) -> Classification {
    let t = ascii_digits(text);
    if KEY_SHAPES.is_match(&t) {
        return Classification::sensitive("credentials: looks like a key or token");
    }
    if CREDENTIAL_WORDS.is_match(&t) {
        return Classification::sensitive("credentials");
    }
    if PRIVATE_MARK.is_match(&t) {
        return Classification::sensitive("marked private");
    }
    if CARD.find_iter(&t).any(|m| luhn(m.as_str())) || IBAN.is_match(&t) {
        return Classification::sensitive("finance: card or account number");
    }
    if SSN.is_match(&t) {
        return Classification::sensitive("identity document number");
    }
    if IDENTITY_WORDS.is_match(&t) {
        return Classification::sensitive("identity documents");
    }
    if HEALTH_WORDS.is_match(&t) {
        return Classification::sensitive("health");
    }
    if FINANCE_WORDS.is_match(&t) {
        return Classification::sensitive("finance");
    }
    // Not certain, so treated as sensitive until the user says otherwise (spec §10.1).
    if EMAIL.is_match(&t) || ADDRESS.is_match(&t) {
        return Classification::sensitive("possible personal identifier (unsure)");
    }
    if PHONE.is_match(&t) || LONG_TOKEN.is_match(&t) {
        return Classification::sensitive("possible number or token (unsure)");
    }
    Classification::normal()
}

fn shingles(text: &str, n: usize) -> std::collections::HashSet<String> {
    let words: Vec<String> = text.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_string).collect();
    if words.len() < n {
        return std::iter::once(words.join(" ")).filter(|s| s.chars().count() >= 12).collect();
    }
    words.windows(n).map(|w| w.join(" ")).collect()
}

/// Would sending `outbound` give away something from `sensitive` (the text of sensitive notes and rules)?
/// A run of four words in the same order is enough; a sensitive text shorter than that must appear whole.
/// Returns the first few words of what matched, for the message to the user.
pub fn leaks(outbound: &str, sensitive: &[String]) -> Option<String> {
    let out = shingles(outbound, 4);
    if out.is_empty() {
        return None;
    }
    let flat = outbound.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" ");
    for s in sensitive {
        let theirs = shingles(s, 4);
        if let Some(hit) = theirs.iter().find(|t| out.contains(*t)) {
            return Some(hit.split(' ').take(3).collect::<Vec<_>>().join(" "));
        }
        let whole = s.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" ");
        if whole.chars().count() >= 12 && flat.contains(&whole) {
            return Some(whole.split(' ').take(3).collect::<Vec<_>>().join(" "));
        }
    }
    None
}

#[derive(Clone, Debug, PartialEq)]
pub enum Judgement {
    Normal,
    Sensitive(String),
    Unsure,
}

/// A judge reads text that patterns did not flag. It must be a model that runs on this machine:
/// asking a cloud model whether a memory is private would send the memory to the cloud.
pub trait Judge: Send + Sync {
    fn judge(&self, text: &str) -> impl std::future::Future<Output = Judgement> + Send;
}

pub struct NoJudge;
impl Judge for NoJudge {
    async fn judge(&self, _: &str) -> Judgement {
        Judgement::Normal
    }
}

/// Patterns first; then the judge for anything the patterns passed. `Unsure` counts as sensitive.
pub async fn classify<J: Judge>(text: &str, judge: &J) -> Classification {
    let d = detect(text);
    if d.privacy == Privacy::Sensitive {
        return d;
    }
    match judge.judge(text).await {
        Judgement::Normal => d,
        Judgement::Sensitive(why) => Classification::sensitive(format!("judged sensitive: {why}")),
        Judgement::Unsure => Classification::sensitive("the model was unsure"),
    }
}

pub const JUDGE_PROMPT: &str = "You decide if a note about a person is sensitive. Sensitive means: passwords, keys or tokens; health; money or accounts; \
identity documents; contact details or home address; anything the person would want kept private. Answer with one word: NORMAL, SENSITIVE or UNSURE, then a colon and at most six words of reason.\n\nNote:\n";

/// Reads the judge model's one-line answer. Anything unreadable is `Unsure`.
pub fn parse_judgement(reply: &str) -> Judgement {
    let first = reply.trim().lines().next().unwrap_or("").trim().trim_matches(|c: char| !c.is_alphanumeric());
    let upper = first.to_uppercase();
    let reason = || first.split_once(':').map(|(_, r)| r.trim().to_string()).unwrap_or_default();
    if upper.starts_with("NORMAL") {
        Judgement::Normal
    } else if upper.starts_with("SENSITIVE") {
        Judgement::Sensitive(reason())
    } else {
        Judgement::Unsure
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sensitive(t: &str) -> bool {
        detect(t).privacy == Privacy::Sensitive
    }

    #[test]
    fn the_default_sensitive_categories_in_english_and_arabic() {
        for t in [
            "My password is hunter2",
            "The API key is sk-abcdefghijklmnop1234",
            "-----BEGIN OPENSSH PRIVATE KEY-----",
            "Token ghp_abcdefghijklmnopqrstuv",
            "The user is allergic to penicillin",
            "Takes metformin for diabetes",
            "Salary is 4200 per month after tax",
            "rent is 1100",
            "Credit card 4111 1111 1111 1111",
            "IBAN DE89 3704 0044 0532 0130 00",
            "Passport number is in the drawer",
            "SSN 123-45-6789",
            "Likes jazz #private",
            "كلمة المرور هي 1234",
            "لديه حساسية من البنسلين",
            "الراتب ٤٢٠٠ شهريا",
            "جواز السفر ينتهي في يونيو",
            "رقم البطاقة ٤١١١ ١١١١ ١١١١ ١١١١",
        ] {
            assert!(sensitive(t), "{t}");
        }
    }

    #[test]
    fn unsure_identifiers_count_as_sensitive() {
        for t in ["Reach me at alex@example.com", "Call +20 100 123 4567", "My address is 12 Palm Street", "The id is 3f9a8c7d6e5b4a3f2e1d0c9b8a7f6e5d4c3b2a1f"] {
            let c = detect(t);
            assert_eq!(c.privacy, Privacy::Sensitive, "{t}");
            assert!(c.reason.contains("unsure") || c.reason.contains("identifier"), "{t}: {}", c.reason);
        }
    }

    #[test]
    fn ordinary_notes_stay_normal() {
        for t in ["Prefers short answers and plain words", "The project uses SQLite for storage", "Flight to Berlin on 14 June", "Wants to learn Arabic calligraphy", "Mother's birthday is 9 October", "يفضل الإجابات القصيرة", "المشروع يستخدم SQLite", "A 12-digit order number 123456 was shipped", "Version 1.2.3 released"] {
            assert!(!sensitive(t), "{t}: {:?}", detect(t));
        }
    }

    #[test]
    fn card_numbers_need_a_valid_check_digit() {
        assert!(detect("4111 1111 1111 1111").reason.starts_with("finance"));
        // a long number that is not a card is still caught, but only as "unsure"
        assert!(detect("1234 5678 9012 3456 is a ticket id").reason.contains("unsure"));
    }

    struct Fixed(Judgement);
    impl Judge for Fixed {
        async fn judge(&self, _: &str) -> Judgement {
            self.0.clone()
        }
    }

    #[tokio::test]
    async fn the_model_can_add_judgment_and_unsure_means_sensitive() {
        let t = "Gets very nervous before every flight";
        assert_eq!(classify(t, &NoJudge).await.privacy, Privacy::Normal);
        assert_eq!(classify(t, &Fixed(Judgement::Sensitive("health".into()))).await.reason, "judged sensitive: health");
        assert_eq!(classify(t, &Fixed(Judgement::Unsure)).await.privacy, Privacy::Sensitive);
        assert_eq!(classify(t, &Fixed(Judgement::Normal)).await.privacy, Privacy::Normal);
        // patterns win without asking the judge
        assert_eq!(classify("password: x", &Fixed(Judgement::Normal)).await.privacy, Privacy::Sensitive);
    }

    #[test]
    fn judge_replies_are_read_strictly() {
        assert_eq!(parse_judgement("NORMAL: just a hobby"), Judgement::Normal);
        assert_eq!(parse_judgement("Sensitive: health condition"), Judgement::Sensitive("health condition".into()));
        assert_eq!(parse_judgement("**SENSITIVE**: money"), Judgement::Sensitive("money".into()));
        assert_eq!(parse_judgement("UNSURE"), Judgement::Unsure);
        assert_eq!(parse_judgement("I think this is fine"), Judgement::Unsure);
        assert_eq!(parse_judgement(""), Judgement::Unsure);
    }

    #[test]
    fn trust_levels_gate_sensitive_pages() {
        assert!(Privacy::Normal.allowed_at(Trust::AnyCloud));
        assert!(Privacy::Sensitive.allowed_at(Trust::Local));
        assert!(!Privacy::Sensitive.allowed_at(Trust::OwnCloud));
        assert!(!Privacy::Sensitive.allowed_at(Trust::AnyCloud));
        assert_eq!(Trust::parse("local"), Some(Trust::Local));
        assert_eq!(Trust::parse("nonsense"), None);
    }
}

#[cfg(test)]
mod leak_tests {
    use super::leaks;

    #[test]
    fn a_run_of_words_from_a_sensitive_text_is_a_leak_and_unrelated_text_is_not() {
        let sens = vec!["My doctor is Dr. Amal and she prescribed metformin for my diabetes.".to_string(), "Takes metformin".to_string()];
        assert!(leaks("search for: doctor is Dr. Amal and she", &sens).is_some());
        assert!(leaks("https://x.example/?q=she prescribed metformin for my", &sens).is_some());
        assert!(leaks("what about Takes metformin?", &sens).is_some(), "a short text must appear whole");
        assert!(leaks("how do I make sourdough bread at home", &sens).is_none());
        assert!(leaks("metformin side effects", &sens).is_none(), "a general question is the user's own to ask");
        assert!(leaks("", &sens).is_none());
        assert!(leaks("anything", &[]).is_none());
    }
}
