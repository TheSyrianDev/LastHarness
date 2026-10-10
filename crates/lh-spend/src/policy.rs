use crate::{dollars, Config, Mode};

#[derive(Clone, Debug, PartialEq)]
pub enum Verdict {
    Allow,
    Ask(String),
    Refuse(String),
}

/// What has been spent so far, in micro-USD.
#[derive(Clone, Copy, Debug, Default)]
pub struct Totals {
    pub today: i64,
    pub this_month: i64,
}

fn micro(usd: f64) -> i64 {
    (usd * 1_000_000.0).round() as i64
}

/// `estimate` is the worst case for this call in micro-USD, or `None` when the model has no known
/// price and runs on a paid service. Local models are priced at zero, so they always pass.
pub fn decide(cfg: &Config, estimate: Option<i64>, spent: Totals) -> Verdict {
    if cfg.mode == Mode::None {
        return Verdict::Allow;
    }
    let Some(est) = estimate else {
        let why = "The price of this model is not known, so its cost cannot be checked. Add a [[price]] entry to the spend settings.".to_string();
        return if cfg.mode == Mode::Caps { Verdict::Refuse(why) } else { Verdict::Ask(why) };
    };
    // (name, amount over the limit) for each limit this call would pass
    let mut hits: Vec<String> = vec![];
    if let Some(cap) = cfg.per_request_cap_usd.map(micro).filter(|c| est > *c) {
        hits.push(format!("one call is expected to cost up to {}, over the {} per-call limit", dollars(est), dollars(cap)));
    }
    if let Some(cap) = cfg.daily_cap_usd.map(micro).filter(|c| spent.today + est > *c) {
        hits.push(format!("today's spending would reach {} of a {} limit", dollars(spent.today + est), dollars(cap)));
    }
    if let Some(cap) = cfg.monthly_cap_usd.map(micro).filter(|c| spent.this_month + est > *c) {
        hits.push(format!("this month's spending would reach {} of a {} limit", dollars(spent.this_month + est), dollars(cap)));
    }
    match cfg.mode {
        Mode::Caps if !hits.is_empty() => Verdict::Refuse(format!("Stopped by a spending cap: {}.", hits.join("; "))),
        Mode::Estimate if !hits.is_empty() => Verdict::Ask(format!("{}.", hits.join("; "))),
        Mode::Estimate if est > micro(cfg.approve_over_usd) => {
            Verdict::Ask(format!("This call is expected to cost up to {} (you are asked above {}).", dollars(est), dollars(micro(cfg.approve_over_usd))))
        }
        _ => Verdict::Allow,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(mode: Mode) -> Config {
        Config { mode, approve_over_usd: 0.25, daily_cap_usd: Some(5.0), monthly_cap_usd: Some(50.0), per_request_cap_usd: Some(1.0), ..Default::default() }
    }
    const NONE: Totals = Totals { today: 0, this_month: 0 };

    #[test]
    fn no_limits_allows_everything() {
        assert_eq!(decide(&cfg(Mode::None), Some(99_000_000), Totals { today: 99_000_000, this_month: 0 }), Verdict::Allow);
        assert_eq!(decide(&cfg(Mode::None), None, NONE), Verdict::Allow);
    }

    #[test]
    fn local_models_are_free_in_every_mode() {
        for m in [Mode::None, Mode::Caps, Mode::Estimate] {
            assert_eq!(decide(&cfg(m), Some(0), Totals { today: 5_000_000, this_month: 0 }), Verdict::Allow);
        }
    }

    #[test]
    fn caps_refuse_and_estimate_asks() {
        let spent = Totals { today: 4_900_000, this_month: 4_900_000 };
        assert!(matches!(decide(&cfg(Mode::Caps), Some(200_000), spent), Verdict::Refuse(m) if m.contains("today's spending")));
        assert!(matches!(decide(&cfg(Mode::Estimate), Some(200_000), spent), Verdict::Ask(_)));
        assert_eq!(decide(&cfg(Mode::Caps), Some(50_000), spent), Verdict::Allow);
        assert!(matches!(decide(&cfg(Mode::Caps), Some(1_500_000), NONE), Verdict::Refuse(m) if m.contains("per-call limit")));
    }

    #[test]
    fn estimate_asks_above_the_threshold_only() {
        assert_eq!(decide(&cfg(Mode::Estimate), Some(240_000), NONE), Verdict::Allow);
        assert!(matches!(decide(&cfg(Mode::Estimate), Some(260_000), NONE), Verdict::Ask(m) if m.contains("$0.26")));
    }

    #[test]
    fn unknown_prices_are_not_waved_through() {
        assert!(matches!(decide(&cfg(Mode::Caps), None, NONE), Verdict::Refuse(_)));
        assert!(matches!(decide(&cfg(Mode::Estimate), None, NONE), Verdict::Ask(_)));
    }
}
