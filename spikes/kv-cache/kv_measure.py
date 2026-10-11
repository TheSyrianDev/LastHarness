#!/usr/bin/env python3
"""Slot save/restore through llama-swap: cache file size, save time, restore time, and the first reply after a
restore against re-reading the prompt. Usage: kv_measure.py TOKENS [MODEL]   (talks to llama-swap on 9292 only)"""
import json, os, sys, time, urllib.request
N = int(sys.argv[1]); M = sys.argv[2] if len(sys.argv) > 2 else "Qwen3.6-35B-A3B-NanoPlus-balanced"
B = "http://127.0.0.1:9292"
def call(path, body=None, method=None, timeout=1800):
    req = urllib.request.Request(B + path, data=None if body is None else json.dumps(body).encode(), headers={"Content-Type": "application/json"}, method=method or ("POST" if body is not None else "GET"))
    t = time.time()
    with urllib.request.urlopen(req, timeout=timeout) as r: raw = r.read()
    dt = time.time() - t
    try: return json.loads(raw), dt
    except Exception: return raw, dt
def chat(prompt):
    """Raw /completion: slotkeeper only touches chat paths, so this measures llama-server's own slot behaviour."""
    r, dt = call("/upstream/%s/completion" % M, {"prompt": prompt, "n_predict": 1, "temperature": 0, "id_slot": 0, "cache_prompt": True})
    t = r.get("timings", {})
    return {"secs": round(dt, 2), "prompt_n": t.get("prompt_n"), "cache_n": t.get("cache_n"), "prompt_ms": round(t.get("prompt_ms", 0)), "answer": r.get("content")}
# text of roughly N tokens: numbered lines with varied words, so nothing repeats exactly
words = "river stone cloud engine garden paper window copper signal market forest ladder orange puzzle harbor quiet violet anchor meadow".split()
def make(n_lines): return "\n".join(f"Entry {i}: the {words[i % 19]} near the {words[(i * 7) % 19]} was noted on day {i * 3 % 365} with code {i * 7919 % 100003}." for i in range(n_lines))
lines = max(1, N // 28)
big = make(lines)
q1 = big + "\n\nReply with the single word:"
dirp = call(f"/upstream/{M}/slotkeeper")[0].get("dir")
out = {"target_tokens": N, "model": M, "slot_dir": dirp}
call(f"/upstream/{M}/slots/0?action=erase", {})
out["cold_read"] = chat(q1)                      # whole prompt read from nothing
name = f"kvtest-{N}.bin"
r, dt = call(f"/upstream/{M}/slots/0?action=save", {"filename": name}); out["save_secs"] = round(dt, 2); out["save_reply"] = {k: r.get(k) for k in ("n_saved", "n_written", "timings")} if isinstance(r, dict) else str(r)[:100]
path = os.path.join(dirp, name); out["file_mib"] = round(os.path.getsize(path) / 1048576, 1) if os.path.exists(path) else None
q2 = q1 + " Noted.\n\nQuestion: which entry number is the first one in this text? Answer:"
call(f"/upstream/{M}/slots/0?action=erase", {})
out["cold_next_turn"] = chat(q2)                 # next turn with no cache: full re-read
call(f"/upstream/{M}/slots/0?action=erase", {})
r, dt = call(f"/upstream/{M}/slots/0?action=restore", {"filename": name}); out["restore_secs"] = round(dt, 2); out["restore_reply"] = {k: r.get(k) for k in ("n_restored", "n_read")} if isinstance(r, dict) else str(r)[:100]
out["next_turn_after_restore"] = chat(q2)        # next turn from a restored cache
os.remove(path) if os.path.exists(path) else None
print(json.dumps(out, indent=1)); json.dump(out, open(f"kv-{N}.json", "w"), indent=1)
