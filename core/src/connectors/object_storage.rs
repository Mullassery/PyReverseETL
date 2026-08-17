/// Object Storage Connectors (S3-compatible)
///
/// The `s3` provider is real: it talks to Amazon S3 or any S3-compatible
/// store (MinIO, in local/testing environments) via `aws-sdk-s3`, including a
/// configurable custom endpoint + path-style addressing for MinIO.
///
/// Records are stored as newline-delimited JSON (`.jsonl`) or CSV, both of
/// which are implemented for real (serialize on write, parse on read).
/// Parquet/Avro/ORC/Iceberg/Delta are declared in [`FileFormat`] for
/// forward-compatibility but are honestly unimplemented -- reads/writes with
/// those formats return an error rather than silently no-op'ing.
///
/// The `gcs`/`azure` providers are likewise honestly unimplemented (real
/// GCS/Azure SDK integration needs credentials this environment doesn't have)
/// and return an explicit error instead of fabricating success.
use super::{Capability, ConnectionTest, Record};
use async_trait::async_trait;
use aws_sdk_s3::primitives::ByteStream;
use serde::{Deserialize, Serialize};

/// File format for object storage
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileFormat {
    /// CSV - simple text format (implemented)
    CSV,
    /// Newline-delimited JSON (implemented)
    JSON,
    /// Parquet - columnar format, highly compressed (not yet implemented)
    Parquet,
    /// Avro - binary format with schema (not yet implemented)
    Avro,
    /// ORC - Hive columnar format (not yet implemented)
    ORC,
    /// Apache Iceberg - data lake table format (not yet implemented)
    Iceberg,
    /// Delta Lake - ACID transactions (not yet implemented)
    Delta,
}

impl FileFormat {
    fn extension(&self) -> &'static str {
        match self {
            FileFormat::CSV => "csv",
            FileFormat::JSON => "jsonl",
            FileFormat::Parquet => "parquet",
            FileFormat::Avro => "avro",
            FileFormat::ORC => "orc",
            FileFormat::Iceberg => "iceberg",
            FileFormat::Delta => "delta",
        }
    }

    fn is_implemented(&self) -> bool {
        matches!(self, FileFormat::CSV | FileFormat::JSON)
    }
}

/// Table format for object storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableFormat {
    /// Format type
    pub format: FileFormat,
    /// Compression codec (gzip, snappy, zstd)
    pub compression: Option<String>,
    /// Partition columns (date, hour, region)
    pub partition_columns: Vec<String>,
    /// Partition path pattern (date=YYYY-MM-DD/hour=HH)
    pub partition_pattern: Option<String>,
}

impl TableFormat {
    /// CSV format
    pub fn csv() -> Self {
        Self {
            format: FileFormat::CSV,
            compression: None,
            partition_columns: vec![],
            partition_pattern: None,
        }
    }

    /// Newline-delimited JSON format (default; always implemented)
    pub fn json() -> Self {
        Self {
            format: FileFormat::JSON,
            compression: None,
            partition_columns: vec![],
            partition_pattern: None,
        }
    }

    /// Parquet format (efficient) -- declared for API compatibility; reads/writes
    /// using this format return an explicit "not yet implemented" error.
    pub fn parquet() -> Self {
        Self {
            format: FileFormat::Parquet,
            compression: Some("snappy".to_string()),
            partition_columns: vec![],
            partition_pattern: None,
        }
    }

    /// Delta Lake (transactional) -- not yet implemented, see [`TableFormat::parquet`].
    pub fn delta() -> Self {
        Self {
            format: FileFormat::Delta,
            compression: None,
            partition_columns: vec!["date".to_string()],
            partition_pattern: Some("date=YYYY-MM-DD".to_string()),
        }
    }

    /// Iceberg (modern data lake) -- not yet implemented, see [`TableFormat::parquet`].
    pub fn iceberg() -> Self {
        Self {
            format: FileFormat::Iceberg,
            compression: Some("zstd".to_string()),
            partition_columns: vec!["date".to_string()],
            partition_pattern: Some("date=YYYY-MM-DD".to_string()),
        }
    }

    /// Add partition column
    pub fn with_partition(mut self, column: &str) -> Self {
        self.partition_columns.push(column.to_string());
        self
    }

    /// Add compression
    pub fn with_compression(mut self, codec: &str) -> Self {
        self.compression = Some(codec.to_string());
        self
    }
}

/// Object storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectStorageConfig {
    /// Provider: s3 (real), gcs/azure (honestly unimplemented)
    pub provider: String,
    /// Bucket name
    pub bucket: String,
    /// Base path/prefix in bucket
    pub path: String,
    /// File format and partitioning
    pub table_format: TableFormat,
    /// Overwrite existing files
    pub overwrite: bool,
    /// Create new folders automatically
    pub create_folders: bool,
    /// Custom endpoint (e.g. `http://localhost:9000` for MinIO). `None` means AWS S3.
    pub endpoint: Option<String>,
    /// AWS region (also required, but ignored, by most S3-compatible stores)
    pub region: String,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
    /// Required for MinIO and most self-hosted S3-compatible stores
    pub force_path_style: bool,
}

impl ObjectStorageConfig {
    /// S3 configuration (JSON lines by default; the only format actually implemented
    /// at the moment along with CSV)
    pub fn s3(bucket: &str, path: &str) -> Self {
        Self {
            provider: "s3".to_string(),
            bucket: bucket.to_string(),
            path: path.to_string(),
            table_format: TableFormat::json(),
            overwrite: false,
            create_folders: true,
            endpoint: None,
            region: "us-east-1".to_string(),
            access_key: None,
            secret_key: None,
            force_path_style: false,
        }
    }

    /// GCS configuration (not yet implemented -- connectors return an explicit error)
    pub fn gcs(bucket: &str, path: &str) -> Self {
        Self {
            provider: "gcs".to_string(),
            bucket: bucket.to_string(),
            path: path.to_string(),
            table_format: TableFormat::json(),
            overwrite: false,
            create_folders: true,
            endpoint: None,
            region: "us-east-1".to_string(),
            access_key: None,
            secret_key: None,
            force_path_style: false,
        }
    }

    /// Azure configuration (not yet implemented -- connectors return an explicit error)
    pub fn azure(container: &str, path: &str) -> Self {
        Self {
            provider: "azure".to_string(),
            bucket: container.to_string(),
            path: path.to_string(),
            table_format: TableFormat::json(),
            overwrite: false,
            create_folders: true,
            endpoint: None,
            region: "us-east-1".to_string(),
            access_key: None,
            secret_key: None,
            force_path_style: false,
        }
    }

    /// Point at a MinIO / S3-compatible endpoint instead of AWS S3.
    pub fn with_minio(mut self, endpoint: &str, access_key: &str, secret_key: &str) -> Self {
        self.endpoint = Some(endpoint.to_string());
        self.access_key = Some(access_key.to_string());
        self.secret_key = Some(secret_key.to_string());
        self.force_path_style = true;
        self
    }

    /// Set table format
    pub fn with_format(mut self, format: TableFormat) -> Self {
        self.table_format = format;
        self
    }

    fn normalized_prefix(&self) -> String {
        self.path.trim_matches('/').to_string()
    }
}

async fn build_s3_client(config: &ObjectStorageConfig) -> crate::Result<aws_sdk_s3::Client> {
    if config.provider != "s3" {
        return Err(crate::Error::ConfigError(format!(
            "object storage provider '{}' is not yet implemented; only 's3' (AWS S3 or an S3-compatible store like MinIO) is real",
            config.provider
        )));
    }

    let region = aws_config::Region::new(config.region.clone());
    let mut loader = aws_config::defaults(aws_config::BehaviorVersion::latest()).region(region);

    if let (Some(access_key), Some(secret_key)) = (&config.access_key, &config.secret_key) {
        let credentials = aws_credential_types::Credentials::new(
            access_key.clone(),
            secret_key.clone(),
            None,
            None,
            "pyreverseetl-static",
        );
        loader = loader.credentials_provider(credentials);
    }

    let shared_config = loader.load().await;
    let mut s3_config_builder = aws_sdk_s3::config::Builder::from(&shared_config);

    if let Some(endpoint) = &config.endpoint {
        s3_config_builder = s3_config_builder.endpoint_url(endpoint);
    }
    if config.force_path_style {
        s3_config_builder = s3_config_builder.force_path_style(true);
    }

    Ok(aws_sdk_s3::Client::from_conf(s3_config_builder.build()))
}

/// Serialize records to bytes for the configured format. Only CSV and JSON are
/// implemented; anything else is an explicit error.
fn serialize_records(records: &[Record], format: FileFormat) -> crate::Result<Vec<u8>> {
    match format {
        FileFormat::JSON => {
            let mut out = String::new();
            for record in records {
                out.push_str(&record.data.to_string());
                out.push('\n');
            }
            Ok(out.into_bytes())
        }
        FileFormat::CSV => {
            if records.is_empty() {
                return Ok(Vec::new());
            }
            let mut columns: Vec<String> = Vec::new();
            for record in records {
                if let Some(obj) = record.data.as_object() {
                    for key in obj.keys() {
                        if !columns.contains(key) {
                            columns.push(key.clone());
                        }
                    }
                }
            }
            let mut out = String::new();
            out.push_str(&columns.join(","));
            out.push('\n');
            for record in records {
                let row: Vec<String> = columns
                    .iter()
                    .map(|c| {
                        record
                            .data
                            .get(c)
                            .map(|v| csv_escape(&value_to_csv_cell(v)))
                            .unwrap_or_default()
                    })
                    .collect();
                out.push_str(&row.join(","));
                out.push('\n');
            }
            Ok(out.into_bytes())
        }
        other => Err(crate::Error::ConfigError(format!(
            "file format {other:?} is not yet implemented (only CSV and JSON are real)"
        ))),
    }
}

fn value_to_csv_cell(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn csv_escape(cell: &str) -> String {
    if cell.contains(',') || cell.contains('"') || cell.contains('\n') {
        format!("\"{}\"", cell.replace('"', "\"\""))
    } else {
        cell.to_string()
    }
}

/// Parse one CSV line into cells, respecting quoted fields (so a
/// comma or an escaped `""` inside a `csv_escape`-quoted field doesn't
/// split into extra cells). Mirrors `csv_escape` -- a naive `line.split(',')`
/// would break on any value written by it that actually contains a comma.
fn parse_csv_line(line: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                // Escaped quote inside a quoted field.
                current.push('"');
                chars.next();
            }
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                cells.push(std::mem::take(&mut current));
            }
            other => current.push(other),
        }
    }
    cells.push(current);
    cells
}

/// Parse bytes downloaded from object storage back into records. Mirrors
/// [`serialize_records`]; only CSV and JSON are implemented.
fn deserialize_records(
    bytes: &[u8],
    format: FileFormat,
    source_key: &str,
) -> crate::Result<Vec<Record>> {
    let text = String::from_utf8_lossy(bytes);
    match format {
        FileFormat::JSON => text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|line| {
                let data: serde_json::Value = serde_json::from_str(line).map_err(|e| {
                    crate::Error::ConfigError(format!("invalid JSON line in {source_key}: {e}"))
                })?;
                Ok(record_from_value(data))
            })
            .collect(),
        FileFormat::CSV => {
            let mut lines = text.lines();
            let header = match lines.next() {
                Some(h) => parse_csv_line(h),
                None => return Ok(Vec::new()),
            };
            lines
                .filter(|l| !l.trim().is_empty())
                .map(|line| {
                    let cells = parse_csv_line(line);
                    let mut map = serde_json::Map::new();
                    for (i, col) in header.iter().enumerate() {
                        let value = cells.get(i).cloned().unwrap_or_default();
                        map.insert(col.clone(), serde_json::Value::String(value));
                    }
                    Ok(record_from_value(serde_json::Value::Object(map)))
                })
                .collect()
        }
        other => Err(crate::Error::ConfigError(format!(
            "file format {other:?} is not yet implemented (only CSV and JSON are real)"
        ))),
    }
}

fn record_from_value(data: serde_json::Value) -> Record {
    let id = data
        .get("id")
        .map(|v| v.to_string().trim_matches('"').to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    Record {
        id,
        data,
        metadata: crate::connectors::RecordMetadata {
            source: "s3".to_string(),
            source_timestamp: None,
            received_at: chrono::Utc::now().to_rfc3339(),
            operation: crate::connectors::RecordOperation::Insert,
        },
    }
}

/// File operations (copy, move, delete)
#[async_trait]
pub trait FileOperations: Send + Sync {
    /// Copy file from source to destination
    async fn copy_file(&self, source: &str, destination: &str) -> crate::Result<()>;

    /// Move file (copy + delete)
    async fn move_file(&self, source: &str, destination: &str) -> crate::Result<()>;

    /// Delete file
    async fn delete_file(&self, path: &str) -> crate::Result<()>;

    /// Delete folder recursively
    async fn delete_folder(&self, path: &str) -> crate::Result<()>;

    /// List files in folder
    async fn list_files(&self, path: &str) -> crate::Result<Vec<String>>;

    /// Create folder if not exists
    async fn create_folder(&self, path: &str) -> crate::Result<()>;
}

/// Object storage destination (for writing records)
#[derive(Debug, Clone)]
pub struct ObjectStorageDestination {
    pub config: ObjectStorageConfig,
}

#[async_trait]
impl FileOperations for ObjectStorageDestination {
    async fn copy_file(&self, source: &str, destination: &str) -> crate::Result<()> {
        let client = build_s3_client(&self.config).await?;
        let copy_source = format!("{}/{}", self.config.bucket, source.trim_start_matches('/'));
        client
            .copy_object()
            .bucket(&self.config.bucket)
            .copy_source(copy_source)
            .key(destination)
            .send()
            .await
            .map_err(|e| crate::Error::ConnectorError(format!("S3 copy_object failed: {e}")))?;
        Ok(())
    }

    async fn move_file(&self, source: &str, destination: &str) -> crate::Result<()> {
        self.copy_file(source, destination).await?;
        self.delete_file(source).await?;
        Ok(())
    }

    async fn delete_file(&self, path: &str) -> crate::Result<()> {
        let client = build_s3_client(&self.config).await?;
        client
            .delete_object()
            .bucket(&self.config.bucket)
            .key(path)
            .send()
            .await
            .map_err(|e| crate::Error::ConnectorError(format!("S3 delete_object failed: {e}")))?;
        Ok(())
    }

    async fn delete_folder(&self, path: &str) -> crate::Result<()> {
        let keys = self.list_files(path).await?;
        let client = build_s3_client(&self.config).await?;
        for key in keys {
            client
                .delete_object()
                .bucket(&self.config.bucket)
                .key(&key)
                .send()
                .await
                .map_err(|e| {
                    crate::Error::ConnectorError(format!("S3 delete_object failed: {e}"))
                })?;
        }
        Ok(())
    }

    async fn list_files(&self, path: &str) -> crate::Result<Vec<String>> {
        let client = build_s3_client(&self.config).await?;
        let resp = client
            .list_objects_v2()
            .bucket(&self.config.bucket)
            .prefix(path)
            .send()
            .await
            .map_err(|e| crate::Error::ConnectorError(format!("S3 list_objects_v2 failed: {e}")))?;
        Ok(resp
            .contents()
            .iter()
            .filter_map(|o| o.key().map(|k| k.to_string()))
            .collect())
    }

    async fn create_folder(&self, path: &str) -> crate::Result<()> {
        let client = build_s3_client(&self.config).await?;
        let key = if path.ends_with('/') {
            path.to_string()
        } else {
            format!("{path}/")
        };
        client
            .put_object()
            .bucket(&self.config.bucket)
            .key(key)
            .body(ByteStream::from_static(b""))
            .send()
            .await
            .map_err(|e| {
                crate::Error::ConnectorError(format!("S3 put_object (folder marker) failed: {e}"))
            })?;
        Ok(())
    }
}

#[async_trait]
impl super::DestinationConnector for ObjectStorageDestination {
    fn name(&self) -> &str {
        match self.config.provider.as_str() {
            "s3" => "Amazon S3",
            "gcs" => "Google Cloud Storage",
            "azure" => "Azure Blob Storage",
            _ => "Object Storage",
        }
    }

    fn description(&self) -> &str {
        "Write data to S3-compatible object storage (real; JSON/CSV formats)"
    }

    async fn test_connection(&self) -> crate::Result<ConnectionTest> {
        let client = match build_s3_client(&self.config).await {
            Ok(c) => c,
            Err(e) => {
                return Ok(ConnectionTest::failure(
                    "Object storage connection failed",
                    Some(e.to_string()),
                ))
            }
        };
        match client
            .head_bucket()
            .bucket(&self.config.bucket)
            .send()
            .await
        {
            Ok(_) => Ok(ConnectionTest::success(&format!(
                "Connected to {} bucket: {}",
                self.config.provider, self.config.bucket
            ))),
            Err(e) => Ok(ConnectionTest::failure(
                "Bucket not reachable",
                Some(e.to_string()),
            )),
        }
    }

    async fn write_record(&self, record: &Record) -> crate::Result<()> {
        self.write_batch(std::slice::from_ref(record)).await?;
        Ok(())
    }

    async fn write_batch(&self, records: &[Record]) -> crate::Result<usize> {
        if records.is_empty() {
            return Ok(0);
        }
        if !self.config.table_format.format.is_implemented() {
            return Err(crate::Error::ConfigError(format!(
                "{:?} is not yet implemented for object storage writes (only CSV/JSON are real)",
                self.config.table_format.format
            )));
        }

        let bytes = serialize_records(records, self.config.table_format.format)?;
        let client = build_s3_client(&self.config).await?;
        let prefix = self.config.normalized_prefix();
        let key = if self.config.overwrite {
            format!(
                "{prefix}/data.{}",
                self.config.table_format.format.extension()
            )
        } else {
            format!(
                "{prefix}/{}-{}.{}",
                chrono::Utc::now().format("%Y%m%dT%H%M%S%.f"),
                uuid::Uuid::new_v4(),
                self.config.table_format.format.extension()
            )
        };

        client
            .put_object()
            .bucket(&self.config.bucket)
            .key(&key)
            .body(ByteStream::from(bytes))
            .send()
            .await
            .map_err(|e| crate::Error::ConnectorError(format!("S3 put_object failed: {e}")))?;

        Ok(records.len())
    }

    async fn validate_records(&self, records: &[Record]) -> crate::Result<()> {
        for record in records {
            if !record.data.is_object() {
                return Err(crate::Error::ConfigError(format!(
                    "record {} is not a JSON object and cannot be serialized to CSV/JSON",
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

/// Object storage source (for reading files)
#[derive(Debug, Clone)]
pub struct ObjectStorageSource {
    pub config: ObjectStorageConfig,
}

impl ObjectStorageSource {
    async fn list_data_files(&self, client: &aws_sdk_s3::Client) -> crate::Result<Vec<String>> {
        let prefix = self.config.normalized_prefix();
        let resp = client
            .list_objects_v2()
            .bucket(&self.config.bucket)
            .prefix(&prefix)
            .send()
            .await
            .map_err(|e| crate::Error::ConnectorError(format!("S3 list_objects_v2 failed: {e}")))?;

        let ext = self.config.table_format.format.extension();
        Ok(resp
            .contents()
            .iter()
            .filter_map(|o| o.key().map(|k| k.to_string()))
            .filter(|k| k.ends_with(&format!(".{ext}")))
            .collect())
    }

    async fn read_object(
        &self,
        client: &aws_sdk_s3::Client,
        key: &str,
    ) -> crate::Result<Vec<Record>> {
        let obj = client
            .get_object()
            .bucket(&self.config.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| {
                crate::Error::ConnectorError(format!("S3 get_object failed for {key}: {e}"))
            })?;
        let bytes = obj
            .body
            .collect()
            .await
            .map_err(|e| {
                crate::Error::ConnectorError(format!("S3 object body read failed for {key}: {e}"))
            })?
            .into_bytes();
        deserialize_records(&bytes, self.config.table_format.format, key)
    }
}

#[async_trait]
impl super::SourceConnector for ObjectStorageSource {
    fn name(&self) -> &str {
        match self.config.provider.as_str() {
            "s3" => "Amazon S3",
            "gcs" => "Google Cloud Storage",
            "azure" => "Azure Blob Storage",
            _ => "Object Storage",
        }
    }

    fn description(&self) -> &str {
        "Read data from S3-compatible object storage (real; JSON/CSV formats)"
    }

    async fn test_connection(&self) -> crate::Result<ConnectionTest> {
        let client = match build_s3_client(&self.config).await {
            Ok(c) => c,
            Err(e) => {
                return Ok(ConnectionTest::failure(
                    "Object storage connection failed",
                    Some(e.to_string()),
                ))
            }
        };
        match client
            .head_bucket()
            .bucket(&self.config.bucket)
            .send()
            .await
        {
            Ok(_) => Ok(ConnectionTest::success(&format!(
                "Connected to {} bucket: {}",
                self.config.provider, self.config.bucket
            ))),
            Err(e) => Ok(ConnectionTest::failure(
                "Bucket not reachable",
                Some(e.to_string()),
            )),
        }
    }

    async fn detect_schema(&self) -> crate::Result<super::Schema> {
        let client = build_s3_client(&self.config).await?;
        let files = self.list_data_files(&client).await?;
        let Some(first) = files.first() else {
            return Ok(super::Schema {
                fields: vec![],
                sample_records: vec![],
            });
        };
        let records = self.read_object(&client, first).await?;
        let fields = records
            .first()
            .and_then(|r| r.data.as_object())
            .map(|obj| {
                obj.keys()
                    .map(|k| super::Field {
                        name: k.clone(),
                        field_type: "string".to_string(),
                        required: false,
                        description: None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(super::Schema {
            fields,
            sample_records: records.into_iter().take(5).collect(),
        })
    }

    async fn read_all(&self) -> crate::Result<Vec<Record>> {
        let client = build_s3_client(&self.config).await?;
        let files = self.list_data_files(&client).await?;
        let mut all = Vec::new();
        for key in files {
            all.extend(self.read_object(&client, &key).await?);
        }
        Ok(all)
    }

    async fn read_batch(&self, offset: u64, limit: u64) -> crate::Result<Vec<Record>> {
        let all = self.read_all().await?;
        Ok(all
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect())
    }

    async fn read_incremental(&self, _last_value: &str) -> crate::Result<Vec<Record>> {
        Err(crate::Error::ConfigError(
            "incremental reads are not yet implemented for object storage (partition-aware incremental listing requires a defined partition scheme)".to_string(),
        ))
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::Read,
            Capability::SchemaDetection,
            Capability::Batch,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectors::{DestinationConnector, SourceConnector};

    #[test]
    fn test_table_formats() {
        let csv = TableFormat::csv();
        assert_eq!(csv.format, FileFormat::CSV);

        let json = TableFormat::json();
        assert_eq!(json.format, FileFormat::JSON);

        let delta = TableFormat::delta();
        assert_eq!(delta.format, FileFormat::Delta);
        assert!(!delta.partition_columns.is_empty());
        assert!(!delta.format.is_implemented());
    }

    #[test]
    fn test_object_storage_config() {
        let config = ObjectStorageConfig::s3("my-bucket", "data/customers");
        assert_eq!(config.provider, "s3");
        assert_eq!(config.bucket, "my-bucket");
        assert_eq!(config.path, "data/customers");

        let gcs_config = ObjectStorageConfig::gcs("gs-bucket", "lake/");
        assert_eq!(gcs_config.provider, "gcs");
    }

    #[test]
    fn test_minio_config() {
        let config = ObjectStorageConfig::s3("bucket", "path").with_minio(
            "http://localhost:9000",
            "minioadmin",
            "minioadmin",
        );
        assert_eq!(config.endpoint.as_deref(), Some("http://localhost:9000"));
        assert!(config.force_path_style);
    }

    #[test]
    fn json_serialize_then_deserialize_round_trips() {
        let records = vec![
            Record {
                id: "1".to_string(),
                data: serde_json::json!({"id": 1, "name": "Alice"}),
                metadata: crate::connectors::RecordMetadata {
                    source: "test".to_string(),
                    source_timestamp: None,
                    received_at: chrono::Utc::now().to_rfc3339(),
                    operation: crate::connectors::RecordOperation::Insert,
                },
            },
            Record {
                id: "2".to_string(),
                data: serde_json::json!({"id": 2, "name": "Bob"}),
                metadata: crate::connectors::RecordMetadata {
                    source: "test".to_string(),
                    source_timestamp: None,
                    received_at: chrono::Utc::now().to_rfc3339(),
                    operation: crate::connectors::RecordOperation::Insert,
                },
            },
        ];

        let bytes = serialize_records(&records, FileFormat::JSON).unwrap();
        let parsed = deserialize_records(&bytes, FileFormat::JSON, "test.jsonl").unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].data["name"], serde_json::json!("Alice"));
        assert_eq!(parsed[1].data["name"], serde_json::json!("Bob"));
    }

    #[test]
    fn csv_serialize_then_deserialize_round_trips() {
        let records = vec![Record {
            id: "1".to_string(),
            data: serde_json::json!({"id": "1", "name": "Alice, Inc."}),
            metadata: crate::connectors::RecordMetadata {
                source: "test".to_string(),
                source_timestamp: None,
                received_at: chrono::Utc::now().to_rfc3339(),
                operation: crate::connectors::RecordOperation::Insert,
            },
        }];

        let bytes = serialize_records(&records, FileFormat::CSV).unwrap();
        let text = String::from_utf8(bytes.clone()).unwrap();
        assert!(
            text.contains("\"Alice, Inc.\""),
            "commas in values must be quoted: {text}"
        );

        let parsed = deserialize_records(&bytes, FileFormat::CSV, "test.csv").unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].data["name"], serde_json::json!("Alice, Inc."));
    }

    #[test]
    fn parquet_format_is_honestly_rejected_not_faked() {
        let records = vec![Record {
            id: "1".to_string(),
            data: serde_json::json!({"id": 1}),
            metadata: crate::connectors::RecordMetadata {
                source: "test".to_string(),
                source_timestamp: None,
                received_at: chrono::Utc::now().to_rfc3339(),
                operation: crate::connectors::RecordOperation::Insert,
            },
        }];
        let result = serialize_records(&records, FileFormat::Parquet);
        assert!(
            result.is_err(),
            "must not silently succeed for unimplemented formats"
        );
    }

    #[tokio::test]
    async fn gcs_provider_is_honestly_rejected_not_faked() {
        let config = ObjectStorageConfig::gcs("bucket", "path");
        let dest = ObjectStorageDestination { config };
        let result = DestinationConnector::write_batch(
            &dest,
            &[Record {
                id: "1".to_string(),
                data: serde_json::json!({"id": 1}),
                metadata: crate::connectors::RecordMetadata {
                    source: "test".to_string(),
                    source_timestamp: None,
                    received_at: chrono::Utc::now().to_rfc3339(),
                    operation: crate::connectors::RecordOperation::Insert,
                },
            }],
        )
        .await;
        assert!(
            result.is_err(),
            "unimplemented providers must error, not fabricate success"
        );
    }

    // -- Real, Docker-backed round-trip tests (MinIO) ----------------------
    // Requires a live MinIO instance; gated behind `#[ignore]`. Run explicitly:
    //
    //   docker run --rm -d -p 9000:9000 -e MINIO_ROOT_USER=minioadmin \
    //       -e MINIO_ROOT_PASSWORD=minioadmin --name pyreverseetl-minio-test \
    //       minio/minio server /data
    //   # create the bucket once: mc alias set local http://localhost:9000 minioadmin minioadmin
    //   #                         mc mb local/pyreverseetl-test
    //   PYREVERSEETL_TEST_MINIO_ENDPOINT=http://localhost:9000 cargo test \
    //       -p pyreverseetl-core --lib connectors::object_storage -- --ignored

    fn minio_test_config(path: &str) -> ObjectStorageConfig {
        let endpoint = std::env::var("PYREVERSEETL_TEST_MINIO_ENDPOINT")
            .unwrap_or_else(|_| "http://localhost:9000".to_string());
        ObjectStorageConfig::s3("pyreverseetl-test", path)
            .with_minio(&endpoint, "minioadmin", "minioadmin")
            .with_format(TableFormat::json())
    }

    #[tokio::test]
    #[ignore]
    async fn real_minio_write_then_read_round_trip() {
        // Unique path per run: writes never overwrite (see `overwrite: false`
        // default), so re-running this test against the same bucket/path would
        // otherwise accumulate files from previous runs and inflate the count.
        let config = minio_test_config(&format!("lineage-test/customers-{}", uuid::Uuid::new_v4()));
        let destination = ObjectStorageDestination {
            config: config.clone(),
        };
        let source = ObjectStorageSource { config };

        let records: Vec<Record> = (0..5)
            .map(|i| Record {
                id: i.to_string(),
                data: serde_json::json!({"id": i, "name": format!("Customer {i}")}),
                metadata: crate::connectors::RecordMetadata {
                    source: "test".to_string(),
                    source_timestamp: None,
                    received_at: chrono::Utc::now().to_rfc3339(),
                    operation: crate::connectors::RecordOperation::Insert,
                },
            })
            .collect();

        let written = DestinationConnector::write_batch(&destination, &records)
            .await
            .unwrap();
        assert_eq!(written, 5);

        let read_back = SourceConnector::read_all(&source).await.unwrap();
        assert_eq!(read_back.len(), 5);
        assert!(read_back
            .iter()
            .any(|r| r.data["name"] == serde_json::json!("Customer 3")));
    }

    #[tokio::test]
    #[ignore]
    async fn real_minio_test_connection() {
        let config = minio_test_config("connection-check");
        let source = ObjectStorageSource { config };
        let result = SourceConnector::test_connection(&source).await.unwrap();
        assert!(result.success, "{:?}", result.details);
    }
}
