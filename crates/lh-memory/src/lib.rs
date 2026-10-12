//! Memory: pages kept as Markdown files, indexed in the local store, with an append-only history.
//! Four layers: preferences, facts about you, projects and decisions (pages), and past
//! conversations (raw, in the database).

mod context;
mod conversations;
pub mod embed;
mod maintenance;
mod page;
mod proposals;
mod privacy;
mod repo;
mod tree;
mod vectors;
pub mod text;

pub use conversations::{ConversationInfo, MessageHit};
pub use tree::{MovePlan, Node, Tree};
pub use maintenance::{contradiction_prompt, parse_contradiction, similarity, Finding, FindingKind, STALE_DAYS};
pub use proposals::{parse_review, Kind, NewProposal, Parsed, Proposal, ProposalEdit, REVIEW_PROMPT};
pub use context::{excerpt, visible, Destination, Note, PageRef, RecallConfig, Recalled};
pub use page::*;
pub use privacy::{classify, detect, leaks, parse_judgement, Classification, Judge, Judgement, NoJudge, Trust, JUDGE_PROMPT};
pub use repo::{Error, HistoryRow, Hit, ImportReport, Memory, Result, ScanReport, State};
