#!/usr/bin/env python3
"""Run one test app against the mock server: PSS and CPU idle and while streaming, plus a screenshot.
Usage: drive.py NAME BINARY [extra app args]   (opens a real window on the current Wayland session)"""
import json, os, subprocess, sys, time

name, binary, extra = sys.argv[1], sys.argv[2], sys.argv[3:]
here = os.path.dirname(os.path.abspath(__file__))
out = f"{here}/out/{name}"; os.makedirs(out, exist_ok=True)
port = 7999
mock = subprocess.Popen([sys.executable, f"{here}/mock_lhd.py", str(port), "60"])
time.sleep(0.6)
app = subprocess.Popen([binary, "--base", f"http://127.0.0.1:{port}", "--token-file", "/dev/null"] + extra, stderr=open(f"{out}/app.log", "w"))
ck = os.sysconf("SC_CLK_TCK")

def pss(pid):
    for l in open(f"/proc/{pid}/smaps_rollup"):
        if l.startswith("Pss:"): return int(l.split()[1]) / 1024
def cpu(pid):
    f = open(f"/proc/{pid}/stat").read().rsplit(")", 1)[1].split()
    return (int(f[11]) + int(f[12])) / ck
def switches(pid):
    t = 0
    for d in os.listdir(f"/proc/{pid}/task"):
        for l in open(f"/proc/{pid}/task/{d}/status"):
            if l.startswith(("voluntary_ctxt_switches", "nonvoluntary_ctxt_switches")): t += int(l.split()[1])
    return t
def window():
    for c in json.loads(subprocess.check_output(["hyprctl", "clients", "-j"])):
        if c["pid"] == app.pid: return c
def window_wait():
    for _ in range(100):
        w = window()
        if w: return w
        time.sleep(0.2)
def window_sample(sec):
    c0, s0, t0 = cpu(app.pid), switches(app.pid), time.time(); peak = 0
    while time.time() - t0 < sec:
        time.sleep(1); peak = max(peak, pss(app.pid))
    dt = time.time() - t0
    return {"cpu_pct": round(100 * (cpu(app.pid) - c0) / dt, 2), "switches_per_s": round((switches(app.pid) - s0) / dt, 1), "pss_mb": round(pss(app.pid), 1), "pss_peak_mb": round(peak, 1)}

res = {}
w = window_wait()
res["window"] = {k: w.get(k) for k in ("class", "title", "size", "xwayland")}
time.sleep(4)
res["start_pss_mb"] = round(pss(app.pid), 1)
res["idle"] = window_sample(15)
subprocess.run(["grim", "-g", f"{w['at'][0]},{w['at'][1]} {w['size'][0]}x{w['size'][1]}", f"{out}/idle.png"])
# type a message (Arabic + English) and send
subprocess.run(["hyprctl", "dispatch", "focuswindow", f"pid:{app.pid}"], capture_output=True)
time.sleep(0.5)
def focused_ok():
    a = json.loads(subprocess.check_output(["hyprctl", "activewindow", "-j"]))
    return a.get("pid") == app.pid
if not focused_ok():
    print("app not focused; refusing to type"); res["typed"] = False; app.terminate(); mock.terminate(); sys.exit(1)
subprocess.run(["wtype", "مرحبا hello"]); time.sleep(0.3)
subprocess.run(["grim", "-g", f"{w['at'][0]},{w['at'][1]} {w['size'][0]}x{w['size'][1]}", f"{out}/typed.png"])
if focused_ok(): subprocess.run(["wtype", "-k", "Return"])
time.sleep(1)
res["streaming"] = window_sample(12)
subprocess.run(["grim", "-g", f"{w['at'][0]},{w['at'][1]} {w['size'][0]}x{w['size'][1]}", f"{out}/streaming.png"])
time.sleep(12)
res["after"] = window_sample(10)
subprocess.run(["grim", "-g", f"{w['at'][0]},{w['at'][1]} {w['size'][0]}x{w['size'][1]}", f"{out}/after.png"])
json.dump(res, open(f"{out}/result.json", "w"), indent=1)
print(json.dumps(res, indent=1))
app.terminate(); mock.terminate()
