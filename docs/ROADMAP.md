# PyReverseETL Roadmap

**Current version:** v3.1.0 (`Cargo.toml` workspace version / `pyproject.toml`)
**Last verified:** 2026-09-06 against commit `724f00d` (HEAD)

## Honest status (read this first)

Earlier versions of this file described phases keyed to invented dates on
top of a "v1.0.0" baseline: v1.1 "Aug 2026" (Salesforce/HubSpot/Marketo
destinations), v1.5 "Sep 2026" (streaming/CDC), v2.0 "Oct 2026" (ML-based
routing), v2.5 "Q4 2026" (GDPR/compliance), and v3.0 "Q1 2027" ("Enterprise
Scale," "99.99% uptime SLA"). That baseline is long superseded — the repo
is already at v3.1.0, and most of that feature list is either genuinely
implemented or explicitly documented as an honest gap in README.md's "What's
real right now" table. This rewrite replaces the fabricated calendar with
what's actually shipped, using the same evidence README.md already cites.

## What's real and working today

- **Sync engine** — `pyreverseetl._core.run_sync` (PyO3 binding over
  `core/src/executor.rs::execute_sync`) is the real end-to-end path: reads
  from a real source, applies real compliance rules, writes to a real
  destination, and records a real lineage edge. There is no simulated
  mode — an unwired connector returns an explicit error rather than
  faking success.
- **Connectors** (authoritative list is README's "What's real right now"
  table):
  - PostgreSQL and MySQL, source + destination, via `sqlx`, verified
    against real Postgres/MySQL containers.
  - S3 / S3-compatible object storage, via `aws-sdk-s3`. JSON-lines and
    CSV are implemented and verified against a real MinIO container;
    Parquet/Avro/ORC/Iceberg/Delta are declared but return an explicit
    "not implemented" error.
  - Webhook destination via real HTTP POST/PATCH/DELETE with real auth
    headers.
  - Salesforce/HubSpot/Marketo destinations
    (`core/src/adapters/{salesforce,hubspot,marketo}.rs`) — real
    REST/OAuth2 clients, verified against a mock HTTP server (no live
    account was available), not stubs.
- **Reliability, wired into the real path** (commit `022ddda`, "Wire
  retry, dry-run, schema-drift, and idempotency into the real sync
  path"):
  - Retry-with-backoff for every HubSpot/Salesforce/Marketo/webhook HTTP
    call, verified with scripted-response tests
    (`MockHttpServer::start_sequence`).
  - `--dry-run`: runs the real source read + compliance engine, skips the
    actual destination write, exposes the exact would-be payload via
    `dry_run_preview`.
  - Schema-drift detection (`--schema-store`): persists the last-seen
    field shape to a SQLite file and diffs it on each run.
  - Idempotency ledger (`--idempotency-store`): SQLite-backed,
    content-hash-keyed, skips already-synced records — including for the
    webhook adapter, which has no upsert semantics of its own.
- **Lineage tracking** — `pyreverseetl_core::lineage::LineageGraph`
  records a real edge (source, destination, record count, timestamps) per
  `run_sync` call, exportable to JSON/Graphviz DOT. This is new: the
  README previously described "lineage tracking" as a feature with zero
  backing code.
- **Compliance / PII masking** — `DefaultComplianceEngine` applies real
  per-record mask/remove/truncate rules; `Encrypt` honestly reports as
  unresolved rather than claiming success it doesn't have.
- **Tests** — 64 `#[test]`-annotated Rust functions in `core/`, plus
  Docker-backed round-trip tests (`--ignored` by default, run explicitly
  against real Postgres/MySQL/MinIO containers) and real end-to-end
  Python tests (`tests/test_real_sync_docker.py`) that run the actual CLI
  as a subprocess. CI (`tests.yml`, `ci.yml`) is green on the latest push,
  after fixing missing native build deps for `rdkafka-sys` (commits
  `7a827d5`, `7f8aa45`, `6ccec29`).

## What's partial, scaffolding, or an explicit non-goal for now

- **Other cloud object storage backends** (beyond S3/MinIO) — return an
  explicit "not implemented" error.
- **Kafka/CDC streaming, HDFS, PySpark transforms, the CLI dashboard,
  StatGuardian quality-gate integration** — present in the codebase from
  earlier work but not wired into `execute`/`run_sync`. Several return
  fixed/fabricated numbers if called directly (e.g.
  `SparkTransformer::submit`), documented inline in the affected modules.
  Treat as experimental, not usable.
- **CRM/marketing-automation adapters** are real API clients but only
  verified against mocked HTTP responses — no live Salesforce/HubSpot/
  Marketo account was available to confirm the real API matches the
  mock's assumed request shape.
- **The "150+ connectors" catalog** in the older `docs/ROADMAP_V2.1.md`
  draft was a database of intended connectors, not a count of working
  implementations. Only the connectors listed in README's table are real
  today.

## Near-term roadmap (concrete, no invented dates)

1. Get a live account for at least one of Salesforce/HubSpot/Marketo to
   validate the adapters against the real API, not just a mock server —
   this is the single biggest unverified-but-plausibly-correct piece of
   the current connector set.
2. Decide the fate of the unwired-but-present modules (Kafka/CDC, HDFS,
   PySpark, dashboard, StatGuardian gate): wire them into the real
   `execute_sync` path for real, or remove them so they stop implying
   capability that doesn't exist yet.
3. Apply the same "wire it into `execute_sync` for real" pattern used for
   retry/dry-run/schema-drift/idempotency (`022ddda`) to whichever of the
   above is picked first.
4. If a cloud storage backend beyond S3/MinIO is genuinely needed, follow
   the existing S3 implementation's pattern (real SDK client, explicit
   "not implemented" for unsupported formats) rather than adding a
   partial stub.

No committed dates, no SLA numbers, no "enterprise scale" claims — none
of that is backed by anything in this repo today.

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md). Open issues at
https://github.com/Mullassery/PyReverseETL/issues
