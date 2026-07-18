/// Comprehensive sync configuration with detailed status messages
/// Handles separate source and destination polling configurations

use crate::sources::polling::PollingConfig;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Result of configuration validation with detailed messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationResult {
    /// Overall status: success or failure
    pub status: ConfigStatus,
    /// Congratulatory message on success or detailed error on failure
    pub message: String,
    /// Detailed breakdown of configuration validity
    pub details: ConfigurationDetails,
    /// Recommendations for fixing issues
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConfigStatus {
    /// All configurations validated successfully
    Success,
    /// Source polling configuration has issues
    SourceProblem,
    /// Destination polling configuration has issues
    DestinationProblem,
    /// Both source and destination have issues
    BothHaveProblem,
    /// Configuration incomplete or missing required fields
    Incomplete,
}

/// Detailed breakdown of what's configured and what's missing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationDetails {
    /// Source polling is configured
    pub source_polling_configured: bool,
    /// Destination polling is configured
    pub destination_polling_configured: bool,
    /// Source polling is valid (if configured)
    pub source_polling_valid: Option<bool>,
    /// Destination polling is valid (if configured)
    pub destination_polling_valid: Option<bool>,
    /// Source timezone is valid (if set)
    pub source_timezone_valid: Option<bool>,
    /// Destination timezone is valid (if set)
    pub destination_timezone_valid: Option<bool>,
    /// Source skip days are configured
    pub source_skip_days: usize,
    /// Destination skip days are configured
    pub destination_skip_days: usize,
    /// Source has time window restrictions
    pub source_has_time_window: bool,
    /// Destination has time window restrictions
    pub destination_has_time_window: bool,
    /// Source has blackout periods
    pub source_has_blackout: bool,
    /// Destination has blackout periods
    pub destination_has_blackout: bool,
}

impl ConfigurationDetails {
    pub fn new() -> Self {
        Self {
            source_polling_configured: false,
            destination_polling_configured: false,
            source_polling_valid: None,
            destination_polling_valid: None,
            source_timezone_valid: None,
            destination_timezone_valid: None,
            source_skip_days: 0,
            destination_skip_days: 0,
            source_has_time_window: false,
            destination_has_time_window: false,
            source_has_blackout: false,
            destination_has_blackout: false,
        }
    }
}

/// Complete sync configuration for source → destination sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfiguration {
    /// Configuration name (e.g., "kafka_to_postgres_sync")
    pub name: String,
    /// Source polling configuration
    pub source_polling: Option<PollingConfig>,
    /// Destination polling configuration (can differ from source)
    pub destination_polling: Option<PollingConfig>,
    /// Description of what this sync does
    pub description: Option<String>,
}

impl SyncConfiguration {
    /// Create new sync configuration
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            source_polling: None,
            destination_polling: None,
            description: None,
        }
    }

    /// Set source polling configuration
    pub fn with_source_polling(mut self, config: PollingConfig) -> Self {
        self.source_polling = Some(config);
        self
    }

    /// Set destination polling configuration (separate from source)
    pub fn with_destination_polling(mut self, config: PollingConfig) -> Self {
        self.destination_polling = Some(config);
        self
    }

    /// Set description
    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    /// Validate configuration and return detailed status
    pub fn validate(&self) -> ConfigurationResult {
        let mut details = ConfigurationDetails::new();
        let mut recommendations = Vec::new();

        // Validate source polling
        let source_ok = if let Some(source_cfg) = &self.source_polling {
            details.source_polling_configured = true;
            let tz_valid = source_cfg.validate_timezone().is_ok();
            details.source_timezone_valid = Some(tz_valid);
            details.source_polling_valid = Some(tz_valid);
            details.source_skip_days = source_cfg.skip_days.len();
            details.source_has_time_window =
                source_cfg.no_sync_after_hour.is_some() && source_cfg.sync_resume_hour.is_some();
            details.source_has_blackout =
                source_cfg.blackout_start.is_some() || source_cfg.blackout_end.is_some();

            if !tz_valid {
                recommendations.push(format!(
                    "⚠️  Source: Invalid timezone '{}'. Use IANA timezone names like 'America/New_York'",
                    source_cfg.timezone
                ));
            }

            tz_valid
        } else {
            false
        };

        // Validate destination polling
        let dest_ok = if let Some(dest_cfg) = &self.destination_polling {
            details.destination_polling_configured = true;
            let tz_valid = dest_cfg.validate_timezone().is_ok();
            details.destination_timezone_valid = Some(tz_valid);
            details.destination_polling_valid = Some(tz_valid);
            details.destination_skip_days = dest_cfg.skip_days.len();
            details.destination_has_time_window =
                dest_cfg.no_sync_after_hour.is_some() && dest_cfg.sync_resume_hour.is_some();
            details.destination_has_blackout =
                dest_cfg.blackout_start.is_some() || dest_cfg.blackout_end.is_some();

            if !tz_valid {
                recommendations.push(format!(
                    "⚠️  Destination: Invalid timezone '{}'. Use IANA timezone names like 'America/New_York'",
                    dest_cfg.timezone
                ));
            }

            tz_valid
        } else {
            // If destination is not configured, that's OK (only source polling)
            true
        };

        // Determine status
        let (status, message) = match (self.source_polling.is_some(), self.destination_polling.is_some(), source_ok, dest_ok) {
            // Both configured
            (true, true, true, true) => {
                let msg = self.success_message();
                (ConfigStatus::Success, msg)
            }
            (true, true, false, true) => {
                (ConfigStatus::SourceProblem, "❌ Source polling configuration has errors".to_string())
            }
            (true, true, true, false) => {
                (ConfigStatus::DestinationProblem, "❌ Destination polling configuration has errors".to_string())
            }
            (true, true, false, false) => {
                (ConfigStatus::BothHaveProblem, "❌ Both source and destination have configuration errors".to_string())
            }
            // Only source configured
            (true, false, true, _) => {
                let msg = self.success_message();
                (ConfigStatus::Success, msg)
            }
            (true, false, false, _) => {
                (ConfigStatus::SourceProblem, "❌ Source polling configuration has errors".to_string())
            }
            // Only destination configured
            (false, true, _, true) => {
                let msg = self.success_message();
                (ConfigStatus::Success, msg)
            }
            (false, true, _, false) => {
                (ConfigStatus::DestinationProblem, "❌ Destination polling configuration has errors".to_string())
            }
            // Neither configured
            (false, false, _, _) => {
                (ConfigStatus::Incomplete, "❌ At least one polling configuration (source or destination) is required".to_string())
            }
        };

        if status == ConfigStatus::Success && recommendations.is_empty() {
            // Remove duplicate "recommendations" when no actual issues
            recommendations.clear();
        }

        ConfigurationResult {
            status,
            message,
            details,
            recommendations,
        }
    }

    /// Generate congratulatory success message with details
    fn success_message(&self) -> String {
        let mut parts = vec![
            "✅ Configuration SUCCESSFUL!".to_string(),
            format!("   Sync: {}", self.name),
        ];

        if let Some(desc) = &self.description {
            parts.push(format!("   Purpose: {}", desc));
        }

        if let Some(source) = &self.source_polling {
            parts.push(format!("   📤 Source: {} polling in {} timezone", source.frequency.label(), source.timezone));
            if !source.skip_days.is_empty() {
                parts.push(format!("      Skip days: {}", source.skip_days.iter().cloned().collect::<Vec<_>>().join(", ")));
            }
            if source.no_sync_after_hour.is_some() && source.sync_resume_hour.is_some() {
                parts.push(format!(
                    "      No-sync window: {}:00 - {}:00",
                    source.no_sync_after_hour.unwrap(),
                    source.sync_resume_hour.unwrap()
                ));
            }
        } else {
            parts.push("   📤 Source: No polling configured (on-demand only)".to_string());
        }

        if let Some(dest) = &self.destination_polling {
            parts.push(format!(
                "   📥 Destination: {} polling in {} timezone",
                dest.frequency.label(),
                dest.timezone
            ));
            if !dest.skip_days.is_empty() {
                parts.push(format!("      Skip days: {}", dest.skip_days.iter().cloned().collect::<Vec<_>>().join(", ")));
            }
            if dest.no_sync_after_hour.is_some() && dest.sync_resume_hour.is_some() {
                parts.push(format!(
                    "      No-sync window: {}:00 - {}:00",
                    dest.no_sync_after_hour.unwrap(),
                    dest.sync_resume_hour.unwrap()
                ));
            }
        } else {
            parts.push("   📥 Destination: No polling configured (on-demand only)".to_string());
        }

        parts.join("\n")
    }

    /// Load from YAML file
    pub fn from_yaml_file(path: &str) -> Result<Self, String> {
        std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read sync config file: {}", e))
            .and_then(|content| Self::from_yaml(&content))
    }

    /// Load from YAML string
    pub fn from_yaml(yaml_str: &str) -> Result<Self, String> {
        serde_yaml::from_str(yaml_str).map_err(|e| format!("Failed to parse YAML sync config: {}", e))
    }

    /// Save to YAML file
    pub fn save_to_yaml_file(&self, path: &str) -> Result<(), String> {
        let yaml = self.to_yaml()?;
        std::fs::write(path, yaml).map_err(|e| format!("Failed to write sync config file: {}", e))
    }

    /// Convert to YAML string
    pub fn to_yaml(&self) -> Result<String, String> {
        serde_yaml::to_string(self).map_err(|e| format!("Failed to serialize to YAML: {}", e))
    }
}

impl fmt::Display for ConfigurationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;
        if !self.recommendations.is_empty() {
            write!(f, "\n\nRecommendations:")?;
            for rec in &self.recommendations {
                write!(f, "\n  {}", rec)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::polling::SyncFrequency;

    #[test]
    fn test_sync_config_success() {
        let source = PollingConfig::new(SyncFrequency::Hourly);
        let dest = PollingConfig::new(SyncFrequency::Daily);

        let config = SyncConfiguration::new("kafka_to_postgres")
            .with_source_polling(source)
            .with_destination_polling(dest)
            .with_description("Sync Kafka events to PostgreSQL database");

        let result = config.validate();
        assert_eq!(result.status, ConfigStatus::Success);
        assert!(result.message.contains("✅"));
        assert!(result.message.contains("kafka_to_postgres"));
    }

    #[test]
    fn test_sync_config_invalid_source_timezone() {
        let mut source = PollingConfig::new(SyncFrequency::Hourly);
        source.timezone = "Invalid/Timezone".to_string();
        let dest = PollingConfig::new(SyncFrequency::Daily);

        let config = SyncConfiguration::new("test_sync")
            .with_source_polling(source)
            .with_destination_polling(dest);

        let result = config.validate();
        assert_eq!(result.status, ConfigStatus::SourceProblem);
        assert!(!result.recommendations.is_empty());
    }

    #[test]
    fn test_sync_config_invalid_dest_timezone() {
        let source = PollingConfig::new(SyncFrequency::Hourly);
        let mut dest = PollingConfig::new(SyncFrequency::Daily);
        dest.timezone = "Bad/Zone".to_string();

        let config = SyncConfiguration::new("test_sync")
            .with_source_polling(source)
            .with_destination_polling(dest);

        let result = config.validate();
        assert_eq!(result.status, ConfigStatus::DestinationProblem);
    }

    #[test]
    fn test_sync_config_both_invalid() {
        let mut source = PollingConfig::new(SyncFrequency::Hourly);
        source.timezone = "Bad/Source".to_string();
        let mut dest = PollingConfig::new(SyncFrequency::Daily);
        dest.timezone = "Bad/Dest".to_string();

        let config = SyncConfiguration::new("test_sync")
            .with_source_polling(source)
            .with_destination_polling(dest);

        let result = config.validate();
        assert_eq!(result.status, ConfigStatus::BothHaveProblem);
    }

    #[test]
    fn test_sync_config_only_source() {
        let source = PollingConfig::new(SyncFrequency::Hourly);
        let config = SyncConfiguration::new("api_to_s3")
            .with_source_polling(source)
            .with_description("Sync API data to S3 bucket");

        let result = config.validate();
        assert_eq!(result.status, ConfigStatus::Success);
        assert!(result.details.source_polling_configured);
        assert!(!result.details.destination_polling_configured);
    }

    #[test]
    fn test_sync_config_yaml_roundtrip() {
        let source = PollingConfig::with_timezone(SyncFrequency::Hourly, "America/New_York");
        let dest = PollingConfig::with_timezone(SyncFrequency::Daily, "Europe/London");

        let config = SyncConfiguration::new("sync_1")
            .with_source_polling(source)
            .with_destination_polling(dest);

        let yaml = config.to_yaml().unwrap();
        let loaded = SyncConfiguration::from_yaml(&yaml).unwrap();

        assert_eq!(loaded.name, config.name);
        assert_eq!(
            loaded.source_polling.unwrap().timezone,
            config.source_polling.unwrap().timezone
        );
        assert_eq!(
            loaded.destination_polling.unwrap().timezone,
            config.destination_polling.unwrap().timezone
        );
    }
}
