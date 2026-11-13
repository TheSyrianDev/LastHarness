//! Outside CLI workers. A role can be bound to an external program (for example a coding CLI). The
//! program runs in a sandbox folder, with a timeout, and its stdout is the step result. A worker that
//! may write files needs a yes from the approver first; without it the step is parked, not run.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use lh_tools::BoxFut;
use tokio::process::Command;

use crate::engine::{Approver, Backend, StepJob, StepResult};
use crate::control::StepControl;
use crate::Role;

/// How to run one outside CLI.
#[derive(Clone, Debug)]
pub struct CliSpec {
    pub program: String,
    /// Arguments; `{brief}` is replaced by the lead's brief for the step.
    pub args: Vec<String>,
    /// Whether the CLI may change files in the sandbox. Such a step asks the approver first.
    pub writes: bool,
    pub timeout: Duration,
}

/// Run one CLI in `sandbox` for `brief`. Returns Done with stdout, Failed on error or timeout, or Parked
/// when `writes` is set and the approver says no.
pub async fn run_cli(spec: &CliSpec, brief: &str, sandbox: &PathBuf, approved: bool) -> StepResult {
    if spec.writes && !approved {
        return StepResult::Parked(format!("{} may change files and nobody approved it", spec.program));
    }
    let args: Vec<String> = spec.args.iter().map(|a| a.replace("{brief}", brief)).collect();
    let mut cmd = Command::new(&spec.program);
    cmd.args(&args).current_dir(sandbox).kill_on_drop(true);
    match tokio::time::timeout(spec.timeout, cmd.output()).await {
        Err(_) => StepResult::Failed(format!("{} ran past {} seconds and was stopped", spec.program, spec.timeout.as_secs())),
        Ok(Err(e)) => StepResult::Failed(format!("{} could not start: {e}", spec.program)),
        Ok(Ok(out)) if !out.status.success() => StepResult::Failed(format!("{} exited with {}: {}", spec.program, out.status, String::from_utf8_lossy(&out.stderr).trim())),
        Ok(Ok(out)) => {
            let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
            StepResult::Done { summary: text.lines().next().unwrap_or("").to_string(), result: text }
        }
    }
}

/// A backend that sends the roles bound to an outside CLI to that CLI, and every other step to `inner`.
pub struct WithExternal<B> {
    pub inner: B,
    /// role name to CLI
    pub clis: HashMap<String, CliSpec>,
    pub sandbox: PathBuf,
    pub approver: Arc<dyn Approver>,
}

impl<B: Backend> Backend for WithExternal<B> {
    fn run_step(&self, job: StepJob, ctl: StepControl) -> impl std::future::Future<Output = StepResult> + Send {
        let bound = self.clis.get(&job.role.name).cloned();
        let sandbox = self.sandbox.clone();
        let approver = self.approver.clone();
        let inner = self.inner.run_step(job.clone(), ctl);
        async move {
            match bound {
                None => inner.await,
                Some(spec) => {
                    let approved = !spec.writes || approver.write_ok(&job.dept, &spec.program).await;
                    run_cli(&spec, &job.brief, &sandbox, approved).await
                }
            }
        }
    }

    fn ask(&self, role: &Role, model: &str, prompt: String) -> impl std::future::Future<Output = Result<String, String>> + Send {
        self.inner.ask(role, model, prompt)
    }

    fn models(&self) -> impl std::future::Future<Output = Vec<String>> + Send {
        self.inner.models()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(program: &str, args: &[&str], writes: bool) -> CliSpec {
        CliSpec { program: program.into(), args: args.iter().map(|s| s.to_string()).collect(), writes, timeout: Duration::from_secs(5) }
    }

    #[tokio::test]
    async fn a_fake_cli_answers_in_the_sandbox() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().to_path_buf();
        let r = run_cli(&spec("sh", &["-c", "echo \"got: $1\" && pwd", "sh", "{brief}"], false), "fix it", &p, false).await;
        match r {
            StepResult::Done { result, .. } => {
                assert!(result.starts_with("got: fix it"), "{result}");
                assert!(result.contains(p.file_name().unwrap().to_str().unwrap()), "ran in the sandbox: {result}");
            }
            other => panic!("{other:?}"),
        }
    }

    #[tokio::test]
    async fn a_writing_cli_is_parked_without_approval() {
        let dir = tempfile::tempdir().unwrap();
        let r = run_cli(&spec("sh", &["-c", "touch made.txt"], true), "x", &dir.path().to_path_buf(), false).await;
        assert!(matches!(r, StepResult::Parked(_)));
        assert!(!dir.path().join("made.txt").exists());
    }

    #[tokio::test]
    async fn a_failing_or_slow_cli_is_a_failed_step() {
        let dir = tempfile::tempdir().unwrap().path().to_path_buf();
        std::fs::create_dir_all(&dir).unwrap();
        assert!(matches!(run_cli(&spec("sh", &["-c", "exit 3"], false), "x", &dir, false).await, StepResult::Failed(m) if m.contains("exited")));
        let mut slow = spec("sh", &["-c", "sleep 3"], false);
        slow.timeout = Duration::from_millis(100);
        assert!(matches!(run_cli(&slow, "x", &dir, false).await, StepResult::Failed(m) if m.contains("ran past")));
    }
}
