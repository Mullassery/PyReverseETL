# PyReverseETL

**The open operational data activation runtime: move data from a source system into the tools your team actually works in, with a real lineage graph and real compliance enforcement.**

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![PyPI](https://img.shields.io/pypi/v/pyreverseetl)](https://pypi.org/project/pyreverseetl/)
[![Python 3.9+](https://img.shields.io/badge/Python-3.9%2B-blue)](https://www.python.org)
[![CI](https://github.com/Mullassery/PyReverseETL/actions/workflows/ci.yml/badge.svg)](https://github.com/Mullassery/PyReverseETL/actions/workflows/ci.yml)
[![Graphify](https://img.shields.io/badge/Graphify-knowledge%20graph-blue)](https://github.com/Graphify-Labs/graphify)

## What it is

PyReverseETL is a Rust sync engine with Python bindings and a CLI. `pyreverseetl execute`
opens a real connection to a real source, reads real records, runs them through a real
compliance/PII-masking engine, writes them to a real destination, and records a real
lineage edge (source, destination, record count, timestamps) that you can query afterward.
There is no simulated mode: if a connector isn't wired to a real backend yet, it's not
offered as an option and not silently faked.

## Features

- **Real connectors, real I/O.** PostgreSQL, MySQL, S3/S3-compatible object storage
  (MinIO) as source and destination; generic-schema read/write, incremental reads, upsert.
- **Real HTTP destinations.** Webhook destination with real auth (Bearer / API key /
  Basic) and JSON payload construction; CRM/marketing-automation REST adapters for
  Salesforce, HubSpot, and Marketo.
- **Compliance/PII handling before data leaves the process.** Mask, remove, or truncate
  fields per-record; `Encrypt` is honestly reported as unresolved rather than faked.
- **Lineage tracking.** Every sync registers real source/destination nodes and appends a
  real edge (record count + timestamps) to an in-process lineage graph, queryable and
  exportable to JSON or Graphviz DOT.
- **Reliability in the real sync path.** Retry with backoff on transient HTTP failures,
  dry-run preview, schema-drift detection, and an idempotency ledger for crash-mid-batch
  recovery.
- **CLI + Python API + optional Flask REST server**, all backed by the same engine.

## Who it's for

- **Data and platform engineers** syncing operational data (Postgres/MySQL) into a CRM or
  marketing tool, with compliance/PII masking applied before it leaves the source.
- **Teams pushing to a webhook destination** with real auth for integrations that don't
  have a dedicated connector yet.
- **Anyone who needs to audit what moved where** — every sync records a real lineage edge
  you can query afterward, not just a log line.

## What it is not

- **Not a streaming/CDC platform.** Kafka, CDC streaming, and HDFS are not wired into the
  real sync path and are out of scope for this pass.
- **Not a Spark/PySpark transform engine.** The transformer is present from earlier work
  but not wired into `execute`/`run_sync`; several such modules return fixed, non-real
  numbers if called directly (documented inline, e.g. `SparkTransformer::submit`).
- **Not a StatGuardian quality-gate integration.** Present in the codebase but not wired
  into the real sync path; treat it as experimental.
- **Not verified against live CRM accounts.** The Salesforce/HubSpot/Marketo adapters are
  real REST clients, but they were verified against local mock HTTP servers that assert
  the exact request shape (no live account was available). Please report request-shape
  mismatches against a real account as issues.
- **Not encryption.** `Encrypt` compliance actions are reported as unresolved rather than
  silently claimed to succeed.

## Connector status

| Connector | Direction | Status |
|---|---|---|
| PostgreSQL | source + destination | Real, via [`sqlx`](https://github.com/launchbadge/sqlx). Generic-schema read/write, incremental reads, upsert. Verified with a real Postgres container. |
| MySQL | source + destination | Real, via `sqlx`. Same capabilities as Postgres. Verified with a real MySQL container. |
| S3 / S3-compatible object storage (MinIO, etc.) | source + destination | Real, via [`aws-sdk-s3`](https://github.com/awslabs/aws-sdk-rust), with a custom-endpoint / path-style option for MinIO. JSON-lines and CSV implemented; Parquet/Avro/ORC/Iceberg/Delta return an explicit "not implemented" error rather than silently no-op'ing. Verified with a real MinIO container. |
| Webhook | destination | Real HTTP POST/PATCH/DELETE via `reqwest`, with real auth headers (Bearer/API key/Basic) and JSON payload construction. |
| CRM (`core/src/adapters/salesforce.rs`) | destination | Real REST API client: OAuth2 token exchange, object-based create/upsert-by-external-ID/delete/describe. Verified against a local mock HTTP server. |
| CRM (`core/src/adapters/hubspot.rs`) | destination | Real v3-style REST API client (create, upsert-by-ID-property, delete, properties/schema). Verified against a mock server. |
| Marketing automation (`core/src/adapters/marketo.rs`) | destination | Real REST API client (identity token endpoint, bulk lead create/update, delete, describe). Verified against a mock server. |
| Other cloud object storage backends | source + destination | **Not implemented.** Returns an explicit error instead of a fake success. |
| Kafka, HDFS, Spark/PySpark transforms, CLI dashboard, StatGuardian gates | — | Present from earlier work, out of scope for this pass, not wired into `execute`/`run_sync`. Treat as experimental. |

If you need a connector marked "not implemented" above, that's an honest gap, not a
documentation oversight — open an issue rather than assuming it works. The exact
third-party API each CRM/marketing adapter targets is named in the source file, not
repeated here to avoid implying vendor endorsement.

## Project status

**Beta.** Version 3.1.6. The Postgres/MySQL/S3/webhook paths are real and verified against
real services; the `web` source connector (MudFish-backed site crawling) is implemented
and unit-tested but not yet exercised end-to-end against a live site; the CRM/marketing
adapters are real but mock-verified only; several
modules from earlier work remain unwired and are called out as such above. For a blunt,
file-and-line-referenced list of what's tested vs. untested vs. broken vs. not built, see
[`ROADMAP_HONEST.md`](ROADMAP_HONEST.md). Changes are tracked in [CHANGELOG.md](CHANGELOG.md).

## Installation

```bash
pip install pyreverseetl
# or
uv pip install pyreverseetl

pyreverseetl --version   # prints the installed version, read straight from the compiled Rust core
```

This installs a compiled Rust extension (built with [maturin](https://github.com/PyO3/maturin)/PyO3)
plus the `pyreverseetl` console command. Requires Python 3.9+.

## Quick start

This spins up a real Postgres container, seeds a table, and syncs it to a webhook using
the real engine end to end.

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

## Singer target protocol

Consume Singer NDJSON and forward RECORD batches to a sink:

```bash
cat singer-output.ndjson | pyreverseetl-singer-target --output records.jsonl
```

```python
from pyreverseetl.singer import run_target, jsonl_sink
run_target(lines, jsonl_sink("records.jsonl"))
```

Supported messages: SCHEMA, RECORD (batched) and STATE (flushes the batch).
This is a protocol adapter; native Rust connectors remain the execution path.

## Changelog

### 3.3.1
- Encrypted CLI state (Fernet/keyring) with legacy plaintext migration.
- Key export/import for backup and recovery.
- Singer SCHEMA/RECORD/STATE target adapter.

## Encrypted local state

Workflow/activation credentials are no longer stored as plaintext JSON. The CLI
state file is encrypted with Fernet; the key lives in the OS keyring, or in
`PYREVERSEETL_STATE_KEY` for headless CI:

```bash
export PYREVERSEETL_STATE_KEY="$(python -c 'from cryptography.fernet import Fernet;print(Fernet.generate_key().decode())')"
```

Existing plaintext state is read and transparently re-encrypted on the next
save. Losing the key means losing the local state file (re-enter credentials);
the error is explicit rather than silently falling back to plaintext.

## Usage

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

### Reliability features

- **Retry with backoff**: every HubSpot/Salesforce/Marketo/webhook HTTP call (upsert,
  delete, schema fetch, OAuth token exchange) retries transient failures (connection
  errors, timeouts, 429 rate limits) with exponential backoff.
- **Dry run** (`--dry-run` / `dry_run=True`): reads from the real source and runs the real
  compliance engine, but never writes to the destination. `dry_run_preview` shows the exact
  payload each record would have sent.
  ```bash
  pyreverseetl execute my_activation --dry-run
  ```
- **Schema-drift detection** (`--schema-store <path>`): persists the last-seen
  field-name/type shape for a source→destination pair to a real SQLite file and reports any
  field added/removed/type-changed since the last run (`schema_changes`).
  ```bash
  pyreverseetl execute my_activation --schema-store .pyreverseetl/schema.db
  ```
- **Idempotency ledger** (`--idempotency-store <path>`): persists which exact record content
  was already sent to which destination to a real SQLite file. Re-running the same sync
  (e.g. after a crash mid-batch) skips records already synced (`rows_skipped_idempotent`)
  while still sending records whose content genuinely changed.
  ```bash
  pyreverseetl execute my_activation --idempotency-store .pyreverseetl/idempotency.db
  ```

### Compliance & PII handling

`DefaultComplianceEngine` (`pyreverseetl_core::governance::compliance_rules`) applies real
per-record rules before a write: mask a field, remove it, truncate it, or (for `Encrypt`)
honestly report it as unresolved — there is no real encryption implementation, and the
engine says so in `check_compliance` rather than silently claiming success.

### Architecture

```
Python CLI / API  →  pyreverseetl._core (PyO3 bindings)  →  pyreverseetl_core::execute_sync
                                                                  │
                                            ┌─────────────────────┼─────────────────────┐
                                       source read          compliance apply        destination write
                                  (postgres/mysql/s3)      (DefaultComplianceEngine)  (postgres/mysql/s3/
                                                                                       webhook/CRM/marketing-
                                                                                       automation adapters)
                                                                  │
                                                          lineage edge recorded
```

- **`core/`** — the Rust engine: connectors, the compliance engine, lineage tracking, the
  sync executor.
- **`python/src/`** — PyO3 bindings (`run_sync`, `lineage_json`, `lineage_dot`, plus the
  lower-level data-model classes `PyWorkflow`/`PyDestination`/etc.).
- **`python/pyreverseetl/`** — the installed Python package: `cli.py` (the `pyreverseetl`
  command) and `server.py` (an optional Flask REST wrapper around the same engine).

## Validation gate

```bash
# File-based report
pyreverseetl execute-activation my-activation --validation-report statguardian-report.json

# Live StatGuardian service: validate a sample before the real sync
pyreverseetl execute-activation my-activation --statguardian-url http://127.0.0.1:8000
```

A file report must have `"passed": true`; anything else blocks the sync. The
live `--statguardian-url` gate dry-runs a sample and calls the service's
`/validate`; a failure or network error blocks the real sync. It is a sample
gate, not a full-table scan.

## Documentation

- [ROADMAP_HONEST.md](ROADMAP_HONEST.md) — what's tested vs. untested vs. broken vs. not
  built, plus known technical debt.
- [docs/ROADMAP.md](docs/ROADMAP.md) — narrative roadmap.
- [CHANGELOG.md](CHANGELOG.md) — release history.
- [COMPLIANCE_CHECKLIST.md](COMPLIANCE_CHECKLIST.md) — compliance posture.
- [SECURITY.md](SECURITY.md) — how to report a vulnerability.

## Known issues and deferred work

**Blocked / failed:**
- Live Salesforce/HubSpot/Marketo verification (no accounts).
- Scheduler/run-history observability.
- CDC/streaming.

**Rejected / deferred:**
- Spark transforms and streaming CDC now — outside the proven sync path.

Full shipped/blocked/rejected detail: [`QUEUE_STATUS.md`](QUEUE_STATUS.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines and
[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) for community standards.

```bash
# Rust: build, test, lint
cargo build -p pyreverseetl-core
cargo test -p pyreverseetl-core --lib          # unit tests (hermetic, no external services)
cargo clippy -p pyreverseetl-core --lib
cargo fmt

# Python bindings
maturin develop --release
pytest tests/ -v
```

The Docker-backed connector round-trip tests are `#[ignore]`d by default; run them
explicitly against real containers:

```bash
docker run --rm -d -p 5439:5432 -e POSTGRES_PASSWORD=postgres \
    -e POSTGRES_DB=pyreverseetl_test --name pyreverseetl-pg-test postgres:16
docker run --rm -d -p 3307:3306 -e MYSQL_ROOT_PASSWORD=mysql \
    -e MYSQL_DATABASE=pyreverseetl_test --name pyreverseetl-mysql-test mysql:8
docker run --rm -d -p 9000:9000 -e MINIO_ROOT_USER=minioadmin \
    -e MINIO_ROOT_PASSWORD=minioadmin --name pyreverseetl-minio-test minio/minio server /data

PYREVERSEETL_TEST_PG_PORT=5439 PYREVERSEETL_TEST_MYSQL_PORT=3307 \
PYREVERSEETL_TEST_MINIO_ENDPOINT=http://localhost:9000 \
    cargo test -p pyreverseetl-core --lib -- --ignored
```

## Related projects

Other tools from the same family, for adjacent parts of the data platform:

- [StatGuardian](https://github.com/Mullassery/StatGuardian) — fast, declarative data quality framework (Rust): schema validation, drift detection, anomaly detection.
- [PyDBTGuard](https://github.com/Mullassery/PyDBTGuard) — analyzes a dbt project's `manifest.json` to flag risky tests, map blast radius, and estimate test cost from static metadata.
- [PyAirflowTester](https://github.com/Mullassery/PyAirflowTester) — dependency intelligence and quality assurance for Airflow and dbt pipelines.
- [PyBeamGuard](https://github.com/Mullassery/PyBeamGuard) — Apache Beam and Dataflow pipeline analysis for pre-deployment validation, cost forecasting, hot-key detection, and reliability.
- [ClusterAudienceKit](https://github.com/Mullassery/ClusterAudienceKit) — Rust-powered customer segmentation: RFM, clustering, churn, CLV, and lookalike audiences.
- [PyWeatherEnriched](https://github.com/Mullassery/PyWeatherEnriched) — real historical weather and geocoding enrichment for ML pipelines.

[Browse all projects →](https://github.com/Mullassery?tab=repositories)

## License

This project is licensed under the [Apache License 2.0](LICENSE).
