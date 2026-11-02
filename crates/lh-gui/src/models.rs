//! The model list as a person sees it, grouped like the `/pick` Pi extension. The router offers one entry per
//! model, context tier and template ("Qwen3.6-35B-A3B-balanced", "Qwen3.6-35B-A3B-max-stock"), and a mode by an
//! alias ("Qwen3.6-35B-A3B-balanced:instruct"). The app shows each model once and offers its settings inside it.
//!
//! A chat keeps its exact choice, `<entry>[:<mode>]`, which is what the core is sent.

/// Context tiers, smallest first.
pub const TIERS: [&str; 4] = ["low", "balanced", "native", "max"];
/// Older router words that may still end a saved model name.
const OLD_WORDS: [&str; 8] = ["fast", "balanced", "max", "stock", "long", "low", "native", "high"];

/// What a mode is called on a button.
pub fn mode_word(mode: &str) -> String {
    match mode {
        "think" => "Thinking",
        "code" => "Coding",
        "instruct" => "No thinking",
        "auto" => "Auto",
        "low" => "Reasoning low",
        "medium" => "Reasoning medium",
        "high" => "Reasoning high",
        other => other,
    }
    .to_string()
}

/// The `/pick` naming rule: drop the parameter count and packing tags, keep a version, set a version off with a
/// space. "Qwen3.6-35B-A3B" is "Qwen 3.6"; "LFM2.5-2.6B" is "LFM 2.5".
pub fn display_name(base: &str) -> String {
    let parts: Vec<&str> = base.split(['-', '_']).filter(|p| !p.is_empty()).collect();
    let is_size = |p: &str| {
        let up = p.to_ascii_uppercase();
        up.strip_suffix('B').is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit() || c == '.'))
    };
    let kept: Vec<&str> = parts.iter().copied().filter(|p| !is_size(p) && !p.eq_ignore_ascii_case("A3B") && !p.eq_ignore_ascii_case("APEX") && !p.chars().all(|c| c.is_ascii_digit())).collect();
    let mut out = String::new();
    for (i, p) in kept.iter().enumerate() {
        let version = p.chars().next().is_some_and(|c| c.is_ascii_digit()) || (p.starts_with('X') && p[1..].chars().next().is_some_and(|c| c.is_ascii_digit()));
        if i > 0 {
            out.push(if version { ' ' } else { '-' });
        }
        out.push_str(p);
    }
    // "Qwen3.6" is "Qwen 3.6"
    if let Some(i) = out.find(|c: char| c.is_ascii_digit()) {
        let (head, tail) = out.split_at(i);
        if !head.is_empty() && head.chars().all(|c| c.is_ascii_alphabetic()) && tail.split(['-', ' ']).next().is_some_and(|v| v.contains('.')) {
            out = format!("{head} {tail}");
        }
    }
    if out.is_empty() {
        base.to_string()
    } else {
        out
    }
}

/// An entry name split into its parts.
#[derive(Clone, Debug, PartialEq)]
pub struct Parsed {
    pub base: String,
    pub tier: String,
    pub tpl: String,
    pub mode: String,
    /// The entry without the mode.
    pub entry: String,
}

pub fn parse(choice: &str) -> Parsed {
    let (entry, mode) = choice.rsplit_once(':').map(|(e, m)| (e, m)).unwrap_or((choice, ""));
    let mut base = entry;
    let mut tpl = "sharp";
    for t in ["stock", "vision"] {
        if let Some(b) = base.strip_suffix(&format!("-{t}")) {
            base = b;
            tpl = t;
        }
    }
    let mut tier = "";
    for t in TIERS {
        if let Some(b) = base.strip_suffix(&format!("-{t}")) {
            base = b;
            tier = t;
            break;
        }
    }
    if tier.is_empty() {
        // no tier: the template word was part of the name
        base = entry;
        tpl = "sharp";
    }
    Parsed { base: base.into(), tier: tier.into(), tpl: tpl.into(), mode: mode.into(), entry: entry.into() }
}

/// One router entry and what the server says about it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModelInfo {
    pub id: String,
    pub modes: Vec<String>,
    pub default_mode: String,
    pub loaded: bool,
}

impl ModelInfo {
    pub fn from_json(v: &serde_json::Value) -> Option<ModelInfo> {
        Some(ModelInfo {
            id: v["id"].as_str()?.to_string(),
            modes: v["modes"].as_array().map(|a| a.iter().filter_map(|m| m.as_str().map(str::to_string)).collect()).unwrap_or_default(),
            default_mode: v["default_mode"].as_str().unwrap_or("").to_string(),
            loaded: v["loaded"].as_bool().unwrap_or(false),
        })
    }
}

/// A model: its entries are its settings.
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub base: String,
    pub label: String,
    /// "35B", or empty.
    pub size: String,
    pub members: Vec<ModelInfo>,
}

impl Group {
    pub fn loaded(&self) -> bool {
        self.members.iter().any(|m| m.loaded)
    }
    /// The entry used when the model is picked with no settings: balanced and sharp if there is one.
    pub fn default_entry(&self) -> &ModelInfo {
        let p = |m: &&ModelInfo| parse(&m.id);
        self.members.iter().find(|m| { let x = p(m); x.tier == "balanced" && x.tpl == "sharp" }).or_else(|| self.members.iter().find(|m| p(m).tpl == "sharp")).unwrap_or(&self.members[0])
    }
    pub fn templates(&self) -> Vec<String> {
        let mut t: Vec<String> = vec![];
        for m in &self.members {
            let x = parse(&m.id).tpl;
            if !t.contains(&x) {
                t.push(x);
            }
        }
        t.sort_by_key(|x| (x != "sharp", x.clone()));
        t
    }
    pub fn tiers(&self, tpl: &str) -> Vec<String> {
        TIERS.iter().filter(|t| self.members.iter().any(|m| { let x = parse(&m.id); x.tier == **t && x.tpl == tpl })).map(|t| t.to_string()).collect()
    }
    /// The entry for a tier and template, falling back to the template, then the default.
    pub fn entry(&self, tier: &str, tpl: &str) -> &ModelInfo {
        self.members
            .iter()
            .find(|m| { let x = parse(&m.id); x.tier == tier && x.tpl == tpl })
            .or_else(|| self.members.iter().find(|m| parse(&m.id).tpl == tpl))
            .unwrap_or_else(|| self.default_entry())
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModelList {
    infos: Vec<ModelInfo>,
    groups: Vec<Group>,
    names: Vec<String>,
}

impl ModelList {
    /// From names only (a server that says nothing more).
    pub fn new(ids: Vec<String>) -> ModelList {
        ModelList::with_details(ids.into_iter().map(|id| ModelInfo { id, ..Default::default() }).collect())
    }

    pub fn with_details(infos: Vec<ModelInfo>) -> ModelList {
        let mut groups: Vec<Group> = vec![];
        for m in &infos {
            let base = parse(&m.id).base;
            match groups.iter_mut().find(|g| g.base == base) {
                Some(g) => g.members.push(m.clone()),
                None => {
                    let size = base.split(['-', '_']).find(|p| p.len() > 1 && p.ends_with(['B', 'b']) && p[..p.len() - 1].chars().all(|c| c.is_ascii_digit() || c == '.')).unwrap_or("").to_string();
                    groups.push(Group { label: display_name(&base), base, size, members: vec![m.clone()] });
                }
            }
        }
        groups.sort_by_key(|g| g.label.to_lowercase());
        let names = groups.iter().map(|g| g.default_entry().id.clone()).collect();
        ModelList { infos, groups, names }
    }

    pub fn groups(&self) -> &[Group] {
        &self.groups
    }

    /// One choice per model (its default entry), for lists that offer models without their settings.
    pub fn names(&self) -> &[String] {
        &self.names
    }

    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    pub fn info(&self, entry: &str) -> Option<&ModelInfo> {
        self.infos.iter().find(|m| m.id == entry)
    }

    pub fn group_of(&self, choice: &str) -> Option<&Group> {
        let base = parse(&self.id_for(choice)).base;
        self.groups.iter().find(|g| g.base == base)
    }

    /// A choice this server offers: a known entry, with a mode it lists when one is given.
    pub fn has(&self, choice: &str) -> bool {
        let p = parse(choice);
        self.info(&p.entry).is_some_and(|m| p.mode.is_empty() || m.modes.contains(&p.mode))
    }

    /// The id to send to the core. A known choice is sent as it is; an older saved name (a model with no
    /// settings, or a label) becomes that model's default entry; anything else is passed on unchanged.
    pub fn id_for(&self, choice: &str) -> String {
        if self.has(choice) || choice.is_empty() {
            return choice.to_string();
        }
        let old = |id: &str| {
            let mut s = id;
            while let Some((head, tail)) = s.rsplit_once('-') {
                if OLD_WORDS.contains(&tail) {
                    s = head;
                } else {
                    break;
                }
            }
            s.to_string()
        };
        self.groups
            .iter()
            .find(|g| g.base == choice || g.label == choice || g.members.iter().any(|m| old(&m.id) == choice))
            .map(|g| g.default_entry().id.clone())
            .unwrap_or_else(|| choice.to_string())
    }

    /// The choice for a model id the core reported: the id itself.
    pub fn name_of(&self, id: &str) -> String {
        id.to_string()
    }

    /// What a choice is called on the model button and in lists: "Qwen 3.6 · max · No thinking". The usual
    /// settings (balanced, sharp, the default mode) are left out.
    pub fn label_of(&self, choice: &str) -> String {
        if choice.is_empty() {
            return String::new();
        }
        let p = parse(&self.id_for(choice));
        let mut bits = vec![display_name(&p.base)];
        if !p.tier.is_empty() && p.tier != "balanced" {
            bits.push(p.tier.clone());
        }
        if p.tpl != "sharp" {
            bits.push(p.tpl.clone());
        }
        if !p.mode.is_empty() {
            bits.push(mode_word(&p.mode));
        }
        bits.join(" · ")
    }

    /// The groups whose name holds every typed word, in any case.
    pub fn search(&self, query: &str) -> Vec<&Group> {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        self.groups.iter().filter(|g| { let hay = format!("{} {}", g.label, g.base).to_lowercase(); words.iter().all(|w| hay.contains(w)) }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(id: &str, modes: &[&str]) -> ModelInfo {
        ModelInfo { id: id.into(), modes: modes.iter().map(|m| m.to_string()).collect(), default_mode: "think".into(), loaded: false }
    }

    fn list() -> ModelList {
        ModelList::with_details(vec![
            info("Qwen3.6-35B-A3B-balanced", &["think", "code", "instruct"]),
            info("Qwen3.6-35B-A3B-max", &["think", "code", "instruct"]),
            info("Qwen3.6-35B-A3B-balanced-stock", &["think"]),
            info("Qwen3.6-35B-A3B-low-vision", &[]),
            info("LFM2.5-2.6B-low", &["think", "instruct"]),
            info("LFM2.5-2.6B-balanced", &["think", "instruct"]),
            info("SignOfFour", &[]),
        ])
    }

    #[test]
    fn names_follow_the_pick_rule() {
        assert_eq!(display_name("Qwen3.6-35B-A3B"), "Qwen 3.6");
        assert_eq!(display_name("LFM2.5-2.6B"), "LFM 2.5");
        assert_eq!(display_name("Tiel-Coder-35B"), "Tiel-Coder");
        assert_eq!(display_name("KAT-Coder-V2.5-Dev"), "KAT-Coder-V2.5-Dev");
        assert_eq!(display_name("gpt-oss-20b"), "gpt-oss");
        assert_eq!(display_name("SignOfFour"), "SignOfFour");
    }

    #[test]
    fn an_entry_splits_into_model_tier_template_and_mode() {
        let p = parse("Qwen3.6-35B-A3B-max-stock:instruct");
        assert_eq!((p.base.as_str(), p.tier.as_str(), p.tpl.as_str(), p.mode.as_str(), p.entry.as_str()), ("Qwen3.6-35B-A3B", "max", "stock", "instruct", "Qwen3.6-35B-A3B-max-stock"));
        let p = parse("SignOfFour");
        assert_eq!((p.base.as_str(), p.tier.as_str(), p.tpl.as_str()), ("SignOfFour", "", "sharp"));
        assert_eq!(parse("Live-stock").base, "Live-stock", "a template word with no tier is part of the name");
    }

    #[test]
    fn each_model_is_one_group_with_its_settings() {
        let l = list();
        let labels: Vec<&str> = l.groups().iter().map(|g| g.label.as_str()).collect();
        assert_eq!(labels, ["LFM 2.5", "Qwen 3.6", "SignOfFour"]);
        let q = &l.groups()[1];
        assert_eq!(q.size, "35B");
        assert_eq!(q.templates(), ["sharp", "stock", "vision"]);
        assert_eq!(q.tiers("sharp"), ["balanced", "max"]);
        assert_eq!(q.entry("max", "sharp").id, "Qwen3.6-35B-A3B-max");
        assert_eq!(q.entry("max", "vision").id, "Qwen3.6-35B-A3B-low-vision", "no such tier: the template's entry");
        assert_eq!(l.names(), ["LFM2.5-2.6B-balanced", "Qwen3.6-35B-A3B-balanced", "SignOfFour"]);
    }

    #[test]
    fn choices_keep_their_mode_and_old_names_find_their_model() {
        let l = list();
        assert!(l.has("Qwen3.6-35B-A3B-max:instruct"));
        assert!(!l.has("Qwen3.6-35B-A3B-max:nope"), "a mode the server does not list");
        assert_eq!(l.id_for("Qwen3.6-35B-A3B-max:instruct"), "Qwen3.6-35B-A3B-max:instruct");
        assert_eq!(l.id_for("LFM2.5-2.6B"), "LFM2.5-2.6B-balanced", "a name saved by the older app");
        assert_eq!(l.id_for("Qwen 3.6"), "Qwen3.6-35B-A3B-balanced", "a label");
        assert_eq!(l.id_for("elsewhere"), "elsewhere");
        assert_eq!(l.label_of("Qwen3.6-35B-A3B-max-stock:instruct"), "Qwen 3.6 · max · stock · No thinking");
        assert_eq!(l.label_of("LFM2.5-2.6B-balanced"), "LFM 2.5");
        assert_eq!(l.search("qwen").len(), 1);
        assert_eq!(l.search("LFM 2.5").len(), 1);
        assert!(l.search("zzz").is_empty());
    }
}
