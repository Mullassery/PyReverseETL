# Multi-Destination Activation

**Sync the same records to multiple destinations with independent rate limits and transformations.**

---

## The Problem

When activating customer data to multiple platforms:

```
Source Data
    ↓
    ├→ Salesforce (25 req/sec, needs name mapping)
    ├→ HubSpot (10 req/sec, needs email lowercased)  
    ├→ Braze (100 req/sec, needs timestamp format)
    └→ Snowflake (50 queries/hr, needs column renames)
```

Each destination needs:
- **Different rate limits** (Salesforce: 25/sec, HubSpot: 10/sec)
- **Different transformations** (field mappings, format changes)
- **Different error handling** (some can be skipped, others must succeed)
- **Different scheduling** (some hourly, others real-time)

Without proper handling, **one slow destination blocks all others**.

---

## Solution: Event Replay Architecture

PyReverseETL implements event replay:

```
Source reads records → Stores in event queue
                       ↓
                  Event 1 → Salesforce (rate: 25/sec)
                  Event 1 → HubSpot (rate: 10/sec)
                  Event 1 → Braze (rate: 100/sec)
                  Event 1 → Snowflake (rate: 50/hr)
```

Each destination:
- **Consumes independently** — No blocking
- **Applies own transformations** — Different mappings per destination
- **Respects own rate limit** — Different throughput per system
- **Retries independently** — Failed HubSpot sync doesn't affect Salesforce

---

## Configuration

### Simple: Multiple Destinations

```yaml
name: omnichannel_customer_activation
source:
  type: postgres
  query: SELECT * FROM customers

destinations:
  - name: crm
    type: salesforce
    rate_limit: 25/sec
    fields:
      customer_id: Id
      email: Email
      first_name: FirstName__c
      
  - name: marketing
    type: braze
    rate_limit: 100/sec
    fields:
      customer_id: external_id
      email: email_address
      segment: user_segment
      
  - name: messaging
    type: twilio
    rate_limit: 50/sec
    fields:
      phone: phone_number
      country_code: country
      
  - name: warehouse
    type: snowflake
    rate_limit: 50/hour
    table: analytics.customers
```

**Result:**
- Source reads once → Event queue
- Each destination pulls at its own pace
- Salesforce: 25 records/sec
- Braze: 100 records/sec (faster!)
- Twilio: 50 records/sec
- Snowflake: ~1 record/min (50/hour)

No blocking. No slow destination affecting others.

---

## Advanced: Per-Destination Configuration

Each destination can have unique settings:

```yaml
name: enterprise_activation
source:
  type: postgres
  query: SELECT * FROM enterprise_customers

destinations:
  # Mission-critical: Salesforce
  - name: salesforce_crm
    type: salesforce
    rate_limit:
      strategy: token_bucket
      requests_per_interval: 20
      max_burst: 30
    retry:
      max_attempts: 5  # More aggressive retry
      backoff: exponential
    on_error: fail  # Stop if Salesforce fails
    
  # Important but not critical: HubSpot
  - name: hubspot_crm
    type: hubspot
    rate_limit: 9/sec
    retry:
      max_attempts: 3
    on_error: warn  # Log but continue
    transform:
      script: transforms/hubspot_format.py
      
  # Analytics: Snowflake
  - name: snowflake_analytics
    type: snowflake
    rate_limit: 50/hour
    on_error: dead_letter_queue  # Send failures to DLQ
    
  # Real-time messaging: Twilio
  - name: twilio_sms
    type: http
    endpoint: https://api.twilio.com/2010-04-01/Accounts/...
    rate_limit: 10/sec  # Twilio SMS is rate-limited
    on_error: skip  # Skip if SMS send fails
```

---

## Per-Destination Transformations

Each destination gets records formatted differently:

```yaml
destinations:
  - name: salesforce
    type: salesforce
    # Salesforce needs: account.Id, account.Name, etc.
    transform:
      - field: customer_id
        as: Id
      - field: company_name
        as: Name
      - field: segment
        allowed_values: [Gold, Silver, Bronze]
        
  - name: braze
    type: braze
    # Braze needs: external_id, custom_attributes
    transform:
      - field: customer_id
        as: external_id
      - field: phone
        transform: format_phone  # Custom formatter
      - field: lifetime_value
        transform: convert_to_float
        
  - name: elasticsearch
    type: elasticsearch
    # ES needs: lowercase fields, no spaces
    transform:
      - field: first_name
        transform: lowercase
      - field: last_name
        transform: lowercase
      - field: email
        transform: lowercase
```

---

## Independent Rate Limiting Example

Same 1000 customer records to 4 destinations:

```
Destination    | Rate Limit | Throughput | Time to Complete
Salesforce     | 25/sec     | 25 rec/sec | 40 seconds
HubSpot        | 10/sec     | 10 rec/sec | 100 seconds  ← Slowest
Braze          | 100/sec    | 100 rec/sec| 10 seconds
Snowflake      | 50/hour    | 0.83/sec   | 1200 seconds ← Quota limited
```

**Without event queue:**
- Can't start Braze until Snowflake finishes
- Would take 1200+ seconds total
- Resources wasted waiting

**With event queue (PyReverseETL):**
- Snowflake takes 1200s, others finish earlier
- Braze finishes in 10s (stays idle waiting for next batch)
- Salesforce finishes in 40s
- HubSpot finishes in 100s
- Total time: 1200s (determined by slowest destination)
- But each destination runs independently!

---

## Error Handling Strategies

Different destinations require different error policies:

```yaml
destinations:
  - name: salesforce
    type: salesforce
    on_error: fail  # Fail entire sync if Salesforce fails
    
  - name: hubspot
    type: hubspot
    on_error: warn  # Log warning but continue
    
  - name: analytics
    type: snowflake
    on_error: dead_letter_queue  # Send failures to DLQ for later processing
    dlq_topic: failed_analytics_records
    
  - name: webhook
    type: http
    on_error: skip  # Skip if webhook fails
```

Combined error policy:
- If Salesforce fails: entire sync fails (critical)
- If HubSpot fails: warning logged, sync continues
- If Snowflake fails: record goes to DLQ
- If webhook fails: record skipped, continue

---

## Real-Time Streaming Example

Process events as they arrive, fan-out to multiple destinations:

```yaml
name: realtime_customer_events
source:
  type: kafka
  topic: customer_events
  group_id: multichannel_activator

destinations:
  # Immediate action: Block fraudulent signups
  - name: fraud_check
    type: http
    endpoint: https://fraud.api.com/check
    rate_limit: 100/sec
    on_error: fail  # Must not block fraudsters
    
  # Real-time engagement: Personalized message
  - name: engage
    type: braze
    rate_limit: 100/sec
    on_error: warn
    
  # Analytics: Store for later analysis
  - name: lake
    type: s3
    rate_limit: 50/sec
    on_error: dlq
    
  # Archive: Long-term storage
  - name: archive
    type: snowflake
    rate_limit: 10/sec
    on_error: dlq
```

Each event flows through all channels independently.

---

## Monitoring Multi-Destination Syncs

Track each destination separately:

```python
from pyreverseetl import Activation

activation = Activation.from_yaml("multi_dest.yaml")
result = activation.execute()

# Get per-destination stats
for dest_name, dest_result in result.destinations.items():
    print(f"\n{dest_name}:")
    print(f"  Records sent: {dest_result.records_sent}")
    print(f"  Errors: {dest_result.errors}")
    print(f"  Success rate: {dest_result.success_rate}%")
    print(f"  Avg latency: {dest_result.avg_latency_ms}ms")
    print(f"  Duration: {dest_result.duration_seconds}s")

# Get queue stats
print(f"\nEvent queue:")
print(f"  Total events: {result.queue_stats.total_events}")
print(f"  Pending: {result.queue_stats.pending_events}")
print(f"  Processed: {result.queue_stats.processed_events}")
```

Output:
```
salesforce:
  Records sent: 980
  Errors: 20
  Success rate: 98.0%
  Avg latency: 45ms
  Duration: 40s

hubspot:
  Records sent: 995
  Errors: 5
  Success rate: 99.5%
  Avg latency: 102ms
  Duration: 100s

braze:
  Records sent: 1000
  Errors: 0
  Success rate: 100.0%
  Avg latency: 10ms
  Duration: 10s

snowflake:
  Records sent: 985
  Errors: 15
  Success rate: 98.5%
  Avg latency: 1200ms
  Duration: 1200s

Event queue:
  Total events: 1000
  Pending: 0
  Processed: 1000
```

---

## Performance Considerations

### Bottleneck Analysis

```
Throughput limited by:
- Source read speed (typically fast with good DB)
- Slowest destination (Snowflake @ 50/hour in example)
- Total sync time determined by slowest path
```

Strategies to improve:
1. **Batch slow destinations** — Queue writes to batch them (Snowflake)
2. **Parallelize** — Run multiple instances of PyReverseETL
3. **Partition data** — Split by time window or region
4. **Async retries** — Don't retry synchronously

---

## Real-World Scenarios

### Scenario 1: Customer Onboarding
```
New customer signup
  ├→ Salesforce (CRM)     Rate: 25/sec
  ├→ HubSpot (Mktg)       Rate: 10/sec
  ├→ Stripe (Billing)     Rate: 100/sec
  └→ DynamoDB (Profile)   Rate: 1000/sec
```

Each system gets the record at its safe rate.

### Scenario 2: Fraud Detection
```
Suspicious transaction detected
  ├→ Block service         on_error: fail (critical)
  ├→ Alert team            on_error: warn
  ├→ Log to warehouse      on_error: dlq (retry later)
  └→ Archive              on_error: skip
```

Fraud blocking is priority-1, others are best-effort.

### Scenario 3: Daily Analytics
```
1M customer records to warehouse
  ├→ Snowflake (warehouse) Rate: 100/hr quota
  ├→ S3 (lake)            Rate: 50/sec
  ├→ Elasticsearch (search) Rate: 100/sec
  └→ Cache (Redis)        Rate: 1000/sec
```

Redis finishes in seconds, warehouse takes hours. No blocking.

---

## Summary

Multi-destination activation with PyReverseETL:

✅ **Independent rate limits** — Each destination has its own pace
✅ **No blocking** — Slow destinations don't affect fast ones  
✅ **Per-destination transforms** — Different formats for each system
✅ **Flexible error handling** — Critical vs best-effort
✅ **Event queue** — Records stored, replayed to each destination
✅ **Observable** — Monitor each destination separately

One sync, multiple destinations, coordinated activation.

---

**Next:** [Rate Limiting](RATE_LIMITING.md) | [Connector Ecosystem](CONNECTOR_ECOSYSTEM.md)
