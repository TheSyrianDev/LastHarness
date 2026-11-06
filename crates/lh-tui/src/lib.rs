//! Terminal front end. It is a thin client: everything real happens in the core (`lhd`).

pub mod app;
pub mod channels_cli;
pub mod client;
pub mod data_cli;
pub mod devices_cli;
pub mod headless;
pub mod learn_cli;
pub mod mcp_cli;
pub mod memory_cli;
pub mod providers_cli;
pub mod team_cli;
pub mod schedule_cli;
pub mod service_cli;
pub mod settings_cli;
pub mod setup;
pub mod terminal;
pub mod tree_ui;
pub mod ui;
