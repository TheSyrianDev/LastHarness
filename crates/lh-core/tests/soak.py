#!/usr/bin/env python3
"""Long-session soak on a local model. Not part of `cargo test` (it needs llama-swap and a release build).

  python3 soak.py [--model NAME] [--turns N] [--bin PATH] [--provider URL]

It starts lhd in a scratch folder, holds one long conversation (with compaction turned low so it happens
several times), runs a department, then leaves the core alone to go idle and counts what it does while
idle. Memory is the proportional set size (PSS) of the lhd process from /proc, which is the honest number
when memory is shared with libraries. Prints a JSON summary.
"""
import argparse, json, os, shutil, signal, subprocess, sys, tempfile, time, urllib.request

ap = argparse.ArgumentParser()
ap.add_argument("--model", default="LFM2.5-2.6B-fast")
ap.add_argument("--turns", type=int, default=60)
ap.add_argument("--bin", default=os.path.join(os.path.dirname(__file__), "../../../target/release/lhd"))
ap.add_argument("--provider", default="http://localhost:9292/v1")
ap.add_argument("--port", type=int, default=7890)
ap.add_argument("--compact-at", type=int, default=3500)
ap.add_argument("--idle-after", type=int, default=20)
ap.add_argument("--idle-watch", type=int, default=60)
ap.add_argument("--dept-timeout", type=int, default=600)
ap.add_argument("--max-tokens", type=int, default=400)
a = ap.parse_args()

tmp = tempfile.mkdtemp(prefix="lh-soak-")
env = dict(os.environ, XDG_DATA_HOME=tmp + "/data", XDG_CONFIG_HOME=tmp + "/config", XDG_RUNTIME_DIR=tmp + "/run")
os.makedirs(tmp + "/run")
tokfile = tmp + "/token"
proc = subprocess.Popen([os.path.abspath(a.bin), "--provider", a.provider, "--listen", f"127.0.0.1:{a.port}", "--token-file", tokfile,
                         "--compact-at", str(a.compact_at), "--idle-after", str(a.idle_after), "--team-max-tokens", "3000"],
                        env=env, stdout=subprocess.DEVNULL, stderr=open(tmp + "/lhd.log", "w"))
base = f"http://127.0.0.1:{a.port}"
for _ in range(100):
    if os.path.exists(tokfile):
        break
    time.sleep(0.1)
tok = open(tokfile).read().strip()


def call(method, path, body=None, timeout=600):
    req = urllib.request.Request(base + path, method=method, data=json.dumps(body).encode() if body is not None else None,
                                 headers={"Authorization": "Bearer " + tok, "Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        t = r.read()
        return json.loads(t) if t else {}


def stream(method, path, body, on_event, timeout=900):
    req = urllib.request.Request(base + path, method=method, data=json.dumps(body).encode() if body is not None else None,
                                 headers={"Authorization": "Bearer " + tok, "Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        buf = ""
        for raw in r:
            line = raw.decode("utf-8", "replace").rstrip("\n")
            if line.startswith("data:"):
                try:
                    on_event(json.loads(line[5:].strip()))
                except json.JSONDecodeError:
                    pass


def pss_kb(pid):
    for l in open(f"/proc/{pid}/smaps_rollup"):
        if l.startswith("Pss:"):
            return int(l.split()[1])
    return -1


def rss_kb(pid):
    for l in open(f"/proc/{pid}/status"):
        if l.startswith("VmRSS:"):
            return int(l.split()[1])
    return -1


def ctxt(pid):
    v = n = 0
    for t in os.listdir(f"/proc/{pid}/task"):
        for l in open(f"/proc/{pid}/task/{t}/status"):
            if l.startswith("voluntary_ctxt_switches:"):
                v += int(l.split()[1])
            if l.startswith("nonvoluntary_ctxt_switches:"):
                n += int(l.split()[1])
    return v, n, len(os.listdir(f"/proc/{pid}/task"))


def cpu_ticks(pid):
    f = open(f"/proc/{pid}/stat").read().rsplit(")", 1)[1].split()
    return int(f[11]) + int(f[12])


out = {"model": a.model, "turns": a.turns, "pss_kb": {}, "samples": []}
time.sleep(3)
out["pss_kb"]["start"] = pss_kb(proc.pid)
out["rss_kb"] = {"start": rss_kb(proc.pid)}
t0 = time.time()

# ---- one long conversation ----
sid = call("POST", "/v1/sessions", {"model": a.model, "mode": "chat", "max_tokens": a.max_tokens})["id"]
topics = ["the water cycle", "how bread rises", "why the sky is blue", "how a bicycle stays upright", "what causes tides", "how vaccines work",
          "how a compass works", "why cats purr", "how glass is made", "what a prime number is"]
stats = {"turns_ok": 0, "errors": [], "compactions": 0, "empty": 0, "max_prompt_tokens": 0, "cached_tokens_last": 0, "first_token_s": []}
for i in range(a.turns):
    prompt = f"Turn {i}: in three short sentences, explain {topics[i % len(topics)]}. Then repeat this sentence: " + "the quick brown fox jumps over the lazy dog " * 8
    reply = []
    state = {"first": None}
    ts = time.time()

    def on(ev):
        t = ev.get("type")
        if t == "text":
            if state["first"] is None:
                state["first"] = time.time() - ts
            reply.append(ev.get("text", ""))
        elif t == "compacted":
            stats["compactions"] += 1
        elif t == "usage":
            stats["max_prompt_tokens"] = max(stats["max_prompt_tokens"], ev.get("prompt_tokens", 0))
            stats["cached_tokens_last"] = ev.get("cached_tokens", 0)
        elif t == "ask":
            call("POST", f"/v1/sessions/{sid}/answers/{ev['id']}", {"answer": "no"})
        elif t in ("error", "refused"):
            stats["errors"].append(f"turn {i}: {ev.get('message')}")
        elif t == "done" and ev.get("empty"):
            stats["empty"] += 1

    try:
        stream("POST", f"/v1/sessions/{sid}/turns", {"text": prompt}, on)
        stats["turns_ok"] += 1
        if state["first"] is not None:
            stats["first_token_s"].append(round(state["first"], 2))
    except Exception as e:  # keep soaking; the error is part of the result
        stats["errors"].append(f"turn {i}: {type(e).__name__}: {e}")
    if i % 10 == 0 or i == a.turns - 1:
        out["samples"].append({"turn": i, "pss_kb": pss_kb(proc.pid), "elapsed_s": round(time.time() - t0)})
    print(f"turn {i} done, {len(''.join(reply))} chars, compactions {stats['compactions']}", file=sys.stderr, flush=True)
out["conversation"] = stats
ft = stats.pop("first_token_s")
out["conversation"]["first_token_s_median"] = sorted(ft)[len(ft) // 2] if ft else None
out["conversation"]["first_token_s_max"] = max(ft) if ft else None

# ---- a department run ----
for t in ("frontier", "mid-tier", "fastest"):
    call("PUT", f"/v1/team/types/{t}", {"model": a.model})
call("PUT", "/v1/team/settings", {"autonomy": "fire-and-forget", "attempts": 2})
call("PUT", "/v1/team/departments/pets", {"text": "---\nname: pets\ndescription: Answers questions about pets.\nlead: planner\nmembers: researcher writer\n---\nKeep answers short.\n"})
run = call("POST", "/v1/team/runs", {"department": "pets", "goal": "Explain in two plain sentences why dogs wag their tails. No files, no web."})["id"]
ts = time.time()
status = "timeout"
while time.time() - ts < a.dept_timeout:
    r = call("GET", f"/v1/team/runs/{run}")
    if r["finished"]:
        o = [e for e in r["events"] if e["type"] == "outcome"][-1]
        status = o["status"]
        out["department"] = {"status": status, "seconds": round(time.time() - ts), "events": len(r["events"]), "result": (o.get("result") or o.get("report") or "")[:300]}
        break
    time.sleep(3)
else:
    r = call("GET", f"/v1/team/runs/{run}")
    out["department"] = {"status": "timeout", "events": [(e.get("type"), (e.get("note") or e.get("summary") or e.get("report") or e.get("verdict") or "")[:120]) for e in r["events"]][-12:]}
out["pss_kb"]["after_department"] = pss_kb(proc.pid)

# ---- idle ----
time.sleep(a.idle_after + 15)
v0, n0, threads = ctxt(proc.pid)
c0 = cpu_ticks(proc.pid)
time.sleep(a.idle_watch)
v1, n1, _ = ctxt(proc.pid)
c1 = cpu_ticks(proc.pid)
hz = os.sysconf("SC_CLK_TCK")
out["idle"] = {"watched_s": a.idle_watch, "threads": threads, "voluntary_ctxt_switches_per_s": round((v1 - v0) / a.idle_watch, 2), "involuntary_per_s": round((n1 - n0) / a.idle_watch, 2),
               "cpu_seconds_used": round((c1 - c0) / hz, 3), "pss_kb": pss_kb(proc.pid)}
out["pss_kb"]["end"] = pss_kb(proc.pid)
out["rss_kb"]["end"] = rss_kb(proc.pid)
out["total_s"] = round(time.time() - t0)
out["lhd_log_tail"] = open(tmp + "/lhd.log").read()[-400:]
print(json.dumps(out, indent=2))
proc.send_signal(signal.SIGINT)
try:
    proc.wait(10)
except subprocess.TimeoutExpired:
    proc.kill()
shutil.rmtree(tmp, ignore_errors=True)
