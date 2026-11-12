//! Create on the Models and keys page: a prompt, Image or Video, Generate, and the jobs with their state.
//! The core runs the jobs (ComfyUI, under the machine lock); this file only asks and shows.

use gpui::prelude::*;
use gpui::*;
use gpui_component::input::{Input, InputState};
use serde_json::{json, Value};

use crate::app::App;
use crate::ui::Kind;
use lh_gui::api::Cmd;

/// Request body and path for one job. `video` picks the Wan MP4 route, else Z-Image.
pub fn job_request(prompt: &str, video: bool) -> (&'static str, Value) {
    if video {
        ("/v1/create/video", json!({ "prompt": prompt.trim() }))
    } else {
        ("/v1/create/image", json!({ "prompt": prompt.trim(), "width": 768, "height": 768, "steps": 8 }))
    }
}

/// Human line for a job state from the core.
pub fn state_label(state: &str) -> &'static str {
    match state {
        "running" => "Generating… (this can take a few minutes on this computer)",
        "done" => "Done",
        "failed" => "Failed",
        _ => "Unknown",
    }
}

/// The still image for each job row, in the same order as `job_rows` (empty when a job has none).
pub fn job_posters(v: &Value) -> Vec<String> {
    v["jobs"].as_array().map(|a| a.iter().rev().map(|j| j["poster"].as_str().unwrap_or("").to_string()).collect()).unwrap_or_default()
}

/// Rows for the jobs list: (prompt, state label, detail). Detail is the first file or the error text.
pub fn job_rows(v: &Value) -> Vec<(String, &'static str, String)> {
    v["jobs"]
        .as_array()
        .map(|a| {
            a.iter()
                .rev()
                .map(|j| {
                    let state = j["state"].as_str().unwrap_or("");
                    let detail = match state {
                        "done" => j["files"].as_array().and_then(|f| f.first()).and_then(|f| f.as_str()).unwrap_or("").to_string(),
                        _ => j["message"].as_str().unwrap_or("").to_string(),
                    };
                    (j["prompt"].as_str().unwrap_or("").to_string(), state_label(state), detail)
                })
                .collect()
        })
        .unwrap_or_default()
}

impl App {
    /// The Create block: prompt box, Image or Video choice, Generate, and the jobs.
    pub fn pg_create(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let input = match self.pg.inputs.get("create-prompt").cloned() {
            Some(s) => s,
            None => {
                let s = cx.new(|cx| InputState::new(window, cx));
                self.pg.inputs.insert("create-prompt".into(), s.clone());
                s
            }
        };
        let video = self.pg.open.contains("create-video");
        let pal = self.pal.clone();
        let choose_image = self.button("create-image", "Image", if video { Kind::Ghost } else { Kind::Primary }, cx, |a, _, cx| {
            a.pg.open.remove("create-video");
            cx.notify();
        });
        let choose_video = self.button("create-video-btn", "Video", if video { Kind::Primary } else { Kind::Ghost }, cx, |a, _, cx| {
            a.pg.open.insert("create-video".into());
            cx.notify();
        });
        let generate = self.button("create-generate", "Generate", Kind::Primary, cx, |a, _, cx| {
            let prompt = a.pg.inputs.get("create-prompt").map(|s| s.read(cx).value().to_string()).unwrap_or_default();
            if prompt.trim().is_empty() {
                a.pg.notes.insert("create".into(), (true, "Write a prompt first.".into()));
                return cx.notify();
            }
            let (path, body) = job_request(&prompt, a.pg.open.contains("create-video"));
            a.backend.send(Cmd::Call { tag: "pg-act:create:start".into(), method: "POST".into(), path: path.into(), body: Some(body) });
            a.pg_fetch("create-jobs", "/v1/create/jobs");
            cx.notify();
        });
        let mut col = div().flex().flex_col().gap_2().pt_4()
            .child(div().text_sm().text_color(pal.text).child("Create"))
            .child(Input::new(&input))
            .child(div().flex().gap_2().items_center().child(choose_image).child(choose_video).child(div().flex_1()).child(generate));
        if let Some((bad, msg)) = self.pg.notes.get("create").cloned() {
            col = col.child(div().text_sm().text_color(if bad { pal.danger } else { pal.muted }).child(msg));
        }
        match self.pg_list("create-jobs") {
            None => col = col.child(self.pg_note_line("No jobs yet.")),
            Some(Err(e)) => col = col.child(self.pg_error_line("The jobs", &e)),
            Some(Ok(v)) => {
                let posters = job_posters(&v);
                for (i, (prompt, state, detail)) in job_rows(&v).into_iter().enumerate() {
                    let mut row = div().flex().flex_col().pl_2().child(div().text_sm().text_color(pal.text).child(prompt)).child(self.pg_muted(state));
                    if !detail.is_empty() {
                        row = row.child(self.pg_muted(detail));
                    }
                    if let Some(p) = posters.get(i).filter(|p| !p.is_empty()) {
                        // the result shown inline: the image, or the first frame of the video
                        row = row.child(img(std::path::PathBuf::from(p)).w(px(320.)).h(px(180.)).object_fit(ObjectFit::Contain));
                    }
                    col = col.child(row);
                }
            }
        }
        col.into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{job_posters, job_request, job_rows, state_label};
    use serde_json::{json, Value};

    #[test]
    fn image_and_video_requests_use_their_routes() {
        let (p, b) = job_request("  a fox  ", false);
        assert_eq!(p, "/v1/create/image");
        assert_eq!(b["prompt"], "a fox");
        assert_eq!(b["width"], 768);
        let (p, _) = job_request("a fox", true);
        assert_eq!(p, "/v1/create/video");
    }

    #[test]
    fn state_labels_are_plain_words() {
        assert_eq!(state_label("done"), "Done");
        assert!(state_label("running").starts_with("Generating"));
        assert_eq!(state_label("weird"), "Unknown");
    }

    #[test]
    fn posters_follow_the_row_order() {
        let v = json!({ "jobs": [
            { "prompt": "old", "state": "failed", "message": "x", "files": [], "poster": null },
            { "prompt": "new", "state": "done", "message": "", "files": ["/a.mp4"], "poster": "/a.poster.png" }
        ]});
        assert_eq!(job_posters(&v), vec!["/a.poster.png".to_string(), String::new()]);
    }

    #[test]
    fn job_rows_show_newest_first_with_file_or_error() {
        let v = json!({ "jobs": [
            { "prompt": "old", "state": "failed", "message": "not enough free RAM", "files": [] },
            { "prompt": "new", "state": "done", "message": "", "files": ["/home/x/AI/Create/a.png"] }
        ]});
        let rows = job_rows(&v);
        assert_eq!(rows[0], ("new".into(), "Done", "/home/x/AI/Create/a.png".into()));
        assert_eq!(rows[1].2, "not enough free RAM");
    }
}
