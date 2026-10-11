#!/usr/bin/env python3
"""GPUI window with a Servo page from a helper process: memory split, crash isolation, restart, and a pop-out window."""
import json, os, signal, subprocess, sys, time
here = os.path.dirname(os.path.abspath(__file__)); root = os.path.dirname(here); out = f"{here}/out"; os.makedirs(out, exist_ok=True)
srv = subprocess.Popen([sys.executable, "-m", "http.server", "8099", "--bind", "127.0.0.1", "-d", f"{here}/pages"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL); time.sleep(0.5)
def pss(pid): return round(next(int(l.split()[1]) for l in open(f"/proc/{pid}/smaps_rollup") if l.startswith("Pss:")) / 1024, 1)
def kids(pid): return [int(x) for x in os.popen(f"pgrep -P {pid}").read().split()]
def window(pid):
    for _ in range(100):
        for c in json.loads(subprocess.check_output(["hyprctl", "clients", "-j"])):
            if c["pid"] == pid: return c
        time.sleep(0.2)
def shot(w, name): subprocess.run(["grim", "-g", f"{w['at'][0]},{w['at'][1]} {w['size'][0]}x{w['size'][1]}", f"{out}/{name}.png"])
res = {}
app = subprocess.Popen([f"{root}/gpui-theme/target/release/gt-gpui-theme", "--embed", "http://127.0.0.1:8099/article.html", "--helper", f"{here}/target/release/servo-helper"], cwd=f"{root}/gpui-theme", stderr=subprocess.DEVNULL)
w = window(app.pid); time.sleep(6)
hp = kids(app.pid)
res["main_pss_mb"] = pss(app.pid); res["helper_pss_mb"] = [pss(p) for p in hp]; shot(w, "embed-article")
# crash the helper hard; the app must stay up, and bring the page back
if hp: os.kill(hp[0], signal.SIGKILL)
time.sleep(1.2); res["app_alive_after_kill"] = app.poll() is None; shot(w, "embed-after-kill")
time.sleep(6); res["helper_restarted"] = len(kids(app.pid)) > 0; shot(w, "embed-restarted")
res["main_pss_after_mb"] = pss(app.pid)
app.terminate(); app.wait()
# pop-out: the same engine in its own window
pw = subprocess.Popen([f"{here}/target/release/servo-window", "--url", "http://127.0.0.1:8099/article.html"], stderr=open(f"{out}/window.log", "w"))
w2 = window(pw.pid); time.sleep(5)
if w2: res["popout"] = {"class": w2["class"], "size": w2["size"], "pss_mb": pss(pw.pid)}; shot(w2, "popout")
pw.terminate(); srv.terminate()
json.dump(res, open(f"{out}/embed.json", "w"), indent=1); print(json.dumps(res, indent=1))
