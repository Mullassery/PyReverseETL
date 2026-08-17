"""Real, end-to-end tests of the Python layer driving the Rust sync engine.

These prove the fix for the core audit finding -- "the Python CLI never
actually invoked the Rust sync engine at all" -- by exercising the real path:
`pyreverseetl.run_sync()` (and the CLI, which calls it) against real
Postgres and MySQL databases (via Docker). Nothing here is simulated: rows
are actually written to Postgres, read back from Postgres, written to MySQL,
and verified there via a real `mysql` query -- through the real Rust engine
and the real compliance engine, not an in-process dict simulator.

Postgres<->MySQL (rather than a Python-side mock HTTP server standing in for
a webhook) is used deliberately: the webhook/Salesforce/HubSpot/Marketo
adapters already have their own real-HTTP-request tests against a
purpose-built Rust mock server (`core/src/testing/mock_http.rs`), so this
file's job is specifically to prove the *Python binding* drives the real
engine end to end, which real database connectors do without the added
noise of getting Python's `http.server` to behave under Rust's `reqwest`
connection pooling.

Docker-backed tests are skipped (not failed) if the expected container isn't
reachable, since this environment isn't guaranteed to have them running:

    docker run --rm -d -p 5439:5432 -e POSTGRES_PASSWORD=postgres \\
        -e POSTGRES_DB=pyreverseetl_test --name pyreverseetl-pg-test postgres:16
    docker run --rm -d -p 3307:3306 -e MYSQL_ROOT_PASSWORD=mysql \\
        -e MYSQL_DATABASE=pyreverseetl_test --name pyreverseetl-mysql-test mysql:8
"""

import json
import os
import socket
import subprocess
import sys
import tempfile
import uuid

import pytest

pyreverseetl = pytest.importorskip("pyreverseetl", reason="run `maturin develop` first")

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def _port_open(host: str, port: int) -> bool:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.settimeout(0.5)
        return sock.connect_ex((host, port)) == 0


PG_PORT = int(os.environ.get("PYREVERSEETL_TEST_PG_PORT", "5439"))
MYSQL_PORT = int(os.environ.get("PYREVERSEETL_TEST_MYSQL_PORT", "3307"))
PG_CONTAINER = os.environ.get("PYREVERSEETL_TEST_PG_CONTAINER", "pyreverseetl-pg-test")
MYSQL_CONTAINER = os.environ.get("PYREVERSEETL_TEST_MYSQL_CONTAINER", "pyreverseetl-mysql-test")

requires_postgres_and_mysql = pytest.mark.skipif(
    not (_port_open("127.0.0.1", PG_PORT) and _port_open("127.0.0.1", MYSQL_PORT)),
    reason=(
        f"needs Postgres on 127.0.0.1:{PG_PORT} and MySQL on 127.0.0.1:{MYSQL_PORT} "
        "(see module docstring)"
    ),
)


def _seed_postgres(table: str, rows: list):
    """Seed a Postgres table via `docker exec ... psql` -- avoids adding a
    psycopg2 dependency just for test setup."""
    columns = "id INT PRIMARY KEY, name TEXT, email TEXT, ltv NUMERIC"
    values = ", ".join(
        "({}, '{}', '{}', {})".format(r["id"], r["name"], r["email"], r.get("ltv", 0)) for r in rows
    )
    sql = (
        f'DROP TABLE IF EXISTS "{table}"; '
        f'CREATE TABLE "{table}" ({columns}); '
        f'INSERT INTO "{table}" (id, name, email, ltv) VALUES {values};'
    )
    subprocess.run(
        ["docker", "exec", "-i", PG_CONTAINER, "psql", "-U", "postgres", "-d", "pyreverseetl_test", "-c", sql],
        check=True,
        capture_output=True,
    )


def _drop_mysql_table(table: str):
    subprocess.run(
        [
            "docker", "exec", "-i", MYSQL_CONTAINER, "mysql", "-uroot", "-pmysql", "pyreverseetl_test",
            "-e", f"DROP TABLE IF EXISTS `{table}`;",
        ],
        check=True,
        capture_output=True,
    )


def _create_mysql_table(table: str):
    """The MySQL connector writes into an existing table (matching real
    reverse-ETL usage against an already-provisioned destination schema); it
    deliberately doesn't auto-create tables, so tests must."""
    sql = f"CREATE TABLE `{table}` (id INT PRIMARY KEY, name VARCHAR(255), email VARCHAR(255), ltv DOUBLE);"
    subprocess.run(
        ["docker", "exec", "-i", MYSQL_CONTAINER, "mysql", "-uroot", "-pmysql", "pyreverseetl_test", "-e", sql],
        check=True,
        capture_output=True,
    )


def _query_mysql(table: str) -> str:
    """Return the real rows written to a MySQL table, as tab-separated text
    straight from the `mysql` client -- this is what actually landed in the
    database, not what the test *expects* to be there."""
    result = subprocess.run(
        [
            "docker", "exec", "-i", MYSQL_CONTAINER, "mysql", "-uroot", "-pmysql", "pyreverseetl_test",
            "-e", f"SELECT id, name, email, ltv FROM `{table}` ORDER BY id;",
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout


def _pg_config(table: str, **overrides) -> str:
    config = {
        "host": "127.0.0.1",
        "port": PG_PORT,
        "database": "pyreverseetl_test",
        "username": "postgres",
        "password": "postgres",
        "table": table,
    }
    config.update(overrides)
    return json.dumps(config)


def _mysql_config(table: str, **overrides) -> str:
    config = {
        "host": "127.0.0.1",
        "port": MYSQL_PORT,
        "database": "pyreverseetl_test",
        "username": "root",
        "password": "mysql",
        "table": table,
        "upsert_key": "id",
    }
    config.update(overrides)
    return json.dumps(config)


@requires_postgres_and_mysql
def test_run_sync_postgres_to_mysql_moves_real_rows():
    """postgres (real DB) -> mysql (real DB): a full real reverse-ETL sync
    through the actual Python binding, not a simulation. Verified by
    querying MySQL directly afterward -- not by trusting the reported count."""
    src_table = f"pytest_pg_{uuid.uuid4().hex[:8]}"
    dst_table = f"pytest_mysql_{uuid.uuid4().hex[:8]}"
    _seed_postgres(
        src_table,
        [
            {"id": 1, "name": "Alice", "email": "alice@example.com", "ltv": 4200.5},
            {"id": 2, "name": "Bob", "email": "bob@example.com", "ltv": 1800},
        ],
    )
    _drop_mysql_table(dst_table)
    _create_mysql_table(dst_table)

    try:
        result = pyreverseetl.run_sync(
            source_type="postgres",
            source_config=_pg_config(src_table),
            destination_type="mysql",
            destination_config=_mysql_config(dst_table),
            limit=None,
            compliance_rules=None,
        )

        assert result.rows_read == 2
        assert result.rows_written == 2
        assert result.rows_failed == 0

        rows = _query_mysql(dst_table)
        assert "Alice" in rows and "alice@example.com" in rows
        assert "Bob" in rows and "bob@example.com" in rows
    finally:
        _drop_mysql_table(dst_table)


@requires_postgres_and_mysql
def test_run_sync_applies_real_compliance_masking():
    """PII masking must actually happen before data leaves the process --
    the raw email must never reach the destination once a masking rule is
    configured, verified by querying the real destination afterward."""
    src_table = f"pytest_pii_{uuid.uuid4().hex[:8]}"
    dst_table = f"pytest_pii_dst_{uuid.uuid4().hex[:8]}"
    _seed_postgres(src_table, [{"id": 1, "name": "Carol", "email": "carol.real@example.com", "ltv": 0}])
    _drop_mysql_table(dst_table)
    _create_mysql_table(dst_table)

    compliance_rules = [
        {
            "id": "mask_email",
            "rule_type": "pii_masking",
            "target_fields": ["email"],
            "action": {"type": "mask", "pattern": "****"},
        }
    ]

    try:
        result = pyreverseetl.run_sync(
            source_type="postgres",
            source_config=_pg_config(src_table),
            destination_type="mysql",
            destination_config=_mysql_config(dst_table),
            limit=None,
            compliance_rules=json.dumps(compliance_rules),
        )

        assert result.rows_written == 1

        rows = _query_mysql(dst_table)
        assert "carol.real@example.com" not in rows, "raw PII must never reach the destination"
        assert "****" in rows
    finally:
        _drop_mysql_table(dst_table)


@requires_postgres_and_mysql
def test_lineage_reflects_real_sync_runs():
    """The lineage graph accumulated by run_sync must show a real edge with
    the real record count for this run."""
    src_table = f"pytest_lineage_{uuid.uuid4().hex[:8]}"
    dst_table = f"pytest_lineage_dst_{uuid.uuid4().hex[:8]}"
    _seed_postgres(
        src_table, [{"id": i, "name": f"Row {i}", "email": f"row{i}@example.com", "ltv": 0} for i in range(3)]
    )
    _drop_mysql_table(dst_table)
    _create_mysql_table(dst_table)

    try:
        result = pyreverseetl.run_sync(
            source_type="postgres",
            source_config=_pg_config(src_table),
            destination_type="mysql",
            destination_config=_mysql_config(dst_table),
            limit=None,
            compliance_rules=None,
        )

        graph = json.loads(pyreverseetl.lineage_json())
        matching_edges = [e for e in graph["edges"] if e["run_id"] == result.run_id]
        assert len(matching_edges) == 1
        assert matching_edges[0]["record_count"] == 3

        dot = pyreverseetl.lineage_dot()
        assert dot.startswith("digraph lineage {")
        assert src_table in dot
        assert dst_table in dot
    finally:
        _drop_mysql_table(dst_table)


@requires_postgres_and_mysql
def test_cli_execute_activation_drives_the_real_engine():
    """The actual `pyreverseetl` CLI command, invoked as a subprocess exactly
    as a real user would, must perform a real sync -- not report a fabricated
    row count. This is the literal fix for the audit finding that the CLI
    never called the Rust engine. Also proves state persists across separate
    CLI invocations (create-workflow / create-activation / execute), which a
    real user relies on."""
    src_table = f"pytest_cli_{uuid.uuid4().hex[:8]}"
    dst_table = f"pytest_cli_dst_{uuid.uuid4().hex[:8]}"
    _seed_postgres(src_table, [{"id": 1, "name": "Eve", "email": "eve@example.com", "ltv": 500}])
    _drop_mysql_table(dst_table)
    _create_mysql_table(dst_table)

    # Isolate this test's CLI state from the real user's ~/.pyreverseetl/state.json.
    state_path = os.path.join(tempfile.mkdtemp(), "state.json")
    env = dict(os.environ, PYREVERSEETL_STATE_PATH=state_path)

    def run_cli(*args):
        # `sys.executable` is the interpreter running this test, which already
        # has the real, maturin-installed `pyreverseetl` (and its compiled
        # `_core` extension) on its path -- no PYTHONPATH hacks needed.
        return subprocess.run(
            [sys.executable, "-m", "pyreverseetl.cli", *args],
            cwd=REPO_ROOT,
            env=env,
            capture_output=True,
            text=True,
            check=True,
        )

    try:
        run_cli(
            "create-workflow", "wf_cli_test", "CLI Test Workflow", "postgres", src_table,
            "--source-config", _pg_config(src_table),
        )
        run_cli(
            "create-activation", "act_cli_test", "wf_cli_test", "mysql",
            "--dest-config", _mysql_config(dst_table),
        )
        execute_result = run_cli("execute", "act_cli_test")

        output = json.loads(execute_result.stdout.strip().splitlines()[-1])
        assert output["status"] == "success", output
        assert output["rows_synced"] == 1, "must be the real row count, not a fabricated placeholder"

        rows = _query_mysql(dst_table)
        assert "eve@example.com" in rows, "the real row read from Postgres must have reached MySQL"
    finally:
        _drop_mysql_table(dst_table)


def test_cli_version_reports_the_real_installed_version():
    result = subprocess.run(
        [sys.executable, "-m", "pyreverseetl.cli", "--version"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=True,
    )
    assert result.stdout.strip() == pyreverseetl.__version__
