pub mod kafka;

pub use kafka::{KafkaSource, KafkaConfig, KafkaMessage};

/// Trait for event sources (Kafka, CDC, API, etc.)
pub trait EventSourceConnector: Send + Sync {
    /// Get source name
    fn name(&self) -> &str;

    /// Get source type
    fn source_type(&self) -> &str;

    /// Connect to source
    fn connect(&mut self) -> crate::Result<()>;

    /// Disconnect from source
    fn disconnect(&mut self) -> crate::Result<()>;

    /// Check if connected
    fn is_connected(&self) -> bool;

    /// Get next event
    fn next_event(&mut self) -> crate::Result<Option<crate::Event>>;
}
