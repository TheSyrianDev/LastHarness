//! Learning: rules, the style profile and skills, each changed only through suggestions the user
//! confirms (or, if the user turned silent learning on, applied and logged). The base prompt is never
//! rewritten: learned things are separate blocks added after it, every version is kept, and any item
//! can be rolled back.

mod assemble;
mod error;
mod history;
mod proposals;
mod rules;
mod signals;
mod skills;
pub mod yaml;
mod style;

pub use assemble::{compose_system, render_learned, Learned};
pub use error::{Error, Result};
pub use history::HistoryEntry;
pub use proposals::{habit_prompt, parse_drafts, parse_skill_change, parse_skill_draft, session_prompt, skill_improve_prompt, skill_prompt, Applied, Draft, DraftKind, Kind, Proposal, SkillDraft};
pub use skills::{parse_skill_md, render_skill_md, valid_name, Skill, SkillEntry, SkillMeta};
pub use signals::{classify_message, norm_words, Cluster, Signal, SignalKind};
pub use rules::{NewRule, Rule, RuleContext, RuleEdit, Status};
pub use style::{Style, STYLE_MAX_CHARS};

use std::sync::Arc;

use lh_store::Store;

/// Entry point for everything learned.
pub struct Learn {
    pub(crate) store: Arc<Store>,
    pub(crate) skills_dir: Option<std::path::PathBuf>,
}

impl std::fmt::Debug for Learn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Learn").field("skills_dir", &self.skills_dir).finish_non_exhaustive()
    }
}

impl Learn {
    pub fn new(store: Arc<Store>) -> Self {
        Self { store, skills_dir: None }
    }

    pub fn store(&self) -> &Arc<Store> {
        &self.store
    }
}

/// A short random id with a prefix, such as `r_1a2b3c4d`.
pub fn new_id(prefix: &str) -> String {
    let mut b = [0u8; 4];
    getrandom::fill(&mut b).expect("the OS random source is unavailable");
    format!("{prefix}_{}", b.iter().map(|x| format!("{x:02x}")).collect::<String>())
}
