# Cross-Repository Blockers — PyReverseETL ↔ Squid-CRM

Detailed specification of the PyReverseETL → Squid-CRM integration blocker,
from the PyReverseETL perspective. The shared context is documented in
`Mullassery/Squid-CRM/docs/cross-repository-blockers.md`; this file records the
PyReverseETL side of the contract and what must change here to unblock it.

Reference pin: `twentyhq/twenty@08a0e5f1ee2e7db7c1e6ef3aeec9bcef731f4e98`
(Twenty server `2.50.0`).

## 1. The blocker

**Status:** BLOCKED — No mutually verified activation payload or idempotency
header contract between PyReverseETL and Squid-CRM.

Squid-CRM can accept idempotent REST creates via a workspace-scoped
`Idempotency-Key` header with body-hash conflict detection and response replay.
PyReverseETL cannot currently set that header, and its exact per-record payload
contract for Squid destinations is not available locally. CRM adapters
(Salesforce/HubSpot/Marketo) are mock-verified only, not live-account verified.

## 2. The two sides

| Aspect | Squid-CRM (destination) | PyReverseETL (source / activation) |
|--------|------------------------|-----------------------------------|
| Transport | REST `/rest/{object}` create; GraphQL `create<Object>`; custom-object REST `/rest/{object}` | Generic webhook destination; CRM adapters; Python connector plugins |
| Idempotency | `Idempotency-Key` header (1–255 printable ASCII), body-hash conflict, response replay, expiry | Cannot currently set the header; posts one record at a time via generic webhook |
| Payload shape | Squid REST create envelope (`data.create<Person>`, etc.) or custom-object `/rest/{object}` create | One record object at a time; no Squid per-object body contract |
| Batch | Custom-object `/rest/{object}/upsert` (idempotent: id present → update, absent → create) | No bulk upsert or durable outbox for Squid deliveries |
| Auth | API key or session cookie | Bearer / API key / Basic on webhook destination |
| Retry | Bounded retries, `Retry-After`, durable `core.webhook_deliveries` log | Retry with backoff on transient HTTP failures; idempotency ledger for crash-mid-batch recovery |
| PII | No field-level PII masking in webhook payloads | `Encrypt` compliance action reported as unresolved (not faked) |

## 3. What must be agreed (PyReverseETL side)

1. **`Idempotency-Key` header support.** PyReverseETL must generate and send a
   stable idempotency key per record, scoped to a Squid workspace, with
   body-hash semantics matching Squid's contract. Without this, retries
   duplicate CRM records.
2. **Per-record payload contract.** The exact JSON field allowlist that maps to
   Squid's REST create envelope (`data.create<Person>`, `data.create<Company>`,
   etc.) or custom-object `/rest/{object}` create must be defined.
3. **Batch vs single-record.** Squid supports custom-object batch upsert;
   PyReverseETL posts one record at a time. Decide whether to buffer N records
   then upsert, or stream single records — this affects throughput and idempotency
   key stability.
4. **Durable outbox.** PyReverseETL has an idempotency ledger for
   crash-mid-batch recovery, but no durable outbox for failed Squid deliveries.
   The retry/backoff contract must match Squid's bounded webhook retry semantics.
5. **Encryption.** PyReverseETL's `Encrypt` compliance action is unresolved —
   PII masking before egress to Squid is not guaranteed.

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
- Python client `squid_crm` 0.2.1 on PyPI supports idempotency, correlation, and
  retry headers (12 tests passing).

## 5. Owner of next move

**PyReverseETL** must add `Idempotency-Key` header support and document its
per-record payload shape for Squid destinations. Squid-CRM cannot unilaterally
resolve this.

## 6. Related blockers

- **PyCustomerHub + Squid:** shared event-schema contract (OCDS envelope mapping)
  — gates Squid webhook delivery to the CDP.
- **PyTagShield + Squid:** consent decision contract — gates whether consent
  metadata is stored/emitted on records or webhooks.
- **ClusterAudienceKit / PyTagManager + Squid:** custom-object schemas — gates
  audience/finding writes to Squid custom objects.
