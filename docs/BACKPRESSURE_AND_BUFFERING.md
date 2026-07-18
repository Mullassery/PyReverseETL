# Backpressure & Buffering Guide - PyReverseETL v2.0.1

Handling traffic bursts and rate limiting with intermediate storage and backpressure management.

## Problem Statement

Real-world data pipelines face challenges:
- **Traffic bursts:** 1000x normal load with no warning
- **Rate limiting:** Destination has max 100 req/sec but source sends 1000 req/sec
- **Uneven flows:** Source fast, destination slow (or vice versa)
- **External failures:** Downstream rate limits kick in, causing cascading failures

PyReverseETL v2.0.1 handles these with **intermediate storage and backpressure management**.

---

## Architecture: Source → Buffer → Destination

```
┌─────────────────────────────────────────────────────────────────┐
│                      Event Stream                               │
│              (millions of events, bursty traffic)               │
└───────────────┬─────────────────────────────────────────────────┘
                │
                ▼
        ┌───────────────┐
        │     Source    │ ◄─── Fast, unpredictable load
        │    Polling    │
        └───────┬───────┘
                │
                ▼
    ┌───────────────────────────┐
    │  Intermediate Storage     │ ◄─── BUFFER: Handles bursts
    │  (Kafka Topic or Local)   │      Decouples source/destination
    │                           │      Dead letter for failures
    │  ├─ Staging topic        │
    │  ├─ Dead letter topic    │
    │  └─ Cache directory      │
    └───────────┬───────────────┘
                │
                ▼
        ┌───────────────┐
        │Transform      │ ◄─── Optional: Python or PySpark
        │(Optional)     │
        └───────┬───────┘
                │
                ▼
    ┌───────────────────────────┐
    │  Destination Polling      │ ◄─── Slow, rate-limited
    │  (Consumer, respects      │      Consumes at safe rate
    │   rate limits, batches)   │
    └───────────────────────────┘
```

---

## Three-Layer Buffering Strategy

### Layer 1: Kafka Topics (Primary Buffer)
**Use when:** High-volume streams, distributed systems  
**Throughput:** Millions of events/second  
**Retention:** Configurable (days to weeks)  
**Recovery:** Full replay capability

```yaml
source_polling:
  frequency: FiveMinutes

transformation:
  enabled: true
  intermediate_topic: kafka_staging  # Layer 1 buffer

destination_polling:
  frequency: Daily
  # Destination pulls from intermediate_topic at safe rate
```

**How it works:**
1. Source writes to `kafka_staging` (fast, no rate limiting)
2. Transformation reads from `kafka_staging`
3. Destination consumes from `kafka_staging` at its rate (300 msg/sec)
4. Kafka handles backpressure via partitions/consumer groups

### Layer 2: Dead Letter Topic (Error Buffer)
**Use when:** Need to track and replay failed events  
**Purpose:** Separate failed events from successful ones  
**Recovery:** Replay from dead letter topic later

```yaml
transformation:
  enabled: true
  dead_letter_topic: transformation_errors

# Separate topic for failed events:
# - Transformation failures
# - Rate limit rejections
# - Timeouts
# - Data validation errors
```

**How it works:**
1. Events fail transformation
2. Sent to `transformation_errors` topic
3. Ops team can investigate and replay
4. Prevents data loss, enables debugging

### Layer 3: Local Cache (Fault Tolerance)
**Use when:** Need offline recovery capability  
**Purpose:** Cache transformation results locally  
**Recovery:** Resume from cache if destination unavailable

```yaml
transformation:
  enabled: true
  enable_caching: true
  cache_dir: /var/cache/pyreverseetl
  max_cache_size_mb: 2048  # 2GB
```

**How it works:**
1. Results cached locally before sending
2. If destination fails, retry from cache
3. Cache is LRU (old data removed first)
4. Enables offline mode / graceful degradation

---

## Handling Backpressure

### Scenario 1: Source Faster Than Destination

```
Source: 10,000 events/sec
Destination: 100 events/sec (rate limited)

Without buffering: LOSS or FAILURE
With buffering: ✅ Handles gracefully
```

**Solution:**
```yaml
# Fast source
source_polling:
  frequency: FiveMinutes  # Check often

# Slow destination respects rate limit
destination_polling:
  frequency: Daily        # Batch consume
  
# Intermediate buffer absorbs difference
transformation:
  intermediate_topic: staging  # Kafka buffer
  max_retries: 5              # Retry on rate limits
  retry_delay_secs: 30        # Exponential backoff
```

### Scenario 2: Destination Rate-Limited

```
Destination API: 100 req/sec max
Your pipeline: 1000 events/sec

Without handling: 9x failures, retry hell
With backpressure: ✅ Queues gracefully
```

**Configuration:**
```yaml
transformation:
  max_retries: 5              # Retry 5 times
  retry_delay_secs: 10        # 10sec initial delay (exponential)
  timeout_secs: 300           # 5 min timeout before failure
  skip_on_error: false        # Don't skip errors
  dead_letter_topic: rate_limit_errors

# Cache results in case of failure
enable_caching: true
cache_dir: /var/cache/api_failures
```

**Retry Policy:**
- Attempt 1: Immediate
- Attempt 2: 10 seconds later
- Attempt 3: ~20 seconds later (exponential)
- Attempt 4: ~40 seconds later
- Attempt 5: ~80 seconds later
- Failed: Send to dead letter topic + cache

### Scenario 3: Sudden Traffic Burst

```
Normal: 100 events/sec
Burst: 50,000 events/sec for 1 hour
Destination: 1000 events/sec

Without buffering: System crash
With buffering: ✅ Smooths traffic
```

**Configuration:**
```yaml
source_polling:
  frequency: FiveMinutes  # Catch everything

transformation:
  intermediate_topic: burst_buffer
  # Kafka can store millions of events
  # Sufficient headroom for 1 hour burst
  # = 50k/sec * 3600 sec = 180M events
  
destination_polling:
  frequency: Hourly  # Slow consumer
  
# Multiple consumers for parallelism
# Each pulls at safe rate
# Together process burst gradually
```

---

## Configuration Checklist

### For High-Volume Streams
```yaml
# 1. Fast source with intermediate buffer
source_polling:
  frequency: FiveMinutes

# 2. Staging topic for buffering
transformation:
  intermediate_topic: staging
  max_retries: 5
  retry_delay_secs: 10
  enable_caching: true
  cache_dir: /var/cache/pyreverseetl
  max_cache_size_mb: 2048
  dead_letter_topic: errors

# 3. Slow destination respecting limits
destination_polling:
  frequency: Daily
```

### For Rate-Limited Destinations
```yaml
# Configuration for strict rate limiting
transformation:
  max_retries: 10              # More retries
  retry_delay_secs: 30         # Longer delays
  timeout_secs: 600            # 10 min timeout
  dead_letter_topic: rate_limits
  enable_caching: true         # Keep for replay
  cache_dir: /var/cache/failed_syncs
```

### For Bursty Traffic
```yaml
# Handle sudden spikes
source_polling:
  frequency: FiveMinutes       # Fast polling

transformation:
  intermediate_topic: burst_buffer
  # Large Kafka topic with retention
  # 1+ week retention for sustained load

destination_polling:
  frequency: Hourly            # Gradual consumption
```

---

## Monitoring Backpressure

### Kafka Metrics to Watch
- **Consumer Lag:** How far behind is consumer?
- **Topic Size:** How much data buffered?
- **Throughput:** Events/sec in vs out?

### System Metrics
- **Memory Usage:** Cache size growing?
- **Disk Usage:** Local cache full?
- **Processing Latency:** How long in buffer?

### Alerts to Set
```
IF consumer_lag > 1_000_000 events:
  "⚠️ Backpressure building - destination slow"

IF cache_size > 80% of max:
  "⚠️ Cache near full - risk of data loss"

IF dead_letter_count > 100:
  "❌ Errors building - investigate failures"
```

---

## Recovery Strategies

### Strategy 1: Replay from Dead Letter
```python
# After fixing destination issue
config = SyncConfiguration.from_yaml_file("sync.yaml")
dead_letter_topic = config.transformation.dead_letter_topic

# Replay all failed events
replay_events_from_topic(dead_letter_topic)
```

### Strategy 2: Resume from Cache
```python
# After downstream recovers
cache_dir = config.transformation.cache_dir

# Replay cached results
replay_events_from_cache(cache_dir)
```

### Strategy 3: Gradual Replay
```python
# Avoid overwhelming destination again
for event in failed_events:
    send_to_destination(event)
    time.sleep(0.1)  # Throttle to 10 events/sec
```

---

## Performance Tuning

### Increase Buffer Capacity
```yaml
transformation:
  intermediate_topic: staging
  # Kafka automatically scales partitions
  # More partitions = more parallelism
  # Kafka manages retention automatically
```

### Optimize Batch Sizes
```python
# Large batches for throughput
destination.batch_size = 1000
destination.batch_timeout_ms = 5000

# Small batches for latency
destination.batch_size = 10
destination.batch_timeout_ms = 100
```

### Parallelize Consumers
```yaml
# Multiple destination consumers
# Each respects rate limit independently
# Together, higher throughput
destination_polling:
  consumer_group: warehouse_consumers
  # Scale consumers to 10x parallelism
  # = 10 x 100 req/sec = 1000 req/sec
```

---

## Common Issues & Solutions

### Issue: "Buffer Full" / Cache At Capacity
**Cause:** Destination slower than source  
**Solution:** Increase cache size or scale destination

```yaml
transformation:
  cache_dir: /var/cache/large
  max_cache_size_mb: 10240  # 10GB instead of 2GB
```

### Issue: "Rate Limited" Errors
**Cause:** Destination has strict limits  
**Solution:** Increase retries and delays

```yaml
transformation:
  max_retries: 10
  retry_delay_secs: 60  # Start at 1 min
```

### Issue: "Data Loss" in Bursts
**Cause:** No buffering for peak loads  
**Solution:** Add dead letter topic

```yaml
transformation:
  dead_letter_topic: lost_events
  # Ensures no data loss
  # Can replay later
```

---

## Summary

| Component | Purpose | Throughput | Retention |
|-----------|---------|------------|-----------|
| **Staging Topic** | Primary buffer | Millions/sec | Days-weeks |
| **Dead Letter** | Error tracking | Thousands/sec | Days-weeks |
| **Local Cache** | Fault tolerance | Millions/sec | Until cleared |

**Together, these three layers provide:**
- ✅ Handle traffic bursts 1000x normal
- ✅ Survive rate limiting gracefully
- ✅ Zero data loss (dead letter tracking)
- ✅ Offline recovery capability
- ✅ Decoupled source/destination speeds

See [SYNC_CONFIGURATION.md](SYNC_CONFIGURATION.md) for complete configuration reference.
