# Cross-Repository Blockers — PyReverseETL ↔ Squid-CRM

Detailed specification of the PyReverseETL → Squid-CRM integration blocker,
from the PyReverseETL perspective. The shared context is documented in
`Mullassery/Squid-CRM/docs/cross-repository-blockers.md`; this file records the
PyReverseETL side of the contract and what must change here to unblock it.

Reference pin: `twentyhq/twenty@08a0e5f1ee2e7db7c1e6ef3aeec9bcef731f4e98`
(Twenty server `2.50.0`).

## 1. The blocker

**Status:** RESOLVED IN WORKING TREE (mock-verified) — PyReverseETL now has a
Squid-CRM destination adapter with a documented payload contract and stable
`Idempotency-Key` headers. Live-account verification remains outstanding.

Squid-CRM can accept idempotent REST creates via a workspace-scoped
`Idempotency-Key` header with body-hash conflict detection and response replay.
The adapter now sets that header. CRM adapters (Salesforce/HubSpot/Marketo/Squid)
are mock-verified only, not live-account verified.

## 2. The two sides

| Aspect | Squid-CRM (destination) | PyReverseETL (source / activation) |
|--------|------------------------|-----------------------------------|
| Transport | REST `/rest/{object}` create; GraphQL `create<Object>`; custom-object REST `/rest/{object}` | Generic webhook destination; CRM adapters; Python connector plugins |
| Idempotency | `Idempotency-Key` header (1–255 printable ASCII), body-hash conflict, response replay, expiry | Squid adapter sends a stable per-record key (`<prefix>:<object>:<record-id>`); generic webhook still requires an idempotent endpoint |
| Payload shape | REST request body is the object input; responses use `data.create<Person>` / `data.create<Company>` | Squid adapter maps an explicit allowlist to native person/company input; generic webhook remains raw JSON |
| Batch | Custom-object `/rest/{object}/upsert` (idempotent: id present → update, absent → create) | Squid adapter is per-record; no bulk upsert/durable outbox yet |
| Auth | API key or session cookie | Bearer / API key / Basic on webhook destination |
| Retry | Bounded retries, `Retry-After`, durable `core.webhook_deliveries` log | Retry with backoff on transient HTTP failures; idempotency ledger for crash-mid-batch recovery |
| PII | No field-level PII masking in webhook payloads | `Encrypt` compliance action reported as unresolved (not faked) |

## 3. What was agreed and implemented (PyReverseETL side)

1. **`Idempotency-Key` header support.** The Squid adapter generates
   `<prefix>:<object>:<record-id>` and sends it on every create; retries reuse
   the same key. `X-Correlation-Id` uses the same value.
2. **Per-record payload contract.** People forward only `first_name`,
   `last_name`, `email`, `phone`, `job_title`, and `company_id` in Squid's
   native `{name, emails, ...}` shape. Companies forward `name` and `domain`
   in the native `{name, domainName}` shape.
3. **Batch vs single-record.** The adapter streams one record at a time and
   explicitly does not claim a bulk upsert path. Custom-object bulk upsert is
   a separate future contract.
4. **Durable outbox.** The existing local idempotency ledger and DLQ remain
   available, but there is still no durable outbox for failed Squid deliveries.
   This is a disclosed follow-up, not part of the header/payload blocker.
5. **Encryption.** PyReverseETL's `Encrypt` compliance action is still
   unresolved — PII masking before egress to Squid is not claimed.

Additional constraint: standard Squid object endpoints are create-only. The
adapter is idempotent for retries, but it does not update an existing Squid
person/company. A changed source record needs an explicit update workflow.

## 4. Evidence

**PyReverseETL:**
- Real Rust sync engine with Python bindings and CLI (`pyreverseetl execute`).
- PostgreSQL/MySQL/S3 as source and destination; generic-schema read/write,
  incremental reads, upsert. Verified with real containers.
- Webhook destination with real auth (Bearer / API key / Basic) and JSON payload
  construction. CRM adapters for Salesforce/HubSpot/Marketo are **mock-verified
  only** (local mock HTTP servers, no live accounts).
- Python connector plugins do **not** run in the Rust core path — they run
  through `run_with_connector`, which does not write the Rust lineage graph.
- `Encrypt` compliance action is honestly reported as unresolved rather than
  faked.

**Squid-CRM:**
- Idempotent REST creates are integration-tested. Custom-object batch upsert
  (`/rest/{object}/upsert`) is integration-tested.
- Python client `squid_crm` 0.3.0 on PyPI supports idempotency, correlation, and
  retry headers (12 tests passing).

## 5. Owner of next move

**PyReverseETL:** implemented in `core/src/adapters/squid.rs`, wired through
`DestinationSpec::Squid` and the Python `destination_type="squid"` path.
**Squid-CRM:** no further change is required for the create path; live
verification and an update contract remain follow-ups.

## 6. Related blockers

- **PyCustomerHub + Squid:** shared event-schema contract (OCDS envelope mapping)
  — gates Squid webhook delivery to the CDP.
- **PyTagShield + Squid:** consent decision contract — gates whether consent
  metadata is stored/emitted on records or webhooks.
- **ClusterAudienceKit / PyTagManager + Squid:** custom-object schemas — gates
  audience/finding writes to Squid custom objects.
