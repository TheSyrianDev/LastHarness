//! The real `Backend`: workers run through the same turn loop as the assistant.
//!
//! A worker's prompt is the base prompt, the role's own short line, and the lead's brief as the one user
//! message. Nothing else is added: no history, no rules text, no memory dump. What the role needs from
//! memory it recalls with its tools.

use std::path::PathBuf;
use std::sync::Arc;

use lh_provider::{ChatRequest, FinishReason, Message, Sampling};
use lh_tools::{base_prompt, run_turn, Asker, BaseContext, Identity, Meter, Mode, Model, Permissions, SessionKind, ToolSet, TurnEnd, TurnOptions};
use serde_json::json;

use crate::control::StepControl;
use crate::engine::{Backend, StepJob, StepResult};
use crate::{excerpt, MemorySlice, Role, RoleSampling};

/// Where and how workers may act. Project work is locked to its folder.
#[derive(Clone, Debug)]
pub struct WorkerEnv {
    pub home: PathBuf,
    pub mode: Mode,
    pub sudo_enabled: bool,
    pub allow_rules: Vec<String>,
    /// Permission settings that may change while a run is going: each step reads them when it starts
    /// and at every tool call. Workers keep their own mode apart from bypass and sudo.
    pub live: Option<Arc<lh_tools::LivePerms>>,
}

/// Gives a role its tools. The core supplies one that knows about memory, web settings and skills.
pub type Toolbox = Arc<dyn Fn(&Role, &StepJob) -> ToolSet + Send + Sync>;

pub struct ModelBackend<M: Model, T: Meter, A: Asker> {
    pub model: Arc<M>,
    pub meter: Arc<T>,
    pub asker: Arc<A>,
    pub env: WorkerEnv,
    pub toolbox: Toolbox,
    pub provider_name: Option<String>,
    /// Token limit for a worker's reply and for the lead's answers. Thinking models need room.
    pub max_tokens: u32,
}

fn sampling(s: &RoleSampling, thinking: Option<bool>) -> Sampling {
    let mut out = Sampling { temperature: s.temperature, top_p: s.top_p, top_k: s.top_k, min_p: s.min_p, repeat_penalty: s.repeat_penalty, ..Default::default() };
    if thinking == Some(false) {
        out.extra.insert("chat_template_kwargs".into(), json!({"enable_thinking": false}));
    }
    out
}

impl<M: Model + 'static, T: Meter + 'static, A: Asker + 'static> ModelBackend<M, T, A> {
    async fn system(&self, role: &Role, model: &str, job: Option<&StepJob>, tools: &ToolSet) -> String {
        let caps = self.model.capabilities(model).await;
        let provider = match &self.provider_name {
            Some(p) => Some(p.clone()),
            None => self.model.provider_name().await,
        };
        let folder = job.and_then(|j| j.project.as_ref().map(PathBuf::from));
        let guidelines = tools.guidelines();
        let mut text = base_prompt(&BaseContext {
            identity: Identity { model: Some(model.to_string()), parameters: caps.parameters, provider },
            kind: if folder.is_some() { SessionKind::Project } else { SessionKind::Assistant },
            folder: folder.as_deref(),
            permissions: self.env.mode,
            sudo_enabled: self.env.sudo_enabled,
            date: &lh_tools::today(),
            guidelines: &guidelines,
        });
        if !role.instructions.trim().is_empty() {
            text.push_str("\n\n");
            text.push_str(role.instructions.trim());
        }
        text
    }

    fn brief(job: &StepJob) -> String {
        let mut out = job.brief.trim().to_string();
        for (title, result) in &job.inputs {
            out.push_str(&format!("\n\nFrom the step \"{title}\":\n{result}"));
        }
        if let Some(f) = &job.fix {
            out.push_str(&format!("\n\nYour earlier result:\n{}\n\nFix only this, then give the whole corrected result: {}", excerpt(&f.previous, 3000), f.instruction));
        }
        out
    }
}

impl<M: Model + 'static, T: Meter + 'static, A: Asker + 'static> Backend for ModelBackend<M, T, A> {
    async fn run_step(&self, job: StepJob, ctl: StepControl) -> StepResult {
        let tools = (self.toolbox)(&job.role, &job);
        let mut perms = match Permissions::new(job.project.as_ref().map(PathBuf::from).as_ref().unwrap_or(&self.env.home)) {
            Ok(p) => p,
            Err(e) => return StepResult::Failed(format!("cannot use the folder: {e}")),
        };
        perms.mode = self.env.mode;
        perms.sudo_enabled = self.env.sudo_enabled;
        perms.allow_rules = self.env.allow_rules.clone();
        perms.live = self.env.live.clone();
        perms.locked = job.project.is_some();
        let system = self.system(&job.role, &job.model, Some(&job), &tools).await;
        let mut req = ChatRequest {
            model: job.model.clone(),
            messages: vec![Message::system(system), Message::user(Self::brief(&job))],
            max_tokens: Some(self.max_tokens),
            sampling: sampling(&job.role.sampling, job.role.thinking),
            ..Default::default()
        };
        let opts = TurnOptions {
            max_steps: job.role.max_steps.max(1),
            worker: job.role.name.clone(),
            gate: Some(Arc::new(move || {
                let c = ctl.clone();
                Box::pin(async move { c.checkpoint().await })
            })),
        };
        match run_turn(&*self.model, &*self.meter, &mut req, &tools, &perms, &*self.asker, &opts, |_| {}).await {
            Ok(TurnEnd::Done(r)) if !r.text.trim().is_empty() => StepResult::Done { summary: excerpt(r.text.trim(), 240), result: r.text.trim().to_string() },
            Ok(TurnEnd::Done(r)) if r.finish == Some(FinishReason::Length) => StepResult::Failed("the model ran out of tokens before it answered (it may have used them all thinking)".into()),
            Ok(TurnEnd::Done(_)) => StepResult::Failed("the model finished without an answer".into()),
            Ok(TurnEnd::StepLimit) => StepResult::Failed(format!("the worker used all {} model calls without finishing", job.role.max_steps)),
            Ok(TurnEnd::Refused(why)) => StepResult::Failed(format!("stopped by the spending rules: {why}")),
            Ok(TurnEnd::Stopped) => StepResult::Stopped,
            Err(e) => StepResult::Failed(e.to_string()),
        }
    }

    async fn ask(&self, role: &Role, model: &str, prompt: String) -> Result<String, String> {
        let tools = ToolSet::default();
        let mut req = ChatRequest {
            model: model.to_string(),
            messages: vec![Message::system(self.system(role, model, None, &tools).await), Message::user(prompt)],
            max_tokens: Some(self.max_tokens),
            sampling: sampling(&role.sampling, role.thinking),
            ..Default::default()
        };
        let opts = TurnOptions { max_steps: 1, worker: role.name.clone(), gate: None };
        let perms = Permissions::new(&self.env.home).map_err(|e| e.to_string())?;
        match run_turn(&*self.model, &*self.meter, &mut req, &tools, &perms, &*self.asker, &opts, |_| {}).await {
            Ok(TurnEnd::Done(r)) if !r.text.trim().is_empty() => Ok(r.text),
            Ok(TurnEnd::Done(_)) => Err("the model gave no answer (it may have run out of tokens while thinking)".into()),
            Ok(TurnEnd::Refused(why)) => Err(format!("stopped by the spending rules: {why}")),
            Ok(_) => Err("the model did not finish".into()),
            Err(e) => Err(e.to_string()),
        }
    }

    async fn models(&self) -> Vec<String> {
        self.model.models().await.unwrap_or_default()
    }
}

/// The tools a role may use, taken from the full set. A role with a memory slice of `None` gets no
/// memory tools; a slice of layers is enforced inside the memory handle the toolbox builds.
pub fn tools_for_role(full: &ToolSet, role: &Role) -> ToolSet {
    let mut names = role.tools.clone();
    if role.memory == MemorySlice::None {
        names.retain(|n| !matches!(n.as_str(), "recall" | "open_note" | "remember" | "forget"));
    }
    full.clone().only(&names)
}
