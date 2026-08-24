# PyReverseETL Roadmap

**Current Version:** v1.0.0

## Vision

PyReverseETL provides data activation and reverse ETL for orchestrating workflows across hundreds of destinations.

## Completed Milestones

✅ **v1.0 (July 2026)** — Data Activation Foundation
- CLI: `pyreverseetl create-workflow`, `create-activation`, `execute`, `status`, `metrics`
- REST API (Port 8000) for automation
- n8n, Power Automate, Temporal, Airflow integration
- Workflow orchestration & metrics tracking
- Comprehensive WORKFLOW_INTEGRATION.md documentation

## In Progress

⏳ **v1.1 (Aug 2026)** — Destination Ecosystem
- Salesforce connector (CRM sync)
- HubSpot integration
- Marketo platform support
- Custom destination APIs

## Planned

📅 **v1.5 (Sep 2026)** — Streaming Activation
- Real-time data sync
- Change data capture (CDC)
- Event-driven activations
- Low-latency delivery (<5s)

📅 **v2.0 (Oct 2026)** — Intelligent Routing
- ML-based destination selection
- Automatic mapping optimization
- Cost-aware destination selection
- Performance analytics

📅 **v2.5 (Q4 2026)** — Compliance & Governance
- GDPR-compliant deletion
- Data lineage tracking
- Audit logging & compliance
- SOX/HIPAA reporting

📅 **v3.0 (Q1 2027)** — Enterprise Scale
- Multi-tenant support
- Distributed execution
- 99.99% uptime SLA
- Advanced monitoring

## Integration Points

- **Destinations:** Salesforce, HubSpot, Marketo, Zendesk, Intercom (20+)
- **Workflow Tools:** n8n, Power Automate, Temporal, Airflow
- **Data Sources:** Snowflake, BigQuery, Redshift, PostgreSQL
- **Frameworks:** Census, Hightouch, RudderStack

## Reliability & Data Integrity (external critique, verified real gaps)

The production sync path (`core/src/executor.rs::execute_sync`) has real infrastructure for these built elsewhere in the crate, but it's unwired from the path actually invoked by the CLI/Python bindings:

- **Retry-with-backoff bypassed in production adapters**: a solid `RetryPolicy` exists (`core/src/adapters/retry_policy.rs`, `core/src/governance/retry_policy.rs`) wired into `adapters/http_client.rs`, but the live HubSpot/Salesforce/Marketo adapters call `reqwest::blocking::Client` directly (e.g. `hubspot.rs:150-169`), bypassing it entirely — a failed request or 429 just increments a failure counter with no retry.
- **Schema-drift detection stubbed**: `SchemaEvolution`/`DefaultSchemaEvolution` (`core/src/governance/schema_evolution.rs`) always returns `Ok(vec![])` ("would compare against registered schema" per its own comment), and even that stub is only wired into `ActivationPipeline`, which the real CLI path (`execute_sync`) never calls. A source schema change today only surfaces as per-record HTTP errors.
- **No dry-run mode**: no `--dry-run`/`dry_run` flag anywhere in `python/pyreverseetl/cli.py` or `core/src/executor.rs` — no way to audit payloads before they hit HubSpot/Salesforce/etc.
- **No local idempotency ledger**: no record-level "already synced" state store — duplicate protection relies entirely on destination-side upsert keys (real, e.g. HubSpot `idProperty=email`, Salesforce external-ID upsert), which don't cover crash-mid-batch recovery, no-op-change detection, or adapters with no upsert semantics at all (e.g. the webhook adapter just POSTs).

## Priority Features

1. **Destination Ecosystem** (Q3 2026) — CRM/Marketing integrations
2. **Streaming Activation** (Q3 2026) — Real-time sync
3. **Intelligent Routing** (Q4 2026) — Smart optimization
4. **Compliance** (Q4 2026) — Enterprise governance

## Community

Contribute:
https://github.com/Mullassery/PyReverseETL/issues
