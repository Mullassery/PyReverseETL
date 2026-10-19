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
