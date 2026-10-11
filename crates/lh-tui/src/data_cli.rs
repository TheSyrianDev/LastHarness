//! `lh export <folder>` and `lh import <folder>`: move memory, rules, the style profile, roles and
//! departments between machines or keep a copy. Import shows what is in the folder and changes nothing
//! until you add `--apply`.

use serde_json::{json, Value};

use crate::client::{Client, Result};

pub const HELP: &str = "lh export <folder>                       write memory, rules, style, roles and departments to a new folder
lh import <folder>                       show what the folder holds (changes nothing)
lh import <folder> --apply [--replace <name>]... [--replace-style]
                                         bring it in; your own roles, departments and style stay unless named";

fn absolute(p: &str) -> String {
    std::path::absolute(p).map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|_| p.to_string())
}

pub async fn export(c: &Client, args: &[String]) -> Result<String> {
    let dir = args.first().ok_or("which folder? Run `lh export <folder>`.")?;
    let v = c.call("POST", "/v1/export", Some(json!({"dir": absolute(dir)}))).await?;
    let counts = v["counts"].as_object().map(|m| m.iter().map(|(k, v)| format!("{k}: {v}")).collect::<Vec<_>>().join(", ")).unwrap_or_default();
    Ok(format!("Exported to {}.\n{counts}\n{}", v["dir"].as_str().unwrap_or(""), v["warning"].as_str().unwrap_or("")))
}

fn names(v: &Value) -> String {
    v.as_array().map(|a| a.iter().map(|i| format!("{}{}", i["name"].as_str().unwrap_or(""), if i["exists"] == true { " (you have one)" } else { "" })).collect::<Vec<_>>().join(", ")).filter(|s| !s.is_empty()).unwrap_or_else(|| "none".into())
}

pub async fn import(c: &Client, args: &[String]) -> Result<String> {
    let dir = args.first().ok_or("which folder? Run `lh import <folder>`.")?;
    let apply = args.iter().any(|a| a == "--apply");
    let replace: Vec<&String> = args.iter().enumerate().filter(|(i, a)| *i > 0 && args[*i - 1] == "--replace" && !a.starts_with("--")).map(|(_, a)| a).collect();
    let v = c.call("POST", "/v1/import", Some(json!({"dir": absolute(dir), "apply": apply, "overwrite": replace, "replace_style": args.iter().any(|a| a == "--replace-style")}))).await?;
    let mut out = format!(
        "In the folder: {} rule(s), {} of them new to you; style profile: {}; memory: {}\nroles: {}\ndepartments: {}",
        v["rules"]["in_folder"], v["rules"]["new"], if v["style"] == true { "yes" } else { "no" }, if v["memory_files"] == true { "yes" } else { "no" }, names(&v["roles"]), names(&v["departments"])
    );
    for p in v["problems"].as_array().into_iter().flatten().filter_map(Value::as_str) {
        out.push_str(&format!("\ncannot use {p}"));
    }
    if let Some(d) = v["done"].as_object() {
        out.push_str("\n\nDone:");
        for (k, val) in d {
            out.push_str(&format!("\n  {k}: {}", if let Some(a) = val.as_array() { a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join("; ") } else { val.to_string() }));
        }
    } else {
        out.push_str("\n\nNothing was changed. Add --apply to bring it in.");
    }
    Ok(out)
}
