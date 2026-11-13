//! Live plan checklist. Folds the team event stream into one row per planned step with its state, so a
//! front end can show progress without reading every event. Pure: no I/O, no model.

use std::collections::BTreeMap;

use crate::engine::TeamEvent;
use crate::plan::PlanStep;
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StepState {
    Waiting,
    Running,
    Done,
    Failed,
    Parked,
    Stopped,
    Blocked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Row {
    pub id: String,
    pub title: String,
    pub state: StepState,
}

/// The checklist for one department run, rows in plan order. Steps the events never mention stay Waiting.
pub fn fold(events: &[TeamEvent]) -> Vec<Row> {
    let mut order: Vec<String> = Vec::new();
    let mut rows: BTreeMap<String, Row> = BTreeMap::new();
    for e in events {
        match e {
            TeamEvent::Plan { steps, .. } => {
                for s in steps {
                    if !rows.contains_key(&s.id) {
                        order.push(s.id.clone());
                        rows.insert(s.id.clone(), Row { id: s.id.clone(), title: s.title.clone(), state: StepState::Waiting });
                    }
                }
            }
            TeamEvent::StepStarted { step, title, .. } => set(&mut rows, &mut order, step, Some(title), StepState::Running),
            TeamEvent::StepDone { step, .. } => set(&mut rows, &mut order, step, None, StepState::Done),
            TeamEvent::StepFailed { step, .. } => set(&mut rows, &mut order, step, None, StepState::Failed),
            TeamEvent::StepParked { step, .. } => set(&mut rows, &mut order, step, None, StepState::Parked),
            TeamEvent::StepStopped { step } => set(&mut rows, &mut order, step, None, StepState::Stopped),
            TeamEvent::StepBlocked { step, .. } => set(&mut rows, &mut order, step, None, StepState::Blocked),
            _ => {}
        }
    }
    order.iter().filter_map(|id| rows.get(id).cloned()).collect()
}

/// The same fold, for events as the run keeps them: the JSON from `TeamEvent::to_json`. Events it does not
/// know about, and the ones the fold ignores anyway, are skipped.
pub fn fold_json(events: &[Value]) -> Vec<Row> {
    let parsed: Vec<TeamEvent> = events.iter().filter_map(from_json).collect();
    fold(&parsed)
}

fn from_json(v: &Value) -> Option<TeamEvent> {
    let s = |k: &str| v[k].as_str().unwrap_or("").to_string();
    Some(match v["type"].as_str()? {
        "plan" => TeamEvent::Plan {
            dept: s("dept"),
            steps: v["steps"]
                .as_array()?
                .iter()
                .map(|x| PlanStep {
                    id: x["id"].as_str().unwrap_or("").to_string(),
                    title: x["title"].as_str().unwrap_or("").to_string(),
                    assignee: x["assignee"].as_str().unwrap_or("").to_string(),
                    brief: x["brief"].as_str().unwrap_or("").to_string(),
                    depends_on: x["depends_on"].as_array().map(|d| d.iter().filter_map(|y| y.as_str().map(str::to_string)).collect()).unwrap_or_default(),
                    milestone: x["milestone"].as_str().map(str::to_string),
                })
                .collect(),
        },
        "step_started" => TeamEvent::StepStarted { dept: s("dept"), step: s("step"), title: s("title"), worker: s("worker"), model: s("model"), attempt: v["attempt"].as_u64().unwrap_or(1) as u32 },
        "step_done" => TeamEvent::StepDone { step: s("step"), summary: s("summary") },
        "step_failed" => TeamEvent::StepFailed { step: s("step"), report: s("report") },
        "step_parked" => TeamEvent::StepParked { step: s("step"), why: s("why") },
        "step_stopped" => TeamEvent::StepStopped { step: s("step") },
        "step_blocked" => TeamEvent::StepBlocked { step: s("step"), because: s("because") },
        _ => return None,
    })
}

fn set(rows: &mut BTreeMap<String, Row>, order: &mut Vec<String>, id: &str, title: Option<&str>, state: StepState) {
    if !rows.contains_key(id) {
        order.push(id.to_string());
    }
    let row = rows.entry(id.to_string()).or_insert_with(|| Row { id: id.to_string(), title: id.to_string(), state: StepState::Waiting });
    if let Some(t) = title {
        row.title = t.to_string();
    }
    row.state = state;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::PlanStep;

    fn step(id: &str, title: &str) -> PlanStep {
        PlanStep { id: id.into(), title: title.into(), assignee: "dev".into(), brief: String::new(), depends_on: vec![], milestone: None }
    }

    #[test]
    fn rows_follow_plan_order_and_latest_state_wins() {
        let events = vec![
            TeamEvent::Plan { dept: "d".into(), steps: vec![step("1", "write"), step("2", "test"), step("3", "ship")] },
            TeamEvent::StepStarted { dept: "d".into(), step: "1".into(), title: "write".into(), worker: "w".into(), model: "m".into(), attempt: 1 },
            TeamEvent::StepDone { step: "1".into(), summary: "ok".into() },
            TeamEvent::StepStarted { dept: "d".into(), step: "2".into(), title: "test".into(), worker: "w".into(), model: "m".into(), attempt: 1 },
            TeamEvent::StepFailed { step: "2".into(), report: "red".into() },
        ];
        let rows = fold(&events);
        let states: Vec<(&str, StepState)> = rows.iter().map(|r| (r.id.as_str(), r.state.clone())).collect();
        assert_eq!(states, vec![("1", StepState::Done), ("2", StepState::Failed), ("3", StepState::Waiting)]);
        assert_eq!(rows[2].title, "ship");
    }

    #[test]
    fn json_events_fold_the_same_as_typed_ones() {
        let events = vec![
            TeamEvent::Plan { dept: "d".into(), steps: vec![step("1", "write"), step("2", "test")] },
            TeamEvent::StepStarted { dept: "d".into(), step: "1".into(), title: "write".into(), worker: "w".into(), model: "m".into(), attempt: 1 },
            TeamEvent::StepDone { step: "1".into(), summary: "ok".into() },
            TeamEvent::StepBlocked { step: "2".into(), because: "waits".into() },
            TeamEvent::ModelNote { note: "ignored".into() },
        ];
        let json: Vec<Value> = events.iter().map(|e| e.to_json()).collect();
        assert_eq!(fold_json(&json), fold(&events));
        assert_eq!(fold_json(&json)[1].state, StepState::Blocked);
    }
}
