//! Codemode: the model writes a short JavaScript script that calls the other tools, and only what the script prints
//! goes back to the model. A script can loop over files, filter a long result or combine several calls in one step,
//! without every intermediate result filling the conversation. The idea and the shape follow Pi's codemode.
//!
//! The script runs in QuickJS, inside this process, with no file system, network or timers: it reaches the outside
//! only through `tools.<name>(args)`, and each of those calls goes through the same permission rules, hooks and undo
//! records as a direct tool call (see `runner`). Memory and time are capped.
//!
//! Globals: `tools.<name>(args)` (async; resolves to the tool's text, rejects with its error), `text(value)`,
//! `console.log(...)`, a top-level `return`, and `ALL_TOOLS` (names and descriptions).

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rquickjs::{Context, Function, Runtime};
use serde_json::Value;

/// Default and longest time for one script.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);
pub const MAX_TIMEOUT: Duration = Duration::from_secs(600);
const MEMORY_LIMIT: usize = 64 * 1024 * 1024;
/// Most characters of output given back to the model; the middle of a longer output is cut.
const MAX_OUTPUT: usize = 20_000;

/// A tool call the script made, answered by the caller: `Ok(text)` or `Err(error text)`.
pub type CallTool = Box<dyn FnMut(&str, Value) -> Result<String, String>>;

/// How a script ended, in the words the model gets back.
#[derive(Debug, Clone, PartialEq)]
pub struct ScriptResult {
    pub ok: bool,
    pub text: String,
    pub calls: usize,
}

/// The options line Pi uses: `// @options: {"timeout_ms": 60000}` as the first line.
pub fn timeout_of(script: &str) -> Duration {
    let first = script.lines().next().unwrap_or("");
    first
        .trim()
        .strip_prefix("// @options:")
        .and_then(|j| serde_json::from_str::<Value>(j.trim()).ok())
        .and_then(|v| v["timeout_ms"].as_u64())
        .map(|ms| Duration::from_millis(ms).min(MAX_TIMEOUT))
        .unwrap_or(DEFAULT_TIMEOUT)
}

const PRELUDE: &str = r#"
const __lines = [];
globalThis.text = (v) => __text(typeof v === "string" ? v : JSON.stringify(v, null, 2));
globalThis.console = { log: (...a) => __lines.push(a.map(x => typeof x === "string" ? x : JSON.stringify(x)).join(" ")) };
console.info = console.warn = console.error = console.debug = console.log;
globalThis.tools = {};
for (const t of ALL_TOOLS) {
  tools[t.name.replace(/[^A-Za-z0-9_$]/g, "_")] = async (args) => {
    const r = JSON.parse(__call(t.name, JSON.stringify(args === undefined ? {} : args)));
    if (r.error !== undefined) throw new Error(r.error);
    return r.ok;
  };
}
"#;

/// Run one script. `tools` lists the callable tools as (name, description); `call` answers each call.
/// Blocking: run it on a thread of its own.
pub fn run(script: &str, tools: &[(String, String)], call: CallTool) -> ScriptResult {
    let deadline = Instant::now() + timeout_of(script);
    let out: Arc<Mutex<Vec<String>>> = Arc::default();
    let calls = Arc::new(Mutex::new(0usize));
    let failure: Arc<Mutex<Option<String>>> = Arc::default();
    let fail = |m: String| ScriptResult { ok: false, text: format!("Script failed\nScript error: {m}"), calls: 0 };
    let rt = match Runtime::new() {
        Ok(r) => r,
        Err(e) => return fail(e.to_string()),
    };
    rt.set_memory_limit(MEMORY_LIMIT);
    rt.set_max_stack_size(1024 * 1024);
    rt.set_interrupt_handler(Some(Box::new(move || Instant::now() > deadline)));
    let ctx = match Context::full(&rt) {
        Ok(c) => c,
        Err(e) => return fail(e.to_string()),
    };
    let call = std::rc::Rc::new(std::cell::RefCell::new(call));
    let setup: Result<(), String> = ctx.with(|ctx| {
        let g = ctx.globals();
        let list: Vec<Value> = tools.iter().map(|(n, d)| serde_json::json!({"name": n, "description": d})).collect();
        let all = ctx.json_parse(Value::Array(list).to_string()).map_err(|e| e.to_string())?;
        g.set("ALL_TOOLS", all).map_err(|e| e.to_string())?;
        let o = out.clone();
        g.set("__text", Function::new(ctx.clone(), move |s: String| o.lock().unwrap().push(s)).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        let n = calls.clone();
        let cell = call.clone();
        g.set(
            "__call",
            Function::new(ctx.clone(), move |name: String, args: String| -> String {
                *n.lock().unwrap() += 1;
                let args: Value = serde_json::from_str(&args).unwrap_or(Value::Null);
                let r = (cell.borrow_mut())(&name, args);
                match r {
                    Ok(t) => serde_json::json!({"ok": t}).to_string(),
                    Err(e) => serde_json::json!({"error": e}).to_string(),
                }
            })
            .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let f = failure.clone();
        g.set("__fail", Function::new(ctx.clone(), move |s: String| *f.lock().unwrap() = Some(s)).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        ctx.eval::<(), _>(PRELUDE).map_err(|e| describe(&ctx, e))?;
        let body = format!("(async function() {{\n{script}\n}})().then(v => {{ if (v !== undefined) text(v); }}, e => __fail(String(e && e.message !== undefined ? e.message : e)));");
        ctx.eval::<(), _>(body).map_err(|e| describe(&ctx, e))?;
        Ok(())
    });
    if let Err(e) = setup {
        *failure.lock().unwrap() = Some(e);
    }
    // run the promise jobs (the awaits) until the script is done
    while rt.is_job_pending() {
        if Instant::now() > deadline {
            *failure.lock().unwrap() = Some("the script ran out of time".into());
            break;
        }
        if let Err(e) = rt.execute_pending_job() {
            *failure.lock().unwrap() = Some(format!("{e:?}"));
            break;
        }
    }
    let console: Vec<String> = ctx.with(|ctx| ctx.globals().get::<_, Vec<String>>("__lines").ok()).unwrap_or_default();
    let console = ctx.with(|ctx| ctx.eval::<Vec<String>, _>("__lines").ok()).unwrap_or(console);
    let mut parts = out.lock().unwrap().clone();
    if parts.len() > 1 {
        let n = parts.len();
        parts = parts.into_iter().enumerate().map(|(i, p)| format!("==> text {}/{n} <==\n{p}", i + 1)).collect();
    }
    let mut body = parts.join("\n");
    if !console.is_empty() {
        body.push_str(&format!("\n<console_output>\n{}\n</console_output>", console.join("\n")));
    }
    let failure = failure.lock().unwrap().clone();
    let calls = *calls.lock().unwrap();
    let head = if failure.is_some() { "Script failed" } else { "Script completed" };
    let mut text = format!("{head} ({calls} tool call{})\n{}", if calls == 1 { "" } else { "s" }, body.trim());
    if let Some(e) = &failure {
        text.push_str(&format!("\nScript error: {e}"));
    }
    ScriptResult { ok: failure.is_none(), text: shorten(&text), calls }
}

fn describe(ctx: &rquickjs::Ctx<'_>, e: rquickjs::Error) -> String {
    if matches!(e, rquickjs::Error::Exception) {
        let ex = ctx.catch();
        if let Some(o) = ex.as_object() {
            if let Ok(m) = o.get::<_, String>("message") {
                return m;
            }
        }
        return format!("{ex:?}");
    }
    e.to_string()
}

fn shorten(s: &str) -> String {
    let n = s.chars().count();
    if n <= MAX_OUTPUT {
        return s.to_string();
    }
    let head: String = s.chars().take(MAX_OUTPUT / 2).collect();
    let tail: String = s.chars().skip(n - MAX_OUTPUT / 2).collect();
    format!("{head}\n… ({} characters left out) …\n{tail}", n - MAX_OUTPUT)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tools() -> Vec<(String, String)> {
        vec![("read_file".into(), "Read a file".into()), ("mcp__x-y__z".into(), "odd name".into())]
    }

    #[test]
    fn a_script_calls_tools_and_only_its_output_comes_back() {
        let seen_c = std::rc::Rc::new(std::cell::RefCell::new(Vec::<(String, Value)>::new()));
        let sc = seen_c.clone();
        let call: CallTool = Box::new(move |name: &str, args: Value| -> Result<String, String> {
            sc.borrow_mut().push((name.to_string(), args.clone()));
            match args["path"].as_str() {
                Some("missing") => Err("no such file".into()),
                Some(p) => Ok(format!("contents of {p}\nline two")),
                None => Ok("z".into()),
            }
        });
        let script = r#"
            const a = await tools.read_file({path: "a.txt"});
            let missing = "";
            try { await tools.read_file({path: "missing"}); } catch (e) { missing = e.message; }
            console.log("checked", 2);
            await tools.mcp__x_y__z();
            return { first: a.split("\n")[0], missing };
        "#;
        let r = run(script, &tools(), call);
        assert!(r.ok, "{}", r.text);
        assert_eq!(r.calls, 3);
        assert!(r.text.starts_with("Script completed (3 tool calls)"), "{}", r.text);
        assert!(r.text.contains("\"first\": \"contents of a.txt\"") && r.text.contains("no such file"), "{}", r.text);
        assert!(r.text.contains("<console_output>\nchecked 2\n</console_output>"), "{}", r.text);
        assert_eq!(seen_c.borrow()[2].0, "mcp__x-y__z", "the odd name is called by its real name");
    }

    #[test]
    fn errors_and_runaway_scripts_end_in_words() {
        let call = || -> CallTool { Box::new(|_: &str, _: Value| -> Result<String, String> { Ok(String::new()) }) };
        let r = run("text('before'); throw new Error('boom');", &tools(), call());
        assert!(!r.ok && r.text.contains("before") && r.text.contains("Script error: boom"), "{}", r.text);
        let r = run("// @options: {\"timeout_ms\": 300}\nwhile (true) {}", &tools(), call());
        assert!(!r.ok, "{}", r.text);
        let r = run("this is not javascript", &tools(), call());
        assert!(!r.ok && r.text.contains("Script error"), "{}", r.text);
        assert!(run("return typeof require + typeof fetch + typeof setTimeout", &tools(), call()).text.contains("undefinedundefinedundefined"), "no outside world");
    }

    #[test]
    fn the_options_line_sets_the_time() {
        assert_eq!(timeout_of("// @options: {\"timeout_ms\": 5000}\nx"), Duration::from_secs(5));
        assert_eq!(timeout_of("x"), DEFAULT_TIMEOUT);
        assert_eq!(timeout_of("// @options: {\"timeout_ms\": 99999999}"), MAX_TIMEOUT);
    }
}
