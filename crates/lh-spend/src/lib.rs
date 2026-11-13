//! Spend tracking and limits. Three modes, chosen by the user (spec §10.3): no limits, hard caps, or
//! estimate-and-approve with thresholds (the default). Every model call is recorded in the local
//! store with its tokens and cost. Money is kept in millionths of a dollar (micro-USD); because a
//! price of "$X per million tokens" equals X micro-USD per token, cost is tokens times price.

mod config;
mod meter;
mod policy;

pub use config::{Config, Mode, Price};
pub use meter::{ChatUsage, SpendMeter, Summary};

/// One line for `/usage`: what this chat has used.
pub fn usage_line(u: &ChatUsage) -> String {
    let mut s = format!("This chat: {} calls, {} prompt tokens, {} completion tokens, {}.", u.calls, u.prompt_tokens, u.completion_tokens, dollars(u.cost_micro_usd));
    if u.unpriced > 0 {
        s.push_str(&format!(" {} calls had no known price.", u.unpriced));
    }
    s
}

/// Lines for `/insights`: this month and the top providers.
pub fn insights_text(s: &Summary) -> String {
    let mut out = format!("This month: {} over {} calls. Today: {} over {} calls.", dollars(s.this_month), s.month_calls, dollars(s.today), s.today_calls);
    for (p, cost, n) in s.by_provider.iter().take(3) {
        out.push_str(&format!("\n{p}: {} over {n} calls", dollars(*cost)));
    }
    if s.unpriced > 0 {
        out.push_str(&format!("\n{} calls this month had no known price.", s.unpriced));
    }
    out
}
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

#[cfg(test)]
mod text_tests {
    use super::*;

    #[test]
    fn usage_and_insights_read_in_plain_words() {
        let u = ChatUsage { calls: 3, prompt_tokens: 1200, completion_tokens: 300, cost_micro_usd: 42_000, unpriced: 1 };
        assert_eq!(usage_line(&u), "This chat: 3 calls, 1200 prompt tokens, 300 completion tokens, $0.04. 1 calls had no known price.");
        let s = Summary { today: 0, today_calls: 0, this_month: 5_000_000, month_calls: 9, unpriced: 0, by_provider: vec![("gpt".into(), 5_000_000, 9)] };
        assert_eq!(insights_text(&s), "This month: $5.00 over 9 calls. Today: $0.00 over 0 calls.\ngpt: $5.00 over 9 calls");
    }
}
