//! The engine side of the page viewer: the Blitz document ([`engine`]) and what it is allowed to ask for ([`net`]).
//! It runs only inside the helper program (`src/bin/lh-pageview-helper.rs`).

pub mod engine;
pub mod net;
