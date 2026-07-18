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
    /// PySpark transformation is configured (optional)
    pub transformation_configured: bool,
    /// PySpark transformation has error handling configured
    pub transformation_has_error_handling: bool,
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
            transformation_configured: false,
            transformation_has_error_handling: false,
        }
    }
}

/// Type of transformation engine
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransformationEngine {
    /// PySpark for distributed transformations
    PySpark,
    /// Python for local/lightweight transformations
    Python,
}

impl std::fmt::Display for TransformationEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransformationEngine::PySpark => write!(f, "PySpark"),
            TransformationEngine::Python => write!(f, "Python"),
        }
    }
}

/// Transformation configuration supporting both PySpark and Python
/// PySpark: distributed, large-scale transformations
/// Python: local, lightweight transformations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformationConfig {
    /// Transformation is enabled
    pub enabled: bool,
    /// Type of transformation engine (PySpark or Python)
    pub engine: TransformationEngine,
    /// Script path or transformation logic
    pub script_path: String,
    /// Intermediate Kafka topic for staging (optional)
    pub intermediate_topic: Option<String>,
    /// Max retries on failure
    pub max_retries: u32,
    /// Retry delay in seconds
    pub retry_delay_secs: u32,
    /// Timeout for transformation in seconds
    pub timeout_secs: u32,
    /// Continue pipeline on transformation failure
    pub skip_on_error: bool,
    /// Dead letter topic for failed transformations
    pub dead_letter_topic: Option<String>,
    /// Enable result caching for fault tolerance (cache before sending to destination)
    pub enable_caching: bool,
    /// Cache directory path (for local or mounted filesystem)
    pub cache_dir: Option<String>,
    /// Maximum cache size in MB (for cleanup)
    pub max_cache_size_mb: Option<u32>,
}

impl TransformationConfig {
    /// Create new PySpark transformation config (default)
    pub fn new(script_path: &str) -> Self {
        Self::pyspark(script_path)
    }

    /// Create new PySpark transformation (distributed, high-volume)
    pub fn pyspark(script_path: &str) -> Self {
        Self {
            enabled: true,
            engine: TransformationEngine::PySpark,
            script_path: script_path.to_string(),
            intermediate_topic: None,
            max_retries: 3,
            retry_delay_secs: 5,
            timeout_secs: 300,
            skip_on_error: false,
            dead_letter_topic: None,
            enable_caching: false,
            cache_dir: None,
            max_cache_size_mb: None,
        }
    }

    /// Create new Python transformation (local, lightweight)
    pub fn python(script_path: &str) -> Self {
        Self {
            enabled: true,
            engine: TransformationEngine::Python,
            script_path: script_path.to_string(),
            intermediate_topic: None,
            max_retries: 3,
            retry_delay_secs: 5,
            timeout_secs: 60,  // Shorter timeout for local execution
            skip_on_error: false,
            dead_letter_topic: None,
            enable_caching: false,
            cache_dir: None,
            max_cache_size_mb: None,
        }
    }

    /// Set intermediate Kafka topic for staging
    pub fn with_intermediate_topic(mut self, topic: &str) -> Self {
        self.intermediate_topic = Some(topic.to_string());
        self
    }

    /// Set dead letter topic for failures
    pub fn with_dead_letter_topic(mut self, topic: &str) -> Self {
        self.dead_letter_topic = Some(topic.to_string());
        self
    }

    /// Set retry policy
    pub fn with_retries(mut self, max_retries: u32, retry_delay_secs: u32) -> Self {
        self.max_retries = max_retries;
        self.retry_delay_secs = retry_delay_secs;
        self
    }

    /// Allow pipeline to continue if transformation fails
    pub fn skip_on_error(mut self, skip: bool) -> Self {
        self.skip_on_error = skip;
        self
    }

    /// Enable caching for fault tolerance (cache results before sending to destination)
    pub fn with_caching(mut self, cache_dir: &str, max_size_mb: u32) -> Self {
        self.enable_caching = true;
        self.cache_dir = Some(cache_dir.to_string());
        self.max_cache_size_mb = Some(max_size_mb);
        self
    }
}

/// Complete sync configuration for source → [transformation] → destination sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfiguration {
    /// Configuration name (e.g., "kafka_to_postgres_sync")
    pub name: String,
    /// Source polling configuration
    pub source_polling: Option<PollingConfig>,
    /// Destination polling configuration (can differ from source)
    pub destination_polling: Option<PollingConfig>,
    /// Optional PySpark transformation (can be applied mid-pipeline)
    pub transformation: Option<TransformationConfig>,
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
            transformation: None,
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

    /// Set optional PySpark transformation (applied between source and destination)
    pub fn with_transformation(mut self, config: TransformationConfig) -> Self {
        self.transformation = Some(config);
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

        // Check transformation configuration
        if let Some(transformation) = &self.transformation {
            details.transformation_configured = true;
            details.transformation_has_error_handling = transformation.max_retries > 0
                || transformation.dead_letter_topic.is_some()
                || transformation.skip_on_error
                || transformation.enable_caching;

            // Validate transformation
            if transformation.intermediate_topic.is_none() && transformation.max_retries == 0 {
                recommendations.push(
                    "⚠️  Transformation: Consider adding intermediate_topic for staging or increase max_retries for reliability".to_string(),
                );
            }

            if transformation.dead_letter_topic.is_none() && !transformation.skip_on_error && !transformation.enable_caching {
                recommendations.push(
                    "⚠️  Transformation: Consider adding dead_letter_topic, enabling skip_on_error, or enabling caching for failure handling".to_string(),
                );
            }

            if transformation.enable_caching && transformation.cache_dir.is_none() {
                recommendations.push(
                    "⚠️  Transformation: Caching enabled but cache_dir not set. Provide cache directory path".to_string(),
                );
            }
        }

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

        if let Some(transform) = &self.transformation {
            parts.push(format!(
                "   ⚙️  Transformation: {} (engine: {}, retries: {}, timeout: {}s)",
                transform.script_path, transform.engine, transform.max_retries, transform.timeout_secs
            ));
            if let Some(intermediate) = &transform.intermediate_topic {
                parts.push(format!("      Staging topic: {}", intermediate));
            }
            if let Some(dlt) = &transform.dead_letter_topic {
                parts.push(format!("      Dead letter topic: {}", dlt));
            }
            if transform.enable_caching {
                parts.push(format!("      Caching: Enabled ({} MB max)", transform.max_cache_size_mb.unwrap_or(0)));
            }
            if transform.skip_on_error {
                parts.push("      Error handling: Skip on error (continue pipeline)".to_string());
            }
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

    #[test]
    fn test_sync_config_with_transformation() {
        let source = PollingConfig::new(SyncFrequency::Hourly);
        let transformation = TransformationConfig::new("transform.py")
            .with_intermediate_topic("transform_staging")
            .with_dead_letter_topic("transform_errors")
            .with_retries(5, 10)
            .with_caching("/var/cache/transforms", 1024);

        let config = SyncConfiguration::new("kafka_to_warehouse")
            .with_source_polling(source)
            .with_transformation(transformation)
            .with_description("Transform Kafka events to warehouse schema");

        let result = config.validate();
        assert_eq!(result.status, ConfigStatus::Success);
        assert!(result.details.transformation_configured);
        assert!(result.details.transformation_has_error_handling);
        assert!(result.message.contains("⚙️"));
    }

    #[test]
    fn test_transformation_config_fault_tolerance() {
        let transform = TransformationConfig::new("script.py")
            .with_intermediate_topic("staging")
            .with_dead_letter_topic("dead_letters")
            .with_retries(3, 5);

        assert_eq!(transform.max_retries, 3);
        assert_eq!(transform.retry_delay_secs, 5);
        assert!(transform.intermediate_topic.is_some());
        assert!(transform.dead_letter_topic.is_some());
        assert!(!transform.enable_caching);
    }

    #[test]
    fn test_transformation_config_with_caching() {
        let transform = TransformationConfig::new("script.py")
            .with_caching("/tmp/cache", 512);

        assert!(transform.enable_caching);
        assert_eq!(transform.cache_dir, Some("/tmp/cache".to_string()));
        assert_eq!(transform.max_cache_size_mb, Some(512));
    }

    #[test]
    fn test_transformation_python_engine() {
        let transform = TransformationConfig::python("transform.py");

        assert_eq!(transform.engine, TransformationEngine::Python);
        assert!(transform.enabled);
        assert_eq!(transform.timeout_secs, 60);  // Shorter timeout for Python
        assert_eq!(transform.script_path, "transform.py");
    }

    #[test]
    fn test_transformation_pyspark_engine() {
        let transform = TransformationConfig::pyspark("transform.py");

        assert_eq!(transform.engine, TransformationEngine::PySpark);
        assert!(transform.enabled);
        assert_eq!(transform.timeout_secs, 300);  // Longer timeout for PySpark
    }

    #[test]
    fn test_sync_config_with_python_transformation() {
        let source = PollingConfig::new(SyncFrequency::Hourly);
        let transform = TransformationConfig::python("simple_transform.py")
            .with_retries(2, 3)
            .with_dead_letter_topic("errors");

        let config = SyncConfiguration::new("api_to_warehouse")
            .with_source_polling(source)
            .with_transformation(transform)
            .with_description("Lightweight Python transformation");

        let result = config.validate();
        assert_eq!(result.status, ConfigStatus::Success);
        assert!(result.message.contains("Python"));
    }

    #[test]
    fn test_sync_config_with_pyspark_transformation() {
        let source = PollingConfig::new(SyncFrequency::FiveMinutes);
        let transform = TransformationConfig::pyspark("transform.py")
            .with_intermediate_topic("staging")
            .with_retries(5, 10);

        let config = SyncConfiguration::new("kafka_to_warehouse")
            .with_source_polling(source)
            .with_transformation(transform);

        let result = config.validate();
        assert_eq!(result.status, ConfigStatus::Success);
        assert!(result.message.contains("PySpark"));
    }
}
