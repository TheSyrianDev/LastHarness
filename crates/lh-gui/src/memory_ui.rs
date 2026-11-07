//! The Memory section's new parts: the "New memory" and edit editor with the agent tidy, the "Memory summary"
//! profile, and the import and export of all notes. The list and its per-note buttons are in panels.rs.
//! Every change goes through the core's memory routes; nothing is written to disk here.

use gpui::prelude::*;
use gpui::*;
use gpui_component::input::{Input, InputState, Textarea, TextareaState};
use gpui_component::text::TextView;
use serde_json::{json, Value};

use crate::app::{App, MemoryPage};
use crate::ui::Kind;
use lh_gui::api::Cmd;

/// The note editor. The original text stays in the editor until the user accepts an agent tidy.
pub struct MemEdit {
    /// None: a new note. Some: the id of the note being edited.
    pub id: Option<String>,
    pub title: Entity<InputState>,
    pub text: Entity<TextareaState>,
    /// The tidy result waiting for the user to accept or discard: (title, text).
    pub tidied: Option<(String, String)>,
    pub tidying: bool,
    pub error: Option<String>,
}

impl App {
    fn mem_entities(&self, title: &str, text: &str, window: &mut Window, cx: &mut Context<Self>) -> (Entity<InputState>, Entity<TextareaState>) {
        let t = cx.new(|cx| InputState::new(window, cx).placeholder("A short title").default_value(title.to_string()));
        let x = cx.new(|cx| TextareaState::new(window, cx).auto_grow(4, 12).placeholder("Write the note").default_value(text.to_string()));
        (t, x)
    }

    pub fn mem_open_new(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (title, text) = self.mem_entities("", "", window, cx);
        self.mem_edit = Some(MemEdit { id: None, title, text, tidied: None, tidying: false, error: None });
        cx.notify();
    }

    pub fn mem_open_edit(&mut self, pg: &MemoryPage, window: &mut Window, cx: &mut Context<Self>) {
        let (title, text) = self.mem_entities(&pg.title, &pg.body, window, cx);
        self.mem_edit = Some(MemEdit { id: Some(pg.id.clone()), title, text, tidied: None, tidying: false, error: None });
        cx.notify();
    }

    pub fn mem_close(&mut self, cx: &mut Context<Self>) {
        self.mem_edit = None;
        cx.notify();
    }

    /// The model for memory writing: the current chat's model, else the first one listed.
    fn mem_model(&self) -> String {
        let name = self.chats.current().map(|c| c.model.clone()).filter(|m| !m.is_empty()).or_else(|| self.models.names().first().cloned()).unwrap_or_default();
        self.models.id_for(&name)
    }

    /// Sends the editor's text to the model for a clearer rewrite. The original stays in the editor.
    pub fn mem_tidy(&mut self, cx: &mut Context<Self>) {
        let model = self.mem_model();
        let Some(ed) = self.mem_edit.as_mut() else { return };
        if ed.tidying {
            return;
        }
        let text = ed.text.read(cx).value().to_string();
        if text.trim().is_empty() {
            ed.error = Some("Write something first, then tidy it.".into());
            cx.notify();
            return;
        }
        if model.is_empty() {
            ed.error = Some("No model is set up to tidy with.".into());
            cx.notify();
            return;
        }
        ed.tidying = true;
        ed.tidied = None;
        ed.error = None;
        self.backend.send(Cmd::Call { tag: "memory-tidy".into(), method: "POST".into(), path: "/v1/memory/tidy".into(), body: Some(json!({"text": text, "model": model})) });
        cx.notify();
    }

    /// Replaces the editor text with the tidy result. The user can still edit it before saving.
    pub fn mem_accept_tidy(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((t, x)) = self.mem_edit.as_ref().and_then(|e| e.tidied.clone()) else { return };
        let id = self.mem_edit.as_ref().and_then(|e| e.id.clone());
        let (title, text) = self.mem_entities(&t, &x, window, cx);
        self.mem_edit = Some(MemEdit { id, title, text, tidied: None, tidying: false, error: None });
        cx.notify();
    }

    pub fn mem_discard_tidy(&mut self, cx: &mut Context<Self>) {
        if let Some(ed) = self.mem_edit.as_mut() {
            ed.tidied = None;
        }
        cx.notify();
    }

    /// Saves the editor: a new note, or the edited one.
    pub fn mem_save(&mut self, cx: &mut Context<Self>) {
        let Some(ed) = self.mem_edit.as_mut() else { return };
        let title = ed.title.read(cx).value().to_string();
        let text = ed.text.read(cx).value().to_string();
        if text.trim().is_empty() {
            ed.error = Some("The note is empty. Write something or cancel.".into());
            cx.notify();
            return;
        }
        let (method, path, body) = match &ed.id {
            Some(id) => ("PATCH", format!("/v1/memory/pages/{id}"), json!({"title": title, "text": text})),
            None => ("POST", "/v1/memory/pages".to_string(), json!({"layer": "facts", "title": title, "text": text, "pinned": false})),
        };
        ed.error = None;
        self.backend.send(Cmd::Call { tag: "memory-save".into(), method: method.into(), path, body: Some(body) });
        cx.notify();
    }

    pub fn mem_delete(&mut self, id: &str, cx: &mut Context<Self>) {
        self.mem_delete_armed = None;
        self.backend.send(Cmd::Call { tag: "memory-delete".into(), method: "DELETE".into(), path: format!("/v1/memory/pages/{id}"), body: None });
        cx.notify();
    }

    fn mem_reload(&mut self) {
        self.backend.send(Cmd::Call { tag: "memory".into(), method: "GET".into(), path: "/v1/memory/pages".into(), body: None });
    }

    /// Opens the summary editor on the current summary text.
    pub fn summary_open_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.panels.summary.clone().unwrap_or_default();
        let x = cx.new(|cx| TextareaState::new(window, cx).auto_grow(4, 14).placeholder("Write a short profile").default_value(text));
        self.summary_edit = Some(x);
        cx.notify();
    }

    pub fn summary_generate(&mut self, cx: &mut Context<Self>) {
        let model = self.mem_model();
        if model.is_empty() {
            self.panels.mem_msg = Some("No model is set up to write the profile with.".into());
            cx.notify();
            return;
        }
        self.panels.summary_busy = true;
        self.backend.send(Cmd::Call { tag: "summary-generate".into(), method: "POST".into(), path: "/v1/memory/summary/generate".into(), body: Some(json!({"model": model})) });
        cx.notify();
    }

    pub fn summary_save(&mut self, cx: &mut Context<Self>) {
        let Some(x) = self.summary_edit.clone() else { return };
        let text = x.read(cx).value().to_string();
        self.backend.send(Cmd::Call { tag: "summary-save".into(), method: "PUT".into(), path: "/v1/memory/summary".into(), body: Some(json!({"text": text})) });
    }

    /// Export all notes to the typed path. The format follows the file name (.json, else Markdown).
    pub fn notes_export(&mut self, cx: &mut Context<Self>) {
        let path = self.mem_path.read(cx).value().trim().to_string();
        if path.is_empty() {
            self.panels.mem_msg = Some("Type a full file path first, for example /home/you/memory.md.".into());
            cx.notify();
            return;
        }
        self.backend.send(Cmd::Call { tag: "notes-export".into(), method: "POST".into(), path: "/v1/memory/notes/export".into(), body: Some(json!({"path": path})) });
    }

    /// Reads the import file and shows the counts. Nothing is added yet.
    pub fn notes_preview(&mut self, cx: &mut Context<Self>) {
        let path = self.mem_path.read(cx).value().trim().to_string();
        if path.is_empty() {
            self.panels.mem_msg = Some("Type the full path of a .md or .json file to import.".into());
            cx.notify();
            return;
        }
        self.panels.import_preview = None;
        self.backend.send(Cmd::Call { tag: "notes-preview".into(), method: "POST".into(), path: "/v1/memory/notes/import".into(), body: Some(json!({"path": path, "apply": false})) });
    }

    pub fn notes_apply(&mut self, cx: &mut Context<Self>) {
        let path = self.mem_path.read(cx).value().trim().to_string();
        self.backend.send(Cmd::Call { tag: "notes-import".into(), method: "POST".into(), path: "/v1/memory/notes/import".into(), body: Some(json!({"path": path, "apply": true})) });
    }

    /// Replies for the Memory section's own calls. Returns false for any other tag.
    pub fn mem_reply(&mut self, tag: &str, result: &Result<Value, String>) -> bool {
        match tag {
            "memory-tidy" => {
                let r = match result {
                    Ok(v) => match (v["title"].as_str(), v["text"].as_str()) {
                        (Some(t), Some(x)) => Ok((t.to_string(), x.to_string())),
                        _ => Err("The model did not return a note.".to_string()),
                    },
                    Err(e) => Err(e.clone()),
                };
                if let Some(ed) = self.mem_edit.as_mut() {
                    ed.tidying = false;
                    match r {
                        Ok(pair) => ed.tidied = Some(pair),
                        Err(e) => ed.error = Some(e),
                    }
                }
            }
            "memory-save" => match result {
                Ok(_) => {
                    self.mem_edit = None;
                    self.mem_reload();
                }
                Err(e) => {
                    if let Some(ed) = self.mem_edit.as_mut() {
                        ed.error = Some(e.clone());
                    }
                }
            },
            "memory-delete" => match result {
                Ok(_) => self.mem_reload(),
                Err(e) => self.panels.mem_msg = Some(e.clone()),
            },
            "summary" => {
                if let Ok(v) = result {
                    let t = v["text"].as_str().unwrap_or("").trim().to_string();
                    self.panels.summary = Some(t);
                }
            }
            "summary-generate" => {
                self.panels.summary_busy = false;
                match result {
                    Ok(v) => {
                        self.panels.summary = Some(v["text"].as_str().unwrap_or("").trim().to_string());
                        self.panels.mem_msg = None;
                    }
                    Err(e) => self.panels.mem_msg = Some(e.clone()),
                }
            }
            "summary-save" => match result {
                Ok(_) => {
                    self.summary_edit = None;
                    self.backend.send(Cmd::Call { tag: "summary".into(), method: "GET".into(), path: "/v1/memory/summary".into(), body: None });
                }
                Err(e) => self.panels.mem_msg = Some(e.clone()),
            },
            "notes-export" => {
                self.panels.mem_msg = Some(match result {
                    Ok(v) => format!("Exported {} notes as {}.", v["count"], v["format"].as_str().unwrap_or("file")),
                    Err(e) => e.clone(),
                });
            }
            "notes-preview" => match result {
                Ok(v) => self.panels.import_preview = Some(v.clone()),
                Err(e) => self.panels.mem_msg = Some(e.clone()),
            },
            "notes-import" => match result {
                Ok(v) => {
                    self.panels.import_preview = None;
                    self.panels.mem_msg = Some(format!("Added {} notes. They are your own notes now.", v["imported"].as_u64().unwrap_or(0)));
                    self.mem_reload();
                }
                Err(e) => self.panels.mem_msg = Some(e.clone()),
            },
            _ => return false,
        }
        true
    }

    /// The "New memory" button, the editor, the Memory summary block and the import and export block.
    pub fn memory_tools(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal.clone();
        let mut col = div().flex().flex_col().gap_2();
        let new_btn = self.button("mem-new", "New memory", Kind::Primary, cx, |a, window, cx| a.mem_open_new(window, cx));
        col = col.child(div().flex().child(new_btn));
        if self.panels.pages.is_empty() && self.panels.proposals.is_empty() {
            col = col.child(self.note("Nothing is remembered yet. What the assistant learns about you shows up here, and you can edit or remove it."));
        }

        if let Some(ed) = self.mem_edit.as_ref() {
            let heading = if ed.id.is_some() { "Edit memory" } else { "New memory" };
            let mut box_ = div().flex().flex_col().gap_2().p_2().rounded(p.inner).border_1().border_color(p.accent).text_color(p.text)
                .child(div().text_xs().text_color(p.accent).child(heading))
                .child(Input::new(&ed.title))
                .child(Textarea::new(&ed.text).aria_label("Memory text"));
            if let Some((t, x)) = ed.tidied.clone() {
                let accept = self.button("mem-tidy-accept", "Use this", Kind::Primary, cx, |a, window, cx| a.mem_accept_tidy(window, cx));
                let discard = self.button("mem-tidy-discard", "Keep original", Kind::Ghost, cx, |a, _, cx| a.mem_discard_tidy(cx));
                box_ = box_
                    .child(div().text_xs().text_color(p.muted).child("Agent tidy. Not saved yet. Your original is still in the box above."))
                    .child(div().p_2().rounded(p.inner).bg(p.sunken).child(div().child(t)).child(div().text_sm().text_color(p.muted).child(x)))
                    .child(div().flex().flex_wrap().gap_1().child(accept).child(discard));
            }
            if let Some(e) = ed.error.clone() {
                box_ = box_.child(self.note(e));
            }
            let tidy_label = if ed.tidying { "Tidying..." } else { "Agent tidy" };
            let tidy = self.button("mem-tidy", tidy_label, Kind::Normal, cx, |a, _, cx| a.mem_tidy(cx));
            let save = self.button("mem-save", "Save", Kind::Primary, cx, |a, _, cx| a.mem_save(cx));
            let cancel = self.button("mem-cancel", "Cancel", Kind::Ghost, cx, |a, _, cx| a.mem_close(cx));
            box_ = box_.child(div().flex().flex_wrap().gap_1().child(tidy).child(save).child(cancel));
            col = col.child(box_);
        }

        // Memory summary: read-only until the user edits it
        let mut summary = div().flex().flex_col().gap_2().p_2().rounded(p.inner).bg(p.sunken).text_color(p.text).child(div().text_xs().text_color(p.muted).child("Memory summary"));
        if let Some(x) = self.summary_edit.clone() {
            let save = self.button("summary-save", "Save profile", Kind::Primary, cx, |a, _, cx| a.summary_save(cx));
            let cancel = self.button("summary-cancel", "Cancel", Kind::Ghost, cx, |a, _, cx| {
                a.summary_edit = None;
                cx.notify();
            });
            summary = summary.child(Textarea::new(&x).aria_label("Memory summary")).child(div().flex().gap_1().child(save).child(cancel));
        } else {
            let has = self.panels.summary.as_deref().is_some_and(|t| !t.is_empty());
            let busy = self.panels.summary_busy;
            if has {
                let text = self.panels.summary.clone().unwrap_or_default();
                summary = summary.child(TextView::markdown(SharedString::from("summary-text"), text));
            } else {
                summary = summary.child(self.note("No summary yet. It is written from all your notes, department notes included."));
            }
            let make_label = if busy { "Writing..." } else if has { "Regenerate" } else { "Write summary" };
            let make = self.button("summary-make", make_label, Kind::Normal, cx, |a, _, cx| a.summary_generate(cx));
            let mut row = div().flex().flex_wrap().gap_1().child(make);
            if has {
                row = row.child(self.button("summary-edit", "Edit", Kind::Ghost, cx, |a, window, cx| a.summary_open_edit(window, cx)));
            }
            summary = summary.child(row);
        }
        col = col.child(summary);

        // import and export of all notes, one file
        let export = self.button("notes-export-btn", "Export all notes", Kind::Normal, cx, |a, _, cx| a.notes_export(cx));
        let preview = self.button("notes-preview-btn", "Check import", Kind::Normal, cx, |a, _, cx| a.notes_preview(cx));
        let mut io = div().flex().flex_col().gap_2().p_2().rounded(p.inner).bg(p.sunken).text_color(p.text)
            .child(div().text_xs().text_color(p.muted).child("Export and import: type the full path of a .md or .json file. Imported text is data, not instructions. Secrets are removed."))
            .child(Input::new(&self.mem_path))
            .child(div().flex().flex_wrap().gap_1().child(export).child(preview));
        if let Some(v) = self.panels.import_preview.clone() {
            let n = v["count"].as_u64().unwrap_or(0);
            io = io.child(div().text_sm().child(format!("{n} notes will be added. {} already here are skipped. {} had a secret removed.", v["duplicates"].as_u64().unwrap_or(0), v["redacted"].as_u64().unwrap_or(0))));
            if n > 0 {
                let apply = self.button("notes-apply", format!("Import {n} notes"), Kind::Primary, cx, |a, _, cx| a.notes_apply(cx));
                io = io.child(apply);
            }
        }
        if let Some(m) = self.panels.mem_msg.clone() {
            io = io.child(self.note(m));
        }
        col = col.child(io);
        col.into_any_element()
    }
}
