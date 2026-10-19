# Audit findings — 2026-10-19

This file records the repository-specific findings from the portfolio audit
conducted on 2026-10-19. It is documentation only; no code or release artifacts
were changed in this pass unless explicitly noted under "Implementation status".

Sources: the repository's own README/source/workflows, GitHub metadata and
Actions runs, PyPI metadata, and official competitor documentation. Claims are
marked as verified, inferred, or open questions. "Absent" means not found in the
inspected repository/docs, not proof that it cannot exist.

## What it is

Operational data activation runtime: warehouse/DB → CRM/marketing destinations with lineage, PII controls, idempotency, and retries.

## Verified strengths

- Real Postgres/MySQL/S3/MinIO/webhook paths verified against containers; incremental reads/upsert; idempotency ledger; retries; schema drift; PII mask/remove/truncate; lineage graph; CLI/Python/Flask API; honest connector table.
- Compliance/lineage-first self-hosted activation is a credible wedge.

## Verified gaps and user-facing risks

- CRM adapters (Salesforce/HubSpot/Marketo) are mock-verified only; no live account evidence.
- No scheduling/observability/alerting; no Rust-side connector SDK (a Python
  plugin interface + local SQLite connector shipped in 3.4.0); macOS arm64 wheel
  only; GitHub release 2.1.1 vs PyPI 3.1.7.
- No HA/production deployment.

## Direct competitors

- Airbyte (large connector ecosystem), Census/Hightouch (managed reverse ETL), RudderStack Reverse ETL.

## Ranked next actions

- P1: live-verify CRM connectors + contract tests (L).
- P1: publish Linux/Windows wheels (S/M).
- P2: scheduling/run history/metrics/alerts (M).
- P2: connector plugin SDK (M) — Python path shipped in 3.4.0; Rust-side
  connector registration remains open.

## Do not build

- CDC/streaming and Spark transforms now; both are unwired and outside the proven path.

## Evidence gaps / validation needed

- No live CRM credentials; no throughput/failure-injection benchmark; no production deployment.

## Recommended disposition

Invest; connector verification is the entry ticket.

## Implementation status

Added file-based and live StatGuardian gates:

- `--validation-report <json>` blocks on a failing/malformed report (fail closed).
- `--statguardian-url <url>` dry-runs a source sample, POSTs rows to
  `/validate`, and blocks the real sync on any row failure or network error.
- Tests: 3 file-gate + 3 live-gate = 6 passed. Version `3.3.0`. The live gate
  is a sample check, not a full-table scan.

Added a Python connector plugin interface and a real local SQLite connector
version `3.4.0`:

- Plugin interface: `pyreverseetl.connectors.BaseConnector`, programmatic
  registration, and the `pyreverseetl.connectors` entry-point group.
- `run_with_connector` reads through a source plugin, applies
  mask/remove/truncate compliance actions, and writes through a destination
  plugin. It is a Python-level path and does not write the Rust lineage graph;
  the Rust `run_sync` connector set is unchanged.
- SQLite is a real source/destination via stdlib `sqlite3` (append/replace/
  upsert), tested against temporary databases. Kafka/HDFS/Spark/dashboard stay
  experimental/unwired.
