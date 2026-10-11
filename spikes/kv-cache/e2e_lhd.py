#!/usr/bin/env python3
"""End to end with the real lhd and the real Qwen model through llama-swap: does saving the KV cache make an old chat
come back fast? Two chats are used on purpose, because the router's own slotkeeper already keeps the LAST conversation
warm; the saved cache matters for the chat before that. Usage: e2e_lhd.py on|off"""
import json, os, subprocess, sys, tempfile, time, urllib.request
variant = sys.argv[1]
root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
model = "Qwen3.6-35B-A3B-NanoPlus-balanced:instruct"
slots = os.path.expanduser("~/AI/cache/slots")
tmp = tempfile.mkdtemp(prefix="lh-kv-")
os.makedirs(tmp + "/config/lastharness"); os.makedirs(tmp + "/run")
open(tmp + "/config/lastharness/config.toml", "w").write(f'[provider]\nurl = "http://localhost:9292/v1"\n[kvcache]\nenabled = {"true" if variant == "on" else "false"}\ndir = "{slots}"\nmax_gb = 5\n')
env = dict(os.environ, XDG_DATA_HOME=tmp + "/data", XDG_CONFIG_HOME=tmp + "/config", XDG_RUNTIME_DIR=tmp + "/run")
port = 7853; procs = []
def start():
    if os.path.exists(tmp + "/token"): os.remove(tmp + "/token")
    p = subprocess.Popen([f"{root}/target/release/lhd", "--listen", f"127.0.0.1:{port}", "--token-file", tmp + "/token", "--idle-after", "0"], env=env, stdout=subprocess.DEVNULL, stderr=open(tmp + "/lhd.log", "a"))
    while not os.path.exists(tmp + "/token"): time.sleep(0.1)
    time.sleep(0.5); return p
def call(method, path, body=None, timeout=900):
    tok = open(tmp + "/token").read().strip()
    req = urllib.request.Request(f"http://127.0.0.1:{port}{path}", data=None if body is None else json.dumps(body).encode(), method=method, headers={"Authorization": f"Bearer {tok}", "Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=timeout) as r: raw = r.read()
    try: return json.loads(raw)
    except Exception: return raw
def turn(sid, text):
    tok = open(tmp + "/token").read().strip()
    req = urllib.request.Request(f"http://127.0.0.1:{port}/v1/sessions/{sid}/turns", data=json.dumps({"text": text}).encode(), headers={"Authorization": f"Bearer {tok}", "Content-Type": "application/json"})
    t0 = time.time(); first = None; usage = {}; answer = ""
    with urllib.request.urlopen(req, timeout=900) as r:
        ev = ""
        for line in r:
            line = line.decode().rstrip("\n")
            if line.startswith("event: "): ev = line[7:]
            elif line.startswith("data: "):
                d = json.loads(line[6:])
                if ev == "text":
                    first = first or time.time() - t0; answer += d.get("text", "")
                if ev == "usage": usage = d
                if ev in ("done", "error"): break
    return {"first_token_s": round(first or 0, 1), "total_s": round(time.time() - t0, 1), "usage": usage, "answer": answer.strip()[:60]}
words = "river stone cloud engine garden paper window copper signal market forest ladder orange puzzle harbor quiet violet anchor meadow".split()
big = "\n".join(f"Entry {i}: the {words[i % 19]} near the {words[(i * 7) % 19]} was noted on day {i * 3 % 365} with code {i * 7919 % 100003}." for i in range(140))
out = {"variant": variant}
lhd = start()
try:
    A = call("POST", "/v1/sessions", {"model": model, "mode": "chat"})["id"]
    out["A_first"] = turn(A, big + "\n\nReply with the single word: Noted.")
    B = call("POST", "/v1/sessions", {"model": model, "mode": "chat"})["id"]
    out["B_first"] = turn(B, "Say hello in three words.")
    time.sleep(2)
    out["files_after"] = sorted(f for d in os.listdir(slots) if os.path.isdir(f"{slots}/{d}") for f in os.listdir(f"{slots}/{d}") if f.startswith("lh-"))
    out["log"] = [(e["kind"], e["detail"]) for e in call("GET", "/v1/activity?limit=50")["entries"] if e["kind"].startswith("kv_cache")][-6:] if isinstance(call("GET", "/v1/activity?limit=50"), dict) else []
    lhd.terminate(); lhd.wait()
    urllib.request.urlopen("http://localhost:9292/unload", timeout=60).read()   # a cold server, like after a swap
    time.sleep(3)
    lhd = start()
    r = call("POST", f"/v1/chats/{A}/resume", {"confirm": True, "model": model})
    out["resume"] = {k: r.get(k) for k in ("session", "context_tokens", "live")}
    out["A_after_restart"] = turn(A, "What is the number of the first entry in the long text? Answer with just the number.")
    out["log2"] = [(e["kind"], e["detail"]) for e in call("GET", "/v1/activity?limit=50").get("entries", []) if e["kind"].startswith("kv_cache")][-4:]
    call("DELETE", f"/v1/chats/{A}"); call("DELETE", f"/v1/chats/{B}")
    out["files_after_delete"] = sorted(f for d in os.listdir(slots) if os.path.isdir(f"{slots}/{d}") for f in os.listdir(f"{slots}/{d}") if f.startswith("lh-"))
finally:
    lhd.terminate()
print(json.dumps(out, indent=1)); json.dump(out, open(f"e2e-{variant}.json", "w"), indent=1)
