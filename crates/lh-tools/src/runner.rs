use std::sync::Arc;

use lh_provider::{Capabilities, ChatRequest, Error, Event, FinishReason, GeneratedImage, Message, Provider, Response, Usage};
use serde_json::Value;

use crate::tools::{BoxFut, Undo};
use crate::{Action, Answer, AskRequest, Asker, Decision, Permissions, Tool, ToolSet};

/// Anything that can answer a chat request. `Provider` is the real one; tests use a script.
pub trait Model: Send + Sync {
    fn complete(&self, req: &ChatRequest, on_event: &mut (dyn FnMut(Event) + Send)) -> impl std::future::Future<Output = Result<Response, Error>> + Send;

    /// What service this endpoint says it is (for the first line of the prompt), if it says.
    fn provider_name(&self) -> impl std::future::Future<Output = Option<String>> + Send {
        async { None }
    }

    /// Make images. Endpoints that cannot say so with an error.
    fn generate_images(&self, _model: &str, _prompt: &str, _size: Option<&str>, _n: u32) -> impl std::future::Future<Output = Result<Vec<GeneratedImage>, Error>> + Send {
        async { Err(Error::Format("this endpoint cannot generate images".into())) }
    }

    /// What the model supports. Servers that cannot say return all-unknown.
    fn capabilities(&self, _model: &str) -> impl std::future::Future<Output = Capabilities> + Send {
        async { Capabilities::default() }
    }

    /// The core went idle: drop connections and caches. Default does nothing.
    fn release_idle(&self) {}

    /// Model names this endpoint offers. Servers that cannot list them return an empty list.
    fn models(&self) -> impl std::future::Future<Output = Result<Vec<String>, Error>> + Send {
        async { Ok(Vec::new()) }
    }

    /// The same list with what the server says about each: `{"id", "modes": [..], "loaded": bool}` when it
    /// says it (llama-swap does). Default: names only.
    fn model_details(&self) -> impl std::future::Future<Output = Result<Vec<serde_json::Value>, Error>> + Send {
        async { Ok(self.models().await?.into_iter().map(|id| serde_json::json!({"id": id})).collect()) }
    }
}

impl Model for Provider {
    async fn complete(&self, req: &ChatRequest, on_event: &mut (dyn FnMut(Event) + Send)) -> Result<Response, Error> {
        self.chat_stream(req, on_event).await
    }

    async fn models(&self) -> Result<Vec<String>, Error> {
        Provider::models(self).await
    }

    async fn model_details(&self) -> Result<Vec<serde_json::Value>, Error> {
        Provider::model_details(self).await
    }

    async fn capabilities(&self, model: &str) -> Capabilities {
        Provider::capabilities(self, model).await
    }

    async fn provider_name(&self) -> Option<String> {
        Provider::service_name(self).await
    }

    async fn generate_images(&self, model: &str, prompt: &str, size: Option<&str>, n: u32) -> Result<Vec<GeneratedImage>, Error> {
        Provider::generate_images(self, model, prompt, size, n).await
    }

    fn release_idle(&self) {
        self.release_connections();
    }
}

/// What the spending rules say about one model call.
#[derive(Clone, Debug, PartialEq)]
pub enum MeterVerdict {
    Allow,
    /// Ask the user first; the text says what the call is expected to cost.
    Ask(String),
    Refuse(String),
}

/// Checks and records the cost of model calls. `NoMeter` does neither.
pub trait Meter: Send + Sync {
    fn before(&self, req: &ChatRequest) -> impl std::future::Future<Output = MeterVerdict> + Send;
    fn after(&self, req: &ChatRequest, usage: Option<Usage>) -> impl std::future::Future<Output = ()> + Send;
    /// The same two steps for generating `count` images with `model`.
    fn before_media(&self, model: &str, count: u32) -> impl std::future::Future<Output = MeterVerdict> + Send;
    fn after_media(&self, model: &str, count: u32) -> impl std::future::Future<Output = ()> + Send;
}

pub struct NoMeter;
impl Meter for NoMeter {
    async fn before_media(&self, _: &str, _: u32) -> MeterVerdict {
        MeterVerdict::Allow
    }
    async fn after_media(&self, _: &str, _: u32) {}
    async fn before(&self, _: &ChatRequest) -> MeterVerdict {
        MeterVerdict::Allow
    }
    async fn after(&self, _: &ChatRequest, _: Option<Usage>) {}
}

/// How a permission check ended.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// The rules allowed it without asking.
    Allowed,
    /// The user was asked and said yes.
    AllowedByUser,
    /// The user was asked and said no.
    Declined,
    /// The rules forbid it (for example `sudo` while it is off).
    Denied(String),
}

/// What a front end sees during a turn.
#[derive(Clone, Debug)]
pub enum TurnEvent {
    /// Streamed pieces from the model: text, thinking, usage, finish.
    Model(Event),
    /// A tool is about to be considered (before any permission prompt).
    ToolStart { id: String, name: String, arguments: String },
    /// The permission rules decided about a call. `action` describes what it wanted to do.
    Decision { id: String, tool: String, action: String, outcome: Outcome },
    /// The text that goes back to the model for that call, and how to undo its effect if it has one.
    ToolResult { id: String, text: String, undo: Option<Undo> },
    /// The diff of a file write that runs without a question (a trusted chat), shown for reading
    /// only. Sent when `permissions.show_diff` allows it. Never logged.
    Diff { id: String, path: String, diff: String },
}

/// Called before every model call. Returns `false` to stop the turn. A worker's pause waits inside it.
pub type Gate = Arc<dyn Fn() -> BoxFut<'static, bool> + Send + Sync>;

pub struct TurnOptions {
    /// Most model calls in one turn, so a confused model cannot loop forever. Default 20.
    pub max_steps: u32,
    /// Name shown in permission prompts.
    pub worker: String,
    pub gate: Option<Gate>,
}

impl Default for TurnOptions {
    fn default() -> Self {
        Self { max_steps: 20, worker: "assistant".into(), gate: None }
    }
}

#[derive(Debug)]
pub enum TurnEnd {
    /// The model gave a final reply (it may be empty if `finish` is `Length`; check it).
    Done(Response),
    /// `max_steps` model calls were used and the model was still asking for tools.
    StepLimit,
    /// The spending rules stopped the turn (a cap was reached, or the user declined the cost).
    Refused(String),
    /// The gate said stop (the user stopped the worker).
    Stopped,
}

/// Run one user turn: call the model, run the tools it asks for under the permission rules, give the
/// results back, and repeat until it answers without tools. `req.messages` ends up holding the whole
/// exchange, ready for the next turn.
// Every argument is a different collaborator (model, meter, tools, rules, asker); a bundle struct would only rename them.
#[allow(clippy::too_many_arguments)]
pub async fn run_turn<M: Model, A: Asker, T: Meter>(
    model: &M,
    meter: &T,
    req: &mut ChatRequest,
    tools: &ToolSet,
    perms: &Permissions,
    asker: &A,
    opts: &TurnOptions,
    mut on_event: impl FnMut(TurnEvent) + Send,
) -> Result<TurnEnd, Error> {
    req.tools = tools.defs();
    for _ in 0..opts.max_steps.max(1) {
        if let Some(g) = &opts.gate {
            if !g().await {
                return Ok(TurnEnd::Stopped);
            }
        }
        match meter.before(req).await {
            MeterVerdict::Allow => {}
            MeterVerdict::Refuse(why) => return Ok(TurnEnd::Refused(why)),
            MeterVerdict::Ask(what) => {
                let q = AskRequest { tool: "model_call", what, reason: "this model call costs money".into(), worker: &opts.worker, detail: String::new() };
                if asker.ask(&q).await == Answer::No {
                    return Ok(TurnEnd::Refused("You declined the cost of this model call.".into()));
                }
            }
        }
        let resp = model.complete(req, &mut |e| on_event(TurnEvent::Model(e))).await?;
        meter.after(req, resp.usage).await;
        if resp.tool_calls.is_empty() {
            req.messages.push(Message::assistant(resp.text.clone(), vec![]));
            return Ok(TurnEnd::Done(resp));
        }
        let calls = resp.tool_calls.clone();
        let cut_off = resp.finish == Some(FinishReason::Length);
        req.messages.push(resp.into_message());
        for call in calls {
            on_event(TurnEvent::ToolStart { id: call.id.clone(), name: crate::canonical_name(&call.name).to_string(), arguments: call.arguments.clone() });
            let result = if call.repaired || cut_off {
                // Arguments that were cut short are a guess. Never run a guessed command.
                ("This tool call was cut off before it was complete, so it was not run. Try again with a shorter call.".to_string(), None)
            } else {
                execute(model, meter, &call.id, &call.name, call.parsed(), tools, perms, asker, opts, &mut on_event).await
            };
            on_event(TurnEvent::ToolResult { id: call.id.clone(), text: result.0.clone(), undo: result.1 });
            let result = result.0;
            req.messages.push(Message::tool_result(&call.id, result));
        }
    }
    Ok(TurnEnd::StepLimit)
}

/// The boxed form of `execute`, so a codemode script's calls can run through it again.
#[allow(clippy::too_many_arguments)]
fn execute_boxed<'a, M: Model, T: Meter, A: Asker>(
    model: &'a M,
    meter: &'a T,
    id: &'a str,
    name: &'a str,
    args: Result<Value, serde_json::Error>,
    tools: &'a ToolSet,
    perms: &'a Permissions,
    asker: &'a A,
    opts: &'a TurnOptions,
    emit: &'a mut (dyn FnMut(TurnEvent) + Send),
) -> std::pin::Pin<Box<dyn std::future::Future<Output = (String, Option<Undo>)> + Send + 'a>> {
    Box::pin(execute(model, meter, id, name, args, tools, perms, asker, opts, emit))
}

/// A codemode script: run it on a thread of its own and answer each tool call it makes through `execute`, so
/// permissions, hooks, events and undo records are the same as for a direct call.
#[allow(clippy::too_many_arguments)]
async fn codemode<M: Model, T: Meter, A: Asker>(
    model: &M,
    meter: &T,
    id: &str,
    args: &Value,
    tools: &ToolSet,
    perms: &Permissions,
    asker: &A,
    opts: &TurnOptions,
    emit: &mut (dyn FnMut(TurnEvent) + Send),
) -> String {
    let script = args["script"].as_str().unwrap_or("").to_string();
    let list: Vec<(String, String)> = tools.defs().into_iter().filter(|d| d.name != "codemode").map(|d| (d.name, d.description)).collect();
    let kv = tools.codemode_kv.clone();
    type Ask = (String, Value, std::sync::mpsc::Sender<Result<String, String>>);
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Ask>();
    let mut job = tokio::task::spawn_blocking(move || {
        let call: crate::codemode::CallTool = Box::new(move |name: &str, args: Value| {
            let (rtx, rrx) = std::sync::mpsc::channel();
            if tx.send((name.to_string(), args, rtx)).is_err() {
                return Err("the turn was stopped".into());
            }
            rrx.recv().unwrap_or_else(|_| Err("the turn was stopped".into()))
        });
        crate::codemode::run(&script, &list, kv, call)
    });
    let mut n = 0;
    loop {
        tokio::select! {
            ask = rx.recv() => {
                let Some((name, args, reply)) = ask else { continue };
                if name == "codemode" {
                    let _ = reply.send(Err("a script cannot start another script".into()));
                    continue;
                }
                // Listing the models changes nothing and leaves no record
                if name == crate::codemode::MODELS_CALL {
                    let r = model.models().await.map(|m| serde_json::to_string(&m).unwrap_or_default()).map_err(|e| e.to_string());
                    let _ = reply.send(r);
                    continue;
                }
                n += 1;
                let sub = format!("{id}.{n}");
                emit(TurnEvent::ToolStart { id: sub.clone(), name: crate::canonical_name(&name).to_string(), arguments: args.to_string() });
                let (text, undo) = execute_boxed(model, meter, &sub, &name, Ok(args), tools, perms, asker, opts, &mut *emit).await;
                emit(TurnEvent::ToolResult { id: sub, text: text.clone(), undo });
                let failed = ["Error:", "Denied:", "The user said no", "Blocked by the hook"].iter().any(|p| text.starts_with(p));
                let _ = reply.send(if failed { Err(text) } else { Ok(text) });
            }
            done = &mut job => {
                return match done {
                    Ok(r) => r.text,
                    Err(e) => format!("Script failed\nScript error: {e}"),
                };
            }
        }
    }
}

/// May this sandboxed command use the network? Returns the answer and how it was reached
/// (`tools.net`). A bypassed session answers yes without asking; the sandbox itself stays on.
#[allow(clippy::too_many_arguments)]
async fn sandbox_net<M: Model, T: Meter, A: Asker>(model: &M, meter: &T, cmd: &str, tools: &ToolSet, perms: &Permissions, asker: &A, opts: &TurnOptions) -> (bool, &'static str) {
    use crate::NetMode;
    let net = &tools.net;
    // What the user allowed never asks, in any mode: a command whose every site is on the list gets the network.
    if net.mode != NetMode::On && net.allows(cmd) {
        return (true, "allowlist");
    }
    match net.mode {
        NetMode::On => return (true, "on"),
        NetMode::Allowlist => return (net.allows(cmd), "allowlist"),
        NetMode::Auto => {
            if let Some(v) = classify_net(model, meter, &net.model, cmd).await {
                return (v, "auto");
            }
        }
        NetMode::Ask => {}
    }
    if perms.effective_mode() == crate::Mode::Bypass {
        return (true, "bypass");
    }
    let hosts = crate::hosts_in(cmd);
    let detail = if hosts.is_empty() { String::new() } else { format!("Hosts named: {}", hosts.join(", ")) };
    let req = AskRequest { tool: "sandbox_network", what: format!("Network access for: {cmd}"), reason: "The command runs in the sandbox. Allow it to use the network this time?".into(), worker: &opts.worker, detail };
    (matches!(asker.ask(&req).await, Answer::Yes), "ask")
}

/// `auto`: a small model answers YES or NO. None when there is no model, it fails, or the answer is unclear.
async fn classify_net<M: Model, T: Meter>(model: &M, meter: &T, name: &str, cmd: &str) -> Option<bool> {
    if name.is_empty() {
        return None;
    }
    let ask = "A shell command will run in a sandbox. Does it need network access to do its job (downloads, package installs, git fetch or clone, API calls)? Answer only YES or NO.";
    let mut req = ChatRequest { model: name.to_string(), messages: vec![Message::user(format!("{ask}\n\nCommand: {}", cmd.chars().take(2000).collect::<String>()))], max_tokens: Some(8), ..Default::default() };
    req.sampling.extra.insert("chat_template_kwargs".into(), serde_json::json!({"enable_thinking": false}));
    if !matches!(meter.before(&req).await, MeterVerdict::Allow) {
        return None;
    }
    let resp = tokio::time::timeout(std::time::Duration::from_secs(30), model.complete(&req, &mut |_| {})).await.ok()?.ok()?;
    meter.after(&req, resp.usage).await;
    let t = resp.text.trim().to_ascii_uppercase();
    match (t.starts_with("YES"), t.starts_with("NO")) {
        (true, false) => Some(true),
        (false, true) => Some(false),
        _ => None,
    }
}

async fn execute<M: Model, T: Meter, A: Asker>(
    model: &M,
    meter: &T,
    id: &str,
    name: &str,
    args: Result<Value, serde_json::Error>,
    tools: &ToolSet,
    perms: &Permissions,
    asker: &A,
    opts: &TurnOptions,
    emit: &mut (dyn FnMut(TurnEvent) + Send),
) -> (String, Option<Undo>) {
    let fail = |m: String| (m, None);
    let name = crate::canonical_name(name); // old names from saved chats still work
    if name == "codemode" && tools.get(name).is_some() {
        return match args {
            Ok(a) => (codemode(model, meter, id, &a, tools, perms, asker, opts, emit).await, None),
            Err(e) => fail(format!("Error: the arguments are not valid JSON ({e}).")),
        };
    }
    let builtin = tools.get(name);
    let extra = if builtin.is_none() { tools.extra(name) } else { None };
    if builtin.is_none() && extra.is_none() {
        return fail(format!("Error: the tool `{name}` is not available."));
    }
    let args = match args {
        Ok(v) => v,
        Err(e) => return fail(format!("Error: the arguments are not valid JSON ({e}).")),
    };
    // Sensitive notes and rules never leave the machine: what a web, image or MCP tool is about to send
    // is checked before the user is even asked.
    if name.starts_with("mcp__") || matches!(name, "web_fetch" | "web_search" | "generate_image") {
        if let Some(g) = &tools.guard.0 {
            if let Some(what) = g(&args.to_string()) {
                return fail(format!("Error: not sent. It repeats something marked sensitive (\"{what}...\"), and sensitive notes and rules do not leave this machine."));
            }
        }
    }
    let action = match (builtin, extra) {
        (Some(tool), _) => match tool.action(&args, perms.root()) {
            Ok(a) => a,
            Err(e) => return fail(format!("Error: {e}")),
        },
        (None, Some(x)) => Action::Extra { name: name.to_string(), what: x.describe(&args), risk: x.risk() },
        (None, None) => unreachable!(),
    };
    let mut decision = |outcome: Outcome| emit(TurnEvent::Decision { id: id.into(), tool: name.into(), action: format!("{action:?}"), outcome });
    match perms.decide(&action) {
        Decision::Deny(why) => {
            decision(Outcome::Denied(why.clone()));
            return fail(format!("Denied: {why}"));
        }
        Decision::Ask { reason, session_grant } => {
            let detail = match &action {
                Action::Write(path) if perms.show_diff().shows(perms.writes_trusted()) => write_diff(path, name, &args),
                _ => String::new(),
            };
            let req = AskRequest { tool: name, what: format!("{action:?}"), reason, worker: &opts.worker, detail };
            match asker.ask(&req).await {
                Answer::No => {
                    decision(Outcome::Declined);
                    return fail("The user said no. Do not retry the same thing; ask what to do instead.".into());
                }
                Answer::Yes => {
                    if session_grant {
                        perms.grant_session_writes();
                    }
                    decision(Outcome::AllowedByUser);
                }
            }
        }
        Decision::Allow => {
            decision(Outcome::Allowed);
            if let Action::Write(path) = &action {
                if perms.show_diff().shows(perms.writes_trusted()) {
                    emit(TurnEvent::Diff { id: id.into(), path: path.display().to_string(), diff: write_diff(path, name, &args) });
                }
            }
        }
    }
    // The user's own hooks, for calls the rules let through
    if let Some(h) = &tools.hooks {
        if let Err(why) = h.before(name, &args, perms.root()).await {
            return fail(why);
        }
    }
    let (text, undo) = if let Some(x) = extra {
        match x.run(args.clone()).await {
            Ok(text) => (text, None),
            Err(e) => fail(format!("Error: {e}")),
        }
    } else {
        let tool = builtin.expect("checked above");
        if tool == Tool::GenerateImage {
            generate_image(model, meter, asker, opts, tools, perms.root(), &args).await
        } else {
            let mut args = args.clone();
            if tool == Tool::RunCommand {
                if let Some(o) = args.as_object_mut() {
                    o.remove(crate::NET_ARG); // Only the runner sets it
                }
                if tools.sandboxed() {
                    let cmd = args["command"].as_str().unwrap_or("").to_string();
                    let (net, how) = sandbox_net(model, meter, &cmd, tools, perms, asker, opts).await;
                    emit(TurnEvent::Decision { id: id.into(), tool: "sandbox_network".into(), action: format!("{how}: {cmd}"), outcome: match (net, how) {
                        (true, "ask") => Outcome::AllowedByUser,
                        (true, _) => Outcome::Allowed,
                        (false, "ask") => Outcome::Declined,
                        (false, _) => Outcome::Denied(format!("network off ({how})")),
                    } });
                    args[crate::NET_ARG] = Value::Bool(net);
                }
            }
            match tool.run(&args, perms.root(), tools).await {
                Ok(out) => (out.text, out.undo),
                Err(e) => fail(format!("Error: {e}")),
            }
        }
    };
    match &tools.hooks {
        Some(h) => (h.after(name, &args, perms.root(), text).await, undo),
        None => (text, undo),
    }
}

fn write_diff(path: &std::path::Path, tool: &str, args: &Value) -> String {
    let before = std::fs::read_to_string(path).ok();
    if tool == "edit" {
        let Some(before) = before.as_deref() else {
            return "Cannot preview this edit because the file could not be read.".into();
        };
        let (Some(old), Some(new)) = (args["old_string"].as_str(), args["new_string"].as_str()) else {
            return "Cannot preview this edit because its old or new text is missing.".into();
        };
        return match crate::edit_text(before, old, new, args["replace_all"].as_bool().unwrap_or(false)) {
            Ok(after) => crate::diff::diff(Some(before), &after),
            Err(e) => format!("Cannot preview this edit: {e}"),
        };
    }
    crate::diff::diff(before.as_deref(), args["content"].as_str().unwrap_or(""))
}

/// Image generation needs the model endpoint, the spending rules and a place to save the file, so it
/// lives here and not in `Tool::run`.
async fn generate_image<M: Model, T: Meter, A: Asker>(model: &M, meter: &T, asker: &A, opts: &TurnOptions, tools: &ToolSet, root: &std::path::Path, args: &Value) -> (String, Option<Undo>) {
    let fail = |m: String| (m, None);
    let Some(image_model) = tools.image_model() else { return fail("Error: image generation is not set up.".into()) };
    let prompt = args["prompt"].as_str().unwrap_or("");
    match meter.before_media(image_model, 1).await {
        MeterVerdict::Allow => {}
        MeterVerdict::Refuse(why) => return fail(format!("Denied: {why}")),
        MeterVerdict::Ask(what) => {
            let q = AskRequest { tool: "image_cost", what, reason: "this image costs money".into(), worker: &opts.worker, detail: String::new() };
            if asker.ask(&q).await == Answer::No {
                return fail("The user declined the cost of the image.".into());
            }
        }
    }
    let imgs = match model.generate_images(image_model, prompt, args["size"].as_str(), 1).await {
        Ok(i) => i,
        Err(e) => return fail(format!("Error: {}", e.explain())),
    };
    meter.after_media(image_model, imgs.len() as u32).await;
    let img = &imgs[0];
    let stem = crate::tools::image_stem(prompt);
    let dir = root.join("generated");
    if let Err(e) = tokio::fs::create_dir_all(&dir).await {
        return fail(format!("Error: {e}"));
    }
    let mut n = 0;
    let path = loop {
        let name = if n == 0 { format!("{stem}.{}", img.ext) } else { format!("{stem}-{n}.{}", img.ext) };
        let p = dir.join(name);
        if !p.exists() {
            break p;
        }
        n += 1;
    };
    if let Err(e) = tokio::fs::write(&path, &img.bytes).await {
        return fail(format!("Error: {e}"));
    }
    (format!("saved the image to {}", path.strip_prefix(root).unwrap_or(&path).display()), Some(Undo { path, previous: None }))
}
