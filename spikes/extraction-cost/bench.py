# Spike 5 (throwaway): cost of the end-of-session extraction turn on a local model.
# Cold prefill vs prefix-cache hit vs slot save/restore, on llama-server. Stdlib only.
# Usage: bench.py <label> <context_tokens_target> -- <llama-server command and flags without --port>
import json, os, random, shutil, subprocess, sys, tempfile, time, urllib.request
PORT = 8792
def http(path, body=None, timeout=900, method=None):
    r = urllib.request.Request(f"http://127.0.0.1:{PORT}{path}", None if body is None else json.dumps(body).encode(), {"Content-Type": "application/json"}, method=method)
    return json.load(urllib.request.urlopen(r, timeout=timeout))
WORDS = "the project memory store should keep notes about decisions and preferences because the user wants answers short and plain; we compared sqlite and files, chose a folder of markdown pages, and agreed to review facts at the end of each session before saving them".split()
def conversation(target_words, rng):
    msgs, n = [], 0
    while n < target_words:
        u = " ".join(rng.choice(WORDS) for _ in range(rng.randint(20, 60)))
        a = " ".join(rng.choice(WORDS) for _ in range(rng.randint(60, 140)))
        msgs += [{"role": "user", "content": u}, {"role": "assistant", "content": a}]
        n += len(u.split()) + len(a.split())
    return msgs
Q = {"role": "user", "content": "Review this conversation. List the facts and decisions worth remembering, one short line each."}
def ask(msgs, label):
    t = time.time()
    d = http("/v1/chat/completions", {"messages": msgs, "max_tokens": 1, "temperature": 0, "cache_prompt": True, "chat_template_kwargs": {"enable_thinking": False}})
    tm = d["timings"]; wall = time.time() - t
    print(f"  {label:42} prompt_n={tm['prompt_n']:>6} cache_n={tm['cache_n']:>6} prompt_ms={tm['prompt_ms']:>9.0f}  wall={wall:6.2f}s")
    return tm
def main():
    label, ctx = sys.argv[1], int(sys.argv[2]); cmd = sys.argv[sys.argv.index("--") + 1:]
    try: urllib.request.urlopen("http://127.0.0.1:9292/unload", timeout=60)  # free llama-swap's model
    except Exception: pass
    time.sleep(3)
    slots = tempfile.mkdtemp(prefix="slots-", dir=os.path.dirname(os.path.abspath(__file__)))
    t0 = time.time()
    p = subprocess.Popen(cmd + ["--port", str(PORT), "--slot-save-path", slots], stdout=subprocess.DEVNULL, stderr=open(f"/tmp/ex-{label}.log", "w"))
    try:
        while True:
            try: urllib.request.urlopen(f"http://127.0.0.1:{PORT}/health", timeout=2); break
            except Exception:
                if p.poll() is not None: raise SystemExit("server died: /tmp/ex-%s.log" % label)
                time.sleep(1)
        print(f"== {label}: server ready in {time.time()-t0:.0f}s, target ~{ctx} words of conversation"); 
        rng = random.Random(7); conv = conversation(int(ctx * 0.75), rng)
        ask(conv, "warm up (loads kernels, fills cache)")  # warm
        for rep in (1, 2):
            http("/slots/0?action=erase", {}); 
            print(f" run {rep}")
            ask(conv + [Q], "A cold: whole session + question")
            http("/slots/0?action=erase", {})
            ask(conv, "   (session turns processed during chat)")
            t = time.time(); http("/slots/0?action=save", {"filename": "s.bin"}); save = time.time() - t
            size = os.path.getsize(f"{slots}/s.bin") / 1048576
            ask(conv + [Q], "B cache hit: question after session")
            http("/slots/0?action=erase", {})
            t = time.time(); r = http("/slots/0?action=restore", {"filename": "s.bin"}); rest = time.time() - t
            print(f"  C slot saved after session: save {save:.2f}s, file {size:.0f} MB; restore {rest:.2f}s ({r.get('n_restored')} tokens)")
            ask(conv + [Q], "C after restore: question")
            http("/slots/0?action=erase", {})
            # changed prefix = what happens if the harness edits earlier turns or compacts: cache misses
            ask([{"role": "system", "content": "You are a careful assistant."}] + conv + [Q], "D prefix changed (new system line)")
    finally:
        p.terminate(); p.wait(); shutil.rmtree(slots, ignore_errors=True)
main()
