# Run with: python3 -I crash_driver.py   (from a scratch directory). Builds the hostile and broken pages, then runs
# the harness and the Falco command line on each in a child process with a 20 s limit and 6 GB of memory, and prints
# the exit code, so a crash or a hang is seen to leave the parent alone.
import subprocess, os, time, resource
os.makedirs("crash", exist_ok=True)
w = lambda n, d, m="w": open("crash/" + n, m).write(d)
w("trunc_mid_tag.html", "<!doctype html><html><body><div class='a'><p>Hello <b>wor")
w("trunc_mid_css.html", "<style>body{color:red} .a{background:linear-gradient(90deg,#f00,")
w("unclosed.html", "<div>" * 5000 + "x")
w("deep_nest_100k.html", "<div>" * 100000 + "x" + "</div>" * 100000)
w("bad_utf8.html", b"<html><body><p>caf\xc3\x28 \xff\xfe\xfd text \xe0\x80\x80</p></body></html>", "wb")
w("nonascii_style_tag.html", "<STYLE>/* İİİ */ p{color:red}</STYLE><p>İstanbul</p><script>1</script>")
w("huge_table.html", "<table>" + "<tr><td>cell</td>" * 20000 + "</table><script>1</script>")
w("huge_attr.html", '<div title="' + "a" * 3_000_000 + '">x</div><script>1</script>')
w("5mb_text.html", "<html><body>" + "<p>Lorem ipsum dolor sit amet, consectetur adipiscing elit.</p>\n" * 90000 + "</body><script>1</script></html>")
w("5mb_divs.html", "<html><body>" + '<div style="margin:1px;padding:1px;border:1px solid #ccc">row</div>\n' * 70000 + "</body><script>1</script></html>")
w("script_loop.html", "<p>loop</p><script>while(true){}</script>")
w("script_recursion.html", "<p>rec</p><script>function f(){ return f(); } f();</script>")
w("script_bigarray.html", "<p>mem</p><script>var a=[]; for(var i=0;i<1e9;i++){ a.push('xxxxxxxxxxxxxxxxxxxxxxxxxxxx'+i); }</script>")
if os.path.exists("/home/omar/index.html"):
    w("trunc_user_index.html", open("/home/omar/index.html", "rb").read(), "wb")
H = "hr-falco"; C = "falco"
env = dict(os.environ, HR_FORCE_SCRIPT="1")
lim = lambda: resource.setrlimit(resource.RLIMIT_AS, (6 * 2**30, 6 * 2**30))
for f in sorted(os.listdir("crash")):
    if not f.endswith(".html"): continue
    for name, cmd in (("lib", [H, "crash/" + f, "crash/" + f + ".png"]), ("cli", ["unshare", "-rn", C, "crash/" + f, "--out", "crash/" + f + ".cli.png", "--width", "900"])):
        t = time.time()
        try:
            r = subprocess.run(cmd, capture_output=True, text=True, timeout=20, env=env, preexec_fn=lim)
            rc = r.returncode
        except subprocess.TimeoutExpired:
            rc = "TIMEOUT 20 s"
        print(f"{f:28} {name:4} rc={rc} {time.time()-t:5.1f}s")
print("parent still alive, pid", os.getpid())
