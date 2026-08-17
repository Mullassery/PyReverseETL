// MySQL Connector Implementation
// Real reads and writes over a live MySQL connection via sqlx.
// Supports: Read, Write, Schema Detection, Incremental Reads, Batch Operations, Upsert

use crate::connectors::{
    Capability, ConnectionTest, ConnectorError, DestinationConnector, Record, Schema,
    SourceConnector,
};
use async_trait::async_trait;
use serde_json::{Map, Value};
use sqlx::mysql::{MySqlPoolOptions, MySqlRow};
use sqlx::MySqlPool;
use sqlx::{Column, Row, TypeInfo};
use tokio::sync::OnceCell;

/// MySQL Connector Configuration
#[derive(Debug, Clone)]
pub struct MySQLConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
    pub ssl_mode: SSLMode,
    pub pool_min: usize,
    pub pool_max: usize,
    pub connect_timeout: u64,
    pub idle_timeout: u64,
    /// Table this connector reads from / writes to.
    pub table: String,
    /// Column used for `read_incremental`. Reads honestly fail if this isn't
    /// configured, rather than fabricating rows.
    pub incremental_column: Option<String>,
    /// Column used to upsert (`ON DUPLICATE KEY UPDATE`) on write.
    pub upsert_key: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum SSLMode {
    Disabled,
    Allow,
    Prefer,
    Require,
}

impl MySQLConfig {
    pub fn new(
        host: impl Into<String>,
        port: u16,
        database: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
        table: impl Into<String>,
    ) -> Self {
        Self {
            host: host.into(),
            port,
            database: database.into(),
            username: username.into(),
            password: password.into(),
            ssl_mode: SSLMode::Prefer,
            pool_min: 2,
            pool_max: 20,
            connect_timeout: 10,
            idle_timeout: 300,
            table: table.into(),
            incremental_column: None,
            upsert_key: None,
        }
    }

    pub fn with_incremental_column(mut self, column: impl Into<String>) -> Self {
        self.incremental_column = Some(column.into());
        self
    }

    pub fn with_upsert_key(mut self, column: impl Into<String>) -> Self {
        self.upsert_key = Some(column.into());
        self
    }

    /// Create from connection string
    pub fn from_url(url: &str) -> Result<Self, ConnectorError> {
        // Parse mysql://user:pass@host:port/database
        let url = url.replace("mysql://", "");
        let parts: Vec<&str> = url.split('@').collect();

        if parts.len() != 2 {
            return Err(ConnectorError::InvalidConfig(
                "Invalid MySQL URL format".to_string(),
            ));
        }

        let (user_pass, host_db) = (parts[0], parts[1]);
        let user_parts: Vec<&str> = user_pass.split(':').collect();
        if user_parts.len() != 2 {
            return Err(ConnectorError::InvalidConfig(
                "Invalid credentials in URL".to_string(),
            ));
        }

        let (username, password) = (user_parts[0].to_string(), user_parts[1].to_string());

        let host_parts: Vec<&str> = host_db.split('/').collect();
        if host_parts.len() != 2 {
            return Err(ConnectorError::InvalidConfig(
                "Invalid host/database in URL".to_string(),
            ));
        }

        let (host_port, database) = (host_parts[0], host_parts[1].to_string());
        let host_port_parts: Vec<&str> = host_port.split(':').collect();

        let (host, port) = if host_port_parts.len() == 2 {
            (
                host_port_parts[0].to_string(),
                host_port_parts[1].parse::<u16>().unwrap_or(3306),
            )
        } else {
            (host_port.to_string(), 3306)
        };

        Ok(Self::new(host, port, database, username, password, "data"))
    }

    /// Connection string for sqlx
    pub fn connection_string(&self) -> String {
        format!(
            "mysql://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database
        )
    }
}

enum ColumnValue {
    Bool(bool),
    I64(i64),
    F64(f64),
    Text(String),
    Json(Value),
}

fn json_to_column_value(value: &Value) -> Option<ColumnValue> {
    match value {
        Value::Null => None,
        Value::Bool(b) => Some(ColumnValue::Bool(*b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(ColumnValue::I64(i))
            } else {
                Some(ColumnValue::F64(n.as_f64().unwrap_or_default()))
            }
        }
        Value::String(s) => Some(ColumnValue::Text(s.clone())),
        other => Some(ColumnValue::Json(other.clone())),
    }
}

/// Generic MySQL row -> JSON object mapping so this connector can work against
/// any table without compile-time knowledge of its schema.
fn mysql_row_to_json(row: &MySqlRow) -> Value {
    let mut map = Map::new();
    for col in row.columns() {
        let name = col.name();
        let type_name = col.type_info().name();
        let value: Value = match type_name {
            "TINYINT" | "SMALLINT" | "INT" | "MEDIUMINT" => row
                .try_get::<i32, _>(name)
                .map(Value::from)
                .unwrap_or(Value::Null),
            "BIGINT" => row
                .try_get::<i64, _>(name)
                .map(Value::from)
                .unwrap_or(Value::Null),
            "FLOAT" => row
                .try_get::<f32, _>(name)
                .map(|v| Value::from(v as f64))
                .unwrap_or(Value::Null),
            "DOUBLE" | "DECIMAL" => row
                .try_get::<f64, _>(name)
                .map(Value::from)
                .unwrap_or(Value::Null),
            "BOOLEAN" | "BOOL" => row
                .try_get::<bool, _>(name)
                .map(Value::from)
                .unwrap_or(Value::Null),
            "JSON" => row.try_get::<Value, _>(name).unwrap_or(Value::Null),
            "TIMESTAMP" | "DATETIME" => row
                .try_get::<chrono::NaiveDateTime, _>(name)
                .map(|v| Value::from(v.and_utc().to_rfc3339()))
                .unwrap_or(Value::Null),
            _ => row
                .try_get::<String, _>(name)
                .map(Value::from)
                .unwrap_or(Value::Null),
        };
        map.insert(name.to_string(), value);
    }
    Value::Object(map)
}

/// MySQL Connector
pub struct MySQLConnector {
    config: MySQLConfig,
    pool: OnceCell<MySqlPool>,
}

impl MySQLConnector {
    pub fn new(config: MySQLConfig) -> Self {
        Self {
            config,
            pool: OnceCell::new(),
        }
    }

    /// Get connection string
    pub fn connection_string(&self) -> String {
        self.config.connection_string()
    }

    /// Parse MySQL URL
    pub fn from_url(url: &str) -> Result<Self, ConnectorError> {
        let config = MySQLConfig::from_url(url)?;
        Ok(Self::new(config))
    }

    async fn pool(&self) -> crate::Result<&MySqlPool> {
        self.pool
            .get_or_try_init(|| async {
                MySqlPoolOptions::new()
                    .max_connections(self.config.pool_max as u32)
                    .connect(&self.config.connection_string())
                    .await
                    .map_err(|e| crate::Error::ConnectorError(format!("MySQL connect failed: {e}")))
            })
            .await
    }

    fn record_from_row(&self, row: &MySqlRow) -> Record {
        let data = mysql_row_to_json(row);
        let id = data
            .get("id")
            .or_else(|| data.get("customer_id"))
            .map(|v| v.to_string().trim_matches('"').to_string())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        Record {
            id,
            data,
            metadata: crate::connectors::RecordMetadata {
                source: "mysql".to_string(),
                source_timestamp: None,
                received_at: chrono::Utc::now().to_rfc3339(),
                operation: crate::connectors::RecordOperation::Insert,
            },
        }
    }
}

#[async_trait]
impl SourceConnector for MySQLConnector {
    fn name(&self) -> &str {
        "MySQL"
    }

    fn description(&self) -> &str {
        "Read data from MySQL databases"
    }

    async fn test_connection(&self) -> crate::Result<ConnectionTest> {
        match self.pool().await {
            Ok(pool) => match sqlx::query("SELECT 1").execute(pool).await {
                Ok(_) => Ok(ConnectionTest::success("MySQL connection successful")),
                Err(e) => Ok(ConnectionTest::failure(
                    "MySQL connection failed",
                    Some(e.to_string()),
                )),
            },
            Err(e) => Ok(ConnectionTest::failure(
                "MySQL connection failed",
                Some(e.to_string()),
            )),
        }
    }

    async fn detect_schema(&self) -> crate::Result<Schema> {
        let pool = self.pool().await?;
        let rows = sqlx::query(
            "SELECT column_name, data_type, is_nullable FROM information_schema.columns \
             WHERE table_schema = DATABASE() AND table_name = ? ORDER BY ordinal_position",
        )
        .bind(&self.config.table)
        .fetch_all(pool)
        .await
        .map_err(|e| crate::Error::ConnectorError(format!("schema detection failed: {e}")))?;

        let fields = rows
            .iter()
            .map(|row| {
                let name: String = row.try_get("column_name").unwrap_or_default();
                let data_type: String = row.try_get("data_type").unwrap_or_default();
                let is_nullable: String = row
                    .try_get("is_nullable")
                    .unwrap_or_else(|_| "YES".to_string());
                crate::connectors::Field {
                    name,
                    field_type: data_type,
                    required: is_nullable == "NO",
                    description: None,
                }
            })
            .collect();

        Ok(Schema {
            fields,
            sample_records: Vec::new(),
        })
    }

    async fn read_all(&self) -> crate::Result<Vec<Record>> {
        let pool = self.pool().await?;
        let query = format!("SELECT * FROM `{}`", self.config.table);
        let rows = sqlx::query(&query)
            .fetch_all(pool)
            .await
            .map_err(|e| crate::Error::ConnectorError(format!("read_all failed: {e}")))?;
        Ok(rows.iter().map(|r| self.record_from_row(r)).collect())
    }

    async fn read_batch(&self, offset: u64, limit: u64) -> crate::Result<Vec<Record>> {
        let pool = self.pool().await?;
        let query = format!("SELECT * FROM `{}` LIMIT ? OFFSET ?", self.config.table);
        let rows = sqlx::query(&query)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(|e| crate::Error::ConnectorError(format!("read_batch failed: {e}")))?;
        Ok(rows.iter().map(|r| self.record_from_row(r)).collect())
    }

    async fn read_incremental(&self, last_value: &str) -> crate::Result<Vec<Record>> {
        let column = self.config.incremental_column.as_ref().ok_or_else(|| {
            crate::Error::ConfigError(
                "read_incremental requires MySQLConfig.incremental_column to be set".to_string(),
            )
        })?;
        let pool = self.pool().await?;
        let query = format!(
            "SELECT * FROM `{}` WHERE `{}` > ? ORDER BY `{}`",
            self.config.table, column, column
        );
        let rows = sqlx::query(&query)
            .bind(last_value)
            .fetch_all(pool)
            .await
            .map_err(|e| crate::Error::ConnectorError(format!("read_incremental failed: {e}")))?;
        Ok(rows.iter().map(|r| self.record_from_row(r)).collect())
    }

    fn capabilities(&self) -> Vec<Capability> {
        let mut caps = vec![
            Capability::Read,
            Capability::SchemaDetection,
            Capability::Batch,
        ];
        if self.config.incremental_column.is_some() {
            caps.push(Capability::IncrementalRead);
        }
        caps
    }
}

#[async_trait]
impl DestinationConnector for MySQLConnector {
    fn name(&self) -> &str {
        "MySQL"
    }

    fn description(&self) -> &str {
        "Write data to MySQL databases"
    }

    async fn test_connection(&self) -> crate::Result<ConnectionTest> {
        SourceConnector::test_connection(self).await
    }

    async fn write_record(&self, record: &Record) -> crate::Result<()> {
        self.write_batch(std::slice::from_ref(record)).await?;
        Ok(())
    }

    async fn write_batch(&self, records: &[Record]) -> crate::Result<usize> {
        let pool = self.pool().await?;
        let mut written = 0usize;

        for record in records {
            let obj = match record.data.as_object() {
                Some(obj) => obj,
                None => continue,
            };

            let columns: Vec<(&String, ColumnValue)> = obj
                .iter()
                .filter_map(|(k, v)| json_to_column_value(v).map(|cv| (k, cv)))
                .collect();
            if columns.is_empty() {
                continue;
            }

            let col_list = columns
                .iter()
                .map(|(c, _)| format!("`{c}`"))
                .collect::<Vec<_>>()
                .join(", ");
            let placeholders = columns.iter().map(|_| "?").collect::<Vec<_>>().join(", ");

            let mut query = format!(
                "INSERT INTO `{}` ({}) VALUES ({})",
                self.config.table, col_list, placeholders
            );

            if self.config.upsert_key.is_some() {
                let updates = columns
                    .iter()
                    .filter(|(c, _)| Some(*c) != self.config.upsert_key.as_ref())
                    .map(|(c, _)| format!("`{c}` = VALUES(`{c}`)"))
                    .collect::<Vec<_>>()
                    .join(", ");
                if !updates.is_empty() {
                    query.push_str(&format!(" ON DUPLICATE KEY UPDATE {updates}"));
                }
            }

            let mut q = sqlx::query(&query);
            for (_, value) in &columns {
                q = match value {
                    ColumnValue::Bool(b) => q.bind(*b),
                    ColumnValue::I64(i) => q.bind(*i),
                    ColumnValue::F64(f) => q.bind(*f),
                    ColumnValue::Text(s) => q.bind(s.clone()),
                    ColumnValue::Json(j) => q.bind(j.clone()),
                };
            }

            q.execute(pool)
                .await
                .map_err(|e| crate::Error::ConnectorError(format!("write failed: {e}")))?;
            written += 1;
        }

        Ok(written)
    }

    async fn validate_records(&self, records: &[Record]) -> crate::Result<()> {
        for record in records {
            if !record.data.is_object() {
                return Err(crate::Error::ConfigError(format!(
                    "record {} is not a JSON object and cannot be written to a SQL table",
                    record.id
                )));
            }
        }
        Ok(())
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::Write, Capability::Batch]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_config() -> MySQLConfig {
        MySQLConfig::new(
            "localhost",
            3306,
            "test_db",
            "test_user",
            "test_password",
            "customers",
        )
    }

    #[test]
    fn test_connection_string_format() {
        let config = test_config();
        let conn_str = config.connection_string();
        assert!(conn_str.contains("mysql://"));
        assert!(conn_str.contains("test_user"));
        assert!(conn_str.contains("localhost:3306"));
        assert!(conn_str.contains("test_db"));
    }

    #[test]
    fn test_from_url() {
        let url = "mysql://user:pass@localhost:3306/testdb";
        let result = MySQLConfig::from_url(url);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 3306);
        assert_eq!(config.username, "user");
        assert_eq!(config.password, "pass");
        assert_eq!(config.database, "testdb");
    }

    #[test]
    fn test_from_url_custom_port() {
        let url = "mysql://root:secret@db.example.com:3307/prod";
        let result = MySQLConfig::from_url(url);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.host, "db.example.com");
        assert_eq!(config.port, 3307);
    }

    #[tokio::test]
    async fn capabilities_reflect_incremental_configuration() {
        let connector = MySQLConnector::new(test_config());
        assert!(!SourceConnector::capabilities(&connector).contains(&Capability::IncrementalRead));

        let connector = MySQLConnector::new(test_config().with_incremental_column("updated_at"));
        assert!(SourceConnector::capabilities(&connector).contains(&Capability::IncrementalRead));
    }

    // -- Real, Docker-backed round-trip tests -----------------------------
    // Requires a live MySQL instance; gated behind `#[ignore]`. Run explicitly:
    //
    //   docker run --rm -d -p 3307:3306 -e MYSQL_ROOT_PASSWORD=mysql \
    //       -e MYSQL_DATABASE=pyreverseetl_test --name pyreverseetl-mysql-test mysql:8
    //   PYREVERSEETL_TEST_MYSQL_PORT=3307 cargo test -p pyreverseetl-core \
    //       --lib connectors::mysql -- --ignored

    fn docker_test_config(table: &str) -> MySQLConfig {
        let port: u16 = std::env::var("PYREVERSEETL_TEST_MYSQL_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3307);
        MySQLConfig::new(
            "127.0.0.1",
            port,
            "pyreverseetl_test",
            "root",
            "mysql",
            table,
        )
        .with_upsert_key("id")
    }

    #[tokio::test]
    #[ignore]
    async fn real_mysql_write_then_read_round_trip() {
        let config = docker_test_config("lineage_test_customers");
        let connector = MySQLConnector::new(config.clone());
        let pool = connector.pool().await.expect("connect to test mysql");

        sqlx::query(&format!(
            "CREATE TABLE IF NOT EXISTS `{}` (id INT PRIMARY KEY, name VARCHAR(255), ltv DOUBLE)",
            config.table
        ))
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(&format!("DELETE FROM `{}`", config.table))
            .execute(pool)
            .await
            .unwrap();

        let records: Vec<Record> = (0..5)
            .map(|i| Record {
                id: i.to_string(),
                data: json!({"id": i, "name": format!("Customer {i}"), "ltv": (i as f64) * 100.5}),
                metadata: crate::connectors::RecordMetadata {
                    source: "test".to_string(),
                    source_timestamp: None,
                    received_at: chrono::Utc::now().to_rfc3339(),
                    operation: crate::connectors::RecordOperation::Insert,
                },
            })
            .collect();

        let written = DestinationConnector::write_batch(&connector, &records)
            .await
            .unwrap();
        assert_eq!(written, 5);

        let read_back = SourceConnector::read_all(&connector).await.unwrap();
        assert_eq!(read_back.len(), 5);
        let customer_2 = read_back.iter().find(|r| r.data["id"] == json!(2)).unwrap();
        assert_eq!(customer_2.data["name"], json!("Customer 2"));

        // Upsert path
        let updated = Record {
            id: "2".to_string(),
            data: json!({"id": 2, "name": "Updated Customer", "ltv": 999.0}),
            metadata: crate::connectors::RecordMetadata {
                source: "test".to_string(),
                source_timestamp: None,
                received_at: chrono::Utc::now().to_rfc3339(),
                operation: crate::connectors::RecordOperation::Update,
            },
        };
        DestinationConnector::write_batch(&connector, std::slice::from_ref(&updated))
            .await
            .unwrap();
        let read_back = SourceConnector::read_all(&connector).await.unwrap();
        assert_eq!(read_back.len(), 5, "upsert must not duplicate rows");
        let customer_2 = read_back.iter().find(|r| r.data["id"] == json!(2)).unwrap();
        assert_eq!(customer_2.data["name"], json!("Updated Customer"));

        sqlx::query(&format!("DROP TABLE `{}`", config.table))
            .execute(pool)
            .await
            .unwrap();
    }
}
