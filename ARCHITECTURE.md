# PyReverseETL Architecture & Ecosystem Boundaries

## Mission

**Operationalize Intelligence**

Core Question: Where should trusted intelligence be activated?

## Core Responsibility

PyReverseETL is **exclusively responsible** for:

- **Reverse ETL** — Moving data from warehouses to operational systems
- **Data Activation** — Getting intelligence to where actions happen
- **Entity Synchronization** — Customer, account, company, lead sync
- **Trait Synchronization** — LTV, risk scores, engagement metrics
- **Metric Synchronization** — MRR, ARR, churn rate, NPS
- **Audience Synchronization** — Audience membership propagation
- **Event Activation** — Trigger-based business event routing
- **Destination State Management** — Track sync status per destination
- **Activation Observability** — OpenTelemetry events for all syncs
- **Activation Analytics** — Which activations drive value
- **Activation Lineage** — Which systems received which data

## What We Do NOT Own

These belong to other products:

### ❌ Data Validation & Quality (StatGuardian)
- Data contracts
- Data validation engines
- Drift detection
- Schema profiling
- Freshness monitoring
- Data observability

**Our role:** Consume validation outcomes. Support validation gates that block bad data.

### ❌ Audience Creation (ClusterAudienceKit)
- Segmentation
- Clustering
- RFM analysis
- Audience definition
- Customer scoring

**Our role:** Activate audiences once they exist.

### ❌ Journey Orchestration (PyCustomerJourney)
- Customer journey definition
- Multi-step workflows
- Journey state management
- Communications (email, SMS, push)
- Customer engagement

**Our role:** Activate customers into journey systems.

## Architectural Principles

### 1. Separation of Concerns

```
Warehouse Intelligence
        ↓
StatGuardian (Trust It)
        ↓
Trusted + Valid
        ↓
PyReverseETL (Activate It)
        ↓
Operational Systems
```

### 2. Validation Gates

PyReverseETL can optionally require StatGuardian validation before activation:

```rust
Activation {
    workflow_id: String,
    destination_id: String,
    validation_gate: Option<ValidationGate>,  // from StatGuardian
    // Only sync if validation passes
}
```

### 3. Activation vs Synchronization

- **Synchronization:** Moving rows (traditional Reverse ETL)
- **Activation:** Moving business intent, intelligence, and outcomes

PyReverseETL focuses on activation.

### 4. No Embedded Validation

```rust
// ❌ NOT OUR JOB
fn validate_data(entity: &Entity) -> Result<()> {
    // Check for nulls
    // Check for outliers
    // Check freshness
}

// ✅ OUR JOB
fn activate(entity: &Entity, validation: &ValidationResult) -> Result<()> {
    if !validation.is_valid() {
        return Err("StatGuardian validation failed");
    }
    sync_to_destination(entity)?;
    Ok(())
}
```

## Boundary Examples

### Scenario: Sync Customer LTV to Salesforce

**Who owns what:**

1. **Data exists in warehouse** — Snowflake
2. **StatGuardian validates** — Checks schema, drift, freshness, contracts
3. **PyReverseETL syncs** — Maps fields, calls Salesforce API, tracks status
4. **Salesforce updates** — Customer record with LTV
5. **PyAudienceJourney reacts** — If LTV > threshold, trigger upsell journey
6. **Journey sends email** — Communication happens

### Scenario: Activate Churn Risk Audience

**Who owns what:**

1. **ClusterAudienceKit creates** — Defines churn_risk audience (clustering)
2. **StatGuardian validates** — Checks audience freshness, member counts
3. **PyReverseETL syncs** — Pushes audience to Braze, Meta Ads, HubSpot
4. **Destinations receive** — Audience membership
5. **PyAudienceJourney reacts** — Triggers retention journey
6. **Marketing activates** — Email, SMS, ads

### Scenario: Stream Events from Kafka

**Who owns what:**

1. **Event producer** — Publishes to Kafka topic
2. **PyReverseETL consumes** — Event-driven activation mode
3. **Routes to destinations** — Based on event type
4. **Tracks outcomes** — Attribution, sink status
5. **Publishes results** — Back to warehouse or event system

## Integration Points

### With StatGuardian

```rust
Activation {
    validation_gate: Option<ValidationGate>,
    // Before syncing, check validation status:
    // GET /statguardian/validations/{dataset_id}
}
```

### With ClusterAudienceKit

```rust
Audience {
    // Audience definition comes from ClusterAudienceKit
    id: String,
    // PyReverseETL just syncs membership:
    member_ids: Vec<String>,
}
```

### With PyCustomerJourney

```rust
// PyReverseETL activates customers into journey system
sync_to_destination(
    entity_id="cust_123",
    destination_type="PyCustomerJourney",
    payload=json!({"enter_journey": "upsell_flow"}),
)?;
```

### With PyStreamMCP (Query Optimization & Context Discovery)

**IMPORTANT:** PyReverseETL must NOT rebuild query optimization or context discovery functionality.

```rust
// ✅ CORRECT: Use PyStreamMCP for intelligent context retrieval
use pystreammcp::Agent, Discovery;

// When activating large audiences or complex contexts:
let discovery = Discovery::new(query_id);
let sources = discovery.discover_sources()?;  // Find optimal data sources
let optimized = discovery.optimize_for_cost()?;  // Reduce token/data usage

// Then activate with optimized context
sync_to_destination(
    entity_id="cust_123",
    context=optimized,
    destination="salesforce"
)?;

// ❌ WRONG: Do NOT rebuild this locally
// Do NOT create your own:
//   - Query planning
//   - Context discovery
//   - Token optimization
//   - Cost estimation
//   - Streaming retrieval
// These are PyStreamMCP responsibilities.
```

**Why?** PyStreamMCP is purpose-built for query optimization with:
- Learned relevance models (> 80% accuracy)
- Multi-agent context sharing (+20% savings)
- Complex query decomposition
- Streaming context windows (< 50ms latency)
- 6+ framework integrations

Rebuilding this in PyReverseETL would:
- Duplicate code across ecosystem
- Miss optimization opportunities
- Create maintenance burden
- Introduce inconsistencies

**When to use PyStreamMCP:**
- Fetching large context for activation workflows
- Discovering optimal data sources
- Estimating data volume before sync
- Progressive retrieval for streaming destinations
- Multi-step query planning for complex activations

## Module Structure

```
core/src/
├── lib.rs                 # Public exports
├── error.rs               # Error types (NO validation errors)
├── workflow.rs            # Source definitions
├── destination.rs         # Destination config
├── activation.rs          # Workflow → Destination mapping
├── entity.rs              # Business objects
├── sync.rs                # SyncRun, SyncRecord, SyncStatus
├── connector/             # Destination connectors
│   ├── mod.rs
│   ├── base.rs           # BaseConnector trait
│   ├── salesforce.rs
│   ├── hubspot.rs
│   ├── braze.rs
│   └── ...
├── orchestrator/          # Sync orchestration
│   ├── mod.rs
│   ├── scheduler.rs      # Time-based triggers
│   ├── executor.rs       # Sync execution
│   └── evaluator.rs      # Condition evaluation
└── storage/              # Persistence
    ├── mod.rs
    ├── schema.rs
    └── repository.rs
```

## What's NOT Here

### Validation Engine

```rust
// ❌ NOT IN PyReverseETL
mod validation {
    fn validate_schema() { }
    fn detect_drift() { }
    fn check_freshness() { }
}
```

### Audience Engine

```rust
// ❌ NOT IN PyReverseETL
mod audience {
    fn cluster() { }
    fn rfm_score() { }
    fn define_segment() { }
}
```

### Journey Engine

```rust
// ❌ NOT IN PyReverseETL
mod journey {
    fn execute_step() { }
    fn branch() { }
    fn send_email() { }
}
```

## Testability

Each boundary is enforced through:

1. **Module privacy** — Validation/journey/audience modules don't exist
2. **Type system** — Can't express validation rules in our types
3. **Integration tests** — Test against mocked StatGuardian/ClusterAudienceKit/PyAudienceJourney APIs
4. **Documentation** — Explicit "out of scope" statements

## Philosophy

PyReverseETL is to Reverse ETL what dbt is to transformation and Airflow is to orchestration.

- dbt owns **transformation logic**, not data quality
- Airflow owns **orchestration**, not scheduling individual SQL queries
- PyReverseETL owns **activation**, not data validation

Each product is best in class at its domain precisely because it doesn't try to own everything else.
