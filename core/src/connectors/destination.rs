/// Destination Connectors - Write data to various destinations
///
/// Unified interface for: Databases, Data Warehouses, APIs, Cloud Storage, SaaS platforms
use super::{Capability, ConnectionTest, Record};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Destination connector configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DestinationConfig {
    /// Connector type: postgres, warehouse, http, s3, etc.
    pub connector_type: String,
    /// Connection parameters (host, port, credentials, API key, etc.)
    pub params: HashMap<String, String>,
    /// Target table/endpoint/bucket
    pub destination: String,
    /// Write mode: append, upsert, replace
    pub write_mode: WriteMode,
    /// Batch size for writing
    pub batch_size: Option<usize>,
    /// Key column for upsert operations
    pub key_column: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum WriteMode {
    /// Append new records
    Append,
    /// Update existing, insert new
    Upsert,
    /// Truncate and replace
    Replace,
    /// Merge (database-specific)
    Merge,
}

/// Destination connector trait. Real implementations: `PostgreSQLConnector`
/// (`connectors/postgres.rs`), `MySQLConnector` (`connectors/mysql.rs`), the
/// object-storage destination (`connectors/object_storage.rs`), and the
/// webhook/Salesforce/HubSpot/Marketo adapters (`adapters/*.rs`) - all wired
/// into the real sync path via `executor.rs`'s `DestinationSpec`.
#[async_trait]
pub trait DestinationConnector: Send + Sync {
    /// Get connector name
    fn name(&self) -> &str;

    /// Get connector description
    fn description(&self) -> &str;

    /// Test connection
    async fn test_connection(&self) -> crate::Result<ConnectionTest>;

    /// Write single record
    async fn write_record(&self, record: &Record) -> crate::Result<()>;

    /// Write multiple records (batch)
    async fn write_batch(&self, records: &[Record]) -> crate::Result<usize>;

    /// Validate records before writing
    async fn validate_records(&self, records: &[Record]) -> crate::Result<()>;

    /// Get supported capabilities
    fn capabilities(&self) -> Vec<Capability>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_write_modes() {
        assert_eq!(WriteMode::Append, WriteMode::Append);
        assert_ne!(WriteMode::Append, WriteMode::Upsert);
    }
}
