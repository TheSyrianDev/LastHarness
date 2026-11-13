//! Notes the user makes by hand: the agent's tidy-up of a draft, the Memory summary built from all
//! notes, and import and export of all notes as one Markdown or JSON file. Imported text is
//! data. It is screened for secrets and stored as notes; nothing in it is run.

use lh_memory::{detect, excerpt, redact, Layer, Page, Privacy};
use serde_json::{json, Value};

/// The prompt that tidies a draft note. The model must keep every fact and answer in one JSON object.
pub const TIDY_PROMPT: &str = "Rewrite the note below so it reads clearly and correctly. Fix grammar and wording. Keep every fact exactly as it is. Do not add facts, do not drop facts. Reply with only one JSON object: {\"title\": \"a short title\", \"text\": \"the rewritten note\"}.\n\nNote:\n";

pub fn tidy_request(draft: &str) -> String {
    format!("{TIDY_PROMPT}{draft}")
}

/// Reads the model's reply. Returns (title, text); the title may be empty. Nothing else in the reply is used.
pub fn parse_tidy(reply: &str) -> Option<(String, String)> {
    let start = reply.find('{')?;
    let end = reply.rfind('}')?;
    if end < start {
        return None;
    }
    let v: Value = serde_json::from_str(&reply[start..=end]).ok()?;
    let text = v["text"].as_str()?.trim().to_string();
    if text.is_empty() {
        return None;
    }
    let title = v["title"].as_str().unwrap_or("").trim().to_string();
    Some((title, text))
}

/// A tidied note, screened for secrets and private paths. `redacted` says something was replaced.
pub struct Screened {
    pub title: String,
    pub text: String,
    pub redacted: bool,
}

/// Screens a title and body. An empty title becomes the first words of the body.
pub fn screen(title: &str, body: &str) -> Screened {
    let t = if title.trim().is_empty() { excerpt(body, 50) } else { title.trim().to_string() };
    let (t, b) = (redact(&t), redact(body));
    Screened { title: t.text, text: b.text, redacted: t.redacted || b.redacted }
}

/// The notes the Memory summary is built from: all live notes, department notes included. A sensitive
/// note is left out unless the model is on this machine.
pub fn summary_sources(pages: &[Page], local: bool) -> Vec<&Page> {
    pages.iter().filter(|p| local || p.privacy == Privacy::Normal).collect()
}

pub fn summary_prompt(pages: &[&Page]) -> String {
    let mut s = String::from("Write a short profile of the person these notes are about. Use three to six plain sentences in the third person. Use only facts that the notes state. Do not guess. Reply with the profile text only.\n\nNotes:\n");
    for p in pages {
        s.push_str(&format!("- {}: {}\n", p.title, p.body.trim()));
    }
    s
}

/// One note as read from an import file or built from an export.
#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub layer: Layer,
    pub title: String,
    pub body: String,
    pub id: Option<String>,
    /// True when the file marked the note sensitive.
    pub sensitive: bool,
}

pub fn export_json(pages: &[Page]) -> String {
    let notes: Vec<Value> = pages
        .iter()
        .map(|p| json!({"id": p.id, "layer": p.layer.as_str(), "title": p.title, "body": p.body, "gist": p.gist, "privacy": p.privacy.as_str(), "pinned": p.pinned, "project": p.project, "department": p.department, "created": p.created, "updated": p.updated}))
        .collect();
    serde_json::to_string_pretty(&json!({"format": "lastai-memory", "version": 1, "notes": notes})).unwrap_or_default() + "\n"
}

/// Markdown: a title line, then one `## Title` per note, each followed by a `layer:` line and the body.
pub fn export_md(pages: &[Page]) -> String {
    let mut s = String::from("# Last AI memory\n\nEach note starts with a `## ` heading and a `layer:` line.\n");
    for p in pages {
        s.push_str(&format!("\n## {}\n\nlayer: {}\n\n{}\n", p.title.trim(), p.layer.as_str(), p.body.trim()));
    }
    s
}

pub fn parse_json(raw: &str) -> Result<Vec<Note>, String> {
    let v: Value = serde_json::from_str(raw).map_err(|e| format!("not valid JSON: {e}"))?;
    let list = match &v {
        Value::Array(a) => a.clone(),
        Value::Object(_) => v["notes"].as_array().cloned().ok_or("the file has no \"notes\" list")?,
        _ => return Err("the file has no notes".into()),
    };
    Ok(list
        .iter()
        .filter_map(|n| {
            let title = n["title"].as_str()?.to_string();
            let body = n["body"].as_str()?.to_string();
            Some(Note {
                layer: n["layer"].as_str().and_then(Layer::parse).unwrap_or(Layer::Facts),
                title,
                body,
                id: n["id"].as_str().map(str::to_string),
                sensitive: n["privacy"].as_str() == Some("sensitive"),
            })
        })
        .collect())
}

pub fn parse_md(raw: &str) -> Vec<Note> {
    let lines: Vec<&str> = raw.lines().collect();
    let mut out = vec![];
    let mut i = 0;
    while i < lines.len() {
        let Some(title) = lines[i].strip_prefix("## ") else {
            i += 1;
            continue;
        };
        // a note needs its `layer:` line right after the heading (blank lines allowed); otherwise it is body text
        let mut j = i + 1;
        while j < lines.len() && lines[j].trim().is_empty() {
            j += 1;
        }
        let Some(layer_line) = lines.get(j).and_then(|l| l.strip_prefix("layer: ")) else {
            i += 1;
            continue;
        };
        let layer = Layer::parse(layer_line.trim()).unwrap_or(Layer::Facts);
        let mut k = j + 1;
        let mut body_lines = vec![];
        while k < lines.len() {
            if lines[k].starts_with("## ") && is_note_start(&lines, k) {
                break;
            }
            body_lines.push(lines[k]);
            k += 1;
        }
        out.push(Note { layer, title: title.trim().to_string(), body: body_lines.join("\n").trim().to_string(), id: None, sensitive: false });
        i = k;
    }
    out
}

fn is_note_start(lines: &[&str], at: usize) -> bool {
    let mut j = at + 1;
    while j < lines.len() && lines[j].trim().is_empty() {
        j += 1;
    }
    lines.get(j).is_some_and(|l| l.starts_with("layer: "))
}

/// What an import would do: the notes that are new, screened, and the counts shown before it is applied.
#[derive(Debug, Default)]
pub struct Plan {
    pub notes: Vec<Note>,
    pub duplicates: usize,
    pub redacted: usize,
}

/// Screens every note for secrets and private paths, and skips notes that the library already has (same
/// layer, title and body). Imported notes always become the user's own notes.
pub fn plan(notes: Vec<Note>, existing: &[Page]) -> Plan {
    let mut out = Plan::default();
    for n in notes {
        let s = screen(&n.title, &n.body);
        let dup = existing.iter().any(|p| p.layer == n.layer && p.title == s.title && p.body == s.text) || out.notes.iter().any(|q| q.layer == n.layer && q.title == s.title && q.body == s.text);
        if dup {
            out.duplicates += 1;
            continue;
        }
        if s.redacted {
            out.redacted += 1;
        }
        out.notes.push(Note { layer: n.layer, title: s.title, body: s.text, id: n.id, sensitive: n.sensitive });
    }
    out
}

/// The privacy class for an imported note: a note the file marked sensitive stays sensitive.
pub fn privacy_of(n: &Note) -> (Privacy, String) {
    if n.sensitive {
        return (Privacy::Sensitive, "marked sensitive in the import file".into());
    }
    let c = detect(&format!("{} {}", n.title, n.body));
    (c.privacy, c.reason)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(title: &str, body: &str, dept: Option<&str>, privacy: Privacy) -> Page {
        Page {
            id: format!("id-{title}"),
            layer: Layer::Facts,
            slug: title.into(),
            title: title.into(),
            body: body.into(),
            gist: String::new(),
            privacy,
            privacy_reason: String::new(),
            pinned: false,
            project: None,
            department: dept.map(str::to_string),
            shared: false,
            source: "user".into(),
            version: 1,
            archived_at: None,
            created: 1,
            updated: 1,
        }
    }

    #[test]
    fn tidy_reply_is_read_from_json_inside_prose() {
        let (t, x) = parse_tidy("Here you go: {\"title\": \"Cat\", \"text\": \"The cat sleeps.\"} done").unwrap();
        assert_eq!((t.as_str(), x.as_str()), ("Cat", "The cat sleeps."));
        assert!(parse_tidy("{\"title\": \"x\", \"text\": \"  \"}").is_none());
        assert!(parse_tidy("no json here").is_none());
    }

    #[test]
    fn tidied_note_is_screened_before_it_is_saved() {
        let s = screen("", "my key is sk-abcdefghijklmnop");
        assert!(s.redacted);
        assert!(!s.text.contains("sk-abcdef"));
        assert!(!s.title.is_empty());
    }

    #[test]
    fn summary_is_built_from_all_notes_including_departments() {
        let pages = vec![page("Mine", "I like tea", None, Privacy::Normal), page("Dept", "Team rule", Some("sales"), Privacy::Normal), page("Secret", "a fact", None, Privacy::Sensitive)];
        let local = summary_sources(&pages, true);
        assert_eq!(local.iter().map(|p| p.title.as_str()).collect::<Vec<_>>(), vec!["Mine", "Dept", "Secret"]);
        let cloud = summary_sources(&pages, false);
        assert_eq!(cloud.iter().map(|p| p.title.as_str()).collect::<Vec<_>>(), vec!["Mine", "Dept"]);
        assert!(summary_prompt(&local).contains("Team rule"), "department notes are in the summary");
    }

    #[test]
    fn json_round_trip_keeps_title_body_and_layer() {
        let pages = vec![page("Tea", "I drink green tea.", None, Privacy::Normal)];
        let notes = parse_json(&export_json(&pages)).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!((notes[0].title.as_str(), notes[0].body.as_str()), ("Tea", "I drink green tea."));
        assert_eq!(notes[0].id.as_deref(), Some("id-Tea"));
    }

    #[test]
    fn markdown_round_trip_keeps_title_body_and_layer() {
        let mut p = page("Editor", "Uses Helix.\n\nAlso likes short replies.", None, Privacy::Normal);
        p.layer = Layer::Preferences;
        let notes = parse_md(&export_md(&[p]));
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].layer, Layer::Preferences);
        assert_eq!(notes[0].title, "Editor");
        assert_eq!(notes[0].body, "Uses Helix.\n\nAlso likes short replies.");
    }

    #[test]
    fn plan_skips_duplicates_and_counts_redactions() {
        let existing = vec![page("Tea", "I drink green tea.", None, Privacy::Normal)];
        let notes = vec![
            Note { layer: Layer::Facts, title: "Tea".into(), body: "I drink green tea.".into(), id: None, sensitive: false },
            Note { layer: Layer::Facts, title: "Key".into(), body: "token sk-abcdefghijklmnop".into(), id: None, sensitive: false },
        ];
        let p = plan(notes, &existing);
        assert_eq!(p.duplicates, 1);
        assert_eq!(p.redacted, 1);
        assert_eq!(p.notes.len(), 1);
        assert!(!p.notes[0].body.contains("sk-abcdef"));
    }
}
