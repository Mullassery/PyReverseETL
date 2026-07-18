# PostgreSQL Connector

**Type:** Source & Destination  
**Category:** Database  
**Rate Limit:** Configurable (default: unlimited)  
**Auth Method:** Username/Password, SSL/TLS  
**Supported Versions:** PostgreSQL 10.0+  

---

## Quick Start

### Installation
```bash
pip install pyreverseetl
```

### Basic Configuration

#### As Source (Read)
```yaml
source:
  type: postgres
  host: db.example.com
  port: 5432
  database: analytics
  username: ${DB_USER}
  password: ${DB_PASSWORD}
  query: SELECT * FROM customers WHERE updated_at > :last_sync
  incremental_column: updated_at
```

#### As Destination (Write)
```yaml
destination:
  type: postgres
  host: db.example.com
  port: 5432
  database: analytics
  username: ${DB_USER}
  password: ${DB_PASSWORD}
  table: customers
  write_strategy: upsert
  key_column: customer_id
  batch_size: 1000
```

### Python Example
```python
from pyreverseetl import PostgreSQLSource, PostgreSQLDestination

# Source
source = PostgreSQLSource(
    host="localhost",
    database="mydb",
    username="user",
    password="pass"
)
records = source.read_all()

# Destination
dest = PostgreSQLDestination(
    host="localhost",
    database="mydb",
    table="sync_customers"
)
dest.write_batch(records)
```

---

## Capabilities

✅ **Read Data** — SELECT queries with pagination  
✅ **Write Data** — INSERT, UPDATE, UPSERT modes  
✅ **Schema Detection** — Auto-detect table schema  
✅ **Incremental Sync** — Track changes via timestamp/sequence  
✅ **Change Data Capture (CDC)** — Logical replication mode  
✅ **Batch Operations** — Parallel inserts  
✅ **Connection Pooling** — Reuse connections efficiently  
✅ **SSL/TLS** — Encrypted connections  

---

## Connection Options

| Option | Required | Default | Description |
|--------|----------|---------|-------------|
| `host` | Yes | - | PostgreSQL hostname or IP |
| `port` | No | 5432 | PostgreSQL port |
| `database` | Yes | - | Database name |
| `username` | Yes | - | Database user |
| `password` | Yes | - | Database password |
| `sslmode` | No | prefer | SSL mode: disable, allow, prefer, require |
| `connect_timeout` | No | 10 | Connection timeout (seconds) |
| `pool_min` | No | 2 | Minimum connections in pool |
| `pool_max` | No | 20 | Maximum connections in pool |
| `idle_timeout` | No | 300 | Idle connection timeout (seconds) |

---

## Write Strategies

### INSERT
```yaml
destination:
  write_strategy: insert
  # Fails if row exists (use for new data only)
```

### UPSERT (Recommended)
```yaml
destination:
  write_strategy: upsert
  key_column: customer_id
  # Updates existing, inserts new
```

### UPDATE
```yaml
destination:
  write_strategy: update
  key_column: customer_id
  # Only updates existing rows
```

### REPLACE
```yaml
destination:
  write_strategy: replace
  key_column: customer_id
  # Delete and re-insert (atomic)
```

---

## Incremental Sync

### Timestamp-Based
```yaml
source:
  query: SELECT * FROM customers WHERE updated_at > :last_sync
  incremental_column: updated_at
  # Tracks last_sync timestamp and resumes from there
```

### Sequence-Based
```yaml
source:
  query: SELECT * FROM customers WHERE id > :last_id
  incremental_column: id
  # Tracks last ID and queries for new records
```

### Change Data Capture (CDC)
```yaml
source:
  type: postgres
  cdc_enabled: true
  cdc_mode: logical_replication
  # Requires: wal_level = logical
```

---

## Rate Limiting

PostgreSQL doesn't have API rate limits, but you can control throughput:

```yaml
destination:
  batch_size: 1000          # Rows per batch
  max_connections: 5        # Parallel writers
  rate_limit: null          # No limit (uses batch size)
```

**Performance Tuning:**
- Increase `batch_size` for throughput (default: 1000)
- Increase `max_connections` for parallelism (default: 5)
- Use `COPY` mode for bulk loads (100x faster)

---

## Authentication

### Basic (Username/Password)
```yaml
source:
  host: db.example.com
  username: user
  password: ${DB_PASSWORD}  # Use environment variable
  sslmode: require
```

### Environment Variables
```bash
POSTGRES_HOST=db.example.com
POSTGRES_DB=mydb
POSTGRES_USER=user
POSTGRES_PASSWORD=secret
```

### SSL/TLS Certificates
```yaml
source:
  host: db.example.com
  sslmode: require
  sslcert: /path/to/client-cert.pem
  sslkey: /path/to/client-key.pem
  sslrootcert: /path/to/ca-cert.pem
```

---

## Performance

| Operation | Throughput | Latency | Notes |
|-----------|-----------|---------|-------|
| Read (sequential) | 50 MB/s | 10-20 ms | Depends on network |
| Read (parallel) | 200 MB/s | 20-50 ms | With 10 parallel connections |
| Write (batch) | 100 MB/s | 5-10 ms | 1000-row batches |
| Write (parallel) | 500 MB/s | 10-20 ms | 5 parallel writers |
| Schema detection | - | 100-200 ms | Single query |

**Optimization Tips:**
- Use `UNLOGGED` tables for temporary data (10x faster)
- Enable `synchronous_commit = off` for staging (risky!)
- Use connection pooling (5-10 connections typical)
- Batch writes in 1000-row chunks
- Create indexes on join/filter columns

---

## Troubleshooting

### Connection Refused
```
Error: connection refused on 127.0.0.1:5432
```
**Causes:**
- PostgreSQL not running
- Wrong host/port
- Firewall blocking connection

**Solution:**
```bash
psql -h db.example.com -U user -d mydb
# Test if you can connect
```

### Authentication Failed
```
Error: password authentication failed for user "user"
```
**Causes:**
- Wrong username/password
- User doesn't have login privilege
- pg_hba.conf restricting access

**Solution:**
```bash
# Check user permissions
psql -h db.example.com -U postgres -c "ALTER USER user WITH ENCRYPTED PASSWORD 'newpass';"
```

### Incremental Sync Not Working
```
Error: column "updated_at" does not exist
```
**Causes:**
- Column doesn't exist
- Column doesn't track updates
- NULL values in timestamp

**Solution:**
- Verify column exists: `\d customers` in psql
- Add timestamp column: `ALTER TABLE customers ADD COLUMN updated_at TIMESTAMP DEFAULT NOW();`
- Create update trigger if needed

### Slow Queries
```
Sync taking >5 minutes for 1M rows
```
**Causes:**
- No index on incremental column
- Full table scan
- Network latency

**Solution:**
```sql
-- Create index on incremental column
CREATE INDEX idx_customers_updated_at ON customers(updated_at);

-- Or for CDC, enable logical replication
ALTER SYSTEM SET wal_level = logical;
```

### Connection Pool Exhausted
```
Error: all connections in pool are in use
```
**Causes:**
- Too many parallel syncs
- Long-running queries
- Connection leaks

**Solution:**
```yaml
destination:
  max_connections: 10      # Increase pool size
  idle_timeout: 300        # Drop idle connections
  statement_timeout: 60000 # Cancel long queries (ms)
```

---

## Security Best Practices

### Credentials
✅ Use environment variables for passwords
✅ Rotate credentials regularly
✅ Use least-privilege user account
❌ Don't hardcode passwords
❌ Don't commit credentials to version control

### Network
✅ Use SSL/TLS for remote connections
✅ Use VPN/private networks
✅ Restrict by IP in pg_hba.conf
❌ Don't use public IP for production database

### Data
✅ Encrypt sensitive columns at-rest
✅ Use row-level security for compliance
✅ Audit all data access
✅ Regular backups

---

## Examples

### Example 1: Sync Customers to Warehouse

```yaml
name: customers_to_warehouse
owner: data_team

source:
  type: postgres
  host: prod.db.com
  database: operational
  query: SELECT * FROM customers WHERE updated_at > :last_sync
  incremental_column: updated_at

transformation:
  - step: cleanse
    script: |
      record['email'] = record['email'].lower()
      record['phone'] = record['phone'].replace('-', '')
      return record

destination:
  type: postgres
  host: warehouse.db.com
  database: analytics
  table: customers_sync
  write_strategy: upsert
  key_column: customer_id
  batch_size: 5000

schedule:
  frequency: hourly
  retry_on_failure: true
```

### Example 2: Real-Time CDC Replication

```python
from pyreverseetl import PostgreSQLSource

source = PostgreSQLSource(
    host="prod.db.com",
    database="mydb",
    cdc_enabled=True,
    cdc_mode="logical_replication"
)

# Replicate all changes in real-time
for change in source.stream_changes():
    print(f"Change: {change.operation} on {change.table}")
    # Replicate to destination...
```

### Example 3: Parallel Bulk Load

```python
from pyreverseetl import PostgreSQLDestination

dest = PostgreSQLDestination(
    host="warehouse.db.com",
    database="analytics",
    table="large_table",
    write_strategy="bulk_load",
    max_connections=10,  # 10 parallel writers
    batch_size=10000     # 10k rows per batch
)

# Load 1M rows in parallel
dest.write_batch(records, use_copy=True)  # 100x faster with COPY
```

---

## Limits & Quotas

- **Max connection pool:** 100 (system dependent)
- **Max batch size:** 100,000 rows (memory dependent)
- **Max query timeout:** Configurable (default: 5 minutes)
- **Max identifier length:** 63 characters
- **Max transaction size:** System RAM dependent

---

## Related Connectors

- **MySQL** — Similar SQL database connector
- **MongoDB** — Document-based alternative
- **Snowflake** — Cloud warehouse option
- **S3** — Data lake export option

---

**Status:** ✅ Production Ready  
**Last Updated:** 2026-07-18  
**Support:** Community & Commercial
