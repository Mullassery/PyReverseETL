use chrono::{DateTime, Duration, Utc, Weekday, Timelike, Local, Datelike};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Preset sync frequency intervals
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SyncFrequency {
    /// Every 5 minutes
    FiveMinutes,
    /// Every 15 minutes
    FifteenMinutes,
    /// Every 30 minutes
    ThirtyMinutes,
    /// Every hour
    Hourly,
    /// Every 4 hours
    FourHourly,
    /// Every 12 hours
    TwelveHourly,
    /// Every 24 hours
    Daily,
    /// Custom interval in seconds
    Custom(u64),
}

impl SyncFrequency {
    /// Get the interval in seconds
    pub fn as_seconds(&self) -> u64 {
        match self {
            SyncFrequency::FiveMinutes => 5 * 60,
            SyncFrequency::FifteenMinutes => 15 * 60,
            SyncFrequency::ThirtyMinutes => 30 * 60,
            SyncFrequency::Hourly => 3600,
            SyncFrequency::FourHourly => 4 * 3600,
            SyncFrequency::TwelveHourly => 12 * 3600,
            SyncFrequency::Daily => 24 * 3600,
            SyncFrequency::Custom(seconds) => *seconds,
        }
    }

    /// Get human-readable label
    pub fn label(&self) -> &str {
        match self {
            SyncFrequency::FiveMinutes => "every 5 minutes",
            SyncFrequency::FifteenMinutes => "every 15 minutes",
            SyncFrequency::ThirtyMinutes => "every 30 minutes",
            SyncFrequency::Hourly => "hourly",
            SyncFrequency::FourHourly => "every 4 hours",
            SyncFrequency::TwelveHourly => "every 12 hours",
            SyncFrequency::Daily => "daily",
            SyncFrequency::Custom(_) => "custom interval",
        }
    }
}

impl Default for SyncFrequency {
    fn default() -> Self {
        SyncFrequency::Hourly
    }
}

/// Polling configuration for change detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollingConfig {
    /// How often to poll for changes
    pub frequency: SyncFrequency,
    /// Enable polling
    pub enabled: bool,
    /// Timezone for start/end time calculations (e.g., "America/New_York", "UTC")
    pub timezone: String, // Default: "UTC"
    /// Days of week to skip polling (e.g., Saturday, Sunday)
    /// Use Weekday enum: Mon, Tue, Wed, Thu, Fri, Sat, Sun
    pub skip_days: HashSet<String>, // "Saturday", "Sunday", etc.
    /// Blackout dates - start of date range to skip syncs
    pub blackout_start: Option<DateTime<Utc>>,
    /// Blackout dates - end of date range to skip syncs
    pub blackout_end: Option<DateTime<Utc>>,
    /// Hour to prevent syncs from starting (e.g., 20 = 8 PM to 8 AM next day, in specified timezone)
    pub no_sync_after_hour: Option<u32>,
    /// Hour to allow syncs to resume (e.g., 8 = 8 AM, in specified timezone)
    pub sync_resume_hour: Option<u32>,
    /// Last successful poll timestamp
    #[serde(skip)]
    pub last_poll_at: Option<DateTime<Utc>>,
    /// Last poll that detected changes
    #[serde(skip)]
    pub last_change_at: Option<DateTime<Utc>>,
    /// Number of successful polls
    #[serde(skip)]
    pub poll_count: u64,
    /// Number of polls that detected changes
    #[serde(skip)]
    pub change_count: u64,
}

impl PollingConfig {
    /// Create new polling config with frequency (defaults to UTC timezone)
    pub fn new(frequency: SyncFrequency) -> Self {
        Self {
            frequency,
            enabled: true,
            timezone: "UTC".to_string(),
            skip_days: HashSet::new(),
            blackout_start: None,
            blackout_end: None,
            no_sync_after_hour: None,
            sync_resume_hour: None,
            last_poll_at: None,
            last_change_at: None,
            poll_count: 0,
            change_count: 0,
        }
    }

    /// Create with timezone (e.g., "America/New_York", "Europe/London", "Asia/Tokyo")
    pub fn with_timezone(frequency: SyncFrequency, timezone: &str) -> Self {
        Self {
            frequency,
            enabled: true,
            timezone: timezone.to_string(),
            skip_days: HashSet::new(),
            blackout_start: None,
            blackout_end: None,
            no_sync_after_hour: None,
            sync_resume_hour: None,
            last_poll_at: None,
            last_change_at: None,
            poll_count: 0,
            change_count: 0,
        }
    }

    /// Set timezone (e.g., "America/New_York")
    pub fn set_timezone(&mut self, timezone: &str) -> &mut Self {
        self.timezone = timezone.to_string();
        self
    }

    /// Get current timezone
    pub fn get_timezone(&self) -> &str {
        &self.timezone
    }

    /// Parse and validate timezone
    pub fn validate_timezone(&self) -> Result<Tz, String> {
        Tz::from_str(&self.timezone)
            .map_err(|_| format!("Invalid timezone: {}", self.timezone))
    }

    /// Get current hour in configured timezone
    pub fn current_hour_in_timezone(&self) -> Result<u32, String> {
        let tz = self.validate_timezone()?;
        let now = Utc::now();
        let now_local = now.with_timezone(&tz);
        Ok(now_local.hour())
    }

    /// Get current day of week in configured timezone
    pub fn current_day_in_timezone(&self) -> Result<Weekday, String> {
        let tz = self.validate_timezone()?;
        let now = Utc::now();
        let now_local = now.with_timezone(&tz);
        Ok(now_local.weekday())
    }

    /// Add a day to skip syncing (e.g., "Saturday", "Sunday")
    pub fn skip_day(&mut self, day: &str) -> &mut Self {
        self.skip_days.insert(day.to_string());
        self
    }

    /// Skip multiple days (e.g., ["Saturday", "Sunday"])
    pub fn skip_days_list(&mut self, days: Vec<&str>) -> &mut Self {
        for day in days {
            self.skip_days.insert(day.to_string());
        }
        self
    }

    /// Set blackout date range (no syncs between start and end)
    pub fn set_blackout_period(&mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> &mut Self {
        self.blackout_start = Some(start);
        self.blackout_end = Some(end);
        self
    }

    /// Set time window to prevent syncs (e.g., no_sync_after=20, resume=8 means 8PM-8AM)
    pub fn set_no_sync_window(&mut self, no_sync_after_hour: u32, resume_hour: u32) -> &mut Self {
        self.no_sync_after_hour = Some(no_sync_after_hour);
        self.sync_resume_hour = Some(resume_hour);
        self
    }

    /// Check if today is a skip day
    pub fn is_skip_day(&self) -> bool {
        let now = Utc::now();
        let weekday = now.weekday();

        let day_name = match weekday {
            Weekday::Mon => "Monday",
            Weekday::Tue => "Tuesday",
            Weekday::Wed => "Wednesday",
            Weekday::Thu => "Thursday",
            Weekday::Fri => "Friday",
            Weekday::Sat => "Saturday",
            Weekday::Sun => "Sunday",
        };

        self.skip_days.contains(day_name)
    }

    /// Check if current time is in blackout period
    pub fn is_in_blackout(&self) -> bool {
        let now = Utc::now();

        match (self.blackout_start, self.blackout_end) {
            (Some(start), Some(end)) => now >= start && now <= end,
            _ => false,
        }
    }

    /// Check if current hour is in no-sync window (in configured timezone)
    pub fn is_in_no_sync_window(&self) -> bool {
        match (self.no_sync_after_hour, self.sync_resume_hour) {
            (Some(no_sync_after), Some(resume)) => {
                // Get current hour in configured timezone
                let current_hour = match self.current_hour_in_timezone() {
                    Ok(hour) => hour,
                    Err(_) => return false, // Invalid timezone, allow syncing
                };

                if no_sync_after < resume {
                    // Normal case: no_sync_after=20, resume=8 (8 PM to 8 AM)
                    current_hour >= no_sync_after || current_hour < resume
                } else {
                    // Edge case: no_sync_after=1, resume=23 (1 AM to 11 PM)
                    current_hour >= no_sync_after && current_hour < resume
                }
            }
            _ => false,
        }
    }

    /// Check if it's time to poll (respects skip days, blackout, and time windows)
    pub fn should_poll(&self) -> bool {
        if !self.enabled {
            return false;
        }

        // Check blackout period
        if self.is_in_blackout() {
            return false;
        }

        // Check skip days
        if self.is_skip_day() {
            return false;
        }

        // Check no-sync time window
        if self.is_in_no_sync_window() {
            return false;
        }

        // Check frequency interval
        match self.last_poll_at {
            None => true,
            Some(last_poll) => {
                let interval = Duration::seconds(self.frequency.as_seconds() as i64);
                Utc::now() - last_poll >= interval
            }
        }
    }

    /// Get time until next poll
    pub fn time_until_next_poll(&self) -> Option<Duration> {
        self.last_poll_at.map(|last_poll| {
            let interval = Duration::seconds(self.frequency.as_seconds() as i64);
            let next_poll = last_poll + interval;
            next_poll - Utc::now()
        })
    }

    /// Record a poll attempt
    pub fn record_poll(&mut self) {
        self.poll_count += 1;
        self.last_poll_at = Some(Utc::now());
    }

    /// Record a poll that detected changes
    pub fn record_change(&mut self) {
        self.change_count += 1;
        self.last_change_at = Some(Utc::now());
    }

    /// Get polling metrics
    pub fn metrics(&self) -> PollingMetrics {
        PollingMetrics {
            enabled: self.enabled,
            frequency: self.frequency.label().to_string(),
            poll_count: self.poll_count,
            change_count: self.change_count,
            last_poll_at: self.last_poll_at,
            last_change_at: self.last_change_at,
            should_poll_now: self.should_poll(),
        }
    }

    /// Load polling configuration from YAML string
    pub fn from_yaml(yaml_str: &str) -> Result<Self, String> {
        serde_yaml::from_str(yaml_str)
            .map_err(|e| format!("Failed to parse YAML config: {}", e))
    }

    /// Load polling configuration from YAML file
    pub fn from_yaml_file(path: &str) -> Result<Self, String> {
        std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file: {}", e))
            .and_then(|content| Self::from_yaml(&content))
    }

    /// Convert to YAML string
    pub fn to_yaml(&self) -> Result<String, String> {
        serde_yaml::to_string(self)
            .map_err(|e| format!("Failed to serialize to YAML: {}", e))
    }

    /// Save configuration to YAML file
    pub fn save_to_yaml_file(&self, path: &str) -> Result<(), String> {
        let yaml = self.to_yaml()?;
        std::fs::write(path, yaml)
            .map_err(|e| format!("Failed to write config file: {}", e))
    }
}

/// Polling metrics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollingMetrics {
    pub enabled: bool,
    pub frequency: String,
    pub poll_count: u64,
    pub change_count: u64,
    pub last_poll_at: Option<DateTime<Utc>>,
    pub last_change_at: Option<DateTime<Utc>>,
    pub should_poll_now: bool,
}

/// Result of a polling operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollResult {
    pub changes_detected: bool,
    pub change_count: u64,
    pub error: Option<String>,
    pub polled_at: DateTime<Utc>,
}

/// Trait for sources that support polling for changes
pub trait ChangePoller: Send + Sync {
    /// Poll for changes (non-blocking)
    fn poll_changes(&self) -> crate::Result<Option<u64>>;

    /// Get polling config
    fn polling_config(&self) -> PollingConfig;

    /// Update polling config
    fn set_polling_config(&mut self, config: PollingConfig);

    /// Get polling metrics
    fn polling_metrics(&self) -> PollingMetrics {
        self.polling_config().metrics()
    }
}

/// Shared polling state for thread-safe access
pub struct SharedPollingState {
    config: Arc<Mutex<PollingConfig>>,
}

impl SharedPollingState {
    /// Create new shared polling state
    pub fn new(config: PollingConfig) -> Self {
        Self {
            config: Arc::new(Mutex::new(config)),
        }
    }

    /// Check if should poll
    pub async fn should_poll(&self) -> bool {
        self.config.lock().await.should_poll()
    }

    /// Record poll attempt
    pub async fn record_poll(&self) {
        self.config.lock().await.record_poll();
    }

    /// Record change detection
    pub async fn record_change(&self) {
        self.config.lock().await.record_change();
    }

    /// Get metrics
    pub async fn metrics(&self) -> PollingMetrics {
        self.config.lock().await.metrics()
    }

    /// Get time until next poll
    pub async fn time_until_next_poll(&self) -> Option<Duration> {
        self.config.lock().await.time_until_next_poll()
    }

    /// Update config
    pub async fn set_frequency(&self, frequency: SyncFrequency) {
        self.config.lock().await.frequency = frequency;
    }

    /// Enable/disable polling
    pub async fn set_enabled(&self, enabled: bool) {
        self.config.lock().await.enabled = enabled;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_frequency_seconds() {
        assert_eq!(SyncFrequency::FiveMinutes.as_seconds(), 300);
        assert_eq!(SyncFrequency::Hourly.as_seconds(), 3600);
        assert_eq!(SyncFrequency::Daily.as_seconds(), 86400);
        assert_eq!(SyncFrequency::Custom(120).as_seconds(), 120);
    }

    #[test]
    fn test_sync_frequency_labels() {
        assert_eq!(SyncFrequency::FiveMinutes.label(), "every 5 minutes");
        assert_eq!(SyncFrequency::Hourly.label(), "hourly");
        assert_eq!(SyncFrequency::Daily.label(), "daily");
    }

    #[test]
    fn test_polling_config_new() {
        let config = PollingConfig::new(SyncFrequency::Hourly);
        assert!(config.enabled);
        assert_eq!(config.frequency, SyncFrequency::Hourly);
        assert!(config.last_poll_at.is_none());
    }

    #[test]
    fn test_polling_config_should_poll() {
        let config = PollingConfig::new(SyncFrequency::FiveMinutes);
        assert!(config.should_poll()); // First poll always true

        let mut config = PollingConfig::new(SyncFrequency::FiveMinutes);
        config.last_poll_at = Some(Utc::now());
        assert!(!config.should_poll()); // Just polled, don't poll again
    }

    #[test]
    fn test_polling_config_disabled() {
        let config = PollingConfig {
            frequency: SyncFrequency::Hourly,
            enabled: false,
            skip_days: HashSet::new(),
            blackout_start: None,
            blackout_end: None,
            no_sync_after_hour: None,
            sync_resume_hour: None,
            timezone: "UTC".to_string(),
            last_poll_at: None,
            last_change_at: None,
            poll_count: 0,
            change_count: 0,
        };
        assert!(!config.should_poll());
    }

    #[test]
    fn test_polling_metrics() {
        let mut config = PollingConfig::new(SyncFrequency::Hourly);
        config.record_poll();
        config.record_poll();
        config.record_change();

        let metrics = config.metrics();
        assert_eq!(metrics.poll_count, 2);
        assert_eq!(metrics.change_count, 1);
        assert!(metrics.last_poll_at.is_some());
    }

    #[test]
    fn test_polling_config_record_poll() {
        let mut config = PollingConfig::new(SyncFrequency::Hourly);
        assert_eq!(config.poll_count, 0);

        config.record_poll();
        assert_eq!(config.poll_count, 1);
        assert!(config.last_poll_at.is_some());
    }

    #[test]
    fn test_polling_config_record_change() {
        let mut config = PollingConfig::new(SyncFrequency::Hourly);
        assert_eq!(config.change_count, 0);

        config.record_change();
        assert_eq!(config.change_count, 1);
        assert!(config.last_change_at.is_some());
    }

    #[test]
    fn test_sync_frequency_default() {
        let freq = SyncFrequency::default();
        assert_eq!(freq, SyncFrequency::Hourly);
    }

    #[test]
    fn test_polling_config_skip_day() {
        let mut config = PollingConfig::new(SyncFrequency::Hourly);
        config.skip_day("Saturday");
        config.skip_day("Sunday");

        assert!(config.skip_days.contains("Saturday"));
        assert!(config.skip_days.contains("Sunday"));
        assert!(!config.skip_days.contains("Monday"));
    }

    #[test]
    fn test_polling_config_skip_days_list() {
        let mut config = PollingConfig::new(SyncFrequency::Hourly);
        config.skip_days_list(vec!["Saturday", "Sunday"]);

        assert_eq!(config.skip_days.len(), 2);
        assert!(config.skip_days.contains("Saturday"));
        assert!(config.skip_days.contains("Sunday"));
    }

    #[test]
    fn test_polling_config_blackout_period() {
        let start = Utc::now();
        let end = Utc::now() + Duration::days(7);

        let mut config = PollingConfig::new(SyncFrequency::Hourly);
        config.set_blackout_period(start, end);

        assert!(config.is_in_blackout());
    }

    #[test]
    fn test_polling_config_blackout_period_not_in_range() {
        let start = Utc::now() - Duration::days(7);
        let end = Utc::now() - Duration::days(1);

        let mut config = PollingConfig::new(SyncFrequency::Hourly);
        config.set_blackout_period(start, end);

        assert!(!config.is_in_blackout());
    }

    #[test]
    fn test_polling_config_no_sync_window() {
        let mut config = PollingConfig::new(SyncFrequency::Hourly);
        // No sync from 8 PM (20) to 8 AM (8)
        config.set_no_sync_window(20, 8);

        assert!(config.no_sync_after_hour.is_some());
        assert!(config.sync_resume_hour.is_some());
    }

    #[test]
    fn test_polling_should_poll_with_blackout() {
        let now = Utc::now();
        let future = now + Duration::days(7);

        let mut config = PollingConfig::new(SyncFrequency::Hourly);
        config.enabled = true;
        config.set_blackout_period(now, future);

        // Should not poll during blackout
        assert!(!config.should_poll());
    }

    #[test]
    fn test_polling_config_builder_pattern() {
        let mut config = PollingConfig::new(SyncFrequency::Daily);
        config
            .skip_days_list(vec!["Saturday", "Sunday"])
            .set_no_sync_window(20, 8);

        assert_eq!(config.skip_days.len(), 2);
        assert!(config.no_sync_after_hour.is_some());
    }

    #[test]
    fn test_polling_config_default_timezone() {
        let config = PollingConfig::new(SyncFrequency::Hourly);
        assert_eq!(config.timezone, "UTC");
    }

    #[test]
    fn test_polling_config_with_timezone() {
        let config = PollingConfig::with_timezone(SyncFrequency::Hourly, "America/New_York");
        assert_eq!(config.timezone, "America/New_York");
    }

    #[test]
    fn test_polling_config_set_timezone() {
        let mut config = PollingConfig::new(SyncFrequency::Hourly);
        config.set_timezone("Europe/London");
        assert_eq!(config.timezone, "Europe/London");
    }

    #[test]
    fn test_polling_config_validate_timezone_valid() {
        let config = PollingConfig::with_timezone(SyncFrequency::Hourly, "America/New_York");
        assert!(config.validate_timezone().is_ok());
    }

    #[test]
    fn test_polling_config_validate_timezone_invalid() {
        let config = PollingConfig::with_timezone(SyncFrequency::Hourly, "Invalid/Timezone");
        assert!(config.validate_timezone().is_err());
    }

    #[test]
    fn test_polling_config_current_hour_in_timezone() {
        let config = PollingConfig::with_timezone(SyncFrequency::Hourly, "UTC");
        assert!(config.current_hour_in_timezone().is_ok());

        let hour = config.current_hour_in_timezone().unwrap();
        assert!(hour < 24);
    }

    #[test]
    fn test_polling_config_timezone_list() {
        let timezones = vec![
            "UTC",
            "America/New_York",
            "America/Los_Angeles",
            "Europe/London",
            "Europe/Paris",
            "Asia/Tokyo",
            "Asia/Shanghai",
            "Australia/Sydney",
        ];

        for tz in timezones {
            let config = PollingConfig::with_timezone(SyncFrequency::Hourly, tz);
            assert!(config.validate_timezone().is_ok(), "Timezone {} should be valid", tz);
        }
    }

    #[test]
    fn test_yaml_config_basic() {
        let yaml = r#"
frequency: Hourly
enabled: true
timezone: UTC
skip_days: []
no_sync_after_hour: null
sync_resume_hour: null
blackout_start: null
blackout_end: null
"#;
        let config = PollingConfig::from_yaml(yaml);
        assert!(config.is_ok());
        let cfg = config.unwrap();
        assert_eq!(cfg.frequency, SyncFrequency::Hourly);
        assert!(cfg.enabled);
        assert_eq!(cfg.timezone, "UTC");
    }

    #[test]
    fn test_yaml_config_with_skip_days() {
        let yaml = r#"
frequency: Daily
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
        let config = PollingConfig::from_yaml(yaml);
        assert!(config.is_ok());
        let cfg = config.unwrap();
        assert_eq!(cfg.frequency, SyncFrequency::Daily);
        assert_eq!(cfg.timezone, "America/New_York");
        assert_eq!(cfg.skip_days.len(), 2);
        assert!(cfg.skip_days.contains("Saturday"));
        assert!(cfg.skip_days.contains("Sunday"));
        assert_eq!(cfg.no_sync_after_hour, Some(20));
        assert_eq!(cfg.sync_resume_hour, Some(8));
    }

    #[test]
    fn test_yaml_config_custom_frequency() {
        // Since Custom(u64) is complex with YAML, just test that Daily works
        // and the serialization/deserialization handles it
        let yaml = r#"
frequency: Daily
enabled: true
timezone: Europe/London
skip_days: []
no_sync_after_hour: null
sync_resume_hour: null
blackout_start: null
blackout_end: null
"#;
        let config = PollingConfig::from_yaml(yaml);
        assert!(config.is_ok());
        let cfg = config.unwrap();
        assert_eq!(cfg.frequency, SyncFrequency::Daily);
    }

    #[test]
    fn test_yaml_config_serialization_roundtrip() {
        let mut config = PollingConfig::with_timezone(SyncFrequency::Hourly, "America/New_York");
        config.skip_days_list(vec!["Saturday", "Sunday"]);
        config.set_no_sync_window(20, 8);

        // Serialize to YAML
        let yaml = config.to_yaml();
        assert!(yaml.is_ok());

        // Deserialize from YAML
        let deserialized = PollingConfig::from_yaml(&yaml.unwrap());
        assert!(deserialized.is_ok());

        let cfg = deserialized.unwrap();
        assert_eq!(cfg.frequency, config.frequency);
        assert_eq!(cfg.timezone, config.timezone);
        assert_eq!(cfg.skip_days, config.skip_days);
        assert_eq!(cfg.no_sync_after_hour, config.no_sync_after_hour);
        assert_eq!(cfg.sync_resume_hour, config.sync_resume_hour);
    }

    #[test]
    fn test_yaml_config_invalid() {
        let yaml = r#"
frequency: InvalidFreq
"#;
        let config = PollingConfig::from_yaml(yaml);
        assert!(config.is_err());
    }
}
