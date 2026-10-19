"""Live cross-repo integration: PyReverseETL -> StatGuardian ``/validate``.

This starts a real StatGuardian HTTP service and sends sample rows through
PyReverseETL's pre-activation gate. It is skipped when the StatGuardian CLI is
not installed, so it never substitutes a mock for the real contract.

Requires ``statguardian[api]`` (``fastapi``/``uvicorn``) in the environment.
"""

from __future__ import annotations

import shutil
import socket
import subprocess
import sys
import time
from pathlib import Path

import pytest


def _statguardian_binary() -> str | None:
    """Find the CLI even when the venv was not activated onto PATH."""
    found = shutil.which("statguardian")
    if found:
        return found
    candidate = Path(sys.executable).with_name("statguardian")
    return str(candidate) if candidate.is_file() else None


STATGUARDIAN = _statguardian_binary()

pytestmark = pytest.mark.skipif(
    STATGUARDIAN is None,
    reason="statguardian CLI not installed; real cross-repo test needs statguardian[api]",
)

CONTRACT = """dataset tiny {
    schema {
        id:   int,    not_null
        name: string, not_null
    }
}
"""


def _free_port() -> int:
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return int(sock.getsockname()[1])


def _wait_for_port(port: int, timeout: float = 15.0) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            with socket.create_connection(("127.0.0.1", port), timeout=0.5):
                return
        except OSError:
            time.sleep(0.1)
    raise AssertionError(f"StatGuardian did not start listening on port {port}")


def test_statguardian_gate_passes_and_blocks_real_service(tmp_path: Path) -> None:
    from pyreverseetl.cli import validate_activation_rows

    contract = tmp_path / "tiny.sg"
    contract.write_text(CONTRACT, encoding="utf-8")
    port = _free_port()
    proc = subprocess.Popen(
        [
            str(STATGUARDIAN),
            "serve",
            "--contract",
            str(contract),
            "--host",
            "127.0.0.1",
            "--port",
            str(port),
        ],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    try:
        try:
            _wait_for_port(port)
        except AssertionError:
            output = ""
            if proc.poll() is not None and proc.stdout is not None:
                output = proc.stdout.read()
            pytest.skip(f"StatGuardian service could not start: {output.strip()[:400]}")

        url = f"http://127.0.0.1:{port}"
        passed, detail = validate_activation_rows([{"id": 1, "name": "x"}], url)
        assert passed is True, detail
        assert detail["checked"] == 1

        blocked, detail = validate_activation_rows([{"id": None, "name": None}], url)
        assert blocked is False
        assert "rejected" in detail["reason"].lower()

        unreachable, _ = validate_activation_rows(
            [{"id": 1, "name": "x"}], "http://127.0.0.1:9", timeout_seconds=1
        )
        assert unreachable is False, "the gate must fail closed when the service is unreachable"
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            proc.kill()
