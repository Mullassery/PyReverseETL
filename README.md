# PyReverseETL

**The Open Operational Data Activation Runtime**

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
- SQLite (local)
- PostgreSQL (enterprise)
- DuckDB (analytics)

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
- **PyReverseETL** — Data activation (operationalizes intelligence)
- **PyAudienceJourney** — Customer engagement (drives outcomes)

## Features

### ✅ Phase 1: Core Foundation (Current)
- Core data model (Workflow, Activation, Destination, Entity)
- SQLite persistence
- Builder patterns for ergonomic API
- 18+ unit tests

### 🚧 Phase 2: Runtime Engine (In Progress)
- Scheduler for time-based triggers
- Executor for action execution
- State manager for transitions
- Condition evaluator

### 📋 Phase 3: Full Activation (Planned)
- Batch and incremental sync
- CDC pipeline
- Streaming integrations
- Bidirectional sync

### 🔮 Phase 4: Advanced Features (Planned)
- Entity graph synchronization
- Activation lineage tracking
- Activation analytics
- AI-assisted workflows
- Enterprise features

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
- **Enterprise-ready** — Multi-tenant, secure, scalable

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
