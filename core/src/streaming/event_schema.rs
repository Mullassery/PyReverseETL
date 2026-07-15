use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

/// Event type classifications
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// Entity created in source
    EntityCreated,
    /// Entity updated in source
    EntityUpdated,
    /// Entity deleted in source
    EntityDeleted,
    /// Sync operation completed
    SyncCompleted,
    /// Custom event type
    Custom(String),
}

impl EventType {
    pub fn as_str(&self) -> &str {
        match self {
            EventType::EntityCreated => "entity.created",
            EventType::EntityUpdated => "entity.updated",
            EventType::EntityDeleted => "entity.deleted",
            EventType::SyncCompleted => "sync.completed",
            EventType::Custom(s) => s,
        }
    }
}

/// Event source identifier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventSource {
    /// From Kafka topic
    Kafka { topic: String, partition: i32 },
    /// From webhook
    Webhook { url: String },
    /// From CDC stream
    CDC { table: String },
    /// From API poll
    API { endpoint: String },
    /// Custom source
    Custom(String),
}

impl EventSource {
    pub fn as_str(&self) -> &str {
        match self {
            EventSource::Kafka { .. } => "kafka",
            EventSource::Webhook { .. } => "webhook",
            EventSource::CDC { .. } => "cdc",
            EventSource::API { .. } => "api",
            EventSource::Custom(s) => s,
        }
    }
}

/// Streaming event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique event identifier
    pub id: String,
    /// Event type classification
    pub event_type: EventType,
    /// Source of the event
    pub source: EventSource,
    /// The actual entity/payload data
    pub entity: Value,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Optional metadata (source headers, etc.)
    pub metadata: HashMap<String, String>,
    /// Optional tracing ID for debugging
    pub trace_id: Option<String>,
}

impl Event {
    /// Create a new event
    pub fn new(event_type: EventType, source: EventSource, entity: Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            event_type,
            source,
            entity,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
            trace_id: None,
        }
    }

    /// Add metadata to event
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Set trace ID for debugging
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    /// Check if event has entity data
    pub fn has_entity(&self) -> bool {
        !self.entity.is_null()
    }

    /// Get entity ID if available
    pub fn entity_id(&self) -> Option<String> {
        self.entity
            .get("id")
            .or_else(|| self.entity.get("_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_event_type_as_str() {
        assert_eq!(EventType::EntityCreated.as_str(), "entity.created");
        assert_eq!(EventType::EntityUpdated.as_str(), "entity.updated");
        assert_eq!(EventType::EntityDeleted.as_str(), "entity.deleted");
    }

    #[test]
    fn test_event_source_as_str() {
        assert_eq!(
            EventSource::Kafka {
                topic: "test".to_string(),
                partition: 0
            }
            .as_str(),
            "kafka"
        );
        assert_eq!(
            EventSource::Webhook {
                url: "http://example.com".to_string()
            }
            .as_str(),
            "webhook"
        );
    }

    #[test]
    fn test_event_creation() {
        let entity = json!({"id": "123", "name": "Test"});
        let event = Event::new(
            EventType::EntityCreated,
            EventSource::Kafka {
                topic: "events".to_string(),
                partition: 0,
            },
            entity,
        );

        assert_eq!(event.event_type, EventType::EntityCreated);
        assert!(event.has_entity());
        assert_eq!(event.entity_id(), Some("123".to_string()));
    }

    #[test]
    fn test_event_with_metadata() {
        let entity = json!({"id": "123"});
        let event = Event::new(
            EventType::EntityCreated,
            EventSource::Kafka {
                topic: "events".to_string(),
                partition: 0,
            },
            entity,
        )
        .with_metadata("source_version", "2.0")
        .with_metadata("priority", "high");

        assert_eq!(event.metadata.len(), 2);
        assert_eq!(event.metadata.get("source_version"), Some(&"2.0".to_string()));
    }

    #[test]
    fn test_event_with_trace_id() {
        let entity = json!({"id": "123"});
        let event = Event::new(
            EventType::EntityCreated,
            EventSource::Kafka {
                topic: "events".to_string(),
                partition: 0,
            },
            entity,
        )
        .with_trace_id("trace-abc-123");

        assert_eq!(event.trace_id, Some("trace-abc-123".to_string()));
    }

    #[test]
    fn test_event_entity_id_extraction() {
        let entity = json!({"_id": "456", "name": "Test"});
        let event = Event::new(
            EventType::EntityCreated,
            EventSource::Webhook {
                url: "http://example.com".to_string(),
            },
            entity,
        );

        assert_eq!(event.entity_id(), Some("456".to_string()));
    }
}
