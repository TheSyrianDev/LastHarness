//! Live plan checklist. Folds the team event stream into one row per planned step with its state, so a
//! front end can show progress without reading every event. Pure: no I/O, no model.

use std::collections::BTreeMap;

use crate::engine::TeamEvent;
use serde::Serialize;

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
}
