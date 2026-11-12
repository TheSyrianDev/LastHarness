//! Search by fit, and the installed list, on the Models and keys page. The search asks the core, which groups
//! the Hub's GGUF repos into four plain groups. Download and Run are not here yet.

use gpui::prelude::*;
use gpui::*;
use gpui_component::input::{Input, InputState};
use serde_json::Value;

use crate::app::App;
use crate::ui::Kind;

/// One repo row in a fit group.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchRow {
    pub repo: String,
    pub quant: String,
    pub size: String,
    pub downloads: u64,
}

/// One installed model row.
#[derive(Debug, Clone, PartialEq)]
pub struct InstalledRow {
    pub id: String,
    pub file: String,
    pub size: String,
    pub kind: String,
}

/// Group headings, in the order the core ranks them (fits first).
const GROUPS: [(&str, &str); 4] = [
    ("fits", "Fits in memory"),
    ("tight", "Tight: needs system RAM as well"),
    ("too_big", "Too big for this computer"),
    ("unknown", "Size not known"),
];

/// Bytes as gigabytes text; empty when the size is 0 (not known).
pub fn gb(bytes: u64) -> String {
    if bytes == 0 {
        String::new()
    } else {
        format!("{:.1} GB", bytes as f64 / 1e9)
    }
}

/// Percent-encode a search text for a query string.
pub fn encode(q: &str) -> String {
    q.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// The four fit groups of a search answer, each with its heading. Missing groups are empty.
pub fn groups(v: &Value) -> Vec<(&'static str, Vec<SearchRow>)> {
    GROUPS
        .iter()
        .map(|(key, label)| {
            let rows = v[*key]
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|h| SearchRow {
                            repo: h["repo"].as_str().unwrap_or("").to_string(),
                            quant: h["best_quant"].as_str().unwrap_or("no GGUF quant found").to_string(),
                            size: gb(h["best_bytes"].as_u64().unwrap_or(0)),
                            downloads: h["downloads"].as_u64().unwrap_or(0),
                        })
                        .collect()
                })
                .unwrap_or_default();
            (*label, rows)
        })
        .collect()
}

/// The installed rows of the registry answer.
pub fn installed(v: &Value) -> Vec<InstalledRow> {
    v["models"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|m| InstalledRow {
                    id: m["id"].as_str().unwrap_or("").to_string(),
                    file: m["file"].as_str().unwrap_or("").to_string(),
                    size: gb(m["size_bytes"].as_u64().unwrap_or(0)),
                    kind: m["kind"].as_str().unwrap_or("text").to_string(),
                })
                .collect()
        })
        .unwrap_or_default()
}

impl App {
    /// The search box, the results by fit, and the installed list.
    pub fn pg_models_search(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let input = match self.pg.inputs.get("models-search").cloned() {
            Some(s) => s,
            None => {
                let s = cx.new(|cx| InputState::new(window, cx));
                self.pg.inputs.insert("models-search".into(), s.clone());
                // Debug only, for screenshots: LASTHARNESS_MODELS_QUERY pre-fills the box and runs the search once.
                if let Ok(q) = std::env::var("LASTHARNESS_MODELS_QUERY") {
                    let q = q.trim().to_string();
                    if !q.is_empty() {
                        s.update(cx, |i, cx| i.set_value(q.clone(), window, cx));
                        self.pg_fetch("models-search", format!("/v1/models/search?q={}", encode(&q)));
                    }
                }
                s
            }
        };
        let go = self.button("models-search-go", "Search", Kind::Primary, cx, |a, _, cx| {
            let q = a.pg.inputs.get("models-search").map(|s| s.read(cx).value().to_string()).unwrap_or_default();
            a.pg_fetch("models-search", format!("/v1/models/search?q={}", encode(q.trim())));
            cx.notify();
        });
        let mut col = div().flex().flex_col().gap_2().pt_4();
        col = col.child(self.pg_title("Search models"));
        col = col.child(self.pg_muted("Search the Hub for GGUF models. Results are grouped by how well they fit this computer's memory."));
        col = col.child(div().flex().items_center().gap_2().child(div().flex_1().child(Input::new(&input).aria_label("Search models"))).child(go));

        match self.pg_list("models-search") {
            None => col = col.child(self.pg_note_line("Type a name and press Search.")),
            Some(Err(e)) => col = col.child(self.pg_error_line("The search", &e)),
            Some(Ok(v)) => {
                let msg = v["message"].as_str().unwrap_or("");
                if !msg.is_empty() {
                    col = col.child(self.pg_note_line(msg.to_string()));
                }
                for (label, rows) in groups(&v) {
                    if rows.is_empty() {
                        continue;
                    }
                    col = col.child(div().pt_2().text_sm().text_color(self.pal.text).child(format!("{label} ({})", rows.len())));
                    for r in rows {
                        let meta = format!("{} · {} · {} downloads", r.quant, if r.size.is_empty() { "size unknown".into() } else { r.size.clone() }, r.downloads);
                        col = col.child(div().flex().flex_col().pl_2().child(div().text_sm().text_color(self.pal.text).child(r.repo)).child(self.pg_muted(meta)));
                    }
                }
            }
        }

        col = col.child(div().pt_4().text_sm().text_color(self.pal.text).child("Installed on this computer"));
        match self.pg_list("models-installed") {
            None => col = col.child(self.pg_note_line("Loading…")),
            Some(Err(e)) => col = col.child(self.pg_error_line("The installed list", &e)),
            Some(Ok(v)) => {
                let msg = v["message"].as_str().unwrap_or("");
                if !msg.is_empty() {
                    col = col.child(self.pg_note_line(msg.to_string()));
                }
                for m in installed(&v) {
                    let meta = format!("{} · {} · {}", m.kind, if m.size.is_empty() { "size unknown".into() } else { m.size.clone() }, m.file);
                    col = col.child(div().flex().flex_col().pl_2().child(div().text_sm().text_color(self.pal.text).child(m.id)).child(self.pg_muted(meta)));
                }
            }
        }
        col.into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{encode, groups, installed, Value};

    #[test]
    fn groups_keep_order_and_labels() {
        let v: Value = serde_json::from_str(
            r#"{"fits": [{"repo": "a/small", "best_quant": "Q4_K_M", "best_bytes": 2000000000, "downloads": 10}],
                "tight": [], "too_big": [{"repo": "b/big", "best_quant": null, "best_bytes": 0, "downloads": 1}]}"#,
        )
        .unwrap();
        let g = groups(&v);
        assert_eq!(g.len(), 4);
        assert_eq!(g[0].0, "Fits in memory");
        assert_eq!(g[0].1[0].repo, "a/small");
        assert_eq!(g[0].1[0].size, "2.0 GB");
        assert_eq!(g[2].0, "Too big for this computer");
        assert_eq!(g[2].1[0].quant, "no GGUF quant found");
        assert!(g[3].1.is_empty());
    }

    #[test]
    fn encode_escapes_spaces_and_symbols() {
        assert_eq!(encode("qwen 3/8b"), "qwen%203%2F8b");
        assert_eq!(encode("a-b_c.d~"), "a-b_c.d~");
    }

    #[test]
    fn installed_reads_rows_and_missing_list_is_empty() {
        let v: Value = serde_json::from_str(r#"{"models": [{"id": "lfm", "file": "lfm.gguf", "size_bytes": 1000000000, "kind": "text"}]}"#).unwrap();
        let rows = installed(&v);
        assert_eq!(rows[0].id, "lfm");
        assert_eq!(rows[0].size, "1.0 GB");
        assert!(installed(&serde_json::from_str::<Value>(r#"{"message": "none"}"#).unwrap()).is_empty());
    }
}
