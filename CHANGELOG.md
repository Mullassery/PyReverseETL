# Changelog

All notable changes to this project are documented here. Format loosely
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

This file was created 2026-09-20 during a documentation/OSS-hygiene pass.
Entries before that date are reconstructed from `git log` and tags where the
history is unambiguous; the project's version history prior to 3.0.0 is
genuinely messy (see `docs/archive/`, and note that the `v2.0.0` git tag
does not point at the commit that actually bumped the version to 2.0.0 —
tags were applied inconsistently). Rather than fabricate a clean history
that didn't happen, gaps are left as gaps. For full detail, use
`git log --oneline`.

## [3.1.6] — 2026-10-18

### Added

- **Dead-letter queue (quarantine) for failed records.** Passing
  `dlq_store_path` to `run_sync` (or `--dlq-store <path>` to
  `pyreverseetl execute`) persists every record the destination rejects to a
  real SQLite dead-letter queue — run id, reason, and the exact payload — so
  a partial batch failure can be inspected and re-driven deliberately
  instead of re-reading the whole source. The count is reported as
  `rows_dead_lettered` on `PySyncResult` and in the CLI response. Applies to
  the adapter-based destinations (webhook/salesforce/hubspot/marketo), where
  per-record success/failure is observable. Addresses RECHECK-018.

## [3.1.5] — 2026-10-18

### Added

- **The `web` source connector is now reachable from Python via
  `run_sync`.** `pyreverseetl.run_sync(source_type="web", source_config=...)`
  accepts `seed_url` (required) plus optional `max_depth`, `max_urls`,
  `same_domain`, and `render_js`, so a Python sync can crawl a site with the
  MudFish engine and write one record per page to any destination —
  no Rust code required.

## [3.1.4] — 2026-10-18

### Added

- **Web source connector (`web` / `WebSource`), backed by the MudFish
  crawling engine.** A sync can now crawl a site (bounded by `max_depth` and
  a hard `max_urls` cap) and read one record per crawled page
  (`url`, `status_code`, `content_type`, `depth`, `title`,
  `meta_description`, `links`, `body_bytes`). It uses `mudfish_engine::crawl`
  directly, inheriting MudFish's post-resolution SSRF protection (private/
  loopback/link-local targets refused), robots.txt and per-host politeness,
  and streaming response-size caps, so it is safe to point at arbitrary
  URLs. Wired into `SourceSpec::Web` (`executor.rs`) and registered as a
  built-in source (`registry.rs`); `mudfish-core`/`mudfish-engine` are pinned
  to the `rust-v0.1.3` release tag.

## [3.1.3] — 2026-10-18

### Fixed

- **REST server now binds to loopback by default.** `PyReverseETLServer`
  and `run_server()` previously defaulted to `0.0.0.0`, exposing the
  unauthenticated REST API on every interface. They now default to
  `127.0.0.1`; exposing the API is an explicit choice via `host=` or the
  `PYREVERSEETL_HOST` environment variable (RECHECK-002).

## [3.1.2] — 2026-10-16

### Fixed

- **Dependency vulnerabilities cleared.** `cargo audit` now reports only one
  unfixable advisory (`rsa` 0.9.10, Marvin timing sidechannel, pulled in by
  `sqlx-mysql` for MySQL auth and flagged "no fixed upgrade available" by the
  advisory DB) plus three unmaintained-crate warnings. Fixed for real:
  - `sqlx` 0.7.4 → 0.8.6 (clears RUSTSEC-2024-0363, binary-protocol
    truncation/overflow), with `rusqlite` 0.30 → 0.32 to keep the shared
    `libsqlite3-sys` native linkage consistent.
  - `pyo3` 0.23.5 → 0.29 (clears RUSTSEC-2025-0020 buffer overflow and
    RUSTSEC-2026-0177 missing-`Sync` bound at the Python/Rust FFI boundary).
  - `reqwest` 0.11 → 0.12 and `aws-sdk-s3` default-features stripped of the
    `rustls` legacy-TLS feature (clears RUSTSEC-2026-0258 on h2 0.3,
    RUSTSEC-2026-0346/-0349 on rustls 0.21.12, and RUSTSEC-2026-0104/-0098/-
    0099 on rustls-webpki 0.101.7).
  - `cargo update` removed the unsound `lru` 0.16.4 (RUSTSEC-2026-0253)
    and `paste`/`rustls-pemfile`.
- **`cargo clippy -p pyreverseetl-core --lib -- -D warnings` is now clean
  (0 warnings; was 481).** CI now enforces it via a new `cargo audit` job and
  a pinned Rust toolchain (1.97) in `.github/workflows/ci.yml`.

## [3.1.1] — 2026-10-12

### Changed
- `core/src/transformers/spark.rs`: `execute_local` now reports the real
  elapsed wall-clock time for `execution_time_ms` instead of a hardcoded
  `5000`; the 1000-in/950-out record counts remain an explicitly documented
  simulation (no Spark job is run).
- Added an optional, isolated Graphify code knowledge-graph backend (`tools/graphify/`, `.graphifyignore`, optional workflow) and bumped version to 3.1.1.

### Fixed
- `core/src/governance/quality_gate.rs`: `StatGuardianGate::validate` and
  `::check_drift` no longer rubber-stamp every entity with a hardcoded
  `passed: true, quality_score: 0.95` / "no drift detected" result; they now
  delegate to a real `StatGuardianClient` (`/validate` and `/detect-changes`).
  A `with_api_key` constructor was added for authenticated deployments; `new`
  keeps sending no bearer token. An unreachable/erroring service now surfaces
  an error instead of a fabricated pass.
- `core/src/pipeline/backpressure.rs`: `BackpressureManager::acquire` and
  `::release` were both check-then-act (a separate atomic load, then a
  separate `fetch_add`/`fetch_sub`), not single atomic operations -- classic
  TOCTOU races reachable by real concurrent callers, since
  `ActivationPipeline` shares one `Arc<BackpressureManager>` across
  concurrently processed events. Verified both races with real
  multi-threaded tests before fixing: (1) 20 real OS threads calling
  `acquire()` concurrently from `queue_depth == 94` (one below the
  reject threshold of 95) all succeeded, taking `queue_depth` to 114 -- 19
  over the configured "hard" cap, where sequential semantics only allow 1 to
  succeed; (2) 2 real concurrent `release()` calls at `queue_depth == 1`
  reproducibly (5/5 runs) underflowed the *unsigned* counter to a wrapped
  `usize::MAX`, which would make `check_load()` return `Reject` forever --
  a permanent, unrecoverable lockout of the whole backpressure manager, not
  just a transient overshoot. Fixed both with a single atomic
  `AtomicUsize::fetch_update` (compare-and-swap retry loop) each --
  `acquire()` re-checks the threshold against the freshly-read value on
  every retry; `release()` uses `checked_sub` so it can never go below zero.
  Added two regression tests using real `std::thread` + `Barrier` (not
  single-threaded tests, which can't exercise a race at all):
  `concurrent_acquire_never_overshoots_the_reject_threshold` and
  `concurrent_release_never_underflows_below_zero`; both re-run 10x with
  zero failures post-fix (the pre-fix equivalents failed every single run).
  Verified: `cargo test -p pyreverseetl-core --lib` -- 455 passed, 0 failed,
  6 ignored; `cargo fmt --check` and `cargo build -p pyreverseetl-core`
  clean (same pre-existing 25 warnings, none new);
  `PYTHONPATH=python pytest tests/ -v` unaffected (5 passed, 4 skipped).
- `core/src/executor.rs`'s real schema-drift detection (wired into
  `execute_sync` per commit `022ddda`) falsely reported **every** field of a
  source->destination pair as "removed" whenever a sync run read zero
  records (e.g. an incremental query that legitimately returns no new rows).
  Root cause: `batch_shape_entity(&compliant_records)` of an empty record
  set infers an empty field shape, and `DefaultSchemaEvolution::detect_changes`
  then diffs that empty shape against the real, non-empty shape persisted
  from the last run with data -- which is indistinguishable, from inside
  `diff_shapes`, from "every field genuinely disappeared". This is a false
  schema-drift alarm caused entirely by there being no data this run, not by
  any real upstream schema change; a caller reading `ExecutionResult::
  schema_changes` (e.g. the CLI) would see something like `["Field 'email'
  removed...", "Field 'name' removed...", "Field 'id' removed..."]` for a
  run that simply had nothing to sync. Verified with a failing test first
  (`core/src/executor.rs`, `empty_batch_does_not_falsely_report_every_field_removed`)
  before fixing. Fix: schema-drift detection is now skipped entirely when
  `compliant_records` is empty, leaving the persisted shape untouched so the
  next non-empty run still diffs correctly against the real last-known
  shape (covered by the same test's third assertion, and by the pre-existing
  `schema_drift_is_detected_and_persisted_across_separate_runs` test, both of
  which still pass). Verified: `cargo test -p pyreverseetl-core --lib` — 453
  passed, 0 failed, 6 ignored (was 452 before this pass's one new test);
  `PYTHONPATH=python pytest tests/ -v` unaffected (5 passed, 4 skipped,
  Python code untouched by this fix).
- `core/src/adapters/http_client.rs:142` used the deprecated `base64::encode`
  free function (removed/deprecated upstream in favor of the `Engine` trait).
  Migrated to `base64::engine::general_purpose::STANDARD.encode(...)`,
  mirroring the pattern already used in `core/src/adapters/webhook.rs`.
  Verified with `cargo build -p pyreverseetl-core` (warning gone, 26 -> 25
  warnings) and `cargo test -p pyreverseetl-core --lib` (452 passed).
- `.cargo/config.toml` hardcoded one contributor's personal machine into
  every macOS build: `[build] jobs = 12` (a specific "Mac Studio" core
  count) and `rustflags = ["-C", "llvm-args=-mcpu=apple-a14"]` (an Apple
  A14-specific codegen target) under `[target.'cfg(target_os = "macos")']`.
  Removed both -- `jobs` now falls back to Cargo's own default (logical CPU
  count) and macOS builds no longer pin a CPU microarchitecture. Verified
  `cargo build -p pyreverseetl-core` and `cargo fmt --check` succeed on a
  non-"Mac Studio" machine. (`cargo lint`, i.e. `clippy --all --release -- -D
  warnings`, still fails -- that's the pre-existing 482 clippy warnings
  documented below, unrelated to and unaffected by this config change.)
- CI's Python lint gate (`.github/workflows/tests.yml`) ran
  `ruff check . 2>/dev/null || true`, silently swallowing every lint
  failure. Re-running `ruff check .` (the repo-wide scope CI actually uses,
  not just `python/`) found **58** real findings, not the 28 an earlier pass
  saw scoped to `python/` only. Fixed all 58:
  - 22 auto-fixed via `ruff check --fix` (unsorted imports, f-strings
    without placeholders, an unused import, unsorted `__all__`).
  - 39 more (some overlapping) via `ruff check --fix --unsafe-fixes`:
    `typing.Optional`/`Dict`/`List` rewritten to `X | None`/`dict`/`list`
    with `from __future__ import annotations` added
    (`python/pyreverseetl/cli.py`, `python/pyreverseetl/server.py` --
    project requires Python >=3.9, so this is a safe, semantics-preserving
    rewrite), plus one unused local variable removed.
  - 4 `datetime.now()` calls without a `tz` argument (`DTZ005`, in
    `tests/*.py` benchmark scripts) fixed by passing `timezone.utc`.
  - 2 files with a shebang but no execute bit (`EXE001`,
    `tests/reverse_etl_benchmark.py`, `tests/realistic_reverse_etl_benchmark.py`)
    fixed with `chmod +x`.
  - 5 blind `except Exception` findings (`BLE001`) are intentional top-level
    catch-all boundaries (the CLI's `main()` error handler;
    count-errors-and-keep-going benchmark loops) -- documented in place with
    `# noqa: BLE001` plus a one-line comment explaining why, rather than
    rewritten to catch narrower exception types that would need
    verification against real failure modes this pass couldn't check.
  - Added `ruff>=0.6` to the `dev` optional-dependency group
    (`pyproject.toml`) since CI's `pip install -e ".[dev]"` step never
    actually installed it before.
  - Removed `|| true` from `.github/workflows/tests.yml`'s Lint step --
    `ruff check .` is now a real, enforced CI gate.
  - Verified: `ruff check .` exits 0; `cargo test -p pyreverseetl-core --lib`
    still 452 passed; `PYTHONPATH=python pytest tests/ -v` still 5 passed, 4
    skipped.
- `core/src/governance/quality_gate.rs`: `StatGuardianGate::validate` and
  `check_drift` always returned a hardcoded `passed: true, quality_score:
  0.95` / "no drift" regardless of input, with no doc-comment disclosure at
  the type/method level (only an inline `// In production, this would...`
  comment). Verified it is genuinely not wired into any non-test code path
  (no non-test caller constructs a `GovernanceEngine` or calls
  `with_governance` anywhere in `core/src`). Added explicit `///` doc
  comments on `StatGuardianGate` and both methods disclosing this; logic
  unchanged.
- `rust-toolchain.toml` pinned `channel = "1.97"` with no stated reason.
  Added a comment explaining why it's pinned (reproducible builds) rather
  than left floating on `stable`.
- `Cargo.lock` is now committed (was gitignored despite the workspace
  producing a real `[[bin]]`, `pyreverseetl-dashboard`).
- `.gitattributes` no longer marks all `*.rs` as `linguist-vendored` /
  forces `linguist-language=Python` — this was hiding the fact that `core/`
  is a substantial Rust codebase from GitHub's language stats.
- Removed `src/pyreverseetl/__init__.py`, a dead, unreferenced duplicate of
  the real `python/pyreverseetl/` package (stale `__version__ = "2.1.1"`,
  not wired into `pyproject.toml`'s build config).
- `CONTRIBUTING.md` had an unresolved git merge-conflict marker
  (`<<<<<<< HEAD` / `=======` / `>>>>>>> origin/main`) committed to `main`,
  and its resolved text still said "Proprietary" after the project
  relicensed to Apache-2.0 (commit `724f00d`). Fixed.
- `.env.example` described generic `DATABASE_URL`/`API_KEY`/`REDIS_URL`
  variables that no code in this repo reads. Replaced with the actual
  environment variables the codebase reads (`STATGUARDIAN_TOKEN` /
  `STATGUARDIAN_API_KEY`, plus the Docker-test-only `PYREVERSEETL_TEST_*`
  vars) and a note that source/destination credentials are passed as CLI/API
  JSON config, not environment variables.
- `.github/workflows/tests.yml` used `actions/setup-python@v4`
  (`actionlint`-flagged as too old); bumped to `v5` to match `ci.yml`.
- `docs/ROADMAP.md` cited a stale Rust test count ("64 tests"); the actual
  count verified locally 2026-09-20 is 452 passing / 6 ignored.

### Added
- `.github/workflows/audit.yml` — `cargo audit` + `pip-audit` dependency
  vulnerability scan, on a weekly schedule and on manifest changes. Not
  verified to actually run green in CI (no network in the sandbox this was
  authored in to fetch the RustSec advisory database or PyPI index).
- `CODE_OF_CONDUCT.md`, `.github/pull_request_template.md`,
  `ROADMAP_HONEST.md` (this repo previously had `docs/ROADMAP.md` but not a
  root-level honesty-first roadmap with an explicit technical-debt section).

### Changed
- Moved `CLI_DASHBOARD.md`, `DASHBOARD_IMPLEMENTATION_SUMMARY.md` (root) and
  `docs/KNOWN_ISSUES.md` into `docs/archive/` with correction banners: their
  status claims ("Production-Ready, 0 errors", "49 semantic errors block
  every build") are stale/false against the current codebase.
- Deleted `.github/CI_ERRORS.md`: pure boilerplate whose only repo-specific
  content was a link to a `CI_ERRORS_REPORT.md` file that does not exist
  anywhere in this repo's history.

## [3.1.0] — 2026-09 (approximate, from commit history)
- Relicensed from Proprietary to Apache License 2.0 (commit `724f00d`).
- Retry-with-backoff, `--dry-run`, schema-drift detection, and an
  idempotency ledger wired into the real `execute_sync` path (commit
  `022ddda`).
- README rewritten to describe only what's genuinely implemented and
  verified, replacing earlier marketing-style claims (commits `4d4c3ce`,
  `a7bf72d`, `5fa7ecf`).

## [3.0.0] and earlier
Real Postgres/MySQL/S3 connectors, Salesforce/HubSpot/Marketo/Webhook
adapters, the lineage graph, and the compliance/PII-masking engine were
built incrementally across commits `c877394`, `14343b0`, `78b29c5`,
`2c14a35`, `a0f7fef`. Versions `1.5.0` through `2.1.2` (see git tags)
predate this changelog and are not reconstructed here in detail; several
intermediate commits (e.g. `92da7ec`, "Revert version bump: PyReverseETL
has code compilation issue") indicate the build was broken more than once
during this period — see `docs/archive/KNOWN_ISSUES.md` for a first-hand
(now-archived) account of one such period.
