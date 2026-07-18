# Parallel Topic Synchronization - PyReverseETL v2.0.1

**Multiple Kafka topics syncing simultaneously, each at its own pace, without blocking each other.**

---

## Question: Can Multiple Topics Sync in Parallel?

### The Answer: YES ✅

```
Topic A (fast source) ──→ 🔄 Consumer A ──→ Warehouse A
Topic B (slow source) ──→ 🔄 Consumer B ──→ Warehouse B  
Topic C (medium)      ──→ 🔄 Consumer C ──→ Warehouse C

Each topic:
- Independent polling frequency
- Independent transformation
- Independent destination
- NO blocking between topics
- Each proceeds at its own optimal rate
```

---

## Architecture: Parallel Consumer Groups

```
                    ┌─────────────────────────────┐
                    │   Kafka Cluster             │
                    │                             │
        ┌──────────┐│  ┌────────┐  ┌─────────┐   │
        │ Topic A  │├─→│Partition│  │Partition│   │
        └──────────┘│  └────────┘  └─────────┘   │
                    │                             │
        ┌──────────┐│  ┌────────┐                 │
        │ Topic B  │├─→│Partition│                 │
        └──────────┘│  └────────┘                 │
                    │                             │
        ┌──────────┐│  ┌────────┐                 │
        │ Topic C  │├─→│Partition│                 │
        └──────────┘│  └────────┘                 │
                    └─────────────────────────────┘
                              ↓↓↓
                    ┌─────────────────────────────┐
                    │  Parallel Consumers         │
                    │                             │
                    │  Consumer A (Group A)       │
                    │    Offset: 1000/10000       │
                    │    Lag: 9000                │
                    │    Rate: 100 msg/sec        │
                    │                             │
                    │  Consumer B (Group B)       │
                    │    Offset: 500/500          │
                    │    Lag: 0 (caught up)       │
                    │    Rate: 10 msg/sec         │
                    │                             │
                    │  Consumer C (Group C)       │
                    │    Offset: 7500/8000        │
                    │    Lag: 500                 │
                    │    Rate: 50 msg/sec         │
                    │                             │
                    └─────────────────────────────┘
                              ↓↓↓
                    ┌─────────────────────────────┐
                    │  Independent Processing     │
                    │                             │
                    │  Transform A ──→ Warehouse  │
                    │  Transform B ──→ DataLake   │
                    │  Transform C ──→ Analytics  │
                    │                             │
                    │ (No shared resources)       │
                    │ (No blocking)               │
                    │ (No conflicts)              │
                    └─────────────────────────────┘
```

---

## Configuration: Multi-Topic Sync

### Example 1: Three Independent Syncs

```python
from pyreverseetl_core.sources import SyncConfiguration

# Sync 1: Events → Warehouse (high volume, bursty)
config_a = SyncConfiguration::new("events_to_warehouse")
    .with_source_polling(PollingConfig::with_frequency(SyncFrequency::FiveMinutes))
    .with_destination_polling(PollingConfig::with_frequency(SyncFrequency::Daily))

# Sync 2: Logs → Analytics (low volume, consistent)
config_b = SyncConfiguration::new("logs_to_analytics")
    .with_source_polling(PollingConfig::with_frequency(SyncFrequency::ThirtyMinutes))
    .with_destination_polling(PollingConfig::with_frequency(SyncFrequency::Hourly))

# Sync 3: Metrics → Monitoring (real-time)
config_c = SyncConfiguration::new("metrics_to_monitoring")
    .with_source_polling(PollingConfig::with_frequency(SyncFrequency::FiveMinutes))
    .with_destination_polling(PollingConfig::with_frequency(SyncFrequency::Hourly))

# All three run in parallel:
# - No blocking
# - No interference
# - Independent consumer groups
# - Independent transformations
# - Independent destinations
```

### Example 2: YAML Configuration for Multiple Topics

```yaml
# config/sync_a.yaml
name: events_to_warehouse
source_polling:
  frequency: FiveMinutes
destination_polling:
  frequency: Daily

---

# config/sync_b.yaml  
name: logs_to_analytics
source_polling:
  frequency: ThirtyMinutes
destination_polling:
  frequency: Hourly

---

# config/sync_c.yaml
name: metrics_to_monitoring
source_polling:
  frequency: FiveMinutes
destination_polling:
  frequency: Hourly
```

```python
# Load all configurations
configs = [
    SyncConfiguration.from_yaml_file("config/sync_a.yaml"),
    SyncConfiguration.from_yaml_file("config/sync_b.yaml"),
    SyncConfiguration.from_yaml_file("config/sync_c.yaml"),
]

# Start all syncs in parallel
for config in configs:
    start_sync(config)  # Non-blocking, runs in background
```

---

## Parallel Execution Model

### Tokio Async Runtime
```rust
// PyReverseETL uses Tokio (async runtime)
// Supports millions of concurrent tasks

#[tokio::main]
async fn main() {
    // Start 3 independent syncs
    let task_a = tokio::spawn(sync_topic_a());
    let task_b = tokio::spawn(sync_topic_b());
    let task_c = tokio::spawn(sync_topic_c());
    
    // All run in parallel
    // No thread blocking
    // Efficient resource usage
    
    // Wait for all to complete (or run forever)
    tokio::join!(task_a, task_b, task_c);
}
```

### Non-Blocking I/O
```
Sync A: Reading from Kafka (I/O wait)
    ↓
  (No blocking! Tokio switches to Sync B)
    ↓
Sync B: Processing transformation
    ↓
  (Sync A I/O complete, Tokio switches back)
    ↓
Sync A: Writing to destination (I/O wait)
    ↓
  (No blocking! Tokio switches to Sync C)
    
Efficiency: 3 tasks on 1-2 CPU cores (no thread overhead)
```

---

## Real-World Example: E-Commerce Platform

### Three Parallel Syncs

**Sync 1: Order Events → Data Warehouse**
```yaml
name: orders_to_warehouse
description: High-volume order events for analytics

source_polling:
  frequency: FiveMinutes      # Catch orders quickly
  timezone: UTC
  
transformation:
  engine: PySpark
  intermediate_topic: orders_staging
  
destination_polling:
  frequency: Daily            # Batch load daily
  timezone: America/New_York
  no_sync_after_hour: 22
  sync_resume_hour: 6

# Throughput: 1000s of orders/sec
# Latency: 5 min to warehouse
```

**Sync 2: Customer Updates → CRM**
```yaml
name: customers_to_crm
description: Customer profile changes to Salesforce

source_polling:
  frequency: Hourly           # Less frequent
  timezone: UTC
  
transformation:
  engine: Python              # Simple mapping
  
destination_polling:
  frequency: Hourly
  timezone: America/Chicago

# Throughput: 10s of updates/sec
# Latency: 1 hour to Salesforce
```

**Sync 3: Analytics → Dashboard**
```yaml
name: analytics_to_dashboard
description: Real-time metrics for business dashboard

source_polling:
  frequency: FiveMinutes      # Real-time
  timezone: UTC
  
transformation:
  engine: Python              # Simple aggregation
  
destination_polling:
  frequency: FiveMinutes      # Real-time consumption
  timezone: UTC

# Throughput: 100s of metrics/sec
# Latency: 5 min to dashboard
```

### Parallel Execution Timeline

```
Time    Sync A (Orders)      Sync B (Customers)     Sync C (Analytics)
────    ─────────────────    ──────────────────     ──────────────────
00:00   Poll orders          Poll customers         Poll analytics
        │1000 events         │10 updates            │500 metrics
        ↓                    ↓                      ↓
00:01   Transform            Transform              Transform
        │PySpark job         │Simple map            │Aggregation
        ↓                    ↓                      ↓
00:02   [Working...]         Write to CRM           Write to DB
        │Staging topic       │Success!              │Success!
        ↓                    
00:03   [Still working...]   Wait 59 min            Poll analytics
        │                                          │[repeat]
00:04   Write to warehouse   Wait 58 min
        │Success!
        
Key: All three syncs proceed independently
- A takes 4 min, B takes 3 min, C takes 4 min
- No blocking between them
- Each continues at its own pace
- No interference
```

---

## Performance Characteristics

### Independent Consumer Lag

| Sync | Source | Destination | Lag | Rate |
|------|--------|-------------|-----|------|
| A (Orders) | 10k/sec | 100/sec | High | Catching up |
| B (Customers) | 10/sec | 10/sec | 0 | Caught up |
| C (Analytics) | 100/sec | 100/sec | 0 | Caught up |

```
Lag in Topic A doesn't affect B or C!
Topic B isn't starved while A processes!
Topic C isn't blocked by A's high volume!
```

### Scalability

```
Single machine:
- 3 topics in parallel

Multiple machines:
- Machine 1: Topics A, B
- Machine 2: Topic C
- Kafka: Scales to 100s of topics
- Each topic: Independent consumer group
- Each group: Can have multiple consumers
```

---

## Configuration for Parallel Syncs

### Option 1: Individual Files (Recommended)
```
config/
├── sync_orders.yaml
├── sync_customers.yaml
└── sync_analytics.yaml
```

**Advantages:**
- Easy to modify one sync without affecting others
- Version control shows what changed
- Can deploy independently
- Test each sync separately

### Option 2: Single Master Config
```yaml
syncs:
  - name: orders_to_warehouse
    source_polling: {...}
    destination_polling: {...}
    
  - name: customers_to_crm
    source_polling: {...}
    destination_polling: {...}
    
  - name: analytics_to_dashboard
    source_polling: {...}
    destination_polling: {...}
```

**Advantages:**
- Single file to manage
- All syncs defined in one place
- Easier to see all configurations

---

## Monitoring Parallel Syncs

### Health Check Dashboard

```
╔════════════════════════════════════════════════════════╗
║          PyReverseETL Sync Status Dashboard            ║
╠════════════════════════════════════════════════════════╣
║                                                        ║
║ Sync: orders_to_warehouse                    ✅ GOOD  ║
║ ├─ Source lag: 500 msgs (catching up)                 ║
║ ├─ Transform: PySpark (5 min)                         ║
║ ├─ Destination: 10k batches/day                       ║
║ └─ Last run: 2 hours ago                              ║
║                                                        ║
║ Sync: customers_to_crm                      ✅ GOOD  ║
║ ├─ Source lag: 0 msgs (caught up!)                    ║
║ ├─ Transform: Python (1 sec)                          ║
║ ├─ Destination: 100k/day                              ║
║ └─ Last run: 5 minutes ago                            ║
║                                                        ║
║ Sync: analytics_to_dashboard                ✅ GOOD  ║
║ ├─ Source lag: 0 msgs                                 ║
║ ├─ Transform: Python (2 sec)                          ║
║ ├─ Destination: Real-time                             ║
║ └─ Last run: 2 minutes ago                            ║
║                                                        ║
╚════════════════════════════════════════════════════════╝
```

### Key Metrics to Monitor

| Metric | What It Means | Healthy | Warning | Critical |
|--------|---------------|---------|---------|----------|
| **Consumer Lag** | How behind is sync? | 0-100 msgs | 1k-10k | > 100k |
| **Poll Interval** | How often polled? | On schedule | Delayed 1x | Delayed 10x+ |
| **Transform Time** | How long to process? | < target | Target ±20% | > target 2x |
| **Error Rate** | % of events failing? | < 0.1% | 0.1-1% | > 1% |
| **Dead Letter Queue** | Failed events? | 0 | < 100 | > 1000 |

---

## Best Practices for Parallel Syncs

### 1. Independent Configuration Files
```
✅ Good
sync_orders.yaml     # Modify without affecting others
sync_customers.yaml
sync_analytics.yaml

❌ Bad
sync.yaml            # One file = tight coupling
```

### 2. Separate Consumer Groups
```yaml
# Sync A
source_polling:
  consumer_group: orders_consumers    # Unique per sync!

# Sync B  
source_polling:
  consumer_group: customers_consumers # Different group
```

### 3. Different Intermediate Topics
```yaml
# Sync A
transformation:
  intermediate_topic: orders_staging

# Sync B
transformation:
  intermediate_topic: customers_staging  # Don't share!
```

### 4. Monitor Each Independently
```python
metrics_a = SyncMetrics.for_sync("orders_to_warehouse")
metrics_b = SyncMetrics.for_sync("customers_to_crm")

# Alert on each independently
if metrics_a.consumer_lag > 10000:
    alert("Orders sync lagging!")
```

### 5. Scale Independently
```yaml
# Sync A needs high throughput
source_polling:
  frequency: FiveMinutes

# Sync B needs low latency
destination_polling:
  frequency: FiveMinutes

# Sync C is fine slow
source_polling:
  frequency: Daily
```

---

## Troubleshooting Parallel Syncs

### Problem: One sync is slow, others blocked
**Cause:** Shared resources (CPU, memory)  
**Solution:** Run on separate machines or containers

### Problem: Lag building up in Topic A only
**Cause:** Consumer for A is slow  
**Solution:** Increase parallelism or destination capacity for A only

### Problem: All syncs slowing down
**Cause:** Kafka broker overloaded  
**Solution:** Scale Kafka (add brokers or machines)

### Problem: Topic B not syncing
**Cause:** Consumer group for B crashed  
**Solution:** Restart consumer for B (doesn't affect A or C)

---

## Summary

| Aspect | Guarantee |
|--------|-----------|
| **Parallel?** | Yes ✅ (independent tasks) |
| **Blocking?** | No ✅ (async non-blocking) |
| **Interference?** | No ✅ (separate consumer groups) |
| **Independent rates?** | Yes ✅ (each polls at own frequency) |
| **Scalability** | High ✅ (100s of topics on 1 machine) |

**Result:** Multiple Kafka topics synced in parallel, each at its own optimal pace, without delay or interference.

See [SYNC_CONFIGURATION.md](SYNC_CONFIGURATION.md) for configuration reference.
