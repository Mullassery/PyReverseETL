# ROADMAP_HONEST.md

Blunt, evidence-based status as of 2026-09-20 (commit `5fa7ecf` + this
pass's uncommitted-at-time-of-writing changes). This file exists alongside
`docs/ROADMAP.md` (which covers the same ground in narrative form) to give
one place with an explicit tested/untested/broken split and a concrete
technical-debt list, per this repo's documentation-honesty convention.
Nothing below is hedged with "planned" or "may be added" for things that
simply don't exist — if it's not built, it says "not built."

## Real bugs found and fixed by a deep code-review pass (2026-09-27)

- **False schema-drift alarm on an empty batch, `core/src/executor.rs`
  (schema-drift block inside `execute_with_records`).** The real schema-drift
  detector wired into `execute_sync` (commit `022ddda`) compares the field
  shape of the current run's records against the last-known persisted shape.
  When a run reads **zero** records (e.g. an incremental query that
  legitimately finds nothing new since the last run), `batch_shape_entity`
  of an empty record set produces an empty shape, and diffing "empty"
  against a real, non-empty stored shape is indistinguishable from "every
  field just vanished" — `ExecutionResult::schema_changes` would report every
  previously-known field as `Removed`, purely because there was no data this
  run, not because of any real upstream schema change. Confirmed with a
  failing test before touching any logic (added
  `empty_batch_does_not_falsely_report_every_field_removed` in
  `core/src/executor.rs`, which failed with exactly this symptom:
  `["Field 'email' removed (was: string)", "Field 'id' removed (was:
  number)", "Field 'name' removed (was: string)"]` for a run with 0 records).
  **Fixed**: schema-drift detection is now skipped entirely when
  `compliant_records` is empty, so the persisted shape is left untouched and
  the next non-empty run still diffs correctly against the real last-known
  shape (same test's third run asserts this explicitly). Verified:
  `cargo test -p pyreverseetl-core --lib` — 453 passed, 0 failed, 6 ignored
  (452 before this pass, +1 new regression test); `cargo fmt --check` clean;
  `PYTHONPATH=python pytest tests/ -v` unaffected (5 passed, 4 skipped, no
  Python code touched).

- **Two TOCTOU races in `BackpressureManager`, `core/src/pipeline/backpressure.rs`
  (`acquire`/`release`) — both verified with real concurrent-thread tests,
  both fixed.** `acquire()` called `check_load()` (a separate atomic read)
  and then, only if that read said `Ok`/`Warn`, did a separate
  `fetch_add(1, ...)` — check and increment were two distinct atomic
  operations, not one. `release()` had the identical shape: a separate load
  of `current`, then an unconditional `fetch_sub(1, ...)` if `current > 0`.
  `ActivationPipeline` holds exactly one shared `Arc<BackpressureManager>`
  across concurrently processed events, so both were reachable by real
  concurrent callers, not just a theoretical concern.
  - Verified `acquire()`'s race with a test that drives load to
    `reject_threshold - 1` (94 of 100) and fires 20 real OS threads at the
    real, public `acquire()` simultaneously via a `Barrier`: **all 20
    succeeded**, taking `queue_depth` to 114 — 19 over the configured
    "hard" reject cap of 95 — where sequential semantics only allow exactly
    1 to succeed.
  - Verified `release()`'s race is worse than a transient overshoot: with
    `queue_depth` at 1, firing 2 concurrent real `release()` calls
    reproducibly took the *unsigned* `current_load` counter from 1 to 0 to a
    wrapped `usize::MAX` (`18446744073709551615`, confirmed across 5 runs).
    Once wrapped, `check_load()` would see an enormous load forever and
    permanently return `Reject` — a total, unrecoverable lockout of the
    backpressure manager (every future event rejected until process
    restart), not just a brief cap overshoot.
  - **Fixed**: both `acquire()` and `release()` now use a single atomic
    `AtomicUsize::fetch_update` (a compare-and-swap retry loop) instead of a
    separate load-then-branch-then-mutate. `acquire()`'s closure re-checks
    `current >= reject_threshold` against the freshly-read value on every
    retry (so only as many concurrent callers as there are actually-available
    slots can win); `release()`'s closure uses `checked_sub(1)` (so it can
    never go below zero regardless of how many more `release()` calls than
    acquired slots race in). `check_load()`'s duplicated inline threshold
    computation was also factored into a shared private `reject_threshold()`
    helper used by both.
  - Regression tests (real concurrent OS threads via `std::thread` +
    `Barrier`, not single-threaded unit tests that can't exercise a race at
    all): `concurrent_acquire_never_overshoots_the_reject_threshold` (asserts
    exactly 1 of 20 concurrent callers wins the last slot, final
    `queue_depth == 95`) and `concurrent_release_never_underflows_below_zero`
    (10 concurrent `release()` calls against 1 acquired slot; asserts
    `queue_depth` saturates at 0, never wraps). Both re-run 10x in a loop
    during this pass with zero failures (the pre-fix versions of these tests
    failed reliably, every single run).
  - Verified: `cargo test -p pyreverseetl-core --lib` — 455 passed, 0 failed,
    6 ignored (453 after the schema-drift fix above, +2 new backpressure
    regression tests); `cargo fmt --check` and `cargo build -p
    pyreverseetl-core` clean (same pre-existing 25 warnings, none new);
    `PYTHONPATH=python pytest tests/ -v` unaffected (5 passed, 4 skipped, no
    Python code touched).

## Bucket 1 — Verified working (I ran these commands myself, 2026-09-20)

- `cargo build -p pyreverseetl-core` — succeeds. 25 compiler warnings (see
  Technical Debt; was 26 before this pass's `base64::encode` fix).
- `cargo test -p pyreverseetl-core --lib` — **452 passed, 0 failed, 6
  ignored** (the ignored ones need real Docker containers per README).
- `cargo fmt --check` — clean, no diff.
- `PYTHONPATH=python pytest tests/ -v` (using a prebuilt
  `_core.cpython-311-darwin.so`, Python 3.11) — **5 passed, 4 skipped**. The
  4 skipped are the Docker-backed end-to-end tests
  (`tests/test_real_sync_docker.py`), skipped because Docker wasn't running
  in this environment, not because they fail.
- Postgres, MySQL, S3/MinIO connectors — real `sqlx`/`aws-sdk-s3` clients,
  covered by the ignored Docker tests above (I did not run those against
  live containers myself this pass; README/ROADMAP.md's prior claim that
  they were verified against real containers is plausible given the test
  code exists and calls real client libraries, but I did not re-run them).
- `.gitattributes` no longer misrepresents this as a pure-Python repo (see
  Technical Debt / Discoverability fix below) — verified by reading the
  file after editing it.

## Bucket 2 — Real, but only mock-tested or otherwise partially unverified

- Salesforce/HubSpot/Marketo adapters (`core/src/adapters/{salesforce,hubspot,marketo}.rs`)
  — real REST/OAuth2 client code, exercised by unit tests against a
  purpose-built mock HTTP server (`core/src/testing/mock_http.rs`), **not**
  against a live account. No live account was available in this sandbox
  either. Anyone relying on these against production Salesforce/HubSpot/
  Marketo should expect the request shape to possibly not match reality
  exactly.
- Retry-with-backoff / dry-run / schema-drift / idempotency — wired into
  the real `execute_sync` path (commit `022ddda`) and covered by unit
  tests; not exercised against a live third-party API, only mocks.
- ~~`python/pyreverseetl/server.py` (197 lines, the optional Flask REST
  wrapper) — **zero test coverage**. Nothing in `tests/` imports or
  exercises it. This is a real gap: it's a network-facing surface
  (`create_flask_app`) with no automated verification at all.~~
  **FIXED (2026-10-16)**: added `tests/test_server.py` (17 tests). It covers
  every delegation method (`create_workflow`/`create_activation`/`list_*`/
  `get_metrics`/`get_run_status`/`get_lineage`/`health_check`), the
  not-found fast paths, and — via Flask's `test_client` (skipped when Flask
  isn't installed, since it isn't in the `dev` extra) — every route's
  status-code and response-shape contract, including the `400`s for a
  missing `workflow_id`/`activation_id` and the `text/vnd.graphviz`
  content-type on `?format=dot`. Tests isolate state via an env-var override
  of the CLI state file and never trigger a live sync (only the
  "activation not found" path of `execute_activation` is exercised).
- `python/pyreverseetl/cli.py` (639 lines) — only 2 of its code paths have
  direct unit tests (`test_cli_execute_parses_dry_run_and_store_path_flags`,
  `test_cli_version_reports_the_real_installed_version`); the rest
  (`create-workflow`, `create-activation`, `lineage`, etc. subcommands) are
  only exercised by the Docker-gated e2e tests, which were skipped in this
  run and in any CI run without Docker services.

## Bucket 3 — Present in the codebase, not wired into the real sync path (experimental)

Unchanged from README/docs/ROADMAP.md's existing disclosure, restated here
for completeness:
- Kafka/CDC streaming (`core/src/sources/kafka.rs`), HDFS, PySpark
  transforms (`core/src/transformers/spark.rs` — `SparkTransformer::submit`
  is documented in its own doc-comment, line 203, as returning "fixed,
  fabricated" numbers).
- The CLI stats dashboard (`core/bin/pyreverseetl_dashboard.rs`, built as
  the real `pyreverseetl-dashboard` binary target). Its docs
  (`docs/archive/CLI_DASHBOARD.md`, `docs/archive/DASHBOARD_IMPLEMENTATION_SUMMARY.md`)
  claimed "Production-Ready (0 errors)" in July 2026; that claim was never
  independently verified and contradicts the current README's own
  "experimental, not wired in" framing, so those two docs were moved to
  `docs/archive/` with correction banners this pass.
- StatGuardian quality-gate integration
  (`core/src/governance/quality_gate.rs`) — `StatGuardianGate`'s `endpoint`
  and `timeout_ms` fields are compiler-flagged dead code (never read); its
  `validate`/`check_drift` methods take an `entity` parameter that's also
  unused, and **always return a hardcoded `passed: true, quality_score:
  0.95`** (lines 66-67) regardless of input — it never actually calls
  StatGuardian or inspects the entity. This is not a partially-working
  feature — it always reports success and does nothing. It is not on the
  real `execute_sync` path (per README), so it cannot currently mislead a
  real sync run, but if it were wired in as-is it would silently rubber-
  stamp everything. **(2026-09-21: re-verified with grep that no non-test
  code anywhere in `core/src` constructs a `GovernanceEngine` or calls
  `with_governance` — genuinely unused outside tests, confirmed rather than
  assumed. Added explicit `///` doc-comment disclosure on the struct and
  both methods; logic intentionally left unchanged pending the real
  wire-it-or-delete-it decision below.)**

## Bucket 4 — Not built / explicit non-goals

- Cloud object storage beyond S3/MinIO (Azure Blob, GCS, etc.) — returns an
  explicit "not implemented" error (`core/src/connectors/object_storage.rs`,
  see `gcs_provider_is_honestly_rejected_not_faked` test at line 921). Not
  scaffolded, not planned in this repo's current form.
- Parquet/Avro/ORC/Iceberg/Delta formats for the S3 connector — declared in
  the type system but return "not implemented" (same file, line ~902's
  test: `parquet_format_is_honestly_rejected_not_faked`).
- Any encryption implementation for the `Encrypt` compliance action — the
  compliance engine honestly reports this as unresolved rather than
  claiming success. There is no encryption code to point to.

## Technical debt (concrete findings, file:line)

### Warrants a dedicated follow-up session

1. **482 clippy warnings** on `cargo clippy -p pyreverseetl-core --lib`
   (verified 2026-09-20, now 481 after fix below — the deprecated
   `base64::encode` call was one of them). Not spot-fixed in this pass
   because the volume needs a dedicated session to avoid rubber-stamping
   `cargo clippy --fix` over code nobody's reviewed. Concrete categories
   still open: dead struct fields (`core/src/adapters/http_client.rs:10`
   `timeout` never read; `core/src/connectors/config.rs:79-80`
   `source_instances`/`destination_instances` never read;
   `core/src/governance/quality_gate.rs:47-48` `endpoint`/`timeout_ms` never
   read; `core/src/pipeline/activation_pipeline.rs:53` `activation` never
   read; `core/src/sources/kafka.rs:70` `last_message_time` never read),
   unused `vec!` where an array would do
   (`core/src/connectors/registry.rs:173,177,184,188`), unused imports
   (`core/src/connectors/registry.rs:4`), unused parameters
   (`core/src/governance/quality_gate.rs:62,73`,
   `core/src/transformers/spark.rs:179`).
   - **FIXED (2026-09-21)**: the deprecated `base64::encode` call at
     `core/src/adapters/http_client.rs:142` — migrated to
     `base64::engine::general_purpose::STANDARD.encode(...)` via the
     `Engine` trait, matching the existing pattern in
     `core/src/adapters/webhook.rs`. Verified: builds clean, 452 tests
     still pass.
    - **FIXED (2026-10-16)**: all 481 `clippy --lib` warnings cleared — the
      library now compiles with **0** warnings. 425 of them were
      `clippy::useless_vec` (mostly `core/src/connectors/connectors_db.rs`),
      auto-fixed with reviewed `cargo clippy --fix`; the remaining ~16 were
      fixed by hand. The dead fields called out above were **wired into real
      behavior**, not suppressed: `http_client.rs` `timeout`/`base_url` gained
      getters; `connectors/config.rs` `source_instances`/`destination_instances`
      gained source/destination instance-cache accessors; `pipeline/
      activation_pipeline.rs` `activation` gained a getter;
      `sources/kafka.rs` `last_message_time` gained a getter and is now set
      in `next_event`; plus a `Default` impl for retry policy, `clamp` fixes,
      `map_while(Result::ok)`, `sort_by_key`, and unused imports removed.
      Verified: `cargo fmt -p pyreverseetl-core -- --check` clean;
      `cargo clippy -p pyreverseetl-core --lib` 0 warnings;
      `cargo test -p pyreverseetl-core` — 455 passed, 0 failed, 6 ignored.
      (Caveat: `cargo clippy --all-targets` — test/bin/bench targets — still
      emits warnings; this repo's CI has no clippy job, so that was out of
      scope for the gated `--lib` cleanup.)
2. ~~**`.cargo/config.toml` hardcodes one contributor's personal machine.**~~
   **FIXED (2026-09-21)**: removed `[build] jobs = 12` (a "Mac Studio" core
   count) and the `-mcpu=apple-a14` rustflag entirely; `jobs` now falls back
   to Cargo's default and macOS builds no longer pin a CPU
   microarchitecture. Verified `cargo build -p pyreverseetl-core` and
   `cargo fmt --check` succeed on a different machine. Note: at the time,
   `cargo lint` (`clippy --all --release -- -D warnings`) also failed — that
   was entirely due to the pre-existing 481 clippy warnings in finding #1
   above, not caused by this config fix. Those `--lib` warnings are now
   cleared (2026-10-16); `--all-targets` (tests/bins/benches) warnings remain,
   so `clippy --all -- -D warnings` would still flag those.
3. **`sqlx` was exact-pinned to `=0.7.4`** (`core/Cargo.toml:33`) and the
   build already emits a "will be rejected by a future version of Rust"
   future-incompatibility warning for `sqlx-postgres v0.7.4`. **PARTIALLY
   FIXED (2026-10-16)**: the exact `=` pin was relaxed to `"0.7.4"` (caret)
   so routine maintenance and Dependabot can move it (`0.7.4` is already the
   last `0.7.x` release, so nothing in the lockfile changed and no rebuild
   was needed).    The future-incompatibility warning *has been cleared* by the `sqlx` 0.7.4 → 0.8.6 migration
   (the breaking 0.7→0.8 bump touched the `postgres`/`mysql` connectors and
   `runtime-tokio-rustls` feature flag; `rusqlite` 0.30 → 0.32 keeps the shared
   `libsqlite3-sys` linkage consistent). That 0.7 → 0.8 migration touches the
  `postgres`/`mysql` connectors (`PgPoolOptions`/`MySqlPoolOptions`,
  `query`/`bind`/`fetch_*`, `Row`/`Column`/`TypeInfo`) and sqlx's feature
  flags (`runtime-tokio-rustls`), so it was left as a dedicated follow-up
   rather than rushed. Relaxing the pin also unblocks Dependabot, which can
   now open a PR against sqlx — contrast with `opentelemetry`, `reqwest`,
   `rusqlite`, and `thiserror`,
   which are loosely pinned and each already have an **open, unmerged**
   Dependabot PR sitting in this repo (`dependabot/cargo/opentelemetry-0.32`,
   `dependabot/cargo/opentelemetry-http-0.32`, `dependabot/cargo/reqwest-0.13`,
   `dependabot/cargo/rusqlite-0.40`, `dependabot/cargo/thiserror-2.0`, plus
   `dependabot/github_actions/actions/checkout-7` and
   `.../actions/setup-python-7`). Current versions in `core/Cargo.toml`:
   `opentelemetry = "0.22"` (7 minor versions behind), `reqwest = "0.11"`
   (2 majors behind), `rusqlite = "0.30"` (10 minors behind), `thiserror =
   "1.0"` (a major version behind). None of these are known-CVE at time of
   writing as far as this pass could check (no network access to RustSec's
   advisory DB in this sandbox — `cargo audit` failed with a network error,
   expected per this environment's constraints), but the sqlx exact-pin
   specifically blocks even routine maintenance. **FIXED (2026-10-16):** migrated
   `sqlx` 0.7.4 → 0.8.6 (the breaking 0.7→0.8 bump touched the `postgres`/`mysql`
   connectors' `PgPoolOptions`/`MySqlPoolOptions`, `query`/`bind`/`fetch_*`,
   `Row`/`Column`/`TypeInfo` and the `runtime-tokio-rustls` feature flag, plus
   `rusqlite` 0.30 → 0.32 to keep the shared `libsqlite3-sys` linkage consistent).
   RUSTSEC-2024-0363 and the future-incompatibility warning are both cleared;
   `cargo test -p pyreverseetl-core --lib` = 455 passed, 0 failed, 6 ignored.
   The exact-pin was previously relaxed to `"0.7.4"` (caret) so Dependabot can
   now open PRs against sqlx; contrast with `opentelemetry`, `reqwest`, `rusqlite`, and `thiserror`, which are loosely pinned and each already have an open Dependabot PR in this repo.
4. ~~**Zero test coverage on `python/pyreverseetl/server.py`** (the Flask
   REST wrapper) — see Bucket 2 above. This is a network-facing surface
   with no tests at all, not even a smoke test that the app object
   constructs.~~ **FIXED (2026-10-16)**: see Bucket 2 above —
   `tests/test_server.py` now covers all delegation methods and every Flask
   route.
5. ~~**CI never actually enforces Python lint.**~~ **FIXED (2026-09-21)**:
   the earlier count of 28 was scoped to `ruff check python/` only; CI
   actually runs `ruff check .` (repo-wide), which had **58** real findings
   at the time of this fix (`python/`, `tests/`, and `examples/` combined).
   All 58 fixed for real (22 safe auto-fixes, 39 more via reviewed
   `--unsafe-fixes` — typing modernization to `X | None`/`dict`/`list` with
   `from __future__ import annotations`, safe given `requires-python =
   ">=3.9"` — plus 4 `datetime.now()` → `datetime.now(timezone.utc)`, 2
   `chmod +x` for shebang'd scripts, and 5 intentional blind-`except`
   call sites documented in place with `# noqa: BLE001` + a one-line
   rationale rather than rewritten). Added `ruff>=0.6` to
   `pyproject.toml`'s `dev` extra (CI's install step never actually
   installed it before) and removed the `|| true` from
   `.github/workflows/tests.yml`. `ruff check .` now exits 0 and is a real,
   enforced gate. CI also now runs `cargo audit` (fails on new
   vulnerabilities) and pins the Rust toolchain to 1.97 to match
   `rust-toolchain.toml`.
6. **`.github/workflows/tests.yml` and `.github/workflows/ci.yml`
   substantially duplicate each other** (both run the Python test matrix on
   push/PR to main). Costs CI minutes and gives two places to keep in sync;
   `tests.yml` is also the less-complete one (older action versions,
   swallowed lint). Consolidating them is a judgment call about which one's
   design wins, so left as a documented gap rather than merged unilaterally.

### Safe to defer, lower priority

7. `docker-compose.local.yml` hardcodes weak default passwords
   (`test_password`, `root_password`, `admin` for Grafana at lines 42, 61,
   63, 83, 197) for **local-only** dev containers. Low risk (not
   internet-exposed by default, not real credentials), but worth swapping
   for `${VAR:-default}` interpolation like `docker-compose.server.yml`
   already does, for consistency and so nobody copy-pastes this file
   straight into a shared environment.
8. ~~`rust-toolchain.toml` pins `channel = "1.97"` with no stated reason~~
   **FIXED (2026-09-21)**: added a comment explaining the pin is
   intentional (reproducible builds across contributors/CI) rather than an
   unexplained accident, and how to change it (bump deliberately, re-verify
   tests/clippy).

## What would most de-risk this project next

1. Get a live Salesforce, HubSpot, or Marketo sandbox account and run the
   adapters against it for real — this is the single biggest
   "plausibly-correct-but-never-actually-checked" surface (Bucket 2).
2. Decide the fate of Bucket 3 (Kafka/CDC, HDFS, PySpark, dashboard,
   StatGuardian): wire it for real or delete it. Half-present code that
   compiles but does nothing is a maintenance cost with no matching value
   today.
3. Fix technical-debt items #3 and #5 above together (they're both "CI
   currently can't tell you about a real problem") before adding new
   dependencies or new Python surface area.
