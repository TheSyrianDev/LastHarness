#!/usr/bin/env python3
"""Spike: after a /tree move, can the saved KV cache of the old leaf be restored and trimmed by the server, so only
the part of the new branch that differs is read again?  Plain llama-server (no router, no slotkeeper).

Usage: branch_trim.py LABEL MODEL.gguf TOKENS [extra llama-server args...]
Builds an old prompt and a branch prompt that share the first 80 percent, then measures from the server's own
timings (prompt_n = tokens evaluated, cache_n = tokens reused):
  cold    erase the slot, send the branch prompt
  save    read the old prompt, save the slot (time and file size)
  restore erase the slot, restore the old file, send the branch prompt
  same    restore again, send the OLD prompt (sanity: everything reused)
Temperature 0, so the replies of 'cold' and 'restore' can be compared."""
import json, os, subprocess, sys, tempfile, time, urllib.request, glob

label, gguf, target = sys.argv[1], sys.argv[2], int(sys.argv[3])
extra = sys.argv[4:]
SERVER = glob.glob(os.path.expanduser("~/AI/engines/_prebuilt/mainline-b11541/llama-b11541/llama-server"))[0]
PORT = 8097
tmp = tempfile.mkdtemp(prefix=f"lh-kvtrim-{label}-")
slots = tmp + "/slots"; os.makedirs(slots)
logf = open(tmp + "/llama.log", "w")
ctx = int(target * 1.15) + 512
cmd = [SERVER, "-m", gguf, "--port", str(PORT), "--host", "127.0.0.1", "-c", str(ctx), "--parallel", "1", "--slot-save-path", slots, "--no-warmup", "--no-webui"] + extra
p = subprocess.Popen(cmd, stdout=logf, stderr=subprocess.STDOUT)

def http(method, path, body=None, timeout=3600):
    req = urllib.request.Request(f"http://127.0.0.1:{PORT}{path}", data=None if body is None else json.dumps(body).encode(), method=method, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        raw = r.read()
    return json.loads(raw) if raw else {}

for _ in range(900):
    try:
        urllib.request.urlopen(f"http://127.0.0.1:{PORT}/health", timeout=2).read(); break
    except Exception:
        if p.poll() is not None: sys.exit(f"server died, see {tmp}/llama.log")
        time.sleep(1)

words = "river stone cloud engine garden paper window copper signal market forest ladder orange puzzle harbor quiet violet anchor meadow".split()
def line(i, tag): return f"{tag} {i}: the {words[i % 19]} near the {words[(i * 7) % 19]} was noted on day {i * 3 % 365} with code {i * 7919 % 100003}.\n"
def ntok(text): return len(http("POST", "/tokenize", {"content": text})["tokens"])
per = ntok("".join(line(i, "Entry") for i in range(200))) / 200
n_total = int(target / per)
n_shared = int(n_total * 0.8)
shared = "".join(line(i, "Entry") for i in range(n_shared))
question = "\nQuestion: what is the code in the very last note above, and what is the number of that note? Answer in one short sentence.\nAnswer:"
old = shared + "".join(line(i, "Old branch note") for i in range(n_total - n_shared)) + question
new = shared + "".join(line(i + 500, "New branch note") for i in range(n_total - n_shared)) + question
t_old, t_new, t_shared = ntok(old), ntok(new), ntok(shared)

def complete(prompt):
    t0 = time.time()
    r = http("POST", "/completion", {"prompt": prompt, "n_predict": 40, "temperature": 0, "seed": 1, "cache_prompt": True, "n_probs": 3})
    tm = r.get("timings", {})
    return {"wall_s": round(time.time() - t0, 2), "prompt_n": tm.get("prompt_n"), "cache_n": tm.get("cache_n"), "prompt_s": round(tm.get("prompt_ms", 0) / 1000, 2), "tokens_evaluated": r.get("tokens_evaluated"), "tokens_cached": r.get("tokens_cached"), "reply": r.get("content", "").strip(), "lp": [[(t.get("token"), t.get("logprob")) for t in (pp.get("top_logprobs") or pp.get("top_probs") or [])] for pp in r.get("completion_probabilities", [])]}

def slot(action, name=None):
    t0 = time.time()
    try:
        r = http("POST", f"/slots/0?action={action}", {"filename": name} if name else {})
        return {"ok": True, "s": round(time.time() - t0, 2), "resp": r}
    except Exception as e:
        return {"ok": False, "s": round(time.time() - t0, 2), "error": str(e)[:300]}

out = {"label": label, "model": os.path.basename(gguf), "args": extra, "tokens": {"old": t_old, "branch": t_new, "shared_start": t_shared, "ctx": ctx}}
try:
    if os.environ.get("PREFIX"):
        # a file made from the shared start alone (what a per-node file would hold), then the branch on top of it
        slot("erase")
        out["cold_branch"] = complete(new)
        slot("erase")
        out["prefix_read"] = complete(shared)
        out["save"] = slot("save", "prefix.bin")
        out["file_bytes"] = os.path.getsize(slots + "/prefix.bin")
        slot("erase")
        out["restore"] = slot("restore", "prefix.bin")
        out["restore_then_branch"] = complete(new)
        out["replies_equal"] = out["cold_branch"]["reply"] == out["restore_then_branch"]["reply"]
        raise SystemExit
    slot("erase")
    out["cold_old_then_save"] = complete(old)            # reads the old leaf from nothing
    out["save"] = slot("save", "old.bin")
    out["file_bytes"] = os.path.getsize(slots + "/old.bin")
    slot("erase")
    out["cold_branch"] = complete(new)                    # (1) branch read from nothing
    if not os.environ.get("QUICK"):
        slot("erase")
        out["cold_branch_again"] = complete(new)          # same again: is a cold read repeatable?
    slot("erase")
    out["restore"] = slot("restore", "old.bin")
    out["restore_then_branch"] = complete(new)           # (2) restore the old leaf, send the branch
    if not os.environ.get("QUICK"):
        slot("erase")
        slot("restore", "old.bin")
        out["restore_then_old"] = complete(old)           # sanity: the old prompt after restore
    c, r = out["cold_branch"], out["restore_then_branch"]
    out["old_reply"] = out["cold_old_then_save"]["reply"]
    out["replies_equal"] = c["reply"] == r["reply"]
    def worst(a, b):
        d = 0.0; same_top = True
        for x, y in zip(a["lp"], b["lp"]):
            if x and y:
                same_top &= x[0][0] == y[0][0]
                d = max(d, abs((x[0][1] or 0) - (y[0][1] or 0)))
        return {"positions": min(len(a["lp"]), len(b["lp"])), "same_top_token_everywhere": same_top, "max_abs_logprob_diff": d}
    out["logprobs_cold_vs_restore"] = worst(c, r)
    if "cold_branch_again" in out: out["logprobs_cold_vs_cold"] = worst(c, out["cold_branch_again"])
    if "cold_branch_again" in out: out["cold_repeat_equal"] = c["reply"] == out["cold_branch_again"]["reply"]
except SystemExit:
    pass
finally:
    p.terminate()
    try: p.wait(30)
    except Exception: p.kill()
def top1(d): return [(x[0][0], round(x[0][1] or 0, 4)) for x in d.get("lp", []) if x]
out["top1_cold_branch"] = top1(out["cold_branch"]) if "cold_branch" in out else []
out["top1_restore_branch"] = top1(out["restore_then_branch"]) if "restore_then_branch" in out else []
for k in list(out):
    if isinstance(out[k], dict) and "lp" in out[k]: out[k].pop("lp")
log = open(tmp + "/llama.log").read().splitlines()
keys = ("checkpoint", "forcing", "re-processing", "restor", "slot", "cache_reuse", "kv cache rm", "invalid", "truncat", "SWA", "swa", "recurrent", "n_past", "common")
out["log_hits"] = [l for l in log if any(k in l for k in keys)][-40:]
out["tmp"] = tmp
json.dump(out, open(os.path.join(os.path.dirname(__file__), f"trim-{label}.json"), "w"), indent=1)
print(json.dumps({k: v for k, v in out.items() if k != "log_hits"}, indent=1))
