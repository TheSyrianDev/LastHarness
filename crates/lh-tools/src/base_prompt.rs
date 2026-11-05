//! The built-in base prompt: what LastHarness is, what the agent's job is, where it works, what it may
//! do without asking, and how to use memory and skills. It always comes first in the system message.
//!
//! It is built once when a session starts, from facts that do not change during the session, so its
//! text is identical on every turn and the server's prefix cache holds. Learning never touches it
//! (rules, the style profile and memory are separate blocks that come after it). Role prompts in
//! departments extend it: the same base, then the role's own instructions.

use std::path::Path;

use crate::Mode;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionKind {
    /// Plain conversation: no tools, no folder.
    Chat,
    /// A general assistant that starts in the home folder.
    Assistant,
    /// Work inside one project folder.
    Project,
}

/// Who the model is, for the first line. Every part comes from what is known; nothing is guessed, and a
/// part that is not known is left out.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Identity {
    /// The session's model id (a path or a file extension is stripped when the line is written).
    pub model: Option<String>,
    /// Parameter count, when the server reports it.
    pub parameters: Option<u64>,
    /// The service behind the endpoint: llama-swap, OpenRouter, LM Studio, or a name the user gave.
    pub provider: Option<String>,
}

/// The model id as a person would write it: last path part, file extension removed.
pub fn display_name(id: &str) -> String {
    let last = id.trim().rsplit(['/', '\\']).next().unwrap_or("").trim();
    for ext in [".gguf", ".safetensors", ".bin", ".onnx"] {
        if let Some(stem) = last.strip_suffix(ext) {
            return stem.to_string();
        }
    }
    last.to_string()
}

/// `34.7B`, `2.7B`, `270M`.
pub fn format_params(n: u64) -> String {
    if n >= 1_000_000_000 {
        let b = format!("{:.1}", n as f64 / 1e9);
        format!("{}B", b.trim_end_matches('0').trim_end_matches('.'))
    } else {
        format!("{}M", (n as f64 / 1e6).round() as u64)
    }
}

/// The first sentence of the prompt, from four complete templates, never with an empty slot. A known
/// provider is named as an endpoint, not the model's maker, so the model does not credit it with making it.
pub fn identity_line(i: &Identity) -> String {
    let model = i.model.as_deref().map(display_name).filter(|m| !m.is_empty());
    let provider = i.provider.as_deref().map(str::trim).filter(|p| !p.is_empty());
    let sized = |m: &str| match i.parameters.filter(|n| *n > 0) {
        Some(n) => format!("{m} ({})", format_params(n)),
        None => m.to_string(),
    };
    match (model, provider) {
        (Some(m), Some(p)) => format!("You are {}, served by {p}, an endpoint, not the model's maker.", sized(&m)),
        (Some(m), None) => format!("You are {}.", sized(&m)),
        (None, Some(p)) => format!("You are an AI model, served by {p}, an endpoint, not the model's maker."),
        (None, None) => "You are an AI assistant.".to_string(),
    }
}

pub struct BaseContext<'a> {
    pub identity: Identity,
    pub kind: SessionKind,
    pub folder: Option<&'a Path>,
    pub permissions: Mode,
    pub sudo_enabled: bool,
    /// `YYYY-MM-DD`. A date, not a time: it must not change within a session.
    pub date: &'a str,
    /// One short line from each enabled tool (see `ToolSet::guidelines`). A tool that is off adds nothing.
    pub guidelines: &'a [String],
}

/// Aim for this many tokens for the base text alone, and never go over the cap for base plus tool lines.
/// A long system prompt raises the time to the first token on a local model. Tokens are estimated.
pub const TARGET_TOKENS: usize = 160;
pub const CAP_TOKENS: usize = 300;

/// A rough token count: about 3.5 characters each.
pub fn estimate_tokens(s: &str) -> usize {
    (s.chars().count() as f64 / 3.5).ceil() as usize
}

/// Today's date in UTC as `YYYY-MM-DD`.
pub fn today() -> String {
    let days = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() / 86_400).unwrap_or(0) as i64;
    ymd(days)
}

/// Days since 1970-01-01 to `YYYY-MM-DD` (Howard Hinnant's civil-from-days).
pub fn ymd(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

pub fn base_prompt(c: &BaseContext) -> String {
    let mut s = format!("{} LastHarness, made by LastSoftware, is the app you run in; it did not make you. Asked who made you, name the model's maker if known, else say you don't know. Be brief.\nToday: {}. ", identity_line(&c.identity), c.date);
    let folder = c.folder.map(|p| p.display().to_string()).unwrap_or_default();
    s.push_str(&match c.kind {
        SessionKind::Chat => "Mode: chat, no files or commands.\n".to_string(),
        SessionKind::Assistant => format!("Mode: assistant, folder {folder} (home).\n"),
        SessionKind::Project => format!("Mode: project, folder {folder} (stay inside it).\n"),
    });
    if c.kind != SessionKind::Chat {
        s.push_str(match c.permissions {
            Mode::Tiered => "Reads are free; writes ask once per session; deleting, installing, network and anything outside the folder always ask.",
            Mode::AskAll => "Every tool call asks the user first.",
            Mode::Bypass => "The user turned permission prompts off: do only what was asked.",
        });
        s.push_str(if c.sudo_enabled { " sudo: if needed." } else { " sudo is off." });
        s.push_str(" A refusal is final: say so, offer another way.\n");
    }
    s.push_str("Sensitive notes never go to a cloud model.");
    for g in c.guidelines {
        s.push('\n');
        s.push_str(g);
    }
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx<'a>(kind: SessionKind, perms: Mode, sudo: bool, folder: Option<&'a Path>, g: &'a [String]) -> BaseContext<'a> {
        BaseContext { identity: Identity::default(), kind, folder, permissions: perms, sudo_enabled: sudo, date: "2026-10-11", guidelines: g }
    }

    #[test]
    fn display_names_drop_paths_and_file_extensions_and_nothing_else() {
        assert_eq!(display_name("/home/user/AI/models/lfm25-2.6b/LFM2.5-2.6B-Q4_K_M.gguf"), "LFM2.5-2.6B-Q4_K_M");
        assert_eq!(display_name("Qwen3.6-35B-A3B-NanoPlus-fast"), "Qwen3.6-35B-A3B-NanoPlus-fast");
        assert_eq!(display_name("openai/gpt-4o"), "gpt-4o");
        assert_eq!(display_name("C:\\models\\mistral.gguf"), "mistral");
        assert_eq!(display_name("llama3.1:8b"), "llama3.1:8b");
        assert_eq!(display_name("  "), "");
    }

    #[test]
    fn parameter_counts_are_written_short() {
        assert_eq!(format_params(34_660_610_688), "34.7B");
        assert_eq!(format_params(2_697_198_592), "2.7B");
        assert_eq!(format_params(8_000_000_000), "8B");
        assert_eq!(format_params(270_000_000), "270M");
    }

    #[test]
    fn the_first_line_has_four_complete_forms_with_no_empty_slot() {
        let id = |m: Option<&str>, p: Option<u64>, v: Option<&str>| Identity { model: m.map(Into::into), parameters: p, provider: v.map(Into::into) };
        let cases = [
            (id(Some("Qwen3.6-35B-A3B"), None, Some("llama-swap")), "You are Qwen3.6-35B-A3B, served by llama-swap, an endpoint, not the model's maker."),
            (id(Some("Qwen3.6-35B-A3B"), Some(34_660_610_688), Some("llama-swap")), "You are Qwen3.6-35B-A3B (34.7B), served by llama-swap, an endpoint, not the model's maker."),
            (id(Some("/m/x/LFM2.5-2.6B.gguf"), Some(2_697_198_592), None), "You are LFM2.5-2.6B (2.7B)."),
            (id(Some("gpt-4o"), None, None), "You are gpt-4o."),
            (id(None, None, Some("OpenRouter")), "You are an AI model, served by OpenRouter, an endpoint, not the model's maker."),
            (id(None, Some(8_000_000_000), Some("OpenRouter")), "You are an AI model, served by OpenRouter, an endpoint, not the model's maker."),
            (id(None, None, None), "You are an AI assistant."),
            (id(Some("  "), Some(5), Some("  ")), "You are an AI assistant."),
            (id(Some("m"), Some(0), Some("")), "You are m."),
        ];
        for (identity, want) in cases {
            let line = identity_line(&identity);
            assert_eq!(line, want);
            assert!(!line.contains("  ") && !line.contains("()") && !line.contains("{}") && !line.ends_with("served by.") && !line.ends_with("served by") && !line.contains("served by ,") && !line.contains("served by ."), "{line}");
            // and inside the whole prompt the first line is that line
            let none: Vec<String> = vec![];
            let p = base_prompt(&BaseContext { identity, kind: SessionKind::Chat, folder: None, permissions: Mode::Tiered, sudo_enabled: false, date: "2026-10-11", guidelines: &none });
            assert!(p.starts_with(&format!("{want} LastHarness, made by LastSoftware, is the app you run in;")), "{p}");
            assert!(!p.contains("  "));
        }
    }

    #[test]
    fn the_app_is_not_the_models_maker_and_work_habits_are_for_tool_modes_only() {
        let g: Vec<String> = vec![];
        let chat = base_prompt(&ctx(SessionKind::Chat, Mode::Tiered, false, None, &g));
        let proj = base_prompt(&ctx(SessionKind::Project, Mode::Tiered, false, Some(Path::new("/p")), &g));
        for p in [&chat, &proj] {
            assert!(p.contains("it did not make you") && p.contains("say you don't know") && !p.contains("the only harness"), "{p}");
        }
        assert!(proj.contains("A refusal is final") && !chat.contains("A refusal is final"));
        assert!(!proj.contains("Read before editing"));
        let id = Identity { model: Some("m".into()), parameters: None, provider: Some("OpenRouter".into()) };
        assert!(identity_line(&id).contains("an endpoint, not the model's maker"));
    }

    #[test]
    fn each_enabled_tool_adds_one_usage_line_and_a_disabled_tool_adds_none() {
        let all = crate::default_tools().guidelines();
        for key in ["edit: old_string must match once", "bash:", "grep: search before"] {
            assert_eq!(all.iter().filter(|l| l.starts_with(key)).count(), 1, "{key} in {all:?}");
        }
        assert!(all.iter().all(|l| !l.starts_with("read:") && !l.starts_with("write:") && !l.starts_with("glob:")));
        let only_edit = crate::ToolSet::new([crate::Tool::Edit]).guidelines();
        assert_eq!(only_edit.len(), 1);
        assert!(only_edit[0].starts_with("edit:"));
        assert!(crate::ToolSet::new([crate::Tool::ReadFile]).guidelines().is_empty());
    }

    #[test]
    fn dates() {
        assert_eq!(ymd(0), "1970-01-01");
        assert_eq!(ymd(20007), "2024-10-11");
        assert_eq!(ymd(19_782), "2024-02-29");
        assert_eq!(today().len(), 10);
    }

    #[test]
    fn it_says_what_the_agent_is_where_it_works_and_what_it_may_do() {
        let g: Vec<String> = vec![];
        let p = base_prompt(&ctx(SessionKind::Project, Mode::Tiered, false, Some(Path::new("/work/shop")), &g));
        for want in ["LastHarness", "Today: 2026-10-11.", "Mode: project, folder /work/shop (stay inside it).", "Reads are free", "writes ask once per session", "always ask", "sudo is off", "A refusal is final", "Sensitive notes never go to a cloud model."] {
            assert!(p.contains(want), "missing {want:?} in:\n{p}");
        }
    }

    #[test]
    fn it_describes_only_what_exists() {
        let none: Vec<String> = vec![];
        let chat = base_prompt(&ctx(SessionKind::Chat, Mode::Tiered, false, None, &none));
        assert!(chat.contains("Mode: chat, no files or commands.") && !chat.contains("sudo") && !chat.contains("refusal"));
        assert!(chat.contains("Sensitive notes never go to a cloud model."), "the sensitive-notes rule is always there");
        let a = base_prompt(&ctx(SessionKind::Assistant, Mode::AskAll, true, Some(Path::new("/home/user")), &none));
        assert!(a.contains("Mode: assistant, folder /home/user (home).") && a.contains("Every tool call asks") && a.contains("sudo: if needed"));
        let b = base_prompt(&ctx(SessionKind::Project, Mode::Bypass, false, Some(Path::new("/p")), &none));
        assert!(b.contains("permission prompts off") && b.contains("sudo is off"));
        // a tool's line appears only if it is given
        let one = vec!["Use recall before guessing something about the user.".to_string()];
        assert!(base_prompt(&ctx(SessionKind::Project, Mode::Tiered, false, Some(Path::new("/p")), &one)).contains("Use recall before guessing"));
        assert!(!base_prompt(&ctx(SessionKind::Project, Mode::Tiered, false, Some(Path::new("/p")), &none)).contains("recall"));
    }

    #[test]
    fn it_is_identical_every_time_it_is_built_from_the_same_facts() {
        let g: Vec<String> = vec!["A line.".into()];
        let a = base_prompt(&ctx(SessionKind::Project, Mode::Tiered, false, Some(Path::new("/p")), &g));
        assert_eq!(a, base_prompt(&ctx(SessionKind::Project, Mode::Tiered, false, Some(Path::new("/p")), &g)));
        assert!(!a.contains("  ") && !a.contains('\t'));
    }

    #[test]
    fn the_base_alone_is_short_and_the_base_with_every_tool_line_stays_under_the_cap() {
        let tools = crate::default_tools();
        let none: Vec<String> = vec![];
        let g = tools.guidelines();
        // a typical session: a real model name, its size, the provider, a short folder
        let typical = Identity { model: Some("Qwen3.6-35B-A3B".into()), parameters: Some(34_660_610_688), provider: Some("llama-swap".into()) };
        let mut c = ctx(SessionKind::Project, Mode::Tiered, false, Some(Path::new("/work/shop")), &none);
        c.identity = typical.clone();
        let base = base_prompt(&c);
        let alone = estimate_tokens(&base);
        assert!(alone <= TARGET_TOKENS, "the base text is {alone} tokens (aim {TARGET_TOKENS}):\n{base}");
        // the worst case: long alias, long folder, sudo on, every default tool line
        let worst_dir = Path::new("/home/someone/Projects/a-fairly-long-project-folder-name");
        let mut w = ctx(SessionKind::Project, Mode::Tiered, true, Some(worst_dir), &g);
        w.identity = Identity { model: Some("Qwen3.6-35B-A3B-NanoPlus-fast".into()), parameters: Some(34_660_610_688), provider: Some("llama-swap".into()) };
        let all = base_prompt(&w);
        let worst = estimate_tokens(&all);
        assert!(worst <= CAP_TOKENS, "base plus tool lines is {worst} tokens (cap {CAP_TOKENS}):\n{all}");
        // every tool that has a line, as in a session with memory, skills, web and a shell
        let mut every: Vec<String> = g.clone();
        every.extend([crate::Tool::Recall, crate::Tool::Remember, crate::Tool::UseSkill].iter().filter_map(|t| t.guideline().map(str::to_string)));
        // the lines the core adds for its own tools (kept in step with lh-core: `show` and the department tools)
        every.push("show opens a long answer, file, image, table, chart or app.".into());
        every.push("Departments do bigger jobs: call_department here, start_project for new ones.".into());
        let mut e = ctx(SessionKind::Project, Mode::Tiered, true, Some(worst_dir), &every);
        e.identity = w.identity.clone();
        let full = estimate_tokens(&base_prompt(&e));
        eprintln!("prompt size: worst {worst}, every tool line {full} (cap {CAP_TOKENS})");
        assert!(full <= CAP_TOKENS, "with every tool line it is {full} tokens (cap {CAP_TOKENS})");
        eprintln!("base prompt: {alone} tokens typical, {worst} worst case with the default tool lines, {full} with every tool line");
    }
}
