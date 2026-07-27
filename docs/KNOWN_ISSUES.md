# PyReverseETL - Known Issues

**Last Updated:** 2026-07-20  
**Version:** 2.0.1  
**Status:** ✅ Published to PyPI (use v2.0.1 for production)

---

## Build Issues (Local Development)

### 49 Semantic Errors in `core/src/connectors/connectors_db.rs`

**Severity:** 🔴 Critical (local builds fail)  
**Affected Versions:** v2.0.2+ (attempted rebuild)  
**Status:** Deferred to maintenance phase  
**Workaround:** Use published v2.0.1 from PyPI

#### Error Categories

| Error Type | Count | Example | Mitigation |
|-----------|-------|---------|-----------|
| Type Mismatches (E0308) | 8 | `Capability` vs `String` | Need trait implementation |
| Unimplemented Traits (E0046) | 5 | Missing methods in ConnectorInfo | Schema redesign needed |
| Argument Mismatches (E0050, E0061) | 12 | Wrong parameter count | Function signature review |
| Lifetime Issues (E0195) | 3 | Invalid lifetime parameter | Rust lifetime analysis |
| Type Constraints (E0277) | 8 | Type not allowed in position | Type system redesign |
| Other (E0282, E0432, E0560) | 6 | Inference, imports, fields | Various fixes needed |

#### Specific Issues

1. **E0308 Type Mismatch (8 instances)**
   - Location: `core/src/testing/harness.rs:258`
   - Problem: `connector.capabilities[0]` returns `String`, but expects `Capability` enum
   - Cause: Connector registry stores capabilities as strings, but test harness expects enum
   - Fix: Implement bidirectional conversion or modify storage format

2. **E0046 Missing Trait Implementation (5 instances)**
   - Problem: ConnectorInfo struct missing required trait methods
   - Cause: Likely from recent trait changes that affected existing implementations
   - Fix: Complete trait implementation or update interface

3. **E0050 Function Signature Mismatch (12 instances)**
   - Problem: Expected X parameters, found Y
   - Cause: ConnectorTest::new() signature changed or callers not updated
   - Fix: Update all callers or revert signature

---

## Brace Mismatch (FIXED in d0d5771)

**Status:** ✅ Fixed  
**Commit:** d0d5771  
**Changes:**
- Removed premature closing brace at line 1802 (was ending impl block)
- Removed duplicate closing brace at line 2321
- Category functions (finance, hr_payroll, etc.) now properly inside impl ConnectorRegistry

**Result:** Syntax errors resolved; semantic errors now visible

---

## Rust Toolchain

**Current:** 1.97  
**Edition:** 2021  
**Status:** ✅ Up to date

Builds successfully with Rust 1.97.1; semantic errors are code-level issues, not toolchain issues.

---

## Workarounds

### For Production
```bash
# Use published v2.0.1 from PyPI
pip install pyreverseetl==2.0.1
```

### For Development (if needed)
1. Use v2.0.1 as baseline for any modifications
2. Don't attempt local rebuild; apply changes manually
3. Test via CI/CD pipeline in GitHub Actions

### CI/CD Considerations
- GitHub Actions with Rust 1.97 can now attempt builds
- Expected to fail on same 49 semantic errors
- Consider fixing as part of next major version

---

## Fix Roadmap

### Phase 1: Analysis (1-2 days)
- Map all 49 errors to root causes
- Identify schema redesign requirements
- Review connector registry architecture

### Phase 2: Design (3-5 days)
- Decide on trait/enum strategy
- Plan backward compatibility
- Document new connector interface

### Phase 3: Implementation (5-10 days)
- Implement trait changes
- Update all connectors (40+ implementations)
- Update test harness
- Update documentation

### Phase 4: Testing (2-3 days)
- Unit tests for each connector
- Integration tests
- End-to-end activation tests

**Total Estimated Effort:** 11-20 days  
**Recommended:** Next major version (v3.0) after current workload stabilizes

---

## Impact Assessment

**Current State:**
- ✅ v2.0.1 works reliably on PyPI
- ✅ All connector implementations functional
- ❌ Cannot rebuild from source locally
- ❌ CI/CD builds fail

**User Impact:**
- ✅ pip install pyreverseetl==2.0.1 works fine
- ❌ Contributors cannot build locally
- ❌ Dependency updates trigger rebuild failures

**Risk Level:** Low for users; Medium for developers/contributors

---

## Prevention

To prevent similar issues in future:

1. **Pre-commit Hooks:** Validate Rust syntax before committing
2. **CI/CD Required:** Require successful CI build before merge
3. **Local Dev:** Enforce `cargo build` in pre-push hooks
4. **Documentation:** Document connector registry schema contract

---

## References

- Commit with partial fix: d0d5771
- Previous Rust syntax errors: Earlier commits show progression
- Rust Edition: 2021 (no edition2024 features used)
- Rust Toolchain: Updated from 1.81 to 1.97 (fixed brace visibility)

---

**Status:** Known, documented, deferred  
**Last Review:** 2026-07-20
