"""Python connector plugins for PyReverseETL.

The Rust sync engine (``pyreverseetl._core.run_sync``) accepts a fixed set of
built-in connector types. It cannot call back into Python from inside
``execute_sync``, so third-party connectors use a separate, Python-level path:

* ``BaseConnector`` is the plugin interface (``read`` and ``write``).
* ``get_connector(name)`` resolves a connector from the explicit registry or
  from the ``pyreverseetl.connectors`` entry-point group.
* ``run_with_connector(...)`` reads through a source connector, applies the
  same compliance actions as the core engine (mask/remove/truncate; ``encrypt``
  is reported as unresolved, never faked), and writes through a destination
  connector.

This path does not use the Rust ``run_sync`` engine or its lineage store. That
boundary is deliberate: the core engine remains the path for Postgres, MySQL,
S3, webhook, and CRM destinations; Python plugins are for local/custom
connectors. A connector plugin declares itself with::

    [project.entry-points."pyreverseetl.connectors"]
    my_connector = "my_package.connectors:MyConnector"

The entry point may be a ``BaseConnector`` subclass or a zero-argument factory
returning one. Bundled implementations are ``SQLiteConnector`` (stdlib
``sqlite3``), ``DuckDBConnector`` (optional ``duckdb``), and
``PostgresConnector`` (optional ``psycopg`` skeleton for a local server). The
optional drivers are not installed by default; using the connector without its
driver raises a clear install hint.
"""

from __future__ import annotations

import json
import re
import sqlite3
import time
import uuid
from abc import ABC, abstractmethod
from collections.abc import Iterable, Iterator, Mapping, Sequence
from contextlib import contextmanager
from dataclasses import dataclass, field
from importlib import metadata
from pathlib import Path
from typing import Any

ENTRY_POINT_GROUP = "pyreverseetl.connectors"

_IDENTIFIER = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")


class ConnectorError(RuntimeError):
    """A connector could not be resolved, configured, or used."""


@dataclass
class ConnectorSyncResult:
    """The real result of a Python-plugin sync."""

    run_id: str
    source: str
    destination: str
    rows_read: int
    rows_written: int
    rows_failed: int
    compliance_violations: list[str] = field(default_factory=list)
    dry_run: bool = False
    dry_run_preview: list[str] = field(default_factory=list)
    started_at: str = ""
    completed_at: str = ""
    duration_ms: int = 0


class BaseConnector(ABC):
    """Source/destination plugin interface.

    ``read`` returns one mapping per record. ``write`` returns the number of
    records actually written and must raise on failure rather than returning a
    fabricated count.
    """

    name: str = ""

    @abstractmethod
    def read(
        self, config: Mapping[str, Any], limit: int | None = None
    ) -> list[dict[str, Any]]:
        """Read real records using ``config``."""

    @abstractmethod
    def write(
        self, config: Mapping[str, Any], rows: Sequence[Mapping[str, Any]]
    ) -> int:
        """Write real records using ``config`` and return the written count."""

    def test_connection(self, config: Mapping[str, Any]) -> bool:
        """Return True when the connector can open a real connection."""
        self.read(config, limit=1)
        return True


def _identifier(value: Any, field_name: str) -> str:
    if not isinstance(value, str) or not _IDENTIFIER.match(value):
        raise ConnectorError(
            f"invalid {field_name} {value!r}: expected [A-Za-z_][A-Za-z0-9_]*"
        )
    return value


def _quote(identifier: str) -> str:
    return '"' + identifier.replace('"', '""') + '"'


def _database_path(config: Mapping[str, Any], connector: str = "sqlite") -> str:
    path = config.get("path", config.get("database"))
    if not isinstance(path, str) or not path:
        raise ConnectorError(
            f"{connector} connector requires a 'path' (or 'database')"
        )
    return path


def _row_columns(rows: Sequence[Mapping[str, Any]]) -> list[str]:
    columns: list[str] = []
    for row in rows:
        for key in row:
            if not isinstance(key, str):
                raise ConnectorError(f"sqlite column names must be strings, got {key!r}")
            if key not in columns:
                columns.append(key)
    return columns


def _sqlite_value(value: Any) -> Any:
    if isinstance(value, (dict, list, tuple)):
        return json.dumps(value, sort_keys=True)
    if isinstance(value, bool):
        return int(value)
    if isinstance(value, (str, bytes, int, float)) or value is None:
        return value
    return json.dumps(value, default=str)


def _sqlite_type(values: Iterable[Any]) -> str:
    seen = [value for value in values if value is not None]
    if not seen:
        return "TEXT"
    if all(isinstance(value, (bool, int)) for value in seen):
        return "INTEGER"
    if all(isinstance(value, (bool, int, float)) for value in seen):
        return "REAL"
    if all(isinstance(value, bytes) for value in seen):
        return "BLOB"
    return "TEXT"


class SQLiteConnector(BaseConnector):
    """Read from and write to a real local SQLite database.

    Read config::

        {"path": "/tmp/source.db", "table": "customers"}

    Write config::

        {"path": "/tmp/destination.db", "table": "customers",
         "if_exists": "append" | "replace" | "fail",
         "upsert_key": "id"}

    ``upsert_key`` may be a string or a list of strings. A table created with
    an upsert key gets a composite primary key; upserting into a pre-existing
    table requires a matching unique/primary key constraint.
    """

    name = "sqlite"

    def read(
        self, config: Mapping[str, Any], limit: int | None = None
    ) -> list[dict[str, Any]]:
        table = _identifier(config.get("table"), "table")
        if limit is not None and limit < 0:
            raise ConnectorError("limit must be >= 0")
        database_path = _database_path(config)
        if not Path(database_path).is_file():
            raise ConnectorError(f"sqlite database not found: {database_path}")
        query = f"SELECT * FROM {_quote(table)}"
        params: tuple[Any, ...] = ()
        if limit is not None:
            query += " LIMIT ?"
            params = (int(limit),)
        try:
            with sqlite3.connect(database_path) as connection:
                connection.row_factory = sqlite3.Row
                cursor = connection.execute(query, params)
                return [dict(row) for row in cursor.fetchall()]
        except sqlite3.Error as exc:
            raise ConnectorError(f"sqlite read failed: {exc}") from exc

    def write(
        self, config: Mapping[str, Any], rows: Sequence[Mapping[str, Any]]
    ) -> int:
        if not rows:
            return 0
        table = _identifier(config.get("table"), "table")
        columns = _row_columns(rows)
        if not columns:
            raise ConnectorError("cannot write rows with no columns")
        for column in columns:
            _identifier(column, "column")

        if_exists = config.get("if_exists", "append")
        if if_exists not in ("append", "replace", "fail"):
            raise ConnectorError("if_exists must be 'append', 'replace', or 'fail'")

        key_config = config.get("upsert_key")
        if key_config is None:
            key_columns: list[str] = []
        elif isinstance(key_config, str):
            key_columns = [_identifier(key_config, "upsert_key")]
        else:
            key_columns = [_identifier(key, "upsert_key") for key in key_config]
        for key in key_columns:
            if key not in columns:
                raise ConnectorError(f"upsert_key {key!r} is not a column in the rows")

        try:
            with sqlite3.connect(_database_path(config)) as connection:
                exists = (
                    connection.execute(
                        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?",
                        (table,),
                    ).fetchone()
                    is not None
                )
                if exists and if_exists == "fail":
                    raise ConnectorError(f"table {table!r} already exists")
                if exists and if_exists == "replace":
                    connection.execute(f"DROP TABLE {_quote(table)}")
                    exists = False

                if not exists:
                    definitions = []
                    for column in columns:
                        column_type = _sqlite_type(row.get(column) for row in rows)
                        definitions.append(f"{_quote(column)} {column_type}")
                    if key_columns:
                        keys = ", ".join(_quote(key) for key in key_columns)
                        definitions.append(f"PRIMARY KEY ({keys})")
                    connection.execute(
                        f"CREATE TABLE {_quote(table)} ({', '.join(definitions)})"
                    )

                placeholders = ", ".join("?" for _ in columns)
                column_list = ", ".join(_quote(column) for column in columns)
                values = [
                    tuple(_sqlite_value(row.get(column)) for column in columns)
                    for row in rows
                ]

                if key_columns:
                    updates = [column for column in columns if column not in key_columns]
                    conflict = ", ".join(_quote(key) for key in key_columns)
                    if updates:
                        assignments = ", ".join(
                            f"{_quote(column)} = excluded.{_quote(column)}"
                            for column in updates
                        )
                        statement = (
                            f"INSERT INTO {_quote(table)} ({column_list}) VALUES "
                            f"({placeholders}) ON CONFLICT ({conflict}) DO UPDATE SET "
                            f"{assignments}"
                        )
                    else:
                        statement = (
                            f"INSERT INTO {_quote(table)} ({column_list}) VALUES "
                            f"({placeholders}) ON CONFLICT ({conflict}) DO NOTHING"
                        )
                else:
                    statement = (
                        f"INSERT INTO {_quote(table)} ({column_list}) VALUES "
                        f"({placeholders})"
                    )

                connection.executemany(statement, values)
            return len(rows)
        except ConnectorError:
            raise
        except sqlite3.Error as exc:
            raise ConnectorError(f"sqlite write failed: {exc}") from exc


def _import_duckdb() -> Any:
    try:
        import duckdb
    except ImportError as exc:
        raise ConnectorError(
            "duckdb connector requires the optional 'duckdb' package; install "
            "it with: pip install \"PyReverseETL[duckdb]\""
        ) from exc
    return duckdb


def _duckdb_value(value: Any) -> Any:
    if isinstance(value, (dict, list, tuple)):
        return json.dumps(value, sort_keys=True)
    return value


def _duckdb_type(values: Iterable[Any]) -> str:
    seen = [value for value in values if value is not None]
    if not seen:
        return "VARCHAR"
    if all(isinstance(value, bool) for value in seen):
        return "BOOLEAN"
    if all(isinstance(value, int) and not isinstance(value, bool) for value in seen):
        return "BIGINT"
    if all(
        isinstance(value, (int, float)) and not isinstance(value, bool)
        for value in seen
    ):
        return "DOUBLE"
    if all(isinstance(value, bytes) for value in seen):
        return "BLOB"
    if all(isinstance(value, str) for value in seen):
        return "VARCHAR"
    return "VARCHAR"


class DuckDBConnector(BaseConnector):
    """Read from and write to a real local DuckDB database.

    DuckDB is an optional dependency (``pip install "PyReverseETL[duckdb]"``).
    The configuration mirrors ``SQLiteConnector``::

        read:  {"path": "/tmp/source.duckdb", "table": "customers"}
        write: {"path": "/tmp/destination.duckdb", "table": "customers",
                "if_exists": "append" | "replace" | "fail",
                "upsert_key": "id"}

    ``upsert_key`` may be a string or a list of strings. A table created with
    an upsert key gets a composite primary key; upserting into a pre-existing
    table requires a matching unique/primary key constraint. This is the
    Python plugin path, not the Rust ``run_sync`` engine.
    """

    name = "duckdb"

    def read(
        self, config: Mapping[str, Any], limit: int | None = None
    ) -> list[dict[str, Any]]:
        table = _identifier(config.get("table"), "table")
        if limit is not None and limit < 0:
            raise ConnectorError("limit must be >= 0")
        database_path = _database_path(config, "duckdb")
        if not Path(database_path).is_file():
            raise ConnectorError(f"duckdb database not found: {database_path}")
        duckdb = _import_duckdb()
        query = f"SELECT * FROM {_quote(table)}"
        params: tuple[Any, ...] = ()
        if limit is not None:
            query += " LIMIT ?"
            params = (int(limit),)
        try:
            connection = duckdb.connect(database_path, read_only=True)
            try:
                cursor = connection.execute(query, params)
                columns = [column[0] for column in cursor.description]
                return [
                    dict(zip(columns, row)) for row in cursor.fetchall()
                ]
            finally:
                connection.close()
        except duckdb.Error as exc:
            raise ConnectorError(f"duckdb read failed: {exc}") from exc

    def write(
        self, config: Mapping[str, Any], rows: Sequence[Mapping[str, Any]]
    ) -> int:
        if not rows:
            return 0
        table = _identifier(config.get("table"), "table")
        columns = _row_columns(rows)
        if not columns:
            raise ConnectorError("cannot write rows with no columns")
        for column in columns:
            _identifier(column, "column")

        if_exists = config.get("if_exists", "append")
        if if_exists not in ("append", "replace", "fail"):
            raise ConnectorError("if_exists must be 'append', 'replace', or 'fail'")

        key_config = config.get("upsert_key")
        if key_config is None:
            key_columns: list[str] = []
        elif isinstance(key_config, str):
            key_columns = [_identifier(key_config, "upsert_key")]
        else:
            key_columns = [_identifier(key, "upsert_key") for key in key_config]
        for key in key_columns:
            if key not in columns:
                raise ConnectorError(f"upsert_key {key!r} is not a column in the rows")

        database_path = _database_path(config, "duckdb")
        duckdb = _import_duckdb()
        try:
            connection = duckdb.connect(database_path)
            try:
                exists = (
                    connection.execute(
                        "SELECT 1 FROM information_schema.tables "
                        "WHERE table_schema = 'main' AND table_name = ?",
                        (table,),
                    ).fetchone()
                    is not None
                )
                if exists and if_exists == "fail":
                    raise ConnectorError(f"table {table!r} already exists")
                if exists and if_exists == "replace":
                    connection.execute(f"DROP TABLE {_quote(table)}")
                    exists = False

                if not exists:
                    definitions = []
                    for column in columns:
                        column_type = _duckdb_type(row.get(column) for row in rows)
                        definitions.append(f"{_quote(column)} {column_type}")
                    if key_columns:
                        keys = ", ".join(_quote(key) for key in key_columns)
                        definitions.append(f"PRIMARY KEY ({keys})")
                    connection.execute(
                        f"CREATE TABLE {_quote(table)} ({', '.join(definitions)})"
                    )

                placeholders = ", ".join("?" for _ in columns)
                column_list = ", ".join(_quote(column) for column in columns)
                values = [
                    tuple(_duckdb_value(row.get(column)) for column in columns)
                    for row in rows
                ]

                if key_columns:
                    updates = [column for column in columns if column not in key_columns]
                    conflict = ", ".join(_quote(key) for key in key_columns)
                    if updates:
                        assignments = ", ".join(
                            f"{_quote(column)} = excluded.{_quote(column)}"
                            for column in updates
                        )
                        statement = (
                            f"INSERT INTO {_quote(table)} ({column_list}) VALUES "
                            f"({placeholders}) ON CONFLICT ({conflict}) DO UPDATE SET "
                            f"{assignments}"
                        )
                    else:
                        statement = (
                            f"INSERT INTO {_quote(table)} ({column_list}) VALUES "
                            f"({placeholders}) ON CONFLICT ({conflict}) DO NOTHING"
                        )
                else:
                    statement = (
                        f"INSERT INTO {_quote(table)} ({column_list}) VALUES "
                        f"({placeholders})"
                    )

                connection.executemany(statement, values)
            finally:
                connection.close()
            return len(rows)
        except ConnectorError:
            raise
        except duckdb.Error as exc:
            raise ConnectorError(f"duckdb write failed: {exc}") from exc


def _import_psycopg() -> Any:
    try:
        import psycopg
    except ImportError as exc:
        raise ConnectorError(
            "postgres connector requires the optional 'psycopg' package; install "
            "it with: pip install \"PyReverseETL[postgres]\""
        ) from exc
    return psycopg


def _postgres_connect_kwargs(config: Mapping[str, Any]) -> dict[str, Any]:
    dsn = config.get("dsn", config.get("database_url"))
    if dsn is not None:
        if not isinstance(dsn, str) or not dsn:
            raise ConnectorError("postgres 'dsn' must be a non-empty string")
        return {"conninfo": dsn}
    kwargs: dict[str, Any] = {
        "host": config.get("host", "localhost"),
        "port": config.get("port", 5432),
        "connect_timeout": config.get("connect_timeout", 5),
    }
    database = config.get("dbname", config.get("database"))
    if database:
        kwargs["dbname"] = database
    user = config.get("user", config.get("username"))
    if user:
        kwargs["user"] = user
    if config.get("password"):
        kwargs["password"] = config["password"]
    if config.get("sslmode"):
        kwargs["sslmode"] = config["sslmode"]
    return kwargs


@contextmanager
def _postgres_connection(
    config: Mapping[str, Any], *, read_only: bool
) -> Iterator[Any]:
    psycopg = _import_psycopg()
    kwargs = _postgres_connect_kwargs(config)
    try:
        connection = psycopg.connect(**kwargs)
    except psycopg.Error as exc:
        raise ConnectorError(f"postgres connection failed: {exc}") from exc
    try:
        connection.read_only = read_only
        yield connection
    finally:
        connection.close()


def _postgres_value(value: Any) -> Any:
    if isinstance(value, (dict, list, tuple)):
        return json.dumps(value, sort_keys=True)
    return value


def _postgres_type(values: Iterable[Any]) -> str:
    seen = [value for value in values if value is not None]
    if not seen:
        return "TEXT"
    if all(isinstance(value, bool) for value in seen):
        return "BOOLEAN"
    if all(isinstance(value, int) and not isinstance(value, bool) for value in seen):
        return "BIGINT"
    if all(
        isinstance(value, (int, float)) and not isinstance(value, bool)
        for value in seen
    ):
        return "DOUBLE PRECISION"
    if all(isinstance(value, bytes) for value in seen):
        return "BYTEA"
    return "TEXT"


class PostgresConnector(BaseConnector):
    """Skeleton source/destination for a local Postgres server via ``psycopg``.

    No driver is bundled and no live-warehouse support is claimed. Install the
    optional driver with ``pip install "PyReverseETL[postgres]"``. Read config
    requires ``table`` plus either ``dsn``/``database_url`` or individual
    connection fields. Write config additionally supports ``if_exists`` and
    ``upsert_key`` like the other bundled connectors.
    """

    name = "postgres"

    def read(
        self, config: Mapping[str, Any], limit: int | None = None
    ) -> list[dict[str, Any]]:
        table = _identifier(config.get("table"), "table")
        if limit is not None and limit < 0:
            raise ConnectorError("limit must be >= 0")
        query = f"SELECT * FROM {_quote(table)}"
        params: tuple[Any, ...] = ()
        if limit is not None:
            query += " LIMIT %s"
            params = (int(limit),)
        psycopg = _import_psycopg()
        try:
            with (
                _postgres_connection(config, read_only=True) as connection,
                connection.cursor() as cursor,
            ):
                cursor.execute(query, params)
                columns = [column[0] for column in cursor.description]
                return [dict(zip(columns, row)) for row in cursor.fetchall()]
        except psycopg.Error as exc:
            raise ConnectorError(f"postgres read failed: {exc}") from exc

    def write(
        self, config: Mapping[str, Any], rows: Sequence[Mapping[str, Any]]
    ) -> int:
        if not rows:
            return 0
        table = _identifier(config.get("table"), "table")
        columns = _row_columns(rows)
        if not columns:
            raise ConnectorError("cannot write rows with no columns")
        for column in columns:
            _identifier(column, "column")

        if_exists = config.get("if_exists", "append")
        if if_exists not in ("append", "replace", "fail"):
            raise ConnectorError("if_exists must be 'append', 'replace', or 'fail'")

        key_config = config.get("upsert_key")
        if key_config is None:
            key_columns: list[str] = []
        elif isinstance(key_config, str):
            key_columns = [_identifier(key_config, "upsert_key")]
        else:
            key_columns = [_identifier(key, "upsert_key") for key in key_config]
        for key in key_columns:
            if key not in columns:
                raise ConnectorError(f"upsert_key {key!r} is not a column in the rows")

        psycopg = _import_psycopg()
        try:
            with _postgres_connection(config, read_only=False) as connection:
                with connection.cursor() as cursor:
                    cursor.execute(
                        "SELECT 1 FROM information_schema.tables "
                        "WHERE table_schema = 'public' AND table_name = %s",
                        (table,),
                    )
                    exists = cursor.fetchone() is not None
                    if exists and if_exists == "fail":
                        raise ConnectorError(f"table {table!r} already exists")
                    if exists and if_exists == "replace":
                        cursor.execute(f"DROP TABLE {_quote(table)}")
                        exists = False

                    if not exists:
                        definitions = []
                        for column in columns:
                            column_type = _postgres_type(
                                row.get(column) for row in rows
                            )
                            definitions.append(f"{_quote(column)} {column_type}")
                        if key_columns:
                            keys = ", ".join(_quote(key) for key in key_columns)
                            definitions.append(f"PRIMARY KEY ({keys})")
                        cursor.execute(
                            f"CREATE TABLE {_quote(table)} "
                            f"({', '.join(definitions)})"
                        )

                    placeholders = ", ".join("%s" for _ in columns)
                    column_list = ", ".join(_quote(column) for column in columns)
                    values = [
                        tuple(
                            _postgres_value(row.get(column)) for column in columns
                        )
                        for row in rows
                    ]

                    if key_columns:
                        updates = [
                            column for column in columns if column not in key_columns
                        ]
                        conflict = ", ".join(_quote(key) for key in key_columns)
                        if updates:
                            assignments = ", ".join(
                                f"{_quote(column)} = excluded.{_quote(column)}"
                                for column in updates
                            )
                            statement = (
                                f"INSERT INTO {_quote(table)} ({column_list}) "
                                f"VALUES ({placeholders}) ON CONFLICT ({conflict}) "
                                f"DO UPDATE SET {assignments}"
                            )
                        else:
                            statement = (
                                f"INSERT INTO {_quote(table)} ({column_list}) "
                                f"VALUES ({placeholders}) ON CONFLICT ({conflict}) "
                                f"DO NOTHING"
                            )
                    else:
                        statement = (
                            f"INSERT INTO {_quote(table)} ({column_list}) "
                            f"VALUES ({placeholders})"
                        )

                    cursor.executemany(statement, values)
                connection.commit()
            return len(rows)
        except ConnectorError:
            raise
        except psycopg.Error as exc:
            raise ConnectorError(f"postgres write failed: {exc}") from exc


_REGISTRY: dict[str, Any] = {
    "sqlite": SQLiteConnector,
    "duckdb": DuckDBConnector,
    "postgres": PostgresConnector,
}


def register_connector(name: str, factory: Any) -> None:
    """Register a connector class or zero-argument factory under ``name``."""
    if not isinstance(name, str) or not name:
        raise ConnectorError("connector name must be a non-empty string")
    if not (isinstance(factory, BaseConnector) or callable(factory)):
        raise ConnectorError("connector factory must be a BaseConnector or callable")
    _REGISTRY[name] = factory


def _entry_points() -> list[Any]:
    discovered = metadata.entry_points()
    if hasattr(discovered, "select"):
        return list(discovered.select(group=ENTRY_POINT_GROUP))
    return list(discovered.get(ENTRY_POINT_GROUP, []))


def _instantiate(obj: Any) -> BaseConnector:
    if isinstance(obj, BaseConnector):
        return obj
    if isinstance(obj, type) and issubclass(obj, BaseConnector):
        return obj()
    if callable(obj):
        candidate = obj()
        if isinstance(candidate, BaseConnector):
            return candidate
    raise ConnectorError(
        "connector entry point must be a BaseConnector subclass, an instance, "
        "or a zero-argument factory returning a BaseConnector"
    )


def get_connector(name: str) -> BaseConnector:
    """Resolve a connector by name from the registry or entry points."""
    if name in _REGISTRY:
        return _instantiate(_REGISTRY[name])
    for entry_point in _entry_points():
        if entry_point.name == name:
            try:
                return _instantiate(entry_point.load())
            except Exception as exc:
                raise ConnectorError(
                    f"failed to load connector plugin {name!r}: {exc}"
                ) from exc
    known = ", ".join(sorted(available_connectors())) or "(none)"
    raise ConnectorError(f"unknown connector {name!r}; available: {known}")


def available_connectors() -> dict[str, str]:
    """Return ``{name: origin}`` for built-in and discoverable plugins."""
    available = {
        name: getattr(factory, "__module__", type(factory).__name__)
        for name, factory in _REGISTRY.items()
    }
    for entry_point in _entry_points():
        available.setdefault(entry_point.name, entry_point.value)
    return available


def _apply_compliance_rules(
    records: Sequence[Mapping[str, Any]], rules: Sequence[Mapping[str, Any]] | None
) -> tuple[list[dict[str, Any]], list[str]]:
    violations: list[str] = []
    prepared: list[dict[str, Any]] = []
    for record in records:
        row = dict(record)
        for rule in rules or []:
            rule_id = rule.get("id", "<unnamed>")
            action = rule.get("action") or {}
            action_type = action.get("type")
            for field_name in rule.get("target_fields", []):
                if field_name not in row:
                    continue
                if action_type == "mask":
                    row[field_name] = action.get("pattern", "****")
                elif action_type == "remove":
                    row.pop(field_name, None)
                elif action_type == "truncate":
                    length = int(action.get("length", 10))
                    value = row[field_name]
                    row[field_name] = (
                        value[:length] if isinstance(value, str) else str(value)[:length]
                    )
                elif action_type == "encrypt":
                    violations.append(
                        f"rule {rule_id!r}: field {field_name!r} requires "
                        "encryption, which is not implemented"
                    )
                else:
                    raise ConnectorError(
                        f"rule {rule_id!r}: unknown action type {action_type!r}"
                    )
        prepared.append(row)
    return prepared, violations


def run_with_connector(
    source: str | BaseConnector,
    source_config: Mapping[str, Any],
    destination: str | BaseConnector,
    destination_config: Mapping[str, Any],
    *,
    limit: int | None = None,
    compliance_rules: Sequence[Mapping[str, Any]] | None = None,
    dry_run: bool = False,
) -> ConnectorSyncResult:
    """Read through a source plugin, mask/remove/truncate, write through a destination.

    This is the Python-plugin path. It is separate from ``run_sync`` and does
    not write the Rust lineage graph. A dry run still reads and applies
    compliance rules, but never calls the destination connector's ``write``.
    """
    source_connector = get_connector(source) if isinstance(source, str) else source
    destination_connector = (
        get_connector(destination) if isinstance(destination, str) else destination
    )
    source_name = getattr(source_connector, "name", None) or source
    destination_name = getattr(destination_connector, "name", None) or destination

    started_at = time.time()
    rows = source_connector.read(source_config, limit=limit)
    rows, violations = _apply_compliance_rules(rows, compliance_rules)

    if dry_run:
        rows_written = 0
        preview = [json.dumps(row, default=str, sort_keys=True) for row in rows]
    else:
        rows_written = destination_connector.write(destination_config, rows)
        preview = []
    completed_at = time.time()

    return ConnectorSyncResult(
        run_id=str(uuid.uuid4()),
        source=str(source_name),
        destination=str(destination_name),
        rows_read=len(rows),
        rows_written=rows_written,
        rows_failed=max(0, len(rows) - rows_written),
        compliance_violations=violations,
        dry_run=dry_run,
        dry_run_preview=preview,
        started_at=time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(started_at)),
        completed_at=time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(completed_at)),
        duration_ms=int((completed_at - started_at) * 1000),
    )


__all__ = [
    "ENTRY_POINT_GROUP",
    "BaseConnector",
    "ConnectorError",
    "ConnectorSyncResult",
    "DuckDBConnector",
    "PostgresConnector",
    "SQLiteConnector",
    "available_connectors",
    "get_connector",
    "register_connector",
    "run_with_connector",
]
