use std::sync::Arc;

use lh_provider::{ChatRequest, Usage};
use lh_store::{now, Store};
use lh_tools::{Meter, MeterVerdict};

use crate::{decide, dollars, Config, Totals, Verdict};

struct Shared {
    store: Arc<Store>,
    /// The rules in force. Replaced when the settings change.
    cfg: std::sync::RwLock<Config>,
    /// The rules as the core started with them (the spend file), which the settings are applied on top of.
    base: Config,
    provider: String,
    /// Runs on this machine or a private network: priced at zero unless a price says otherwise.
    local: bool,
}

/// Checks each model call against the spending rules and records what it cost.
#[derive(Clone)]
pub struct SpendMeter {
    shared: Arc<Shared>,
    session: String,
}

/// What was spent, for the Activity page. Money is in micro-USD.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Summary {
    pub today: i64,
    pub today_calls: i64,
    pub this_month: i64,
    pub month_calls: i64,
    /// Calls this month with no known price.
    pub unpriced: i64,
    /// This month: provider, micro-USD, calls. Most expensive first.
    pub by_provider: Vec<(String, i64, i64)>,
}

/// Fewer bytes per token than English text really has, so the estimate errs high.
const BYTES_PER_TOKEN: usize = 3;
/// Used when the request sets no output limit.
const DEFAULT_MAX_OUTPUT: u64 = 4096;

impl SpendMeter {
    pub fn new(store: Arc<Store>, cfg: Config, provider: impl Into<String>, local: bool) -> Self {
        Self { shared: Arc::new(Shared { store, cfg: std::sync::RwLock::new(cfg.clone()), base: cfg, provider: provider.into(), local }), session: String::new() }
    }

    /// Change the rules while the core runs: `f` starts from the rules the meter was built with, so a setting
    /// that is reset goes back to what the spend file says. Calls already being checked finish under the old rules.
    pub fn reconfigure(&self, f: impl FnOnce(&mut Config)) {
        let mut c = self.shared.base.clone();
        f(&mut c);
        *self.shared.cfg.write().unwrap() = c;
    }

    pub fn config(&self) -> Config {
        self.shared.cfg.read().unwrap().clone()
    }

    /// What one input token of `model` costs in micro-USD, if the user listed a price for it. Cost is tokens times this.
    pub fn input_price(&self, model: &str) -> Option<f64> {
        self.shared.cfg.read().unwrap().price_of(model).map(|p| p.input).filter(|p| *p > 0.0)
    }

    pub fn for_session(&self, id: impl Into<String>) -> Self {
        Self { shared: self.shared.clone(), session: id.into() }
    }

    pub fn totals(&self) -> Totals {
        let t = now();
        let (day, month) = (t - t.rem_euclid(86_400), month_start(t));
        let sum = |since: i64| {
            self.shared
                .store
                .with(|c| c.query_row("select coalesce(sum(cost_micro_usd), 0) from spend where at >= ?1", [since], |r| r.get::<_, i64>(0)))
                .unwrap_or(0)
        };
        Totals { today: sum(day), this_month: sum(month) }
    }

    /// Spending today and this month (UTC), and this month per provider, for people to read. `unpriced` counts
    /// calls whose cost is unknown because the model has no price.
    pub fn summary(&self) -> Summary {
        let t = now();
        let (day, month) = (t - t.rem_euclid(86_400), month_start(t));
        let sum = |since: i64| self.shared.store.with(|c| c.query_row("select coalesce(sum(cost_micro_usd), 0), count(*) from spend where at >= ?1", [since], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))).unwrap_or((0, 0));
        let unpriced = self.shared.store.with(|c| c.query_row("select count(*) from spend where at >= ?1 and cost_micro_usd is null", [month], |r| r.get::<_, i64>(0))).unwrap_or(0);
        let by_provider = self
            .shared
            .store
            .with(|c| {
                let mut q = c.prepare("select provider, coalesce(sum(cost_micro_usd), 0), count(*) from spend where at >= ?1 group by provider order by 2 desc, 1")?;
                let rows = q.query_map([month], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?)))?.collect::<Result<Vec<_>, _>>();
                rows
            })
            .unwrap_or_default();
        let (today, today_calls) = sum(day);
        let (this_month, month_calls) = sum(month);
        Summary { today, today_calls, this_month, month_calls, unpriced, by_provider }
    }

    /// Worst-case cost of this request in micro-USD, or `None` if the model has no known price.
    fn estimate(&self, req: &ChatRequest) -> Option<i64> {
        let bytes: usize = req.messages.iter().map(|m| m.content.as_deref().map_or(0, str::len) + m.tool_calls.iter().map(|c| c.arguments.len() + c.name.len()).sum::<usize>()).sum::<usize>()
            + req.tools.iter().map(|t| t.description.len() + t.parameters.to_string().len()).sum::<usize>();
        let prompt = (bytes / BYTES_PER_TOKEN) as u64;
        let out = req.max_tokens.map_or(DEFAULT_MAX_OUTPUT, u64::from);
        self.cost(&req.model, Usage { prompt_tokens: prompt, completion_tokens: out, cached_tokens: 0 })
    }

    fn cost(&self, model: &str, u: Usage) -> Option<i64> {
        match self.shared.cfg.read().unwrap().price_of(model) {
            Some(p) => {
                let fresh = u.prompt_tokens.saturating_sub(u.cached_tokens) as f64;
                let cached = u.cached_tokens as f64;
                Some((fresh * p.input + cached * p.cached_input.unwrap_or(p.input) + u.completion_tokens as f64 * p.output).round() as i64)
            }
            None if self.shared.local => Some(0),
            None => None,
        }
    }
}

impl SpendMeter {
    /// Cost of `count` images in micro-USD. `None`: paid service with no price.
    fn media_cost(&self, model: &str, count: u32) -> Option<i64> {
        match self.shared.cfg.read().unwrap().price_of(model).and_then(|p| p.image) {
            Some(usd) => Some((usd * 1_000_000.0 * f64::from(count)).round() as i64),
            None if self.shared.local => Some(0),
            None => None,
        }
    }
}

impl Meter for SpendMeter {
    async fn before_media(&self, model: &str, count: u32) -> MeterVerdict {
        match decide(&self.shared.cfg.read().unwrap(), self.media_cost(model, count), self.totals()) {
            Verdict::Allow => MeterVerdict::Allow,
            Verdict::Ask(m) => MeterVerdict::Ask(m),
            Verdict::Refuse(m) => MeterVerdict::Refuse(m),
        }
    }

    async fn after_media(&self, model: &str, count: u32) {
        let (s, cost) = (&self.shared, self.media_cost(model, count));
        let r = s.store.with(|c| {
            c.execute(
                "insert into spend(at, provider, model, session, prompt_tokens, completion_tokens, cached_tokens, cost_micro_usd) values (?1,?2,?3,?4,0,0,0,?5)",
                (now(), &s.provider, model, &self.session, cost),
            )
        });
        if !s.local {
            let _ = s.store.log("request_sent", &self.session, "assistant", &serde_json::json!({"provider": s.provider, "model": model, "images": count, "cost_micro_usd": cost}));
        }
        if let Err(e) = r {
            eprintln!("lh-spend: could not record an image ({e})");
        }
    }

    async fn before(&self, req: &ChatRequest) -> MeterVerdict {
        let est = self.estimate(req);
        match decide(&self.shared.cfg.read().unwrap(), est, self.totals()) {
            Verdict::Allow => MeterVerdict::Allow,
            Verdict::Ask(m) => MeterVerdict::Ask(m),
            Verdict::Refuse(m) => MeterVerdict::Refuse(m),
        }
    }

    async fn after(&self, req: &ChatRequest, usage: Option<Usage>) {
        let u = usage.unwrap_or_default();
        // Without a usage report from the server the tokens are unknown; the row says so with zeros.
        let cost = if usage.is_some() { self.cost(&req.model, u) } else { self.cost(&req.model, Usage::default()) };
        let s = &self.shared;
        let r = s.store.with(|c| {
            c.execute(
                "insert into spend(at, provider, model, session, prompt_tokens, completion_tokens, cached_tokens, cost_micro_usd) values (?1,?2,?3,?4,?5,?6,?7,?8)",
                rusqlite_params(now(), &s.provider, &req.model, &self.session, u, cost),
            )
        });
        if !s.local {
            let detail = serde_json::json!({"provider": s.provider, "model": req.model, "prompt_tokens": u.prompt_tokens, "completion_tokens": u.completion_tokens, "cost_micro_usd": cost});
            let _ = s.store.log("request_sent", &self.session, "assistant", &detail);
        }
        if let Err(e) = r {
            eprintln!("lh-spend: could not record a model call ({e}); {} of spending is not in the ledger", cost.map(dollars).unwrap_or_else(|| "unknown".into()));
        }
    }
}

#[allow(clippy::type_complexity)]
fn rusqlite_params<'a>(at: i64, provider: &'a str, model: &'a str, session: &'a str, u: Usage, cost: Option<i64>) -> (i64, &'a str, &'a str, &'a str, i64, i64, i64, Option<i64>) {
    (at, provider, model, session, u.prompt_tokens as i64, u.completion_tokens as i64, u.cached_tokens as i64, cost)
}

/// Unix time of 00:00 UTC on the first day of the month containing `t`.
fn month_start(t: i64) -> i64 {
    let days = t.div_euclid(86_400);
    // civil-from-days (Howard Hinnant's algorithm)
    let z = days + 719_468;
    let doe = z.rem_euclid(146_097); // day of the 400-year era
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    (days - (d - 1)) * 86_400
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Mode, Price};
    use lh_provider::Message;

    fn meter(cfg: Config, local: bool) -> SpendMeter {
        SpendMeter::new(Arc::new(Store::open_memory().unwrap()), cfg, "test", local).for_session("s1")
    }
    fn gpt_cfg(mode: Mode) -> Config {
        Config {
            mode,
            daily_cap_usd: Some(0.05),
            prices: vec![Price { model: "gpt-*".into(), input: 2.5, output: 10.0, cached_input: Some(1.25), image: None }],
            ..Default::default()
        }
    }
    fn req(model: &str, max: u32) -> ChatRequest {
        ChatRequest { model: model.into(), messages: vec![Message::user("hello there")], max_tokens: Some(max), ..Default::default() }
    }

    #[test]
    fn month_start_is_the_first_of_the_month() {
        assert_eq!(month_start(0), 0); // 1970-01-01
        assert_eq!(month_start(1791590400 + 4000), 1790812800); // 2026-10-10 -> 2026-10-01
        assert_eq!(month_start(1790812800 - 1), 1788220800); // 2026-09-30 -> 2026-09-01
        assert_eq!(month_start(1709164800 + 5), 1706745600); // leap day
    }

    #[tokio::test]
    async fn the_summary_adds_up_per_provider() {
        let m = meter(gpt_cfg(Mode::None), false);
        m.after(&req("gpt-4o", 100), Some(Usage { prompt_tokens: 0, completion_tokens: 1000, cached_tokens: 0 })).await;
        m.after(&req("mystery", 100), Some(Usage::default())).await;
        let s = m.summary();
        assert_eq!(s.today, 10_000);
        assert_eq!(s.this_month, 10_000);
        assert_eq!((s.today_calls, s.month_calls, s.unpriced), (2, 2, 1));
        assert_eq!(s.by_provider, vec![("test".to_string(), 10_000, 2)]);
        assert_eq!(meter(Config::default(), true).summary(), Summary::default());
    }

    #[test]
    fn cost_uses_cached_price_and_unknown_cloud_models_have_none() {
        let m = meter(gpt_cfg(Mode::Caps), false);
        let c = m.cost("gpt-4o", Usage { prompt_tokens: 1000, completion_tokens: 100, cached_tokens: 400 }).unwrap();
        assert_eq!(c, (600.0 * 2.5 + 400.0 * 1.25 + 100.0 * 10.0) as i64);
        assert_eq!(m.cost("mystery", Usage::default()), None);
        assert_eq!(meter(Config::default(), true).cost("llama", Usage { prompt_tokens: 9, completion_tokens: 9, cached_tokens: 0 }), Some(0));
    }

    #[tokio::test]
    async fn local_models_never_trip_a_cap_and_are_still_recorded() {
        let m = meter(Config { mode: Mode::Caps, daily_cap_usd: Some(0.0), ..Default::default() }, true);
        assert_eq!(m.before(&req("qwen", 100_000)).await, MeterVerdict::Allow);
        m.after(&req("qwen", 100), Some(Usage { prompt_tokens: 50, completion_tokens: 20, cached_tokens: 0 })).await;
        let n: i64 = m.shared.store.with(|c| c.query_row("select count(*) from spend where session='s1' and cost_micro_usd = 0", [], |r| r.get(0))).unwrap();
        assert_eq!(n, 1);
    }

    #[tokio::test]
    async fn a_daily_cap_closes_after_enough_spending() {
        let m = meter(gpt_cfg(Mode::Caps), false);
        assert_eq!(m.before(&req("gpt-4o", 100)).await, MeterVerdict::Allow); // up to ~$0.001
        // spend about $0.049: 4900 output tokens at $10 per million
        m.after(&req("gpt-4o", 100), Some(Usage { prompt_tokens: 0, completion_tokens: 4900, cached_tokens: 0 })).await;
        assert_eq!(m.totals().today, 49_000);
        assert!(matches!(m.before(&req("gpt-4o", 200)).await, MeterVerdict::Refuse(w) if w.contains("today's spending")));
        // the same call in estimate mode asks instead of refusing
        let e = SpendMeter::new(m.shared.store.clone(), gpt_cfg(Mode::Estimate), "test", false).for_session("s2");
        assert!(matches!(e.before(&req("gpt-4o", 200)).await, MeterVerdict::Ask(_)));
        // and an unpriced cloud model asks / is refused
        assert!(matches!(e.before(&req("mystery", 10)).await, MeterVerdict::Ask(_)));
        assert!(matches!(m.before(&req("mystery", 10)).await, MeterVerdict::Refuse(_)));
    }

    #[tokio::test]
    async fn the_estimate_grows_with_the_request() {
        let m = meter(gpt_cfg(Mode::Estimate), false);
        let small = m.estimate(&req("gpt-4o", 10)).unwrap();
        let big = m.estimate(&req("gpt-4o", 10_000)).unwrap();
        assert!(big > small * 100);
    }
}
