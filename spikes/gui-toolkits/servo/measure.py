#!/usr/bin/env python3
"""Run servo-helper (headless) on the three local test pages served from loopback; record time, PSS and CPU.
Everything stays on this machine: the pages come from a local http.server."""
import json, os, subprocess, sys, time
here = os.path.dirname(os.path.abspath(__file__)); out = f"{here}/out"; os.makedirs(out, exist_ok=True)
helper = f"{here}/target/release/servo-helper"
srv = subprocess.Popen([sys.executable, "-m", "http.server", "8099", "--bind", "127.0.0.1", "-d", f"{here}/pages"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(0.5)
def tree(pid):
    ps = [pid]
    for l in os.popen(f"pgrep -P {pid}").read().split(): ps += tree(int(l))
    return ps
def pss(pid):
    tot = 0
    for p in tree(pid):
        try: tot += next(int(l.split()[1]) for l in open(f"/proc/{p}/smaps_rollup") if l.startswith("Pss:"))
        except Exception: pass
    return round(tot / 1024, 1)
def cpu(pid):
    t = 0
    for p in tree(pid):
        try:
            f = open(f"/proc/{p}/stat").read().rsplit(")", 1)[1].split(); t += int(f[11]) + int(f[12])
        except Exception: pass
    return t / os.sysconf("SC_CLK_TCK")
res = {}
for name in ("article", "js", "arabic"):
    png = f"{out}/{name}.png"
    if os.path.exists(png): os.remove(png)
    t0 = time.time()
    p = subprocess.Popen([helper, "--url", f"http://127.0.0.1:8099/{name}.html", "--out", png, "--size", "900x700", "--every-ms", "1000"], stderr=subprocess.PIPE, text=True)
    peak = 0; c0 = cpu(p.pid); ready = None
    while time.time() - t0 < 60:
        peak = max(peak, pss(p.pid))
        if os.path.exists(png) and ready is None: ready = round(time.time() - t0, 1)
        if ready and time.time() - t0 > ready + 3: break
        time.sleep(0.5)
    load_cpu = round(cpu(p.pid) - c0, 1)
    ci = cpu(p.pid); ti = time.time(); time.sleep(10)
    idle = {"cpu_pct": round(100 * (cpu(p.pid) - ci) / (time.time() - ti), 2), "pss_mb": pss(p.pid)}
    procs = [open(f"/proc/{q}/comm").read().strip() for q in tree(p.pid) if os.path.exists(f"/proc/{q}/comm")]
    p.terminate()
    try: err = p.communicate(timeout=5)[1][-400:]
    except Exception: err = ""
    res[name] = {"first_png_s": ready, "pss_peak_mb": peak, "cpu_seconds_loading": load_cpu, "idle": idle, "processes": procs, "stderr_tail": err.strip().splitlines()[-3:]}
srv.terminate()
json.dump(res, open(f"{out}/measure.json", "w"), indent=1); print(json.dumps(res, indent=1))
