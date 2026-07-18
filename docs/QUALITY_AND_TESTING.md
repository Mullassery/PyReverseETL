# PyReverseETL Quality & Testing

**Ensure data quality at every step of your sync pipeline.**

---

## Data Quality Testing

PyReverseETL supports built-in data quality testing alongside transformations. Every sync can validate data automatically.

### YAML-Based Quality Tests

Define quality checks directly in your configuration:

```yaml
name: customer_sync
source:
  type: postgres
  query: SELECT * FROM customers
  
destination:
  type: snowflake
  table: analytics.customers

# Data quality tests
quality_checks:
  - name: no_nulls_in_id
    type: column
    column: customer_id
    check: NOT NULL
    fail_action: skip_record
    
  - name: valid_email_format
    type: column
    column: email
    check: regex
    pattern: "^[\\w\\.-]+@[\\w\\.-]+\\.\\w+$"
    fail_action: mark_invalid
    
  - name: age_within_bounds
    type: column
    column: age
    check: between
    min: 18
    max: 150
    fail_action: skip_record
    
  - name: duplicate_check
    type: uniqueness
    column: customer_id
    fail_action: error_and_stop
    
  - name: record_count_threshold
    type: row_count
    min_records: 100
    fail_action: error_and_stop
```

### Python Script Quality Tests

Use Python scripts for complex data quality logic:

```yaml
name: customer_sync
source:
  type: postgres
  query: SELECT * FROM customers

destination:
  type: snowflake
  table: analytics.customers

# Python-based quality testing
quality_script: tests/validate_customers.py
```

**validate_customers.py:**

```python
from pyreverseetl import QualityTest, TestResult

class CustomerQualityTests(QualityTest):
    """Custom quality tests for customer data"""
    
    def test_email_validity(self, record):
        """Validate email format and domain"""
        email = record.get('email', '')
        if '@' not in email or '.' not in email.split('@')[1]:
            return TestResult.fail(f"Invalid email: {email}")
        return TestResult.pass_test()
    
    def test_ltv_sanity(self, record):
        """Validate lifetime value makes sense"""
        ltv = record.get('lifetime_value', 0)
        if ltv < 0:
            return TestResult.fail(f"Negative LTV: {ltv}")
        if ltv > 1_000_000_000:
            return TestResult.fail(f"Unrealistic LTV: {ltv}")
        return TestResult.pass_test()
    
    def test_phone_format(self, record):
        """Validate phone number format"""
        phone = record.get('phone', '')
        if phone and not phone.replace('-', '').replace(' ', '').isdigit():
            return TestResult.fail(f"Invalid phone format: {phone}")
        return TestResult.pass_test()
    
    def test_aggregate_stats(self, records):
        """Validate aggregate statistics"""
        if not records:
            return TestResult.fail("No records to validate")
        
        ltv_values = [r.get('lifetime_value', 0) for r in records]
        avg_ltv = sum(ltv_values) / len(ltv_values)
        
        # Flag if average LTV changed significantly
        if avg_ltv == 0:
            return TestResult.warn("Average LTV is zero")
        
        return TestResult.pass_test()
```

### Quality Test Actions

When a quality check fails, PyReverseETL takes the specified action:

- **skip_record** — Skip this record, continue with others
- **mark_invalid** — Include record with invalid flag, continue
- **warn** — Log warning, continue
- **error_and_stop** — Fail the entire sync

### Quality Test Reporting

Get detailed quality reports:

```python
from pyreverseetl import Activation

activation = Activation.from_yaml("sync.yaml")
run = activation.execute()

# Quality summary
print(f"Records processed: {run.records_processed}")
print(f"Records passed: {run.quality.passed}")
print(f"Records failed: {run.quality.failed}")
print(f"Records skipped: {run.quality.skipped}")

# Detailed failures
for failure in run.quality.failures:
    print(f"  {failure.record_id}: {failure.check_name} - {failure.reason}")
```

---

## StatGuardian Integration

Use **StatGuardian** to add enterprise-grade data quality to your syncs.

### What StatGuardian Adds

StatGuardian provides:
- **Schema contracts** — Define expected data shape
- **Anomaly detection** — Flag unexpected data patterns
- **Data drift detection** — Track value distribution changes
- **Statistical validation** — Ensure data sanity
- **Lineage tracking** — Know where data came from
- **Quality scoring** — Rate data quality over time

### Enable StatGuardian

```yaml
name: customer_sync
source:
  type: postgres
  query: SELECT * FROM customers

destination:
  type: snowflake
  table: analytics.customers

# Enable StatGuardian integration
statguardian:
  enabled: true
  contract: contracts/customer_contract.yaml
  detect_anomalies: true
  detect_drift: true
```

### StatGuardian Contract

**contracts/customer_contract.yaml:**

```yaml
# Data contract - defines expected structure and quality
contract:
  name: customer_schema
  version: "1.0"
  
  # Expected columns and types
  schema:
    customer_id:
      type: integer
      required: true
      unique: true
      
    email:
      type: string
      required: true
      pattern: "^[\\w\\.-]+@[\\w\\.-]+\\.\\w+$"
      
    lifetime_value:
      type: decimal
      required: false
      min: 0
      max: 1000000
      
    segment:
      type: string
      required: false
      allowed_values: [vip, premium, standard, inactive]
  
  # Anomaly detection thresholds
  anomaly_detection:
    - field: lifetime_value
      method: zscore
      threshold: 3.0  # Flag values > 3 standard deviations
      
    - field: segment
      method: frequency
      threshold: 0.1  # Flag if any value drops below 10% frequency
  
  # Drift detection (track distribution changes)
  drift_detection:
    enabled: true
    fields: [segment, lifetime_value]
    frequency: hourly
```

### Quality Reports with StatGuardian

StatGuardian enhances your reports:

```python
run = activation.execute()

# Get StatGuardian report
if run.statguardian:
    print(f"Quality score: {run.statguardian.quality_score}%")
    print(f"Schema violations: {len(run.statguardian.schema_violations)}")
    print(f"Anomalies detected: {len(run.statguardian.anomalies)}")
    print(f"Drift detected: {run.statguardian.drift_detected}")
    
    for violation in run.statguardian.schema_violations:
        print(f"  {violation.field}: {violation.issue}")
    
    for anomaly in run.statguardian.anomalies:
        print(f"  ANOMALY in {anomaly.field}: {anomaly.description}")
```

### StatGuardian + PyReverseETL Workflow

Typical flow:

1. **Source data** — Extract from database
2. **Quality checks** — PyReverseETL basic validation
3. **StatGuardian validation** — Deep quality analysis
4. **Transform** — Apply transformations (if data passes)
5. **Final validation** — Post-transform quality check
6. **Destination** — Write to target system
7. **Report** — Combined quality + StatGuardian report

```yaml
name: secure_customer_sync
source:
  type: postgres
  query: SELECT * FROM customers

# Stage 1: Basic quality
quality_checks:
  - name: no_nulls_in_id
    type: column
    column: customer_id
    check: NOT NULL
    fail_action: error_and_stop

# Stage 2: Deep quality with StatGuardian
statguardian:
  enabled: true
  contract: contracts/customer_contract.yaml
  detect_anomalies: true
  on_anomaly: warn  # Continue but flag issues

# Stage 3: Transform
transform:
  script: transforms/normalize_customer.py
  
# Stage 4: Post-transform validation
post_transform_checks:
  - name: verify_transformation
    type: custom
    script: tests/verify_transform.py

destination:
  type: snowflake
  table: analytics.customers
```

### Monitoring Data Health

StatGuardian integration enables continuous monitoring:

```python
from pyreverseetl import SyncSchedule

schedule = SyncSchedule.from_yaml("sync.yaml")

# Get historical quality metrics
history = schedule.quality_history(days=30)
for day_metrics in history:
    print(f"{day_metrics.date}: Quality {day_metrics.statguardian.quality_score}%")
    if day_metrics.statguardian.anomalies:
        print(f"  ⚠️  Anomalies detected: {len(day_metrics.statguardian.anomalies)}")
```

---

## Combined Testing Example

Full pipeline with both YAML tests and StatGuardian:

```yaml
name: enterprise_customer_sync
owner: data_team

source:
  type: postgres
  host: ${DB_HOST}
  database: core
  query: SELECT * FROM customers WHERE updated_at > :last_sync

destination:
  type: snowflake
  warehouse: compute
  table: analytics.customers

# Level 1: Fast YAML-based checks
quality_checks:
  - name: id_not_null
    type: column
    column: customer_id
    check: NOT NULL
    fail_action: error_and_stop

# Level 2: Deep quality with StatGuardian
statguardian:
  enabled: true
  contract: contracts/customer_v2.yaml
  detect_anomalies: true
  on_anomaly: warn
  on_drift: warn

# Level 3: Custom Python validation
quality_script: tests/customer_quality.py

# Schedule with proper filtering
schedule:
  frequency: hourly
  timezone: America/New_York
  skip_days: [Saturday, Sunday]
  no_sync_hours:
    start: "22:00"
    end: "08:00"

# Success criteria
acceptance:
  min_records: 10
  max_error_rate: 0.01  # 1% tolerance
  statguardian_quality_minimum: 0.95  # 95% quality score
```

---

## Performance Considerations

- **YAML checks** — Fastest, good for simple validation
- **Python scripts** — Slower but support complex logic
- **StatGuardian** — Moderate overhead, adds deep analysis
- **Aggregate tests** — Run once per batch, not per record

For high-volume syncs, place expensive checks after filtering.

---

## Summary

PyReverseETL provides multi-level data quality:

1. **YAML-based tests** — Quick, simple validation
2. **Python scripts** — Complex custom logic
3. **StatGuardian integration** — Enterprise data contracts
4. **Combined reports** — See full quality picture

Enable what you need. Skip what you don't.
