"""Tests for the optional DuckDB and Postgres connector plugins.

DuckDB tests run against real temporary database files when the optional
``duckdb`` driver is installed; otherwise they skip with an explicit reason.
Postgres tests require both the optional ``psycopg`` driver and a reachable
local server configured through ``PYREVERSEETL_TEST_POSTGRES_DSN``. Nothing
here fakes a connection or a row count.
"""

from __future__ import annotations

import os
import uuid

import pytest
from pyreverseetl.connectors import (
    ConnectorError,
    DuckDBConnector,
    PostgresConnector,
    available_connectors,
    get_connector,
    run_with_connector,
)


@pytest.fixture
def duckdb():
    return pytest.importorskip(
        "duckdb", reason="optional 'duckdb' driver is not installed"
    )


@pytest.fixture
def source_database(tmp_path, duckdb):
    path = tmp_path / "source.duckdb"
    connection = duckdb.connect(str(path))
    try:
        connection.execute(
            "CREATE TABLE customers "
            "(id INTEGER PRIMARY KEY, name VARCHAR, email VARCHAR)"
        )
        connection.executemany(
            "INSERT INTO customers VALUES (?, ?, ?)",
            [
                (1, "Alice", "alice@example.com"),
                (2, "Bob", "bob@example.com"),
            ],
        )
    finally:
        connection.close()
    return path


def _read_table(duckdb, path, table):
    connection = duckdb.connect(str(path), read_only=True)
    try:
        cursor = connection.execute(f'SELECT * FROM "{table}" ORDER BY id')
        columns = [column[0] for column in cursor.description]
        return [dict(zip(columns, row)) for row in cursor.fetchall()]
    finally:
        connection.close()


def test_connectors_are_registered():
    names = available_connectors()
    assert "duckdb" in names
    assert "postgres" in names
    assert isinstance(get_connector("duckdb"), DuckDBConnector)
    assert isinstance(get_connector("postgres"), PostgresConnector)


def test_duckdb_reads_real_rows_with_limit(source_database):
    rows = DuckDBConnector().read(
        {"path": str(source_database), "table": "customers"}, limit=1
    )

    assert rows == [{"id": 1, "name": "Alice", "email": "alice@example.com"}]


def test_run_with_connector_copies_duckdb_to_duckdb(
    tmp_path, source_database, duckdb
):
    destination = tmp_path / "destination.duckdb"

    result = run_with_connector(
        "duckdb",
        {"path": str(source_database), "table": "customers"},
        "duckdb",
        {"path": str(destination), "table": "customers_copy"},
    )

    assert result.rows_read == 2
    assert result.rows_written == 2
    assert result.rows_failed == 0
    assert _read_table(duckdb, destination, "customers_copy") == [
        {"id": 1, "name": "Alice", "email": "alice@example.com"},
        {"id": 2, "name": "Bob", "email": "bob@example.com"},
    ]


def test_duckdb_upsert_updates_existing_keys(tmp_path, source_database, duckdb):
    destination = tmp_path / "destination.duckdb"
    config = {
        "path": str(destination),
        "table": "customers",
        "upsert_key": "id",
    }

    first = run_with_connector(
        "duckdb",
        {"path": str(source_database), "table": "customers"},
        "duckdb",
        config,
    )
    assert first.rows_written == 2

    connection = duckdb.connect(str(source_database))
    try:
        connection.execute("UPDATE customers SET name = 'Alicia' WHERE id = 1")
    finally:
        connection.close()

    second = run_with_connector(
        "duckdb",
        {"path": str(source_database), "table": "customers"},
        "duckdb",
        config,
    )
    assert second.rows_written == 2
    assert _read_table(duckdb, destination, "customers") == [
        {"id": 1, "name": "Alicia", "email": "alice@example.com"},
        {"id": 2, "name": "Bob", "email": "bob@example.com"},
    ]


def test_duckdb_rejects_injected_identifier(source_database):
    with pytest.raises(ConnectorError, match="invalid table"):
        DuckDBConnector().read(
            {
                "path": str(source_database),
                "table": "customers; DROP TABLE customers",
            }
        )


@pytest.fixture
def postgres_dsn():
    psycopg = pytest.importorskip(
        "psycopg", reason="optional 'psycopg' driver is not installed"
    )
    dsn = os.environ.get("PYREVERSEETL_TEST_POSTGRES_DSN")
    if not dsn:
        pytest.skip(
            "set PYREVERSEETL_TEST_POSTGRES_DSN to a local Postgres server to run "
            "this test"
        )
    try:
        connection = psycopg.connect(dsn, connect_timeout=2)
    except Exception as exc:  # noqa: BLE001 - surface the real reason in the skip
        pytest.skip(f"local Postgres server is not reachable: {exc}")
    else:
        connection.close()
    return dsn


def test_postgres_round_trip_against_a_local_server(postgres_dsn):
    import psycopg

    source_table = f"pyreverseetl_src_{uuid.uuid4().hex}"
    destination_table = f"pyreverseetl_dst_{uuid.uuid4().hex}"
    with (
        psycopg.connect(postgres_dsn) as connection,
        connection.cursor() as cursor,
    ):
        cursor.execute(
            f'CREATE TABLE "{source_table}" '
            "(id INTEGER PRIMARY KEY, name TEXT, email TEXT)"
        )
        cursor.executemany(
            f'INSERT INTO "{source_table}" VALUES (%s, %s, %s)',
            [(1, "Alice", "alice@example.com"), (2, "Bob", "bob@example.com")],
        )

    try:
        result = run_with_connector(
            "postgres",
            {"dsn": postgres_dsn, "table": source_table},
            "postgres",
            {
                "dsn": postgres_dsn,
                "table": destination_table,
                "upsert_key": "id",
            },
        )
        assert result.rows_read == 2
        assert result.rows_written == 2

        rows = PostgresConnector().read(
            {"dsn": postgres_dsn, "table": destination_table}, limit=1
        )
        assert rows == [{"id": 1, "name": "Alice", "email": "alice@example.com"}]
    finally:
        with (
            psycopg.connect(postgres_dsn) as connection,
            connection.cursor() as cursor,
        ):
            cursor.execute(f'DROP TABLE IF EXISTS "{source_table}"')
            cursor.execute(f'DROP TABLE IF EXISTS "{destination_table}"')
