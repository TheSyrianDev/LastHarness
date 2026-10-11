import json, os, time, urllib.request
B = "http://127.0.0.1:9292"; Q = "Qwen3.6-35B-A3B-NanoPlus-balanced"; S = "LFM2.5-2.6B-balanced"
def call(path, body=None, timeout=900):
    req = urllib.request.Request(B + path, data=None if body is None else json.dumps(body).encode(), headers={"Content-Type": "application/json"})
    t = time.time()
    with urllib.request.urlopen(req, timeout=timeout) as r: raw = r.read()
    try: return json.loads(raw), round(time.time() - t, 2)
    except Exception: return raw, round(time.time() - t, 2)
text = "\n".join(f"Entry {i}: the river near the stone was noted on day {i} with code {i*7919%100003}." for i in range(60))
out = {}
call(f"/upstream/{Q}/slots/0?action=erase", {})
r, dt = call(f"/upstream/{Q}/completion", {"prompt": text, "n_predict": 1, "id_slot": 0, "cache_prompt": True}); out["prefill_tokens"] = r["timings"]["prompt_n"]
d = call(f"/upstream/{Q}/slotkeeper")[0]["dir"]
r, _ = call(f"/upstream/{Q}/slots/0?action=save", {"filename": "swaptest.bin"}); out["saved_tokens"] = r["n_saved"]
slots = call(f"/upstream/{Q}/slots")[0]; out["slot_before_swap"] = {k: slots[0].get(k) for k in ("n_prompt_tokens", "n_ctx")}
# swap to another model, then come back
r, dt = call("/v1/chat/completions", {"model": S, "messages": [{"role": "user", "content": "Say OK."}], "max_tokens": 3}); out["swap_to_small_s"] = dt
running = call("/running")[0]; out["running_after_swap"] = [m["model"] for m in running["running"]]
r, dt = call("/v1/chat/completions", {"model": Q + ":instruct", "messages": [{"role": "user", "content": "Say OK."}], "max_tokens": 3}); out["swap_back_s"] = dt
slots = call(f"/upstream/{Q}/slots")[0]; out["slot_after_swap_back"] = {k: slots[0].get(k) for k in ("n_prompt_tokens",)}
out["file_survived"] = os.path.exists(os.path.join(d, "swaptest.bin"))
call(f"/upstream/{Q}/slots/0?action=erase", {})
r, dt = call(f"/upstream/{Q}/slots/0?action=restore", {"filename": "swaptest.bin"}); out["restore_after_swap"] = {"n_restored": r.get("n_restored"), "secs": dt}
os.remove(os.path.join(d, "swaptest.bin"))
print(json.dumps(out, indent=1)); json.dump(out, open("swap.json", "w"), indent=1)
