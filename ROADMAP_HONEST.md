# ROADMAP_HONEST.md

Blunt, evidence-based status as of 2026-09-20 (commit `5fa7ecf` + this
pass's uncommitted-at-time-of-writing changes). This file exists alongside
`docs/ROADMAP.md` (which covers the same ground in narrative form) to give
one place with an explicit tested/untested/broken split and a concrete
technical-debt list, per this repo's documentation-honesty convention.
Nothing below is hedged with "planned" or "may be added" for things that
simply don't exist — if it's not built, it says "not built."

## Bucket 1 — Verified working (I ran these commands myself, 2026-09-20)

- `cargo build -p pyreverseetl-core` — succeeds. 26 compiler warnings (see
  Technical Debt).
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
- `python/pyreverseetl/server.py` (197 lines, the optional Flask REST
  wrapper) — **zero test coverage**. Nothing in `tests/` imports or
  exercises it. This is a real gap: it's a network-facing surface
  (`create_flask_app`) with no automated verification at all.
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
  stamp everything.

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
   (verified 2026-09-20). Not spot-fixed in this pass because the volume
   needs a dedicated session to avoid rubber-stamping `cargo clippy --fix`
   over code nobody's reviewed. Concrete categories seen: dead struct
   fields (`core/src/adapters/http_client.rs:10` `timeout` never read;
   `core/src/connectors/config.rs:79-80` `source_instances`/
   `destination_instances` never read; `core/src/governance/quality_gate.rs:47-48`
   `endpoint`/`timeout_ms` never read; `core/src/pipeline/activation_pipeline.rs:53`
   `activation` never read; `core/src/sources/kafka.rs:70`
   `last_message_time` never read), unused `vec!` where an array would do
   (`core/src/connectors/registry.rs:173,177,184,188`), a deprecated API
   call (`base64::encode` at `core/src/adapters/http_client.rs:142` — the
   crate's own docs say to use `Engine::encode` instead), unused imports
   (`core/src/connectors/registry.rs:4`), unused parameters
   (`core/src/governance/quality_gate.rs:62,73`,
   `core/src/transformers/spark.rs:179`).
2. **`.cargo/config.toml` hardcodes one contributor's personal machine.**
   `rustflags = ["-C", "llvm-args=-mcpu=apple-a14"]` under
   `[target.'cfg(target_os = "macos")']` targets a specific Apple Silicon
   microarchitecture baseline for *every* macOS build of this project by
   *every* contributor, and `[build] jobs = 12` hardcodes a "Mac Studio"
   core count. This is committed project config, not a personal
   `~/.cargo/config.toml` — it affects every clone. Also, the `[alias] lint
   = "clippy --all --release -- -D warnings"` alias documented nowhere else
   would currently **fail** if anyone ran `cargo lint`, given finding #1
   above (482 warnings, `-D warnings` turns every one into a hard error).
   Needs a dedicated session to either genericize this file or remove the
   parts that shouldn't be committed at all.
3. **`sqlx` is exact-pinned to `=0.7.4`** (`core/Cargo.toml:33`) and the
   build already emits a "will be rejected by a future version of Rust"
   future-incompatibility warning for `sqlx-postgres v0.7.4`. The exact pin
   (`=`, not `^`) also means Dependabot cannot open a PR to bump it at all —
   contrast with `opentelemetry`, `reqwest`, `rusqlite`, and `thiserror`,
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
   specifically blocks even routine maintenance and should be relaxed.
4. **Zero test coverage on `python/pyreverseetl/server.py`** (the Flask
   REST wrapper) — see Bucket 2 above. This is a network-facing surface
   with no tests at all, not even a smoke test that the app object
   constructs.
5. **CI never actually enforces Python lint.**
   `.github/workflows/tests.yml` runs `ruff check . 2>/dev/null || true` —
   the `|| true` means a lint failure can never fail the build. Running
   `ruff check python/` locally (2026-09-20) found **28 real issues**
   (mostly `UP006`/`FA100` typing modernization and unsorted imports, e.g.
   `python/pyreverseetl/server.py:83,87,102,105`) that CI has been silently
   swallowing. Not fixed in this pass because turning on enforcement means
   either fixing all 28 first or the next PR's CI goes red for unrelated
   reasons — needs a dedicated session to do both at once.
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
8. `rust-toolchain.toml` pins `channel = "1.97"` with no stated reason;
   fine as-is, just flagging that nobody has documented why 1.97
   specifically vs. `stable`.

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
