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
ap.add_argument("--steps", default="", help="semicolon list: key:ctrl+t, type:text, enter, wait:2, shot:name, prefs, move:X,Y / click:X,Y (pointer, screenshot pixels), mem")
ap.add_argument("--page", default="", help="seed one memory page: TITLE|TEXT")
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
if a.page:
    import urllib.request
    title, text = a.page.split("|", 1)
    tok = open(tmp + "/token").read().strip()
    rq = urllib.request.Request(f"http://127.0.0.1:{a.port}/v1/memory/pages", data=json.dumps({"layer": "facts", "title": title, "text": text, "gist": text[:40]}).encode(), headers={"Authorization": f"Bearer {tok}", "Content-Type": "application/json"})
    urllib.request.urlopen(rq).read()
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
def px_scale():
    """Screenshot pixels per compositor unit: the width in the PNG header over the window's reported width."""
    import struct
    cur = next(c for c in json.loads(subprocess.check_output(["hyprctl", "clients", "-j"])) if c["pid"] == app.pid)
    f = f"{out}/{a.name}-start.png"
    return struct.unpack(">I", open(f, "rb").read(24)[16:20])[0] / cur["size"][0]
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
        def focused():
            for _ in range(8):
                act = json.loads(subprocess.check_output(["hyprctl", "activewindow", "-j"]))
                if act.get("pid") == app.pid: return act
                subprocess.run(["hyprctl", "dispatch", "focuswindow", f"pid:{app.pid}"], capture_output=True); time.sleep(0.5)
            return act
        for step in a.steps.split(";"):
            step = step.strip()
            act = focused()
            if act.get("pid") != app.pid and not step.startswith(("wait", "shot", "mem", "windows", "cpu")):
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
            elif step.startswith("theme:"):
                fn, body = step[6:].split("|", 1)
                os.makedirs(tmp + "/themes", exist_ok=True); open(f"{tmp}/themes/{fn}", "w").write(body.replace("\\n", "\n"))
            elif step.startswith("move:"):
                # X,Y are pixels in the screenshots; the compositor works in scaled units
                scale = px_scale()
                x, y = (int(v) for v in step[5:].split(","))
                subprocess.run(["hyprctl", "dispatch", f"hl.dsp.cursor.move({{ x = {w['at'][0] + x / scale}, y = {w['at'][1] + y / scale} }})"], capture_output=True); time.sleep(0.6)
                res.setdefault("cursor", []).append(subprocess.check_output(["hyprctl", "cursorpos"]).decode().strip())
            elif step.startswith("click:"):
                # a real left click at X,Y (screenshot pixels): move the pointer, then send the button to the app's window
                scale = px_scale()
                x, y = (int(v) for v in step[6:].split(","))
                subprocess.run(["hyprctl", "dispatch", f"hl.dsp.cursor.move({{ x = {w['at'][0] + x / scale}, y = {w['at'][1] + y / scale} }})"], capture_output=True); time.sleep(0.3)
                subprocess.run(["hyprctl", "dispatch", f'hl.dsp.send_shortcut({{ mods = "", key = "mouse:272", window = "pid:{app.pid}" }})'], capture_output=True); time.sleep(0.6)
            elif step.startswith("shot:"): shot(w, step[5:])
            elif step.startswith("shotall:"): subprocess.run(["grim", f"{out}/{a.name}-{step[8:]}.png"])
            elif step == "windows": res["windows"] = [c["class"] for c in json.loads(subprocess.check_output(["hyprctl", "clients", "-j"])) if c["pid"] == app.pid]
            elif step == "cpu":
                f = lambda: (lambda x: (int(x[11]) + int(x[12])) / os.sysconf("SC_CLK_TCK"))(open(f"/proc/{app.pid}/stat").read().rsplit(")", 1)[1].split())
                c0, t0 = f(), time.time(); time.sleep(10); res.setdefault("cpu_pct_10s", []).append(round(100 * (f() - c0) / (time.time() - t0), 2))
            elif step == "prefs": res.setdefault("prefs", []).append(open(prefs).read().replace("\n", " ") if os.path.exists(prefs) else "(none)")
            elif step == "mem": res.setdefault("mem", []).append(pss(app.pid))
    res["pss_mb_after"] = pss(app.pid)
    print(json.dumps(res))
finally:
    app.terminate(); lhd.terminate()
    print("app log tail:", open(tmp + "/app.log").read()[-300:])
