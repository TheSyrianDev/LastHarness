//! Create tab jobs on the ComfyUI service (127.0.0.1:8188): fill a workflow template, submit, poll, read outputs.
//! Rules come from `~/AI/Rig/comfyui/CREATE_TAB_CONTRACT.md`. This module is pure apart from `ComfyHttp`.

use serde_json::{json, Value};

/// Fill the Z-Image workflow (node ids fixed in `z-image-turbo-gguf.json`).
pub fn image_workflow(template: &Value, prompt: &str, seed: u64, width: u32, height: u32, steps: u32, cfg: f64, prefix: &str) -> Result<Value, String> {
    let mut w = template.clone();
    set(&mut w, "4", "text", json!(prompt))?;
    set(&mut w, "8", "seed", json!(seed))?;
    set(&mut w, "6", "width", json!(width))?;
    set(&mut w, "6", "height", json!(height))?;
    set(&mut w, "8", "steps", json!(steps))?;
    set(&mut w, "8", "cfg", json!(cfg))?;
    set(&mut w, "10", "filename_prefix", json!(prefix))?;
    Ok(w)
}

/// Fill the Wan video workflow. `frames` must be 4n+1 (17, 21, ...).
pub fn video_workflow(template: &Value, prompt: &str, negative: &str, seed: u64, width: u32, height: u32, frames: u32, steps: u32, cfg: f64, prefix: &str) -> Result<Value, String> {
    if frames % 4 != 1 {
        return Err(format!("frames must be 4n+1 (17, 21, 25 ...), not {frames}"));
    }
    let mut w = template.clone();
    set(&mut w, "4", "text", json!(prompt))?;
    set(&mut w, "5", "text", json!(negative))?;
    set(&mut w, "8", "seed", json!(seed))?;
    set(&mut w, "6", "width", json!(width))?;
    set(&mut w, "6", "height", json!(height))?;
    set(&mut w, "6", "length", json!(frames))?;
    set(&mut w, "8", "steps", json!(steps))?;
    set(&mut w, "8", "cfg", json!(cfg))?;
    set(&mut w, "12", "filename_prefix", json!(prefix))?;
    Ok(w)
}

fn set(w: &mut Value, node: &str, input: &str, value: Value) -> Result<(), String> {
    let slot = w.get_mut(node).and_then(|n| n.get_mut("inputs")).and_then(|i| i.get_mut(input)).ok_or_else(|| format!("workflow node {node} has no input {input}"))?;
    *slot = value;
    Ok(())
}

/// One finished output file, as ComfyUI lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutFile {
    pub filename: String,
    pub subfolder: String,
}

/// Job state from `GET /history/<id>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Done {
    /// Not finished yet (or not in history yet): poll again.
    Pending,
    /// Finished. `outputs` are the files of the output node (`10` for image, `12` for video).
    Success { outputs: Vec<OutFile> },
    /// `status_str: error`, with the messages ComfyUI gives.
    Failed { message: String },
}

/// Read `GET /history/<prompt_id>` and say what state the job is in.
pub fn parse_history(history: &Value, prompt_id: &str, output_node: &str) -> Result<Done, String> {
    let Some(job) = history.get(prompt_id) else { return Ok(Done::Pending) };
    let status = &job["status"];
    if !status["completed"].as_bool().unwrap_or(false) {
        return Ok(Done::Pending);
    }
    if status["status_str"].as_str() == Some("error") {
        let message = status["messages"].to_string();
        return Ok(Done::Failed { message });
    }
    let outputs = job["outputs"][output_node]["images"]
        .as_array()
        .map(|a| a.iter().filter_map(|f| Some(OutFile { filename: f["filename"].as_str()?.to_string(), subfolder: f["subfolder"].as_str().unwrap_or("").to_string() })).collect())
        .unwrap_or_default();
    Ok(Done::Success { outputs })
}

/// Seed for a new request: never a repeat, so ComfyUI's cache is not hit.
pub fn fresh_seed(nanos: u128) -> u64 {
    // 0..=2^31-1 for every job (image and video): the 64-bit seeds of earlier runs gave colour noise on Wan
    let x = (nanos as u64) ^ 0x9E37_79B9_7F4A_7C15;
    x % 0x8000_0000
}

#[cfg(test)]
mod seed_tests {
    #[test]
    fn fresh_seed_stays_in_31_bits_for_every_input() {
        for n in [0u128, 1, u64::MAX as u128, 9714302917628910720u128, u128::MAX] {
            let s = super::fresh_seed(n);
            assert!(s < 1 << 31, "{s}");
        }
    }
}

/// Minimal HTTP to the ComfyUI service through `curl`, so the core needs no new HTTP dependency.
pub struct ComfyHttp {
    pub base: String,
}

impl ComfyHttp {
    pub fn local() -> Self {
        Self { base: "http://127.0.0.1:8188".into() }
    }

    fn curl(&self, args: &[&str]) -> Result<String, String> {
        let out = std::process::Command::new("curl").args(["-s", "-m", "30"]).args(args).output().map_err(|e| e.to_string())?;
        if out.status.success() { Ok(String::from_utf8_lossy(&out.stdout).to_string()) } else { Err(format!("cannot reach {}", self.base)) }
    }

    /// `system_stats` answers: the service is up.
    pub fn up(&self) -> bool {
        self.curl(&[&format!("{}/system_stats", self.base)]).is_ok()
    }

    /// Submit a filled workflow. Returns the prompt id. `node_errors` non-empty is an error.
    pub fn submit(&self, workflow: &Value, client_id: &str) -> Result<String, String> {
        let body = json!({ "prompt": workflow, "client_id": client_id }).to_string();
        let text = self.curl(&["-H", "Content-Type: application/json", "-d", &body, &format!("{}/prompt", self.base)])?;
        let v: Value = serde_json::from_str(&text).map_err(|e| format!("bad answer from ComfyUI: {e}"))?;
        if v["node_errors"].as_object().is_some_and(|m| !m.is_empty()) {
            return Err(format!("ComfyUI refused the workflow: {}", v["node_errors"]));
        }
        v["prompt_id"].as_str().map(str::to_string).ok_or_else(|| "ComfyUI gave no prompt id".into())
    }

    pub fn history(&self, prompt_id: &str) -> Result<Value, String> {
        let text = self.curl(&[&format!("{}/history/{prompt_id}", self.base)])?;
        serde_json::from_str(&text).map_err(|e| format!("bad history from ComfyUI: {e}"))
    }

    /// Free VRAM after every job (contract step 5).
    pub fn free(&self) -> Result<(), String> {
        self.curl(&["-H", "Content-Type: application/json", "-d", r#"{"unload_models":true,"free_memory":true}"#, &format!("{}/free", self.base)]).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn z_template() -> Value {
        json!({
            "4": {"class_type": "CLIPTextEncode", "inputs": {"text": "x"}},
            "6": {"class_type": "EmptySD3LatentImage", "inputs": {"width": 768, "height": 768}},
            "8": {"class_type": "KSampler", "inputs": {"seed": 1, "steps": 8, "cfg": 1.0}},
            "10": {"class_type": "SaveImage", "inputs": {"filename_prefix": "bench/zimage"}}
        })
    }

    #[test]
    fn image_workflow_sets_prompt_seed_size_steps_and_prefix() {
        let w = image_workflow(&z_template(), "a red fox", 42, 512, 512, 8, 1.0, "app/z").unwrap();
        assert_eq!(w["4"]["inputs"]["text"], "a red fox");
        assert_eq!(w["8"]["inputs"]["seed"], 42);
        assert_eq!(w["6"]["inputs"]["width"], 512);
        assert_eq!(w["10"]["inputs"]["filename_prefix"], "app/z");
    }

    #[test]
    fn missing_node_is_an_error_not_a_silent_skip() {
        let bad = json!({"4": {"inputs": {"text": ""}}});
        assert!(image_workflow(&bad, "x", 1, 8, 8, 1, 1.0, "p").is_err());
    }

    #[test]
    fn video_frames_must_be_4n_plus_1() {
        let t = json!({"4": {"inputs": {"text": ""}}, "5": {"inputs": {"text": ""}}, "6": {"inputs": {"width": 0, "height": 0, "length": 0}}, "8": {"inputs": {"seed": 0, "steps": 0, "cfg": 0.0}}, "12": {"inputs": {"filename_prefix": ""}}});
        assert!(video_workflow(&t, "p", "n", 1, 832, 480, 16, 20, 5.0, "v").is_err());
        let ok = video_workflow(&t, "p", "n", 1, 832, 480, 17, 20, 5.0, "v").unwrap();
        assert_eq!(ok["6"]["inputs"]["length"], 17);
    }

    #[test]
    fn history_pending_success_and_error() {
        let pending = json!({});
        assert_eq!(parse_history(&pending, "id1", "10").unwrap(), Done::Pending);
        let running = json!({"id1": {"status": {"completed": false, "status_str": "success"}}});
        assert_eq!(parse_history(&running, "id1", "10").unwrap(), Done::Pending);
        let ok = json!({"id1": {"status": {"completed": true, "status_str": "success"},
            "outputs": {"10": {"images": [{"filename": "zimage_00001_.png", "subfolder": "bench", "type": "output"}]}}}});
        assert_eq!(
            parse_history(&ok, "id1", "10").unwrap(),
            Done::Success { outputs: vec![OutFile { filename: "zimage_00001_.png".into(), subfolder: "bench".into() }] }
        );
        let err = json!({"id1": {"status": {"completed": true, "status_str": "error", "messages": [["x", "out of memory"]]}}});
        match parse_history(&err, "id1", "10").unwrap() {
            Done::Failed { message } => assert!(message.contains("out of memory")),
            other => panic!("expected failure, got {other:?}"),
        }
    }

    #[test]
    fn seeds_differ_for_different_times() {
        assert_ne!(fresh_seed(1), fresh_seed(2));
    }
}

/// Where a finished image is copied: `~/AI/Create/`.
pub fn output_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join("AI/Create"))
}

/// Free RAM in bytes from `/proc/meminfo` (`MemAvailable`).
pub fn mem_available() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    let line = text.lines().find(|l| l.starts_with("MemAvailable:"))?;
    let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kb * 1024)
}

/// Longest a job may run before we give up and free VRAM (a 17-frame Wan video takes about 3 minutes).
pub const MAX_WAIT_SECS: u64 = 900;

/// Steps of one job (image or video), in contract order: lock, unload llama-swap, RAM check, run, free, unlock.
/// Returns the copied file paths. The lock file gets one line from us, removed only if still ours.
pub fn run_image_job(lock: &std::path::Path, workflow: &Value, output_node: &str, ram_needed: u64, http: &ComfyHttp, out_dir: &std::path::Path, label: &str) -> Result<Vec<std::path::PathBuf>, String> {
    crate::machine_lock::take(lock, &format!("Create {label}"))?;
    let result = (|| {
        if !http.up() {
            return Err("ComfyUI is not running on 127.0.0.1:8188 (start the comfyui service)".to_string());
        }
        let _ = std::process::Command::new("curl").args(["-s", "-m", "10", "127.0.0.1:9292/unload"]).output();
        match mem_available() {
            Some(free) if free < ram_needed => return Err(format!("not enough free RAM: {} MiB free, {} MiB needed", free >> 20, ram_needed >> 20)),
            None => return Err("cannot read free RAM from /proc/meminfo".into()),
            _ => {}
        }
        let id = http.submit(workflow, "lastai-create")?;
        let started = std::time::Instant::now();
        let outputs = loop {
            std::thread::sleep(std::time::Duration::from_secs(2));
            if started.elapsed() > std::time::Duration::from_secs(MAX_WAIT_SECS) {
                return Err(format!("ComfyUI did not finish within {MAX_WAIT_SECS} s"));
            }
            match parse_history(&http.history(&id)?, &id, output_node)? {
                Done::Pending => continue,
                Done::Failed { message } => return Err(format!("ComfyUI failed: {message}")),
                Done::Success { outputs } => break outputs,
            }
        };
        std::fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
        let mut saved = Vec::new();
        for f in outputs {
            let url = format!("{}/view?filename={}&subfolder={}&type=output", http.base, f.filename, f.subfolder);
            let out = std::process::Command::new("curl").args(["-s", "-m", "60", "-o"]).arg(out_dir.join(&f.filename)).arg(&url).status().map_err(|e| e.to_string())?;
            if !out.success() {
                return Err(format!("cannot copy {}", f.filename));
            }
            saved.push(out_dir.join(&f.filename));
        }
        Ok(saved)
    })();
    // always free VRAM after the job (contract step 5), then release our lock
    let _ = http.free();
    crate::machine_lock::release(lock);
    result
}

#[cfg(test)]
mod job_tests {
    use super::*;

    #[test]
    fn job_refuses_while_another_session_holds_the_lock() {
        let dir = std::env::temp_dir().join(format!("lh-create-lock-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let lock = dir.join(".machine-busy");
        std::fs::write(&lock, "lastengine-74 | busy | 2026-11-12\n").unwrap();
        let err = run_image_job(&lock, &json!({}), "10", 1, &ComfyHttp { base: "http://127.0.0.1:1".into() }, &dir, "t").unwrap_err();
        assert!(err.contains("busy"));
        assert_eq!(std::fs::read_to_string(&lock).unwrap(), "lastengine-74 | busy | 2026-11-12\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn mem_available_reads_a_positive_number_on_linux() {
        assert!(mem_available().unwrap_or(1) > 0);
    }
}

/// A still image to show inline for a finished job: the file itself for an image, or the first frame
/// of an MP4 (written next to it as `<name>.poster.png`, via `ffmpeg`). `None` when ffmpeg is missing.
pub fn poster_for(file: &std::path::Path, ffmpeg: &str) -> Option<std::path::PathBuf> {
    let ext = file.extension()?.to_str()?.to_ascii_lowercase();
    if ext != "mp4" {
        return Some(file.to_path_buf());
    }
    let out = file.with_extension("poster.png");
    let status = std::process::Command::new(ffmpeg).args(["-v", "error", "-y", "-i"]).arg(file).args(["-frames:v", "1"]).arg(&out).status().ok()?;
    (status.success() && out.exists()).then_some(out)
}

#[cfg(test)]
mod poster_tests {
    use super::poster_for;
    use std::path::Path;

    #[test]
    fn images_are_their_own_poster_and_missing_ffmpeg_gives_none() {
        assert_eq!(poster_for(Path::new("/x/a.png"), "ffmpeg"), Some(Path::new("/x/a.png").to_path_buf()));
        assert_eq!(poster_for(Path::new("/x/a.mp4"), "/nonexistent/ffmpeg-binary"), None);
    }
}

#[cfg(test)]
mod mock_comfy_tests {
    use super::*;
    use std::io::{Read, Write};

    /// A stand-in ComfyUI on 127.0.0.1: answers each route the job runner uses, then stops.
    fn serve(video_name: &'static str) -> String {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", l.local_addr().unwrap());
        std::thread::spawn(move || {
            for stream in l.incoming().take(12) {
                let Ok(mut s) = stream else { continue };
                let mut buf = [0u8; 4096];
                let n = s.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let line = req.lines().next().unwrap_or("").to_string();
                let body: Vec<u8> = if line.starts_with("POST /prompt") {
                    br#"{"prompt_id":"p1","node_errors":{}}"#.to_vec()
                } else if line.starts_with("GET /history/p1") {
                    format!(r#"{{"p1":{{"status":{{"completed":true,"status_str":"success"}},"outputs":{{"12":{{"images":[{{"filename":"{video_name}","subfolder":"","type":"output"}}]}}}}}}}}"#).into_bytes()
                } else if line.starts_with("GET /view") {
                    b"FAKEVIDEO".to_vec()
                } else {
                    b"{}".to_vec()
                };
                let head = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                let _ = s.write_all(head.as_bytes());
                let _ = s.write_all(&body);
            }
        });
        base
    }

    #[test]
    fn image_job_submits_waits_copies_and_releases_the_lock() {
        let dir = std::env::temp_dir().join(format!("lh-create-mock-{}", std::process::id()));
        let out = dir.join("out");
        std::fs::create_dir_all(&out).unwrap();
        let lock = dir.join(".machine-busy");
        let _ = std::fs::remove_file(&lock);
        let http = ComfyHttp { base: serve("wan22_test.mp4") };
        let files = run_image_job(&lock, &json!({}), "12", 0, &http, &out, "test").unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(std::fs::read(&files[0]).unwrap(), b"FAKEVIDEO");
        assert!(!lock.exists(), "our lock must be removed");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
