# PyReverseETL

**The Open Operational Data Activation Runtime**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
![Version: v2.0.0](https://img.shields.io/badge/Version-v2.0.0-blue)
![Status: Production Ready](https://img.shields.io/badge/Status-Production%20Ready-brightgreen)

PyReverseETL is an open-source, Rust-powered platform for operationalizing warehouse intelligence across all business systems. It goes beyond traditional Reverse ETL by focusing on **activation** rather than synchronization.

## Philosophy

Traditional Reverse ETL platforms move rows from warehouses into SaaS applications. PyReverseETL moves **business intent**.

```
Business Intelligence
        ↓
Operational Activation
        ↓
Business Outcomes
```

The platform sits between analytical systems and operational systems, continuously delivering trusted intelligence to where action happens.

## Core Capabilities

### Event Sources (NEW in v2.0)
- **Kafka** — Real-time event streaming from Kafka topics
- **CDC** — Change Data Capture from databases (PostgreSQL, MySQL, MongoDB)
- **API Polling** — REST endpoint polling and webhook receivers
- **Scheduled Polling** — Configurable intervals (5min to 24hours)
- **Change Detection** — Automatic detection of data changes
- **Event Metadata** — Preserve source context (topic, partition, offset, key)

### Data Transformations (NEW in v2.0)
- **PySpark Processing** — Real-time Spark transformations
- **Multi-Stage Pipelines** — Chain transformations with error handling
- **Intermediate Staging** — Use Kafka topics between stages
- **Cost Optimization** — Filter data early in pipeline
- **Feature Engineering** — ML-ready feature preparation
- **Spark Cluster Support** — Local, YARN, Kubernetes deployment

### Synchronization Engine
- **Batch Sync** — Scheduled synchronization
- **Incremental Sync** — Change-based synchronization
- **CDC Sync** — Database change streams
- **Streaming Sync** — Kafka, Pulsar, Redpanda
- **Event-Driven Sync** — Trigger on business events
- **Hybrid Sync** — Combine scheduling with events

### Destination Ecosystem
- **CRM** — Salesforce, HubSpot, Dynamics
- **Marketing** — Braze, Iterable, Customer.io, Mailchimp, Klaviyo
- **Advertising** — Meta Ads, Google Ads, LinkedIn Ads, Amazon Ads, TikTok Ads
- **Support** — Zendesk, Freshdesk, Intercom
- **Analytics** — Mixpanel, Amplitude, PostHog
- **Data Platforms** — Kafka, Pulsar, Redpanda
- **Custom** — Webhooks, custom connectors

### Activation Objects
- **Entities** — Customer, Account, Company, Lead, Subscription, Order, Product
- **Traits** — Dynamic attributes (LTV, Churn Risk, Lead Score, etc.)
- **Audiences** — Segmented groups (VIP Customers, Churn Risk, etc.)
- **Metrics** — Business measurements (MRR, ARR, NPS, etc.)
- **Events** — Business events (Subscription Renewed, Payment Failed, etc.)

## Architecture

**Rust Core**
- Sync runtime
- Scheduling engine
- State management
- Connector runtime
- Telemetry

**Python Layer**
- SDK and bindings
- Custom extensions
- AI integrations
- Developer experience

**Persistence**
- SQLite (v1.5 - lightweight, no setup required)
- PostgreSQL (v2.0+)
- DuckDB (v2.0+)

## Getting Started

### Installation

```bash
pip install pyreverseetl
```

### Quick Example

```python
from pyreverseetl import Workflow, Destination, Activation

# Define a workflow
workflow = Workflow.from_table(
    name="LTV to CRM",
    table="customers",
    owner="data_team"
)

# Add field mappings
workflow.add_mapping("customer_id", "customerId")
workflow.add_mapping("lifetime_value", "customerLTV")
workflow.add_mapping("segment", "segment")

# Define destination
salesforce = Destination.salesforce(
    name="Production Salesforce",
    instance_url="https://yourinstance.salesforce.com",
    api_version="v60.0"
)

# Create activation
activation = Activation(
    name="Daily LTV Sync",
    workflow=workflow,
    destination=salesforce
)

# Schedule
activation.schedule_daily(hour=2, minute=0)

# Execute
run = activation.execute()
print(f"Synced {run.rows_processed} records")
```

### Kafka Event Source with Polling

```python
from pyreverseetl import KafkaSource, KafkaConfig, SyncFrequency

# Configure Kafka source
kafka_config = KafkaConfig(
    brokers="localhost:9092",
    topic="customer-events",
    group_id="pyreverseetl-consumer"
)

# Create source with hourly polling
source = KafkaSource(kafka_config)
source.set_sync_frequency(SyncFrequency.Hourly)

# Connect and poll for events
source.connect()
while True:
    event = source.next_event()
    if event:
        print(f"Received: {event.entity_id} from {event.source}")
```

### PySpark Data Transformation Pipeline

```python
from pyreverseetl import (
    SparkTransformer, SparkConfig, TransformationPipeline, TransformationStage
)

# Define transformation stages
normalize_stage = TransformationStage(
    name="normalize",
    config=SparkConfig(
        script="/path/to/normalize.py",
        input_topic="raw-events",
        output_topic="normalized-events"
    ),
    retry_count=3,
    skip_on_error=False
)

enrich_stage = TransformationStage(
    name="enrich",
    config=SparkConfig(
        script="/path/to/enrich.py",
        input_topic="normalized-events",
        output_topic="enriched-events"
    ),
    retry_count=2,
    skip_on_error=False
)

# Create pipeline
pipeline = TransformationPipeline()\
    .add_stage(normalize_stage)\
    .add_stage(enrich_stage)

# Execute pipeline
for stage in pipeline.stages:
    transformer = SparkTransformer(stage.config)
    result = transformer.execute()
    print(f"{stage.name}: {result.records_output} records output")
```

## Core Concepts

### Workflows
Define data sources and how to extract them:
- Table extraction
- Model extraction
- SQL queries
- Audience definitions
- Event streams

### Activations
Connect workflows to destinations with policies:
- Field mappings
- Transformations
- Validation gates
- Error handling
- Scheduling

### Sync Runs
Track execution:
- Status (Pending → Running → Success/Failed)
- Row counts
- Error details
- Timing information

## Ecosystem

PyReverseETL is part of a larger platform:

- **StatGuardian** — Data quality and contracts (ensures data is trustworthy)
- **ClusterAudienceKit** — Customer segmentation (identifies who matters)
- **PyStreamMCP** — Query optimization & context discovery (60-75% cost reduction)
- **PyReverseETL** — Data activation (operationalizes intelligence)
- **PyCustomerJourney** — Customer engagement (drives outcomes)

### Integration with PyStreamMCP

PyReverseETL integrates with **PyStreamMCP** for intelligent context retrieval:

```python
from pyreverseetl import Activation
from pystreammcp import Agent, Discovery

# Use PyStreamMCP to optimize context discovery
discovery = Discovery.new(query_id="activation_1")
sources = discovery.discover_sources()  # Find optimal data
optimized = discovery.optimize_for_cost()  # Reduce volume

# Activate with optimized context
activation = Activation(
    name="Smart LTV Sync",
    query=optimized,  # Use PyStreamMCP's optimized query
    destination="salesforce"
)
```

**Do NOT rebuild query optimization in PyReverseETL.** PyStreamMCP provides:
- Query planning and optimization (60-75% token reduction)
- Intelligent source discovery
- Cost estimation
- Progressive streaming retrieval
- Multi-step query decomposition

See [ARCHITECTURE.md](ARCHITECTURE.md#with-pystreammcp-query-optimization--context-discovery) for details.

## Observability & Open-Source Stack

PyReverseETL includes full OpenTelemetry integration for observability. Works with any open-source monitoring backend:

- **Metrics:** Prometheus, OpenMetrics
- **Traces:** Jaeger, Tempo
- **Logs:** Loki, OpenSearch
- **Dashboards:** Grafana, custom tools
- **Alerts:** Alert Manager, native backend support

See [OSS_ALTERNATIVES.md](docs/OSS_ALTERNATIVES.md) for complete open-source stack recommendations and setup guides.

## Roadmap

### ✅ Phase 1: Core Foundation (v1.0.0)
- Core data model (Workflow, Activation, Destination, Entity)
- SQLite persistence with CRUD repositories
- Builder patterns for ergonomic API
- 59 tests passing

### ✅ Phase 2: Destination Ecosystem (v1.1.0)
- 4 Production adapters (Webhook, Salesforce, HubSpot, Marketo)
- YAML-based field mapping configuration
- Automatic schema detection with type inference
- OpenTelemetry-compatible alert message structures
- 48 tests passing

### ✅ Phase 3 Week 1: Resilience & HTTP (v1.1.5)
- Exponential backoff retry logic
- Production HTTP client with connection pooling
- OAuth token manager with automatic refresh
- 24 tests passing

### ✅ Phase 3 Weeks 3-4: Real-Time Activation (v1.5.0)
- Change Data Capture (CDC) engine with changelog persistence
- Real-time activation pipeline with latency tracking
- Backpressure management and checkpoint recovery
- 36 new tests (178 total)

### ✅ Phase 4: Event Sources & Transformations (v2.0.0 → v2.0.1)
- **Event Sources**: Kafka connector with SSL/SASL support
- **Sync Frequency**: Configurable polling (5min-24hours) with timezone support
- **Change Detection**: Track changes at preset intervals
- **PySpark Transformations**: Multi-stage processing pipelines (optional)
- **Intermediate Staging**: Kafka topics between transformation stages
- **YAML Configuration**: Load/save configurations from YAML files
- **Separate Source/Destination Polling**: Different schedules per system
- **Transformation Error Handling**: Dead letter topics, retries, caching
- **Detailed Status Messages**: Congratulatory success + actionable error messages
- **Timezone Support**: IANA timezone database (400+ timezones)
- **Day-of-Week & Blackout Filtering**: Skip syncs on specific days/dates
- **Fault Tolerance & Caching**: Result caching for reliability
- **Auto-Scaling**: Kafka (by lag/throughput) & PySpark (by size/latency)
- 50+ new tests (265+ total passing)

## Platform Philosophy

PyReverseETL should be:

- **Rust-powered** — Performance and reliability
- **Python-extensible** — Ecosystem and integration
- **OpenTelemetry-native** — Observability from day one
- **Deployment-agnostic** — Laptop to Kubernetes
- **Warehouse-native** — Snowflake, BigQuery, Databricks, DuckDB, Postgres
- **Event-aware** — React to business events
- **AI-assisted** — Learn from historical patterns
- **Lineage-aware** — Track data flow and impact
- **Production-ready** — Multi-tenant, secure, scalable

while remaining **simple enough for data teams to adopt**.

## Development

### Building

```bash
# Build Rust core
cargo build -p pyreverseetl-core

# Build Python bindings
maturin develop

# Run tests
cargo test
pytest tests/
```

### Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT License. See [LICENSE](LICENSE) for details.

## Support

- GitHub Issues: [PyReverseETL/issues](https://github.com/Mullassery/PyReverseETL/issues)
- Discussions: [PyReverseETL/discussions](https://github.com/Mullassery/PyReverseETL/discussions)

---

**PyReverseETL: Operationalize Your Data Intelligence**
