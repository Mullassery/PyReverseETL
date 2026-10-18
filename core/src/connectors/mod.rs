pub mod config;
pub mod connectors_db;
pub mod database_advanced;
pub mod destination;
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
pub mod error;
pub mod hdfs;
pub mod mysql;
pub mod object_storage;
pub mod postgres;
pub mod rate_limiting;
pub mod registry;
pub mod saas;
pub mod source;
pub mod web;

pub use config::{ConnectionPool, ConnectorConfig, ConnectorType};
pub use connectors_db::{
    ConnectorCategory, ConnectorInfo, ConnectorRegistry as ConnectorDb, ConnectorTypeInfo,
    RateLimitDefault,
};
pub use database_advanced::{
    AdvancedDatabaseDestination, AdvancedDatabaseSource, DatabaseConfig, TableSchema, WriteStrategy,
};
pub use destination::{DestinationConfig, DestinationConnector, WriteMode};
pub use error::{ConnectorError, ConnectorResult};
pub use hdfs::{FileStatus, HdfsAuth, HdfsConfig, HdfsDestination, HdfsOperations, HdfsSource};
pub use mysql::{MySQLConfig, MySQLConnector};
pub use object_storage::{
    FileFormat, FileOperations, ObjectStorageConfig, ObjectStorageDestination, ObjectStorageSource,
    TableFormat,
};
pub use postgres::{PostgreSQLConfig, PostgreSQLConnector};
pub use rate_limiting::{
    RateLimitConfig, RateLimitStats, RateLimitStrategy, RateLimiter, RateLimiterRegistry,
};
pub use registry::{BuiltInConnectors, ConnectorDescriptor, ConnectorRegistry};
pub use saas::{SaaSConfig, SaaSConnector};
pub use source::{SourceConfig, SourceConnector};
pub use web::{WebConfig, WebSource};

use serde::{Deserialize, Serialize};

/// Validate a SQL identifier that is about to be interpolated into a query
/// string.
///
/// SQL does not allow binding identifiers (table/column names) as query
/// parameters, so the SQL connectors build those queries by `format!`-ing
/// names straight into the statement. Every such identifier is therefore an
/// injection surface and must be validated rather than trusted. This accepts
/// a bare identifier matching `[A-Za-z_][A-Za-z0-9_]*`, or a schema-qualified
/// one whose dot-separated parts each match that pattern (e.g.
/// `public.orders`). Anything else — quotes, semicolons, whitespace, empty
/// parts — is rejected with the offending identifier in the error.
pub fn validate_sql_identifier(identifier: &str) -> crate::Result<()> {
    fn is_valid_part(part: &str) -> bool {
        let mut chars = part.chars();
        match chars.next() {
            Some(c) if c == '_' || c.is_ascii_alphabetic() => {}
            _ => return false,
        }
        chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
    }

    if identifier.is_empty() || !identifier.split('.').all(is_valid_part) {
        return Err(crate::Error::ConfigError(format!(
            "invalid SQL identifier {identifier:?}: expected [A-Za-z_][A-Za-z0-9_]* \
             (optionally schema-qualified with dots)"
        )));
    }
    Ok(())
}

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

        let failure = ConnectionTest::failure(
            "Connection refused",
            Some("Port 5432 unreachable".to_string()),
        );
        assert!(!failure.success);
        assert!(failure.details.is_some());
    }

    #[test]
    fn test_validate_sql_identifier_accepts_valid() {
        for ok in [
            "customers",
            "_internal",
            "t1",
            "public.orders",
            "db.schema.table",
        ] {
            assert!(
                validate_sql_identifier(ok).is_ok(),
                "expected {ok:?} to be accepted"
            );
        }
    }

    #[test]
    fn test_validate_sql_identifier_rejects_injection() {
        for bad in [
            "",
            "customers; DROP TABLE users",
            "customers\"",
            "a b",
            "1table",
            "public.",
            ".orders",
            "public..orders",
            "users--",
        ] {
            assert!(
                validate_sql_identifier(bad).is_err(),
                "expected {bad:?} to be rejected"
            );
        }
    }
}
