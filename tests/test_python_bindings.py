"""Test Python bindings for PyReverseETL Rust core."""

import sys

import pytest


def test_pyreverseetl_import():
    """Verify Python bindings are accessible."""
    try:
        import pyreverseetl

        assert pyreverseetl is not None
    except ImportError:
        pytest.skip("pyreverseetl bindings not built yet (run maturin develop)")


def test_pyreverseetl_version():
    """Verify version is set."""
    try:
        import pyreverseetl

        assert hasattr(pyreverseetl, "__version__")
    except ImportError:
        pytest.skip("pyreverseetl bindings not built yet")


def test_run_sync_accepts_dry_run_and_store_path_kwargs():
    """`run_sync` gained `dry_run`, `schema_store_path`, and
    `idempotency_store_path` keyword arguments (see core/src/executor.rs's
    `ExecuteOptions`). This environment has no live Postgres/MySQL/S3 source
    to run a real sync against, so this test can't observe a successful
    result -- but it proves the new kwargs are genuinely wired through the
    PyO3 FFI boundary (a real `TypeError: unexpected keyword argument` would
    fire here if they weren't), by exercising them against a real (if
    unreachable) source spec and checking the failure is a real connection
    error, not an argument-binding error.
    """
    try:
        from pyreverseetl import _core
    except ImportError:
        pytest.skip("pyreverseetl bindings not built yet (run maturin develop)")

    import json

    source_config = json.dumps(
        {
            "host": "127.0.0.1",
            "port": 1,  # nothing listens here -- a real, immediate connection failure
            "database": "nonexistent",
            "user": "nobody",
            "password": "",
            "table": "nonexistent",
        }
    )
    destination_config = json.dumps({"url": "http://127.0.0.1:1"})

    with pytest.raises(Exception) as exc_info:
        _core.run_sync(
            source_type="postgres",
            source_config=source_config,
            destination_type="webhook",
            destination_config=destination_config,
            dry_run=True,
            schema_store_path="/tmp/does-not-matter-schema.db",
            idempotency_store_path="/tmp/does-not-matter-idempotency.db",
        )

    # A TypeError here would mean the kwargs aren't real parameters on the
    # Rust side; a connection/runtime error means they were accepted and we
    # got as far as actually trying (and failing) to reach the source.
    assert not isinstance(exc_info.value, TypeError)


def test_cli_execute_parses_dry_run_and_store_path_flags(monkeypatch, tmp_path):
    """`pyreverseetl execute <id> --dry-run --schema-store <p> --idempotency-store <p>`
    must parse those three flags out of sys.argv and pass them through to
    `CLIInterface.execute_activation` in the right order. Mocks
    `execute_activation` itself (no real activation/source needed) so this
    is purely testing the argv-parsing layer in `cli.py::main`.
    """
    from pyreverseetl.cli import CLIInterface, main

    captured = {}

    def fake_execute_activation(self, activation_id, limit, compliance_rules, dry_run, schema_store_path, idempotency_store_path):
        captured["activation_id"] = activation_id
        captured["limit"] = limit
        captured["dry_run"] = dry_run
        captured["schema_store_path"] = schema_store_path
        captured["idempotency_store_path"] = idempotency_store_path
        return {"status": "success", "run_id": "fake"}

    monkeypatch.setattr(CLIInterface, "execute_activation", fake_execute_activation)
    monkeypatch.setattr(
        sys,
        "argv",
        [
            "pyreverseetl",
            "execute",
            "my_activation",
            "--dry-run",
            "--schema-store",
            str(tmp_path / "schema.db"),
            "--idempotency-store",
            str(tmp_path / "idempotency.db"),
        ],
    )
    monkeypatch.setenv("PYREVERSEETL_STATE_PATH", str(tmp_path / "state.json"))

    main()

    assert captured["activation_id"] == "my_activation"
    assert captured["dry_run"] is True
    assert captured["schema_store_path"] == str(tmp_path / "schema.db")
    assert captured["idempotency_store_path"] == str(tmp_path / "idempotency.db")
