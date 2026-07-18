# Quick Start - PyReverseETL v2.0.1

**Get up and running in 5 minutes with a configuration file + Python script.**

---

## The Simplest Example

### 1. Create Configuration File

Create `sync.yaml`:
```yaml
name: my_first_sync
description: Sync data to warehouse

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

# Load your configuration
config = SyncConfiguration.from_yaml_file("sync.yaml")

# Verify it's valid
result = config.validate()
print(result)

# Check result
if result.status == "Success":
    print("✅ Ready to sync!")
else:
    print("❌ Fix configuration errors above")
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
cp examples/polling_config_basic.yaml sync.yaml
```

### Step 2: Customize (Edit sync.yaml)
```yaml
name: orders_to_warehouse
description: Sync orders to data warehouse

source_polling:
  frequency: FiveMinutes
  timezone: America/New_York

destination_polling:
  frequency: Daily
  timezone: America/New_York
  no_sync_after_hour: 22    # Stop at 10 PM
  sync_resume_hour: 6       # Resume at 6 AM
```

### Step 3: Create Python Script

`run_sync.py`:
```python
from pyreverseetl_core.sources import SyncConfiguration

def main():
    # Load configuration
    config = SyncConfiguration.from_yaml_file("sync.yaml")
    
    # Verify configuration
    result = config.validate()
    print(result)
    
    # Check if valid
    if result.status == "Success":
        print("\n✅ Configuration valid - ready to deploy!")
        print(f"   Name: {config.name}")
        print(f"   Check frequency: {config.source_polling.frequency.label()}")
        print(f"   Write frequency: {config.destination_polling.frequency.label()}")
    else:
        print("\n❌ Configuration has errors - see above")
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
✅ Configuration valid - ready to deploy!
   Name: orders_to_warehouse
   Check frequency: every 5 minutes
   Write frequency: every 24 hours
```

---

## Simplest Python Integration

```python
from pyreverseetl_core.sources import SyncConfiguration

# Load configuration
config = SyncConfiguration.from_yaml_file("sync.yaml")

# Verify it works
result = config.validate()
print(result)
```

---

## Common Patterns

### Pattern 1: Basic Setup (5 min)
```yaml
name: simple_sync
source_polling:
  frequency: Hourly
```

### Pattern 2: Business Hours Only (10 min)
```yaml
name: business_sync
source_polling:
  frequency: FiveMinutes
  skip_days: [Saturday, Sunday]
  no_sync_after_hour: 18    # Stop at 6 PM
  sync_resume_hour: 9       # Start at 9 AM
```

### Pattern 3: Multiple Time Zones (15 min)
```yaml
name: global_sync
source_polling:
  frequency: Hourly
  timezone: America/New_York
destination_polling:
  frequency: Daily
  timezone: Europe/London
```

### Pattern 4: With Data Transformation (20 min)
```yaml
name: transform_sync
source_polling:
  frequency: FiveMinutes
transformation:
  enabled: true
  script_path: transform.py
destination_polling:
  frequency: Hourly
```

---

## Troubleshooting (30 seconds)

### Problem: "Invalid timezone"
**Solution:** Use city/region names like these:
```yaml
✗ timezone: EST              # Too vague
✓ timezone: America/New_York # Clear and specific
```

**Valid examples:**
- America/New_York
- America/Chicago
- Europe/London
- Europe/Paris
- Asia/Tokyo
- Australia/Sydney

### Problem: "Configuration invalid"
**Solution:** Check the error message for specific issues and follow the recommendations

### Problem: Can't find configuration file
**Solution:** Make sure file exists and path is correct
```bash
ls -la sync.yaml
```

---

## Next Steps

### Verify configuration works
```python
result = config.validate()
if result.status == "Success":
    print("Ready to use!")
```

### Update configuration
```python
config.source_polling.skip_days_list(["Saturday", "Sunday"])
config.save_to_yaml_file("sync_updated.yaml")
```

### Add transformation
```python
from pyreverseetl_core.sources import TransformationConfig

transform = TransformationConfig.python("transform.py")
config.with_transformation(transform)
```

### Run multiple syncs in parallel
```python
configs = [
    SyncConfiguration.from_yaml_file("sync_a.yaml"),
    SyncConfiguration.from_yaml_file("sync_b.yaml"),
    SyncConfiguration.from_yaml_file("sync_c.yaml"),
]

# All run simultaneously without blocking each other
for config in configs:
    start_sync(config)
```

---

## Complete Copy-Paste Example

### File: `sync.yaml`
```yaml
name: orders_to_warehouse
description: Sync orders to data warehouse

source_polling:
  frequency: FiveMinutes
  timezone: America/New_York

destination_polling:
  frequency: Daily
  timezone: America/New_York
```

### File: `run.py`
```python
from pyreverseetl_core.sources import SyncConfiguration

config = SyncConfiguration.from_yaml_file("sync.yaml")
result = config.validate()
print(result)
```

### Run:
```bash
python run.py
```

---

## What You Get

✅ **Reliable delivery** - Data reaches destination reliably  
✅ **Handles high volume** - Works with large data streams  
✅ **Automatic recovery** - Continues working if something fails  
✅ **Rate limiting support** - Respects destination limits  
✅ **Time zone support** - Works globally  
✅ **Cost optimization** - Scales resources automatically  

**All without complex configuration!**

---

For more details, see:
- [USER_GUIDE_v2.0.1.md](USER_GUIDE_v2.0.1.md) - Complete guide with examples
- [YAML_CONFIGURATION.md](YAML_CONFIGURATION.md) - All configuration options
