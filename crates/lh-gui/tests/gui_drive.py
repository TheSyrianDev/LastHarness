#!/usr/bin/env python3
"""Drive the real desktop app in a window on this Wayland session: start a scratch lhd, open the app, take
screenshots, and read memory and CPU. Usage: gui_drive.py SHOT_NAME [--theme LABEL] [--rtl] [--seconds N] [--type TEXT] [--send]
Needs: grim, hyprctl, wtype (only with --type). Nothing leaves the machine; the provider is the local llama-swap."""
import argparse, json, os, subprocess, sys, tempfile, time

ap = argparse.ArgumentParser()
ap.add_argument("name")
ap.add_argument("--theme", default="")
ap.add_argument("--rtl", action="store_true")
ap.add_argument("--seconds", type=float, default=6)
ap.add_argument("--type", dest="text", default="")
ap.add_argument("--send", action="store_true")
ap.add_argument("--wait", type=float, default=0, help="seconds to wait after sending")
ap.add_argument("--steps", default="", help="semicolon list: key:ctrl+t, type:text, enter, wait:2, shot:name, mem")
ap.add_argument("--provider", default="http://localhost:9292/v1")
ap.add_argument("--port", type=int, default=7851)
a = ap.parse_args()
root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../.."))
out = f"{root}/target/gui-shots"; os.makedirs(out, exist_ok=True)
tmp = tempfile.mkdtemp(prefix="lh-gui-")
env = dict(os.environ, XDG_DATA_HOME=tmp + "/data", XDG_CONFIG_HOME=tmp + "/config", XDG_RUNTIME_DIR=tmp + "/run", LH_NO_TRAY="1")
os.makedirs(tmp + "/run")
app_env = dict(os.environ, XDG_DATA_HOME=tmp + "/data", XDG_CONFIG_HOME=tmp + "/config", LH_NO_TRAY="1")  # the app keeps the real runtime folder: the Wayland socket is there
lhd = subprocess.Popen([f"{root}/target/release/lhd", "--provider", a.provider, "--listen", f"127.0.0.1:{a.port}", "--token-file", tmp + "/token", "--idle-after", "0"], env=env, stdout=subprocess.DEVNULL, stderr=open(tmp + "/lhd.log", "w"))
for _ in range(100):
    if os.path.exists(tmp + "/token"): break
    time.sleep(0.1)
prefs = tmp + "/gui.toml"
if a.theme or a.rtl:
    open(prefs, "w").write((f'theme = "{a.theme}"\n' if a.theme else "") + ('direction = "rtl"\n' if a.rtl else ""))
app = subprocess.Popen([f"{root}/target/release/lastharness-gui", "--base", f"http://127.0.0.1:{a.port}", "--token-file", tmp + "/token", "--prefs", prefs, "--themes", tmp + "/themes"], env=app_env, stderr=open(tmp + "/app.log", "w"))
def window():
    for _ in range(100):
        for c in json.loads(subprocess.check_output(["hyprctl", "clients", "-j"])):
            if c["pid"] == app.pid: return c
        time.sleep(0.2)
def shot(w, n): subprocess.run(["grim", "-g", f"{w['at'][0]},{w['at'][1]} {w['size'][0]}x{w['size'][1]}", f"{out}/{a.name}-{n}.png"])
def pss(pid): return round(next(int(l.split()[1]) for l in open(f"/proc/{pid}/smaps_rollup") if l.startswith("Pss:")) / 1024, 1)
try:
    w = window(); time.sleep(a.seconds); shot(w, "start")
    res = {"pss_mb": pss(app.pid)}
    if a.text:
        subprocess.run(["hyprctl", "dispatch", "focuswindow", f"pid:{app.pid}"], capture_output=True); time.sleep(0.4)
        act = json.loads(subprocess.check_output(["hyprctl", "activewindow", "-j"]))
        if act.get("pid") == app.pid:
            subprocess.run(["wtype", a.text]); time.sleep(0.4); shot(w, "typed")
            if a.send:
                subprocess.run(["wtype", "-k", "Return"]); time.sleep(2); shot(w, "sent")
                if a.wait: time.sleep(a.wait); shot(w, "answer")
        else: res["typed"] = "window was not focused; nothing typed"
    if a.steps:
        subprocess.run(["hyprctl", "dispatch", "focuswindow", f"pid:{app.pid}"], capture_output=True); time.sleep(0.4)
        for step in a.steps.split(";"):
            step = step.strip()
            act = json.loads(subprocess.check_output(["hyprctl", "activewindow", "-j"]))
            if act.get("pid") != app.pid and not step.startswith(("wait", "shot", "mem")):
                res["stopped"] = f"window not focused at step {step!r}"; break
            if step.startswith("key:"):
                mods = step[4:].split("+"); key = mods.pop()
                cmd = ["wtype"]
                for m in mods: cmd += ["-M", m]
                cmd += ["-k", key]
                for m in reversed(mods): cmd += ["-m", m]
                subprocess.run(cmd); time.sleep(0.5)
            elif step.startswith("type:"): subprocess.run(["wtype", step[5:]]); time.sleep(0.4)
            elif step == "enter": subprocess.run(["wtype", "-k", "Return"]); time.sleep(0.5)
            elif step.startswith("wait:"): time.sleep(float(step[5:]))
            elif step.startswith("shot:"): shot(w, step[5:])
            elif step == "mem": res.setdefault("mem", []).append(pss(app.pid))
    res["pss_mb_after"] = pss(app.pid)
    print(json.dumps(res))
finally:
    app.terminate(); lhd.terminate()
    print("app log tail:", open(tmp + "/app.log").read()[-300:])
