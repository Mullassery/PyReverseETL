# Exactly-Once Semantics - PyReverseETL v2.0.1

**The Most Important Guarantee: Every event is delivered exactly once, no duplicates, no loss.**

---

## What Exactly-Once Means

```
Source Event → [pipeline] → Destination Result

Guarantees:
✅ Delivered: Event makes it to destination
✅ No duplicates: Event appears exactly once (not 2x, 3x, 10x)
✅ No loss: Event never disappears
✅ Order preserved: Events in source order (if needed)
```

**Why it matters:**
- Financial transactions: $100 charge must happen exactly once (not twice!)
- Customer records: "John" appears once (not duplicated 3 times)
- Event counts: "1000 events processed" is accurate (not "3000" from duplicates)
- Data integrity: Warehouse numbers match source numbers

---

## How PyReverseETL Achieves Exactly-Once

### Layer 1: Source Polling Idempotency
Source polls and remembers what was already seen:

```yaml
source_polling:
  frequency: Hourly

# System tracks:
# - last_poll_at: When was last poll?
# - last_change_at: When was last change detected?
# - poll_count: How many polls executed?
# - change_count: How many detected changes?
```

**Mechanism:**
1. Poll source at configured interval
2. Ask: "What changed since last poll?"
3. Mark timestamp of this poll
4. Next poll only retrieves changes after that timestamp
5. Duplicate prevention: Timestamps ensure no re-polling

### Layer 2: Kafka Offset Management
Kafka tracks consumer position automatically:

```yaml
transformation:
  intermediate_topic: staging

# Kafka guarantees:
# - Consumer offset: "I've read up to message 1000"
# - Only messages > 1000 are consumed next
# - If consumer crashes, resumes from offset 1000
# - No replaying, no duplicates
```

**Mechanism:**
1. Consumer reads from partition (messages 0-100)
2. Kafka server commits offset = 100
3. Process messages
4. If crash, restart from offset 100
5. No re-processing, no duplicates

### Layer 3: Idempotent Destination Writes
Destination handles duplicate messages gracefully:

```python
# Destination must be idempotent:
# - Primary key prevents duplicates
# - Update vs Insert logic handles replays
# - Unique constraints prevent doubles

# Example: User record
PUT /users/john-123
{
  "id": "john-123",        # Unique ID
  "email": "john@example", # Unique email
  "name": "John",
  "updated_at": "2026-07-18T12:00:00Z"
}

# Duplicate delivery:
# Send 1: Creates user
# Send 2: Updates same user (idempotent!)
# Result: ONE user record, not two
```

### Layer 4: Transaction Boundaries
Transform execution is atomic:

```yaml
transformation:
  enabled: true
  script_path: transform.py
  timeout_secs: 300
  max_retries: 5

# Transaction guarantee:
# - Transform starts
# - ALL transformations complete
# - ALL results written to staging topic
# - OR all rolled back (if failure)
# - No partial results
```

**Mechanism:**
```
Transform: event_1 → output_1
           event_2 → output_2
           event_3 → output_3

Success: Write ALL 3 outputs to Kafka OR
Failure: Write NOTHING (atomic rollback)

No possibility of: 1.5 events delivered
```

### Layer 5: Dead Letter & Retry
Failed events are trapped, not lost:

```yaml
transformation:
  dead_letter_topic: transform_errors
  max_retries: 5
  retry_delay_secs: 10

# Failure handling:
# - Event fails transformation
# - Sent to dead_letter_topic (captured!)
# - Retried 5 times with backoff
# - If still fails, captured for investigation
# - Zero chance of silent loss
```

**Mechanism:**
```
Event fails 5 retries
    ↓
Sent to dead_letter_topic
    ↓
Alert ops team
    ↓
Investigate & fix
    ↓
Replay from dead letter
    ↓
Event delivered successfully (exactly once!)
```

---

## Exactly-Once in Practice

### Example 1: Financial Transaction
```
Source: "Charge $100 to John"
↓
Poll detects new charge
↓
Transform: Verify balance → Apply fee
↓
Staging: Intermediate storage (Kafka)
↓
Destination: Update accounting database
  - Check: "Is this charge already recorded?"
  - No duplicate exists (unique tx_id)
  - Record once
  - Return success
↓
Result: Exactly $100 charged, not $200, not $0
```

### Example 2: Customer Record Sync
```
Source: CRM updated John's email
↓
Poll detects change
↓
Transform: Format for data warehouse
↓
Staging: Intermediate storage (Kafka)
↓
Destination: Update warehouse
  - Check: "Do we have this customer?"
  - Yes (id = john-123)
  - Update (not duplicate insert)
  - Return success
↓
Result: One John record with new email, not three Johns
```

### Example 3: Handling Failures
```
Source: 1000 events ready
↓
Pull into staging (Kafka)
   ✓ Events 1-500 succeed
   ✗ Event 501 fails (destination rate limit)
   ✓ Events 502-1000 pause
↓
Retry logic:
   - Event 501: Retry with exponential backoff
   - Attempt 1: Fails (rate limited)
   - Attempt 2: 10s later, fails
   - Attempt 3: 20s later, succeeds ✓
   - Events 502-1000: Continue
↓
Result: All 1000 events delivered exactly once (no gaps, no duplicates)
```

---

## Configuration for Exactly-Once

### Polling Configuration
```yaml
source_polling:
  frequency: Hourly          # Regular polling prevents loss
  enabled: true              # Must be enabled
  skip_days: []              # No gaps

  # Timezone matters! Must be consistent
  timezone: UTC              # Track time accurately
```

### Transformation Configuration
```yaml
transformation:
  enabled: true
  intermediate_topic: staging  # Kafka maintains exact order
  max_retries: 5              # Retry on failure
  retry_delay_secs: 10        # Exponential backoff
  timeout_secs: 300           # Don't hang forever
  
  # Dead letter for failures
  dead_letter_topic: errors
  skip_on_error: false        # Don't silently skip
  
  # Cache for recovery
  enable_caching: true
  cache_dir: /var/cache/transforms
```

### Destination Configuration
```yaml
destination_polling:
  frequency: Daily            # Consume reliably
  enabled: true
  
  # Idempotent destination:
  # - Primary key (unique ID per record)
  # - Upsert (update if exists, insert if new)
  # - Unique constraints
  # - Duplicate detection
```

---

## Verification: Exactly-Once Delivery

### Check 1: All Events Arrived
```sql
-- Source count
SELECT COUNT(*) FROM source_db.events;
-- Result: 1,000,000

-- Destination count
SELECT COUNT(*) FROM warehouse.events;
-- Result: 1,000,000 ✅ (match!)
```

### Check 2: No Duplicates
```sql
-- Check for duplicate IDs
SELECT event_id, COUNT(*) as cnt
FROM warehouse.events
GROUP BY event_id
HAVING cnt > 1;
-- Result: (empty) ✅ No duplicates!
```

### Check 3: No Gaps
```sql
-- Check for sequential IDs
SELECT COUNT(*) as delivered,
       MAX(event_id) - MIN(event_id) + 1 as expected
FROM warehouse.events
WHERE status = 'delivered';

-- delivered = expected ✅ No gaps!
```

### Check 4: Failed Events Captured
```sql
-- Dead letter topic should have any failures
SELECT COUNT(*) FROM kafka.topic('transform_errors');
-- Result: 0 (or small number with specific errors to fix)
```

---

## When Exactly-Once Might Fail

### Scenario 1: Idempotent Key Missing
```sql
-- Bad: No unique constraint
CREATE TABLE users (
  name TEXT,
  email TEXT
);
-- Duplicate key error risk!

-- Good: Idempotent
CREATE TABLE users (
  id UUID PRIMARY KEY,  -- ← Unique!
  name TEXT,
  email TEXT UNIQUE     -- ← Unique!
);
-- Safely handles duplicates
```

**Fix:** Ensure destination has idempotent keys.

### Scenario 2: Polling Too Infrequent
```yaml
source_polling:
  frequency: Weekly  # ← TOO INFREQUENT!

# Risk: Changes missed between polls
# Fix: Increase polling frequency
source_polling:
  frequency: Hourly  # ← Better
```

### Scenario 3: No Dead Letter Topic
```yaml
transformation:
  dead_letter_topic: null  # ← Missing!

# Risk: Failed events lost silently
# Fix: Add dead letter topic
transformation:
  dead_letter_topic: errors  # ← Captures failures
```

---

## Exactly-Once Across Failure Scenarios

### Network Failure
```
Source → [network down] → Kafka

Before: Retried 5x, backed off, sent to dead letter
After: Network restored, replayed from dead letter
Result: ✅ Exactly once (no duplicates despite retries)
```

### Destination Crash
```
Kafka → [destination crashes] → Database

Before: Last event written at offset 500
After: Destination restarts, resumes from offset 500
Result: ✅ Exactly once (no re-processing of 1-499)
```

### Transformation Failure
```
[Transformation error] → dead_letter_topic

Later: Fix bug, replay from dead letter
Result: ✅ Exactly once (proper dedup on replay)
```

### Duplicate Message (Multiple Sends)
```
Webhook received: {"user_id": "123", "action": "update"}
Kafka offset: 1000

Message accidentally replayed:
Kafka offset: 1001 (same message)

Destination receives twice:
- Attempt 1: Updates record for user 123 (offset 1000)
- Attempt 2: Updates same record (offset 1001)

Database PRIMARY KEY prevents duplicate:
  UPDATE users SET ... WHERE id = 123
  
Result: ✅ Exactly once (idempotent key)
```

---

## Best Practices for Exactly-Once

1. **Use Idempotent IDs**
   ```yaml
   # Every record must have a unique ID
   # That ID survives replays
   ```

2. **Track Polling State**
   ```yaml
   source_polling:
     # System automatically tracks:
     # last_poll_at, last_change_at
   ```

3. **Use Staging Topics**
   ```yaml
   transformation:
     intermediate_topic: staging
     # Kafka maintains order and offset
   ```

4. **Configure Dead Letters**
   ```yaml
   transformation:
     dead_letter_topic: errors
     # Captures all failures
   ```

5. **Enable Caching**
   ```yaml
   transformation:
     enable_caching: true
     cache_dir: /var/cache
     # Recovery from local copy
   ```

6. **Monitor Offsets**
   ```
   Consumer lag: Should be ~0 (caught up)
   Dead letter count: Should be < 1% of throughput
   Cache size: Should be stable (not growing)
   ```

7. **Test Failure Recovery**
   ```
   Simulate:
   - Network partition
   - Destination crash
   - Transformation error
   
   Verify:
   - All events still delivered
   - No duplicates
   - All recovered from dead letter
   ```

---

## Exactly-Once Checklist

- [ ] Source polls at regular intervals
- [ ] Kafka intermediate topic configured
- [ ] Dead letter topic configured
- [ ] Destination has idempotent keys (PRIMARY KEY)
- [ ] Destination has unique constraints
- [ ] Caching enabled for recovery
- [ ] Max retries configured (3-5)
- [ ] Dead letter monitored
- [ ] Cache size monitored
- [ ] Consumer lag monitored

---

## Summary

| Aspect | Guarantee |
|--------|-----------|
| **Delivery** | Every event reaches destination ✅ |
| **Duplicates** | No duplicates (idempotent) ✅ |
| **Loss** | No data loss (dead letter tracks failures) ✅ |
| **Order** | Preserved (Kafka offset order) ✅ |
| **Failures** | Handled gracefully (retries + dead letter) ✅ |
| **Recovery** | Replay from cache or dead letter ✅ |

**Result:** Exactly-once delivery guarantee for production pipelines.

See [SYNC_CONFIGURATION.md](SYNC_CONFIGURATION.md) and [BACKPRESSURE_AND_BUFFERING.md](BACKPRESSURE_AND_BUFFERING.md) for configuration details.
