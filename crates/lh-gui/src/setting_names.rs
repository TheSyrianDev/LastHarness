//! Plain names for the settings the core lists. The core knows each setting by a key such as `kvcache.max_gb`
//! and a raw value such as `ask`. A person should see "Most disk for saved chat memory" and "Ask each time".
//! This module holds, for every setting, its category, a title, one sentence and the words for its values.
//! It draws nothing, so it is tested without a window. A key the table does not know yet still gets a readable
//! title and the category Other, so a new setting never disappears or shows a bare variable name.

/// A page of the Settings window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    /// The app's own look and layout. Not a core setting; it works when the core is away.
    Appearance,
    Models,
    Chats,
    Memory,
    /// Rules, the style text and skills. No core setting lives here; the page is built from its own calls.
    Learning,
    Departments,
    Permissions,
    Spending,
    Web,
    /// MCP servers and folder trust. Not core settings; the page is drawn by the app.
    Tools,
    Channels,
    Data,
    Activity,
    /// Commands before and after tool calls, from hooks.toml.
    Hooks,
    /// Tasks that run on a timetable.
    Schedules,
    Updates,
    /// Settings the core has that this table does not name yet.
    Other,
}

impl Category {
    pub const ALL: [Category; 17] = [
        Category::Appearance,
        Category::Models,
        Category::Chats,
        Category::Memory,
        Category::Learning,
        Category::Departments,
        Category::Permissions,
        Category::Spending,
        Category::Web,
        Category::Tools,
        Category::Channels,
        Category::Data,
        Category::Activity,
        Category::Hooks,
        Category::Schedules,
        Category::Updates,
        Category::Other,
    ];

    /// What is saved in `gui.toml` to remember the page.
    pub fn id(self) -> &'static str {
        match self {
            Category::Appearance => "appearance",
            Category::Models => "models",
            Category::Chats => "chats",
            Category::Memory => "memory",
            Category::Learning => "learning",
            Category::Departments => "departments",
            Category::Permissions => "permissions",
            Category::Spending => "spending",
            Category::Web => "web",
            Category::Tools => "tools",
            Category::Channels => "channels",
            Category::Data => "data",
            Category::Activity => "activity",
            Category::Hooks => "hooks",
            Category::Schedules => "schedules",
            Category::Updates => "updates",
            Category::Other => "other",
        }
    }

    /// A saved id back to a page. Anything unknown is the first page.
    pub fn from_id(id: &str) -> Category {
        Category::ALL.into_iter().find(|c| c.id() == id).unwrap_or(Category::Appearance)
    }

    pub fn title(self) -> &'static str {
        match self {
            Category::Appearance => "Appearance",
            Category::Models => "Models and keys",
            Category::Chats => "Chats and speed",
            Category::Memory => "Memory",
            Category::Learning => "Learning",
            Category::Departments => "Departments",
            Category::Permissions => "Permissions",
            Category::Spending => "Spending",
            Category::Web => "Web and phone",
            Category::Tools => "Tools and MCP",
            Category::Channels => "Channels",
            Category::Data => "Your data",
            Category::Activity => "Activity",
            Category::Hooks => "Hooks",
            Category::Schedules => "Scheduled tasks",
            Category::Updates => "Updates and reports",
            Category::Other => "More",
        }
    }

    /// One line under the page title.
    pub fn blurb(self) -> &'static str {
        match self {
            Category::Appearance => "How the window looks and how wide the chat is",
            Category::Models => "Where your models come from, and the key to reach them",
            Category::Chats => "How chats are kept, shortened and resumed",
            Category::Memory => "What the app remembers about you, and how much of it goes with a message",
            Category::Learning => "Rules, how I talk to you, and skills, with a way back for each change",
            Category::Departments => "Departments, their roles and model types, and how they work on a big goal",
            Category::Permissions => "What agents may do without asking. Take care here.",
            Category::Spending => "What happens when a model service costs money",
            Category::Web => "Web search, and reaching the app from a browser or a phone",
            Category::Tools => "Servers that give the assistant more tools, and folders that may bring their own",
            Category::Channels => "Talk to the assistant from Telegram, Matrix, Signal or Slack. All are off until you switch them on.",
            Category::Data => "Take your memory, rules and roles with you, and start with the computer",
            Category::Activity => "What the assistant did, newest first, and what it cost",
            Category::Hooks => "Your own commands, run before or after the assistant uses a tool",
            Category::Schedules => "Tasks the assistant starts on its own, on a timetable",
            Category::Updates => "Updates and crash reports",
            Category::Other => "Settings that have no page of their own yet",
        }
    }
}

/// How a value is written for a person.
#[derive(Clone, Copy, Debug)]
pub enum Show {
    /// "On" and "Off".
    OnOff,
    /// Each stored word and the words shown for it.
    Words(&'static [(&'static str, &'static str)]),
    /// A whole number with its unit, one and many: "1 worker", "3 workers".
    Count(&'static str, &'static str),
    /// A number of gigabytes.
    Gb,
    /// US dollars; empty means no limit.
    Money,
    /// Seconds written as seconds, minutes or hours; 0 means never.
    Duration,
    /// A number of tokens, `auto` or 0.
    Tokens,
    /// Free text; the words shown when it is empty.
    Text(&'static str),
}

#[derive(Clone, Copy, Debug)]
pub struct Info {
    pub key: &'static str,
    pub category: Category,
    pub title: &'static str,
    /// One short sentence: what it does and when it matters.
    pub about: &'static str,
    pub show: Show,
    /// Settings that can cost money, open a way in, or skip a safety question. They are marked in the window.
    pub careful: bool,
}

const fn i(key: &'static str, category: Category, title: &'static str, about: &'static str, show: Show, careful: bool) -> Info {
    Info { key, category, title, about, show, careful }
}

use Category::*;

/// Every setting in the core's list, once.
pub const INFO: &[Info] = &[
    i("provider.url", Models, "Model server address", "Where your models are served from, for example http://localhost:9292/v1.", Show::Text("Not set"), false),
    i("provider.name", Models, "Name of the model service", "Told to the model so it knows what serves it. Leave it empty to use the name the server gives.", Show::Text("Use the server's own name"), false),
    i("provider.key", Models, "Key for the model service", "Points to the stored secret that holds your key. The key itself is never written in the settings file.", Show::Text("None"), false),
    i("media.image_model", Models, "Image model", "The model that makes images. Leave it empty to turn image making off.", Show::Text("Off"), false),
    i("memory.enabled", Memory, "Remember between chats", "Keeps notes about you and your work so later chats can use them.", Show::OnOff, false),
    i("memory.trust", Memory, "Where memory may be sent", "Sensitive notes never leave this computer. This sets how far the model service may see the rest.", Show::Words(&[("auto", "Decide for me"), ("local", "This computer only"), ("own", "My own cloud account"), ("any", "Any cloud service")]), false),
    i("memory.recall_items", Memory, "Notes added to a message", "The most notes memory adds to one message you send.", Show::Count("note", "notes"), false),
    i("memory.recall_chars", Memory, "Memory added to a message, in characters", "The most text, all notes together, that memory adds to one message you send.", Show::Count("character", "characters"), false),
    i("learning.mode", Learning, "Learning from your habits", "How the app picks up your habits. It can suggest and wait for you, apply changes quietly, or not learn.", Show::Words(&[("propose", "Suggest and wait"), ("silent", "Apply quietly"), ("off", "Off")]), false),
    i("learning.threshold", Learning, "Chats before it counts as a habit", "How many chats a wish must come up in before the app treats it as a habit.", Show::Count("chat", "chats"), false),
    i("team.autonomy", Departments, "How much departments ask you", "How often a department checks with you while it works on a big goal.", Show::Words(&[("plan-first", "Show a plan first"), ("check-ins", "Check in as it goes"), ("fire-and-forget", "Just do it")]), false),
    i("team.parallel", Departments, "Workers at the same time", "How many workers a department may run at once.", Show::Count("worker", "workers"), false),
    i("team.attempts", Departments, "Tries for each step", "How many times a step is tried before the failure is passed up to you.", Show::Count("try", "tries"), false),
    i("team.unavailable", Departments, "When a model is missing", "Stop and tell you, or try the backup models listed for that kind of job.", Show::Words(&[("stop", "Stop and tell me"), ("fallback", "Try a backup model")]), false),
    i("web.listen", Web, "Address for the browser and phone", "Lets you use LastHarness in a browser over HTTPS, for example at 127.0.0.1:7843. Every device must be paired first. Leave it empty to keep this off.", Show::Text("Off"), true),
    i("web.tailscale", Web, "Reach it from your phone over Tailscale", "Also opens the browser page on this computer's Tailscale address, so a phone on your tailnet can use it. It never listens on other networks. Sign the phone in under Phone. Needs a restart.", Show::OnOff, true),
    i("tools.codemode", Tools, "Let the model write small scripts", "The model can run a short script that calls its other tools and keeps only what matters from long results. Every call inside is checked like any other. Off by default: it adds one tool to every request.", Show::OnOff, false),
    i("tools.sandbox", Tools, "Keep commands in the chat's folder", "Commands the model runs can change files only in the chat's folder and a private temporary folder. They can still read the rest, and network access is set under Permissions. Needs bubblewrap.", Show::OnOff, true),
    i("web.use_proxy", Web, "Use the system proxy for web pages", "A proxy looks up names itself, so the check against private addresses cannot see where a request goes. This is off unless you turn it on.", Show::OnOff, true),
    i("web.searxng", Web, "Web search server", "The address of a SearXNG server used for web search. Leave it empty to turn search off.", Show::Text("Off"), false),
    i("spending.mode", Spending, "What happens as costs add up", "Only record what you spend, stop at your limits, or ask before a call that looks expensive.", Show::Words(&[("none", "Only record"), ("caps", "Stop at my limits"), ("estimate", "Ask before costly calls")]), true),
    i("spending.approve_over_usd", Spending, "Ask before a call over", "When asking is on, you are asked before one call that is expected to cost more than this.", Show::Money, true),
    i("spending.per_request_cap_usd", Spending, "Most for one call", "Refuses any single call that would cost more than this.", Show::Money, true),
    i("spending.daily_cap_usd", Spending, "Most per day", "Refuses further calls once a day's spending passes this.", Show::Money, true),
    i("spending.monthly_cap_usd", Spending, "Most per month", "Refuses further calls once a month's spending passes this.", Show::Money, true),
    i("permissions.mode", Permissions, "When to ask permission", "Reads are free, writes ask once, and risky things always ask. Or ask for everything.", Show::Words(&[("tiered", "Ask for risky things"), ("ask-all", "Ask for everything")]), true),
    i("permissions.bypass", Permissions, "Skip all permission questions", "Turns every permission question off. You accept the risk, and the project is not responsible for the outcome.", Show::OnOff, true),
    i("permissions.show_diff", Permissions, "Show the changes of a file write", "When a write is allowed without asking, the chat shows what changed. Choose where that happens.", Show::Words(&[("always", "Always"), ("untrusted", "In folders I have not trusted"), ("trusted", "In trusted folders"), ("never", "Never")]), false),
    i("tools.sandbox_network", Permissions, "Network for sandboxed commands", "Whether commands in the sandbox may use the network. Ask each run, allow only the sites listed below, always allow, or let a small model decide and ask you when it cannot.", Show::Words(&[("ask", "Ask each time"), ("allowlist", "Only listed sites"), ("on", "Always"), ("auto", "Let a model decide")]), true),
    i("tools.sandbox_network_allow", Permissions, "Sites sandboxed commands may reach", "Used when only listed sites are allowed. Separate them with commas, for example github.com, crates.io. A site covers its subdomains.", Show::Text("None"), false),
    i("tools.sandbox_network_model", Permissions, "Model that decides network access", "A small, fast model that decides each run when a model decides. Leave it empty and you are asked instead.", Show::Text("Not set, so you are asked"), false),
    i("permissions.sudo", Permissions, "Allow sudo", "Lets agents run commands with sudo. It still asks like any other command.", Show::OnOff, true),
    i("tree.summarize", Chats, "Summary when you leave a branch", "When you move to another branch of a chat, a short summary of the branch you left can be written where you come back.", Show::Words(&[("ask", "Ask each time"), ("always", "Always"), ("never", "Never")]), false),
    i("kvcache.enabled", Chats, "Saved chat memory for faster resume", "Saves the local model's work on a chat so a stored chat goes on quickly instead of being read again. It needs the folder below, and is never used for private chats or cloud models.", Show::OnOff, false),
    i("kvcache.dir", Chats, "Folder for saved chat memory", "The folder your local model server saves into. With llama-swap, it is the folder that holds the per-model ones. Leave it empty to turn saving off.", Show::Text("Not set, so saving is off"), false),
    i("kvcache.max_gb", Chats, "Most disk for saved chat memory", "The most disk the saved chat memory may use in total. The oldest is removed first.", Show::Gb, false),
    i("memory.check_model", Memory, "Model that checks notes", "While the app is idle, a model on this computer looks for notes that disagree and suggests archiving the older one. You accept or reject each suggestion.", Show::Text("Not set, so notes are not checked"), false),
    i("chats.title_model", Chats, "Model that names chats", "A small, fast model that gives a chat a short name after the first reply. Leave it empty to use the first words of your first message. Private chats are never named by a model.", Show::Text("Not set, so the first words are used"), false),
    i("core.idle_after", Chats, "Sleep after being idle for", "How long the core waits without any activity before it goes idle. Never means it stays awake.", Show::Duration, false),
    i("core.compact_at", Chats, "Shorten a long chat at", "Long conversations are shortened at about this size. Automatic means three quarters of what the model can hold. Never means no shortening.", Show::Tokens, false),
    i("reports.crash", Updates, "Crash reports", "Anonymous reports when something breaks. You can send none, see exactly what would be sent and decide, or send them automatically.", Show::Words(&[("off", "Never"), ("ask", "Ask first"), ("auto", "Send automatically")]), false),
    i("updates.enabled", Updates, "Check for updates", "Looks for signed updates.", Show::OnOff, false),
];

/// Is the setting at a value that opens a way in, skips a safety question or stops the limits? Only then is it
/// marked in red. The same setting at its safe value is just a setting.
pub fn is_risky(key: &str, value: &str) -> bool {
    let v = value.trim();
    match key {
        "permissions.bypass" | "permissions.sudo" | "web.use_proxy" => v == "true",
        "tools.sandbox" => v == "false",
        "tools.sandbox_network" => v == "on",
        "spending.mode" => v == "none",
        "web.listen" => !v.is_empty() && !is_local_address(v),
        _ => false,
    }
}

/// An address that only this computer can reach: localhost or a loopback number, with or without a port.
pub fn is_local_address(addr: &str) -> bool {
    let a = addr.trim().trim_start_matches("https://").trim_start_matches("http://");
    let host = if let Some(rest) = a.strip_prefix('[') {
        rest.split(']').next().unwrap_or("")
    } else if a.matches(':').count() > 1 {
        a
    } else {
        a.split(':').next().unwrap_or("")
    };
    let host = host.trim_end_matches('/');
    host.eq_ignore_ascii_case("localhost") || host == "::1" || (host.starts_with("127.") && host.split('.').count() == 4 && host.split('.').all(|p| p.parse::<u8>().is_ok()))
}

/// Does any setting in this page sit at a risky value right now? `settings` are (key, value).
pub fn page_is_risky<'a>(page: Category, settings: impl IntoIterator<Item = (&'a str, &'a str)>) -> bool {
    settings.into_iter().any(|(k, v)| describe(k, "").category == page && is_risky(k, v))
}

pub fn info(key: &str) -> Option<&'static Info> {
    INFO.iter().find(|i| i.key == key)
}

/// A readable title for a key the table does not know: "kvcache.max_gb" becomes "Kvcache max gb".
fn humanize(key: &str) -> String {
    let words = key.replace(['.', '_', '-'], " ");
    let mut chars = words.trim().chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => "Setting".to_string(),
    }
}

/// What the window shows for one setting.
#[derive(Clone, Debug, PartialEq)]
pub struct Described {
    pub category: Category,
    pub title: String,
    pub about: String,
    pub careful: bool,
}

/// Names for a setting. `help` is the core's own sentence, used only when the table has nothing for the key.
pub fn describe(key: &str, help: &str) -> Described {
    match info(key) {
        Some(i) => Described { category: i.category, title: i.title.into(), about: i.about.into(), careful: i.careful },
        None => Described { category: Category::Other, title: humanize(key), about: help.into(), careful: false },
    }
}

fn plural(n: i64, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

fn duration(secs: i64) -> String {
    match secs {
        0 => "Never".into(),
        s if s % 3600 == 0 => plural(s / 3600, "hour", "hours"),
        s if s % 60 == 0 => plural(s / 60, "minute", "minutes"),
        s => plural(s, "second", "seconds"),
    }
}

/// The stored value as words a person reads. A value the table cannot read is shown as it is.
pub fn value_text(key: &str, raw: &str) -> String {
    let raw = raw.trim();
    let Some(i) = info(key) else {
        return if raw.is_empty() { "Not set".into() } else { raw.into() };
    };
    let number = raw.parse::<i64>().ok();
    match i.show {
        Show::OnOff => match raw {
            "true" => "On".into(),
            "false" => "Off".into(),
            other => other.into(),
        },
        Show::Words(w) => w.iter().find(|(k, _)| *k == raw).map(|(_, v)| (*v).to_string()).unwrap_or_else(|| raw.into()),
        Show::Count(one, many) => number.map(|n| plural(n, one, many)).unwrap_or_else(|| raw.into()),
        Show::Gb => number.map(|n| format!("{n} GB")).unwrap_or_else(|| raw.into()),
        Show::Money => {
            if raw.is_empty() {
                "No limit".into()
            } else {
                raw.parse::<f64>().map(|f| format!("${f:.2}")).unwrap_or_else(|_| raw.into())
            }
        }
        Show::Duration => number.map(duration).unwrap_or_else(|| raw.into()),
        Show::Tokens => match (raw, number) {
            ("auto", _) => "Automatic".into(),
            (_, Some(0)) => "Never".into(),
            (_, Some(n)) => format!("{} tokens", thousands(n)),
            _ => raw.into(),
        },
        Show::Text(empty) => {
            if raw.is_empty() {
                empty.into()
            } else {
                raw.into()
            }
        }
    }
}

fn thousands(n: i64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Whether a setting matches words typed in the search box: every word must be in the title, the sentence, the
/// page name or the key (so a person who knows the file name can still find it).
pub fn matches(key: &str, d: &Described, query: &str) -> bool {
    let hay = format!("{} {} {} {}", d.title, d.about, d.category.title(), key).to_lowercase();
    query.split_whitespace().all(|w| hay.contains(&w.to_lowercase()))
}

/// Words that find the Appearance page in the search box: the controls on it are drawn by the app, not listed by the core.
pub const APPEARANCE_WORDS: &str = "appearance name greeting hello web page pages html browser inside look theme mode light dark system direction left right language chat width wide column preset slider size chat list font fonts latin arabic naskh";

pub fn appearance_matches(query: &str) -> bool {
    query.split_whitespace().all(|w| APPEARANCE_WORDS.contains(&w.to_lowercase()))
}

impl Category {
    /// Pages the app draws itself, listed even when the core has no setting for them.
    pub fn always_listed(self) -> bool {
        matches!(self, Category::Appearance | Category::Tools | Category::Channels | Category::Data | Category::Activity | Category::Hooks | Category::Schedules)
    }
}

/// The pages to list in the sidebar: the pages the app draws itself, then every other page that has at least one
/// setting, in the usual order. Learning has no settings of its own and shows whenever the core is there. `keys` are the keys the core listed.
pub fn pages<'a>(keys: impl IntoIterator<Item = &'a str>) -> Vec<Category> {
    let used: Vec<Category> = keys.into_iter().map(|k| describe(k, "").category).collect();
    Category::ALL.into_iter().filter(|c| c.always_listed() || used.contains(c) || (*c == Category::Learning && !used.is_empty())).collect()
}

/// The page above or below this one in the sidebar, stopping at the ends.
pub fn neighbour(pages: &[Category], current: Category, down: bool) -> Category {
    let at = pages.iter().position(|c| *c == current).unwrap_or(0);
    let next = if down { (at + 1).min(pages.len().saturating_sub(1)) } else { at.saturating_sub(1) };
    pages.get(next).copied().unwrap_or(Category::Appearance)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_setting_the_core_lists_has_exactly_one_page_and_nothing_extra_is_named() {
        for d in lh_config::SCHEMA {
            let n = INFO.iter().filter(|i| i.key == d.key).count();
            assert_eq!(n, 1, "{} must be named exactly once", d.key);
            let described = describe(d.key, d.help);
            assert_ne!(described.category, Category::Other, "{} has no page", d.key);
            assert_ne!(described.category, Category::Appearance, "{} is a core setting, not an appearance one", d.key);
        }
        for i in INFO {
            assert!(lh_config::def(i.key).is_some(), "{} is named but the core has no such setting", i.key);
        }
        assert_eq!(INFO.len(), lh_config::SCHEMA.len());
        // each setting is listed under one page only
        let mut total = 0;
        for c in Category::ALL {
            total += INFO.iter().filter(|i| i.category == c).count();
        }
        assert_eq!(total, lh_config::SCHEMA.len());
    }

    #[test]
    fn no_title_is_the_raw_key_and_every_name_and_sentence_is_filled_in() {
        for i in INFO {
            assert!(!i.title.trim().is_empty() && !i.about.trim().is_empty(), "{}", i.key);
            assert_ne!(i.title, i.key);
            assert!(!i.title.contains('.') && !i.title.contains('_'), "{}: {}", i.key, i.title);
            for d in lh_config::SCHEMA {
                assert!(!i.title.contains(d.key) && !i.about.contains(d.key), "{} shows the key {}", i.key, d.key);
            }
            assert!(!i.about.contains('_') && !i.about.contains('—'), "{}: {}", i.key, i.about);
            assert!(i.about.ends_with('.'), "{}: one sentence ending in a full stop", i.key);
            assert!(i.about.split(". ").count() <= 3, "{}: keep it short", i.key);
            let first = i.title.chars().next().unwrap();
            assert!(first.is_uppercase(), "{}", i.title);
        }
        for c in Category::ALL {
            assert!(!c.title().is_empty() && !c.blurb().is_empty());
            assert_eq!(Category::from_id(c.id()), c, "the saved id comes back");
        }
        assert_eq!(Category::from_id("nonsense"), Category::Appearance);
        assert_eq!(Category::from_id(""), Category::Appearance);
    }

    #[test]
    fn values_are_words_not_raw_values() {
        for d in lh_config::SCHEMA {
            let default_text = value_text(d.key, d.default);
            assert!(!default_text.trim().is_empty(), "{} shows nothing for its default", d.key);
            match d.kind {
                lh_config::Kind::Bool => {
                    for (raw, want) in [("true", "On"), ("false", "Off")] {
                        assert_eq!(value_text(d.key, raw), want, "{}", d.key);
                    }
                }
                lh_config::Kind::Choice(opts) => {
                    let mut shown = vec![];
                    for o in opts {
                        let t = value_text(d.key, o);
                        assert_ne!(t, *o, "{}: {o} has no words", d.key);
                        assert!(!t.contains('-') || t.contains(' '), "{}: {t}", d.key);
                        shown.push(t);
                    }
                    shown.sort();
                    shown.dedup();
                    assert_eq!(shown.len(), opts.len(), "{}: two options read the same", d.key);
                }
                _ => {}
            }
        }
        assert_eq!(value_text("tree.summarize", "ask"), "Ask each time");
        assert_eq!(value_text("tree.summarize", "always"), "Always");
        assert_eq!(value_text("tree.summarize", "never"), "Never");
        assert_eq!(value_text("kvcache.enabled", "true"), "On");
        assert_eq!(value_text("kvcache.max_gb", "5"), "5 GB");
        assert_eq!(value_text("team.parallel", "1"), "1 worker");
        assert_eq!(value_text("team.parallel", "3"), "3 workers");
        assert_eq!(value_text("core.idle_after", "600"), "10 minutes");
        assert_eq!(value_text("core.idle_after", "0"), "Never");
        assert_eq!(value_text("core.idle_after", "3600"), "1 hour");
        assert_eq!(value_text("core.idle_after", "45"), "45 seconds");
        assert_eq!(value_text("core.compact_at", "auto"), "Automatic");
        assert_eq!(value_text("core.compact_at", "0"), "Never");
        assert_eq!(value_text("core.compact_at", "128000"), "128,000 tokens");
        assert_eq!(value_text("spending.daily_cap_usd", ""), "No limit");
        assert_eq!(value_text("spending.approve_over_usd", "0.25"), "$0.25");
        assert_eq!(value_text("kvcache.dir", ""), "Not set, so saving is off");
        assert_eq!(value_text("web.listen", ""), "Off");
        assert_eq!(value_text("provider.url", "http://localhost:9292/v1"), "http://localhost:9292/v1");
        assert_eq!(value_text("memory.trust", "own"), "My own cloud account");
        // a word the table does not know is shown as it is, never hidden
        assert_eq!(value_text("tree.summarize", "weekly"), "weekly");
        assert_eq!(value_text("new.thing", ""), "Not set");
        assert_eq!(value_text("new.thing", "x"), "x");
    }

    #[test]
    fn a_setting_the_table_does_not_know_still_shows_with_a_readable_title() {
        let d = describe("brand.new_key", "Does a new thing.");
        assert_eq!((d.category, d.title.as_str(), d.about.as_str()), (Category::Other, "Brand new key", "Does a new thing."));
        assert_eq!(describe("", "").title, "Setting");
        assert!(pages(["brand.new_key"]).contains(&Category::Other));
    }

    #[test]
    fn the_risky_settings_are_marked_and_their_warnings_are_kept() {
        for key in ["permissions.mode", "permissions.bypass", "permissions.sudo", "spending.mode", "spending.daily_cap_usd", "web.listen", "web.use_proxy"] {
            assert!(describe(key, "").careful, "{key}");
        }
        assert!(!describe("memory.enabled", "").careful);
        let bypass = describe("permissions.bypass", "");
        assert!(bypass.about.contains("accept the risk") && bypass.about.contains("not responsible"));
        assert!(describe("permissions.sudo", "").about.contains("still asks"));
        assert!(describe("kvcache.enabled", "").about.contains("never used for private chats"));
        assert!(describe("web.listen", "").about.contains("paired"));
    }

    #[test]
    fn the_sidebar_lists_the_pages_that_have_settings_and_up_down_walks_them() {
        let keys: Vec<&str> = lh_config::SCHEMA.iter().map(|d| d.key).collect();
        let p = pages(keys.iter().copied());
        assert_eq!(p[0], Category::Appearance);
        assert_eq!(p.len(), 16, "nine pages with settings, Learning, six the app draws, and More only when something is unnamed: {p:?}");
        assert!(!p.contains(&Category::Other));
        assert_eq!(pages(std::iter::empty()), vec![Category::Appearance, Category::Tools, Category::Channels, Category::Data, Category::Activity, Category::Hooks, Category::Schedules], "with the core away only the pages the app draws show");
        assert_eq!(neighbour(&p, Category::Appearance, false), Category::Appearance, "stops at the top");
        assert_eq!(neighbour(&p, Category::Appearance, true), p[1]);
        assert_eq!(neighbour(&p, *p.last().unwrap(), true), *p.last().unwrap(), "stops at the bottom");
        assert_eq!(neighbour(&p, p[3], false), p[2]);
        assert_eq!(neighbour(&[], Category::Models, true), Category::Appearance);
        // permissions are no more than a few steps from the top
        assert!(p.iter().position(|c| *c == Category::Permissions).unwrap() <= 6);
    }

    #[test]
    fn only_a_risky_value_is_marked() {
        assert!(is_risky("permissions.bypass", "true") && !is_risky("permissions.bypass", "false"));
        assert!(is_risky("permissions.sudo", "true") && !is_risky("permissions.sudo", "false"));
        assert!(is_risky("web.use_proxy", "true") && !is_risky("web.use_proxy", "false"));
        assert!(is_risky("spending.mode", "none") && !is_risky("spending.mode", "caps") && !is_risky("spending.mode", "estimate"));
        assert!(!is_risky("web.listen", "") && !is_risky("web.listen", "127.0.0.1:7843") && !is_risky("web.listen", "localhost:7843") && !is_risky("web.listen", "[::1]:7843"));
        assert!(is_risky("web.listen", "0.0.0.0:7843") && is_risky("web.listen", "192.168.1.5:7843") && is_risky("web.listen", "myhost.local:1"));
        assert!(!is_risky("spending.daily_cap_usd", "100"), "a careful setting is not risky by itself");
        assert!(!is_risky("provider.url", "anything"));
    }

    #[test]
    fn local_addresses_are_recognised() {
        for a in ["127.0.0.1", "127.0.0.1:80", "127.5.5.5:1", "localhost", "LOCALHOST:9", "[::1]:7843", "::1", "https://127.0.0.1:7843"] {
            assert!(is_local_address(a), "{a}");
        }
        for a in ["0.0.0.0:1", "127.0.0.1.evil.com:1", "10.0.0.2", "[::]:1", "127.1"] {
            assert!(!is_local_address(a), "{a}");
        }
    }

    #[test]
    fn a_page_shows_its_dot_only_while_a_risky_value_is_on() {
        let safe = [("permissions.bypass", "false"), ("permissions.sudo", "false"), ("spending.mode", "estimate")];
        assert!(!page_is_risky(Category::Permissions, safe) && !page_is_risky(Category::Spending, safe));
        let risky = [("permissions.sudo", "true"), ("spending.mode", "none"), ("web.listen", "0.0.0.0:1")];
        assert!(page_is_risky(Category::Permissions, risky) && page_is_risky(Category::Spending, risky) && page_is_risky(Category::Web, risky));
        assert!(!page_is_risky(Category::Models, risky));
    }

    #[test]
    fn search_finds_by_title_sentence_page_or_key() {
        let d = describe("kvcache.max_gb", "");
        assert!(matches("kvcache.max_gb", &d, "disk"));
        assert!(matches("kvcache.max_gb", &d, "SAVED memory"));
        assert!(matches("kvcache.max_gb", &d, "kvcache"), "a person who knows the file name still finds it");
        assert!(matches("kvcache.max_gb", &d, "chats and speed"), "by page name");
        assert!(matches("kvcache.max_gb", &d, ""), "nothing typed matches everything");
        assert!(!matches("kvcache.max_gb", &d, "spending"));
        assert!(appearance_matches("dark") && appearance_matches("chat width") && !appearance_matches("sudo") && appearance_matches(""));
        assert!(appearance_matches("web pages") && appearance_matches("html browser"), "the web page switch is on the Appearance page");
        // the words for permissions find the risky ones
        let hits: Vec<&str> = lh_config::SCHEMA.iter().filter(|s| matches(s.key, &describe(s.key, s.help), "sudo")).map(|s| s.key).collect();
        assert!(hits.contains(&"permissions.sudo"), "{hits:?}");
    }
}
