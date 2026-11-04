//! The parts of the desktop app that do not draw anything: the link to the core, the chat and run models,
//! themes, layout, right-to-left rules and preferences. They live here so they can be tested without a window.

pub mod api;
pub mod appearance;
pub mod bidi;
pub mod brand;
pub mod chrome;
pub mod config;
pub mod content;
pub mod fonts;
pub mod follow;
pub mod greeting;
pub mod icons;
pub mod layout;
pub mod model;
pub mod motion;
pub mod models;
pub mod omarchy;
pub mod page_fetch;
pub mod pages;
pub mod pages_logic;
pub mod palette;
pub mod permissions;
pub mod prefs;
pub mod rtl;
pub mod runs;
pub mod setting_names;
pub mod settings;
pub mod sidebar;
pub mod slide;
pub mod theme;
pub mod tiers;
pub mod wheel;
pub mod thinking;
pub mod transcript;
pub mod treeview;
