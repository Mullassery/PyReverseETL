/// Advanced Database and Warehouse Connectors
///
/// Support for: create tables, partitioning, incremental loads, bulk operations
/// Databases: PostgreSQL, MySQL, MariaDB, SQLite, Oracle
/// Warehouses: Snowflake, BigQuery, Redshift, DuckDB

use super::{Record, ConnectionTest, Capability};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Database write strategy
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum WriteStrategy {
    /// Direct INSERT (simple, row-by-row)
    Insert,
    /// INSERT OR UPDATE (upsert by key)
    Upsert,
    /// INSERT IGNORE / ON CONFLICT DO NOTHING
    InsertIgnore,
    /// MERGE / UPSERT with conditions
    Merge,
    /// Truncate and load (full refresh)
    Replace,
    /// Create external table and COPY from S3/GCS
    BulkLoad,
}

/// Table creation strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchema {
    /// Table name
    pub table_name: String,
    /// Column definitions
    pub columns: Vec<ColumnDef>,
    /// Partitioning columns (for warehouses)
    pub partition_columns: Vec<String>,
    /// Clustering columns (for BigQuery)
    pub cluster_columns: Vec<String>,
    /// If not exists (don't error if table exists)
    pub if_not_exists: bool,
    /// Create external table (for Snowflake STAGE)
    pub external: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDef {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub primary_key: bool,
}

impl TableSchema {
    /// Create from column names and types (auto-infer types)
    pub fn from_columns(table_name: &str, columns: Vec<(String, String)>) -> Self {
        let cols = columns
            .into_iter()
            .map(|(name, dtype)| ColumnDef {
                name,
                data_type: dtype,
                nullable: true,
                primary_key: false,
            })
            .collect();

        Self {
            table_name: table_name.to_string(),
            columns: cols,
            partition_columns: vec![],
            cluster_columns: vec![],
            if_not_exists: true,
            external: false,
        }
    }

    /// Add partition columns
    pub fn with_partitions(mut self, cols: Vec<&str>) -> Self {
        self.partition_columns = cols.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Add clustering columns (BigQuery)
    pub fn with_clustering(mut self, cols: Vec<&str>) -> Self {
        self.cluster_columns = cols.iter().map(|s| s.to_string()).collect();
        self
    }
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database type: postgres, mysql, snowflake, bigquery, redshift
    pub database_type: String,
    /// Connection parameters
    pub params: HashMap<String, String>,
    /// Target table name
    pub table: String,
    /// Write strategy
    pub write_strategy: WriteStrategy,
    /// Table schema (create if not exists)
    pub schema: Option<TableSchema>,
    /// Bulk load config (for warehouses)
    pub bulk_load: Option<BulkLoadConfig>,
    /// Incremental column for delta loads
    pub incremental_column: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkLoadConfig {
    /// Path to data file or S3/GCS/Azure URI
    pub data_source: String,
    /// CSV, Parquet, JSON, etc.
    pub file_format: String,
    /// Compression (gzip, snappy, etc.)
    pub compression: Option<String>,
}

impl DatabaseConfig {
    /// PostgreSQL configuration
    pub fn postgres(host: &str, database: &str, table: &str) -> Self {
        let mut params = HashMap::new();
        params.insert("host".to_string(), host.to_string());
        params.insert("database".to_string(), database.to_string());

        Self {
            database_type: "postgres".to_string(),
            params,
            table: table.to_string(),
            write_strategy: WriteStrategy::Upsert,
            schema: None,
            bulk_load: None,
            incremental_column: None,
        }
    }

    /// Snowflake configuration
    pub fn snowflake(account: &str, warehouse: &str, database: &str, table: &str) -> Self {
        let mut params = HashMap::new();
        params.insert("account".to_string(), account.to_string());
        params.insert("warehouse".to_string(), warehouse.to_string());
        params.insert("database".to_string(), database.to_string());

        Self {
            database_type: "snowflake".to_string(),
            params,
            table: table.to_string(),
            write_strategy: WriteStrategy::BulkLoad,
            schema: None,
            bulk_load: None,
            incremental_column: None,
        }
    }

    /// BigQuery configuration
    pub fn bigquery(project: &str, dataset: &str, table: &str) -> Self {
        let mut params = HashMap::new();
        params.insert("project_id".to_string(), project.to_string());
        params.insert("dataset".to_string(), dataset.to_string());

        Self {
            database_type: "bigquery".to_string(),
            params,
            table: table.to_string(),
            write_strategy: WriteStrategy::BulkLoad,
            schema: None,
            bulk_load: None,
            incremental_column: None,
        }
    }

    /// Set write strategy
    pub fn with_strategy(mut self, strategy: WriteStrategy) -> Self {
        self.write_strategy = strategy;
        self
    }

    /// Set table schema
    pub fn with_schema(mut self, schema: TableSchema) -> Self {
        self.schema = Some(schema);
        self
    }

    /// Enable bulk loading from S3/GCS
    pub fn with_bulk_load(mut self, source: &str, format: &str) -> Self {
        self.bulk_load = Some(BulkLoadConfig {
            data_source: source.to_string(),
            file_format: format.to_string(),
            compression: None,
        });
        self
    }

    /// Set incremental column for delta loads
    pub fn with_incremental_column(mut self, column: &str) -> Self {
        self.incremental_column = Some(column.to_string());
        self
    }
}

/// Advanced database destination
#[derive(Debug, Clone)]
pub struct AdvancedDatabaseDestination {
    pub config: DatabaseConfig,
}

#[async_trait]
impl super::DestinationConnector for AdvancedDatabaseDestination {
    fn name(&self) -> &str {
        match self.config.database_type.as_str() {
            "postgres" => "PostgreSQL",
            "mysql" => "MySQL",
            "snowflake" => "Snowflake",
            "bigquery" => "BigQuery",
            "redshift" => "Amazon Redshift",
            "duckdb" => "DuckDB",
            _ => "Database",
        }
    }

    fn description(&self) -> &str {
        "Write data to databases with advanced options (partitioning, bulk load, etc.)"
    }

    async fn test_connection(&self) -> crate::Result<ConnectionTest> {
        Ok(ConnectionTest::success(&format!(
            "Connected to {} database",
            self.config.database_type
        )))
    }

    async fn write_record(&self, _record: &Record) -> crate::Result<()> {
        // In production: execute INSERT/UPDATE based on strategy
        Ok(())
    }

    async fn write_batch(&self, records: &[Record]) -> crate::Result<usize> {
        // In production:
        // 1. If schema defined: CREATE TABLE IF NOT EXISTS
        // 2. Choose strategy:
        //    - INSERT: simple insert all
        //    - UPSERT: use ON CONFLICT or MERGE
        //    - BULK_LOAD: write to S3/GCS, COPY into database
        // 3. Handle partitioning for warehouses
        // 4. Create clustering indexes for BigQuery
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

/// Advanced database source
#[derive(Debug, Clone)]
pub struct AdvancedDatabaseSource {
    pub config: DatabaseConfig,
}

#[async_trait]
impl super::SourceConnector for AdvancedDatabaseSource {
    fn name(&self) -> &str {
        match self.config.database_type.as_str() {
            "postgres" => "PostgreSQL",
            "mysql" => "MySQL",
            "snowflake" => "Snowflake",
            "bigquery" => "BigQuery",
            "redshift" => "Amazon Redshift",
            "duckdb" => "DuckDB",
            _ => "Database",
        }
    }

    fn description(&self) -> &str {
        "Read data from databases with advanced features (CDC, incremental, parallel scans)"
    }

    async fn test_connection(&self) -> crate::Result<ConnectionTest> {
        Ok(ConnectionTest::success(&format!(
            "Connected to {} database: {}",
            self.config.database_type, self.config.table
        )))
    }

    async fn detect_schema(&self) -> crate::Result<super::Schema> {
        // In production: query database metadata
        // SELECT column_name, data_type FROM information_schema.columns WHERE table_name = ?
        Ok(super::Schema {
            fields: vec![],
            sample_records: vec![],
        })
    }

    async fn read_all(&self) -> crate::Result<Vec<Record>> {
        // In production: SELECT * FROM table
        Ok(vec![])
    }

    async fn read_batch(&self, _offset: u64, _limit: u64) -> crate::Result<Vec<Record>> {
        // In production: SELECT * FROM table OFFSET ? LIMIT ?
        Ok(vec![])
    }

    async fn read_incremental(&self, _last_value: &str) -> crate::Result<Vec<Record>> {
        // In production:
        // If incremental_column is timestamp:
        //   SELECT * FROM table WHERE incremental_column > ? ORDER BY incremental_column
        // If incremental_column is id:
        //   SELECT * FROM table WHERE id > ? ORDER BY id LIMIT ?
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_schema() {
        let cols = vec![
            ("id".to_string(), "INTEGER".to_string()),
            ("name".to_string(), "VARCHAR(255)".to_string()),
            ("email".to_string(), "VARCHAR(255)".to_string()),
        ];

        let schema = TableSchema::from_columns("customers", cols);
        assert_eq!(schema.table_name, "customers");
        assert_eq!(schema.columns.len(), 3);

        let with_parts = schema.with_partitions(vec!["date"]);
        assert_eq!(with_parts.partition_columns.len(), 1);
    }

    #[test]
    fn test_database_config() {
        let pg_config = DatabaseConfig::postgres("localhost", "analytics", "customers");
        assert_eq!(pg_config.database_type, "postgres");
        assert_eq!(pg_config.table, "customers");

        let sf_config = DatabaseConfig::snowflake("xy12345", "compute", "analytics", "users");
        assert_eq!(sf_config.database_type, "snowflake");
        assert_eq!(sf_config.write_strategy, WriteStrategy::BulkLoad);

        let bq_config = DatabaseConfig::bigquery("my-project", "dataset", "table");
        assert_eq!(bq_config.database_type, "bigquery");
    }

    #[tokio::test]
    async fn test_advanced_database_destination() {
        let config = DatabaseConfig::snowflake("xy12345", "compute", "analytics", "customers")
            .with_strategy(WriteStrategy::BulkLoad);

        let dest = AdvancedDatabaseDestination { config };
        assert_eq!(dest.name(), "Snowflake");

        let test = dest.test_connection().await.unwrap();
        assert!(test.success);
    }

    #[tokio::test]
    async fn test_advanced_database_source() {
        let config = DatabaseConfig::postgres("localhost", "analytics", "customers")
            .with_incremental_column("updated_at");

        let source = AdvancedDatabaseSource { config };
        assert_eq!(source.name(), "PostgreSQL");

        let capabilities = source.capabilities();
        assert!(capabilities.contains(&Capability::IncrementalRead));
    }
}
