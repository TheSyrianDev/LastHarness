//! Tools an agent can call, the permission rules that guard them, and the loop that runs a turn.
//! Tools are deny-by-default: a role only gets the tools it is given.

mod base_prompt;
mod command;
mod compact;
mod memory_tools;
mod perm;
mod runner;
mod search;
mod tools;
mod web;


pub use base_prompt::{base_prompt, display_name, estimate_tokens as estimate_text_tokens, format_params, identity_line, today, ymd, BaseContext, Identity, SessionKind, CAP_TOKENS, TARGET_TOKENS};
pub use command::{classify, Class};
pub use memory_tools::{MemoryHandle, PrivacyJudge};
pub mod codemode;
pub mod diff;
pub mod hooks;
pub mod lsp;
pub use hooks::Hooks;
pub use compact::{compact_if_needed, SUMMARY_MARKER, estimate_tokens, CompactError, CompactOptions, Compacted};
pub use perm::{Action, Answer, AskRequest, Asker, Decision, LivePerms, Mode, Permissions, ShowDiff};
pub use runner::{run_turn, Gate, Meter, MeterVerdict, Model, NoMeter, Outcome, TurnEnd, TurnEvent, TurnOptions};
pub use web::{html_to_text, is_public, WebConfig};
pub use tools::{canonical_name, edit_text, hosts_in, NetMode, SandboxNet, NET_ARG, default_tools, BoxFut, ExtraTool, GuardFn, OutboundGuard, Risk, Tool, ToolError, ToolOutput, ToolSet, Undo};
