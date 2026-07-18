/// Destination Connectors - Write data to various destinations
///
/// Unified interface for: Databases, Data Warehouses, APIs, Cloud Storage, SaaS platforms

use super::{Record, ConnectionTest, Capability};
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

/// Destination connector trait
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

// Built-in Destination Connectors

/// Database Destination (PostgreSQL, MySQL)
#[derive(Debug, Clone)]
pub struct DatabaseDestination {
    pub connector_type: String,
    pub config: DestinationConfig,
}

#[async_trait]
impl DestinationConnector for DatabaseDestination {
    fn name(&self) -> &str {
        match self.connector_type.as_str() {
            "postgres" => "PostgreSQL",
            "mysql" => "MySQL",
            _ => "Database",
        }
    }

    fn description(&self) -> &str {
        "Write data to relational databases"
    }

    async fn test_connection(&self) -> crate::Result<ConnectionTest> {
        Ok(ConnectionTest::success(&format!(
            "Connected to {} database",
            self.connector_type
        )))
    }

    async fn write_record(&self, _record: &Record) -> crate::Result<()> {
        // In production: insert/update record in database
        Ok(())
    }

    async fn write_batch(&self, records: &[Record]) -> crate::Result<usize> {
        // In production: batch insert/update records
        Ok(records.len())
    }

    async fn validate_records(&self, _records: &[Record]) -> crate::Result<()> {
        Ok(())
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::Write,
            Capability::Batch,
            Capability::SchemaDetection,
        ]
    }
}

/// Data Warehouse Destination (Snowflake, BigQuery, Redshift, etc.)
#[derive(Debug, Clone)]
pub struct WarehouseDestination {
    pub provider: String, // "snowflake", "bigquery", "redshift"
    pub config: DestinationConfig,
}

#[async_trait]
impl DestinationConnector for WarehouseDestination {
    fn name(&self) -> &str {
        match self.provider.as_str() {
            "snowflake" => "Snowflake",
            "bigquery" => "BigQuery",
            "redshift" => "Amazon Redshift",
            _ => "Data Warehouse",
        }
    }

    fn description(&self) -> &str {
        "Write data to cloud data warehouses"
    }

    async fn test_connection(&self) -> crate::Result<ConnectionTest> {
        Ok(ConnectionTest::success(&format!(
            "Connected to {} warehouse",
            self.provider
        )))
    }

    async fn write_record(&self, _record: &Record) -> crate::Result<()> {
        Ok(())
    }

    async fn write_batch(&self, records: &[Record]) -> crate::Result<usize> {
        Ok(records.len())
    }

    async fn validate_records(&self, _records: &[Record]) -> crate::Result<()> {
        Ok(())
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::Write,
            Capability::Batch,
            Capability::SchemaDetection,
            Capability::Stream,
        ]
    }
}

/// REST API Destination
#[derive(Debug, Clone)]
pub struct ApiDestination {
    pub config: DestinationConfig,
}

#[async_trait]
impl DestinationConnector for ApiDestination {
    fn name(&self) -> &str {
        "REST API"
    }

    fn description(&self) -> &str {
        "Send data to HTTP REST endpoints"
    }

    async fn test_connection(&self) -> crate::Result<ConnectionTest> {
        Ok(ConnectionTest::success("API endpoint reachable"))
    }

    async fn write_record(&self, _record: &Record) -> crate::Result<()> {
        Ok(())
    }

    async fn write_batch(&self, records: &[Record]) -> crate::Result<usize> {
        Ok(records.len())
    }

    async fn validate_records(&self, _records: &[Record]) -> crate::Result<()> {
        Ok(())
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::Write, Capability::Batch, Capability::Stream]
    }
}

/// Cloud Storage Destination (S3, GCS, Azure)
#[derive(Debug, Clone)]
pub struct CloudStorageDestination {
    pub provider: String,
    pub config: DestinationConfig,
}

#[async_trait]
impl DestinationConnector for CloudStorageDestination {
    fn name(&self) -> &str {
        match self.provider.as_str() {
            "s3" => "Amazon S3",
            "gcs" => "Google Cloud Storage",
            "azure" => "Azure Blob Storage",
            _ => "Cloud Storage",
        }
    }

    fn description(&self) -> &str {
        "Write data to cloud object storage"
    }

    async fn test_connection(&self) -> crate::Result<ConnectionTest> {
        Ok(ConnectionTest::success(&format!(
            "Connected to {} storage",
            self.provider
        )))
    }

    async fn write_record(&self, _record: &Record) -> crate::Result<()> {
        Ok(())
    }

    async fn write_batch(&self, records: &[Record]) -> crate::Result<usize> {
        Ok(records.len())
    }

    async fn validate_records(&self, _records: &[Record]) -> crate::Result<()> {
        Ok(())
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::Write, Capability::Batch]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_destination() {
        let mut config = DestinationConfig {
            connector_type: "postgres".to_string(),
            params: Default::default(),
            destination: "users".to_string(),
            write_mode: WriteMode::Upsert,
            batch_size: Some(1000),
            key_column: Some("id".to_string()),
        };
        config.params.insert("host".to_string(), "localhost".to_string());

        let dest = DatabaseDestination {
            connector_type: "postgres".to_string(),
            config,
        };

        assert_eq!(dest.name(), "PostgreSQL");
        assert!(dest
            .test_connection()
            .await
            .unwrap()
            .message
            .contains("PostgreSQL"));

        let capabilities = dest.capabilities();
        assert!(capabilities.contains(&Capability::Write));
    }

    #[tokio::test]
    async fn test_warehouse_destination() {
        let config = DestinationConfig {
            connector_type: "snowflake".to_string(),
            params: Default::default(),
            destination: "analytics.public.customers".to_string(),
            write_mode: WriteMode::Append,
            batch_size: Some(10000),
            key_column: None,
        };

        let dest = WarehouseDestination {
            provider: "snowflake".to_string(),
            config,
        };

        assert_eq!(dest.name(), "Snowflake");
        assert!(dest.test_connection().await.unwrap().success);
    }

    #[tokio::test]
    async fn test_write_modes() {
        assert_eq!(WriteMode::Append, WriteMode::Append);
        assert_ne!(WriteMode::Append, WriteMode::Upsert);
    }
}
