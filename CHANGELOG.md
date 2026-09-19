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

## [Unreleased]

### Fixed
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
