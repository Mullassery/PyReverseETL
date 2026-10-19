# Queue status — 2026-10-19

This file records what was shipped, what is blocked/failed, and what is rejected for this repository. It complements `AUDIT_FINDINGS.md`.

## Shipped
- File-based and live StatGuardian sample gates (3.3.0).
- Fernet-encrypted local CLI state with keyring/env key and legacy migration; Singer target protocol adapter (3.3.1).

## Blocked / failed
- Live Salesforce/HubSpot/Marketo verification (no accounts).
- Scheduler/run-history observability.
- CDC/streaming.

## Rejected / deferred
- Spark transforms and streaming CDC now — outside the proven sync path.

## Policy

Any future feature or upgrade must check `AUDIT_FINDINGS.md` and this file before implementation. A blocked item is retried only when its external dependency is available; a rejected item needs new user evidence or a design change to reopen.
