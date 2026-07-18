# Rate Limiting (Core Feature)

**Prevent overwhelming external systems. Built-in to every destination.**

---

## Why Rate Limiting is Core

External APIs, data warehouses, and SaaS platforms have limits:
- **Salesforce** — 25,000 API calls/24h (developer orgs)
- **Snowflake** — Credits per query, concurrent query limits
- **Braze** — 100 requests/sec, burst capacity
- **HubSpot** — 10 requests/sec (public API)

Without rate limiting, a large sync can:
- ❌ Fail with 429 (Too Many Requests)
- ❌ Trigger security alerts
- ❌ Block future API calls
- ❌ Cost money (Snowflake credits, API rate overage fees)

PyReverseETL's rate limiting prevents this. **It's built-in, always active.**

---

## Rate Limiting Strategies

### 1. Token Bucket (Default)
Allows bursts but maintains steady rate:

```yaml
destination:
  type: salesforce
  rate_limit:
    strategy: token_bucket
    requests_per_interval: 100
    interval: 1 second
    max_burst: 200  # Allow 200 requests at once, refill to 100/sec
```

**Best for:** APIs with burst capacity (most SaaS platforms)

### 2. Leaky Bucket
Smooth, steady rate without bursts:

```yaml
destination:
  type: braze
  rate_limit:
    strategy: leaky_bucket
    requests_per_interval: 50
    interval: 1 second
    cooldown: 20ms  # Wait 20ms between requests
```

**Best for:** Rate-sensitive APIs (strict rate limits)

### 3. Quota-Based
Fixed quota per time window (hourly, daily):

```yaml
destination:
  type: snowflake
  rate_limit:
    strategy: quota
    requests_per_interval: 10000
    interval: 1 hour  # 10k queries per hour
```

**Best for:** Credit-based systems (Snowflake, BigQuery)

---

## Pre-configured Platform Limits

PyReverseETL includes sensible defaults for popular platforms:

```python
from pyreverseetl import RateLimitConfig

# Use platform defaults
salesforce_limit = RateLimitConfig.for_platform("salesforce")
snowflake_limit = RateLimitConfig.for_platform("snowflake")
braze_limit = RateLimitConfig.for_platform("braze")
```

### Default Limits by Platform

| Platform | Strategy | Rate | Burst |
|----------|----------|------|-------|
| **Salesforce** | Token Bucket | 25/sec | 50 |
| **HubSpot** | Token Bucket | 10/sec | 20 |
| **Braze** | Leaky Bucket | 100/sec | None |
| **Snowflake** | Quota | 100/hour | None |
| **BigQuery** | Quota | 100/hour | None |
| **HTTP API** | Token Bucket | 50/sec | 100 |

---

## YAML Configuration

### Simple: Use Defaults
```yaml
name: customer_sync
source:
  type: postgres
  query: SELECT * FROM customers

destination:
  type: salesforce
  # Uses default rate limit: 25 requests/sec, burst to 50
```

### Custom: Override Defaults
```yaml
destination:
  type: salesforce
  rate_limit:
    strategy: token_bucket
    requests_per_interval: 10  # More conservative
    interval: 1 second
    max_burst: 15
```

### Advanced: Adaptive Rate Limiting
```yaml
destination:
  type: snowflake
  rate_limit:
    strategy: quota
    requests_per_interval: 1000
    interval: 1 hour
    adaptive: true  # Reduce rate if errors occur
    cooldown: 5 minutes  # Wait 5 min before retrying
```

When `adaptive: true`:
- Monitors error rate
- Automatically reduces rate after 3+ consecutive errors
- Backs off for cooldown period
- Resumes normal rate when errors clear

---

## Python API

### Creating Rate Limiters

```python
from pyreverseetl import RateLimitConfig, RateLimiter

# Token bucket (bursts allowed)
config = RateLimitConfig.token_bucket(requests_per_sec=100)
limiter = RateLimiter(config)

# Leaky bucket (smooth rate)
config = RateLimitConfig.leaky_bucket(requests_per_sec=50)
limiter = RateLimiter(config)

# Quota (fixed per window)
config = RateLimitConfig.quota(requests_per_hour=10000)
limiter = RateLimiter(config)
```

### Using Rate Limiters

Non-blocking (check if allowed):
```python
if limiter.is_allowed():
    # Send request
    send_to_api(record)
    limiter.record_success()
else:
    limiter.record_error()
```

Blocking (wait for permit):
```python
# Automatically waits until allowed
await limiter.acquire_permit()
send_to_api(record)
limiter.record_success()
```

### Monitoring Rate Limits

```python
stats = limiter.stats()
print(f"Tokens: {stats.tokens_available}")
print(f"Success rate: {stats.success_rate}%")
print(f"Total requests: {stats.total_requests}")

if stats.success_rate < 95:
    logger.warning(f"Rate limit issues: {stats.failed_requests} failures")
```

---

## Per-Destination Limits

Set different limits for different destinations:

```yaml
name: multi_destination_sync
source:
  type: postgres
  query: SELECT * FROM customers

destinations:
  - type: salesforce
    name: crm
    rate_limit:
      strategy: token_bucket
      requests_per_interval: 20  # Conservative for production
      
  - type: braze
    name: marketing
    rate_limit:
      strategy: leaky_bucket
      requests_per_interval: 100  # Higher for Braze
      
  - type: http
    name: webhook
    rate_limit:
      strategy: token_bucket
      requests_per_interval: 50
```

---

## Global Rate Limiting Registry

Coordinate rate limits across multiple syncs:

```python
from pyreverseetl import RateLimiterRegistry

registry = RateLimiterRegistry()

# Share Salesforce limiter across all syncs
salesforce_limiter = registry.get_or_create(
    "salesforce_prod",
    RateLimitConfig.token_bucket(25)
)

# All syncs targeting Salesforce share the same limit
sync1.use_limiter("salesforce_prod", salesforce_limiter)
sync2.use_limiter("salesforce_prod", salesforce_limiter)

# Now both syncs respect the shared 25 req/sec limit
```

This prevents multiple syncs from overwhelming a single destination.

---

## Error Handling with Rate Limits

Rate limits trigger automatic retry:

```yaml
destination:
  type: salesforce
  rate_limit:
    strategy: token_bucket
    requests_per_interval: 25
    
  retry:
    max_attempts: 3
    backoff: exponential
    base_delay: 1 second  # Start at 1s, double each retry
```

When a 429 (Too Many Requests) error occurs:
1. RateLimiter blocks new requests
2. Retry logic waits exponentially
3. After cooldown, resumes normal rate
4. Adaptive mode might reduce rate for future requests

---

## Real-World Scenarios

### Scenario 1: Salesforce Production Sync

```yaml
name: enterprise_customer_sync
source:
  type: postgres
  query: SELECT * FROM enterprise_customers

destination:
  type: salesforce
  rate_limit:
    strategy: token_bucket
    requests_per_interval: 15  # Conservative (Salesforce limit is ~25/sec)
    interval: 1 second
    max_burst: 25
    adaptive: true  # Back off on 429 errors
    cooldown: 5 minutes
```

**Result:** Safely syncs thousands of enterprise customers without hitting Salesforce limits.

### Scenario 2: Multi-Platform Sync

```yaml
name: unified_customer_activation
source:
  type: data_warehouse
  query: SELECT * FROM unified_customers

destinations:
  - type: salesforce
    rate_limit: 20/sec
  - type: hubspot
    rate_limit: 9/sec  # HubSpot is stricter
  - type: braze
    rate_limit: 100/sec  # Braze is more lenient
```

**Result:** Each platform gets requests at its own safe rate.

### Scenario 3: Hourly Quota Sync

```yaml
name: snowflake_data_load
source:
  type: postgres
  query: SELECT * FROM large_table

destination:
  type: snowflake
  rate_limit:
    strategy: quota
    requests_per_interval: 50
    interval: 1 hour  # 50 queries/hour quota
    adaptive: true
```

**Result:** Never exceeds hourly credit limits, adapts if queries run slow.

---

## Monitoring & Alerts

### Track Rate Limiting Impact

```python
from pyreverseetl import Activation

activation = Activation.from_yaml("sync.yaml")
result = activation.execute()

# Get rate limiting stats
for dest_name, limiter_stats in result.rate_limits.items():
    print(f"{dest_name}:")
    print(f"  Throughput: {limiter_stats.tokens_available} tokens")
    print(f"  Success rate: {limiter_stats.success_rate}%")
    print(f"  Failures: {limiter_stats.failed_requests}")
    
    if limiter_stats.success_rate < 99:
        logger.warning(f"Rate limit issues on {dest_name}")
```

### OpenTelemetry Metrics

```bash
# Prometheus metrics
curl http://localhost:8080/metrics | grep rate_limit

pyreverseetl_rate_limit_hit_total{destination="salesforce"} 3
pyreverseetl_rate_limit_backoff_total_seconds{destination="braze"} 45.2
pyreverseetl_rate_limit_success_rate{destination="snowflake"} 0.99
```

---

## Best Practices

1. **Always use defaults first** — Let PyReverseETL set sensible limits
2. **Go conservative** — Start low, increase if needed
3. **Monitor success rates** — Alert if < 98%
4. **Enable adaptive mode** — For production syncs
5. **Share limiters** — Use registry for multi-sync coordination
6. **Test with real data** — Load testing helps tune limits
7. **Document platform limits** — Know what each API supports

---

## FAQ

**Q: Won't rate limiting slow down my syncs?**
A: Yes, but without it, syncs fail completely. Rate limiting trades speed for reliability.

**Q: How do I know what rate limit to use?**
A: Start with platform defaults, monitor success rate, adjust if >99% successful.

**Q: Can I disable rate limiting?**
A: Not recommended, but `rate_limit: disabled` bypasses it (risky).

**Q: What if I have multiple syncs to the same destination?**
A: Use the RateLimiterRegistry to share limits across syncs.

**Q: How adaptive mode works?**
A: Monitors error rate. If 3+ consecutive failures, backs off for cooldown. Resumes when errors clear.

---

## Summary

Rate limiting is **core to PyReverseETL**:

- ✅ **Built-in** — Every destination has rate limiting
- ✅ **Safe defaults** — Pre-configured for 20+ platforms
- ✅ **Flexible** — Token bucket, leaky bucket, quota modes
- ✅ **Adaptive** — Automatically backs off on errors
- ✅ **Observable** — Full metrics and monitoring
- ✅ **Coordinated** — Share limits across multiple syncs

Your data reaches its destination safely, at the right pace.

---

**Next:** [Connector Ecosystem](CONNECTOR_ECOSYSTEM.md) | [Observability](OBSERVABILITY.md)
