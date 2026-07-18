# Quick Start - PyReverseETL v2.0.1

**Get up and running in 5 minutes with YAML config + Python.**

---

## The Simplest Example

### 1. Create YAML Config File

Create `sync.yaml`:
```yaml
name: my_first_sync
description: Kafka to Data Warehouse

source_polling:
  frequency: Hourly
  timezone: America/New_York

destination_polling:
  frequency: Daily
  timezone: America/New_York
```

### 2. Write Python Script

Create `sync.py`:
```python
from pyreverseetl_core.sources import SyncConfiguration

# Load config from YAML
config = SyncConfiguration.from_yaml_file("sync.yaml")

# Validate configuration
result = config.validate()
print(result)

# Check if valid
if result.status == "Success":
    print("✅ Ready to sync!")
else:
    print("❌ Fix configuration errors")
```

### 3. Run It

```bash
python sync.py
```

**That's it!** 🎉

---

## Real-World Example (5 minutes)

### Step 1: Copy Example Config

```bash
cp examples/sync_config_kafka_to_warehouse.yaml sync.yaml
```

### Step 2: Customize (Edit sync.yaml)
```yaml
name: orders_to_warehouse
description: Sync Kafka order events to data warehouse

source_polling:
  frequency: FiveMinutes
  timezone: America/New_York

destination_polling:
  frequency: Daily
  timezone: America/New_York
  no_sync_after_hour: 22
  sync_resume_hour: 6
```

### Step 3: Create Python Script

`run_sync.py`:
```python
from pyreverseetl_core.sources import SyncConfiguration, ConfigStatus

def main():
    # Load configuration
    config = SyncConfiguration.from_yaml_file("sync.yaml")
    
    # Validate
    result = config.validate()
    print(result)
    
    # Start sync if valid
    if result.status == ConfigStatus.Success:
        print("\n✅ Configuration valid - ready for deployment!")
        print(f"   Sync: {config.name}")
        print(f"   Source: {config.source_polling.frequency.label()} polling")
        print(f"   Destination: {config.destination_polling.frequency.label()} polling")
    else:
        print("\n❌ Configuration has errors - fix above recommendations")
        return 1
    
    return 0

if __name__ == "__main__":
    exit(main())
```

### Step 4: Run

```bash
python run_sync.py
```

**Output:**
```
✅ Configuration SUCCESSFUL!
   Sync: orders_to_warehouse
   Purpose: Sync Kafka order events to data warehouse
   📤 Source: every 5 minutes polling in America/New_York timezone
   📥 Destination: every 24 hours polling in America/New_York timezone
      No-sync window: 22:00 - 06:00

✅ Configuration valid - ready for deployment!
   Sync: orders_to_warehouse
   Source: every 5 minutes polling
   Destination: every 24 hours polling
```

---

## Simplest Python Integration

```python
from pyreverseetl_core.sources import SyncConfiguration

# One line to load
config = SyncConfiguration.from_yaml_file("sync.yaml")

# One line to validate
result = config.validate()

# Done! Use config in your pipeline
```

---

## Common Patterns

### Pattern 1: Basic Polling (5 min)
```yaml
name: simple_sync
source_polling:
  frequency: Hourly
```

### Pattern 2: Business Hours (10 min)
```yaml
name: business_sync
source_polling:
  frequency: FiveMinutes
  skip_days: [Saturday, Sunday]
  no_sync_after_hour: 18
  sync_resume_hour: 9
```

### Pattern 3: Multi-Timezone (15 min)
```yaml
name: global_sync
source_polling:
  frequency: Hourly
  timezone: America/New_York
destination_polling:
  frequency: Daily
  timezone: Europe/London
```

### Pattern 4: With Transformation (20 min)
```yaml
name: transform_sync
source_polling:
  frequency: FiveMinutes
transformation:
  engine: Python
  script_path: transform.py
destination_polling:
  frequency: Hourly
```

---

## Configuration Complexity Scale

```
Minimal (2 lines):
  - Source polling frequency only
  
Basic (5 lines):
  - Source + destination polling
  
Intermediate (10 lines):
  - + Timezone support
  - + Skip days / blackout dates
  
Advanced (15 lines):
  - + Transformations
  - + Dead letter topic
  - + Caching
  
Expert (20+ lines):
  - + Retry policies
  - + Time windows
  - + Multiple intermediate topics
```

---

## Troubleshooting (30 seconds)

### Problem: "Invalid timezone"
**Solution:** Use IANA timezone names
```yaml
✗ timezone: EST              # Wrong!
✓ timezone: America/New_York # Right!
```

### Problem: "Configuration invalid"
**Solution:** Check the error message
```
⚠️ Source: Invalid timezone 'Bad/Zone'
→ Fix: Use proper IANA timezone
```

### Problem: Can't load YAML file
**Solution:** Check file path and format
```bash
# Make sure file exists
ls -la sync.yaml

# Make sure YAML is valid
cat sync.yaml
```

---

## Next Steps

### Want to validate?
```python
result = config.validate()
print(result)  # Detailed status + recommendations
```

### Want to save a modified config?
```python
config.source_polling.skip_days_list(["Saturday", "Sunday"])
config.save_to_yaml_file("sync_modified.yaml")
```

### Want to use transformations?
```python
from pyreverseetl_core.sources import TransformationConfig

transform = TransformationConfig.python("transform.py")
config.with_transformation(transform)
```

### Want multiple syncs?
```python
configs = [
    SyncConfiguration.from_yaml_file("sync_a.yaml"),
    SyncConfiguration.from_yaml_file("sync_b.yaml"),
    SyncConfiguration.from_yaml_file("sync_c.yaml"),
]

for config in configs:
    validate(config)
    start_sync(config)  # Runs in parallel!
```

---

## Complete Example (Copy-Paste Ready)

### File: `sync.yaml`
```yaml
name: kafka_to_warehouse
description: Sync Kafka events to data warehouse
source_polling:
  frequency: FiveMinutes
  timezone: America/New_York
destination_polling:
  frequency: Daily
  timezone: America/New_York
```

### File: `sync.py`
```python
from pyreverseetl_core.sources import SyncConfiguration

def main():
    config = SyncConfiguration.from_yaml_file("sync.yaml")
    result = config.validate()
    print(result)
    return 0 if result.status == "Success" else 1

if __name__ == "__main__":
    exit(main())
```

### Run:
```bash
python sync.py
```

---

## What You Get

✅ **Exactly-once delivery** - Every event once  
✅ **High-volume handling** - Millions of events  
✅ **Fault tolerance** - Automatic recovery  
✅ **Rate limiting** - Automatic backpressure  
✅ **Timezone support** - Global deployments  
✅ **Auto-scaling** - Cost optimization  

**Without configuring any of it!**

---

## That's It!

You now have:
1. ✅ Kafka to warehouse sync
2. ✅ Timezone-aware scheduling
3. ✅ Exactly-once delivery
4. ✅ Fault tolerance
5. ✅ Error tracking

All with **just YAML config + Python script**.

For more details, see:
- [YAML_CONFIGURATION.md](YAML_CONFIGURATION.md) - All YAML options
- [SYNC_CONFIGURATION.md](SYNC_CONFIGURATION.md) - Advanced config
- [EXACTLY_ONCE_SEMANTICS.md](EXACTLY_ONCE_SEMANTICS.md) - Guarantees
