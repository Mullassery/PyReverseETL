# Competitive intelligence in roadmap and upgrade work

**Policy date:** 2026-10-19

Competitive findings are an input to product decisions, not a one-off report.
Every pending roadmap item, new feature, and routine dependency upgrade should
check the repository's [`AUDIT_FINDINGS.md`](AUDIT_FINDINGS.md) before work
starts, and record the decision in the PR or changelog.

## Required check before building

1. **Read `AUDIT_FINDINGS.md`.** Find the relevant competitor row and the
   ranked next actions. If the finding is stale, update it with a dated source.
2. **Name the competitor and the user problem.** A feature is not justified by
   "competitor has it". State the workflow it unlocks, who needs it, and how
   often. Mark unverified demand as a hypothesis.
3. **Prefer reuse over new code.** Check whether an existing Mullassery repo
   already provides the capability (see `BUNDLES.md` in the profile repo and
   the bundle role file in this repo, when present). Typical reuse:
   - crawling/fetching/robots/SSRF → **MudFish**
   - architecture maps / PR diffs → **CodeAtlas**
   - data-quality contracts / validation → **StatGuardian**
   - token counting / cost tables → **PyTokenCalc**
   - dependency graph / advisories / SBOM → **PyDeepMesh**
   - segmentation / CLV / churn → **ClusterAudienceKit**
   - activation / lineage / PII → **PyReverseETL**
   - fleet state sync → **SyncFleet** (only after conflict resolution)
4. **Choose one disposition:** build, integrate, defer, or reject. Write it
   down. A rejection with a reason is a successful outcome.
5. **For routine upgrades**, check whether the upgrade closes a known
   competitor gap or breaks a differentiated capability. Run the tests that
   cover the affected path; do not merge a green build that silently changes a
   user-visible contract.

## PR / changelog checklist

- [ ] Competitor and source URL/date recorded (or "no direct competitor").
- [ ] User impact and frequency stated; unverified demand labelled.
- [ ] Reuse check done against sibling repos/bundles.
- [ ] Failure mode and rollback considered (especially auth, PII, cost).
- [ ] Docs/roadmap status updated honestly.
- [ ] Tests or a manual verification command included.

## Review cadence

- Refresh the competitor row when a release changes the capability.
- Re-check the full `AUDIT_FINDINGS.md` at least quarterly and before a major
  release.
- If a finding is wrong or stale, correct it in the same PR that relies on it.

## Non-goals

- Copying features for parity.
- Adding a dependency just because a competitor uses it.
- Treating star counts or download counts as proof of user need.

---

## OSS gap analysis — 2026-10-24

**Scope:** PyReverseETL 3.5.3 (Apache-2.0): Rust sync engine plus Python, CLI, and optional Flask surfaces. Compared the implemented activation path — sources, destinations, incremental/upsert, retries, idempotency, schema drift, PII actions, lineage, scheduling, and packaging. Excluded experimental Kafka/HDFS/Spark paths and unreleased roadmap claims.

### Comparable OSS projects

- **Multiwoven** — AGPL-3.0; active (last push 2026-10-24); 1,683 GitHub stars. Self-hosted reverse ETL with a UI control plane, models, syncs, and warehouse sources (Redshift, BigQuery, Snowflake, Databricks, PostgreSQL) to CRM, marketing, support, ads, and collaboration destinations.
- **drt** — Apache-2.0; active (last push 2026-10-24); 37 stars. Code-first reverse ETL with YAML syncs, `plan`/`apply`, delete guards, tests, dbt/CI integration, and an MCP server.
- **Airbyte** — Elastic License 2.0; active (last push 2026-10-24); 22,263 stars. ELT connector ecosystem, not a reverse-ETL control plane; used here as the connector-breadth and packaging benchmark.
- **Grouparoo** — MIT; archived, last push April 2022. The closest historical OSS customer-data sync framework; its archive is a maintenance-risk signal, not an active alternative.

### Capability comparison

| Capability | PyReverseETL | Multiwoven | drt | Airbyte |
|---|---|---|---|---|
| Warehouse as source (Snowflake/BigQuery/Redshift) | No | Yes | Yes | Partial |
| Postgres/MySQL/S3 sources | Yes | Partial | Yes | Yes |
| CRM/marketing destinations | Partial | Yes | Partial | No |
| Config-as-code sync definitions | Partial | No | Yes | Partial |
| Built-in scheduling/run history | No | Yes | Partial | Yes |
| Web UI/control plane | No | Yes | No | Yes |
| Field-level PII actions | Yes | No | Partial | No |
| Self-hosted without cloud dependency | Yes | Yes | Yes | Yes |

Nuance: our CRM row means Salesforce, HubSpot, Marketo, and Squid-CRM, all mock-verified only; Multiwoven lists checked integrations for Salesforce, HubSpot, Klaviyo, Zendesk, Facebook Ads, Slack, Sheets, and Airtable. "Config-as-code" is Partial because our sync state lives in a CLI state file and JSON flags, not reviewed YAML. Airbyte's warehouse-source cell is Partial because its product direction is warehouse-as-destination. Cells reflect reviewed public docs, not an exhaustive code audit.

### Gaps that matter (ranked)

1. **No warehouse sources.** Reverse ETL normally begins in Snowflake, BigQuery, Redshift, or Databricks; we support only Postgres, MySQL, S3/MinIO, and Python plugins. A warehouse team cannot adopt us without copying data into Postgres first. This is the category entry ticket and the largest gap (L).
2. **Destination breadth and live verification.** Four CRM/marketing adapters is thin against Multiwoven's list, and ours are explicitly not verified against live accounts. Missing Klaviyo, Zendesk, Braze, Slack, Google Sheets, Airtable, and Facebook Ads closes common activation use cases. Start with sandbox contract tests, not more mocks (M–L).
3. **No scheduler, durable run history, or alerting.** `run_sync` records lineage in-process; `lineage_json()` and run status disappear with the process. Operators must supply cron/Airflow and lose cross-run audit. Multiwoven ships a control plane; drt ships `status`, CI deployment, and structured JSON output. A persistent run store plus metrics/alerts is the minimum (M).
4. **No reviewed change set or destructive-write guard.** drt's `plan`/`apply` previews row-level creates/updates/deletes and refuses to apply a stale plan; our dry-run preview shows payloads but does not gate a real apply or stop mass overwrites. Add a plan artifact and delete threshold before widening destination support (M).
5. **Lineage is not durable.** The graph is real but per-process, so a long-running Flask service or repeated CLI calls cannot answer "what moved last week?" without external storage. Persist nodes/edges with run IDs and expose retention/export (M).
6. **Distribution blocks non-macOS users.** PyPI 3.5.3 publishes only a macOS arm64 wheel (plus sdist), and the latest GitHub release is v3.5.2. Linux/Windows wheels and one version story are adoption prerequisites, not polish (S–M).
7. **`Encrypt` is advertised but unresolved.** Mask/remove/truncate are implemented in the core write path and reported honestly, but encryption has no code. Either implement envelope encryption with key management or remove `Encrypt` from the public action list (S–M).

### Where we are differentiated

- **Compliance before egress is core behavior, not a UI setting.** Per-field mask/remove/truncate rules run inside the Rust sync path and violations are surfaced in the result; Multiwoven and drt do not advertise equivalent field-level controls, and Airbyte is ELT-first.
- **Retry safety is concrete.** A persisted idempotency ledger, content hashing, schema-drift detection, and Squid `Idempotency-Key` replay are wired into a real execution path and unit-tested. That is a stronger operational story than a dashboard-only competitor for failure-prone CRM writes.
- **Automation-first surfaces.** Python API, CLI, Flask API, JSON/DOT lineage export, and StatGuardian CLI gates fit existing orchestrators; no control-plane deployment is required.
- **License and footprint.** Apache-2.0 avoids Multiwoven's AGPL-3.0 and Airbyte's ELv2 hosted-service restriction, and the Rust core is lighter than a Rails/React or JVM control plane.
- **Honest connector table.** Documenting mock-only adapters and non-goals is rare and builds trust; it should remain a differentiator, not become permanent technical debt.

### Disposition

- **Build:** warehouse sources (Snowflake, BigQuery, Redshift first; Databricks next); five live-verified destinations (Klaviyo, Zendesk, Slack/Sheets, Braze, Facebook Ads); persistent run/lineage store with metrics and alerts; plan/apply with delete guard; Linux/Windows wheels and version reconciliation.
- **Integrate:** keep StatGuardian as the CLI pre-sync gate; ingest dbt `manifest.json` for model/column selection rather than inventing a modeling layer; emit OpenTelemetry metrics/traces.
- **Defer:** full web UI/control plane, Airbyte-scale ELT catalog, CDC/streaming, and the experimental Kafka/HDFS/Spark paths. These do not strengthen the compliance-first activation wedge.
- **Reject:** hosted-service parity with Census/Hightouch; it is a commercial motion, not an OSS feature gap.

### Sources

- Local evidence: `README.md`, `AUDIT_FINDINGS.md`, `ROADMAP_HONEST.md`, `pyproject.toml`, `Cargo.toml` (accessed 2026-10-24).
- https://github.com/Multiwoven/multiwoven (README, LICENSE; accessed 2026-10-24).
- https://github.com/drt-hub/drt (README; accessed 2026-10-24).
- https://github.com/airbytehq/airbyte (README, LICENSE; accessed 2026-10-24).
- https://github.com/grouparoo/grouparoo (archived status; accessed 2026-10-24).
