# Advanced Connectors: Object Storage, Databases, HDFS

**Enterprise-grade connectors with multi-format support, partitioning, and Hadoop integration.**

---

## Table of Contents

1. [Object Storage (S3, GCS, Azure)](#object-storage)
2. [Advanced Databases & Warehouses](#advanced-databases)
3. [Apache Hadoop HDFS](#apache-hadoop-hdfs)

---

## Object Storage

### Write Multiple File Formats

Write the same data in multiple formats with automatic partitioning:

```yaml
name: multi_format_data_lake
source:
  type: postgres
  query: SELECT * FROM customers

destination:
  type: s3
  bucket: data-lake
  path: customers/
  table_format: parquet
  create_folders: true
```

#### Supported File Formats

| Format | Use Case | Compression | Partitioning |
|--------|----------|-------------|--------------|
| **CSV** | Simple data exchange | gzip, deflate | ✅ |
| **Parquet** | Analytics, efficient | snappy, gzip | ✅ |
| **JSON** | APIs, flexibility | gzip | ✅ |
| **Avro** | Kafka integration | deflate | ✅ |
| **ORC** | Hive warehouses | snappy, zlib | ✅ |
| **Delta** | Transactional tables | snappy | ✅ (built-in) |
| **Iceberg** | Modern data lakes | zstd | ✅ (built-in) |

### Auto-Partitioning

Write data with automatic folder creation:

```yaml
destination:
  type: s3
  bucket: data-lake
  path: analytics/customers/
  table_format:
    format: parquet
    compression: snappy
    partition_columns:
      - date        # Partition by date
      - region      # Then by region
    partition_pattern: "date=YYYY-MM-DD/region=VALUE"
```

**Result:**
```
s3://data-lake/analytics/customers/
├── date=2024-01-15/
│   ├── region=us-east/
│   │   └── part-0001.parquet
│   ├── region=us-west/
│   │   └── part-0002.parquet
│   └── region=eu/
│       └── part-0003.parquet
└── date=2024-01-16/
    ├── region=us-east/
    │   └── part-0001.parquet
    ...
```

### Copy Files Between Storage Buckets

```python
from pyreverseetl import ObjectStorageSource, ObjectStorageDestination

# Read from source bucket
source = ObjectStorageSource(
    ObjectStorageConfig.s3("source-bucket", "data/")
)

# Write to destination bucket
dest = ObjectStorageDestination(
    ObjectStorageConfig.s3("dest-bucket", "archive/")
)

# Copy all files
files = await source.list_files("data/")
for file in files:
    data = await source.read_file(file)
    await dest.write_file(f"archive/{file}", data)
```

### Table Format Examples

#### Delta Lake (ACID Transactions)

```yaml
destination:
  type: s3
  bucket: data-lake
  path: customers/
  table_format: delta
```

**Benefits:**
- ACID transactions
- Data versioning
- Time travel queries
- Schema enforcement

#### Apache Iceberg (Modern Data Lake)

```yaml
destination:
  type: s3
  bucket: data-lake
  path: customers/
  table_format: iceberg
```

**Benefits:**
- Schema evolution
- Hidden partitioning
- Partition pruning
- Snapshots

---

## Advanced Databases

### Create Tables Automatically

Let PyReverseETL create tables with proper schema:

```yaml
destination:
  type: snowflake
  account: xy12345
  warehouse: compute
  database: analytics
  schema: create_if_not_exists
  columns:
    - name: customer_id
      type: INTEGER
      primary_key: true
    - name: email
      type: VARCHAR(255)
    - name: lifetime_value
      type: DECIMAL(10,2)
```

### Write Strategies

Choose the right write strategy for your use case:

#### 1. Direct Insert

```yaml
destination:
  write_strategy: insert
```

**Use when:**
- Appending new records
- No duplicates expected
- Fastest option

#### 2. Upsert (Insert or Update)

```yaml
destination:
  write_strategy: upsert
  key_column: customer_id
```

**Use when:**
- Updating existing records
- Handle duplicates
- Most common approach

#### 3. Merge (Advanced Upsert)

```yaml
destination:
  write_strategy: merge
  key_column: customer_id
  conditions:
    - when_matched: update
      then: SET name = VALUES(name), updated_at = NOW()
    - when_not_matched: insert
```

#### 4. Bulk Load

```yaml
destination:
  type: snowflake
  write_strategy: bulk_load
  bulk_load:
    data_source: s3://staging/data.parquet
    file_format: parquet
    compression: snappy
```

**Use when:**
- Very large datasets (100GB+)
- Fastest load performance
- Staging storage available

#### 5. Replace (Full Refresh)

```yaml
destination:
  write_strategy: replace
```

**Use when:**
- Refreshing entire table
- Replacing historical data
- Slow but simple

### Incremental Loads

Load only changed records since last sync:

```yaml
destination:
  type: postgres
  incremental_column: updated_at
```

Query generated:
```sql
SELECT * FROM customers 
WHERE updated_at > '2024-01-15 10:30:00'
ORDER BY updated_at
```

### Partitioned Warehouse Tables

```yaml
destination:
  type: snowflake
  schema:
    table_name: customers
    partition_columns:
      - date
      - region
    cluster_columns:
      - customer_id
      - email
```

**BigQuery:**
```yaml
destination:
  type: bigquery
  schema:
    cluster_columns:
      - country
      - segment
      - signup_date
```

---

## Apache Hadoop HDFS

### Simple (No Security)

```yaml
source:
  type: hdfs
  namenode: namenode.local
  port: 8020
  path: /data/customers
  file_format: parquet
```

### Kerberos Security

```yaml
source:
  type: hdfs
  namenode: secure-namenode.local
  port: 8020
  path: /secure/data/
  auth: kerberos
  kerberos_principal: hdfs@REALM.COM
  kerberos_keytab: /etc/security/keytabs/hdfs.keytab
```

### WebHDFS (HTTP)

```yaml
destination:
  type: hdfs
  namenode: namenode.local
  port: 50070
  path: /output/customers
  use_webhdfs: true
  file_format: orc
  replication_factor: 3
  block_size: 268435456  # 256MB blocks
```

### File Operations

Copy files within HDFS:

```python
from pyreverseetl import HdfsSource, HdfsConfig

source = HdfsSource(HdfsConfig.simple("namenode", "/input/"))

# List files
files = await source.list_files("/input/")

# Get file status
status = await source.get_file_status("/input/data.parquet")
print(f"Size: {status.size}, Replication: {status.replication}")

# Read file
data = await source.read_file("/input/data.csv")
```

---

## Real-World Examples

### Example 1: Multi-Destination Data Lake

Sync to multiple storage systems with different formats:

```yaml
name: unified_data_lake
source:
  type: postgres
  query: SELECT * FROM orders

destinations:
  # S3 for analytics (Parquet)
  - name: s3_analytics
    type: s3
    bucket: analytics-lake
    table_format: parquet
    partition_columns: [date]
    
  # Snowflake for reporting
  - name: snowflake_reporting
    type: snowflake
    account: xy12345
    warehouse: compute
    table: orders
    
  # HDFS for Hive/Spark jobs
  - name: hdfs_warehouse
    type: hdfs
    namenode: hadoop-cluster
    path: /warehouse/orders/
    file_format: orc
```

### Example 2: Incremental Warehouse Load

Load changes efficiently into BigQuery:

```yaml
name: incremental_bigquery_sync
source:
  type: postgres
  query: SELECT * FROM customers WHERE updated_at > :last_sync
  incremental_column: updated_at

destination:
  type: bigquery
  project: my-project
  dataset: analytics
  table: customers
  write_strategy: upsert
  key_column: customer_id
  schema:
    cluster_columns:
      - segment
      - signup_date
```

### Example 3: Data Lake Archival

Archive old data to cold storage:

```yaml
name: archive_to_s3_glacier
source:
  type: snowflake
  query: SELECT * FROM events WHERE event_date < DATEADD(MONTH, -6, CURRENT_DATE)

destination:
  type: s3
  bucket: data-archive
  path: events/archive/
  table_format:
    format: parquet
    compression: zstd
    partition_columns: [year, month]
  storage_class: GLACIER  # Move to Glacier after 30 days
```

### Example 4: Hadoop to Data Warehouse

Move data from Hadoop cluster to modern warehouse:

```yaml
name: hadoop_to_snowflake_migration
source:
  type: hdfs
  namenode: hadoop-prod.local
  path: /warehouse/customers/
  auth: kerberos
  kerberos_principal: hdfs@CORP.COM
  kerberos_keytab: /etc/hdfs.keytab

destination:
  type: snowflake
  account: xy12345
  warehouse: migrate
  database: legacy_data
  write_strategy: bulk_load
  bulk_load:
    data_source: s3://staging/hdfs-extract/
```

---

## Performance Comparison

| Operation | Parquet | Iceberg | Delta | ORC |
|-----------|---------|---------|-------|-----|
| Write speed | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐ |
| Read speed | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐ |
| Compression | snappy | zstd | snappy | zstd |
| Schema evolution | No | ✅ | ✅ | No |
| ACID transactions | No | ✅ | ✅ | No |
| Time travel | No | ✅ | ✅ | No |

---


## Best Practices

### 1. Choose Right Format

- **Parquet** — Fastest, best compression (analytics default)
- **CSV** — Simplest, widely supported (data exchange)
- **Iceberg** — Schema evolution needed (modern data lakes)
- **Delta** — ACID transactions needed (point updates)
- **ORC** — Hive integration (Hadoop ecosystems)

### 2. Optimize Partitioning

```yaml
# Good: balanced partitions
partition_columns: [date, region]  # Creates date/region=VALUE folders

# Avoid: too many partitions
partition_columns: [year, month, day, hour, customer_id]  # Too granular
```

### 3. Use Bulk Load for Large Syncs

```yaml
# For 100GB+ loads
write_strategy: bulk_load
# Faster than row-by-row inserts
```

### 4. Monitor Rate Limits

Even with advanced connectors, respect API limits:

```yaml
destination:
  rate_limit:
    strategy: quota
    requests_per_hour: 100  # Snowflake query quota
```

---

## Summary

PyReverseETL's advanced connectors provide:

✅ **Multi-format support** — CSV, Parquet, JSON, ORC, Iceberg, Delta  
✅ **Auto-partitioning** — Date, hour, region-based  
✅ **Advanced databases** — Snowflake, BigQuery, Redshift with transactional support  
✅ **Hadoop integration** — HDFS with Kerberos security  
✅ **Modern data lakes** — Iceberg and Delta Lake  
✅ **Efficient operations** — Bulk load, incremental sync, merge  

Built for modern data operations.

---

**Next:** [Orchestration & API](ORCHESTRATION_AND_API.md) | [Rate Limiting](RATE_LIMITING.md)
