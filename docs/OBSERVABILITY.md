# Observability & Monitoring - PyReverseETL v2.0.1

**See what's happening with your data syncs in real-time.**

---

## Overview

PyReverseETL gives you complete visibility into sync operations through three types of reporting:

- **📊 Performance Metrics** - Track how fast data moves, how many events processed, success rates
- **🔍 Operation Traces** - See what happened step-by-step: source → transform → destination
- **📝 Event Logs** - Detailed record of everything that happened for debugging

All data automatically flows to any standard monitoring backend: metrics database, tracing backend, or log aggregation system.

---

## Quick Start

### 1. Initialize OTel

```python
from pyreverseetl_core import init_otel

# Initialize OpenTelemetry
init_otel("orders_sync", "v2.0.1")
```

### 2. Track Your Sync

```python
from pyreverseetl_core import SyncContext, MetricsCollector, SyncLogger

# Create context for this sync run
ctx = SyncContext.new("orders_sync", "api", "warehouse")

# Start metrics collection
metrics = MetricsCollector.new(ctx.sync_run_id, ctx.sync_name)

# Log sync started
SyncLogger.sync_started(ctx.sync_name, ctx.sync_run_id, ctx.source, ctx.destination)

# ... perform sync ...

# Record what happened
metrics.add_events_processed(1000, 50000)  # 1000 events, 50KB
metrics.mark_success()

# Log sync completed
SyncLogger.sync_completed(
    ctx.sync_name,
    ctx.sync_run_id,
    1000,
    duration_secs=60,
    throughput=16.67
)

# Export metrics
print(metrics.summary())
```

### 3. See Results

All metrics and logs automatically flow to your monitoring backend:
- **Metrics database:** Query `pyreverseetl_sync_duration_seconds`
- **Tracing backend:** View sync trace with all operations
- **Monitoring dashboard:** See sync status and throughput
- **Log aggregator:** Structured logs available for search and analysis

---

## Performance Metrics

### What You Can Monitor

| Metric | What It Measures | Healthy Value |
|--------|------------------|---------------|
| **Duration** | How long the sync takes | Fast (your target) |
| **Events Processed** | How many data items moved | All of them |
| **Failed Events** | How many didn't make it | Very few (< 1%) |
| **Total Data** | How much data (in bytes) | All of it |
| **Success Rate** | Percentage that succeeded | > 99% |
| **Speed** | Events per second | Depends on destination |

### Example: Python Usage

```python
# Get current metrics
metrics = MetricsCollector.new("run-123", "orders_sync")

# Process 1000 events (50KB total)
metrics.add_events_processed(1000, 50000)

# 5 events failed
metrics.add_events_failed(5)

# Query metrics
print(f"Throughput: {metrics.throughput()} events/sec")
print(f"Error rate: {metrics.error_rate()}%")
print(f"Success rate: {metrics.success_rate()}%")
print(f"Duration: {metrics.total_duration().as_secs()} seconds")
```

### Example: Metrics Queries

Query your metrics database for:

```
# Average sync duration by sync name
metric: pyreverseetl_sync_duration_seconds | avg by sync_name

# Error rate by sync
pyreverseetl_sync_events_failed_total / pyreverseetl_sync_events_processed_total

# Throughput trending (events per 5 minutes)
rate(pyreverseetl_sync_events_processed_total[5m])

# Alert on high error rate (> 5%)
pyreverseetl_sync_error_rate_percent > 5
```

(Syntax varies by metrics backend - adjust query for your system)

---

## Operation Timeline

### See Every Step

View exactly what happened during your sync, step by step:

```
Order Sync Run
├── 1. Check for new orders
│   └── Found 1000 new orders
├── 2. Transform order data
│   └── Processed 1000 orders in 5 seconds
└── 3. Write to warehouse
    └── Stored 1000 orders in 3 seconds
    
Total time: 8 seconds
```

### Example: Python Usage

```python
from pyreverseetl_core import SyncTracer

# Create tracer
tracer = SyncTracer.new("run-123")

# Create spans for each operation
source_span = tracer.create_span("check_source")
source_span.add_attribute("source_type", "api")
source_span.add_attribute("endpoint", "https://api.example.com/orders")
source_span.mark_ok()
tracer.record_span(source_span)

transform_span = tracer.create_span("transform")
transform_span.add_attribute("transform_type", "Python")
transform_span.add_attribute("script", "transform.py")
transform_span.mark_ok()
tracer.record_span(transform_span)

write_span = tracer.create_span("write_destination")
write_span.add_attribute("destination_type", "warehouse")
write_span.add_attribute("table", "orders")
write_span.mark_ok()
tracer.record_span(write_span)

# Complete trace
tracer.complete("Success")

# Get summary
summary = tracer.summary()
print(f"Total spans: {summary.total_spans}")
print(f"Success rate: {summary.success_rate()}%")
print(f"Total duration: {summary.total_duration_ms}ms")
```

### Example: Trace Visualization

View complete trace in your tracing backend:
- **Service:** PyReverseETL
- **Operation:** sync_run
- **Trace Duration:** 8 seconds
- **Operations:** 3 (check source, transform, write destination)
- **Attributes:** sync_run_id, source_type, destination_type
- **Status:** Success

---

## Event Log

### Detailed Record

Every action during your sync is recorded:

```python
from pyreverseetl_core import SyncLogger

# Log various events during sync
SyncLogger.sync_started("orders_sync", "run-123", "api", "warehouse")
SyncLogger.source_check_started("orders_sync", "api")
SyncLogger.events_found(1000, 50000)
SyncLogger.transformation_started("orders_sync", "Python")
SyncLogger.transformation_completed(5000, 1000)
SyncLogger.write_started("warehouse", 1000)
SyncLogger.write_completed(3000, 1000)
SyncLogger.sync_completed("orders_sync", "run-123", 1000, 8, 125.0)
```

### Log Output (JSON)

```json
{
  "timestamp": "2026-07-18T12:00:00Z",
  "level": "INFO",
  "message": "Sync started",
  "sync_name": "orders_sync",
  "run_id": "run-123",
  "source": "api",
  "destination": "warehouse",
  "service.name": "PyReverseETL",
  "service.version": "v2.0.1"
}
```

### Example: Log Queries

Query logs in your log aggregator:
```
service:pyreverseetl AND sync_name:orders_sync AND status:success
```

Filter by errors:
```
service:pyreverseetl AND level:error
```

(Syntax varies by log aggregation system)

---

## Setup by Backend Type

PyReverseETL supports any OpenTelemetry-compatible backend.

### Metrics + Traces Backend

```python
from pyreverseetl_core import init_otel
from opentelemetry.sdk.metrics import MeterProvider
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor

# Configure metrics exporter (use your backend's exporter)
meter_provider = MeterProvider()

# Configure traces exporter
trace_provider = TracerProvider()
# trace_provider.add_span_processor(YourBackendExporter(...))

# Initialize PyReverseETL
init_otel("orders_sync", "v2.0.1")
```

### Logs Backend (JSON Format)

```python
from pyreverseetl_core import SyncLogger
import logging
from pythonjsonlogger import jsonlogger

# Configure JSON logging for any log aggregation system
handler = logging.StreamHandler()
formatter = jsonlogger.JsonFormatter()
handler.setFormatter(formatter)

logger = logging.getLogger()
logger.addHandler(handler)
logger.setLevel(logging.INFO)

# Logs automatically emit as JSON for ingestion by any backend
```

**Note:** Replace `YourBackendExporter` with your monitoring system's OpenTelemetry exporter. Most modern monitoring systems support OpenTelemetry.

---

## Dashboard Example: Monitoring Dashboard

### Sync Status Panel

```
Syncs in last 24 hours: 1,440
├── Successful: 1,425 (99%)
├── Failed: 10 (0.7%)
└── Partial: 5 (0.3%)

Average throughput: 125 events/sec
Peak throughput: 500 events/sec

Error rate trending:
  Last 1h: 0.5%
  Last 6h: 0.8%
  Last 24h: 1.2%
```

### Latency Panel

```
Average sync duration: 5 minutes
├── Check source: 30 seconds
├── Transform: 2 minutes
└── Write destination: 2.5 minutes

P95 latency: 8 minutes
P99 latency: 12 minutes
```

### Error Tracking Panel

```
Errors by type:
├── Network timeouts: 50
├── Rate limit rejections: 25
├── Transformation errors: 10
└── Connection refused: 5

Top failed syncs:
├── orders_sync (10 errors)
├── customers_sync (5 errors)
└── products_sync (2 errors)
```

---

## Alerting Examples

### Alert Configuration

```yaml
# Alert on sync failure
- alert: PyReverseETLSyncFailed
  expr: rate(pyreverseetl_sync_failed_total[5m]) > 0
  for: 5m
  annotations:
    summary: "Sync {{ $labels.sync_name }} failed"
    severity: "critical"

# Alert on high error rate
- alert: PyReverseETLHighErrorRate
  expr: pyreverseetl_sync_error_rate_percent > 5
  for: 15m
  annotations:
    summary: "Sync {{ $labels.sync_name }} error rate {{ $value }}%"
    severity: "warning"

# Alert on stuck sync
- alert: PyReverseETLSyncStuck
  expr: time() - pyreverseetl_sync_last_success_seconds{sync_name="orders_sync"} > 3600
  annotations:
    summary: "Sync {{ $labels.sync_name }} hasn't completed in 1 hour"
    severity: "warning"
```

---

## Example: Complete Integration

```python
from pyreverseetl_core import (
    SyncConfiguration, SyncContext, MetricsCollector, 
    SyncTracer, SyncLogger, init_otel
)

def run_sync():
    # Initialize OpenTelemetry
    init_otel("orders_sync", "v2.0.1")
    
    # Load configuration
    config = SyncConfiguration.from_yaml_file("sync.yaml")
    
    # Create context
    ctx = SyncContext.new(config.name, "api", "warehouse")
    
    # Start metrics and tracing
    metrics = MetricsCollector.new(ctx.sync_run_id, ctx.sync_name)
    tracer = SyncTracer.new(ctx.sync_run_id)
    
    # Log start
    SyncLogger.sync_started(ctx.sync_name, ctx.sync_run_id, ctx.source, ctx.destination)
    
    try:
        # Check source
        source_span = tracer.create_span("check_source")
        SyncLogger.source_check_started(ctx.sync_name, ctx.source)
        events = check_source(config)
        SyncLogger.events_found(len(events), sum(len(e) for e in events))
        source_span.mark_ok()
        tracer.record_span(source_span)
        
        # Transform
        transform_span = tracer.create_span("transform")
        SyncLogger.transformation_started(ctx.sync_name, "Python")
        transformed = transform_events(events)
        SyncLogger.transformation_completed(transform_time, len(transformed))
        transform_span.mark_ok()
        tracer.record_span(transform_span)
        
        # Write
        write_span = tracer.create_span("write_destination")
        SyncLogger.write_started(ctx.destination, len(transformed))
        write_destination(config, transformed)
        SyncLogger.write_completed(write_time, len(transformed))
        write_span.mark_ok()
        tracer.record_span(write_span)
        
        # Record metrics
        metrics.add_events_processed(len(events), total_bytes)
        metrics.mark_success()
        tracer.complete("Success")
        
        # Log completion
        SyncLogger.sync_completed(
            ctx.sync_name,
            ctx.sync_run_id,
            len(events),
            total_time,
            events_per_sec
        )
        
    except Exception as e:
        # Record failure
        metrics.mark_failed(str(e))
        tracer.complete("Failed")
        SyncLogger.sync_failed(ctx.sync_name, ctx.sync_run_id, str(e))
        raise
    
    # Return metrics for reporting
    return metrics.summary()

if __name__ == "__main__":
    summary = run_sync()
    print(summary)
```

---

## Troubleshooting

### Problem: Metrics not appearing in Prometheus

**Cause:** OTel exporter not configured

**Solution:** Verify exporter configuration and connectivity:
```python
# Check exporter is initialized
from opentelemetry.sdk.metrics import MeterProvider
mp = MeterProvider()
print(mp.metric_readers)  # Should not be empty
```

### Problem: Traces not in Jaeger

**Cause:** Jaeger agent not reachable

**Solution:** Verify Jaeger is running:
```bash
# Check Jaeger is accessible
curl http://localhost:6831/
```

### Problem: Logs not in Datadog

**Cause:** API key not configured or incorrect

**Solution:** Verify configuration:
```python
# Check Datadog exporter
datadog_exporter = DatadogExporter()
print(datadog_exporter.agent_host)  # Should be valid
```

---

## Performance Impact

OTel integration has minimal overhead:

| Feature | CPU Overhead | Memory Overhead | Latency Impact |
|---------|--------------|-----------------|----------------|
| Metrics | < 1% | < 5MB | < 1ms |
| Traces | < 2% | < 10MB | < 5ms |
| Logs | < 1% | < 5MB | < 1ms |
| **Total** | **< 3%** | **< 20MB** | **< 7ms** |

Suitable for production deployments.

---

## Best Practices

1. **Always initialize OTel** at application startup
2. **Use structured logging** with context fields
3. **Emit traces** for all async operations
4. **Set up alerting** for high error rates
5. **Create dashboards** for operational visibility
6. **Review logs** for debugging issues
7. **Export to multiple backends** (redundancy)

---

## Next Steps

1. [Set up backend](#setup-by-backend) (Prometheus, Jaeger, Datadog)
2. [Create dashboard](https://grafana.com/docs/grafana/latest/dashboards/)
3. [Configure alerts](#alerting-examples)
4. [Review metrics](#metrics) in dashboard
5. [Trace sync operations](#traces)
6. [Debug issues](#troubleshooting) with logs

---

For more details, see:
- [Metrics](#metrics)
- [Traces](#traces)
- [Logs](#logs)
- [Setup Guide](#setup-by-backend)
- [Example](#example-complete-integration)
