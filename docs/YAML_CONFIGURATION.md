# YAML Configuration Guide - PyReverseETL v2.0.1

Configure polling and sync settings using YAML files for production deployments.

## Quick Start

```rust
// Load from YAML file
let config = PollingConfig::from_yaml_file("config/polling.yaml")?;

// Load from YAML string
let yaml = r#"
frequency: Hourly
enabled: true
timezone: America/New_York
skip_days:
  - Saturday
  - Sunday
no_sync_after_hour: 20
sync_resume_hour: 8
blackout_start: null
blackout_end: null
"#;
let config = PollingConfig::from_yaml(yaml)?;

// Save configuration to file
config.save_to_yaml_file("config/polling_backup.yaml")?;

// Convert to YAML string
let yaml_string = config.to_yaml()?;
```

## Configuration Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `frequency` | SyncFrequency | Yes | - | How often to poll: FiveMinutes, FifteenMinutes, ThirtyMinutes, Hourly, FourHourly, TwelveHourly, Daily |
| `enabled` | bool | No | true | Enable/disable polling |
| `timezone` | String | No | "UTC" | Timezone for time calculations (e.g., "America/New_York", "Europe/London", "Asia/Tokyo") |
| `skip_days` | List[String] | No | [] | Days to skip (e.g., "Saturday", "Sunday", "Monday") |
| `no_sync_after_hour` | Integer | No | null | Hour to stop syncing (in timezone, 0-23) |
| `sync_resume_hour` | Integer | No | null | Hour to resume syncing (in timezone, 0-23) |
| `blackout_start` | DateTime | No | null | ISO 8601 UTC datetime to start blackout period |
| `blackout_end` | DateTime | No | null | ISO 8601 UTC datetime to end blackout period |

## Examples

### Example 1: Every 5 Minutes (No Restrictions)
```yaml
frequency: FiveMinutes
enabled: true
timezone: UTC
skip_days: []
no_sync_after_hour: null
sync_resume_hour: null
blackout_start: null
blackout_end: null
```

### Example 2: Business Hours Only (NY Timezone)
```yaml
frequency: Hourly
enabled: true
timezone: America/New_York
skip_days:
  - Saturday
  - Sunday
no_sync_after_hour: 20  # 8 PM - 8 AM off
sync_resume_hour: 8
blackout_start: null
blackout_end: null
```

### Example 3: With Holiday Maintenance
```yaml
frequency: Daily
enabled: true
timezone: America/New_York
skip_days:
  - Saturday
  - Sunday
no_sync_after_hour: 18  # 6 PM - 6 AM off
sync_resume_hour: 6
# Skip Dec 20-26 for holiday maintenance
blackout_start: 2026-12-20T00:00:00Z
blackout_end: 2026-12-26T23:59:59Z
```

### Example 4: European Timezone
```yaml
frequency: ThirtyMinutes
enabled: true
timezone: Europe/London
skip_days:
  - Saturday
  - Sunday
  - "Monday"  # Office closed Mondays
no_sync_after_hour: 22  # 10 PM - 7 AM off
sync_resume_hour: 7
blackout_start: null
blackout_end: null
```

### Example 5: Asian Timezone
```yaml
frequency: FourHourly
enabled: true
timezone: Asia/Tokyo
skip_days: []  # No skip days
no_sync_after_hour: 23  # 11 PM - 7 AM off
sync_resume_hour: 7
blackout_start: null
blackout_end: null
```

## Supported Timezones

**US Timezones:**
- America/New_York
- America/Chicago
- America/Denver
- America/Los_Angeles
- America/Anchorage
- Pacific/Honolulu

**Europe Timezones:**
- UTC
- Europe/London
- Europe/Paris
- Europe/Berlin
- Europe/Amsterdam
- Europe/Brussels
- Europe/Vienna
- Europe/Prague
- Europe/Warsaw
- Europe/Moscow

**Asia Timezones:**
- Asia/Tokyo
- Asia/Shanghai
- Asia/Hong_Kong
- Asia/Singapore
- Asia/Bangkok
- Asia/Shanghai
- Asia/Kolkata
- Asia/Dubai

**Australia Timezones:**
- Australia/Sydney
- Australia/Melbourne
- Australia/Brisbane
- Australia/Perth
- Australia/Adelaide

**And 400+ more IANA timezones...**

## Field Descriptions

### frequency
How often to check for changes:
- `FiveMinutes`: Every 5 minutes (300 seconds)
- `FifteenMinutes`: Every 15 minutes (900 seconds)
- `ThirtyMinutes`: Every 30 minutes (1800 seconds)
- `Hourly`: Every hour (3600 seconds)
- `FourHourly`: Every 4 hours (14400 seconds)
- `TwelveHourly`: Every 12 hours (43200 seconds)
- `Daily`: Every 24 hours (86400 seconds)

### timezone
IANA timezone string used for:
- Evaluating `skip_days` (what day is it NOW?)
- Calculating `no_sync_after_hour` and `sync_resume_hour` (what time is it NOW?)

Note: `blackout_start` and `blackout_end` are always UTC for consistency.

### skip_days
List of day names to skip syncing. Supported values:
- Monday, Tuesday, Wednesday, Thursday, Friday, Saturday, Sunday

Example:
```yaml
skip_days:
  - Saturday
  - Sunday
```

### no_sync_after_hour / sync_resume_hour
Define a daily time window when syncing is disabled.

Examples:
- `no_sync_after_hour: 20, sync_resume_hour: 8` → No syncs 8 PM - 8 AM
- `no_sync_after_hour: 22, sync_resume_hour: 6` → No syncs 10 PM - 6 AM
- `no_sync_after_hour: 18, sync_resume_hour: 9` → No syncs 6 PM - 9 AM

All times are in the configured `timezone`.

### blackout_start / blackout_end
Absolute date ranges to disable all syncing (e.g., for maintenance windows).

Format: ISO 8601 UTC datetime
Example: `2026-12-20T00:00:00Z` (December 20, 2026 at midnight UTC)

Example:
```yaml
blackout_start: 2026-12-20T00:00:00Z  # Dec 20 at midnight
blackout_end: 2026-12-26T23:59:59Z    # Dec 26 at end of day
```

## Usage in Code

### Load and Use
```rust
use pyreverseetl_core::sources::polling::PollingConfig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load from file
    let config = PollingConfig::from_yaml_file("config/polling.yaml")?;
    
    // Check if ready to poll
    if config.should_poll() {
        println!("Time to sync!");
    } else {
        println!("Skipping sync - outside allowed window");
    }
    
    // Get metrics
    let metrics = config.metrics();
    println!("Polls: {}, Changes: {}", metrics.poll_count, metrics.change_count);
    
    Ok(())
}
```

### Modify and Save
```rust
let mut config = PollingConfig::from_yaml_file("config/polling.yaml")?;

// Modify
config.skip_day("Tuesday");
config.set_no_sync_window(18, 9);

// Save changes
config.save_to_yaml_file("config/polling_modified.yaml")?;
```

### Dynamic Timezone Update
```rust
let mut config = PollingConfig::new(SyncFrequency::Hourly);
config.set_timezone("Europe/London");

// Verify timezone is valid
if config.validate_timezone().is_ok() {
    println!("Timezone set to: {}", config.timezone);
}
```

## Error Handling

```rust
match PollingConfig::from_yaml_file("config.yaml") {
    Ok(config) => println!("Config loaded: {:?}", config),
    Err(e) => eprintln!("Failed to load config: {}", e),
}
```

Common errors:
- `Failed to read config file: ...` - File doesn't exist or permission denied
- `Failed to parse YAML config: ...` - YAML syntax error
- `Invalid timezone: ...` - Unsupported timezone string

## Best Practices

1. **Use UTC for blackout periods** - Always specify blackout dates in UTC for consistency across deployments
2. **Timezone abbreviations** - Use full IANA timezone names (e.g., `America/New_York`, not `EST`)
3. **Validate timezones** - Call `config.validate_timezone()` after loading to catch errors early
4. **Version control** - Keep configs in git for deployment tracking
5. **Comment complex configs** - Document why certain skip days or blackout periods exist
6. **Test time windows** - Verify skip windows work as expected in your timezone

## Troubleshooting

**Problem:** Syncs running when they shouldn't
- Check current timezone: `config.current_hour_in_timezone()` and `config.current_day_in_timezone()`
- Verify skip_days match your timezone's day-of-week
- Ensure `no_sync_after_hour` < `sync_resume_hour` (or understand edge case logic)

**Problem:** "Invalid timezone"
- Use IANA timezone database names (case-sensitive)
- Not supported: `EST`, `PST`, `GMT`, `IST` (too ambiguous)
- Do use: `America/New_York`, `Europe/London`, `Asia/Kolkata`

**Problem:** Config fails to load from YAML
- Check YAML syntax (spacing, indentation)
- Ensure all fields match expected types (integers, booleans, strings)
- Verify datetime format matches ISO 8601
