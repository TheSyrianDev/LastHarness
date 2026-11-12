//! Register a downloaded GGUF with llama-swap: matrix entry, config entry, validation, restart, check.
//! Follows `comfyui/CREATE_TAB_CONTRACT.md` ("Registering a newly downloaded GGUF with llama-swap").
//! The text edits are pure functions so tests run on a temp copy; `run` does the file and process steps.

use std::path::{Path, PathBuf};

/// What to register: the model id (`<Model>-low`), the folder under `~/AI/Models`, and the file name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunPlan {
    pub id: String,
    pub folder: String,
    pub file: String,
    pub ctx: u32,
}

/// Model id from a GGUF file name: drop the quant tag, keep the name, add `-low`. `Qwen3-1.7B-Q4_K_M.gguf` becomes `Qwen3-1.7B-low`.
pub fn default_id(file: &str) -> String {
    let stem = file.strip_suffix(".gguf").unwrap_or(file);
    let mut parts: Vec<&str> = stem.split('-').collect();
    if parts.len() > 1 && crate::models_catalog::is_quant_tag(parts[parts.len() - 1]) {
        parts.pop();
    }
    format!("{}-low", parts.join("-"))
}

/// The config.yaml block for one model (4 GB card defaults from the contract, KV q8_0, ttl 300).
pub fn config_entry(plan: &RunPlan) -> String {
    format!(
        "  \"{id}\":\n    name: \"{id}\"\n    ttl: 300\n    cmd: ${{mainline}} --port ${{PORT}} -m ${{M}}/{folder}/{file} -ngl 99 -fa on -ctk q8_0 -ctv q8_0 -t 6 -np 1 -ub 1024 --cache-ram 0 -c {ctx} --jinja\n",
        id = plan.id,
        folder = plan.folder,
        file = plan.file,
        ctx = plan.ctx
    )
}

/// The matrix.yaml block (tier low only, text sharp template, no vision).
pub fn matrix_entry(plan: &RunPlan) -> String {
    format!(
        "  {id}:\n    tiers: [low]\n    contexts: {{low: {ctx}}}\n    text: [sharp]\n    vision: false\n    modes: [think, instruct]\n    defaultMode: think\n\n",
        id = plan.id,
        ctx = plan.ctx
    )
}

/// Put `block` right after the first line equal to `header` (`models:`). Refuses if `id_key` is already there.
fn insert_after_header(text: &str, header: &str, id_key: &str, block: &str) -> Result<String, String> {
    if text.lines().any(|l| l.trim() == id_key) {
        return Err(format!("{id_key} is already registered"));
    }
    let mut out = String::with_capacity(text.len() + block.len());
    let mut done = false;
    for line in text.split_inclusive('\n') {
        out.push_str(line);
        if !done && line.trim_end() == header {
            out.push_str(block);
            done = true;
        }
    }
    if done { Ok(out) } else { Err(format!("no `{header}` section found")) }
}

/// Add the model to the config text. `Err` when the id already exists or the section is missing.
pub fn add_config_entry(config: &str, plan: &RunPlan) -> Result<String, String> {
    insert_after_header(config, "models:", &format!("\"{}\":", plan.id), &config_entry(plan))
}

/// Add the model to the matrix text.
pub fn add_matrix_entry(matrix: &str, plan: &RunPlan) -> Result<String, String> {
    insert_after_header(matrix, "models:", &format!("{}:", plan.id), &matrix_entry(plan))
}

/// True when the config already names this file (so the model is registered).
pub fn is_registered(config: &str, file: &str) -> bool {
    config.contains(&format!("/{file}"))
}

/// What `run` did, for the report to the app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunReport {
    pub id: String,
    pub backup: PathBuf,
}

/// The external steps `run` needs. Production uses `SystemSteps`; tests use a fake.
pub trait Steps {
    /// Run the matrix validator. `Err` carries its output.
    fn validate(&self, rig: &Path) -> Result<(), String>;
    /// Restart llama-swap, then return the model ids it lists.
    fn restart_and_list(&self) -> Result<Vec<String>, String>;
}

/// Full flow. Refuses while the machine lock has content (another session). Writes the matrix and config
/// only after taking the lock, keeps `.bak` copies, and restores them when validation fails.
pub fn run(rig: &Path, lock: &Path, plan: &RunPlan, steps: &dyn Steps) -> Result<RunReport, String> {
    crate::machine_lock::take(lock, &format!("Run {}", plan.id))?;
    let result = run_locked(rig, plan, steps);
    crate::machine_lock::release(lock);
    result
}

fn run_locked(rig: &Path, plan: &RunPlan, steps: &dyn Steps) -> Result<RunReport, String> {
    let cfg_path = rig.join("router/config.yaml");
    let mtx_path = rig.join("router/matrix.yaml");
    let config = std::fs::read_to_string(&cfg_path).map_err(|e| format!("cannot read config.yaml: {e}"))?;
    let matrix = std::fs::read_to_string(&mtx_path).map_err(|e| format!("cannot read matrix.yaml: {e}"))?;
    let new_matrix = add_matrix_entry(&matrix, plan)?;
    let new_config = add_config_entry(&config, plan)?;
    let stamp = now_stamp().replace([':', '.', '+'], "");
    let backup = rig.join(format!("router/config.yaml.bak-run-{stamp}"));
    std::fs::write(&backup, &config).map_err(|e| format!("cannot write the backup: {e}"))?;
    std::fs::write(&mtx_path, &new_matrix).map_err(|e| format!("cannot write matrix.yaml: {e}"))?;
    std::fs::write(&cfg_path, &new_config).map_err(|e| format!("cannot write config.yaml: {e}"))?;
    let restore = || {
        let _ = std::fs::write(&cfg_path, &config);
        let _ = std::fs::write(&mtx_path, &matrix);
    };
    if let Err(out) = steps.validate(rig) {
        restore();
        return Err(format!("the matrix check failed, files restored: {out}"));
    }
    let ids = steps.restart_and_list().map_err(|e| format!("llama-swap restart failed: {e}"))?;
    if !ids.iter().any(|i| i == &plan.id) {
        return Err(format!("{} is not in /v1/models after the restart", plan.id));
    }
    Ok(RunReport { id: plan.id.clone(), backup })
}

/// Current local time as an ISO-like stamp for lock lines and backup names.
fn now_stamp() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    format!("{secs}")
}

/// Production steps: `tools/check-router-matrix.py` with `/usr/bin/python3`, `systemctl --user restart llama-swap`, `GET 127.0.0.1:9292/v1/models`.
pub struct SystemSteps;

impl Steps for SystemSteps {
    fn validate(&self, rig: &Path) -> Result<(), String> {
        let out = std::process::Command::new("/usr/bin/python3").arg("tools/check-router-matrix.py").current_dir(rig).output().map_err(|e| e.to_string())?;
        if out.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&out.stdout).trim().to_string() + &String::from_utf8_lossy(&out.stderr)) }
    }

    fn restart_and_list(&self) -> Result<Vec<String>, String> {
        let st = std::process::Command::new("systemctl").args(["--user", "restart", "llama-swap"]).status().map_err(|e| e.to_string())?;
        if !st.success() {
            return Err("systemctl restart returned an error".into());
        }
        std::thread::sleep(std::time::Duration::from_secs(5));
        let body = ureq_get("http://127.0.0.1:9292/v1/models")?;
        let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
        Ok(v["data"].as_array().map(|a| a.iter().filter_map(|m| m["id"].as_str().map(str::to_string)).collect()).unwrap_or_default())
    }
}

fn ureq_get(url: &str) -> Result<String, String> {
    let out = std::process::Command::new("curl").args(["-s", "-m", "20", url]).output().map_err(|e| e.to_string())?;
    if out.status.success() { Ok(String::from_utf8_lossy(&out.stdout).to_string()) } else { Err(format!("cannot reach {url}")) }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = "models:\n  # comment\n  \"Old-low\":\n    name: \"Old-low\"\n";
    const MATRIX: &str = "helpers: []\n\nmodels:\n  Old:\n    tiers: [low]\n";

    fn plan() -> RunPlan {
        RunPlan { id: "Qwen3-1.7B-low".into(), folder: "qwen3-1.7b-gguf".into(), file: "Qwen3-1.7B-Q4_K_M.gguf".into(), ctx: 32768 }
    }

    struct Fake {
        valid: bool,
        listed: Vec<String>,
    }
    impl Steps for Fake {
        fn validate(&self, _: &Path) -> Result<(), String> {
            if self.valid { Ok(()) } else { Err("matrix problem".into()) }
        }
        fn restart_and_list(&self) -> Result<Vec<String>, String> {
            Ok(self.listed.clone())
        }
    }

    #[test]
    fn default_id_drops_quant_tag() {
        assert_eq!(default_id("Qwen3-1.7B-Q4_K_M.gguf"), "Qwen3-1.7B-low");
        assert_eq!(default_id("Model-Name-BF16.gguf"), "Model-Name-low");
        assert_eq!(default_id("Plain.gguf"), "Plain-low");
    }

    #[test]
    fn config_entry_is_inserted_under_models_and_refused_twice() {
        let out = add_config_entry(CONFIG, &plan()).unwrap();
        assert!(out.starts_with("models:\n  \"Qwen3-1.7B-low\":"));
        assert!(out.contains("-m ${M}/qwen3-1.7b-gguf/Qwen3-1.7B-Q4_K_M.gguf -ngl 99"));
        assert!(out.contains("-c 32768 --jinja"));
        assert!(add_config_entry(&out, &plan()).unwrap_err().contains("already registered"));
    }

    #[test]
    fn matrix_entry_is_added_and_missing_section_is_an_error() {
        let out = add_matrix_entry(MATRIX, &plan()).unwrap();
        assert!(out.contains("models:\n  Qwen3-1.7B-low:\n    tiers: [low]"));
        assert!(add_matrix_entry("tiers: []\n", &plan()).is_err());
    }

    #[test]
    fn registered_check_uses_the_file_name() {
        let cfg = add_config_entry(CONFIG, &plan()).unwrap();
        assert!(is_registered(&cfg, "Qwen3-1.7B-Q4_K_M.gguf"));
        assert!(!is_registered(CONFIG, "Qwen3-1.7B-Q4_K_M.gguf"));
    }

    fn temp_rig(name: &str) -> (PathBuf, PathBuf) {
        let base = std::env::temp_dir().join(format!("lh-run-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("router")).unwrap();
        std::fs::write(base.join("router/config.yaml"), CONFIG).unwrap();
        std::fs::write(base.join("router/matrix.yaml"), MATRIX).unwrap();
        let lock = base.join("machine-busy");
        let _ = std::fs::remove_file(&lock);
        (base, lock)
    }

    #[test]
    fn run_writes_then_reports_success_and_releases_lock() {
        let (rig, lock) = temp_rig("ok");
        let fake = Fake { valid: true, listed: vec!["Qwen3-1.7B-low".into()] };
        let r = run(&rig, &lock, &plan(), &fake).unwrap();
        assert_eq!(r.id, "Qwen3-1.7B-low");
        assert!(r.backup.exists());
        assert!(std::fs::read_to_string(rig.join("router/config.yaml")).unwrap().contains("Qwen3-1.7B-low"));
        assert!(!lock.exists());
    }

    #[test]
    fn run_restores_files_when_validation_fails() {
        let (rig, lock) = temp_rig("bad");
        let fake = Fake { valid: false, listed: vec![] };
        let err = run(&rig, &lock, &plan(), &fake).unwrap_err();
        assert!(err.contains("files restored"));
        assert_eq!(std::fs::read_to_string(rig.join("router/config.yaml")).unwrap(), CONFIG);
        assert_eq!(std::fs::read_to_string(rig.join("router/matrix.yaml")).unwrap(), MATRIX);
    }

    #[test]
    fn run_refuses_while_another_session_holds_the_lock() {
        let (rig, lock) = temp_rig("busy");
        std::fs::write(&lock, "lastengine-74 test\n").unwrap();
        let fake = Fake { valid: true, listed: vec![] };
        assert!(run(&rig, &lock, &plan(), &fake).unwrap_err().contains("busy"));
        assert_eq!(std::fs::read_to_string(rig.join("router/config.yaml")).unwrap(), CONFIG);
        assert_eq!(std::fs::read_to_string(&lock).unwrap(), "lastengine-74 test\n");
    }
}
