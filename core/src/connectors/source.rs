/// Source Connectors - Read data from various sources
///
/// Unified interface for: Databases, APIs, Files, Event Streams, Data Warehouses
use super::{Capability, ConnectionTest, Record, Schema};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Source connector configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    /// Connector type: postgres, mysql, csv, api, s3, etc.
    pub connector_type: String,
    /// Connection parameters (host, port, credentials, etc.)
    pub params: HashMap<String, String>,
    /// Query/path to read from
    pub source: String,
    /// Batch size for reading
    pub batch_size: Option<usize>,
    /// Incremental read column (for delta reads)
    pub incremental_column: Option<String>,
}

/// Source connector trait. Real implementations: `PostgreSQLConnector`
/// (`connectors/postgres.rs`), `MySQLConnector` (`connectors/mysql.rs`),
/// and the object-storage source (`connectors/object_storage.rs`) - all
/// wired into the real sync path via `executor.rs`'s `SourceSpec`.
#[async_trait]
pub trait SourceConnector: Send + Sync {
    /// Get connector name
    fn name(&self) -> &str;

    /// Get connector description
    fn description(&self) -> &str;

    /// Test connection
    async fn test_connection(&self) -> crate::Result<ConnectionTest>;

    /// Detect schema automatically
    async fn detect_schema(&self) -> crate::Result<Schema>;

    /// Read all records
    async fn read_all(&self) -> crate::Result<Vec<Record>>;

    /// Read records in batches
    async fn read_batch(&self, offset: u64, limit: u64) -> crate::Result<Vec<Record>>;

    /// Read incremental (changed records since last read)
    async fn read_incremental(&self, last_value: &str) -> crate::Result<Vec<Record>>;

    /// Get supported capabilities
    fn capabilities(&self) -> Vec<Capability>;
}
