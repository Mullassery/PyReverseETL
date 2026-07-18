/// Universal Connector Ecosystem
/// Unified interface for all data sources and destinations
///
/// Features:
/// - Code-first configuration (no UI needed)
/// - Zero-config defaults for common platforms
/// - Built-in connectors for 20+ platforms
/// - Easy custom connector creation
/// - Automatic schema detection
/// - Connection pooling and optimization

pub mod destination;
pub mod source;
pub mod config;
pub mod registry;
pub mod saas;
pub mod rate_limiting;
pub mod object_storage;
pub mod database_advanced;
pub mod hdfs;
pub mod connectors_db;

pub use destination::{DestinationConnector, DestinationConfig, WriteMode};
pub use source::{SourceConnector, SourceConfig};
pub use config::{ConnectorConfig, ConnectionPool, ConnectorType};
pub use registry::{ConnectorRegistry, ConnectorDescriptor, BuiltInConnectors};
pub use saas::{SaaSConnector, SaaSConfig};
pub use rate_limiting::{RateLimiter, RateLimitConfig, RateLimitStrategy, RateLimiterRegistry, RateLimitStats};
pub use object_storage::{ObjectStorageConfig, ObjectStorageSource, ObjectStorageDestination, FileFormat, TableFormat, FileOperations};
pub use database_advanced::{DatabaseConfig, AdvancedDatabaseSource, AdvancedDatabaseDestination, WriteStrategy, TableSchema};
pub use hdfs::{HdfsConfig, HdfsSource, HdfsDestination, HdfsAuth, HdfsOperations, FileStatus};
pub use connectors_db::{ConnectorRegistry as ConnectorDb, ConnectorInfo, ConnectorCategory, ConnectorTypeInfo, RateLimitDefault};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Universal data record type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Record {
    /// Unique record identifier
    pub id: String,
    /// Actual data (flexible JSON)
    pub data: serde_json::Value,
    /// Metadata about the record
    pub metadata: RecordMetadata,
}

/// Record metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecordMetadata {
    /// Source this record came from
    pub source: String,
    /// When was this record created in source
    pub source_timestamp: Option<String>,
    /// When did we receive this record
    pub received_at: String,
    /// Operation type (insert, update, delete)
    pub operation: RecordOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecordOperation {
    Insert,
    Update,
    Delete,
    Upsert,
}

/// Connection test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTest {
    pub success: bool,
    pub message: String,
    pub details: Option<String>,
}

impl ConnectionTest {
    pub fn success(msg: &str) -> Self {
        Self {
            success: true,
            message: msg.to_string(),
            details: None,
        }
    }

    pub fn failure(msg: &str, details: Option<String>) -> Self {
        Self {
            success: false,
            message: msg.to_string(),
            details,
        }
    }
}

/// Connector capability
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    /// Connector can read data
    Read,
    /// Connector can write data
    Write,
    /// Connector supports incremental reads
    IncrementalRead,
    /// Connector detects schema automatically
    SchemaDetection,
    /// Connector supports batch operations
    Batch,
    /// Connector supports streaming operations
    Stream,
}

/// Schema detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub fields: Vec<Field>,
    pub sample_records: Vec<Record>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub field_type: String,
    pub required: bool,
    pub description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_creation() {
        let record = Record {
            id: "rec-1".to_string(),
            data: serde_json::json!({
                "name": "John",
                "age": 30
            }),
            metadata: RecordMetadata {
                source: "database".to_string(),
                source_timestamp: None,
                received_at: chrono::Utc::now().to_rfc3339(),
                operation: RecordOperation::Insert,
            },
        };

        assert_eq!(record.id, "rec-1");
        assert_eq!(record.metadata.operation, RecordOperation::Insert);
    }

    #[test]
    fn test_connection_test() {
        let success = ConnectionTest::success("Connected successfully");
        assert!(success.success);
        assert_eq!(success.message, "Connected successfully");

        let failure = ConnectionTest::failure("Connection refused", Some("Port 5432 unreachable".to_string()));
        assert!(!failure.success);
        assert!(failure.details.is_some());
    }
}
