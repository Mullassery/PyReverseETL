// PostgreSQL Connector Implementation
// Real reads and writes over a live PostgreSQL connection via sqlx.
// Supports: Read, Write, Schema Detection, Incremental Reads, Batch Operations, Upsert

use crate::connectors::{
    Capability, ConnectionTest, DestinationConnector, Record, Schema, SourceConnector,
};
use async_trait::async_trait;
#[cfg(test)]
use serde_json::json;
use serde_json::{Map, Value};
use sqlx::postgres::{PgPoolOptions, PgRow};
use sqlx::PgPool;
use sqlx::{Column, Row, TypeInfo};
use tokio::sync::OnceCell;

#[derive(Debug, Clone)]
pub struct PostgreSQLConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
    /// Table this connector reads from / writes to.
    pub table: String,
    /// Column used for `read_incremental`. Reads honestly fail if this isn't
    /// configured, rather than fabricating rows.
    pub incremental_column: Option<String>,
    /// Column used to upsert (`ON CONFLICT (key) DO UPDATE`) on write. Without
    /// it, writes are plain `INSERT`.
    pub upsert_key: Option<String>,
}

impl PostgreSQLConfig {
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

    pub fn connection_string(&self) -> String {
        format!(
            "postgresql://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database
        )
    }
}

/// Convert a JSON value into something we can bind as a SQL parameter,
/// skipping `null` (the column keeps its default/NULL rather than us having to
/// guess a Postgres type for an untyped null parameter).
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

enum ColumnValue {
    Bool(bool),
    I64(i64),
    F64(f64),
    Text(String),
    Json(Value),
}

/// Generic Postgres row -> JSON object mapping so this connector can work
/// against any table without compile-time knowledge of its schema.
fn pg_row_to_json(row: &PgRow) -> Value {
    let mut map = Map::new();
    for col in row.columns() {
        let name = col.name();
        let type_name = col.type_info().name();
        let value: Value = match type_name {
            "INT2" => row
                .try_get::<i16, _>(name)
                .map(Value::from)
                .unwrap_or(Value::Null),
            "INT4" => row
                .try_get::<i32, _>(name)
                .map(Value::from)
                .unwrap_or(Value::Null),
            "INT8" => row
                .try_get::<i64, _>(name)
                .map(Value::from)
                .unwrap_or(Value::Null),
            "FLOAT4" => row
                .try_get::<f32, _>(name)
                .map(|v| Value::from(v as f64))
                .unwrap_or(Value::Null),
            "FLOAT8" | "NUMERIC" => row
                .try_get::<f64, _>(name)
                .map(Value::from)
                .unwrap_or(Value::Null),
            "BOOL" => row
                .try_get::<bool, _>(name)
                .map(Value::from)
                .unwrap_or(Value::Null),
            "JSON" | "JSONB" => row.try_get::<Value, _>(name).unwrap_or(Value::Null),
            "TIMESTAMP" | "TIMESTAMPTZ" => row
                .try_get::<chrono::DateTime<chrono::Utc>, _>(name)
                .map(|v| Value::from(v.to_rfc3339()))
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

pub struct PostgreSQLConnector {
    config: PostgreSQLConfig,
    pool: OnceCell<PgPool>,
}

impl PostgreSQLConnector {
    pub fn new(config: PostgreSQLConfig) -> Self {
        Self {
            config,
            pool: OnceCell::new(),
        }
    }

    pub fn connection_string(&self) -> String {
        self.config.connection_string()
    }

    async fn pool(&self) -> crate::Result<&PgPool> {
        self.pool
            .get_or_try_init(|| async {
                PgPoolOptions::new()
                    .max_connections(5)
                    .connect(&self.config.connection_string())
                    .await
                    .map_err(|e| {
                        crate::Error::ConnectorError(format!("PostgreSQL connect failed: {e}"))
                    })
            })
            .await
    }

    /// Look up a column's `information_schema.columns.data_type` so callers can
    /// cast an incoming text parameter to the right type before comparing it
    /// against that column (see `read_incremental`).
    async fn column_pg_type(&self, column: &str, pool: &PgPool) -> crate::Result<String> {
        let row = sqlx::query(
            "SELECT data_type FROM information_schema.columns WHERE table_name = $1 AND column_name = $2",
        )
        .bind(&self.config.table)
        .bind(column)
        .fetch_optional(pool)
        .await
        .map_err(|e| crate::Error::ConnectorError(format!("column type lookup failed: {e}")))?;
        Ok(row
            .and_then(|r| r.try_get::<String, _>("data_type").ok())
            .unwrap_or_else(|| "text".to_string()))
    }

    fn record_from_row(&self, row: &PgRow) -> Record {
        let data = pg_row_to_json(row);
        let id = data
            .get("id")
            .map(|v| v.to_string().trim_matches('"').to_string())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        Record {
            id,
            data,
            metadata: crate::connectors::RecordMetadata {
                source: "postgres".to_string(),
                source_timestamp: None,
                received_at: chrono::Utc::now().to_rfc3339(),
                operation: crate::connectors::RecordOperation::Insert,
            },
        }
    }
}

#[async_trait]
impl SourceConnector for PostgreSQLConnector {
    fn name(&self) -> &str {
        "PostgreSQL"
    }

    fn description(&self) -> &str {
        "Read data from PostgreSQL databases"
    }

    async fn test_connection(&self) -> crate::Result<ConnectionTest> {
        match self.pool().await {
            Ok(pool) => match sqlx::query("SELECT 1").execute(pool).await {
                Ok(_) => Ok(ConnectionTest::success("PostgreSQL connection successful")),
                Err(e) => Ok(ConnectionTest::failure(
                    "PostgreSQL connection failed",
                    Some(e.to_string()),
                )),
            },
            Err(e) => Ok(ConnectionTest::failure(
                "PostgreSQL connection failed",
                Some(e.to_string()),
            )),
        }
    }

    async fn detect_schema(&self) -> crate::Result<Schema> {
        let pool = self.pool().await?;
        let rows = sqlx::query(
            "SELECT column_name, data_type, is_nullable FROM information_schema.columns \
             WHERE table_name = $1 ORDER BY ordinal_position",
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
        let query = format!("SELECT * FROM \"{}\"", self.config.table);
        let rows = sqlx::query(&query)
            .fetch_all(pool)
            .await
            .map_err(|e| crate::Error::ConnectorError(format!("read_all failed: {e}")))?;
        Ok(rows.iter().map(|r| self.record_from_row(r)).collect())
    }

    async fn read_batch(&self, offset: u64, limit: u64) -> crate::Result<Vec<Record>> {
        let pool = self.pool().await?;
        let query = format!("SELECT * FROM \"{}\" LIMIT $1 OFFSET $2", self.config.table);
        let rows = sqlx::query(&query)
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(pool)
            .await
            .map_err(|e| crate::Error::ConnectorError(format!("read_batch failed: {e}")))?;
        Ok(rows.iter().map(|r| self.record_from_row(r)).collect())
    }

    async fn read_incremental(&self, last_value: &str) -> crate::Result<Vec<Record>> {
        let column = self.config.incremental_column.as_ref().ok_or_else(|| {
            crate::Error::ConfigError(
                "read_incremental requires PostgreSQLConfig.incremental_column to be set"
                    .to_string(),
            )
        })?;
        let pool = self.pool().await?;

        // `last_value` always arrives as text, but the incremental column can be
        // any type (integer id, timestamp, etc). sqlx binds `&str` as a TEXT
        // parameter, and Postgres won't implicitly compare `integer > text` --
        // so cast the *parameter* to the column's real type in the query. The
        // cast keyword comes from a fixed whitelist (never interpolated from
        // user input) so this stays injection-safe despite being string-built SQL.
        let pg_type = self.column_pg_type(column, pool).await?;
        let cast = match pg_type.as_str() {
            "integer" | "smallint" => "::integer",
            "bigint" => "::bigint",
            "numeric" | "real" | "double precision" => "::double precision",
            "timestamp without time zone" | "timestamp with time zone" => "::timestamptz",
            "date" => "::date",
            _ => "::text",
        };

        let query = format!(
            "SELECT * FROM \"{}\" WHERE \"{}\" > $1{cast} ORDER BY \"{}\"",
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
impl DestinationConnector for PostgreSQLConnector {
    fn name(&self) -> &str {
        "PostgreSQL"
    }

    fn description(&self) -> &str {
        "Write data to PostgreSQL databases"
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
                .map(|(c, _)| format!("\"{c}\""))
                .collect::<Vec<_>>()
                .join(", ");
            let placeholders = (1..=columns.len())
                .map(|i| format!("${i}"))
                .collect::<Vec<_>>()
                .join(", ");

            let mut query = format!(
                "INSERT INTO \"{}\" ({}) VALUES ({})",
                self.config.table, col_list, placeholders
            );

            if let Some(key) = &self.config.upsert_key {
                let updates = columns
                    .iter()
                    .filter(|(c, _)| *c != key)
                    .map(|(c, _)| format!("\"{c}\" = EXCLUDED.\"{c}\""))
                    .collect::<Vec<_>>()
                    .join(", ");
                if updates.is_empty() {
                    query.push_str(&format!(" ON CONFLICT (\"{key}\") DO NOTHING"));
                } else {
                    query.push_str(&format!(" ON CONFLICT (\"{key}\") DO UPDATE SET {updates}"));
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

    fn test_config() -> PostgreSQLConfig {
        PostgreSQLConfig::new("localhost", 5432, "testdb", "user", "pass", "customers")
    }

    #[test]
    fn connection_string_format() {
        let config = test_config();
        let conn_str = config.connection_string();
        assert!(conn_str.contains("postgresql://"));
        assert!(conn_str.contains("user:pass@localhost:5432/testdb"));
    }

    #[test]
    fn json_to_column_value_skips_null() {
        assert!(json_to_column_value(&json!(null)).is_none());
        assert!(matches!(
            json_to_column_value(&json!(5)),
            Some(ColumnValue::I64(5))
        ));
        assert!(matches!(
            json_to_column_value(&json!(5.5)),
            Some(ColumnValue::F64(_))
        ));
        assert!(matches!(
            json_to_column_value(&json!(true)),
            Some(ColumnValue::Bool(true))
        ));
        assert!(matches!(
            json_to_column_value(&json!("x")),
            Some(ColumnValue::Text(_))
        ));
    }

    #[test]
    fn config_builders_set_optional_fields() {
        let config = test_config()
            .with_incremental_column("updated_at")
            .with_upsert_key("id");
        assert_eq!(config.incremental_column.as_deref(), Some("updated_at"));
        assert_eq!(config.upsert_key.as_deref(), Some("id"));
    }

    #[tokio::test]
    async fn capabilities_reflect_incremental_configuration() {
        let connector = PostgreSQLConnector::new(test_config());
        assert!(!SourceConnector::capabilities(&connector).contains(&Capability::IncrementalRead));

        let connector =
            PostgreSQLConnector::new(test_config().with_incremental_column("updated_at"));
        assert!(SourceConnector::capabilities(&connector).contains(&Capability::IncrementalRead));
    }

    // -- Real, Docker-backed round-trip tests -----------------------------
    // These require a live PostgreSQL instance and are gated behind `#[ignore]`
    // so `cargo test` stays hermetic by default. Run them explicitly with a
    // real database, e.g.:
    //
    //   docker run --rm -d -p 5439:5432 -e POSTGRES_PASSWORD=postgres \
    //       -e POSTGRES_DB=pyreverseetl_test --name pyreverseetl-pg-test postgres:16
    //   PYREVERSEETL_TEST_PG_PORT=5439 cargo test -p pyreverseetl-core \
    //       --lib connectors::postgres -- --ignored

    fn docker_test_config(table: &str) -> PostgreSQLConfig {
        let port: u16 = std::env::var("PYREVERSEETL_TEST_PG_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(5439);
        PostgreSQLConfig::new(
            "127.0.0.1",
            port,
            "pyreverseetl_test",
            "postgres",
            "postgres",
            table,
        )
        .with_upsert_key("id")
    }

    #[tokio::test]
    #[ignore]
    async fn real_postgres_write_then_read_round_trip() {
        let config = docker_test_config("lineage_test_customers");
        let connector = PostgreSQLConnector::new(config.clone());
        let pool = connector.pool().await.expect("connect to test postgres");

        sqlx::query(&format!(
            "CREATE TABLE IF NOT EXISTS \"{}\" (id INT PRIMARY KEY, name TEXT, ltv DOUBLE PRECISION)",
            config.table
        ))
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(&format!("DELETE FROM \"{}\"", config.table))
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
        assert_eq!(customer_2.data["ltv"], json!(201.0));

        // Upsert path: re-write id=2 with a new name, row count stays 5.
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

        sqlx::query(&format!("DROP TABLE \"{}\"", config.table))
            .execute(pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn real_postgres_schema_detection() {
        let config = docker_test_config("lineage_test_schema");
        let connector = PostgreSQLConnector::new(config.clone());
        let pool = connector.pool().await.expect("connect to test postgres");

        sqlx::query(&format!(
            "CREATE TABLE IF NOT EXISTS \"{}\" (id INT PRIMARY KEY, email TEXT NOT NULL)",
            config.table
        ))
        .execute(pool)
        .await
        .unwrap();

        let schema = SourceConnector::detect_schema(&connector).await.unwrap();
        assert!(schema.fields.iter().any(|f| f.name == "id"));
        let email_field = schema.fields.iter().find(|f| f.name == "email").unwrap();
        assert!(
            email_field.required,
            "NOT NULL column must be reported as required"
        );

        sqlx::query(&format!("DROP TABLE \"{}\"", config.table))
            .execute(pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn real_postgres_incremental_read() {
        let config = docker_test_config("lineage_test_incremental").with_incremental_column("id");
        let connector = PostgreSQLConnector::new(config.clone());
        let pool = connector.pool().await.expect("connect to test postgres");

        sqlx::query(&format!(
            "CREATE TABLE IF NOT EXISTS \"{}\" (id INT PRIMARY KEY, name TEXT)",
            config.table
        ))
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(&format!("DELETE FROM \"{}\"", config.table))
            .execute(pool)
            .await
            .unwrap();

        for i in 1..=10 {
            sqlx::query(&format!(
                "INSERT INTO \"{}\" (id, name) VALUES ($1, $2)",
                config.table
            ))
            .bind(i)
            .bind(format!("Row {i}"))
            .execute(pool)
            .await
            .unwrap();
        }

        let incremental = SourceConnector::read_incremental(&connector, "5")
            .await
            .unwrap();
        assert_eq!(
            incremental.len(),
            5,
            "only rows with id > 5 should come back"
        );

        sqlx::query(&format!("DROP TABLE \"{}\"", config.table))
            .execute(pool)
            .await
            .unwrap();
    }
}
