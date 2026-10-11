//! Plans, and the prompts a lead uses: to plan a goal, to check a result, to decide what to do about a
//! failure, and to sum up. Each prompt starts with a tag in brackets so the kind of call is clear in
//! logs and in tests. The readers of the model's answers are strict about structure and forgiving about
//! the prose around it.

use std::collections::{HashMap, HashSet};

use serde_json::Value;

use crate::{Department, Target};

pub const MAX_STEPS: usize = 20;

#[derive(Clone, Debug, PartialEq)]
pub struct PlanStep {
    pub id: String,
    pub title: String,
    /// A member of the department: a role name, or `dept:name` for a sub-department.
    pub assignee: String,
    pub brief: String,
    pub depends_on: Vec<String>,
    /// Steps of one milestone finish together; with check-ins on, the user is asked at the end of each.
    pub milestone: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub goal: String,
    pub steps: Vec<PlanStep>,
}

fn json_in(reply: &str, open: char, close: char) -> Option<Value> {
    let (a, b) = (reply.find(open)?, reply.rfind(close)?);
    (b > a).then(|| serde_json::from_str(&reply[a..=b]).ok()).flatten()
}

pub fn plan_prompt(goal: &str, dept: &Department, members: &[(String, String)], context: &str) -> String {
    let mut s = format!(
        "[PLAN] You lead the \"{}\" department. {}\nSplit the goal into small steps and give each step to the best member. Independent steps can run at the same time. A step sees only its own brief and the results of the steps listed in its depends_on, nothing else, so list every step whose result it needs and write each brief so it makes sense alone.\n\nGoal: {goal}\n",
        dept.name, dept.instructions
    );
    if !context.trim().is_empty() {
        s.push_str(&format!("\nContext:\n{}\n", context.trim()));
    }
    s.push_str("\nMembers:\n");
    for (n, d) in members {
        s.push_str(&format!("- {n}: {d}\n"));
    }
    s.push_str(&format!(
        "\nAnswer with ONLY JSON: {{\"steps\":[{{\"id\":\"1\",\"title\":\"short title\",\"assignee\":\"<member name>\",\"brief\":\"what to do and what to hand back\",\"depends_on\":[],\"milestone\":\"optional name\"}}]}}. At most {MAX_STEPS} steps. Every assignee must be a member listed above."
    ));
    s
}

/// Read and check a plan. `members` are the names a step may be given to. The error is plain text that
/// can be sent back to the lead so it can correct the plan.
pub fn parse_plan(reply: &str, members: &[String]) -> Result<Vec<PlanStep>, String> {
    let v = json_in(reply, '{', '}').or_else(|| json_in(reply, '[', ']')).ok_or("the answer holds no JSON plan")?;
    let arr = v["steps"].as_array().or_else(|| v.as_array()).ok_or("the plan needs a \"steps\" list")?;
    if arr.is_empty() {
        return Err("the plan has no steps".into());
    }
    if arr.len() > MAX_STEPS {
        return Err(format!("the plan has {} steps; the most allowed is {MAX_STEPS}", arr.len()));
    }
    let text = |x: &Value, k: &str| x[k].as_str().map(|s| s.split_whitespace().collect::<Vec<_>>().join(" ")).filter(|s| !s.is_empty());
    let id_of = |x: &Value| match &x["id"] {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    };
    let mut steps: Vec<PlanStep> = vec![];
    for (i, x) in arr.iter().enumerate() {
        let id = id_of(x).ok_or(format!("step {} has no id", i + 1))?;
        if steps.iter().any(|s| s.id == id) {
            return Err(format!("the id {id} is used twice"));
        }
        let assignee = text(x, "assignee").ok_or(format!("step {id} has no assignee"))?;
        // "design" is accepted for "dept:design" when no role has that name
        let assignee = if members.contains(&assignee) {
            assignee
        } else if members.contains(&format!("dept:{assignee}")) {
            format!("dept:{assignee}")
        } else {
            return Err(format!("step {id} is given to \"{assignee}\", who is not a member. Members: {}", members.join(", ")));
        };
        let depends_on: Vec<String> = x["depends_on"].as_array().map(|a| a.iter().filter_map(|d| d.as_str().map(str::to_string).or_else(|| d.as_i64().map(|n| n.to_string()))).collect()).unwrap_or_default();
        steps.push(PlanStep { title: text(x, "title").unwrap_or_else(|| format!("Step {id}")), brief: text(x, "brief").ok_or(format!("step {id} has no brief"))?, milestone: text(x, "milestone"), id, assignee, depends_on });
    }
    let ids: HashSet<&str> = steps.iter().map(|s| s.id.as_str()).collect();
    for s in &steps {
        if let Some(d) = s.depends_on.iter().find(|d| !ids.contains(d.as_str())) {
            return Err(format!("step {} waits for {d}, which is not a step", s.id));
        }
        if s.depends_on.contains(&s.id) {
            return Err(format!("step {} waits for itself", s.id));
        }
    }
    // no cycles: peel off steps whose dependencies are all peeled
    let mut left: HashMap<&str, Vec<&str>> = steps.iter().map(|s| (s.id.as_str(), s.depends_on.iter().map(String::as_str).collect())).collect();
    while !left.is_empty() {
        let ready: Vec<&str> = left.iter().filter(|(_, d)| d.iter().all(|x| !left.contains_key(x))).map(|(k, _)| *k).collect();
        if ready.is_empty() {
            return Err("the steps wait for each other in a circle".into());
        }
        for r in ready {
            left.remove(r);
        }
    }
    Ok(steps)
}

// ---- checking a result ----

#[derive(Clone, Debug, PartialEq)]
pub enum Check {
    Ok,
    /// Only small mistakes: fix them in place.
    Small(String),
    /// A larger miss: this is a failure.
    Fail(String),
}

pub fn check_prompt(goal: &str, step: &PlanStep, summary: &str, result: &str) -> String {
    format!(
        "[CHECK] Check a worker's result against the step it was given and the overall goal.\n\nGoal: {goal}\nStep: {}\nBrief: {}\n\nThe worker's summary: {summary}\n\nStart of the result:\n{}\n\nAnswer with ONE line: OK if it meets the step and fits the goal; SMALL: <one sentence on what to fix> if only small mistakes remain; FAIL: <what is wrong> if it misses the step or goes off the goal in a larger way.",
        step.title,
        step.brief,
        excerpt(result, 2500)
    )
}

pub fn parse_check(reply: &str) -> Check {
    let line = reply.lines().map(|l| l.trim().trim_matches(|c: char| c == '*' || c == '`' || c == '"')).find(|l| !l.is_empty()).unwrap_or("");
    let upper = line.to_uppercase();
    let reason = || line.split_once(':').map(|(_, r)| r.trim().to_string()).filter(|r| !r.is_empty()).unwrap_or_else(|| "no reason given".into());
    if upper.starts_with("SMALL") {
        Check::Small(reason())
    } else if upper.starts_with("FAIL") {
        Check::Fail(reason())
    } else {
        Check::Ok // OK, or an answer that cannot be read: do not block good work on a muddled check
    }
}

// ---- handling a failure ----

#[derive(Clone, Debug, PartialEq)]
pub enum Remedy {
    Retry,
    ChangeModel(Target),
    DoItYourself,
    Subagent,
    HandOff(String),
    GiveUp,
}

pub fn failure_prompt(step: &PlanStep, worker: &str, reason: &str, attempt: u32, cap: u32, members: &[String]) -> String {
    format!(
        "[FAILURE] A step failed and you are responsible for it.\n\nStep: {}\nBrief: {}\nWorker: {worker}\nWhat went wrong: {reason}\nAttempts used: {attempt} of {cap}\n\nChoose ONE and answer with one line:\nRETRY | CHANGE_MODEL <model type or exact model> | DO_IT_YOURSELF | SUBAGENT | HAND_OFF <member> | GIVE_UP\nMembers: {}",
        step.title,
        step.brief,
        members.join(", ")
    )
}

pub fn parse_remedy(reply: &str, members: &[String], types: &[String]) -> Remedy {
    let line = reply.lines().map(|l| l.trim().trim_matches(|c: char| c == '*' || c == '`' || c == '"')).find(|l| !l.is_empty()).unwrap_or("");
    let mut it = line.splitn(2, char::is_whitespace);
    let word = it.next().unwrap_or("").trim_end_matches(':').to_uppercase();
    let arg = it.next().unwrap_or("").trim().trim_matches(|c: char| c == '<' || c == '>' || c == '`' || c == '"').to_string();
    match word.as_str() {
        "RETRY" => Remedy::Retry,
        "DO_IT_YOURSELF" | "DO_IT_MYSELF" => Remedy::DoItYourself,
        "SUBAGENT" | "SPAWN_SUBAGENT" => Remedy::Subagent,
        "GIVE_UP" => Remedy::GiveUp,
        "HAND_OFF" if members.contains(&arg) => Remedy::HandOff(arg),
        "CHANGE_MODEL" if !arg.is_empty() => Remedy::ChangeModel(if types.contains(&arg.to_lowercase()) { Target::Type(arg.to_lowercase()) } else { Target::Model(arg) }),
        _ => Remedy::Retry, // an answer that cannot be read: try again rather than give up
    }
}

pub fn final_prompt(goal: &str, steps: &[(String, String)], parked: &[String]) -> String {
    let mut s = format!("[FINAL] All steps are finished. Write the department's answer to the goal: what was done, where the results are, and anything the user must know. Be brief and plain. State only what the summaries show: do not say a file was saved or a command was run unless a summary says so, and say plainly what was refused or left undone.\n\nGoal: {goal}\n\nWhat each step handed back:\n");
    for (t, sum) in steps {
        s.push_str(&format!("- {t}: {sum}\n"));
    }
    if !parked.is_empty() {
        s.push_str(&format!("\nParts that were set aside for the user, because nobody was there to decide:\n{}\n", parked.iter().map(|p| format!("- {p}")).collect::<Vec<_>>().join("\n")));
    }
    s
}

pub fn excerpt(t: &str, n: usize) -> String {
    let flat = t.trim();
    if flat.chars().count() <= n { flat.to_string() } else { format!("{}…", flat.chars().take(n).collect::<String>()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn members() -> Vec<String> {
        ["researcher", "writer", "dept:design"].iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_plan_is_read_from_the_models_answer_and_checked() {
        let reply = "Here is the plan:\n```json\n{\"steps\":[\
            {\"id\":1,\"title\":\"Research\",\"assignee\":\"researcher\",\"brief\":\"find facts\",\"depends_on\":[]},\
            {\"id\":\"2\",\"title\":\"Design\",\"assignee\":\"design\",\"brief\":\"make a layout\",\"depends_on\":[1],\"milestone\":\"draft\"},\
            {\"id\":\"3\",\"title\":\"Write\",\"assignee\":\"writer\",\"brief\":\"write it\",\"depends_on\":[\"1\",2],\"milestone\":\"draft\"}]}\n```";
        let p = parse_plan(reply, &members()).unwrap();
        assert_eq!(p.iter().map(|s| (s.id.as_str(), s.assignee.as_str(), s.depends_on.len())).collect::<Vec<_>>(), [("1", "researcher", 0), ("2", "dept:design", 1), ("3", "writer", 2)]);
        assert_eq!(p[1].milestone.as_deref(), Some("draft"));
    }

    #[test]
    fn a_bad_plan_is_refused_with_a_reason_the_lead_can_act_on() {
        let one = |extra: &str| format!("{{\"steps\":[{{\"id\":\"1\",\"assignee\":\"researcher\",\"brief\":\"b\"{extra}}}]}}");
        let why = |r: &str| parse_plan(r, &members()).unwrap_err();
        assert!(why("no json here").contains("no JSON"));
        assert!(why("{\"steps\":[]}").contains("no steps"));
        assert!(why("{\"steps\":[{\"id\":\"1\",\"assignee\":\"ghost\",\"brief\":\"b\"}]}").contains("not a member"));
        assert!(why("{\"steps\":[{\"id\":\"1\",\"assignee\":\"writer\"}]}").contains("no brief"));
        assert!(why("{\"steps\":[{\"assignee\":\"writer\",\"brief\":\"b\"}]}").contains("no id"));
        assert!(why("{\"steps\":[{\"id\":\"1\",\"assignee\":\"writer\",\"brief\":\"b\"},{\"id\":\"1\",\"assignee\":\"writer\",\"brief\":\"b\"}]}").contains("used twice"));
        assert!(why(&one(",\"depends_on\":[\"9\"]")).contains("not a step"));
        assert!(why(&one(",\"depends_on\":[\"1\"]")).contains("waits for itself"));
        let circle = "{\"steps\":[{\"id\":\"a\",\"assignee\":\"writer\",\"brief\":\"b\",\"depends_on\":[\"b\"]},{\"id\":\"b\",\"assignee\":\"writer\",\"brief\":\"b\",\"depends_on\":[\"a\"]}]}";
        assert!(why(circle).contains("circle"));
        let many = format!("{{\"steps\":[{}]}}", (0..21).map(|i| format!("{{\"id\":\"{i}\",\"assignee\":\"writer\",\"brief\":\"b\"}}")).collect::<Vec<_>>().join(","));
        assert!(why(&many).contains("most allowed"));
    }

    #[test]
    fn check_answers_are_read_and_a_muddled_one_does_not_block_good_work() {
        assert_eq!(parse_check("OK"), Check::Ok);
        assert_eq!(parse_check("**SMALL**: add the missing title"), Check::Small("add the missing title".into()));
        assert_eq!(parse_check("Fail: it answers a different question"), Check::Fail("it answers a different question".into()));
        assert_eq!(parse_check("looks fine to me, I think"), Check::Ok);
        assert_eq!(parse_check(""), Check::Ok);
        assert_eq!(parse_check("\nSMALL"), Check::Small("no reason given".into()));
    }

    #[test]
    fn failure_choices_are_read_and_unreadable_ones_mean_try_again() {
        let m: Vec<String> = vec!["writer".into(), "coder".into()];
        let t: Vec<String> = vec!["fastest".into(), "coder".into()];
        assert_eq!(parse_remedy("RETRY", &m, &t), Remedy::Retry);
        assert_eq!(parse_remedy("CHANGE_MODEL fastest", &m, &t), Remedy::ChangeModel(Target::Type("fastest".into())));
        assert_eq!(parse_remedy("CHANGE_MODEL <big-exact-model>", &m, &t), Remedy::ChangeModel(Target::Model("big-exact-model".into())));
        assert_eq!(parse_remedy("**HAND_OFF** coder", &m, &t), Remedy::Retry, "bold markers around the word still read as that word");
        assert_eq!(parse_remedy("HAND_OFF coder", &m, &t), Remedy::HandOff("coder".into()));
        assert_eq!(parse_remedy("HAND_OFF ghost", &m, &t), Remedy::Retry, "not a member");
        assert_eq!(parse_remedy("DO_IT_YOURSELF", &m, &t), Remedy::DoItYourself);
        assert_eq!(parse_remedy("subagent", &m, &t), Remedy::Subagent);
        assert_eq!(parse_remedy("GIVE_UP: hopeless", &m, &t), Remedy::GiveUp);
        assert_eq!(parse_remedy("hmm, let me think", &m, &t), Remedy::Retry);
    }
}
