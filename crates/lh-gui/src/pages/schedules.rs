//! Scheduled tasks: what the core's `/v1/schedules` lists, and the body for adding one.

use serde_json::{json, Value};

use super::s;

#[derive(Clone, Debug, PartialEq)]
pub struct Task {
    pub id: i64,
    pub name: String,
    pub prompt: String,
    pub when: String,
    pub model: String,
    pub enabled: bool,
    pub last_result: String,
}

pub fn tasks(v: &Value) -> Vec<Task> {
    v["schedules"].as_array().into_iter().flatten().map(|t| Task {
        id: t["id"].as_i64().unwrap_or(0),
        name: s(t, "name"),
        prompt: s(t, "prompt"),
        when: s(t, "when"),
        model: s(t, "model"),
        enabled: t["enabled"] != false,
        last_result: s(t, "last_result"),
    }).collect()
}

/// The body for a new task, or what is wrong with the form. An empty folder means the home folder.
pub fn new_task(when: &str, task: &str, model: &str, folder: &str) -> Result<Value, String> {
    lh_store::schedules::When::parse(when)?;
    if task.trim().is_empty() {
        return Err("Write what the task should do.".into());
    }
    if model.trim().is_empty() {
        return Err("Choose a model for the task.".into());
    }
    let folder = if folder.trim().is_empty() { std::env::var("HOME").unwrap_or_else(|_| "/".into()) } else { folder.trim().to_string() };
    Ok(json!({"name": "", "prompt": task.trim(), "folder": folder, "model": model.trim(), "when": when.trim()}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_task_is_checked_before_it_is_sent() {
        assert!(new_task("every 30m", "tidy", "m", "/tmp").is_ok());
        assert!(new_task("daily 08:00", "tidy", "m", "").unwrap()["folder"].as_str().is_some_and(|f| !f.is_empty()));
        assert!(new_task("sometimes", "tidy", "m", "").unwrap_err().contains("every 30m"));
        assert!(new_task("every 1d", " ", "m", "").is_err());
        assert!(new_task("every 1d", "x", "", "").is_err());
    }

    #[test]
    fn the_list_reads() {
        let v = json!({"schedules": [{"id": 3, "name": "n", "prompt": "p", "when": "every 2h", "model": "m", "enabled": false, "last_result": "ok"}]});
        let t = &tasks(&v)[0];
        assert_eq!((t.id, t.when.as_str(), t.enabled), (3, "every 2h", false));
    }
}
