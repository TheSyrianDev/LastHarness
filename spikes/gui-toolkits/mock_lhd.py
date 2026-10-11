#!/usr/bin/env python3
"""Stand-in for lhd with the same four calls the test apps use. Streams a fixed Arabic/English text
at a steady pace so every toolkit sees the same load. Usage: mock_lhd.py PORT [chars_per_second]"""
import json, sys, time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 7999
CPS = float(sys.argv[2]) if len(sys.argv) > 2 else 40
TEXT = ("مرحبا بالعالم! هذا اختبار للنص العربي المتصل من اليمين إلى اليسار.\n"
        "Mixed line: النص العربي مع English داخل نفس السطر، ثم رقم 12345 وكلمة LastHarness في النهاية.\n"
        "الحروف المتصلة: كتاب، مكتبة، يكتبون، للّه. Plain English follows, then Arabic again: السلام عليكم ورحمة الله.\n") * 12

class H(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    def log_message(self, *a): pass
    def _json(self, obj, code=200):
        b = json.dumps(obj, ensure_ascii=False).encode()
        self.send_response(code); self.send_header("Content-Type", "application/json"); self.send_header("Content-Length", str(len(b))); self.end_headers(); self.wfile.write(b)
    def do_GET(self):
        if self.path == "/v1/models":
            self._json({"data": [{"id": n} for n in ["LFM2.5-2.6B-fast", "نموذج عربي", "Qwen3.6-35B", "Tiel-Coder", "Ornith"]]})
        else:
            self._json({"ok": True})
    def do_POST(self):
        n = int(self.headers.get("Content-Length", 0)); self.rfile.read(n)
        if self.path == "/v1/sessions":
            return self._json({"id": "mock"})
        if self.path.endswith("/turns"):
            self.send_response(200); self.send_header("Content-Type", "text/event-stream"); self.send_header("Transfer-Encoding", "chunked"); self.end_headers()
            def chunk(s):
                b = s.encode(); self.wfile.write(f"{len(b):x}\r\n".encode() + b + b"\r\n"); self.wfile.flush()
            step = max(1, int(CPS / 10))
            try:
                for i in range(0, len(TEXT), step):
                    chunk("event: text\ndata: " + json.dumps({"type": "text", "text": TEXT[i:i+step]}, ensure_ascii=False) + "\n\n")
                    time.sleep(0.1)
                chunk("event: done\ndata: {}\n\n"); self.wfile.write(b"0\r\n\r\n")
            except (BrokenPipeError, ConnectionResetError):
                pass
            return
        self._json({"ok": True})

ThreadingHTTPServer(("127.0.0.1", PORT), H).serve_forever()
