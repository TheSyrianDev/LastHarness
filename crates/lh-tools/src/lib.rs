//! Tools an agent can call, the permission rules that guard them, and the loop that runs a turn.
//! Tools are deny-by-default: a role only gets the tools it is given.

mod command;
mod perm;
mod runner;
mod tools;

pub use command::{classify, Class};
pub use perm::{Action, Answer, AskRequest, Asker, Decision, Mode, Permissions};
pub use runner::{run_turn, Model, TurnEnd, TurnOptions};
pub use tools::{default_tools, Tool, ToolError, ToolSet};
