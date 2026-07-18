use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
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
    /// Create new polling config with frequency
    pub fn new(frequency: SyncFrequency) -> Self {
        Self {
            frequency,
            enabled: true,
            last_poll_at: None,
            last_change_at: None,
            poll_count: 0,
            change_count: 0,
        }
    }

    /// Check if it's time to poll
    pub fn should_poll(&self) -> bool {
        if !self.enabled {
            return false;
        }

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
}
