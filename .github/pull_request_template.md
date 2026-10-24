## What does this change do?

<!-- One or two sentences. If it touches a connector/adapter, name it. -->

## Why?

<!-- Link an issue if there is one. If not, explain the motivation. -->

## Honesty checklist

This project's README/ROADMAP_HONEST.md are held to a strict "no
overclaiming" standard - please keep your PR consistent with that:

- [ ] If this adds or changes a connector/adapter, it's backed by a real
      client against a real (or realistically mocked) backend - not a stub
      that returns fixed/fabricated data.
- [ ] If a feature is untested or only tested against a mock (no live
      account/service available), the PR description and any doc updates
      say so explicitly, rather than implying it's verified.
- [ ] `README.md` / `docs/ROADMAP.md` / `../docs/internal/ROADMAP_HONEST.md` are updated if
      this changes what's real vs. not-yet-wired.

## Testing

<!-- What did you actually run, and what was the result? -->

- [ ] `cargo test -p pyreverseetl-core --lib` passes
- [ ] `cargo clippy -p pyreverseetl-core --lib` reviewed (existing warnings
      are tracked in `../docs/internal/ROADMAP_HONEST.md`; please don't add new ones)
- [ ] `pytest tests/` passes (or documents why a case is skipped, e.g. no
      Docker)
