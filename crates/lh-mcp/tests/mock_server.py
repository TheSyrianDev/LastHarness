# A tiny MCP server over stdio for tests: tools "echo" and "boom". Newline-delimited JSON-RPC.
import json, sys

def reply(i, result=None, error=None):
    m = {"jsonrpc": "2.0", "id": i}
    if error is not None: m["error"] = error
    else: m["result"] = result
    print(json.dumps(m), flush=True)

for line in sys.stdin:
    try: m = json.loads(line)
    except Exception: continue
    method, i = m.get("method"), m.get("id")
    if method == "initialize":
        # a server may send a request or notification before the reply; the client must ignore it
        print(json.dumps({"jsonrpc": "2.0", "method": "notifications/message", "params": {"level": "info", "data": "hello"}}), flush=True)
        reply(i, {"protocolVersion": "2025-06-18", "capabilities": {"tools": {}}, "serverInfo": {"name": "mock", "version": "1"}})
    elif method == "tools/list":
        if m.get("params", {}).get("cursor") == "p2":
            reply(i, {"tools": [{"name": "boom", "description": "Always fails.", "inputSchema": {"type": "object"}}]})
        else:
            reply(i, {"tools": [{"name": "echo", "description": "Repeats the text.", "inputSchema": {"type": "object", "properties": {"text": {"type": "string"}}, "required": ["text"]}}], "nextCursor": "p2"})
    elif method == "tools/call":
        p = m["params"]
        if p["name"] == "echo":
            reply(i, {"content": [{"type": "text", "text": "echo: " + p["arguments"].get("text", "")}, {"type": "image", "data": "x", "mimeType": "image/png"}]})
        elif p["name"] == "boom":
            reply(i, {"content": [{"type": "text", "text": "it broke"}], "isError": True})
        else:
            reply(i, error={"code": -32602, "message": "unknown tool"})
    elif i is not None:
        reply(i, error={"code": -32601, "message": "method not found"})
