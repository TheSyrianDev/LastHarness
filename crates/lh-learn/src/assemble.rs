//! Putting learned things in front of the model. The user's own system text goes first and is never
//! changed; the learned blocks follow it, each under its own heading. Blocks are built once when a
//! session starts, so the start of every prompt stays identical for the server's cache.

use lh_memory::{detect, Privacy, Trust};

use crate::Rule;

#[derive(Clone, Debug, Default)]
pub struct Learned {
    pub rules: Vec<Rule>,
    pub style: Option<String>,
    /// Name and description of each available skill (see the skills module).
    pub skills: Vec<(String, String)>,
}

pub fn render_learned(l: &Learned, trust: Trust) -> String {
    let mut out: Vec<String> = vec![];
    if !l.rules.is_empty() {
        let mut s = String::from("Rules the user has set for how you work. Follow them unless the user says otherwise in this conversation:\n");
        for r in &l.rules {
            s.push_str(&format!("- {}\n", r.text));
        }
        out.push(s.trim_end().to_string());
    }
    if let Some(style) = l.style.as_ref().filter(|s| !s.trim().is_empty()) {
        // the style profile has no privacy flag, so it is checked each time it is used
        if detect(style).privacy == Privacy::Normal || trust == Trust::Local {
            out.push(format!("How to talk to this user:\n{}", style.trim()));
        }
    }
    if !l.skills.is_empty() {
        let mut s = String::from("Skills you can use. Each is a procedure the user approved. Call use_skill with its name to read it before you start such a task:\n");
        for (n, d) in &l.skills {
            s.push_str(&format!("- {n}: {}\n", d.split_whitespace().collect::<Vec<_>>().join(" ")));
        }
        out.push(s.trim_end().to_string());
    }
    out.join("\n\n")
}

/// The system message: the base text exactly as given, then the other blocks. `None` if all are empty.
pub fn compose_system(base: Option<&str>, blocks: &[&str]) -> Option<String> {
    let mut parts: Vec<&str> = vec![];
    if let Some(b) = base.filter(|b| !b.is_empty()) {
        parts.push(b);
    }
    parts.extend(blocks.iter().copied().filter(|b| !b.trim().is_empty()));
    (!parts.is_empty()).then(|| parts.join("\n\n"))
}
