//! Reward signals: small records of how things went, kept for later (a future training run, or just to see what
//! works). Nothing is sent anywhere and nothing is trained here. Each record is a line in the activity log with
//! kind `reward`: which signal, a value from -1 to 1, and the chat it belongs to. No conversation text is kept.
//!
//! Signals: `thumbs` (up 1, down -1), `proposal` (a learning or memory suggestion accepted 1, rejected -1),
//! `tests` (a test command passed 1, failed -1), `undone` (the user put files back, -1) and `declined` (the user
//! said no to a tool, -0.5).

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use lh_tools::Model;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{ApiError, AppState};

/// Write one reward record. Incognito chats leave none.
pub(crate) fn reward(store: &lh_store::Store, session: &str, signal: &str, value: f32, incognito: bool) {
    if incognito {
        return;
    }
    let _ = store.log("reward", session, "user", &json!({"signal": signal, "value": value}));
}

/// Does this command run tests? Then its exit code is a reward signal.
pub(crate) fn is_test_command(cmd: &str) -> bool {
    const RUNNERS: &[&str] = &["cargo test", "cargo nextest", "pytest", "python -m pytest", "python3 -m pytest", "npm test", "npm run test", "yarn test", "pnpm test", "go test", "make test", "make check", "ctest", "jest", "vitest", "mix test", "bun test", "deno test"];
    let c = cmd.split_whitespace().collect::<Vec<_>>().join(" ");
    RUNNERS.iter().any(|r| c.starts_with(r) || c.contains(&format!("&& {r}")) || c.contains(&format!("; {r}")))
}

/// The exit code a `run_command` result starts with.
pub(crate) fn exit_code(result: &str) -> Option<i32> {
    result.strip_prefix("exit code ")?.split_whitespace().next()?.parse().ok()
}

pub(crate) fn routes<M: Model + 'static>() -> Router<Arc<AppState<M>>> {
    Router::new().route("/v1/rewards", get(list::<M>))
}

#[derive(Deserialize)]
struct Q {
    limit: Option<u32>,
}

/// The newest reward records, with a sum per signal.
async fn list<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Query(q): Query<Q>) -> Result<Json<Value>, ApiError> {
    let entries = st.settings.store.recent(q.limit.unwrap_or(500).min(10_000), Some("reward")).map_err(|e| ApiError(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let mut totals = serde_json::Map::new();
    let rows: Vec<Value> = entries
        .iter()
        .map(|e| {
            let d = &e.detail;
            let (sig, v) = (d["signal"].as_str().unwrap_or("").to_string(), d["value"].as_f64().unwrap_or(0.0));
            let t = totals.entry(sig.clone()).or_insert(json!({"count": 0, "sum": 0.0}));
            t["count"] = json!(t["count"].as_i64().unwrap_or(0) + 1);
            t["sum"] = json!(t["sum"].as_f64().unwrap_or(0.0) + v);
            json!({"at": e.at, "session": e.session, "signal": sig, "value": v})
        })
        .collect();
    Ok(Json(json!({"rewards": rows, "totals": totals})))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commands_and_exit_codes_are_recognised() {
        for c in ["cargo test -p x", "cd a && cargo test", "pytest -q", "npm test", "go  test ./..."] {
            assert!(is_test_command(c), "{c}");
        }
        for c in ["ls", "cargo build", "echo pytest", "cat test.txt"] {
            assert!(!is_test_command(c), "{c}");
        }
        assert_eq!((exit_code("exit code 0\nok"), exit_code("exit code 101\n"), exit_code("exit code none (killed by a signal)"), exit_code("x")), (Some(0), Some(101), None, None));
    }
}
