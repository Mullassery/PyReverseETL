use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

/// PySpark auto-scaling policy
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ScalingPolicy {
    /// No auto-scaling (static configuration)
    Static,
    /// Scale based on input data size
    DataSize,
    /// Scale based on processing latency
    Latency,
    /// Scale based on CPU/memory usage
    ResourceUtilization,
    /// Aggressive scaling for batch jobs
    Aggressive,
}

/// PySpark transformer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparkConfig {
    /// Spark application name
    pub app_name: String,
    /// Spark master URL (local[*], yarn, k8s, etc.)
    pub master: String,
    /// PySpark script path or code
    pub script: String,
    /// Input Kafka topic
    pub input_topic: String,
    /// Output Kafka topic
    pub output_topic: String,
    /// Kafka broker addresses
    pub kafka_brokers: String,
    /// Number of partitions
    pub num_partitions: u32,
    /// Batch interval in seconds
    pub batch_interval_seconds: u32,
    /// Driver memory (e.g., "4g")
    pub driver_memory: String,
    /// Executor memory (e.g., "2g")
    pub executor_memory: String,
    /// Number of executors (initial)
    pub num_executors: u32,
    /// Min executors for auto-scaling
    pub min_executors: u32,
    /// Max executors for auto-scaling
    pub max_executors: u32,
    /// Auto-scaling policy
    pub scaling_policy: ScalingPolicy,
    /// Auto-shutdown after completion (seconds)
    pub auto_shutdown_seconds: Option<u32>,
    /// Checkpointing directory
    pub checkpoint_dir: Option<String>,
    /// Additional Spark configurations
    pub spark_conf: HashMap<String, String>,
    /// Custom parameters for PySpark script
    pub script_parameters: HashMap<String, String>,
}

impl Default for SparkConfig {
    fn default() -> Self {
        Self {
            app_name: "pyreverseetl-spark".to_string(),
            master: "local[*]".to_string(),
            script: String::new(),
            input_topic: "events".to_string(),
            output_topic: "transformed-events".to_string(),
            kafka_brokers: "localhost:9092".to_string(),
            num_partitions: 4,
            batch_interval_seconds: 5,
            driver_memory: "2g".to_string(),
            executor_memory: "2g".to_string(),
            num_executors: 2,
            min_executors: 1,
            max_executors: 16,
            scaling_policy: ScalingPolicy::DataSize,
            auto_shutdown_seconds: Some(300), // 5 min auto-shutdown
            checkpoint_dir: None,
            spark_conf: HashMap::new(),
            script_parameters: HashMap::new(),
        }
    }
}

/// PySpark job submission result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparkJobResult {
    pub job_id: String,
    pub status: String,
    pub records_processed: u64,
    pub records_output: u64,
    pub execution_time_ms: u64,
    pub error_message: Option<String>,
}

/// PySpark transformer
pub struct SparkTransformer {
    config: SparkConfig,
    job_id: String,
}

impl SparkTransformer {
    /// Create new PySpark transformer
    pub fn new(config: SparkConfig) -> Self {
        let job_id = format!(
            "spark-{}-{}",
            chrono::Utc::now().timestamp(),
            uuid::Uuid::new_v4().to_string()[..8].to_string()
        );

        Self { config, job_id }
    }

    /// Get job ID
    pub fn job_id(&self) -> &str {
        &self.job_id
    }

    /// Build Spark submit command
    pub fn build_spark_submit_command(&self) -> String {
        let mut cmd = String::from("spark-submit");

        // Master
        cmd.push_str(&format!(" --master {}", self.config.master));

        // Application name
        cmd.push_str(&format!(" --name {}", self.config.app_name));

        // Memory settings
        cmd.push_str(&format!(" --driver-memory {}", self.config.driver_memory));
        cmd.push_str(&format!(" --executor-memory {}", self.config.executor_memory));

        // Number of executors (if not local)
        if !self.config.master.starts_with("local") {
            cmd.push_str(&format!(" --num-executors {}", self.config.num_executors));
        }

        // Custom Spark configurations
        for (key, value) in &self.config.spark_conf {
            cmd.push_str(&format!(" --conf {}={}", key, value));
        }

        // Add Kafka broker configuration
        cmd.push_str(&format!(
            " --conf spark.kafka.bootstrap.servers={}",
            self.config.kafka_brokers
        ));

        // Add script
        cmd.push_str(&format!(" {}", self.config.script));

        // Add script parameters
        cmd.push_str(&format!(" --input-topic {}", self.config.input_topic));
        cmd.push_str(&format!(" --output-topic {}", self.config.output_topic));
        cmd.push_str(&format!(
            " --batch-interval {}",
            self.config.batch_interval_seconds
        ));
        cmd.push_str(&format!(
            " --num-partitions {}",
            self.config.num_partitions
        ));

        if let Some(checkpoint_dir) = &self.config.checkpoint_dir {
            cmd.push_str(&format!(" --checkpoint-dir {}", checkpoint_dir));
        }

        // Custom parameters
        for (key, value) in &self.config.script_parameters {
            cmd.push_str(&format!(" --param-{} {}", key, value));
        }

        cmd
    }

    /// Simulate job execution (for testing/local use)
    pub fn execute_local(&self) -> crate::Result<SparkJobResult> {
        let start = Instant::now();

        // Simulate transformation: 1000 input records → 950 output records (5% filtered)
        let records_processed = 1000;
        let records_output = (records_processed as f64 * 0.95) as u64;
        let execution_time_ms = 5000;

        Ok(SparkJobResult {
            job_id: self.job_id.clone(),
            status: "SUCCEEDED".to_string(),
            records_processed,
            records_output,
            execution_time_ms,
            error_message: None,
        })
    }

    /// Submit job to Spark cluster
    pub async fn submit(&self) -> crate::Result<SparkJobResult> {
        // For now, use local execution
        // In production, this would use spark-submit or Spark REST API
        self.execute_local()
    }

    /// Get Spark configuration
    pub fn config(&self) -> &SparkConfig {
        &self.config
    }

    /// Update configuration
    pub fn set_config(&mut self, config: SparkConfig) {
        self.config = config;
    }

    /// Calculate optimal executor count based on data size
    pub fn calculate_executors_for_data_size(&self, data_size_mb: u64) -> u32 {
        let scale_factor = (data_size_mb / 1000).max(1); // 1 executor per 1GB
        ((scale_factor as u32) * self.config.num_executors)
            .min(self.config.max_executors)
            .max(self.config.min_executors)
    }

    /// Calculate optimal executor count based on latency target
    pub fn calculate_executors_for_latency(&self, current_latency_ms: u64, target_latency_ms: u64) -> u32 {
        if current_latency_ms <= target_latency_ms {
            return self.config.num_executors; // No scaling needed
        }

        let ratio = current_latency_ms as f64 / target_latency_ms as f64;
        let new_executors = (self.config.num_executors as f64 * ratio).ceil() as u32;
        new_executors
            .min(self.config.max_executors)
            .max(self.config.min_executors)
    }

    /// Auto-scale executors based on policy
    pub fn get_auto_scaled_executor_count(
        &self,
        data_size_mb: Option<u64>,
        current_latency_ms: Option<u64>,
    ) -> u32 {
        match self.config.scaling_policy {
            ScalingPolicy::Static => self.config.num_executors,
            ScalingPolicy::DataSize => {
                data_size_mb.map(|size| self.calculate_executors_for_data_size(size))
                    .unwrap_or(self.config.num_executors)
            }
            ScalingPolicy::Latency => {
                current_latency_ms.map(|latency| self.calculate_executors_for_latency(latency, 5000))
                    .unwrap_or(self.config.num_executors)
            }
            ScalingPolicy::ResourceUtilization => {
                // Default to 70% of max for balanced resource utilization
                ((self.config.max_executors as f64 * 0.7) as u32)
                    .min(self.config.max_executors)
                    .max(self.config.min_executors)
            }
            ScalingPolicy::Aggressive => {
                // Use max executors for aggressive parallel processing
                self.config.max_executors
            }
        }
    }

    /// Build Spark submit command with auto-scaled executors
    pub fn build_spark_submit_command_with_scaling(
        &self,
        data_size_mb: Option<u64>,
        current_latency_ms: Option<u64>,
    ) -> String {
        let scaled_executors = self.get_auto_scaled_executor_count(data_size_mb, current_latency_ms);

        let mut cmd = String::from("spark-submit");
        cmd.push_str(&format!(" --master {}", self.config.master));
        cmd.push_str(&format!(" --name {}", self.config.app_name));
        cmd.push_str(&format!(" --driver-memory {}", self.config.driver_memory));
        cmd.push_str(&format!(" --executor-memory {}", self.config.executor_memory));

        if !self.config.master.starts_with("local") {
            cmd.push_str(&format!(" --num-executors {}", scaled_executors));

            // Add dynamic allocation for YARN/K8s
            if self.config.master == "yarn" || self.config.master.contains("k8s") {
                cmd.push_str(&format!(" --conf spark.dynamicAllocation.minExecutors={}", self.config.min_executors));
                cmd.push_str(&format!(" --conf spark.dynamicAllocation.maxExecutors={}", self.config.max_executors));
                cmd.push_str(" --conf spark.dynamicAllocation.enabled=true");
            }

            // Add auto-shutdown
            if let Some(shutdown_secs) = self.config.auto_shutdown_seconds {
                cmd.push_str(&format!(" --conf spark.dynamicAllocation.executorIdleTimeout={}s", shutdown_secs));
            }
        }

        for (key, value) in &self.config.spark_conf {
            cmd.push_str(&format!(" --conf {}={}", key, value));
        }

        cmd.push_str(&format!(
            " --conf spark.kafka.bootstrap.servers={}",
            self.config.kafka_brokers
        ));

        cmd.push_str(&format!(" {}", self.config.script));
        cmd.push_str(&format!(" --input-topic {}", self.config.input_topic));
        cmd.push_str(&format!(" --output-topic {}", self.config.output_topic));
        cmd.push_str(&format!(" --batch-interval {}", self.config.batch_interval_seconds));
        cmd.push_str(&format!(" --num-partitions {}", self.config.num_partitions));

        if let Some(checkpoint_dir) = &self.config.checkpoint_dir {
            cmd.push_str(&format!(" --checkpoint-dir {}", checkpoint_dir));
        }

        for (key, value) in &self.config.script_parameters {
            cmd.push_str(&format!(" --param-{} {}", key, value));
        }

        cmd
    }
}

impl super::Transformer for SparkTransformer {
    fn name(&self) -> &str {
        &self.config.app_name
    }

    fn transformation_type(&self) -> &str {
        "spark"
    }

    fn execute(&self) -> crate::Result<super::TransformationResult> {
        let result = self.execute_local()?;

        Ok(super::TransformationResult {
            status: super::TransformationStatus::Completed,
            records_processed: result.records_processed,
            records_output: result.records_output,
            error: result.error_message,
            execution_time_ms: result.execution_time_ms,
            output_topic: self.config.output_topic.clone(),
        })
    }

    fn validate(&self) -> crate::Result<()> {
        if self.config.script.is_empty() {
            return Err(crate::Error::ConnectorError(
                "PySpark script path/code cannot be empty".to_string(),
            ));
        }

        if self.config.input_topic.is_empty() {
            return Err(crate::Error::ConnectorError(
                "Input topic cannot be empty".to_string(),
            ));
        }

        if self.config.output_topic.is_empty() {
            return Err(crate::Error::ConnectorError(
                "Output topic cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    fn config(&self) -> super::TransformationConfig {
        super::TransformationConfig {
            name: self.config.app_name.clone(),
            description: None,
            input_topic: self.config.input_topic.clone(),
            output_topic: self.config.output_topic.clone(),
            transformation_type: "spark".to_string(),
            parameters: self.config.script_parameters.clone(),
            enabled: true,
            timeout_seconds: 300,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::Transformer;

    #[test]
    fn test_spark_config_default() {
        let config = SparkConfig::default();
        assert_eq!(config.app_name, "pyreverseetl-spark");
        assert_eq!(config.master, "local[*]");
        assert_eq!(config.driver_memory, "2g");
        assert_eq!(config.executor_memory, "2g");
    }

    #[test]
    fn test_spark_transformer_creation() {
        let config = SparkConfig {
            script: "/path/to/transform.py".to_string(),
            input_topic: "raw".to_string(),
            output_topic: "transformed".to_string(),
            ..Default::default()
        };

        let transformer = SparkTransformer::new(config);
        assert_eq!(transformer.name(), "pyreverseetl-spark");
        assert_eq!(transformer.transformation_type(), "spark");
    }

    #[test]
    fn test_spark_job_id_generation() {
        let config = SparkConfig::default();
        let t1 = SparkTransformer::new(config.clone());
        let t2 = SparkTransformer::new(config);

        assert_ne!(t1.job_id(), t2.job_id());
        assert!(t1.job_id().starts_with("spark-"));
    }

    #[test]
    fn test_spark_submit_command_local() {
        let config = SparkConfig {
            app_name: "test-app".to_string(),
            master: "local[*]".to_string(),
            script: "transform.py".to_string(),
            input_topic: "input".to_string(),
            output_topic: "output".to_string(),
            ..Default::default()
        };

        let transformer = SparkTransformer::new(config);
        let cmd = transformer.build_spark_submit_command();

        assert!(cmd.contains("spark-submit"));
        assert!(cmd.contains("--master local[*]"));
        assert!(cmd.contains("--name test-app"));
        assert!(cmd.contains("transform.py"));
        assert!(cmd.contains("--input-topic input"));
        assert!(cmd.contains("--output-topic output"));
    }

    #[test]
    fn test_spark_submit_command_cluster() {
        let config = SparkConfig {
            master: "yarn".to_string(),
            num_executors: 4,
            script: "transform.py".to_string(),
            ..Default::default()
        };

        let transformer = SparkTransformer::new(config);
        let cmd = transformer.build_spark_submit_command();

        assert!(cmd.contains("--master yarn"));
        assert!(cmd.contains("--num-executors 4"));
    }

    #[test]
    fn test_spark_transformer_validation() {
        let config = SparkConfig {
            script: "/path/to/script.py".to_string(),
            input_topic: "input".to_string(),
            output_topic: "output".to_string(),
            ..Default::default()
        };

        let transformer = SparkTransformer::new(config);
        assert!(transformer.validate().is_ok());
    }

    #[test]
    fn test_spark_transformer_validation_missing_script() {
        let config = SparkConfig {
            script: String::new(),
            ..Default::default()
        };

        let transformer = SparkTransformer::new(config);
        assert!(transformer.validate().is_err());
    }

    #[test]
    fn test_spark_transformer_execute_local() {
        let config = SparkConfig {
            script: "transform.py".to_string(),
            ..Default::default()
        };

        let transformer = SparkTransformer::new(config);
        let result = transformer.execute_local().unwrap();

        assert_eq!(result.status, "SUCCEEDED");
        assert_eq!(result.records_processed, 1000);
        assert_eq!(result.records_output, 950);
    }

    #[test]
    fn test_spark_transformer_implement_trait() {
        let config = SparkConfig {
            script: "transform.py".to_string(),
            ..Default::default()
        };

        let transformer = SparkTransformer::new(config);
        let result = transformer.execute().unwrap();

        assert_eq!(result.status, super::super::TransformationStatus::Completed);
        assert_eq!(result.records_processed, 1000);
    }

    #[test]
    fn test_spark_config_with_custom_params() {
        let mut config = SparkConfig::default();
        config.script_parameters.insert("version".to_string(), "v1".to_string());
        config.script_parameters.insert("env".to_string(), "prod".to_string());

        let transformer = SparkTransformer::new(config);
        let cmd = transformer.build_spark_submit_command();

        assert!(cmd.contains("--param-version v1"));
        assert!(cmd.contains("--param-env prod"));
    }

    #[test]
    fn test_spark_auto_scaling_data_size() {
        let config = SparkConfig {
            max_executors: 16,
            min_executors: 1,
            num_executors: 2,
            ..Default::default()
        };

        let transformer = SparkTransformer::new(config);

        // 1GB data → 2 executors
        assert_eq!(transformer.calculate_executors_for_data_size(1000), 2);

        // 5GB data → 10 executors (capped at max 16)
        assert_eq!(transformer.calculate_executors_for_data_size(5000), 10);

        // 20GB data → 16 executors (max capped)
        assert_eq!(transformer.calculate_executors_for_data_size(20000), 16);
    }

    #[test]
    fn test_spark_auto_scaling_latency() {
        let config = SparkConfig {
            max_executors: 16,
            min_executors: 1,
            num_executors: 2,
            ..Default::default()
        };

        let transformer = SparkTransformer::new(config);

        // Current latency = target latency → no scaling
        assert_eq!(transformer.calculate_executors_for_latency(5000, 5000), 2);

        // Current latency 10s, target 5s → scale up 2x
        let scaled = transformer.calculate_executors_for_latency(10000, 5000);
        assert!(scaled >= 4 && scaled <= 5);

        // Current latency 1s, target 5s → no need to scale
        assert_eq!(transformer.calculate_executors_for_latency(1000, 5000), 2);
    }

    #[test]
    fn test_spark_scaling_policy_static() {
        let mut config = SparkConfig::default();
        config.scaling_policy = ScalingPolicy::Static;
        let transformer = SparkTransformer::new(config);

        assert_eq!(
            transformer.get_auto_scaled_executor_count(Some(5000), None),
            2 // Original num_executors
        );
    }

    #[test]
    fn test_spark_scaling_policy_aggressive() {
        let mut config = SparkConfig::default();
        config.scaling_policy = ScalingPolicy::Aggressive;
        config.max_executors = 16;
        let transformer = SparkTransformer::new(config);

        assert_eq!(transformer.get_auto_scaled_executor_count(None, None), 16);
    }

    #[test]
    fn test_spark_auto_shutdown_config() {
        let config = SparkConfig {
            auto_shutdown_seconds: Some(300),
            ..Default::default()
        };

        assert_eq!(config.auto_shutdown_seconds, Some(300));
    }

    #[test]
    fn test_spark_submit_with_auto_scaling() {
        let config = SparkConfig {
            master: "yarn".to_string(),
            max_executors: 16,
            min_executors: 2,
            auto_shutdown_seconds: Some(300),
            ..Default::default()
        };

        let transformer = SparkTransformer::new(config);
        let cmd = transformer.build_spark_submit_command_with_scaling(Some(5000), None);

        assert!(cmd.contains("--master yarn"));
        assert!(cmd.contains("spark.dynamicAllocation.enabled=true"));
        assert!(cmd.contains("spark.dynamicAllocation.minExecutors=2"));
        assert!(cmd.contains("spark.dynamicAllocation.maxExecutors=16"));
        assert!(cmd.contains("executorIdleTimeout=300s"));
    }
}
