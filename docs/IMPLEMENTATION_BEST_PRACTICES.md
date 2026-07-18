# PyReverseETL: Implementation Best Practices

**Production patterns for reliable, performant, and secure data pipelines.**

---

## DATA PIPELINE DESIGN

### 1. Idempotent Transformations

✅ **Design for Fault Tolerance**

Ensure transformations can be safely re-run without duplicating or corrupting data:

```python
# ✅ GOOD: Idempotent transformation
def upsert_customer(record):
    # Uses customer_id as key - safe to retry
    customer = db.upsert(
        table='customers',
        key_column='customer_id',
        record=record
    )
    return customer

# ❌ BAD: Non-idempotent transformation
def add_customer(record):
    # INSERT without checking if exists - fails on retry
    db.insert('customers', record)
    return record
```

✅ **Idempotent Keys for Kafka**

```yaml
source:
  type: kafka
  topic: customer_events

transformation:
  idempotency:
    key_column: event_id  # Unique ID per event
    deduplication: enabled
    
destination:
  write_strategy: upsert
  key_column: event_id  # Same key prevents duplicates on retry
```

✅ **Checkpointing for Streaming**

```python
# Commit offsets only after successful processing
for message in kafka_consumer:
    try:
        result = transform(message)
        destination.write(result)
        kafka_consumer.commit()  # Only if successful
    except Exception as e:
        logger.error(f"Failed: {message}")
        # Consumer can retry from last committed offset
```

### 2. Circuit Breakers for Third-Party APIs

✅ **Prevent Cascading Failures**

```yaml
destination:
  type: salesforce
  circuit_breaker:
    enabled: true
    failure_threshold: 5  # Open after 5 failures
    timeout: 60s
    half_open_max_calls: 2
    recovery_timeout: 300s

    on_open:
      action: send_to_dlq  # Route to dead-letter queue
      notify: on_call_team
      metric: circuit_breaker_open
```

✅ **Implementation Pattern**

```python
from pyreverseetl import CircuitBreaker

circuit_breaker = CircuitBreaker(
    failure_threshold=5,
    recovery_timeout=300,
    half_open_max_calls=2
)

def send_to_api(record):
    @circuit_breaker
    def _send():
        return api.post('/records', record)
    
    try:
        return _send()
    except CircuitBreakerOpen:
        # Send to DLQ for later retry
        dlq.send(record)
```

### 3. Data Partitioning Strategies

✅ **Time-Based Partitioning (Most Common)**

```yaml
destination:
  type: s3
  bucket: data-lake
  table_format: delta
  
  partitioning:
    strategy: time_based
    column: created_at
    granularity: daily  # date=2024-07-18/
    ttl: 2555  # Keep 7 years of data
```

**Benefits:**
- Automatic purging of old data
- Parallel pruning in queries
- Efficient incremental syncs
- Fast historical queries

✅ **Key-Based Partitioning (For Workload Distribution)**

```yaml
destination:
  type: snowflake
  partitioning:
    strategy: key_based
    column: customer_id
    num_partitions: 256  # Distribute across 256 buckets
    
    # Prevents hotspots
    hash_function: md5
```

**Benefits:**
- Distributes load evenly
- Prevents single-partition hotspots
- Enables parallel writes

✅ **Hybrid Partitioning (Recommended for Large Datasets)**

```yaml
destination:
  partitioning:
    strategy: hybrid
    primary: date  # date=2024-07-18/
    secondary: customer_id  # customer_id=1234/
    
    # Enables:
    # s3://data-lake/events/date=2024-07-18/customer_id=1234/part-*.parquet
```

---

## PERFORMANCE OPTIMIZATION

### 1. Columnar Storage

✅ **Use Columnar Formats for Analytics**

```yaml
destination:
  type: s3
  table_format: parquet  # Columnar, compressed

  # Configuration
  compression: snappy   # Fast compression
  codec: snappy
  row_group_size: 128MB  # Optimal for queries
  
  # Result: 10x smaller than CSV, 100x faster queries
```

**Format Comparison:**
| Format | Compression | Query Speed | Size |
|--------|-------------|-------------|------|
| CSV | gzip | 1x | 1x |
| JSON | snappy | 5x | 0.8x |
| Parquet | snappy | 100x | 0.1x |
| ORC | zstd | 80x | 0.08x |

### 2. Smart Caching Mechanisms

✅ **Query Result Caching (Redis)**

```python
from pyreverseetl import CacheLayer
import redis

cache = CacheLayer(
    backend='redis',
    ttl=3600,  # 1 hour
    pattern='[job_id]:[query_hash]'
)

@cache
def get_customer_segment(customer_id):
    # First call: executes query
    # Second call (within 1h): returns cached result
    return db.query(f"SELECT segment FROM customers WHERE id = {customer_id}")
```

✅ **Transform Result Caching**

```yaml
transformation:
  cache:
    enabled: true
    backend: redis
    ttl: 7200  # 2 hours
    invalidate_on:
      - source_updated  # Cache expires if source changes
      - schedule: daily  # Daily invalidation
```

**Use Cases:**
- Repeated dimension lookups
- Expensive ML model scoring
- Cross-sync reference data

### 3. Query Plan Analysis

✅ **Analyze and Optimize Transformations**

```python
from pyreverseetl import QueryAnalyzer

analyzer = QueryAnalyzer()

# Expensive transformation
query = """
SELECT c.id, c.name, SUM(o.amount) as total
FROM customers c
LEFT JOIN orders o ON c.id = o.customer_id  -- SLOW
GROUP BY c.id, c.name
"""

plan = analyzer.analyze(query)
print(plan.estimated_cost)  # 50,000 cost units
print(plan.recommendations)  # [Suggestion: add index on orders(customer_id)]
```

✅ **Optimization Patterns**

```python
# ❌ SLOW: Full join
SELECT c.*, o.* FROM customers c LEFT JOIN orders o ON c.id = o.customer_id

# ✅ FAST: Separate aggregations
SELECT c.*, o.total_amount
FROM customers c
LEFT JOIN (
    SELECT customer_id, SUM(amount) as total_amount
    FROM orders
    GROUP BY customer_id
) o ON c.id = o.customer_id
```

---

## SECURITY FRAMEWORK

### 1. End-to-End Encryption

✅ **AES-256-GCM for Data at Rest**

```yaml
security:
  encryption:
    at_rest:
      enabled: true
      algorithm: AES-256-GCM
      key_management: aws_kms  # or azure_keyvault, vault
      
      # Auto-encryption for sensitive columns
      column_encryption:
        - column: ssn
          key_id: customer_ssn_key
        - column: credit_card
          key_id: payment_card_key
```

✅ **TLS 1.3 for Data in Transit**

```yaml
connectors:
  postgres_prod:
    host: db.example.com
    port: 5432
    ssl_mode: require
    ssl_version: tls_1_3
    certificate_validation: strict
```

✅ **Field-Level Encryption in Code**

```python
from pyreverseetl import FieldEncryption

encryptor = FieldEncryption(algorithm='AES-256-GCM', key_id='prod_key')

# Encrypt sensitive fields during transformation
def transform_customer(record):
    if 'ssn' in record:
        record['ssn'] = encryptor.encrypt(record['ssn'])
    if 'credit_card' in record:
        record['credit_card'] = encryptor.encrypt(record['credit_card'])
    return record
```

### 2. Tokenization for Sensitive Fields

✅ **Replace PII with Tokens**

```yaml
transformation:
  tokenization:
    enabled: true
    
    rules:
      - field: email
        method: hash_token  # Hash-based, non-reversible
        algorithm: sha256
        
      - field: phone
        method: format_token  # Keep format, mask value
        format: "(XXX) XXX-XXXX"
        
      - field: ssn
        method: deterministic_token  # Same input = same token
        key: ssn_tokenization_key
```

**Example:**
```
Original: 123-45-6789
Tokenized: tok_a7f9e2c5d1b8
Stored: Secure vault, not in warehouse
```

### 3. Audit Trails with Immutable Logging

✅ **Immutable Audit Log**

```yaml
security:
  audit:
    enabled: true
    backend: s3  # Immutable storage
    
    events:
      - type: data_access
        fields: [user_id, connector, timestamp, records_accessed]
      
      - type: configuration_change
        fields: [user_id, change_type, before, after, timestamp]
      
      - type: auth_event
        fields: [user_id, action, source_ip, success, timestamp]
    
    retention: unlimited  # Keep forever
    write_once: true  # Cannot be deleted
```

✅ **Immutable Logging Implementation**

```python
from pyreverseetl import AuditLog

audit = AuditLog(backend='s3_write_once')

# Log all data access
audit.log_access(
    user_id='user_123',
    connector='salesforce',
    operation='read',
    record_count=5000,
    timestamp=datetime.utcnow(),
    source_ip='192.168.1.1'
)

# Log configuration changes
audit.log_configuration_change(
    user_id='admin_456',
    change_type='connector_update',
    connector='postgres_prod',
    before={'port': 5432},
    after={'port': 5433},
    timestamp=datetime.utcnow()
)

# Immutable log entries cannot be modified
# Read-only queries only
logs = audit.query(user_id='user_123', days_back=30)
```

---

## COMPLETE EXAMPLE: Production-Grade Pipeline

```yaml
name: secure_financial_sync
version: "2.0"

# DATA PIPELINE DESIGN
source:
  type: postgres
  host: prod.db.com
  query: SELECT * FROM transactions WHERE created_at > :last_sync
  polling:
    frequency: hourly
    incremental_column: created_at

# IDEMPOTENCY
transformation:
  idempotency:
    key_column: transaction_id
    deduplication: enabled
  
  steps:
    # Security: Tokenize PII
    - name: tokenize_pii
      script: |
        record['card_number'] = tokenize(record['card_number'], 'card_token_key')
        record['ssn'] = tokenize(record['ssn'], 'ssn_token_key')
        return record
    
    # Performance: Cache merchant lookups
    - name: enrich_merchant
      cache: enabled
      script: |
        merchant = get_merchant_info(record['merchant_id'])
        record['merchant_category'] = merchant['category']
        return record
    
    # Optimization: Partition early
    - name: add_partition_key
      script: |
        record['date_partition'] = record['created_at'].date()
        return record

# PERFORMANCE: Columnar + Caching
destination:
  type: snowflake
  table: analytics.transactions
  
  # Columnar storage
  table_format: iceberg
  partition_columns: [date_partition, merchant_category]
  
  # Caching
  cache:
    enabled: true
    ttl: 3600
  
  # Smart connection pooling
  connection_pool:
    min_size: 5
    max_size: 20
    idle_timeout: 300

# RELIABILITY: Circuit breaker
  circuit_breaker:
    enabled: true
    failure_threshold: 5
    timeout: 60
    on_open: send_to_dlq

# SECURITY: Encryption + Audit
security:
  encryption:
    at_rest: AES-256-GCM
    in_transit: TLS-1.3
    
  audit:
    enabled: true
    backend: s3_write_once
    log_events: [data_access, configuration_change, auth]

# MONITORING
monitoring:
  metrics: [throughput, latency, error_rate]
  alerts:
    - metric: error_rate
      threshold: 5%
      action: page_oncall
    - metric: circuit_breaker_open
      action: notify_slack
```

---

## Checklist for Production Deployment

- [ ] All transformations are idempotent
- [ ] Circuit breakers configured for external APIs
- [ ] Data partitioning strategy defined
- [ ] Columnar storage format selected
- [ ] Query plans analyzed and optimized
- [ ] End-to-end encryption enabled
- [ ] Sensitive fields tokenized or encrypted
- [ ] Immutable audit logs configured
- [ ] RBAC configured per connector
- [ ] Monitoring and alerting active
- [ ] Disaster recovery tested
- [ ] Performance benchmarks established

---

**Next:** [Advanced Patterns](ADVANCED_ARCHITECTURE_PATTERNS.md) | [ETL/ELT Architecture](ETL_ELT_ARCHITECTURE.md)
