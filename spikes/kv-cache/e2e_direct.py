#!/usr/bin/env python3
"""End to end on a plain llama-server (no router, no slotkeeper) with the real lhd: does saving the KV cache make an
old chat come back fast? Usage: e2e_direct.py on|off   (small model so it runs in a minute)"""
import json, os, subprocess, sys, tempfile, time, urllib.request, glob
variant = sys.argv[1]
root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
server = glob.glob(os.path.expanduser("~/AI/engines/_prebuilt/mainline-b11541/llama-b11541/llama-server"))[0]
gguf = "/home/omar/AI/models/lfm25-2.6b/LFM2.5-2.6B-Q4_K_M.gguf"
tmp = tempfile.mkdtemp(prefix="lh-kvd-"); slots = tmp + "/slots"; os.makedirs(slots)
os.makedirs(tmp + "/config/lastharness"); os.makedirs(tmp + "/run")
open(tmp + "/config/lastharness/config.toml", "w").write(f'[provider]\nurl = "http://127.0.0.1:8098/v1"\n[kvcache]\nenabled = {"true" if variant == "on" else "false"}\ndir = "{slots}"\nmax_gb = 5\n')
env = dict(os.environ, XDG_DATA_HOME=tmp + "/data", XDG_CONFIG_HOME=tmp + "/config", XDG_RUNTIME_DIR=tmp + "/run")
lport = 7854
def llama():
    p = subprocess.Popen([server, "-m", gguf, "--port", "8098", "--host", "127.0.0.1", "-c", "16384", "-ngl", "99", "--parallel", "1", "--slot-save-path", slots, "--jinja", "--no-warmup"], stdout=subprocess.DEVNULL, stderr=open(tmp + "/llama.log", "w"))
    for _ in range(300):
        try:
            urllib.request.urlopen("http://127.0.0.1:8098/health", timeout=2).read(); return p
        except Exception: time.sleep(1)
    raise SystemExit("llama-server did not start")
def lhd_start():
    if os.path.exists(tmp + "/token"): os.remove(tmp + "/token")
    p = subprocess.Popen([f"{root}/target/release/lhd", "--listen", f"127.0.0.1:{lport}", "--token-file", tmp + "/token", "--idle-after", "0"], env=env, stdout=subprocess.DEVNULL, stderr=open(tmp + "/lhd.log", "a"))
    while not os.path.exists(tmp + "/token"): time.sleep(0.1)
    time.sleep(0.5); return p
def call(method, path, body=None):
    tok = open(tmp + "/token").read().strip()
    req = urllib.request.Request(f"http://127.0.0.1:{lport}{path}", data=None if body is None else json.dumps(body).encode(), method=method, headers={"Authorization": f"Bearer {tok}", "Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=600) as r: raw = r.read()
    try: return json.loads(raw)
    except Exception: return raw
def turn(sid, text):
    tok = open(tmp + "/token").read().strip()
    req = urllib.request.Request(f"http://127.0.0.1:{lport}/v1/sessions/{sid}/turns", data=json.dumps({"text": text}).encode(), headers={"Authorization": f"Bearer {tok}", "Content-Type": "application/json"})
    t0 = time.time(); first = None; usage = {}; ans = ""
    with urllib.request.urlopen(req, timeout=600) as r:
        ev = ""
        for line in r:
            line = line.decode().rstrip("\n")
            if line.startswith("event: "): ev = line[7:]
            elif line.startswith("data: "):
                d = json.loads(line[6:])
                if ev in ("text", "reasoning"): first = first or time.time() - t0
                if ev == "text": ans += d.get("text", "")
                if ev == "usage": usage = d
                if ev in ("done", "error"): break
    return {"first_token_s": round(first or 0, 2), "total_s": round(time.time() - t0, 2), "prompt_tokens": usage.get("prompt_tokens"), "cached_tokens": usage.get("cached_tokens"), "answer": ans.strip()[:50]}
words = "river stone cloud engine garden paper window copper signal market forest ladder orange puzzle harbor quiet violet anchor meadow".split()
big = "\n".join(f"Entry {i}: the {words[i % 19]} near the {words[(i * 7) % 19]} was noted on day {i * 3 % 365} with code {i * 7919 % 100003}." for i in range(330))
out = {"variant": variant}
try: urllib.request.urlopen("http://localhost:9292/unload", timeout=60).read()  # free the GPU for this test
except Exception: pass
time.sleep(3)
lp = llama(); lhd = lhd_start()
try:
    A = call("POST", "/v1/sessions", {"model": "lfm", "mode": "chat", "max_tokens": 200})["id"]
    out["A_first"] = turn(A, big + "\n\nReply with the single word: Noted.")
    B = call("POST", "/v1/sessions", {"model": "lfm", "mode": "chat", "max_tokens": 200})["id"]
    out["B_first"] = turn(B, "Say hello in three words.")
    time.sleep(1)
    out["files"] = sorted(os.listdir(slots))
    mode = [oct(os.stat(f"{slots}/{f}").st_mode & 0o777) for f in os.listdir(slots)]
    out["file_modes"] = mode
    lhd.terminate(); lhd.wait(); lp.terminate(); lp.wait(); time.sleep(2)   # a cold server
    lp = llama(); lhd = lhd_start()
    r = call("POST", f"/v1/chats/{A}/resume", {"confirm": True, "model": "lfm"})
    out["resume"] = {k: r.get(k) for k in ("context_tokens",)}
    out["A_after_restart"] = turn(A, "What is the number of the first entry in the long text? Answer with just the number.")
    out["log"] = [(e["kind"], e["detail"]) for e in call("GET", "/v1/activity?limit=60").get("entries", []) if e["kind"].startswith("kv_cache")][:6]
    call("DELETE", f"/v1/chats/{A}"); call("DELETE", f"/v1/chats/{B}")
    out["files_after_delete"] = sorted(os.listdir(slots))
finally:
    lhd.terminate(); lp.terminate()
print(json.dumps(out, indent=1)); json.dump(out, open(f"e2e-direct-{variant}.json", "w"), indent=1)
