# Top 10 Connectors: Implementation Sprint

**Focus: PostgreSQL, MySQL, Snowflake, Salesforce, BigQuery, S3, Kafka, Redshift, HubSpot, Braze**

**Duration**: 1-2 weeks | **LOC Target**: 2000+ (impl) + 1000+ (tests)

---

## Priority Order & Implementation Strategy

### Phase 1: Databases (Days 1-2) — 600 LOC
Implement database connectors with connection pooling & incremental reads

```
1. MySQL ✅ Start
   └─ Similar to PostgreSQL
   ├─ Connection pooling
   ├─ Incremental reads (auto_increment)
   ├─ Schema detection
   ├─ Batch writes
   └─ 20 test cases

2. (PostgreSQL already documented)
```

### Phase 2: Data Warehouses (Days 2-3) — 600 LOC
Cloud-native warehouses with bulk loading

```
3. Snowflake ✅ Start after MySQL
   ├─ OAuth token handling
   ├─ Snowflake stage (temp files)
   ├─ COPY INTO command
   ├─ Snowflake-specific write strategies
   ├─ Time travel support
   └─ 20 test cases

4. Google BigQuery
   ├─ Service account auth
   ├─ Streaming inserts
   ├─ Batch loads
   ├─ Auto-partitioning
   └─ 20 test cases

5. Amazon Redshift
   ├─ IAM role auth
   ├─ S3 staging
   ├─ COPY command
   ├─ Spectrum support
   └─ 20 test cases
```

### Phase 3: Cloud Storage (Days 3-4) — 300 LOC
Object storage with multi-format support

```
6. Amazon S3
   ├─ Multi-part uploads
   ├─ Parquet/Avro formats
   ├─ Partitioning (date, customer)
   ├─ Auto-retry on transient errors
   └─ 15 test cases
```

### Phase 4: Streaming (Days 4-5) — 400 LOC
Message queue integration

```
7. Kafka
   ├─ Consumer groups
   ├─ Offset tracking
   ├─ Auto-scaling by lag
   ├─ Error topic routing
   ├─ Batch aggregation
   └─ 20 test cases
```

### Phase 5: SaaS Platforms (Days 5-7) — 400 LOC
API-based connectors with rate limiting

```
8. Salesforce
   ├─ OAuth2 token refresh
   ├─ Bulk API 2.0
   ├─ Rate limiting (25 req/sec)
   ├─ Composite requests
   ├─ Upsert by external ID
   └─ 20 test cases

9. HubSpot
   ├─ API key auth
   ├─ Rate limiting (10 req/sec)
   ├─ Pagination (after token)
   ├─ Custom properties
   └─ 15 test cases

10. Braze
    ├─ API key auth
    ├─ Batch endpoint (500 users max)
    ├─ Rate limiting (100 req/sec)
    ├─ Audience export
    └─ 15 test cases
```

---

## Day-by-Day Breakdown

### Day 1: MySQL Implementation
**Goal**: Working MySQL connector with tests

```bash
# 1. Create MySQL connector file
# core/src/connectors/mysql.rs (200 LOC)

# 2. Implement:
#    - Connection pooling
#    - Connection string parsing
#    - Schema detection
#    - Read (with incremental support)
#    - Write (INSERT, UPSERT modes)
#    - Batch operations

# 3. Add tests
# core/src/connectors/tests/mysql.rs (150 LOC)

# 4. Document
# docs/connectors/MYSQL.md (350 lines)

# 5. Verify
cargo test mysql --all
```

### Day 2: Snowflake Implementation
**Goal**: Production Snowflake connector

```bash
# 1. Create Snowflake connector
# core/src/connectors/snowflake.rs (250 LOC)

# 2. Implement:
#    - OAuth token management
#    - SQL Warehouse execution
#    - Stage operations (file uploads)
#    - COPY INTO command
#    - Schema detection
#    - Time travel support

# 3. Add tests
# core/src/connectors/tests/snowflake.rs (150 LOC)

# 4. Document
# docs/connectors/SNOWFLAKE.md (350 lines)

# 5. Verify
cargo test snowflake --all
```

### Day 3: BigQuery + Redshift
**Goal**: Both cloud warehouses

**BigQuery** (200 LOC)
- Service account auth
- Streaming inserts
- Batch loads via GCS

**Redshift** (200 LOC)
- IAM role auth
- S3 staging
- COPY command

### Day 4: S3 + Kafka
**Goal**: Cloud storage and streaming

**S3** (200 LOC)
- Multi-part uploads
- Format support (Parquet, Avro)
- Partitioning logic

**Kafka** (250 LOC)
- Consumer groups
- Offset tracking
- Auto-scaling by lag

### Days 5-7: SaaS (Salesforce, HubSpot, Braze)
**Goal**: Three fully functional SaaS connectors

Each 100-150 LOC + 100-150 LOC tests

---

## Implementation Template

### Connector Structure

```rust
// core/src/connectors/[NAME].rs

use async_trait::async_trait;
use crate::connectors::{SourceConnector, DestinationConnector, Record, Capability};

pub struct [NameConnector] {
    config: [NameConfig],
    connection_pool: ConnectionPool,
    metrics: MetricsCollector,
}

impl [NameConnector] {
    pub fn new(config: [NameConfig]) -> Self {
        Self {
            config,
            connection_pool: ConnectionPool::new(config.pool_size),
            metrics: MetricsCollector::new("[name]"),
        }
    }
}

#[async_trait]
impl SourceConnector for [NameConnector] {
    async fn test_connection(&self) -> Result<(), ConnectorError> {
        // Test connection logic
    }
    
    async fn detect_schema(&self, table: &str) -> Result<Schema, ConnectorError> {
        // Schema detection logic
    }
    
    async fn read_all(&self) -> Result<Vec<Record>, ConnectorError> {
        // Full read logic
    }
    
    async fn read_batch(&self, limit: usize) -> Result<Vec<Record>, ConnectorError> {
        // Batch read logic
    }
    
    async fn read_incremental(&self, since: Checkpoint) -> Result<Vec<Record>, ConnectorError> {
        // Incremental read logic
    }
    
    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::Read,
            Capability::SchemaDetection,
            Capability::IncrementalRead,
            Capability::Batch,
        ]
    }
}

#[async_trait]
impl DestinationConnector for [NameConnector] {
    async fn test_connection(&self) -> Result<(), ConnectorError> {
        // Test connection logic
    }
    
    async fn write_record(&self, record: &Record) -> Result<(), ConnectorError> {
        // Single record write
    }
    
    async fn write_batch(&self, records: &[Record]) -> Result<usize, ConnectorError> {
        // Batch write logic
        Ok(records.len())
    }
    
    async fn validate_records(&self, records: &[Record]) -> Result<Vec<bool>, ConnectorError> {
        // Validation logic
    }
    
    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::Write,
            Capability::Batch,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_connection() {
        let config = [NameConfig]::test();
        let connector = [NameConnector]::new(config);
        assert!(connector.test_connection().await.is_ok());
    }
    
    // ... more tests
}
```

---

## Testing Strategy

### For Each Connector

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    // 1. Connection tests
    #[tokio::test]
    async fn test_connection() { }
    
    #[tokio::test]
    async fn test_connection_pool_reuse() { }
    
    #[tokio::test]
    async fn test_connection_timeout() { }
    
    // 2. Schema detection
    #[tokio::test]
    async fn test_schema_detection() { }
    
    // 3. Read operations
    #[tokio::test]
    async fn test_read_all() { }
    
    #[tokio::test]
    async fn test_read_batch() { }
    
    #[tokio::test]
    async fn test_read_incremental() { }
    
    // 4. Write operations
    #[tokio::test]
    async fn test_write_single_record() { }
    
    #[tokio::test]
    async fn test_write_batch() { }
    
    #[tokio::test]
    async fn test_write_upsert() { }
    
    // 5. Rate limiting
    #[tokio::test]
    async fn test_rate_limiting() { }
    
    // 6. Error recovery
    #[tokio::test]
    async fn test_retry_on_transient_error() { }
    
    // 7. Integration
    #[tokio::test]
    async fn test_end_to_end_sync() { }
}
```

**Target**: 20 tests per connector

---

## Documentation Template

Each connector gets `docs/connectors/[NAME].md` with:

1. **Quick Start** (YAML + Python)
2. **Capabilities** (read, write, incremental, etc.)
3. **Connection Options** (table with all params)
4. **Authentication** (setup instructions)
5. **Rate Limiting** (defaults + recommendations)
6. **Performance** (benchmarks)
7. **Troubleshooting** (common issues)
8. **Examples** (3-5 real use cases)
9. **Related Connectors**

---

## Progress Tracking

### Daily Checklist

```
Day 1 - MySQL
[ ] Connector implementation (200 LOC)
[ ] Test cases (150 LOC)
[ ] Documentation (350 lines)
[ ] Verify: cargo test mysql --all passes
[ ] Verify: docs/connectors/MYSQL.md complete

Day 2 - Snowflake
[ ] Connector implementation (250 LOC)
[ ] Test cases (150 LOC)
[ ] Documentation (350 lines)
[ ] Verify: cargo test snowflake --all passes
[ ] Verify: docs/connectors/SNOWFLAKE.md complete

... (repeat for each)

Week End - All 10 Done
[ ] 2000+ LOC of implementations
[ ] 1000+ LOC of tests
[ ] 3500+ lines of documentation
[ ] 200 test cases total
[ ] <1% failure rate
[ ] All 10 merged to main
[ ] Ready for v2.1-beta release
```

---

## Integration Points

### Update These Files as You Go

1. **core/src/connectors/mod.rs**
   - Add `pub mod mysql;`, `pub mod snowflake;`, etc.
   - Export in `pub use` statements

2. **core/src/connectors/registry.rs**
   - Add each connector to `BuiltInConnectors`
   - Add capability detection
   - Add to connector list

3. **core/src/connectors/connectors_db.rs**
   - Add connector metadata
   - Pre-configured rate limits
   - Connection examples

4. **docs/CONNECTOR_ECOSYSTEM.md**
   - Update capability matrix
   - Add to relevant category

---

## Success Metrics

### Per Connector
- ✅ Connection test passes
- ✅ Schema detection works
- ✅ Read operations functional
- ✅ Write operations functional
- ✅ Rate limiting respected
- ✅ Error recovery working
- ✅ All 20 tests passing
- ✅ Documentation complete

### Overall (Top 10)
- ✅ 2000+ implementation LOC
- ✅ 1000+ test LOC
- ✅ 200+ passing tests
- ✅ <1% error rate
- ✅ 10,000+ documentation lines
- ✅ All CI/CD checks passing
- ✅ Ready for production

---

## Git Workflow

### Per Connector Implementation

```bash
# Create feature branch
git checkout -b feat/connector-mysql

# Implement
# - core/src/connectors/mysql.rs
# - core/src/connectors/tests/mysql.rs
# - docs/connectors/MYSQL.md
# - Update core/src/connectors/mod.rs
# - Update registry

# Test locally
cargo test mysql --all --release
cargo clippy --all
cargo fmt --all

# Commit
git add -A
git commit -m "feat: Add MySQL connector with 20 tests"

# Push feature branch
git push origin feat/connector-mysql

# Create PR for review
gh pr create --title "MySQL Connector" --body "Implements MySQL connector with full test coverage"

# After review, merge to main
gh pr merge --squash
```

---

## Quick Reference: Connector Patterns

### Connection Pooling
```rust
let pool = ConnectionPool::new(config.pool_size);
let conn = pool.get("mysql").await;
// Use connection
pool.release("mysql", conn).await;
```

### Incremental Reads
```rust
// Track last sync checkpoint
let checkpoint = Checkpoint {
    last_id: 1000,
    last_sync: "2024-07-18T10:00:00Z",
};

// Query incrementally
let records = connector.read_incremental(checkpoint).await?;
```

### Batch Writes
```rust
// Write in chunks
const BATCH_SIZE: usize = 10000;
for chunk in records.chunks(BATCH_SIZE) {
    connector.write_batch(chunk).await?;
}
```

### Rate Limiting
```rust
let rate_limiter = RateLimiter::new(
    RateLimitStrategy::TokenBucket {
        tokens_per_second: 25,
        max_burst: 100,
    }
);

// Apply before each API call
rate_limiter.acquire(1).await?;
```

---

## Next: Start with MySQL

Ready to begin? Let's implement MySQL connector now!

**Estimated Time**: 2-3 hours (including tests + docs)

---

**Status**: 🚀 Ready to start Top 10  
**Target Completion**: 2026-07-25 (1 week)  
**Branch**: main (after each PR merge)
