//! Code intelligence: a small client for language servers (LSP), behind one tool, `code_lookup`. It finds where
//! a name is defined, where it is used, what it is, the errors in a file, and the names in a file.
//!
//! A server is started the first time a file of its language is asked about, kept for the chat, and stopped
//! with it. Only servers found on this machine are used; nothing is installed.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin};
use tokio::sync::{oneshot, Mutex};

/// Known servers: file endings, then the program and its arguments. The first one installed for an ending wins.
const SERVERS: &[(&[&str], &str, &[&str])] = &[
    (&["rs"], "rust-analyzer", &[]),
    (&["py"], "pyright-langserver", &["--stdio"]),
    (&["py"], "pylsp", &[]),
    (&["ts", "tsx", "js", "jsx", "mjs"], "typescript-language-server", &["--stdio"]),
    (&["go"], "gopls", &[]),
    (&["c", "h", "cc", "cpp", "hpp", "cxx"], "clangd", &[]),
];

/// Longest wait for one answer. A server indexing a big project for the first time can be slow.
const ANSWER_WAIT: Duration = Duration::from_secs(30);

fn on_path(program: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|p| std::env::split_paths(&p).map(|d| d.join(program)).find(|f| f.is_file()))
}

/// Is any known server installed? The tool is only offered when one is, so the model is not told about a
/// tool that can never work.
pub fn any_server() -> bool {
    SERVERS.iter().any(|(_, prog, _)| on_path(prog).is_some())
}

fn server_for(path: &Path) -> Option<(PathBuf, &'static [&'static str], &'static str)> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    SERVERS.iter().filter(|(exts, _, _)| exts.contains(&ext.as_str())).find_map(|(_, prog, args)| on_path(prog).map(|p| (p, *args, *prog)))
}

fn uri(p: &Path) -> String {
    format!("file://{}", p.display())
}

fn path_of(uri: &str) -> String {
    uri.strip_prefix("file://").unwrap_or(uri).to_string()
}

type Waiters = Arc<std::sync::Mutex<HashMap<i64, oneshot::Sender<Value>>>>;

struct Server {
    stdin: Mutex<ChildStdin>,
    next: std::sync::atomic::AtomicI64,
    waiting: Waiters,
    /// The newest errors and warnings per file, as the server sends them.
    diagnostics: Arc<std::sync::Mutex<HashMap<String, Vec<Value>>>>,
    opened: Mutex<HashMap<PathBuf, i64>>,
    _child: Child,
}

impl Server {
    async fn start(program: &Path, args: &[&str], root: &Path) -> Result<Server, String> {
        let mut child = tokio::process::Command::new(program)
            .args(args)
            .current_dir(root)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| format!("cannot start {}: {e}", program.display()))?;
        let stdin = child.stdin.take().expect("piped");
        let stdout = child.stdout.take().expect("piped");
        let waiting: Waiters = Default::default();
        let diagnostics: Arc<std::sync::Mutex<HashMap<String, Vec<Value>>>> = Default::default();
        let (w, d) = (waiting.clone(), diagnostics.clone());
        tokio::spawn(async move {
            let mut r = BufReader::new(stdout);
            while let Some(msg) = read_message(&mut r).await {
                if let Some(id) = msg["id"].as_i64().filter(|_| msg.get("method").is_none()) {
                    if let Some(tx) = w.lock().unwrap().remove(&id) {
                        let _ = tx.send(msg);
                    }
                } else if msg["method"] == "textDocument/publishDiagnostics" {
                    let p = path_of(msg["params"]["uri"].as_str().unwrap_or(""));
                    d.lock().unwrap().insert(p, msg["params"]["diagnostics"].as_array().cloned().unwrap_or_default());
                }
            }
        });
        let s = Server { stdin: Mutex::new(stdin), next: 1.into(), waiting, diagnostics, opened: Default::default(), _child: child };
        let caps = json!({"textDocument": {"hover": {"contentFormat": ["plaintext"]}, "documentSymbol": {"hierarchicalDocumentSymbolSupport": true}, "publishDiagnostics": {}}});
        s.request("initialize", json!({"processId": std::process::id(), "rootUri": uri(root), "capabilities": caps, "workspaceFolders": [{"uri": uri(root), "name": "project"}]})).await?;
        s.notify("initialized", json!({})).await?;
        Ok(s)
    }

    async fn send(&self, msg: Value) -> Result<(), String> {
        let body = msg.to_string();
        let mut w = self.stdin.lock().await;
        w.write_all(format!("Content-Length: {}\r\n\r\n{body}", body.len()).as_bytes()).await.map_err(|e| format!("the language server stopped: {e}"))?;
        w.flush().await.map_err(|e| e.to_string())
    }

    async fn notify(&self, method: &str, params: Value) -> Result<(), String> {
        self.send(json!({"jsonrpc": "2.0", "method": method, "params": params})).await
    }

    async fn request(&self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();
        self.waiting.lock().unwrap().insert(id, tx);
        self.send(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params})).await?;
        match tokio::time::timeout(ANSWER_WAIT, rx).await {
            Ok(Ok(v)) if v.get("error").is_some() => Err(format!("the language server said: {}", v["error"]["message"].as_str().unwrap_or("error"))),
            Ok(Ok(v)) => Ok(v["result"].clone()),
            Ok(Err(_)) => Err("the language server stopped".into()),
            Err(_) => {
                self.waiting.lock().unwrap().remove(&id);
                Err(format!("the language server gave no answer in {} s (it may still be reading the project; try again)", ANSWER_WAIT.as_secs()))
            }
        }
    }

    /// Tell the server about the file as it is on disk now: open it the first time, send the new text after.
    async fn sync(&self, file: &Path, language: &str) -> Result<(), String> {
        let text = tokio::fs::read_to_string(file).await.map_err(|e| format!("cannot read {}: {e}", file.display()))?;
        let mut opened = self.opened.lock().await;
        match opened.get_mut(file) {
            None => {
                opened.insert(file.to_path_buf(), 1);
                self.notify("textDocument/didOpen", json!({"textDocument": {"uri": uri(file), "languageId": language, "version": 1, "text": text}})).await
            }
            Some(v) => {
                *v += 1;
                self.notify("textDocument/didChange", json!({"textDocument": {"uri": uri(file), "version": *v}, "contentChanges": [{"text": text}]})).await
            }
        }
    }
}

async fn read_message<R: tokio::io::AsyncBufRead + Unpin>(r: &mut R) -> Option<Value> {
    let mut len = None;
    loop {
        let mut line = String::new();
        if r.read_line(&mut line).await.ok()? == 0 {
            return None;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some(v) = line.strip_prefix("Content-Length:") {
            len = v.trim().parse::<usize>().ok();
        }
    }
    let mut buf = vec![0; len?];
    r.read_exact(&mut buf).await.ok()?;
    serde_json::from_slice(&buf).ok()
}

/// The servers of one chat, one per program.
#[derive(Default)]
pub struct Lsp {
    servers: Mutex<HashMap<&'static str, Arc<Server>>>,
}

impl std::fmt::Debug for Lsp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Lsp")
    }
}

impl Lsp {
    async fn server(&self, file: &Path, root: &Path) -> Result<Arc<Server>, String> {
        let (program, args, name) = server_for(file).ok_or_else(|| format!("no language server for {} files is installed on this machine", file.extension().and_then(|e| e.to_str()).unwrap_or("these")))?;
        let mut all = self.servers.lock().await;
        if let Some(s) = all.get(name) {
            return Ok(s.clone());
        }
        let s = Arc::new(Server::start(&program, args, root).await?);
        all.insert(name, s.clone());
        Ok(s)
    }

    /// Answer one `code_lookup` call. `file` is inside the project; `line` and `column` count from 1.
    pub async fn lookup(&self, root: &Path, what: &str, file: &Path, line: u64, column: u64) -> Result<String, String> {
        let s = self.server(file, root).await?;
        let language = match file.extension().and_then(|e| e.to_str()).unwrap_or("") {
            "rs" => "rust",
            "py" => "python",
            "ts" | "tsx" => "typescript",
            "js" | "jsx" | "mjs" => "javascript",
            "go" => "go",
            "c" | "h" => "c",
            _ => "cpp",
        };
        s.sync(file, language).await?;
        let at = json!({"textDocument": {"uri": uri(file)}, "position": {"line": line.saturating_sub(1), "character": column.saturating_sub(1)}});
        let rel = |p: &str| p.strip_prefix(&format!("{}/", root.display())).unwrap_or(p).to_string();
        let place = |l: &Value| {
            let u = l["uri"].as_str().or(l["targetUri"].as_str()).unwrap_or("");
            let r = if l.get("range").is_some() { &l["range"] } else { &l["targetSelectionRange"] };
            format!("{}:{}:{}", rel(&path_of(u)), r["start"]["line"].as_u64().unwrap_or(0) + 1, r["start"]["character"].as_u64().unwrap_or(0) + 1)
        };
        let places = |v: Value| -> String {
            let list = match v {
                Value::Array(a) => a,
                Value::Null => vec![],
                one => vec![one],
            };
            if list.is_empty() {
                return "Nothing found there.".into();
            }
            let n = list.len();
            let mut out: Vec<String> = list.iter().take(50).map(place).collect();
            if n > 50 {
                out.push(format!("… and {} more", n - 50));
            }
            out.join("\n")
        };
        match what {
            "definition" => Ok(places(s.request("textDocument/definition", at).await?)),
            "references" => {
                let mut p = at;
                p["context"] = json!({"includeDeclaration": true});
                Ok(places(s.request("textDocument/references", p).await?))
            }
            "hover" => {
                let v = s.request("textDocument/hover", at).await?;
                let c = &v["contents"];
                let text = match c {
                    Value::String(t) => t.clone(),
                    Value::Array(a) => a.iter().map(|x| x.as_str().map(String::from).unwrap_or_else(|| x["value"].as_str().unwrap_or("").to_string())).collect::<Vec<_>>().join("\n"),
                    _ => c["value"].as_str().unwrap_or("").to_string(),
                };
                Ok(if text.trim().is_empty() { "Nothing known about that spot.".into() } else { text })
            }
            "symbols" => {
                let v = s.request("textDocument/documentSymbol", json!({"textDocument": {"uri": uri(file)}})).await?;
                let mut out = vec![];
                fn walk(v: &Value, depth: usize, out: &mut Vec<String>) {
                    for x in v.as_array().into_iter().flatten() {
                        let r = if x.get("selectionRange").is_some() { &x["selectionRange"] } else { &x["location"]["range"] };
                        out.push(format!("{}{} (line {})", "  ".repeat(depth), x["name"].as_str().unwrap_or("?"), r["start"]["line"].as_u64().unwrap_or(0) + 1));
                        walk(&x["children"], depth + 1, out);
                    }
                }
                walk(&v, 0, &mut out);
                Ok(if out.is_empty() { "No names found in this file.".into() } else { out.join("\n") })
            }
            "diagnostics" => {
                // servers send these on their own after a file is opened or changed; give them a moment
                let key = file.display().to_string();
                for _ in 0..30 {
                    if s.diagnostics.lock().unwrap().contains_key(&key) {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
                let list = s.diagnostics.lock().unwrap().get(&key).cloned();
                let Some(list) = list else { return Ok("The language server has not reported on this file yet.".into()) };
                if list.is_empty() {
                    return Ok("No errors or warnings.".into());
                }
                Ok(list
                    .iter()
                    .take(50)
                    .map(|d| {
                        let sev = match d["severity"].as_u64() {
                            Some(1) => "error",
                            Some(2) => "warning",
                            Some(3) => "note",
                            _ => "hint",
                        };
                        format!("{}:{} {sev}: {}", d["range"]["start"]["line"].as_u64().unwrap_or(0) + 1, d["range"]["start"]["character"].as_u64().unwrap_or(0) + 1, d["message"].as_str().unwrap_or(""))
                    })
                    .collect::<Vec<_>>()
                    .join("\n"))
            }
            other => Err(format!("`what` must be definition, references, hover, symbols or diagnostics, not {other}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn messages_are_framed_by_their_length() {
        let raw = b"Content-Length: 13\r\n\r\n{\"id\":1,\"a\":2}Content-Length: 2\r\n\r\n{}";
        let mut r = BufReader::new(&raw[..]);
        // the first message claims 13 bytes; the JSON is 14, so it is cut and does not parse
        assert_eq!(read_message(&mut r).await, None);
        let raw = b"Content-Length: 14\r\n\r\n{\"id\":1,\"a\":2}Content-Length: 2\r\n\r\n{}";
        let mut r = BufReader::new(&raw[..]);
        assert_eq!(read_message(&mut r).await, Some(json!({"id": 1, "a": 2})));
        assert_eq!(read_message(&mut r).await, Some(json!({})));
        assert_eq!(read_message(&mut r).await, None);
    }

    #[tokio::test]
    async fn clangd_finds_a_definition_and_an_error() {
        if on_path("clangd").is_none() {
            return;
        }
        let d = tempfile::tempdir().unwrap();
        let root = d.path().canonicalize().unwrap();
        let f = root.join("a.c");
        // the error sits on a line of its own: clang drops a whole expression that holds an error
        std::fs::write(&f, "int twice(int x) { return 2 * x; }\nint main(void) { return twice(3); }\nint broken(void) { return nope; }\n").unwrap();
        let lsp = Lsp::default();
        let def = lsp.lookup(&root, "definition", &f, 2, 25).await.unwrap();
        assert_eq!(def, "a.c:1:5");
        let diag = lsp.lookup(&root, "diagnostics", &f, 1, 1).await.unwrap();
        assert!(diag.contains("error") && diag.contains("nope"), "{diag}");
        let syms = lsp.lookup(&root, "symbols", &f, 1, 1).await.unwrap();
        assert!(syms.contains("twice (line 1)") && syms.contains("main (line 2)"), "{syms}");
        let hover = lsp.lookup(&root, "hover", &f, 2, 25).await.unwrap();
        assert!(hover.contains("twice"), "{hover}");
    }
}
