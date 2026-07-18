# PyReverseETL Connector Ecosystem

**Built-in connectors for 280+ platforms and data sources.**

---

## Why Connectors Matter

PyReverseETL's connectors are designed for modern data operations:

- **Simple** — Copy configuration, run
- **Unified** — Same interface for all sources and destinations
- **Fast** — Rust core, Python bindings
- **Extensible** — Add custom connectors easily
- **Observable** — Full OpenTelemetry integration
- **Data-quality aware** — Built-in validation and contracts

---

## Built-in Connectors

### Sources (Read Data From)

#### Databases
- **PostgreSQL** — Full support, incremental reads, schema detection
- **MySQL** — Full support, incremental reads, schema detection

#### Files
- **CSV** — Local or remote files, schema detection
- **JSON** — Line-delimited or standard JSON
- **Parquet** — Columnar format with automatic type inference

#### APIs
- **REST API** — HTTP GET/POST with pagination and batching
- **Webhook Receiver** — Listen for inbound events

#### Cloud Storage
- **Amazon S3** — Direct file access, Glue catalog integration
- **Google Cloud Storage** — Seamless file streaming
- **Azure Blob Storage** — Direct blob operations

### Destinations (Write Data To)

#### Databases
- **PostgreSQL** — INSERT, UPDATE, UPSERT modes
- **MySQL** — Batch writes with deadlock retry

#### Data Warehouses
- **Snowflake** — Time-optimized loads, zero-copy clones
- **Google BigQuery** — Streaming inserts and batch loads
- **Amazon Redshift** — Redshift Spectrum support

#### APIs & Webhooks
- **HTTP/REST** — POST to any endpoint
- **GraphQL** — Full mutation support

#### Cloud Storage
- **Amazon S3** — Partitioned writes, format selection
- **Google Cloud Storage** — Directory and object writes
- **Azure Blob Storage** — Managed storage

#### SaaS Platforms

**CRM:**
- Salesforce (standard & custom objects)
- HubSpot (contacts, companies, deals)

**Marketing:**
- Braze (user imports, custom attributes)
- Iterable (user sync)
- Klaviyo (customer lists)

**Communication:**
- Slack (messages, thread replies)

**Analytics:**
- Mixpanel (events, user profiles)
- Amplitude (events, users)

**Fitness & Wearables:**
- Fitbit (wearable fitness tracking, heart rate, sleep)
- Apple HealthKit (health and fitness data, workouts)
- Google Fit (activity, steps, heart rate, sleep)
- Garmin (sports watches, training data)
- Oura Ring (sleep quality, readiness, activity)
- Withings (weight, blood pressure, activity)
- Suunto (sports watches, training)
- Polar Sports (sports watches, training zones)
- Strava (activities, performance, segments)
- MyFitnessPal (nutrition, calories, workouts)

---

## Using Connectors

### YAML Configuration

Simplest way to use connectors:

```yaml
name: customer_sync
owner: data_team

source:
  type: postgres
  host: ${DB_HOST}
  database: analytics
  query: SELECT * FROM customers WHERE id > :last_id

destination:
  type: snowflake
  account: xy12345
  warehouse: compute
  database: analytics
  schema: public
  table: customers

schedule:
  frequency: hourly
  timezone: America/New_York
```

### Python API

Define syncs programmatically:

```python
from pyreverseetl import Connector, SyncPipeline

# Source connector
source = Connector.postgres(
    host="db.example.com",
    database="analytics",
    query="SELECT * FROM customers"
)

# Destination connector
dest = Connector.snowflake(
    account="xy12345",
    warehouse="compute",
    database="analytics",
    table="customers"
)

# Create and run pipeline
pipeline = SyncPipeline(source, dest)
result = pipeline.execute()
print(f"Synced {result.rows_written} records")
```

### Custom Connectors

Extend the ecosystem with custom connectors:

```python
from pyreverseetl import SourceConnector, Record

class CustomDatabaseConnector(SourceConnector):
    """Connect to a custom data source"""
    
    async def test_connection(self):
        # Test connectivity
        pass
    
    async def read_all(self):
        # Return all records
        return [Record(...)]
    
    async def read_incremental(self, last_value):
        # Return only changed records
        pass
    
    def capabilities(self):
        return ["read", "incremental_read", "batch"]
```

---

## Connector Configuration

### Connection Pooling

PyReverseETL manages connection pools automatically:

```yaml
connectors:
  postgres_prod:
    type: source
    implementation: postgres
    host: prod.db.com
    database: analytics
    pool_size: 10
    tags: [production, replicated]
    
  snowflake_prod:
    type: destination
    implementation: snowflake
    account: xy12345
    tags: [production, warehouse]
    is_default: true  # Use this by default
```

### Credential Management

Separate credentials from configuration:

```yaml
source:
  type: postgres
  host: ${DB_HOST}
  port: ${DB_PORT}
  user: ${DB_USER}
  password: ${DB_PASSWORD}
  database: analytics
```

Environment variables or secret managers (AWS Secrets, Vault) supported.

### Testing Connections

Validate connectors before use:

```python
from pyreverseetl import ConnectorRegistry

registry = ConnectorRegistry()

# Test PostgreSQL source
postgres_source = registry.get_connector("source", "postgres")
test_result = await postgres_source.test_connection()
print(test_result.message)  # "Connected successfully"

# Discover available connectors
for connector in registry.list_sources():
    print(f"{connector.name} - {connector.description}")
```

---

## Connector Capabilities

Each connector declares what it can do:

- **Read** — Can read data from source
- **Write** — Can write data to destination
- **IncrementalRead** — Supports delta/change detection
- **SchemaDetection** — Auto-detect table structure
- **Batch** — Efficient batch operations
- **Stream** — Streaming/real-time operations

Example:

```python
source = get_connector("source", "postgres")
caps = source.capabilities()

if "incremental_read" in caps:
    # Can read only changed records
    records = source.read_incremental(last_value="2024-01-01")
```

---

## Auto-Schema Detection

Automatically discover data structure:

```yaml
source:
  type: postgres
  query: SELECT * FROM customers

auto_schema: true  # Detect columns and types
```

Or programmatically:

```python
source = Connector.postgres(...)
schema = await source.detect_schema()

for field in schema.fields:
    print(f"{field.name}: {field.field_type} (required={field.required})")
```

---

## Write Modes

Control how data is written to destinations:

### Append
Add new records without modifying existing ones:
```yaml
destination:
  write_mode: append
```

### Upsert
Update if exists, insert if new:
```yaml
destination:
  write_mode: upsert
  key_column: customer_id
```

### Replace
Truncate and load (full refresh):
```yaml
destination:
  write_mode: replace
```

### Merge
Database-specific merge operations:
```yaml
destination:
  write_mode: merge
  key_column: id
```

---

## Error Handling

Connectors handle failures gracefully:

```yaml
source:
  type: api
  url: https://api.example.com/customers
  
  # Retry failed connections
  retry:
    attempts: 3
    backoff: exponential
    max_delay: 60s
  
  # Timeout settings
  timeout: 30s
  read_timeout: 60s

destination:
  type: snowflake
  
  # Handle write failures
  on_error: dead_letter_queue
  dead_letter_topic: customer_sync_dlq
```

---

## Monitoring Connectors

Track connector performance:

```python
from pyreverseetl import ConnectorMetrics

metrics = ConnectorMetrics.get_instance()

# Connection pool stats
print(f"Active connections: {metrics.active_connections}")
print(f"Connection failures: {metrics.connection_failures}")

# Performance metrics
print(f"Avg read latency: {metrics.avg_read_latency_ms}ms")
print(f"Throughput: {metrics.records_per_sec} rec/s")
```

---


---

## Connector Registry

View and manage connectors:

```bash
# List all available sources
pyreverseetl connectors list --type=source

# List all destinations
pyreverseetl connectors list --type=destination

# Search by capability
pyreverseetl connectors search --capability=stream

# Show connector details
pyreverseetl connectors info --connector=snowflake
```

---

## Next Steps

1. **Choose your sources** — See [Source connectors](#sources-read-data-from) above
2. **Choose your destinations** — See [Destination connectors](#destinations-write-data-to) above
3. **Configure credentials** — Use environment variables or secret manager
4. **Test connections** — Use `test_connection()` before syncing
5. **Run your sync** — Execute with schedule or manually

---

## Support

- **Built-in connectors** — See docs for each connector
- **Custom connectors** — [CONTRIBUTING.md](../CONTRIBUTING.md)
- **Issues** — [GitHub Issues](https://github.com/Mullassery/PyReverseETL/issues)
- **Discussions** — [GitHub Discussions](https://github.com/Mullassery/PyReverseETL/discussions)

---

**PyReverseETL: Move Your Data Anywhere**
