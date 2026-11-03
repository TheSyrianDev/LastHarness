//! Export and import: memory, rules, the style profile, roles and departments, as plain files in a
//! folder you choose. Importing shows what is in the folder first, and never replaces anything of yours
//! unless you name it. The folder holds your private notes, so keep it where you keep private things.

use std::path::Path as FsPath;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use lh_learn::{NewRule, Status};
use lh_memory::Privacy;
use lh_tools::Model;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{ApiError, AppState};

pub(crate) fn routes<M: Model + 'static>() -> Router<Arc<AppState<M>>> {
    Router::new().route("/v1/export", post(export::<M>)).route("/v1/import", post(import::<M>))
}

fn ise(e: impl std::fmt::Display) -> ApiError {
    ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}
fn bad(e: impl Into<String>) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, e.into())
}

#[derive(Deserialize)]
struct Export {
    dir: String,
}

async fn export<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Json(b): Json<Export>) -> Result<Json<Value>, ApiError> {
    let dir = FsPath::new(&b.dir);
    if dir.exists() && std::fs::read_dir(dir).map(|mut d| d.next().is_some()).unwrap_or(true) {
        return Err(bad(format!("{} already exists and is not empty; choose a new folder", dir.display())));
    }
    std::fs::create_dir_all(dir).map_err(ise)?;
    let mut counts = serde_json::Map::new();
    if let Some(m) = &st.settings.memory {
        let _ = m.memory.scan();
        m.memory.export(&dir.join("memory")).map_err(ise)?;
        counts.insert("memory_pages".into(), json!(m.memory.list(None, lh_memory::State::Live).map(|p| p.len()).unwrap_or(0)));
    }
    if let Some(l) = &st.settings.learn {
        let rules = l.learn.list_rules(true).map_err(ise)?;
        let j: Vec<Value> = rules.iter().map(|r| json!({"topic": r.topic, "text": r.text, "project": r.project, "role": r.role, "privacy": r.privacy.as_str(), "active": r.status == Status::Active, "evidence": r.evidence, "source": r.source})).collect();
        std::fs::write(dir.join("rules.json"), serde_json::to_string_pretty(&j).unwrap_or_default()).map_err(ise)?;
        counts.insert("rules".into(), json!(j.len()));
        if let Some(s) = l.learn.style().map_err(ise)? {
            std::fs::write(dir.join("style.txt"), s.text).map_err(ise)?;
            counts.insert("style".into(), json!(true));
        }
    }
    if let Some(t) = &st.settings.team {
        std::fs::create_dir_all(dir.join("roles")).map_err(ise)?;
        std::fs::create_dir_all(dir.join("departments")).map_err(ise)?;
        let mut n = 0;
        for e in t.roles.list().map_err(ise)? {
            if let Ok(r) = e.role {
                std::fs::write(dir.join("roles").join(format!("{}.md", r.name)), r.render()).map_err(ise)?;
                n += 1;
            }
        }
        counts.insert("roles".into(), json!(n));
        n = 0;
        for (_, d) in t.depts.list().map_err(ise)? {
            if let Ok(d) = d {
                std::fs::write(dir.join("departments").join(format!("{}.md", d.name)), d.render()).map_err(ise)?;
                n += 1;
            }
        }
        counts.insert("departments".into(), json!(n));
    }
    std::fs::write(dir.join("manifest.json"), json!({"format": "lastharness-export", "version": 1, "counts": counts}).to_string()).map_err(ise)?;
    let _ = st.settings.store.log("export", &b.dir, "user", &json!({"counts": counts}));
    Ok(Json(json!({"dir": b.dir, "counts": counts, "warning": "this folder holds your private notes, rules and roles; keep it somewhere private"})))
}

#[derive(Deserialize)]
struct Import {
    dir: String,
    /// Without this nothing is changed; the reply only says what is in the folder.
    #[serde(default)]
    apply: bool,
    /// Roles and departments that may replace your own.
    #[serde(default)]
    overwrite: Vec<String>,
    /// Replace the style profile even if you already have one.
    #[serde(default)]
    replace_style: bool,
}

async fn import<M: Model + 'static>(State(st): State<Arc<AppState<M>>>, Json(b): Json<Import>) -> Result<Json<Value>, ApiError> {
    let dir = FsPath::new(&b.dir);
    let manifest: Value = std::fs::read_to_string(dir.join("manifest.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).ok_or_else(|| bad(format!("{} is not a LastHarness export (no manifest.json)", b.dir)))?;
    if manifest["format"] != "lastharness-export" || manifest["version"].as_u64().is_none_or(|v| v > 1) {
        return Err(bad("this export was made by a newer or different program and cannot be read"));
    }
    let rules: Vec<Value> = std::fs::read_to_string(dir.join("rules.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    let style = std::fs::read_to_string(dir.join("style.txt")).ok();
    let existing_rules: Vec<String> = st.settings.learn.as_ref().and_then(|l| l.learn.list_rules(true).ok()).unwrap_or_default().into_iter().map(|r| r.text).collect();
    let new_rules: Vec<&Value> = rules.iter().filter(|r| r["text"].as_str().is_some_and(|t| !existing_rules.iter().any(|e| e == t))).collect();
    let mut out = json!({"apply": b.apply, "rules": {"in_folder": rules.len(), "new": new_rules.len()}, "style": style.is_some()});
    if let Some(t) = &st.settings.team {
        let pv = lh_team::plugin::preview(dir, &t.roles, &t.depts).map_err(|e| bad(e.to_string()))?;
        out["roles"] = json!(pv.roles.iter().map(|i| json!({"name": i.name, "exists": i.exists})).collect::<Vec<_>>());
        out["departments"] = json!(pv.departments.iter().map(|i| json!({"name": i.name, "exists": i.exists})).collect::<Vec<_>>());
        out["problems"] = json!(pv.problems);
    }
    out["memory_files"] = json!(std::fs::read_dir(dir.join("memory")).map(|d| d.flatten().filter(|e| e.path().is_dir()).count()).unwrap_or(0) > 0);
    if !b.apply {
        return Ok(Json(out));
    }
    let mut done = serde_json::Map::new();
    if let (Some(m), true) = (&st.settings.memory, dir.join("memory").is_dir()) {
        let r = m.memory.import(&dir.join("memory")).map_err(ise)?;
        done.insert("memory_pages_added".into(), json!(r.imported));
        done.insert("memory_pages_already_here".into(), json!(r.already_present));
    }
    if let Some(l) = &st.settings.learn {
        let mut n = 0;
        for r in new_rules {
            let mut nr = NewRule::new(r["text"].as_str().unwrap_or_default());
            nr.topic = r["topic"].as_str().unwrap_or_default().to_string();
            nr.project = r["project"].as_str().map(str::to_string);
            nr.role = r["role"].as_str().map(str::to_string);
            nr.privacy = r["privacy"].as_str().and_then(Privacy::parse).unwrap_or(Privacy::Sensitive); // unsure counts as sensitive
            nr.evidence = r["evidence"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default();
            nr.source = "import".into();
            let rule = l.learn.add_rule(nr, "import").map_err(ise)?;
            if r["active"] == false {
                l.learn.set_rule_status(&rule.id, Status::Disabled, "import").map_err(ise)?;
            }
            n += 1;
        }
        done.insert("rules_added".into(), json!(n));
        if let Some(s) = style {
            if b.replace_style || l.learn.style().map_err(ise)?.is_none() {
                l.learn.set_style(&s, "import").map_err(ise)?;
                done.insert("style_set".into(), json!(true));
            } else {
                done.insert("style_set".into(), json!(false));
            }
        }
    }
    if let Some(t) = &st.settings.team {
        let r = lh_team::plugin::install(dir, &t.roles, &t.depts, &std::env::temp_dir().join("lh-unused-themes"), &b.overwrite, "import").map_err(|e| bad(e.to_string()))?;
        done.insert("installed".into(), json!(r.installed));
        done.insert("skipped".into(), json!(r.skipped));
    }
    let _ = st.settings.store.log("import", &b.dir, "user", &json!({"done": done}));
    out["done"] = Value::Object(done);
    Ok(Json(out))
}
