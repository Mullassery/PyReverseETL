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

/// Kafka event source connector
pub struct KafkaSource {
    config: KafkaConfig,
    consumer: Option<BaseConsumer>,
    connected: bool,
    message_count: u64,
}

impl KafkaSource {
    /// Create a new Kafka source
    pub fn new(config: KafkaConfig) -> Self {
        Self {
            config,
            consumer: None,
            connected: false,
            message_count: 0,
        }
    }

    /// Get metrics
    pub fn metrics(&self) -> KafkaSourceMetrics {
        KafkaSourceMetrics {
            messages_consumed: self.message_count,
            connected: self.connected,
        }
    }
}

/// Kafka source metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaSourceMetrics {
    pub messages_consumed: u64,
    pub connected: bool,
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
}
