#!/usr/bin/env python3
"""Resume in the real window: chat once, close the app, open it again against the same core, pick the stored chat,
read the warning, continue. Shots go to target/gui-shots/resume-*.png."""
import json, os, subprocess, sys, tempfile, time
root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../.."))
out = f"{root}/target/gui-shots"; os.makedirs(out, exist_ok=True)
tmp = tempfile.mkdtemp(prefix="lh-gui-resume-")
env = dict(os.environ, XDG_DATA_HOME=tmp + "/data", XDG_CONFIG_HOME=tmp + "/config", XDG_RUNTIME_DIR=tmp + "/run"); os.makedirs(tmp + "/run")
app_env = dict(os.environ, XDG_DATA_HOME=tmp + "/data", XDG_CONFIG_HOME=tmp + "/config", LH_NO_TRAY="1")
port = 7852
lhd = subprocess.Popen([f"{root}/target/release/lhd", "--provider", "http://localhost:9292/v1", "--listen", f"127.0.0.1:{port}", "--token-file", tmp + "/token", "--idle-after", "0"], env=env, stdout=subprocess.DEVNULL, stderr=open(tmp + "/lhd.log", "w"))
while not os.path.exists(tmp + "/token"): time.sleep(0.1)
def launch():
    return subprocess.Popen([f"{root}/target/release/lastharness-gui", "--base", f"http://127.0.0.1:{port}", "--token-file", tmp + "/token", "--prefs", tmp + "/gui.toml", "--themes", tmp + "/themes"], env=app_env, stderr=open(tmp + "/app.log", "a"))
def window(app):
    for _ in range(100):
        for c in json.loads(subprocess.check_output(["hyprctl", "clients", "-j"])):
            if c["pid"] == app.pid: return c
        time.sleep(0.2)
def shot(app, w, n): subprocess.run(["grim", "-g", f"{w['at'][0]},{w['at'][1]} {w['size'][0]}x{w['size'][1]}", f"{out}/resume-{n}.png"])
def focus(app):
    for _ in range(10):
        a = json.loads(subprocess.check_output(["hyprctl", "activewindow", "-j"]))
        if a.get("pid") == app.pid: return True
        subprocess.run(["hyprctl", "dispatch", "focuswindow", f"pid:{app.pid}"], capture_output=True); time.sleep(0.5)
    return False
def key(app, spec):
    if not focus(app): raise SystemExit("window not focused")
    mods = spec.split("+"); k = mods.pop(); cmd = ["wtype"]
    for m in mods: cmd += ["-M", m]
    cmd += ["-k", k]
    for m in reversed(mods): cmd += ["-m", m]
    subprocess.run(cmd); time.sleep(0.6)
def typ(app, t):
    if not focus(app): raise SystemExit("window not focused")
    subprocess.run(["wtype", t]); time.sleep(0.4)
try:
    a1 = launch(); w = window(a1); time.sleep(5)
    typ(a1, "Name one planet."); key(a1, "Return"); time.sleep(15); shot(a1, w, "1-first-run")
    a1.terminate(); a1.wait(); time.sleep(1)
    # the core keeps running; a fresh window must list the chat; restart the core too for the harder case
    lhd.terminate(); lhd.wait()
    lhd = subprocess.Popen([f"{root}/target/release/lhd", "--provider", "http://localhost:9292/v1", "--listen", f"127.0.0.1:{port}", "--token-file", tmp + "/token", "--idle-after", "0"], env=env, stdout=subprocess.DEVNULL, stderr=open(tmp + "/lhd.log", "a"))
    time.sleep(2)
    a2 = launch(); w = window(a2); time.sleep(6); shot(a2, w, "2-reopened")
    # the new empty chat is on top; the stored one is below it
    key(a2, "ctrl+Next"); time.sleep(1.5); shot(a2, w, "3-stored-chat-with-warning")
    key(a2, "ctrl+Return"); time.sleep(2)
    typ(a2, "And its biggest moon?"); key(a2, "Return"); time.sleep(15); shot(a2, w, "4-continued")
    a2.terminate(); a2.wait()
finally:
    lhd.terminate()
    print("app log:", open(tmp + "/app.log").read()[-300:])
