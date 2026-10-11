use lh_store::{now, Store};

use crate::{Error, Result};

/// What to do when the model a role needs is not there (spec section 5.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnavailablePolicy {
    /// Stop with a clear message and let the user pick another model now. The default.
    Stop,
    /// Try the type's fallback list in order and say so.
    Fallback,
}

/// How much the user wants to be asked on a big goal (spec section 8.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Autonomy {
    /// Show the plan and wait for approval before anything runs. The default.
    PlanFirst,
    /// After approval, pause at each milestone for a go-ahead.
    CheckIns,
    /// Run the plan without stopping.
    FireAndForget,
}

impl Autonomy {
    pub fn as_str(self) -> &'static str {
        match self {
            Autonomy::PlanFirst => "plan-first",
            Autonomy::CheckIns => "check-ins",
            Autonomy::FireAndForget => "fire-and-forget",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        [Autonomy::PlanFirst, Autonomy::CheckIns, Autonomy::FireAndForget].into_iter().find(|a| a.as_str() == s)
    }
}

/// The team settings, kept in the local store so they survive restarts.
pub struct TeamSettings<'a> {
    pub(crate) store: &'a Store,
}

pub const MAX_PARALLEL: usize = 16;
pub const MAX_ATTEMPTS: u32 = 10;

impl<'a> TeamSettings<'a> {
    pub fn new(store: &'a Store) -> Self {
        Self { store }
    }

    fn get(&self, key: &str) -> Result<Option<String>> {
        use lh_store::rusqlite::OptionalExtension;
        Ok(self.store.with(|c| c.query_row("select value from settings where key = ?1", [key], |r| r.get(0)).optional())?)
    }

    fn put(&self, key: &str, value: &str) -> Result<()> {
        self.store.with(|c| c.execute("insert into settings(key, value, updated) values (?1, ?2, ?3) on conflict(key) do update set value = excluded.value, updated = excluded.updated", (key, value, now())).map(|_| ()))?;
        let _ = self.store.log("setting_changed", "-", "user", &serde_json::json!({"key": key, "value": value}));
        Ok(())
    }

    pub fn unavailable(&self) -> Result<UnavailablePolicy> {
        Ok(if self.get("unavailable")?.as_deref() == Some("fallback") { UnavailablePolicy::Fallback } else { UnavailablePolicy::Stop })
    }
    pub fn set_unavailable(&self, p: UnavailablePolicy) -> Result<()> {
        self.put("unavailable", if p == UnavailablePolicy::Fallback { "fallback" } else { "stop" })
    }

    /// Workers running at once. 1 means one after the other; more means that many at a time, the rest queue.
    pub fn parallel(&self) -> Result<usize> {
        Ok(self.get("parallel")?.and_then(|v| v.parse().ok()).filter(|n| (1..=MAX_PARALLEL).contains(n)).unwrap_or(1))
    }
    pub fn set_parallel(&self, n: usize) -> Result<()> {
        if !(1..=MAX_PARALLEL).contains(&n) {
            return Err(Error::Invalid(format!("parallel workers must be between 1 and {MAX_PARALLEL}")));
        }
        self.put("parallel", &n.to_string())
    }

    pub fn autonomy(&self) -> Result<Autonomy> {
        Ok(self.get("autonomy")?.and_then(|v| Autonomy::parse(&v)).unwrap_or(Autonomy::PlanFirst))
    }
    pub fn set_autonomy(&self, a: Autonomy) -> Result<()> {
        self.put("autonomy", a.as_str())
    }

    /// Attempts per step before a failure goes up to the caller (including retries and spawned subagents).
    pub fn attempts(&self) -> Result<u32> {
        Ok(self.get("attempts")?.and_then(|v| v.parse().ok()).filter(|n| (1..=MAX_ATTEMPTS).contains(n)).unwrap_or(3))
    }
    pub fn set_attempts(&self, n: u32) -> Result<()> {
        if !(1..=MAX_ATTEMPTS).contains(&n) {
            return Err(Error::Invalid(format!("attempts must be between 1 and {MAX_ATTEMPTS}")));
        }
        self.put("attempts", &n.to_string())
    }

    /// Away mode: the user is not there, so nobody is asked anything. `Some(since)` while it is on.
    pub fn away_since(&self) -> Result<Option<i64>> {
        Ok(self.get("away_since")?.and_then(|v| v.parse().ok()).filter(|t| *t > 0))
    }
    pub fn set_away(&self, on: bool) -> Result<()> {
        self.put("away_since", &if on { now() } else { 0 }.to_string())
    }
}
