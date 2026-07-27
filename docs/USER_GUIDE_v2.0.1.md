# PyReverseETL v2.0.1 - Complete User Guide

**Everything you need to know to use PyReverseETL in production.**

---

## Table of Contents

1. [What Is PyReverseETL?](#what-is-pyreverseetl)
2. [Core Concepts](#core-concepts)
3. [Getting Started](#getting-started)
4. [Configuration Guide](#configuration-guide)
5. [Real-World Examples](#real-world-examples)
6. [Troubleshooting](#troubleshooting)

---

## What Is PyReverseETL?

**PyReverseETL** is a production-grade data synchronization platform that moves data from sources (event streams, APIs, databases) to destinations (data warehouses, CRMs, analytics platforms) with:

- ✅ **Reliable delivery** - Every piece of data reaches its destination exactly once, no duplicates, no loss
- ✅ **High-volume handling** - Works with massive data streams and unexpected traffic spikes
- ✅ **Automatic resource optimization** - Scales resources up and down to minimize costs
- ✅ **Global timezone support** - Works with 400+ world timezones for multi-region deployments
- ✅ **Optional data transformation** - Simple Python or advanced distributed processing options
- ✅ **Automatic failure recovery** - Continues working even if something fails

**Result:** Your data reliably reaches its destination, on schedule, in the right format.

---

## Core Concepts

### 1. Check Frequency
**What:** How often should we check your data source for new information?

**Options:** Every 5 minutes, 15 minutes, 30 minutes, hourly, 4 hours, 12 hours, or daily

**Example:** "Check for new orders every 5 minutes"

### 2. Write Frequency
**What:** How often should we send data to your destination?

**Options:** Can be different from check frequency (fast checking, slower writing)

**Example:** "Write all new orders to warehouse once per day"

### 3. Optional Data Transformation
**What:** Should the data be modified before delivery to the destination?

**Options:** 
- **No transformation** - Send data as-is
- **Simple transformation** - Use Python scripts for mapping, filtering, simple calculations
- **Advanced transformation** - Use distributed processing for complex operations

**Example:** "Reformat order data to match warehouse schema"

### 4. Timezone Support
**What:** Make scheduling decisions in your local time zone, not just UTC

**Impact:** Skip days and time windows work in your time zone

**Example:** "Don't run syncs between 8 PM - 8 AM in New York time"

### 5. Automatic Failure Recovery
**What:** If something fails, the system automatically recovers

**What happens:** Failed data is tracked and can be retried later, plus automatic recovery from temporary issues

---

## Getting Started

### Installation

```bash
pip install pyreverseetl==2.0.1
```

### 5-Minute Example

**Step 1: Create `sync.yaml`**
```yaml
name: my_sync
source_polling:
  frequency: Hourly
  timezone: UTC
destination_polling:
  frequency: Daily
  timezone: UTC
```

**Step 2: Create `run.py`**
```python
from pyreverseetl_core.sources import SyncConfiguration

config = SyncConfiguration.from_yaml_file("sync.yaml")
result = config.validate()
print(result)
```

**Step 3: Run**
```bash
python run.py
```

---

## Configuration Guide

### Basic Setup

The foundation: how often to check for data and when to write it.

#### How Often to Check

```yaml
source_polling:
  frequency: FiveMinutes      # Check every 5 minutes
  # Other options: FifteenMinutes, ThirtyMinutes, Hourly, FourHourly, TwelveHourly, Daily
```

#### Your Time Zone

```yaml
source_polling:
  timezone: America/New_York      # Your local time
  # Other examples: Europe/London, Asia/Tokyo, Australia/Sydney
  
# All scheduling respects your time zone
```

#### Skip Specific Days

```yaml
source_polling:
  skip_days:
    - Saturday
    - Sunday
  # Only sync Monday through Friday
```

#### Business Hours Only

```yaml
destination_polling:
  no_sync_after_hour: 18    # Stop at 6 PM
  sync_resume_hour: 9       # Start at 9 AM
  # No syncing between 6 PM - 9 AM (in your timezone)
```

#### Maintenance Windows

```yaml
source_polling:
  blackout_start: 2026-12-20T00:00:00Z  # Dec 20 at midnight
  blackout_end: 2026-12-26T23:59:59Z    # Dec 26 at end of day
  # No syncing during this period
```

### Data Transformation (Optional)

Optional: Transform data before sending to destination.

#### Simple Data Transformation

```yaml
transformation:
  enabled: true
  engine: Python
  script_path: transform.py
  
  timeout_secs: 60
  skip_on_error: false
```

#### Advanced Data Transformation

```yaml
transformation:
  enabled: true
  engine: PySpark
  script_path: transform.py
  
  timeout_secs: 300
```

### Complete Configuration Example

```yaml
name: complete_example
description: Full-featured sync configuration

# Source: Poll frequently for fast data ingestion
source_polling:
  frequency: FiveMinutes
  enabled: true
  timezone: America/New_York
  skip_days: [Saturday, Sunday]
  no_sync_after_hour: 20
  sync_resume_hour: 8
  blackout_start: null
  blackout_end: null

# Transform: Optional PySpark for complex logic
transformation:
  enabled: true
  engine: PySpark
  script_path: transform.py
  intermediate_topic: staging
  max_retries: 5
  retry_delay_secs: 10
  timeout_secs: 300
  dead_letter_topic: errors
  enable_caching: true
  cache_dir: /var/cache
  max_cache_size_mb: 2048

# Destination: Write at safe rate
destination_polling:
  frequency: Daily
  enabled: true
  timezone: America/New_York
  skip_days: [Saturday, Sunday]
  no_sync_after_hour: 22
  sync_resume_hour: 6
```

---

## Real-World Examples

### Example 1: E-Commerce Orders to Warehouse

**Business Context:**
- Orders come in continuously to Kafka (thousands per minute)
- Data warehouse accepts 1000 rows/minute max
- Need to track exactly how many orders synced
- No syncs during weekend maintenance

**Configuration:**

```yaml
name: orders_to_warehouse
description: Real-time order events to analytics warehouse

source_polling:
  frequency: FiveMinutes        # Catch orders quickly
  timezone: UTC
  skip_days: []                 # 24/7 polling

transformation:
  enabled: true
  engine: PySpark
  script_path: orders_transform.py
  intermediate_topic: orders_staging
  max_retries: 5
  dead_letter_topic: order_errors

destination_polling:
  frequency: Daily              # Batch load once per day
  timezone: America/New_York
  skip_days: [Saturday, Sunday]
  no_sync_after_hour: 22        # Midnight - 6 AM maintenance
  sync_resume_hour: 6
```

**Python Usage:**

```python
from pyreverseetl_core.sources import SyncConfiguration

# Load and validate
config = SyncConfiguration.from_yaml_file("orders_sync.yaml")
result = config.validate()

if result.status == "Success":
    print(f"✅ {config.name} ready to deploy")
    print(f"   Source: {config.source_polling.frequency.label()}")
    print(f"   Destination: {config.destination_polling.frequency.label()}")
```

### Example 2: API Data to Data Lake

**Business Context:**
- REST API provides customer data updates hourly
- Data lake accepts files in S3 bucket
- Need reliable delivery without duplicates
- Support 3 time zones globally

**Configuration:**

```yaml
name: api_to_datalake
description: Customer data sync from SaaS API to S3

source_polling:
  frequency: Hourly
  timezone: UTC

transformation:
  enabled: true
  engine: Python
  script_path: format_for_s3.py

destination_polling:
  frequency: Hourly
  timezone: UTC
```

**Deploy in Multiple Regions:**

```python
# US-East deployment
config_us = SyncConfiguration.from_yaml_file("api_to_datalake.yaml")
config_us.source_polling.set_timezone("America/New_York")
config_us.destination_polling.set_timezone("America/New_York")

# EU deployment
config_eu = SyncConfiguration.from_yaml_file("api_to_datalake.yaml")
config_eu.source_polling.set_timezone("Europe/London")
config_eu.destination_polling.set_timezone("Europe/London")

# Start both deployments
start_sync(config_us)  # Runs in parallel!
start_sync(config_eu)  # No blocking
```

### Example 3: Logs to Analytics with Hourly Schedule

**Business Context:**
- Application logs streamed to Kafka in real-time
- Analytics platform needs hourly rollups
- Skip weekends and nights for cost savings
- Only sync during business hours (9 AM - 6 PM)

**Configuration:**

```yaml
name: logs_to_analytics
description: Parse and aggregate logs to analytics

source_polling:
  frequency: Hourly
  timezone: America/Chicago
  skip_days: [Saturday, Sunday]
  no_sync_after_hour: 18    # 6 PM
  sync_resume_hour: 9       # 9 AM

transformation:
  enabled: true
  engine: Python
  script_path: parse_logs.py

destination_polling:
  frequency: Hourly
  timezone: America/Chicago
  skip_days: [Saturday, Sunday]
```

**Business Benefit:**
- Logs processed during business hours only
- Weekends skipped (saves costs)
- Metrics always available 9 AM - 6 PM Chicago time
- After-hours, historical data available

---

## Monitoring & Operations

### What to Monitor

```python
# Get configuration details
result = config.validate()

# Check source setup
print(f"Source frequency: {result.details.source_skip_days} skip days")
print(f"Source timezone: {config.source_polling.timezone}")
print(f"Source enabled: {config.source_polling.enabled}")

# Check destination setup
print(f"Destination frequency: {result.details.destination_skip_days} skip days")
print(f"Destination time window: {config.destination_polling.no_sync_after_hour}:00")
print(f"Destination timezone: {config.destination_polling.timezone}")
```

### Key Metrics

| Metric | Healthy | Warning | Critical |
|--------|---------|---------|----------|
| **Consumer Lag** | < 100 | 1k-10k | > 100k |
| **Events Synced** | Steady | Fluctuating | Stopped |
| **Error Rate** | < 0.1% | 0.1-1% | > 1% |
| **Dead Letters** | 0-10/day | 100-1k/day | > 10k/day |

---

## Troubleshooting

### "Invalid timezone"

**Error:**
```
Invalid timezone: Bad/Zone
```

**Fix:** Use IANA timezone database names
```yaml
✗ timezone: EST              # Ambiguous!
✓ timezone: America/New_York # Clear!
```

**Valid Timezones:**
- US: America/New_York, America/Chicago, America/Denver, America/Los_Angeles
- Europe: Europe/London, Europe/Paris, Europe/Berlin, Europe/Amsterdam
- Asia: Asia/Tokyo, Asia/Shanghai, Asia/Hong_Kong, Asia/Singapore
- Australia: Australia/Sydney, Australia/Melbourne

### "Syncs not running"

**Causes:**
- `enabled: false` in configuration
- `skip_days` includes today
- In `no_sync_after_hour` window

**Debug:**
```python
print(f"Enabled: {config.source_polling.enabled}")
print(f"Skip day? {config.source_polling.is_skip_day()}")
print(f"In no-sync window? {config.source_polling.is_in_no_sync_window()}")
print(f"Should poll? {config.source_polling.should_poll()}")
```

### "Transformation failing"

**Check:**
1. Script file exists: `ls -la transforms/transform.py`
2. Script is valid Python: `python transforms/transform.py --test`
3. Timeout sufficient: `timeout_secs` > expected run time
4. Dead letter topic exists: Check Kafka topic list

### "Data not arriving"

**Investigate:**
1. Source connected? Check source polling metrics
2. Transformation working? Check dead letter topic
3. Destination accepting? Check destination polling metrics
4. Rate limiting? Check retry counts and delays

---

## Production Best Practices

### 1. Use YAML Files (Not Code)

```python
# Good: Version-controlled configuration
config = SyncConfiguration.from_yaml_file("sync.yaml")

# Avoid: Hardcoded configuration
config = SyncConfiguration.new("my_sync")
config.with_source_polling(...)  # Not version-controlled
```

### 2. Validate on Startup

```python
config = SyncConfiguration.from_yaml_file("sync.yaml")
result = config.validate()

if result.status != "Success":
    print(f"❌ Configuration invalid:\n{result}")
    exit(1)
```

### 3. Monitor Dead Letter Topics

```
Check daily:
  - Are there errors in dead letter topic?
  - Any unusual error patterns?
  - Need to replay from dead letter?
```

### 4. Test Configuration Changes

```bash
# Test new configuration before deploying
python -c "from pyreverseetl_core.sources import SyncConfiguration; \
  print(SyncConfiguration.from_yaml_file('sync.yaml').validate())"
```

### 5. Run Multiple Syncs in Parallel

```python
configs = [
    SyncConfiguration.from_yaml_file("sync_1.yaml"),
    SyncConfiguration.from_yaml_file("sync_2.yaml"),
    SyncConfiguration.from_yaml_file("sync_3.yaml"),
]

# All run independently, in parallel
for config in configs:
    start_sync(config)
```

---

## FAQ

**Q: What if my destination is slower than my source?**  
A: PyReverseETL automatically buffers data in Kafka. Your source can run fast while destination consumes at its own pace.

**Q: Can I have exactly-once delivery?**  
A: Yes! PyReverseETL guarantees exactly-once delivery with idempotent keys and offset tracking.

**Q: What happens if destination goes down?**  
A: Events are cached and retried automatically. No data loss.

**Q: Can I transform data?**  
A: Yes, with optional Python or PySpark transformations.

**Q: Can I sync multiple topics?**  
A: Yes! Each topic runs independently with its own configuration.

**Q: How do I skip weekends?**  
A: Add `skip_days: [Saturday, Sunday]` to your polling config.

**Q: How do I set business hours only?**  
A: Use `no_sync_after_hour` and `sync_resume_hour` with your timezone.

---

## Next Steps

1. **Start Simple:** Copy `examples/polling_config_basic.yaml` and modify
2. **Add Timezone:** Set your timezone in config
3. **Add Filtering:** Add skip days or time windows
4. **Add Transformation:** Enable Python transformation if needed
5. **Monitor:** Check metrics and dead letter topics daily
6. **Scale:** Deploy multiple syncs in parallel as needed

---

## Resources

- [Quick Start Guide](QUICK_START.md) - 5-minute getting started
- [YAML Configuration Guide](YAML_CONFIGURATION.md) - All YAML options
- [Exactly-Once Semantics](EXACTLY_ONCE_SEMANTICS.md) - Delivery guarantees
- [Backpressure & Buffering](BACKPRESSURE_AND_BUFFERING.md) - Handle traffic bursts
- [Parallel Topic Sync](PARALLEL_TOPIC_SYNC.md) - Multiple topics
- [Release Notes](RELEASE_v2.0.1.md) - Complete v2.0.1 features

---

**PyReverseETL: Operational Data Activation, Made Simple**
