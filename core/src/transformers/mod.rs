pub mod spark;

pub use spark::{SparkTransformer, SparkConfig, SparkJobResult};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Transformation status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransformationStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

/// Transformation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformationConfig {
    /// Transformation name/ID
    pub name: String,
    /// Description of the transformation
    pub description: Option<String>,
    /// Input Kafka topic
    pub input_topic: String,
    /// Intermediate topic for transformed data
    pub output_topic: String,
    /// Type of transformation (spark, sql, python, etc.)
    pub transformation_type: String,
    /// Transformation parameters
    pub parameters: HashMap<String, String>,
    /// Whether transformation is enabled
    pub enabled: bool,
    /// Timeout in seconds
    pub timeout_seconds: u64,
}

impl Default for TransformationConfig {
    fn default() -> Self {
        Self {
            name: "default-transformation".to_string(),
            description: None,
            input_topic: "events".to_string(),
            output_topic: "transformed-events".to_string(),
            transformation_type: "spark".to_string(),
            parameters: HashMap::new(),
            enabled: true,
            timeout_seconds: 300,
        }
    }
}

/// Transformation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformationResult {
    pub status: TransformationStatus,
    pub records_processed: u64,
    pub records_output: u64,
    pub error: Option<String>,
    pub execution_time_ms: u64,
    pub output_topic: String,
}

/// Trait for transformation engines
pub trait Transformer: Send + Sync {
    /// Get transformation name
    fn name(&self) -> &str;

    /// Get transformation type
    fn transformation_type(&self) -> &str;

    /// Execute transformation
    fn execute(&self) -> crate::Result<TransformationResult>;

    /// Validate transformation config
    fn validate(&self) -> crate::Result<()>;

    /// Get config
    fn config(&self) -> TransformationConfig;
}

/// Transformation pipeline orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformationPipeline {
    pub stages: Vec<TransformationStage>,
    pub description: Option<String>,
    pub enabled: bool,
}

/// Single stage in transformation pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformationStage {
    pub name: String,
    pub config: TransformationConfig,
    pub retry_count: u32,
    pub skip_on_error: bool,
}

impl TransformationPipeline {
    /// Create new pipeline
    pub fn new() -> Self {
        Self {
            stages: Vec::new(),
            description: None,
            enabled: true,
        }
    }

    /// Add transformation stage
    pub fn add_stage(mut self, stage: TransformationStage) -> Self {
        self.stages.push(stage);
        self
    }

    /// Get number of stages
    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }

    /// Get input topic (from first stage)
    pub fn input_topic(&self) -> Option<&str> {
        self.stages.first().map(|s| s.config.input_topic.as_str())
    }

    /// Get output topic (from last stage)
    pub fn output_topic(&self) -> Option<&str> {
        self.stages.last().map(|s| s.config.output_topic.as_str())
    }
}

impl Default for TransformationPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transformation_config_default() {
        let config = TransformationConfig::default();
        assert_eq!(config.name, "default-transformation");
        assert_eq!(config.input_topic, "events");
        assert_eq!(config.transformation_type, "spark");
        assert!(config.enabled);
    }

    #[test]
    fn test_transformation_result_creation() {
        let result = TransformationResult {
            status: TransformationStatus::Completed,
            records_processed: 1000,
            records_output: 950,
            error: None,
            execution_time_ms: 5000,
            output_topic: "transformed".to_string(),
        };

        assert_eq!(result.status, TransformationStatus::Completed);
        assert_eq!(result.records_processed, 1000);
    }

    #[test]
    fn test_transformation_pipeline_creation() {
        let pipeline = TransformationPipeline::new();
        assert_eq!(pipeline.stage_count(), 0);
        assert!(pipeline.enabled);
    }

    #[test]
    fn test_transformation_pipeline_add_stage() {
        let stage = TransformationStage {
            name: "stage1".to_string(),
            config: TransformationConfig::default(),
            retry_count: 3,
            skip_on_error: false,
        };

        let pipeline = TransformationPipeline::new().add_stage(stage);
        assert_eq!(pipeline.stage_count(), 1);
        assert_eq!(pipeline.stages[0].name, "stage1");
    }

    #[test]
    fn test_transformation_pipeline_topics() {
        let mut config = TransformationConfig::default();
        config.input_topic = "raw-events".to_string();
        config.output_topic = "transformed-events".to_string();

        let stage = TransformationStage {
            name: "transform".to_string(),
            config,
            retry_count: 1,
            skip_on_error: false,
        };

        let pipeline = TransformationPipeline::new().add_stage(stage);
        assert_eq!(pipeline.input_topic(), Some("raw-events"));
        assert_eq!(pipeline.output_topic(), Some("transformed-events"));
    }

    #[test]
    fn test_transformation_pipeline_multiple_stages() {
        let stage1 = TransformationStage {
            name: "stage1".to_string(),
            config: TransformationConfig {
                input_topic: "events".to_string(),
                output_topic: "normalized".to_string(),
                ..Default::default()
            },
            retry_count: 1,
            skip_on_error: false,
        };

        let stage2 = TransformationStage {
            name: "stage2".to_string(),
            config: TransformationConfig {
                input_topic: "normalized".to_string(),
                output_topic: "enriched".to_string(),
                ..Default::default()
            },
            retry_count: 1,
            skip_on_error: false,
        };

        let pipeline = TransformationPipeline::new()
            .add_stage(stage1)
            .add_stage(stage2);

        assert_eq!(pipeline.stage_count(), 2);
        assert_eq!(pipeline.input_topic(), Some("events"));
        assert_eq!(pipeline.output_topic(), Some("enriched"));
    }

    #[test]
    fn test_transformation_status_values() {
        assert_ne!(TransformationStatus::Completed, TransformationStatus::Failed);
        assert_ne!(TransformationStatus::Running, TransformationStatus::Pending);
        assert_eq!(TransformationStatus::Skipped, TransformationStatus::Skipped);
    }
}
