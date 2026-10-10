# Spike 2 driver: run the TUI in a pty, type prompts, sample PSS from /proc/<pid>/smaps_rollup.
import os, pty, sys, time, select, fcntl, termios, struct
TURNS = int(sys.argv[1]) if len(sys.argv) > 1 else 30
def pss(pid):
    for l in open(f"/proc/{pid}/smaps_rollup"):
        if l.startswith("Pss:"): return int(l.split()[1])
    return -1
def rss(pid):
    for l in open(f"/proc/{pid}/status"):
        if l.startswith("VmRSS:"): return int(l.split()[1])
pid, fd = pty.fork()
if pid == 0:
    os.execv("./target/release/tui-ram", ["tui-ram"])
fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 50, 160, 0, 0))
def drain(t):
    end = time.time() + t
    while time.time() < end:
        r, _, _ = select.select([fd], [], [], 0.1)
        if r:
            try: os.read(fd, 65536)
            except OSError: return
def quiet(maxwait=120, gap=2.0):
    last = time.time(); start = time.time()
    while time.time() - start < maxwait:
        r, _, _ = select.select([fd], [], [], 0.2)
        if r:
            try: os.read(fd, 65536)
            except OSError: return
            last = time.time()
        elif time.time() - last > gap: return
sys.stdout.reconfigure(line_buffering=True)
drain(3)
print(f"start        pss={pss(pid)} kB rss={rss(pid)} kB")
drain(10)
print(f"idle 10s     pss={pss(pid)} kB rss={rss(pid)} kB")
peak = 0
for i in range(1, TURNS + 1):
    os.write(fd, f"Write a detailed paragraph about topic number {i}: the history of bridges.".encode() + b"\r")
    drain(9)
    p = pss(pid); peak = max(peak, p)
    if i in (1, 5, 10, 20, 30, 40, 50, 60) or i == TURNS:
        print(f"turn {i:<3}     pss={p} kB rss={rss(pid)} kB")
drain(10)
print(f"idle after   pss={pss(pid)} kB rss={rss(pid)} kB  peak_pss={peak} kB")
os.write(fd, b"\x1b"); time.sleep(0.5)
