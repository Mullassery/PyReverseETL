"""Sample-row StatGuardian gate tests (stdlib HTTP server, no engine)."""

import json
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

from pyreverseetl.cli import validate_activation_rows


class _Handler(BaseHTTPRequestHandler):
    passed = True

    def log_message(self, *_args):
        pass

    def do_POST(self):
        length = int(self.headers.get("Content-Length", "0"))
        self.rfile.read(length)
        payload = {
            "passed": self.passed,
            "quality_score": 1.0 if self.passed else 0.0,
            "issues": [] if self.passed else ["amount < 0"],
            "schema_version": "test",
        }
        body = json.dumps(payload).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


def _serve(passed: bool):
    _Handler.passed = passed
    server = ThreadingHTTPServer(("127.0.0.1", 0), _Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    return server, f"http://127.0.0.1:{server.server_address[1]}"


def test_passing_rows_allow_activation():
    server, url = _serve(True)
    try:
        allowed, detail = validate_activation_rows([{"amount": 1}], url)
        assert allowed is True
        assert detail["checked"] == 1
    finally:
        server.shutdown()


def test_failing_row_blocks_activation():
    server, url = _serve(False)
    try:
        allowed, detail = validate_activation_rows([{"amount": -1}], url)
        assert allowed is False
        assert detail["issues"] == ["amount < 0"]
    finally:
        server.shutdown()


def test_unreachable_service_fails_closed():
    allowed, detail = validate_activation_rows([{"amount": 1}], "http://127.0.0.1:1")
    assert allowed is False
    assert "failed" in detail["reason"]
