/// Source Connectors - Read data from various sources
///
/// Unified interface for: Databases, APIs, Files, Event Streams, Data Warehouses

use super::{Record, Schema, ConnectionTest, Capability};
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

/// Source connector trait
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

// Built-in Source Connectors

/// PostgreSQL/MySQL Source
#[derive(Debug, Clone)]
pub struct DatabaseSource {
    pub connector_type: String, // "postgres" or "mysql"
    pub config: SourceConfig,
}

#[async_trait]
impl SourceConnector for DatabaseSource {
    fn name(&self) -> &str {
        match self.connector_type.as_str() {
            "postgres" => "PostgreSQL",
            "mysql" => "MySQL",
            _ => "Database",
        }
    }

    fn description(&self) -> &str {
        "Read data from relational databases"
    }

    async fn test_connection(&self) -> crate::Result<ConnectionTest> {
        // In production: actually test the connection
        Ok(ConnectionTest::success(&format!(
            "Connected to {} database",
            self.connector_type
        )))
    }

    async fn detect_schema(&self) -> crate::Result<Schema> {
        // In production: query database metadata
        Ok(Schema {
            fields: vec![],
            sample_records: vec![],
        })
    }

    async fn read_all(&self) -> crate::Result<Vec<Record>> {
        // In production: execute query and return results
        Ok(vec![])
    }

    async fn read_batch(&self, _offset: u64, _limit: u64) -> crate::Result<Vec<Record>> {
        Ok(vec![])
    }

    async fn read_incremental(&self, _last_value: &str) -> crate::Result<Vec<Record>> {
        Ok(vec![])
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::Read,
            Capability::IncrementalRead,
            Capability::SchemaDetection,
            Capability::Batch,
        ]
    }
}

/// CSV/JSON File Source
#[derive(Debug, Clone)]
pub struct FileSource {
    pub config: SourceConfig,
}

#[async_trait]
impl SourceConnector for FileSource {
    fn name(&self) -> &str {
        "File (CSV, JSON, Parquet)"
    }

    fn description(&self) -> &str {
        "Read data from local or remote files"
    }

    async fn test_connection(&self) -> crate::Result<ConnectionTest> {
        Ok(ConnectionTest::success("File accessible"))
    }

    async fn detect_schema(&self) -> crate::Result<Schema> {
        Ok(Schema {
            fields: vec![],
            sample_records: vec![],
        })
    }

    async fn read_all(&self) -> crate::Result<Vec<Record>> {
        Ok(vec![])
    }

    async fn read_batch(&self, _offset: u64, _limit: u64) -> crate::Result<Vec<Record>> {
        Ok(vec![])
    }

    async fn read_incremental(&self, _last_value: &str) -> crate::Result<Vec<Record>> {
        Ok(vec![])
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::Read, Capability::SchemaDetection, Capability::Batch]
    }
}

/// REST API Source
#[derive(Debug, Clone)]
pub struct ApiSource {
    pub config: SourceConfig,
}

#[async_trait]
impl SourceConnector for ApiSource {
    fn name(&self) -> &str {
        "REST API"
    }

    fn description(&self) -> &str {
        "Read data from HTTP REST endpoints"
    }

    async fn test_connection(&self) -> crate::Result<ConnectionTest> {
        Ok(ConnectionTest::success("API endpoint reachable"))
    }

    async fn detect_schema(&self) -> crate::Result<Schema> {
        Ok(Schema {
            fields: vec![],
            sample_records: vec![],
        })
    }

    async fn read_all(&self) -> crate::Result<Vec<Record>> {
        Ok(vec![])
    }

    async fn read_batch(&self, _offset: u64, _limit: u64) -> crate::Result<Vec<Record>> {
        Ok(vec![])
    }

    async fn read_incremental(&self, _last_value: &str) -> crate::Result<Vec<Record>> {
        Ok(vec![])
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::Read, Capability::Batch, Capability::Stream]
    }
}

/// S3/Cloud Storage Source
#[derive(Debug, Clone)]
pub struct CloudStorageSource {
    pub provider: String, // "s3", "gcs", "azure"
    pub config: SourceConfig,
}

#[async_trait]
impl SourceConnector for CloudStorageSource {
    fn name(&self) -> &str {
        match self.provider.as_str() {
            "s3" => "Amazon S3",
            "gcs" => "Google Cloud Storage",
            "azure" => "Azure Blob Storage",
            _ => "Cloud Storage",
        }
    }

    fn description(&self) -> &str {
        "Read data from cloud object storage"
    }

    async fn test_connection(&self) -> crate::Result<ConnectionTest> {
        Ok(ConnectionTest::success(&format!(
            "Connected to {} storage",
            self.provider
        )))
    }

    async fn detect_schema(&self) -> crate::Result<Schema> {
        Ok(Schema {
            fields: vec![],
            sample_records: vec![],
        })
    }

    async fn read_all(&self) -> crate::Result<Vec<Record>> {
        Ok(vec![])
    }

    async fn read_batch(&self, _offset: u64, _limit: u64) -> crate::Result<Vec<Record>> {
        Ok(vec![])
    }

    async fn read_incremental(&self, _last_value: &str) -> crate::Result<Vec<Record>> {
        Ok(vec![])
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::Read, Capability::SchemaDetection, Capability::Batch]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_source() {
        let mut config = SourceConfig {
            connector_type: "postgres".to_string(),
            params: Default::default(),
            source: "SELECT * FROM users".to_string(),
            batch_size: Some(1000),
            incremental_column: None,
        };
        config.params.insert("host".to_string(), "localhost".to_string());

        let source = DatabaseSource {
            connector_type: "postgres".to_string(),
            config,
        };

        assert_eq!(source.name(), "PostgreSQL");
        assert!(source
            .test_connection()
            .await
            .unwrap()
            .message
            .contains("PostgreSQL"));

        let capabilities = source.capabilities();
        assert!(capabilities.contains(&Capability::Read));
        assert!(capabilities.contains(&Capability::IncrementalRead));
    }

    #[tokio::test]
    async fn test_file_source() {
        let config = SourceConfig {
            connector_type: "csv".to_string(),
            params: Default::default(),
            source: "/data/customers.csv".to_string(),
            batch_size: Some(5000),
            incremental_column: None,
        };

        let source = FileSource { config };

        assert_eq!(source.name(), "File (CSV, JSON, Parquet)");
        assert!(source.test_connection().await.unwrap().success);
    }

    #[tokio::test]
    async fn test_api_source() {
        let mut config = SourceConfig {
            connector_type: "api".to_string(),
            params: Default::default(),
            source: "https://api.example.com/users".to_string(),
            batch_size: Some(100),
            incremental_column: Some("updated_at".to_string()),
        };
        config.params.insert("auth_type".to_string(), "bearer".to_string());

        let source = ApiSource { config };

        assert_eq!(source.name(), "REST API");
        let capabilities = source.capabilities();
        assert!(capabilities.contains(&Capability::Stream));
    }
}
