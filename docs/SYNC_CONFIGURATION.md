# Sync Configuration Guide - PyReverseETL v2.0.1

Complete guide to configuring and validating source → destination syncs with detailed status messages and fault tolerance.

## Overview

PyReverseETL v2.0.1 supports **separate polling configurations for sources and destinations**, allowing fine-grained control over:
- How often to check each system for changes
- Timezone-aware scheduling
- Time windows and maintenance periods
- Comprehensive validation with detailed messages

## Quick Start

```rust
use pyreverseetl_core::sources::{SyncConfiguration, SyncFrequency, PollingConfig};

// Load configuration from YAML file
let config = SyncConfiguration::from_yaml_file("sync.yaml")?;

// Validate configuration (get detailed status)
let result = config.validate();
println!("{}", result);  // Prints congratulatory or error message

// Check if configuration is valid
match result.status {
    ConfigStatus::Success => {
        println!("✅ All systems go!");
    }
    ConfigStatus::SourceProblem => {
        println!("❌ Fix source configuration");
    }
    ConfigStatus::DestinationProblem => {
        println!("❌ Fix destination configuration");
    }
    ConfigStatus::BothHaveProblem => {
        println!("❌ Fix both configurations");
    }
    ConfigStatus::Incomplete => {
        println!("❌ Missing required configuration");
    }
}
```

## Configuration Structure

### YAML Format

```yaml
name: kafka_to_warehouse              # Sync identifier
description: Kafka → Data Warehouse   # Description (optional)

# Source polling (optional)
source_polling:
  frequency: FiveMinutes              # Polling interval
  enabled: true
  timezone: America/New_York
  skip_days: [Saturday, Sunday]
  no_sync_after_hour: 20              # No syncs 8 PM - 8 AM
  sync_resume_hour: 8
  blackout_start: null
  blackout_end: null

# Destination polling (optional)
destination_polling:
  frequency: Daily
  enabled: true
  timezone: America/New_York
  skip_days: [Saturday, Sunday]
  no_sync_after_hour: 22              # Different schedule!
  sync_resume_hour: 6
  blackout_start: null
  blackout_end: null
```

## Configuration Result

After validation, you get a detailed `ConfigurationResult`:

```rust
pub struct ConfigurationResult {
    pub status: ConfigStatus,           // Success or specific problem
    pub message: String,                // Congratulatory or error message
    pub details: ConfigurationDetails,  // Breakdown of configuration
    pub recommendations: Vec<String>,   // How to fix issues
}
```

### Success Message Example

```
✅ Configuration SUCCESSFUL!
   Sync: kafka_to_redshift
   Purpose: Real-time event sync to data warehouse
   📤 Source: every 5 minutes polling in America/New_York timezone
      Skip days: Saturday, Sunday
      No-sync window: 20:00 - 08:00
   📥 Destination: every 24 hours polling in America/New_York timezone
      Skip days: Saturday, Sunday
      No-sync window: 22:00 - 06:00
```

### Error Message Example

```
❌ Source polling configuration has errors

Recommendations:
  ⚠️  Source: Invalid timezone 'Bad/Zone'. Use IANA timezone names like 'America/New_York'
```

## Source vs Destination Polling

Different source and destination systems have different performance characteristics:

| Aspect | Source | Destination |
|--------|--------|-------------|
| **Typical case** | Check frequently | Validate less often |
| **Example: Kafka → SQL** | Poll Kafka every 5 min | Verify SQL insert hourly |
| **Example: API → S3** | API every 30 min | S3 daily |
| **Timezone** | Source system's timezone | Destination system's timezone |
| **Maintenance** | Source downtime windows | Destination maintenance windows |

### When Source and Destination Differ

Use different polling schedules when:

1. **Performance requirements differ** - Source needs frequent checks, destination can batch
2. **System maintenance schedules differ** - Source available 24/7, destination has maintenance windows
3. **Cost considerations** - Destination has usage-based billing, need to limit API calls
4. **Timezone requirements** - Multi-region deployments need local timezone awareness
5. **Compliance/SLA differences** - Different uptime/availability guarantees

## Usage Examples

### Example 1: Kafka → Redshift (Different Frequencies)

```yaml
name: kafka_to_redshift

# Kafka: Frequent polling (capture events quickly)
source_polling:
  frequency: FiveMinutes
  enabled: true
  timezone: UTC
  skip_days: []

# Redshift: Less frequent (batch insertions, cost optimization)
destination_polling:
  frequency: Daily
  enabled: true
  timezone: America/New_York
  no_sync_after_hour: 22
  sync_resume_hour: 6
```

**Result:**
```
✅ Configuration SUCCESSFUL!
   Sync: kafka_to_redshift
   📤 Source: every 5 minutes polling in UTC timezone
   📥 Destination: every 24 hours polling in America/New_York timezone
      No-sync window: 22:00 - 06:00
```

### Example 2: API → S3 (Same Frequency, Different Timezones)

```yaml
name: api_to_s3

# API: UTC
source_polling:
  frequency: Hourly
  timezone: UTC
  skip_days: []

# S3: Business hours only, US timezone
destination_polling:
  frequency: Hourly
  timezone: America/Los_Angeles
  no_sync_after_hour: 18  # 6 PM
  sync_resume_hour: 9     # 9 AM
  skip_days: [Saturday, Sunday]
```

### Example 3: Only Source Polling

When you only need to monitor source for changes:

```yaml
name: api_monitoring

source_polling:
  frequency: ThirtyMinutes
  timezone: America/New_York
  skip_days: [Saturday]

# No destination_polling = on-demand only
```

**Result:**
```
✅ Configuration SUCCESSFUL!
   Sync: api_monitoring
   📤 Source: every 30 minutes polling in America/New_York timezone
      Skip days: Saturday
   📥 Destination: No polling configured (on-demand only)
```

## Fault Tolerance & Error Handling

PyReverseETL v2.0.1 provides detailed fault tolerance:

### 1. Configuration Validation
- Both source and destination timezones are validated
- Invalid timezones are reported with recommendations
- Clear distinction between source and destination errors

### 2. Error Recovery
- If source fails: destination polling continues
- If destination fails: source polling continues
- Both can operate independently

### 3. Message Delivery Guarantees
- Validation messages are always logged
- Errors are reported with actionable recommendations
- Success messages confirm all settings are correct

### Code Example: Error Recovery

```rust
let config = SyncConfiguration::from_yaml_file("sync.yaml")?;
let result = config.validate();

// Log the result (success or error)
match result.status {
    ConfigStatus::Success => {
        // Start sync with confidence
        start_sync(&config)?;
    }
    ConfigStatus::SourceProblem => {
        // Disable source polling, keep destination alive
        eprintln!("Warning: {}", result.message);
        for rec in &result.recommendations {
            eprintln!("{}", rec);
        }
        start_sync_destination_only(&config)?;
    }
    ConfigStatus::DestinationProblem => {
        // Disable destination polling, keep source alive
        eprintln!("Warning: {}", result.message);
        start_sync_source_only(&config)?;
    }
    ConfigStatus::BothHaveProblem => {
        // Stop everything until fixed
        eprintln!("Error: {}", result.message);
        return Err("Cannot start sync with configuration errors".into());
    }
    ConfigStatus::Incomplete => {
        eprintln!("Error: {}", result.message);
        return Err("Sync configuration is incomplete".into());
    }
}
```

## API Reference

### SyncConfiguration

```rust
// Create new sync
let config = SyncConfiguration::new("my_sync");

// Configure source polling
config.with_source_polling(PollingConfig::new(SyncFrequency::Hourly));

// Configure destination polling (different schedule!)
config.with_destination_polling(PollingConfig::with_timezone(
    SyncFrequency::Daily,
    "America/New_York"
));

// Add description
config.with_description("Sync data to warehouse");

// Validate
let result = config.validate();

// Load from YAML
let config = SyncConfiguration::from_yaml_file("sync.yaml")?;

// Save to YAML
config.save_to_yaml_file("sync_backup.yaml")?;
```

### ConfigurationResult

```rust
// Check status
match result.status {
    ConfigStatus::Success => { /* ... */ }
    ConfigStatus::SourceProblem => { /* ... */ }
    ConfigStatus::DestinationProblem => { /* ... */ }
    ConfigStatus::BothHaveProblem => { /* ... */ }
    ConfigStatus::Incomplete => { /* ... */ }
}

// Print formatted message
println!("{}", result);

// Access detailed configuration breakdown
println!("Source configured: {}", result.details.source_polling_configured);
println!("Destination configured: {}", result.details.destination_polling_configured);
println!("Source skip days: {}", result.details.source_skip_days);
println!("Destination skip days: {}", result.details.destination_skip_days);

// Get specific recommendations
for recommendation in &result.recommendations {
    eprintln!("Fix: {}", recommendation);
}
```

## Best Practices

### 1. **Use YAML Files for Production**
Keep configuration in version-controlled YAML files instead of hardcoding.

```rust
// Good: Load from YAML
let config = SyncConfiguration::from_yaml_file("config/sync.yaml")?;
```

### 2. **Validate on Startup**
Always validate configuration before starting syncs.

```rust
let config = SyncConfiguration::from_yaml_file("sync.yaml")?;
let result = config.validate();
if result.status != ConfigStatus::Success {
    eprintln!("{}", result);
    return Err("Invalid configuration".into());
}
```

### 3. **Use Timezone for Multi-Region Deployments**
Different regions should use different timezones:

```yaml
# US region
source_polling:
  timezone: America/New_York

# EU region (same deployment, different sync config)
source_polling:
  timezone: Europe/London

# APAC region
source_polling:
  timezone: Asia/Tokyo
```

### 4. **Document Why Configurations Differ**
Use the `description` field to explain sync behavior:

```yaml
name: kafka_to_warehouse
description: |
  - Kafka polled every 5 min (catch events quickly)
  - Warehouse polled daily (batch optimization)
  - No syncs weekends (cost savings)
  - Maintenance window 10 PM - 6 AM (backup time)
```

### 5. **Test Configuration Changes**
Validate before deploying:

```bash
# Test new config
cargo test --lib sync_config
```

## Troubleshooting

### Problem: "Invalid timezone"
**Solution:** Use IANA timezone database names
- ✗ Invalid: `EST`, `PST`, `GMT`, `IST`
- ✓ Valid: `America/New_York`, `Europe/London`, `Asia/Tokyo`

### Problem: Source syncs but destination doesn't
**Solution:** Check destination timezone and no-sync window
```rust
let result = config.validate();
println!("Dest timezone valid: {:?}", result.details.destination_timezone_valid);
println!("Dest has time window: {}", result.details.destination_has_time_window);
```

### Problem: Syncs stopped on weekends
**Solution:** Review skip_days configuration
```rust
println!("Dest skip days: {}", result.details.destination_skip_days);
```

## Testing Your Configuration

```rust
#[test]
fn test_my_sync_config() {
    let config = SyncConfiguration::from_yaml_file("config/my_sync.yaml").unwrap();
    let result = config.validate();
    
    // Verify it's valid
    assert_eq!(result.status, ConfigStatus::Success);
    
    // Verify it has the expected settings
    assert!(result.details.source_polling_configured);
    assert!(result.details.destination_polling_configured);
    
    // No warnings
    assert!(result.recommendations.is_empty());
}
```

## See Also

- [YAML Configuration Guide](YAML_CONFIGURATION.md) - Polling configuration details
- [Examples](../examples/) - Sample sync configurations
- [Error Messages Guide](#troubleshooting) - Common issues and solutions
