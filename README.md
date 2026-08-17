# PyReverseETL

**Reverse ETL: move data from a source system into the tools your team actually
works in, with a real lineage graph and real compliance enforcement.**

[![License: Proprietary](https://img.shields.io/badge/License-Proprietary-red.svg)](LICENSE)
![Version: v3.0.0](https://img.shields.io/badge/Version-v3.0.0-blue)
[![CI](https://github.com/Mullassery/PyReverseETL/actions/workflows/ci.yml/badge.svg)](https://github.com/Mullassery/PyReverseETL/actions/workflows/ci.yml)

A Rust sync engine with Python bindings and a CLI. `pyreverseetl execute`
opens a real connection to a real source, reads real records, runs them
through a real compliance/PII-masking engine, writes them to a real
destination, and records a real lineage edge (source, destination, record
count, timestamps) that you can query afterward. There is no simulated mode:
if a connector isn't wired to a real backend yet, it's not offered as an
option, not silently faked.

## What's real right now

| Connector | Direction | Status |
|---|---|---|
| PostgreSQL | source + destination | Real, via [`sqlx`](https://github.com/launchbadge/sqlx). Generic-schema read/write, incremental reads, upsert. Verified with a real Postgres container. |
| MySQL | source + destination | Real, via `sqlx`. Same capabilities as Postgres. Verified with a real MySQL container. |
| S3 / S3-compatible object storage (MinIO, etc.) | source + destination | Real, via [`aws-sdk-s3`](https://github.com/awslabs/aws-sdk-rust), with a custom-endpoint / path-style option for MinIO. JSON-lines and CSV formats are implemented; Parquet/Avro/ORC/Iceberg/Delta are declared but return an explicit "not implemented" error rather than silently no-op'ing. Verified with a real MinIO container. |
| Webhook | destination | Real HTTP POST/PATCH/DELETE via `reqwest`, with real auth headers (Bearer/API key/Basic) and real JSON payload construction. |
| Salesforce | destination | Real REST API client: OAuth2 token exchange, `sobjects` create/upsert-by-external-ID/delete/describe against the actual Salesforce endpoint shapes. No live Salesforce account was available to verify against, so this is verified against a local mock HTTP server that asserts the exact request shape (method, path, auth header, body) the real API expects. |
| HubSpot | destination | Real CRM v3 API client (create, upsert-by-`idProperty`, delete, properties/schema). Same caveat: verified against a mock server, not a live account. |
| Marketo | destination | Real REST API client (identity token endpoint, `createOrUpdate` bulk leads, lead delete, describe). Same caveat: verified against a mock server, not a live account. |
| GCS / Azure Blob | source + destination | **Not implemented.** Calling them returns an explicit error instead of a fake success. |
| Kafka, HDFS, Spark/PySpark transforms, CDC streaming, the CLI dashboard, StatGuardian quality gates | — | Present in the codebase from earlier work but out of scope for this pass and not wired into `execute` / `run_sync`. Treat as experimental; several return fixed/fabricated numbers (documented inline where that's the case, e.g. `SparkTransformer::submit`). |

If you need a connector marked "not implemented" above, that's an honest gap,
not a documentation oversight — open an issue rather than assuming it works.

## Install

```bash
pip install pyreverseetl
# or
uv pip install pyreverseetl

pyreverseetl --version    # prints the installed version, read straight from the compiled Rust core
```

This installs a compiled Rust extension (built with [maturin](https://github.com/PyO3/maturin)/PyO3) plus the `pyreverseetl` console command.

## Quick start: a real sync against a local Postgres

This spins up a real Postgres container, seeds a table, and syncs it to a
webhook using the real engine end to end.

```bash
docker run --rm -d -p 5432:5432 \
  -e POSTGRES_PASSWORD=postgres -e POSTGRES_DB=demo \
  --name pyreverseetl-demo-pg postgres:16

docker exec -i pyreverseetl-demo-pg psql -U postgres -d demo -c "
  CREATE TABLE customers (id INT PRIMARY KEY, name TEXT, email TEXT, ltv NUMERIC);
  INSERT INTO customers VALUES
    (1, 'Alice', 'alice@example.com', 4200.50),
    (2, 'Bob',   'bob@example.com',   1800.00);
"

# In another terminal: a throwaway HTTP endpoint to receive the synced rows
python3 -m http.server 8000 &   # or use https://webhook.site for a real inspectable URL

pyreverseetl create-workflow ltv_sync "LTV to webhook" postgres customers \
  --source-config '{"host":"localhost","port":5432,"database":"demo","username":"postgres","password":"postgres"}'

pyreverseetl create-activation ltv_to_hook ltv_sync webhook \
  --dest-config '{"url":"http://localhost:8000/hook","auth":{"type":"bearer","token":"demo"}}'

pyreverseetl execute ltv_to_hook
# {"status": "success", "run_id": "...", "rows_synced": 2, "rows_read": 2,
#  "rows_failed": 0, "compliance_violations": [], "duration_ms": 12,
#  "message": "Activation executed: 2 rows written to webhook"}

pyreverseetl lineage
# {"status": "success", "format": "json",
#  "lineage": {"nodes": {...}, "edges": [{"run_id": "...", "record_count": 2, ...}]}}
```

Mask PII before it ever leaves the process:

```bash
pyreverseetl execute ltv_to_hook --compliance-rules \
  '[{"id":"mask_email","rule_type":"pii_masking","target_fields":["email"],"action":{"type":"mask","pattern":"****"}}]'
```

Every one of these calls goes through the real Rust engine
(`pyreverseetl._core.run_sync`) — `rows_synced` is the number of records the
destination connector actually wrote, not a placeholder.

### Python API

```python
import json
import pyreverseetl

result = pyreverseetl.run_sync(
    source_type="postgres",
    source_config=json.dumps({
        "host": "localhost", "port": 5432, "database": "demo",
        "username": "postgres", "password": "postgres", "table": "customers",
    }),
    destination_type="webhook",
    destination_config=json.dumps({
        "url": "http://localhost:8000/hook",
        "auth": {"type": "bearer", "token": "demo"},
    }),
    limit=None,
    compliance_rules=None,
)
print(result.rows_written, result.duration_ms)

# Real lineage graph accumulated across every run_sync call in this process
print(pyreverseetl.lineage_json())
print(pyreverseetl.lineage_dot())  # Graphviz DOT export
```

## Lineage tracking

Every `run_sync` call registers real source/destination nodes and appends a
real edge — actual record count, actual start/completion timestamps — to an
in-process lineage graph (`pyreverseetl_core::lineage::LineageGraph`). It
supports upstream/downstream queries and exports to JSON or Graphviz DOT.
This did not exist anywhere in the codebase before this pass; the README
previously described "lineage tracking" as a feature with zero backing code.

## Compliance & PII handling

`DefaultComplianceEngine` (`pyreverseetl_core::governance::compliance_rules`)
applies real per-record rules before a write: mask a field, remove it,
truncate it, or (for `Encrypt`) honestly report it as unresolved — there is
no real encryption implementation, and the engine says so in
`check_compliance` rather than silently claiming success. A `MockComplianceEngine`
still exists but is `#[cfg(test)]`-only, so it can never run in a real build;
it exists purely as a test double for exercising governance wiring without
needing real masking behavior.

## Architecture

```
Python CLI / API  →  pyreverseetl._core (PyO3 bindings)  →  pyreverseetl_core::execute_sync
                                                                  │
                                            ┌─────────────────────┼─────────────────────┐
                                       source read          compliance apply        destination write
                                  (postgres/mysql/s3)      (DefaultComplianceEngine)  (postgres/mysql/s3/
                                                                                       webhook/salesforce/
                                                                                       hubspot/marketo)
                                                                  │
                                                          lineage edge recorded
```

- **`core/`** — the Rust engine: connectors, the compliance engine, lineage
  tracking, the sync executor.
- **`python/src/`** — PyO3 bindings (`run_sync`, `lineage_json`, `lineage_dot`,
  plus the lower-level data-model classes `PyWorkflow`/`PyDestination`/etc.).
- **`python/pyreverseetl/`** — the installed Python package: `cli.py` (the
  `pyreverseetl` command), `server.py` (an optional Flask REST wrapper around
  the same engine), both backed by the real engine rather than any
  in-process simulation.

## Development

```bash
# Rust: build, test, lint
cargo build -p pyreverseetl-core
cargo test -p pyreverseetl-core --lib          # unit tests (hermetic, no external services)
cargo test -p pyreverseetl-core --lib -- --ignored   # real Docker-backed round-trip tests, see below
cargo clippy -p pyreverseetl-core --lib
cargo fmt

# Python bindings
maturin develop --release
pytest tests/ -v
```

### Running the real, Docker-backed connector tests

Unit tests are hermetic by design (no network, no containers). The
round-trip tests that prove the Postgres/MySQL/S3 connectors actually talk to
a real service are `#[ignore]`d by default; run them explicitly against real
containers:

```bash
docker run --rm -d -p 5439:5432 -e POSTGRES_PASSWORD=postgres \
    -e POSTGRES_DB=pyreverseetl_test --name pyreverseetl-pg-test postgres:16
docker run --rm -d -p 3307:3306 -e MYSQL_ROOT_PASSWORD=mysql \
    -e MYSQL_DATABASE=pyreverseetl_test --name pyreverseetl-mysql-test mysql:8
docker run --rm -d -p 9000:9000 -e MINIO_ROOT_USER=minioadmin \
    -e MINIO_ROOT_PASSWORD=minioadmin --name pyreverseetl-minio-test minio/minio server /data
docker run --rm --entrypoint sh minio/mc -c \
    "mc alias set local http://host.docker.internal:9000 minioadmin minioadmin && mc mb local/pyreverseetl-test"

PYREVERSEETL_TEST_PG_PORT=5439 PYREVERSEETL_TEST_MYSQL_PORT=3307 \
PYREVERSEETL_TEST_MINIO_ENDPOINT=http://localhost:9000 \
    cargo test -p pyreverseetl-core --lib -- --ignored
```

The same Docker services also back real, end-to-end Python-level tests in
`tests/test_real_sync_docker.py` (they run the actual `pyreverseetl` CLI
command as a subprocess and check real rows moved through it); those are
skipped, not failed, when the containers aren't running.

### Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Known gaps (deliberately out of scope for this pass)

- **GCS / Azure object storage**: return an explicit "not implemented" error.
- **Kafka / CDC streaming, HDFS, PySpark transforms, the CLI dashboard,
  StatGuardian quality-gate integration**: present in the codebase but not
  wired into the real sync path (`execute` / `run_sync`); several of these
  return fixed, non-real numbers if you call their APIs directly (this is
  documented inline in the affected modules, e.g. `SparkTransformer::submit`).
  Treat anything not listed in the connector table above as unverified.
- Salesforce/HubSpot/Marketo clients are real API implementations but were
  only verified against mocked HTTP responses (no live account was available
  in this environment) — please report any request-shape mismatches against
  a real account as issues.

## License

Proprietary License. See [LICENSE](LICENSE) for details. All rights reserved.

## Support

- GitHub Issues: [PyReverseETL/issues](https://github.com/Mullassery/PyReverseETL/issues)
- Discussions: [PyReverseETL/discussions](https://github.com/Mullassery/PyReverseETL/discussions)
