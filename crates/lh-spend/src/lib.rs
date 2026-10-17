//! Spend tracking and limits. Three modes, chosen by the user (spec §10.3): no limits, hard caps, or
//! estimate-and-approve with thresholds (the default). Every model call is recorded in the local
//! store with its tokens and cost. Money is kept in millionths of a dollar (micro-USD); because a
//! price of "$X per million tokens" equals X micro-USD per token, cost is tokens times price.

mod config;
mod meter;
mod policy;

pub use config::{Config, Mode, Price};
pub use meter::{SpendMeter, Summary};
pub use policy::{decide, Totals, Verdict};

/// Format micro-USD for people: `$0.42`, or four decimals under a cent.
pub fn dollars(micro: i64) -> String {
    let d = micro as f64 / 1_000_000.0;
    if d.abs() < 0.01 && micro != 0 {
        format!("${d:.4}")
    } else {
        format!("${d:.2}")
    }
}
