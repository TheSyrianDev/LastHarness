#!/usr/bin/env python3
"""Theming spike driver: screenshots of each theme, hover animation frames, live reload, tray, AT-SPI, PSS and CPU."""
import json, os, shutil, subprocess, sys, time
here = os.path.dirname(os.path.abspath(__file__)); app_dir = f"{here}/gpui-theme"; out = f"{here}/out/theme"; os.makedirs(out, exist_ok=True)
binary = f"{app_dir}/target/release/gt-gpui-theme"
res = {}
def start(*args):
    return subprocess.Popen([binary, *args], cwd=app_dir, stderr=open(f"{out}/app.log", "a"))
def window(p):
    for _ in range(100):
        for c in json.loads(subprocess.check_output(["hyprctl", "clients", "-j"])):
            if c["pid"] == p.pid: return c
        time.sleep(0.2)
def shot(w, name):
    subprocess.run(["grim", "-g", f"{w['at'][0]},{w['at'][1]} {w['size'][0]}x{w['size'][1]}", f"{out}/{name}.png"])
def pss(pid): return round(next(int(l.split()[1]) for l in open(f"/proc/{pid}/smaps_rollup") if l.startswith("Pss:")) / 1024, 1)
def cpu(pid):
    f = open(f"/proc/{pid}/stat").read().rsplit(")", 1)[1].split(); return (int(f[11]) + int(f[12])) / os.sysconf("SC_CLK_TCK")
def sample(pid, sec):
    c0, t0 = cpu(pid), time.time(); time.sleep(sec); return {"cpu_pct": round(100 * (cpu(pid) - c0) / (time.time() - t0), 2), "pss_mb": pss(pid)}

names = sorted(f[:-5] for f in os.listdir(f"{app_dir}/themes") if f.endswith(".toml"))
# 1. every theme, one launch each (index is the sorted file order)
for i, n in enumerate(names):
    p = start("--theme", str(i)); w = window(p); time.sleep(2.5); shot(w, f"theme-{n}"); p.terminate(); p.wait()
res["themes"] = names

# 2. one long run: tray, AT-SPI, idle numbers, live reload, hover animation, Arabic typing, RTL
p = start("--theme", "4", "--tray", "1"); w = window(p); time.sleep(3)
res["window"] = {k: w.get(k) for k in ("class", "title", "size", "xwayland")}
res["idle"] = sample(p.pid, 10)
try:
    items = subprocess.run(["busctl", "--user", "get-property", "org.kde.StatusNotifierWatcher", "/StatusNotifierWatcher", "org.kde.StatusNotifierWatcher", "RegisteredStatusNotifierItems"], capture_output=True, text=True)
    res["tray_watcher"] = (items.stdout or items.stderr).strip()[:300]
except Exception as e: res["tray_watcher"] = str(e)
res["tray_app_log"] = [l.strip() for l in open(f"{out}/app.log") if "tray" in l][-1:]
atspi = subprocess.run([sys.executable, f"{here}/atspi_dump.py", "gt-gpui-theme"], capture_output=True, text=True)
open(f"{out}/atspi.txt", "w").write(atspi.stdout + atspi.stderr)
shot(w, "glass-dark-idle")
# live reload: change the Glass dark file while running
f = f"{app_dir}/themes/glass-dark.toml"; orig = open(f).read()
open(f, "w").write(orig.replace('roundness = 18', 'roundness = 4').replace('accent = "#6aa6ff"', 'accent = "#ff6a6a"').replace('panel = "#ffffff1a"', 'panel = "#ff000030"'))
time.sleep(1.5); shot(w, "live-reload-after")
open(f, "w").write(orig); time.sleep(1.2); shot(w, "live-reload-restored")
# hover animation: move the pointer onto the Send button and capture frames
subprocess.run(["hyprctl", "dispatch", "focuswindow", f"pid:{p.pid}"], capture_output=True)
bx = w["at"][0] + w["size"][0] - 330; by = w["at"][1] + w["size"][1] - 80
subprocess.run(["hyprctl", "dispatch", "movecursor", str(bx), str(by)], capture_output=True)
for k in range(6):
    shot(w, f"hover-{k}"); time.sleep(0.07)
# Arabic typing and RTL mirror (only when this window is focused)
act = json.loads(subprocess.check_output(["hyprctl", "activewindow", "-j"]))
if act.get("pid") == p.pid:
    subprocess.run(["wtype", "مرحبا بالعالم hello"]); time.sleep(0.5); shot(w, "arabic-typed"); res["typed"] = True
else: res["typed"] = False
res["animating"] = sample(p.pid, 8)
p.terminate(); p.wait()
# 3. mirrored layout
p = start("--theme", "5", "--rtl", "1"); w = window(p); time.sleep(2.5); shot(w, "rtl-mirrored"); p.terminate(); p.wait()
json.dump(res, open(f"{out}/result.json", "w"), indent=1); print(json.dumps(res, indent=1))
