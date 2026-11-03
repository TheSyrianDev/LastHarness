//! A department run as the department page shows it: the plan, each step and where it stands, the
//! questions waiting for the user, and the outcome. Built from the events the core streams; no window code.

use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepState {
    Waiting,
    Running,
    Done,
    Failed,
    Parked,
    Stopped,
    Blocked,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    pub id: String,
    pub title: String,
    pub assignee: String,
    pub depends_on: Vec<String>,
    pub state: StepState,
    pub worker: String,
    pub model: String,
    pub attempt: u64,
    /// The latest thing said about the step: the check's verdict, why it retried, what it produced.
    pub note: String,
}

/// Something waiting for the user.
#[derive(Clone, Debug, PartialEq)]
pub enum Question {
    /// Approve the plan.
    Plan { id: String, goal: String },
    /// A milestone was reached: carry on?
    CheckIn { id: String, milestone: String, summaries: Vec<String> },
    /// May this tool run?
    Permission { id: String, tool: String, what: String, reason: String, worker: String },
}

impl Question {
    pub fn id(&self) -> &str {
        match self {
            Question::Plan { id, .. } | Question::CheckIn { id, .. } | Question::Permission { id, .. } => id,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Done { result: String, parked: usize },
    Failed(String),
    Rejected(String),
    Stopped,
}

/// Whether a run started by chat session `run_chat` (none: not by a chat) shows with the chat whose session is `chat`.
pub fn shown_with(run_chat: Option<&str>, chat: Option<&str>) -> bool {
    run_chat.is_none_or(|r| Some(r) == chat)
}

#[derive(Clone, Debug, Default)]
pub struct Run {
    pub id: String,
    pub dept: String,
    pub goal: String,
    pub status: String,
    pub steps: Vec<Step>,
    pub questions: Vec<Question>,
    pub notes: Vec<String>,
    pub milestones: Vec<String>,
    pub outcome: Option<Outcome>,
    pub finished: bool,
}

fn s(v: &Value, k: &str) -> String {
    match &v[k] {
        Value::String(x) => x.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

impl Run {
    pub fn new(id: &str, dept: &str, goal: &str) -> Run {
        Run { id: id.into(), dept: dept.into(), goal: goal.into(), status: "Starting".into(), ..Default::default() }
    }

    fn step(&mut self, id: &str) -> Option<&mut Step> {
        self.steps.iter_mut().find(|x| x.id == id)
    }

    pub fn apply(&mut self, e: &Value) {
        match e["type"].as_str().unwrap_or("") {
            "planning" => self.status = "Planning".into(),
            "plan" => {
                self.steps = e["steps"].as_array().map(|a| a.iter().map(|x| Step { id: s(x, "id"), title: s(x, "title"), assignee: s(x, "assignee"), depends_on: x["depends_on"].as_array().map(|d| d.iter().map(|y| y.to_string().trim_matches('"').to_string()).collect()).unwrap_or_default(), state: StepState::Waiting, worker: String::new(), model: String::new(), attempt: 0, note: String::new() }).collect()).unwrap_or_default();
                self.status = "Plan ready".into();
            }
            "plan_decided" => {
                self.questions.retain(|q| !matches!(q, Question::Plan { .. }));
                self.status = if e["approved"] == true { "Working".into() } else { "Plan not approved".into() };
            }
            "step_started" => {
                self.status = "Working".into();
                let id = s(e, "step");
                if let Some(st) = self.step(&id) {
                    st.state = StepState::Running;
                    st.worker = s(e, "worker");
                    st.model = s(e, "model");
                    st.attempt = e["attempt"].as_u64().unwrap_or(1);
                }
            }
            "step_checked" => {
                let id = s(e, "step");
                let v = s(e, "verdict");
                if let Some(st) = self.step(&id) {
                    st.note = format!("check: {v}");
                }
            }
            "step_retry" => {
                let id = s(e, "step");
                let note = format!("trying again ({}): {}", s(e, "remedy"), s(e, "reason"));
                if let Some(st) = self.step(&id) {
                    st.note = note;
                }
            }
            "step_done" => {
                let id = s(e, "step");
                let sum = s(e, "summary");
                if let Some(st) = self.step(&id) {
                    st.state = StepState::Done;
                    st.note = sum;
                }
            }
            "step_failed" => {
                let id = s(e, "step");
                let r = s(e, "report");
                if let Some(st) = self.step(&id) {
                    st.state = StepState::Failed;
                    st.note = r;
                }
            }
            "step_parked" => {
                let id = s(e, "step");
                let r = s(e, "why");
                if let Some(st) = self.step(&id) {
                    st.state = StepState::Parked;
                    st.note = r;
                }
            }
            "step_stopped" => {
                let id = s(e, "step");
                if let Some(st) = self.step(&id) {
                    st.state = StepState::Stopped;
                }
            }
            "step_blocked" => {
                let id = s(e, "step");
                let r = s(e, "because");
                if let Some(st) = self.step(&id) {
                    st.state = StepState::Blocked;
                    st.note = r;
                }
            }
            "model_note" => self.notes.push(s(e, "note")),
            "model_unavailable" => self.notes.push(format!("{}: {}", s(e, "role"), s(e, "message"))),
            "milestone" => self.milestones.push(s(e, "name")),
            "finished" => {
                self.finished = true;
                if self.outcome.is_none() {
                    self.status = if e["ok"] == true { "Finished".into() } else { "Finished with problems".into() };
                }
            }
            "outcome" => {
                self.finished = true;
                self.questions.clear();
                self.outcome = Some(match e["status"].as_str().unwrap_or("") {
                    "done" => Outcome::Done { result: s(e, "result"), parked: e["parked"].as_array().map(|a| a.len()).unwrap_or(0) },
                    "failed" => Outcome::Failed(s(e, "report")),
                    "rejected" => Outcome::Rejected(s(e, "note")),
                    _ => Outcome::Stopped,
                });
                self.status = match &self.outcome {
                    Some(Outcome::Done { .. }) => "Done",
                    Some(Outcome::Failed(_)) => "Failed",
                    Some(Outcome::Rejected(_)) => "Plan rejected",
                    _ => "Stopped",
                }
                .into();
            }
            "plan_needed" => {
                self.status = "Waiting for you: approve the plan".into();
                self.questions.push(Question::Plan { id: s(e, "id"), goal: s(e, "goal") });
            }
            "checkin_needed" => {
                self.status = "Waiting for you: milestone reached".into();
                self.questions.push(Question::CheckIn { id: s(e, "id"), milestone: s(e, "milestone"), summaries: e["summaries"].as_array().map(|a| a.iter().map(|x| x.as_str().map(str::to_string).unwrap_or_else(|| x.to_string())).collect()).unwrap_or_default() });
            }
            "ask" => self.questions.push(Question::Permission { id: s(e, "id"), tool: s(e, "tool"), what: s(e, "what"), reason: s(e, "reason"), worker: s(e, "worker") }),
            "away_denied" => self.notes.push(format!("Refused while you were away: {} {}", s(e, "tool"), s(e, "what"))),
            _ => {}
        }
    }

    /// The user answered (or the question went away).
    pub fn answered(&mut self, q: &str) {
        self.questions.retain(|x| x.id() != q);
    }

    pub fn is_waiting(&self) -> bool {
        !self.questions.is_empty()
    }

    /// Steps finished over steps planned, for a progress line.
    pub fn progress(&self) -> (usize, usize) {
        (self.steps.iter().filter(|x| x.state == StepState::Done).count(), self.steps.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_chats_run_shows_only_with_that_chat() {
        assert!(shown_with(None, None) && shown_with(None, Some("s1")), "a run from the Departments page shows everywhere");
        assert!(shown_with(Some("s1"), Some("s1")));
        assert!(!shown_with(Some("s1"), Some("s2")) && !shown_with(Some("s1"), None));
    }

    fn run_with(events: &[Value]) -> Run {
        let mut r = Run::new("r1", "pets", "explain purring");
        for e in events {
            r.apply(e);
        }
        r
    }

    fn plan() -> Value {
        json!({"type":"plan","dept":"pets","steps":[{"id":1,"title":"Draft","assignee":"writer","brief":"b","depends_on":[],"milestone":null},{"id":2,"title":"Edit","assignee":"writer","brief":"b","depends_on":[1],"milestone":"Two sentences"}]})
    }

    #[test]
    fn a_plan_becomes_steps_that_move_through_their_states() {
        let r = run_with(&[json!({"type":"planning"}), plan(), json!({"type":"step_started","step":1,"title":"Draft","worker":"writer","model":"Qwen","attempt":1}), json!({"type":"step_checked","step":1,"verdict":"ok"}), json!({"type":"step_done","step":1,"summary":"Cats purr."})]);
        assert_eq!(r.steps.len(), 2);
        assert_eq!(r.steps[0].state, StepState::Done);
        assert_eq!(r.steps[0].note, "Cats purr.");
        assert_eq!(r.steps[0].worker, "writer");
        assert_eq!(r.steps[1].state, StepState::Waiting);
        assert_eq!(r.steps[1].depends_on, vec!["1"]);
        assert_eq!(r.progress(), (1, 2));
        assert_eq!(r.status, "Working");
    }

    #[test]
    fn retries_failures_parking_and_blocking_are_recorded_on_the_step() {
        let r = run_with(&[plan(), json!({"type":"step_started","step":1,"title":"t","worker":"w","model":"m","attempt":1}), json!({"type":"step_retry","step":1,"remedy":"Retry","reason":"the result misses the step"}), json!({"type":"step_failed","step":1,"report":"gave up"}), json!({"type":"step_blocked","step":2,"because":"step 1 failed"})]);
        assert_eq!(r.steps[0].state, StepState::Failed);
        assert_eq!(r.steps[0].note, "gave up");
        assert_eq!(r.steps[1].state, StepState::Blocked);
        let r = run_with(&[plan(), json!({"type":"step_parked","step":2,"why":"needs you"}), json!({"type":"step_stopped","step":1})]);
        assert_eq!(r.steps[1].state, StepState::Parked);
        assert_eq!(r.steps[0].state, StepState::Stopped);
        let r = run_with(&[plan(), json!({"type":"step_retry","step":1,"remedy":"Retry","reason":"r"})]);
        assert!(r.steps[0].note.contains("trying again (Retry): r"));
    }

    #[test]
    fn questions_wait_until_answered_and_the_plan_one_goes_when_decided() {
        let mut r = run_with(&[plan(), json!({"type":"plan_needed","dept":"pets","goal":"g","steps":[],"id":"q1"}), json!({"type":"ask","tool":"write_file","what":"Write(x)","reason":"changes files","worker":"writer","id":"q2"})]);
        assert!(r.is_waiting());
        assert_eq!(r.questions.len(), 2);
        assert!(r.status.contains("approve the plan"));
        r.apply(&json!({"type":"plan_decided","dept":"pets","approved":true,"note":""}));
        assert_eq!(r.questions.len(), 1, "the plan question is gone, the permission one stays");
        r.answered("q2");
        assert!(!r.is_waiting());
        r.apply(&json!({"type":"checkin_needed","dept":"pets","milestone":"Two sentences","summaries":["a","b"],"id":"q3"}));
        assert!(matches!(&r.questions[0], Question::CheckIn { milestone, summaries, .. } if milestone == "Two sentences" && summaries.len() == 2));
        r.apply(&json!({"type":"plan_decided","approved":false,"note":"no"}));
        assert_eq!(r.status, "Plan not approved");
    }

    #[test]
    fn the_outcome_ends_the_run_and_clears_every_open_question() {
        let mut r = run_with(&[json!({"type":"ask","tool":"t","what":"w","reason":"","worker":"","id":"q1"}), json!({"type":"milestone","dept":"pets","name":"Draft"})]);
        assert_eq!(r.milestones, vec!["Draft"]);
        r.apply(&json!({"type":"outcome","status":"done","result":"All good","steps":[],"parked":["x"]}));
        assert!(r.finished && r.questions.is_empty());
        assert_eq!(r.outcome, Some(Outcome::Done { result: "All good".into(), parked: 1 }));
        assert_eq!(r.status, "Done");
        for (status, want) in [("failed", "Failed"), ("rejected", "Plan rejected"), ("stopped", "Stopped")] {
            let r = run_with(&[json!({"type":"outcome","status":status,"report":"r","note":"n"})]);
            assert_eq!(r.status, want);
        }
        let r = run_with(&[json!({"type":"outcome","status":"failed","report":"broke"})]);
        assert_eq!(r.outcome, Some(Outcome::Failed("broke".into())));
    }

    #[test]
    fn notes_and_unknown_events_do_not_break_anything() {
        let r = run_with(&[json!({"type":"model_note","note":"used the fallback"}), json!({"type":"model_unavailable","role":"writer","message":"offline"}), json!({"type":"away_denied","tool":"run_command","what":"rm x","worker":"w","reason":"r"}), json!({"type":"something_new"}), json!({"type":"finished","dept":"pets","ok":false})]);
        assert_eq!(r.notes.len(), 3);
        assert!(r.finished);
        assert_eq!(r.status, "Finished with problems");
    }
}
