# Phase 4 Week 1: Build System Error Analysis

**Analysis Date**: 2026-07-26  
**Build Status**: 🔴 Failed with 10+ semantic errors  
**Total Issues**: 15 (10 errors + 15+ warnings)  
**Critical Blocker**: Yes - Build system completely broken

---

## Error Summary

### Critical Errors (Build Blocking)

#### 1. Missing `ConnectorError` Import
**Error Type**: E0432 (unresolved import)  
**Severity**: 🔴 Critical  
**Files Affected**: 
- `core/src/connectors/mysql.rs` (line 6)
- `core/src/connectors/postgres.rs` (line 5)

**Root Cause**: 
MySQL and PostgreSQL connectors are trying to import `ConnectorError` from the connectors module, but it doesn't exist there. The main crate has `Error::ConnectorError(String)` in the error module, but not as a standalone type.

**Current Code**:
```rust
// mysql.rs line 6 (WRONG)
use crate::connectors::{..., ConnectorError, ...};
```

**Fix Strategy**:
Option A: Create a standalone `ConnectorError` type in connectors/error.rs (Recommended - better API)
Option B: Use `crate::Error` instead and convert (simpler but less specific)

**Recommended Fix**: Option A - Create dedicated ConnectorError type with variants for different error kinds.

---

#### 2. Missing `parking_lot` Dependency
**Error Type**: E0432 (unresolved import)  
**Severity**: 🔴 Critical  
**File**: `core/src/connectors/rate_limiting.rs:9`

**Root Cause**: 
Rate limiting code imports `parking_lot::Mutex` but the dependency isn't declared in Cargo.toml.

**Current Code**:
```rust
// rate_limiting.rs:9 (WRONG)
use parking_lot::Mutex;
```

**Fix Strategy**:
Add `parking_lot` to Cargo.toml dependencies. Check what version is compatible with other deps.

---

#### 3. Missing Trait Methods in MySQL Connector
**Error Type**: E0046 (missing trait items)  
**Severity**: 🔴 Critical  
**File**: `core/src/connectors/mysql.rs:123` (SourceConnector impl)

**Root Cause**: 
MySQLConnector implements SourceConnector trait but is missing required methods:
- `name()` - returns connector name
- `description()` - returns connector description

**Fix Strategy**:
Add these methods to both SourceConnector and DestinationConnector impl blocks:
```rust
fn name(&self) -> &str { "MySQL" }
fn description(&self) -> &str { "Read/Write MySQL databases" }
```

---

#### 4. Method Signature Mismatch: `detect_schema`
**Error Type**: E0050 (mismatched parameter count)  
**Severity**: 🔴 Critical  
**File**: `core/src/connectors/mysql.rs:133`

**Root Cause**:
Trait definition expects: `async fn detect_schema(&self)`  
Implementation has: `async fn detect_schema(&self, table: &str)`

**Current Code**:
```rust
// source.rs (trait)
async fn detect_schema(&self) -> crate::Result<Schema>;

// mysql.rs (impl - WRONG)
async fn detect_schema(&self, table: &str) -> Result<Schema, ConnectorError> {
```

**Fix Strategy**:
Either:
- Remove the `table` parameter and read from config/state
- Or add `table` to trait definition (requires all connectors to implement)

**Recommended**: Remove `table` parameter, store table name in config instead.

---

#### 5. Method Signature Mismatch: `read_batch`
**Error Type**: E0050 (mismatched parameter count)  
**Severity**: 🔴 Critical  
**File**: `core/src/connectors/mysql.rs:181`

**Root Cause**:
Trait definition expects: `async fn read_batch(&self, offset: u64, limit: u64)`  
Implementation has: `async fn read_batch(&self, limit: usize)`

**Current Code**:
```rust
// source.rs (trait)
async fn read_batch(&self, offset: u64, limit: u64) -> crate::Result<Vec<Record>>;

// mysql.rs (impl - WRONG)
async fn read_batch(&self, limit: usize) -> Result<Vec<Record>, ConnectorError> {
```

**Fix Strategy**:
Update MySQL implementation to match trait signature with offset + limit both as u64.

---

#### 6. Missing Trait Methods in DestinationConnector
**Error Type**: E0046 (missing trait items)  
**Severity**: 🔴 Critical  
**File**: `core/src/connectors/mysql.rs:245` (DestinationConnector impl)

**Root Cause**:
MySQLConnector implements DestinationConnector but missing:
- `name()`
- `description()`

**Fix Strategy**:
Same as #3 above.

---

#### 7. Missing Trait Methods in PostgreSQL Connector
**Error Type**: E0046 (missing trait items)  
**Severity**: 🔴 Critical  
**Files**: `core/src/connectors/postgres.rs:28` and `postgres.rs:39`

**Root Cause**:
PostgreSQL connector missing multiple trait methods for both SourceConnector and DestinationConnector:
- SourceConnector missing: `name`, `description`, `detect_schema`, `read_all`, `read_batch`, `read_incremental`
- DestinationConnector missing: `name`, `description`, `write_record`, `write_batch`, `validate_records`

**Fix Strategy**:
Implement all missing methods in both trait impl blocks.

---

#### 8. Missing Method in ConnectorRegistry
**Error Type**: E0599 (no method found)  
**Severity**: 🔴 Critical  
**File**: `core/src/testing/harness.rs:92`

**Root Cause**:
Test harness calls `registry.list_all_connectors()` but this method doesn't exist in ConnectorRegistry struct.

**Current Code**:
```rust
// harness.rs:92 (WRONG)
let connector_ids = self.registry.list_all_connectors();
```

**Fix Strategy**:
Either add `list_all_connectors()` method to ConnectorRegistry or use existing method.

---

### Warning Fixes (Non-Blocking but Important)

#### Unused Imports (15+ warnings)
**Files**: Multiple (mapping.rs, retry_policy.rs, activation_pipeline.rs, etc.)

**Fix Strategy**: Remove all unused imports.

**List**:
- `std::collections::HashMap` in mapping.rs:3
- `std::pin::Pin` in retry_policy.rs:3
- `EventSource`, `EventType` in activation_pipeline.rs:3
- `EventSource` in kafka.rs:1
- `Local` in polling.rs:1
- `std::sync::Arc` in metrics.rs:4 and mod.rs:12
- `Instant` in metrics.rs:5 and traces.rs:4
- `DestinationConnector`, `SourceConnector` in config.rs:3
- `ConnectorConfig` in registry.rs:5
- `std::collections::HashMap` in object_storage.rs:10
- `Value` in mysql.rs:7
- `serde_json::Value` in postgres.rs:6
- `Duration`, `Instant` in connector_test.rs:3

---

#### Deprecated Function Usage
**Error Type**: Deprecated warnings

**Files**:
- `core/src/adapters/http_client.rs:141` - `base64::encode()` deprecated
- `core/src/adapters/webhook.rs:60` - `base64::encode()` deprecated

**Fix**: Update to `Engine::encode()`

---

## Implementation Plan

### Phase 1: Core Fixes (Day 1-7)
1. **Create ConnectorError type** (Day 1)
   - New file: `core/src/connectors/error.rs`
   - Define `ConnectorError` enum with variants
   - Export from connectors/mod.rs

2. **Add parking_lot dependency** (Day 1)
   - Update Cargo.toml
   - Verify no version conflicts

3. **Fix MySQL connector** (Day 2-4)
   - Add `name()` and `description()` to both trait impls
   - Fix `detect_schema()` signature
   - Fix `read_batch()` signature and implementation
   - Update error type from `ConnectorError` to proper type

4. **Fix PostgreSQL connector** (Day 3-5)
   - Implement all missing trait methods
   - Update error handling
   - Match MySQL patterns

5. **Fix ConnectorRegistry** (Day 5-6)
   - Add `list_all_connectors()` method
   - Implement in registry.rs

### Phase 2: Cleanup (Day 7-14)
1. **Remove unused imports** (Day 7-8)
2. **Update deprecated base64 calls** (Day 8)
3. **Run cargo clippy** (Day 8)
4. **Final build verification** (Day 9-14)

---

## Success Criteria

- [ ] `cargo build` succeeds without errors
- [ ] `cargo test` passes all 169 tests
- [ ] `cargo clippy` produces no new warnings
- [ ] No compilation errors in any module
- [ ] All trait implementations complete

---

## Estimated Effort

| Task | Days | Effort |
|------|------|--------|
| ConnectorError type | 1 | Easy |
| parking_lot dependency | 0.5 | Trivial |
| MySQL connector | 3 | Medium |
| PostgreSQL connector | 2 | Medium |
| ConnectorRegistry | 1 | Easy |
| Cleanup | 2 | Trivial |
| Verification | 5 | Medium |
| **Total** | **14.5** | **11-20 day estimate** |

---

## Next Steps

1. ✅ Analyze all errors (COMPLETE)
2. **Create ConnectorError type** (Day 1)
3. **Add parking_lot to Cargo.toml** (Day 1)
4. **Fix MySQL connector** (Days 2-4)
5. **Fix PostgreSQL connector** (Days 3-5)
6. **Fix registry + cleanup** (Days 5-9)
7. **Verify build** (Days 9-14)

**Ready to start Day 1 fixes?**
