//! The model list as a person sees it. The router offers one entry per model and context tier
//! ("Qwen3.6-35B-balanced", "Qwen3.6-35B-max"); the app shows the model once, by its name, and picks
//! the entry itself.

/// Words the router adds at the end of a model id to say which context tier or template it is.
const TIERS: [&str; 8] = ["fast", "balanced", "max", "stock", "long", "low", "native", "high"];

/// The name without its tier words: "Qwen-fast-stock" is "Qwen".
pub fn display_name(id: &str) -> String {
    let mut s = id;
    while let Some((head, tail)) = s.rsplit_once('-') {
        if TIERS.contains(&tail) {
            s = head;
        } else {
            break;
        }
    }
    s.to_string()
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModelList {
    ids: Vec<String>,
    names: Vec<String>,
}

impl ModelList {
    pub fn new(ids: Vec<String>) -> ModelList {
        let mut names: Vec<String> = vec![];
        for id in &ids {
            let n = display_name(id);
            if !names.contains(&n) {
                names.push(n);
            }
        }
        ModelList { ids, names }
    }

    pub fn names(&self) -> &[String] {
        &self.names
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    pub fn has(&self, name: &str) -> bool {
        self.names.iter().any(|n| n == name)
    }

    /// The router entry to use for a model name: the "balanced" one if there is one, else the first.
    /// A name that is not in the list is passed on as it is (the core knows best what it means).
    pub fn id_for(&self, name: &str) -> String {
        let all: Vec<&String> = self.ids.iter().filter(|i| display_name(i) == name).collect();
        all.iter().find(|i| i.ends_with("-balanced")).or(all.first()).map(|i| i.to_string()).unwrap_or_else(|| name.to_string())
    }

    /// The name to show for a model id the core reported.
    pub fn name_of(&self, id: &str) -> String {
        display_name(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list() -> ModelList {
        ModelList::new(["LFM2.5-2.6B-max", "LFM2.5-2.6B-balanced", "Qwen3.6-35B-A3B-NanoPlus-fast-stock-long", "Qwen3.6-35B-A3B-NanoPlus-fast", "SignOfFour", "Tiel-Coder-35B-low"].map(String::from).to_vec())
    }

    #[test]
    fn tier_words_are_dropped_from_the_end_only() {
        assert_eq!(display_name("Qwen3.6-35B-A3B-NanoPlus-fast-stock-long"), "Qwen3.6-35B-A3B-NanoPlus");
        assert_eq!(display_name("LFM2.5-2.6B-max"), "LFM2.5-2.6B");
        assert_eq!(display_name("SignOfFour"), "SignOfFour");
        assert_eq!(display_name("fast-model-x"), "fast-model-x", "a tier word inside the name stays");
        assert_eq!(display_name("max"), "max");
        assert_eq!(display_name(""), "");
    }

    #[test]
    fn each_model_is_listed_once_in_the_order_first_seen() {
        assert_eq!(list().names(), ["LFM2.5-2.6B", "Qwen3.6-35B-A3B-NanoPlus", "SignOfFour", "Tiel-Coder-35B"]);
        assert!(list().has("SignOfFour") && !list().has("nope"));
        assert!(ModelList::default().is_empty());
    }

    #[test]
    fn the_balanced_entry_is_preferred_then_the_first_then_the_name_itself() {
        let l = list();
        assert_eq!(l.id_for("LFM2.5-2.6B"), "LFM2.5-2.6B-balanced");
        assert_eq!(l.id_for("Qwen3.6-35B-A3B-NanoPlus"), "Qwen3.6-35B-A3B-NanoPlus-fast-stock-long", "no balanced one: the first");
        assert_eq!(l.id_for("SignOfFour"), "SignOfFour");
        assert_eq!(l.id_for("unknown"), "unknown");
        assert_eq!(l.name_of("Tiel-Coder-35B-low"), "Tiel-Coder-35B");
    }
}
