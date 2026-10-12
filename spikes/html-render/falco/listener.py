import http.server, socketserver, sys, threading
port = int(sys.argv[1]); log = open(sys.argv[2], "a", buffering=1)
class H(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        log.write(f"HTTP {self.command} {self.path} UA={self.headers.get('User-Agent')} UPGRADE={self.headers.get('Upgrade')}\n")
        self.send_response(404); self.end_headers()
    do_POST = do_GET
    def log_message(self, *a): pass
class S(socketserver.ThreadingTCPServer):
    allow_reuse_address = True
S(("127.0.0.1", port), H).serve_forever()
