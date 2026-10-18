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
- No scheduling/observability/alerting; no connector SDK; macOS arm64 wheel only; GitHub release 2.1.1 vs PyPI 3.1.7.
- No HA/production deployment.

## Direct competitors

- Airbyte (large connector ecosystem), Census/Hightouch (managed reverse ETL), RudderStack Reverse ETL.

## Ranked next actions

- P1: live-verify CRM connectors + contract tests (L).
- P1: publish Linux/Windows wheels (S/M).
- P2: scheduling/run history/metrics/alerts (M).
- P2: connector plugin SDK (M).

## Do not build

- CDC/streaming and Spark transforms now; both are unwired and outside the proven path.

## Evidence gaps / validation needed

- No live CRM credentials; no throughput/failure-injection benchmark; no production deployment.

## Recommended disposition

Invest; connector verification is the entry ticket.

## Implementation status

No code changes in this pass.
