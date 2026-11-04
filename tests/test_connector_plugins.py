"""Tests for the Python connector plugin path and the bundled SQLite connector.

These tests use real temporary SQLite databases (stdlib ``sqlite3``); no cloud
service or network access is involved.
"""

from __future__ import annotations

import json
import sqlite3

import pytest
from pyreverseetl.connectors import (
    ENTRY_POINT_GROUP,
    BaseConnector,
    ConnectorError,
    SQLiteConnector,
    available_connectors,
    get_connector,
    register_connector,
    run_with_connector,
)


def _make_source(path, rows) -> None:
    with sqlite3.connect(path) as connection:
        connection.execute(
            "CREATE TABLE customers (id INTEGER PRIMARY KEY, name TEXT, email TEXT)"
        )
        connection.executemany(
            "INSERT INTO customers (id, name, email) VALUES (?, ?, ?)", rows
        )


def _read_table(path, table):
    with sqlite3.connect(path) as connection:
        connection.row_factory = sqlite3.Row
        return [dict(row) for row in connection.execute(f'SELECT * FROM "{table}"')]


def test_sqlite_is_a_builtin_connector():
    assert "sqlite" in available_connectors()
    assert isinstance(get_connector("sqlite"), SQLiteConnector)
    assert ENTRY_POINT_GROUP == "pyreverseetl.connectors"


def test_sqlite_reads_real_rows_with_limit(tmp_path):
    database = tmp_path / "source.db"
    _make_source(
        database, [(1, "Alice", "alice@example.com"), (2, "Bob", "bob@example.com")]
    )

    connector = SQLiteConnector()
    rows = connector.read({"path": str(database), "table": "customers"}, limit=1)

    assert rows == [{"id": 1, "name": "Alice", "email": "alice@example.com"}]


def test_run_with_connector_copies_sqlite_to_sqlite(tmp_path):
    source = tmp_path / "source.db"
    destination = tmp_path / "destination.db"
    _make_source(source, [(1, "Alice", "alice@example.com"), (2, "Bob", "bob@example.com")])

    result = run_with_connector(
        "sqlite",
        {"path": str(source), "table": "customers"},
        "sqlite",
        {"path": str(destination), "table": "customers_copy"},
    )

    assert result.rows_read == 2
    assert result.rows_written == 2
    assert result.rows_failed == 0
    assert result.source == "sqlite"
    assert result.destination == "sqlite"
    assert _read_table(destination, "customers_copy") == [
        {"id": 1, "name": "Alice", "email": "alice@example.com"},
        {"id": 2, "name": "Bob", "email": "bob@example.com"},
    ]


def test_dry_run_reads_but_never_writes(tmp_path):
    source = tmp_path / "source.db"
    destination = tmp_path / "destination.db"
    _make_source(source, [(1, "Alice", "alice@example.com")])

    result = run_with_connector(
        "sqlite",
        {"path": str(source), "table": "customers"},
        "sqlite",
        {"path": str(destination), "table": "should_not_exist"},
        dry_run=True,
    )

    assert result.rows_read == 1
    assert result.rows_written == 0
    assert result.dry_run is True
    assert json.loads(result.dry_run_preview[0])["name"] == "Alice"
    with sqlite3.connect(destination) as connection:
        existing = connection.execute(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?",
            ("should_not_exist",),
        ).fetchone()
    assert existing is None


def test_compliance_actions_apply_before_write(tmp_path):
    source = tmp_path / "source.db"
    destination = tmp_path / "destination.db"
    with sqlite3.connect(source) as connection:
        connection.execute(
            "CREATE TABLE customers (id INTEGER, name TEXT, email TEXT, ssn TEXT, token TEXT)"
        )
        connection.execute(
            "INSERT INTO customers VALUES (?, ?, ?, ?, ?)",
            (1, "Alexandra", "alice@example.com", "123-45-6789", "secret"),
        )

    result = run_with_connector(
        "sqlite",
        {"path": str(source), "table": "customers"},
        "sqlite",
        {"path": str(destination), "table": "masked"},
        compliance_rules=[
            {
                "id": "mask_email",
                "rule_type": "pii_masking",
                "target_fields": ["email"],
                "action": {"type": "mask", "pattern": "****"},
            },
            {
                "id": "drop_ssn",
                "rule_type": "pii_masking",
                "target_fields": ["ssn"],
                "action": {"type": "remove"},
            },
            {
                "id": "trim_name",
                "rule_type": "pii_masking",
                "target_fields": ["name"],
                "action": {"type": "truncate", "length": 4},
            },
            {
                "id": "encrypt_token",
                "rule_type": "pii_masking",
                "target_fields": ["token"],
                "action": {"type": "encrypt"},
            },
        ],
    )

    written = _read_table(destination, "masked")
    assert written == [
        {
            "id": 1,
            "name": "Alex",
            "email": "****",
            "token": "secret",
        }
    ]
    assert any("encryption" in violation for violation in result.compliance_violations)


def test_sqlite_upsert_updates_existing_keys(tmp_path):
    source = tmp_path / "source.db"
    destination = tmp_path / "destination.db"
    _make_source(source, [(1, "Alice", "alice@example.com")])
    config = {"path": str(destination), "table": "customers", "upsert_key": "id"}

    first = run_with_connector(
        "sqlite", {"path": str(source), "table": "customers"}, "sqlite", config
    )
    assert first.rows_written == 1

    with sqlite3.connect(source) as connection:
        connection.execute("UPDATE customers SET name = 'Alicia' WHERE id = 1")

    second = run_with_connector(
        "sqlite", {"path": str(source), "table": "customers"}, "sqlite", config
    )
    assert second.rows_written == 1
    assert _read_table(destination, "customers") == [
        {"id": 1, "name": "Alicia", "email": "alice@example.com"}
    ]


def test_missing_source_database_is_a_clear_error(tmp_path):
    with pytest.raises(ConnectorError, match="database not found"):
        SQLiteConnector().read(
            {"path": str(tmp_path / "missing.db"), "table": "customers"}
        )


def test_sqlite_rejects_injected_identifier(tmp_path):
    database = tmp_path / "source.db"
    _make_source(database, [(1, "Alice", "alice@example.com")])

    with pytest.raises(ConnectorError, match="invalid table"):
        SQLiteConnector().read(
            {"path": str(database), "table": "customers; DROP TABLE customers"}
        )


class _MemoryConnector(BaseConnector):
    name = "memory_test"

    def read(self, config, limit=None):
        rows = [dict(row) for row in config.get("rows", [])]
        return rows[:limit] if limit is not None else rows

    def write(self, config, rows):
        config.setdefault("written", []).extend(dict(row) for row in rows)
        return len(rows)


def test_custom_connector_can_be_registered_and_used():
    register_connector("memory_test", _MemoryConnector)
    destination_config = {}

    result = run_with_connector(
        "memory_test",
        {"rows": [{"id": 1, "name": "Alice"}]},
        "memory_test",
        destination_config,
    )

    assert result.rows_written == 1
    assert destination_config["written"] == [{"id": 1, "name": "Alice"}]


def test_entry_point_plugins_are_discovered(monkeypatch):
    class EntryPointConnector(BaseConnector):
        name = "from_entry_point"

        def read(self, config, limit=None):
            return [{"ok": True}]

        def write(self, config, rows):
            return len(rows)

    class FakeEntryPoint:
        name = "from_entry_point"
        value = "fake:EntryPointConnector"

        def load(self):
            return EntryPointConnector

    monkeypatch.setattr(
        "pyreverseetl.connectors._entry_points", lambda: [FakeEntryPoint()]
    )

    connector = get_connector("from_entry_point")
    assert isinstance(connector, EntryPointConnector)
    assert available_connectors()["from_entry_point"] == "fake:EntryPointConnector"


def test_unknown_connector_is_an_error():
    with pytest.raises(ConnectorError, match="unknown connector"):
        get_connector("does_not_exist")


# -- WebhookConnector: real HTTP against a local server -----------------------

import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

from pyreverseetl.connectors import WebhookConnector


class _RecordingHandler(BaseHTTPRequestHandler):
    """Programmable local webhook: status codes per request, records everything."""

    behavior: list = []  # list of (status, retry_after, headers_dict_per_call)
    seen: list = []

    def do_POST(self) -> None:  # noqa: N802 - stdlib naming
        length = int(self.headers.get("Content-Length", "0"))
        body = self.rfile.read(length)
        self.seen.append(
            {
                "path": self.path,
                "body": body,
                "signature": self.headers.get("x-voxora-signature"),
                "content_type": self.headers.get("Content-Type"),
            }
        )
        status, retry_after = self.behavior.pop(0) if self.behavior else (200, None)
        self.send_response(status)
        if retry_after:
            self.send_header("Retry-After", str(retry_after))
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(b'{"ok":true}')

    def log_message(self, *args) -> None:  # silence test output
        return


def _start_server(behavior):
    _RecordingHandler.behavior = behavior
    _RecordingHandler.seen = []
    server = ThreadingHTTPServer(("127.0.0.1", 0), _RecordingHandler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    return server


def test_webhook_connector_delivers_exact_body_and_signature(tmp_path):
    server = _start_server([(200, None)])
    try:
        connector = WebhookConnector()
        record = {
            "event_id": "evt_1",
            "body": '{"event_id": "evt_1", "event_type": "voice_session.created"}',
            "signature": "v1=deadbeef",
        }
        written = connector.write(
            {
                "url": f"http://127.0.0.1:{server.server_port}/wh",
                "signature_header": "x-voxora-signature",
                "signature_field": "signature",
                "body_field": "body",
            },
            [record],
        )
        assert written == 1
        seen = _RecordingHandler.seen
        assert len(seen) == 1
        # Exact signed body bytes, never re-serialized.
        assert seen[0]["body"] == record["body"].encode("utf-8")
        assert seen[0]["signature"] == "v1=deadbeef"
        assert seen[0]["content_type"] == "application/json"
    finally:
        server.shutdown()


def test_webhook_connector_retries_429_honoring_retry_after(tmp_path):
    server = _start_server([(429, 1), (200, None)])
    try:
        started = time.monotonic()
        written = WebhookConnector().write(
            {
                "url": f"http://127.0.0.1:{server.server_port}/wh",
                "max_attempts": 3,
                "retry_base_delay": 0.05,
            },
            [{"event_id": "evt_2"}],
        )
        assert written == 1
        assert time.monotonic() - started >= 1.0  # honored the 1s Retry-After
        assert len(_RecordingHandler.seen) == 2
    finally:
        server.shutdown()


def test_webhook_connector_dead_letters_terminal_failures(tmp_path):
    server = _start_server([(500, None), (500, None)])
    dead_letter = tmp_path / "dl.jsonl"
    written = WebhookConnector().write(
        {
            "url": f"http://127.0.0.1:{server.server_port}/wh",
            "max_attempts": 2,
            "retry_base_delay": 0.01,
            "dead_letter_path": str(dead_letter),
        },
        [{"event_id": "evt_3", "body": "x", "signature": "v1=aa"}],
    )
    assert written == 0  # true count: failure is never fabricated as success
    assert len(_RecordingHandler.seen) == 2  # one retry after the first 500
    entry = json.loads(dead_letter.read_text().splitlines()[0])
    assert entry["error"] == "http_500" and entry["attempts"] == 2
    assert entry["record"]["event_id"] == "evt_3"


def test_webhook_connector_rejects_partial_signature_config():
    with pytest.raises(ConnectorError):
        WebhookConnector().write(
            {"url": "http://127.0.0.1:1/wh", "signature_header": "x-voxora-signature"},
            [{"signature": "v1=aa"}],
        )


def test_webhook_connector_requires_url():
    with pytest.raises(ConnectorError):
        WebhookConnector().write({}, [{}])
