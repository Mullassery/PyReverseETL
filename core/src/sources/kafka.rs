use crate::{Event, EventSource, EventType};
use rdkafka::consumer::{Consumer, BaseConsumer};
use rdkafka::message::Message;
use rdkafka::ClientConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

/// Kafka message wrapper for deserialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaMessage {
    pub key: Option<String>,
    pub value: serde_json::Value,
    pub headers: HashMap<String, String>,
    pub partition: i32,
    pub offset: i64,
}

/// Kafka source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaConfig {
    /// Kafka broker addresses (comma-separated)
    pub brokers: String,
    /// Topic to consume from
    pub topic: String,
    /// Consumer group ID
    pub group_id: String,
    /// Starting offset: "earliest" or "latest"
    pub auto_offset_reset: String,
    /// Session timeout (seconds)
    pub session_timeout_sec: u64,
    /// Max bytes to fetch per request
    pub max_bytes: usize,
    /// Enable SSL
    pub use_ssl: bool,
    /// SASL mechanism (plain, scram-sha-256, scram-sha-512)
    pub sasl_mechanism: Option<String>,
    /// SASL username
    pub sasl_username: Option<String>,
    /// SASL password
    pub sasl_password: Option<String>,
}

impl Default for KafkaConfig {
    fn default() -> Self {
        Self {
            brokers: "localhost:9092".to_string(),
            topic: "events".to_string(),
            group_id: format!("pyreverseetl-{}", Uuid::new_v4().to_string()),
            auto_offset_reset: "latest".to_string(),
            session_timeout_sec: 30,
            max_bytes: 1_048_576, // 1 MB
            use_ssl: false,
            sasl_mechanism: None,
            sasl_username: None,
            sasl_password: None,
        }
    }
}

/// Kafka event source connector with auto-scaling support
pub struct KafkaSource {
    config: KafkaConfig,
    consumer: Option<BaseConsumer>,
    connected: bool,
    message_count: u64,
    polling_config: super::polling::PollingConfig,
    consumer_lag: u64,
    last_message_time: Option<std::time::Instant>,
    start_time: std::time::Instant,
}

impl KafkaSource {
    /// Create a new Kafka source
    pub fn new(config: KafkaConfig) -> Self {
        Self {
            config,
            consumer: None,
            connected: false,
            message_count: 0,
            polling_config: super::polling::PollingConfig::new(super::polling::SyncFrequency::Hourly),
            consumer_lag: 0,
            last_message_time: None,
            start_time: std::time::Instant::now(),
        }
    }

    /// Create with polling configuration
    pub fn with_polling(config: KafkaConfig, polling: super::polling::PollingConfig) -> Self {
        Self {
            config,
            consumer: None,
            connected: false,
            message_count: 0,
            polling_config: polling,
            consumer_lag: 0,
            last_message_time: None,
            start_time: std::time::Instant::now(),
        }
    }

    /// Calculate current throughput in messages/second
    pub fn calculate_throughput(&self) -> f64 {
        let elapsed_secs = self.start_time.elapsed().as_secs_f64().max(1.0);
        self.message_count as f64 / elapsed_secs
    }

    /// Calculate recommended parallelism based on consumer lag and throughput
    pub fn calculate_recommended_parallelism(&self) -> u32 {
        let throughput = self.calculate_throughput();

        // Target: 1000 msgs/sec per partition
        // Adjust based on lag: high lag = more partitions needed
        let partition_target = 1000.0;
        let throughput_multiplier = (throughput / partition_target).max(1.0).ceil() as u32;

        // Lag penalty: 10ms lag per 100 messages behind
        let lag_multiplier = ((self.consumer_lag / 100).max(1) as u32).min(4);

        let recommended = throughput_multiplier.saturating_mul(lag_multiplier);
        recommended.max(1).min(32) // Reasonable bounds: 1-32 partitions
    }

    /// Update consumer lag
    pub fn set_consumer_lag(&mut self, lag: u64) {
        self.consumer_lag = lag;
    }

    /// Get metrics with auto-scaling info
    pub fn metrics(&self) -> KafkaSourceMetrics {
        let throughput = self.calculate_throughput();
        let recommended_parallelism = self.calculate_recommended_parallelism();

        KafkaSourceMetrics {
            messages_consumed: self.message_count,
            connected: self.connected,
            consumer_lag: self.consumer_lag,
            current_throughput_msgs_sec: throughput,
            recommended_parallelism,
        }
    }

    /// Set sync frequency for polling
    pub fn set_sync_frequency(&mut self, frequency: super::polling::SyncFrequency) {
        self.polling_config.frequency = frequency;
    }

    /// Get polling metrics
    pub fn polling_metrics(&self) -> super::polling::PollingMetrics {
        self.polling_config.metrics()
    }

    /// Check if should poll for changes
    pub fn should_poll(&self) -> bool {
        self.polling_config.should_poll()
    }
}

/// Kafka source metrics with auto-scaling info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaSourceMetrics {
    pub messages_consumed: u64,
    pub connected: bool,
    pub consumer_lag: u64,
    pub current_throughput_msgs_sec: f64,
    pub recommended_parallelism: u32,
}

impl super::EventSourceConnector for KafkaSource {
    fn name(&self) -> &str {
        "kafka"
    }

    fn source_type(&self) -> &str {
        "kafka"
    }

    fn connect(&mut self) -> crate::Result<()> {
        let mut client_config = ClientConfig::new();

        client_config
            .set("bootstrap.servers", &self.config.brokers)
            .set("group.id", &self.config.group_id)
            .set("auto.offset.reset", &self.config.auto_offset_reset)
            .set("session.timeout.ms", (self.config.session_timeout_sec * 1000).to_string())
            .set("fetch.max.bytes", self.config.max_bytes.to_string());

        // Configure SSL if enabled
        if self.config.use_ssl {
            client_config.set("security.protocol", "ssl");
        }

        // Configure SASL if provided
        if let (Some(mechanism), Some(username), Some(password)) = (
            &self.config.sasl_mechanism,
            &self.config.sasl_username,
            &self.config.sasl_password,
        ) {
            client_config
                .set("security.protocol", "sasl_ssl")
                .set("sasl.mechanism", mechanism)
                .set("sasl.username", username)
                .set("sasl.password", password);
        }

        let consumer: BaseConsumer = client_config
            .create()
            .map_err(|e| crate::Error::ConnectorError(format!("Failed to create Kafka consumer: {}", e)))?;

        // Subscribe to topic
        consumer
            .subscribe(&[&self.config.topic])
            .map_err(|e| crate::Error::ConnectorError(format!("Failed to subscribe to topic: {}", e)))?;

        self.consumer = Some(consumer);
        self.connected = true;

        Ok(())
    }

    fn disconnect(&mut self) -> crate::Result<()> {
        self.consumer = None;
        self.connected = false;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected && self.consumer.is_some()
    }

    fn next_event(&mut self) -> crate::Result<Option<Event>> {
        let consumer = self
            .consumer
            .as_ref()
            .ok_or_else(|| crate::Error::ConnectorError("Not connected to Kafka".to_string()))?;

        match consumer.poll(Duration::from_millis(100)) {
            Some(Ok(borrowed_message)) => {
                self.message_count += 1;

                let key = borrowed_message
                    .key()
                    .and_then(|k| std::str::from_utf8(k).ok())
                    .map(|s| s.to_string());

                let payload_bytes = borrowed_message
                    .payload()
                    .ok_or_else(|| crate::Error::ConnectorError("Empty message payload".to_string()))?;

                let value_str = std::str::from_utf8(payload_bytes)
                    .map_err(|e| crate::Error::ConnectorError(format!("Invalid UTF-8 in message: {}", e)))?;

                let payload: serde_json::Value = serde_json::from_str(value_str)
                    .map_err(|e| crate::Error::ConnectorError(format!("Failed to parse message JSON: {}", e)))?;

                let message = KafkaMessage {
                    key,
                    value: payload.clone(),
                    headers: Default::default(),
                    partition: borrowed_message.partition(),
                    offset: borrowed_message.offset(),
                };

                // Create event from Kafka message
                let mut event = Event::new(
                    EventType::Custom("kafka.message".to_string()),
                    crate::EventSource::Kafka {
                        topic: self.config.topic.clone(),
                        partition: message.partition,
                    },
                    message.value.clone(),
                );

                // Add Kafka metadata
                event = event
                    .with_metadata("kafka_topic", self.config.topic.clone())
                    .with_metadata("kafka_partition", message.partition.to_string())
                    .with_metadata("kafka_offset", message.offset.to_string());

                if let Some(key) = &message.key {
                    event = event.with_metadata("kafka_key", key.clone());
                }

                Ok(Some(event))
            }
            Some(Err(_)) => Ok(None),
            None => Ok(None),
        }
    }
}

impl super::polling::ChangePoller for KafkaSource {
    fn poll_changes(&self) -> crate::Result<Option<u64>> {
        if !self.connected {
            return Err(crate::Error::ConnectorError(
                "Cannot poll: not connected to Kafka".to_string(),
            ));
        }

        // For Kafka, polling for changes means checking if there are messages
        // This is a simplified check - a real implementation might track offset changes
        Ok(Some(self.message_count))
    }

    fn polling_config(&self) -> super::polling::PollingConfig {
        self.polling_config.clone()
    }

    fn set_polling_config(&mut self, config: super::polling::PollingConfig) {
        self.polling_config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::EventSourceConnector;

    #[test]
    fn test_kafka_config_default() {
        let config = KafkaConfig::default();
        assert_eq!(config.brokers, "localhost:9092");
        assert_eq!(config.topic, "events");
        assert_eq!(config.auto_offset_reset, "latest");
        assert!(!config.use_ssl);
    }

    #[test]
    fn test_kafka_source_creation() {
        let config = KafkaConfig {
            brokers: "broker1:9092,broker2:9092".to_string(),
            topic: "test-topic".to_string(),
            ..Default::default()
        };

        let source = KafkaSource::new(config);
        assert_eq!(source.name(), "kafka");
        assert_eq!(source.source_type(), "kafka");
        assert!(!source.is_connected());
    }

    #[test]
    fn test_kafka_source_properties() {
        let config = KafkaConfig::default();
        let source = KafkaSource::new(config);
        assert_eq!(source.message_count, 0);
        let metrics = source.metrics();
        assert_eq!(metrics.messages_consumed, 0);
        assert!(!metrics.connected);
    }

    #[test]
    fn test_kafka_message_serialization() {
        let message = KafkaMessage {
            key: Some("test-key".to_string()),
            value: serde_json::json!({"id": 1, "name": "test"}),
            headers: Default::default(),
            partition: 0,
            offset: 100,
        };

        let json = serde_json::to_string(&message).unwrap();
        let deserialized: KafkaMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.key, message.key);
        assert_eq!(deserialized.offset, message.offset);
    }

    #[test]
    fn test_kafka_config_with_sasl() {
        let config = KafkaConfig {
            brokers: "broker:9092".to_string(),
            topic: "test-topic".to_string(),
            use_ssl: true,
            sasl_mechanism: Some("PLAIN".to_string()),
            sasl_username: Some("user".to_string()),
            sasl_password: Some("pass".to_string()),
            ..Default::default()
        };

        assert!(config.sasl_mechanism.is_some());
        assert_eq!(config.sasl_mechanism.unwrap(), "PLAIN");
    }

    #[test]
    fn test_kafka_source_with_polling() {
        use super::super::polling::SyncFrequency;

        let config = KafkaConfig::default();
        let polling = super::super::polling::PollingConfig::new(SyncFrequency::FiveMinutes);
        let source = KafkaSource::with_polling(config, polling);

        assert_eq!(source.polling_metrics().frequency, "every 5 minutes");
        assert!(source.polling_metrics().enabled);
    }

    #[test]
    fn test_kafka_source_set_sync_frequency() {
        use super::super::polling::SyncFrequency;

        let config = KafkaConfig::default();
        let mut source = KafkaSource::new(config);

        source.set_sync_frequency(SyncFrequency::Daily);
        assert_eq!(source.polling_metrics().frequency, "daily");
    }

    #[test]
    fn test_kafka_source_should_poll() {
        let config = KafkaConfig::default();
        let source = KafkaSource::new(config);

        // First check should return true (no previous polls)
        assert!(source.should_poll());
    }
}
