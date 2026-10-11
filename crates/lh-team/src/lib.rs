//! The team: roles, model types, departments, and the engine that runs a goal through them.
//!
//! Roles and departments are Markdown files with a header, kept in folders the user can read and edit.
//! A role asks for a kind of model (a model type) and the type says which model that is today. A
//! department has a lead and members; the lead plans, hands out steps, checks results and handles
//! failures. Every change to a role or department is versioned and can be rolled back.

mod dept;
mod error;
mod plan;
mod roles;
mod settings;
mod types;

pub use dept::{Department, DeptBook, Member};
pub use error::{Error, Result};
pub use plan::{check_prompt, excerpt, failure_prompt, final_prompt, parse_check, parse_plan, parse_remedy, plan_prompt, Check, Plan, PlanStep, Remedy, MAX_STEPS};
pub use roles::{MemorySlice, Role, RoleBook, RoleSampling, Target, KNOWN_TOOLS};
pub use settings::{Autonomy, TeamSettings, UnavailablePolicy};
pub use types::{ModelType, ModelTypes, Resolution};
