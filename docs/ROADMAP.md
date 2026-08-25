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

## Reliability & Data Integrity (external critique, verified real gaps) — Done

The production sync path (`core/src/executor.rs::execute_sync`) had real infrastructure for these built elsewhere in the crate, but it was unwired from the path actually invoked by the CLI/Python bindings. All four are now fixed and wired into the real path, not a separate/dead one:

- **Retry-with-backoff bypassed in production adapters — fixed.** The live HubSpot/Salesforce/Marketo/webhook adapters used to call `reqwest::blocking::Client` directly, bypassing the crate's `RetryPolicy` entirely. `RetryPolicy` (`core/src/adapters/retry_policy.rs`) gained a synchronous `execute_blocking` (mirroring the existing async `execute`, using `std::thread::sleep` instead of `tokio::time::sleep` since these adapters run inside `executor.rs`'s `spawn_blocking`), and all four adapters now wrap their `upsert`/`delete`/`get_schema`/token-exchange calls with it. Verified with real retry-then-succeed tests against `MockHttpServer::start_sequence` (a new test helper that serves a scripted sequence of responses, e.g. two 429s then a 200) for every adapter.
- **Schema-drift detection stubbed — fixed.** `DefaultSchemaEvolution::detect_changes` (`core/src/governance/schema_evolution.rs`) used to unconditionally return `Ok(vec![])`. It now persists the last-seen field-name/type shape (in-memory or a real SQLite file) and does a real field-by-field diff against it, wired directly into `execute_with_records` (not the unused `ActivationPipeline`/`GovernanceEngine` path) via `ExecuteOptions::schema_store_path`. Deliberately scoped to *structural* shape tracking, not statistical data-quality drift — see `governance/quality_gate.rs`'s `StatGuardianGate`, which correctly remains a real-but-unwired integration point for that (per the architectural note in `error.rs`: distribution-drift/quality-score validation belongs to StatGuardian, not this crate).
- **No dry-run mode — fixed.** `ExecuteOptions::dry_run` (and `--dry-run` in the CLI) reads records from the real source and runs them through the real compliance engine exactly as normal, but never calls `write_to_destination` — zero HTTP/DB/object-storage calls reach the destination. `ExecutionResult::dry_run_preview` holds the exact post-compliance payload for every record that would have been sent, so it can be audited before a real run.
- **No local idempotency ledger — fixed.** A new `IdempotencyLedger` (`core/src/idempotency.rs`), a real SQLite-backed store keyed by `(destination, record_id)` with a content hash, is checked before and updated after every adapter-based write when `ExecuteOptions::idempotency_store_path` is set. Content-hash-based (not just id-based), so a re-run after a mid-batch crash skips records already synced but still sends a record whose content genuinely changed even if its id was seen before — including for the webhook adapter, which has no upsert semantics of its own.

## Priority Features

1. **Destination Ecosystem** (Q3 2026) — CRM/Marketing integrations
2. **Streaming Activation** (Q3 2026) — Real-time sync
3. **Intelligent Routing** (Q4 2026) — Smart optimization
4. **Compliance** (Q4 2026) — Enterprise governance

## Community

Contribute:
https://github.com/Mullassery/PyReverseETL/issues
