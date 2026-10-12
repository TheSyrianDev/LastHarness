//! The parts of the desktop app that do not draw anything: the link to the core, the chat and run models,
//! themes, layout, right-to-left rules and preferences. They live here so they can be tested without a window.

pub mod api;
pub mod config;
pub mod content;
pub mod layout;
pub mod model;
pub mod models;
pub mod prefs;
pub mod rtl;
pub mod runs;
pub mod scene;
pub mod settings;
pub mod theme;
