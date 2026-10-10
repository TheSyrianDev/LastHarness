use lh_provider::{ChatRequest, Error, Event, FinishReason, Message, Provider, Response};
use serde_json::Value;

use crate::{Answer, AskRequest, Asker, Decision, Permissions, ToolSet};

/// Anything that can answer a chat request. `Provider` is the real one; tests use a script.
pub trait Model: Send + Sync {
    fn complete(&self, req: &ChatRequest, on_event: &mut (dyn FnMut(Event) + Send)) -> impl std::future::Future<Output = Result<Response, Error>> + Send;
}

impl Model for Provider {
    async fn complete(&self, req: &ChatRequest, on_event: &mut (dyn FnMut(Event) + Send)) -> Result<Response, Error> {
        self.chat_stream(req, on_event).await
    }
}

pub struct TurnOptions {
    /// Most model calls in one turn, so a confused model cannot loop forever. Default 20.
    pub max_steps: u32,
    /// Name shown in permission prompts.
    pub worker: String,
}

impl Default for TurnOptions {
    fn default() -> Self {
        Self { max_steps: 20, worker: "assistant".into() }
    }
}

#[derive(Debug)]
pub enum TurnEnd {
    /// The model gave a final reply (it may be empty if `finish` is `Length`; check it).
    Done(Response),
    /// `max_steps` model calls were used and the model was still asking for tools.
    StepLimit,
}

/// Run one user turn: call the model, run the tools it asks for under the permission rules, give the
/// results back, and repeat until it answers without tools. `req.messages` ends up holding the whole
/// exchange, ready for the next turn.
pub async fn run_turn<M: Model, A: Asker>(
    model: &M,
    req: &mut ChatRequest,
    tools: &ToolSet,
    perms: &Permissions,
    asker: &A,
    opts: &TurnOptions,
    mut on_event: impl FnMut(Event) + Send,
) -> Result<TurnEnd, Error> {
    req.tools = tools.defs();
    for _ in 0..opts.max_steps.max(1) {
        let resp = model.complete(req, &mut on_event).await?;
        if resp.tool_calls.is_empty() {
            req.messages.push(Message::assistant(resp.text.clone(), vec![]));
            return Ok(TurnEnd::Done(resp));
        }
        let calls = resp.tool_calls.clone();
        let cut_off = resp.finish == Some(FinishReason::Length);
        req.messages.push(resp.into_message());
        for call in calls {
            let result = if call.repaired || cut_off {
                // Arguments that were cut short are a guess. Never run a guessed command.
                "This tool call was cut off before it was complete, so it was not run. Try again with a shorter call.".to_string()
            } else {
                execute(&call.name, call.parsed(), tools, perms, asker, opts).await
            };
            req.messages.push(Message::tool_result(&call.id, result));
        }
    }
    Ok(TurnEnd::StepLimit)
}

async fn execute<A: Asker>(name: &str, args: Result<Value, serde_json::Error>, tools: &ToolSet, perms: &Permissions, asker: &A, opts: &TurnOptions) -> String {
    let Some(tool) = tools.get(name) else { return format!("Error: the tool `{name}` is not available.") };
    let args = match args {
        Ok(v) => v,
        Err(e) => return format!("Error: the arguments are not valid JSON ({e})."),
    };
    let action = match tool.action(&args, perms.root()) {
        Ok(a) => a,
        Err(e) => return format!("Error: {e}"),
    };
    match perms.decide(&action) {
        Decision::Deny(why) => return format!("Denied: {why}"),
        Decision::Ask { reason, session_grant } => {
            let req = AskRequest { tool: tool.name(), what: format!("{action:?}"), reason, worker: &opts.worker };
            match asker.ask(&req).await {
                Answer::No => return "The user said no. Do not retry the same thing; ask what to do instead.".into(),
                Answer::Yes if session_grant => perms.grant_session_writes(),
                Answer::Yes => {}
            }
        }
        Decision::Allow => {}
    }
    match tool.run(&args, perms.root()).await {
        Ok(out) => out,
        Err(e) => format!("Error: {e}"),
    }
}
