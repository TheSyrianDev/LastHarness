//! Tools an agent can call, the permission rules that guard them, and the loop that runs a turn.
//! Tools are deny-by-default: a role only gets the tools it is given.

mod command;
mod compact;
mod perm;
mod runner;
mod tools;
mod web;


pub use command::{classify, Class};
pub use compact::{compact_if_needed, estimate_tokens, CompactError, CompactOptions, Compacted};
pub use perm::{Action, Answer, AskRequest, Asker, Decision, Mode, Permissions};
pub use runner::{run_turn, Meter, MeterVerdict, Model, NoMeter, Outcome, TurnEnd, TurnEvent, TurnOptions};
pub use web::{html_to_text, is_public, WebConfig};
pub use tools::{default_tools, Tool, ToolError, ToolOutput, ToolSet, Undo};
