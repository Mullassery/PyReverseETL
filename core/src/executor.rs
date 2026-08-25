//! The real sync engine.
//!
//! This is what was missing end-to-end: previously `python/pyreverseetl/cli.py`
//! never called into this crate at all -- `execute_activation()` was a
//! standalone in-memory dict simulator that fabricated `rows_synced = limit or
//! 1000` regardless of what (if anything) actually happened. `execute_sync`
//! below is the real thing the Python bindings now call (see
//! `python/src/lib.rs::run_sync`): it reads real records from a real source
//! connector, applies the real compliance engine (PII masking / violation
//! detection), writes them through a real destination connector or adapter,
//! and records a real lineage edge with the actual record count and
//! wall-clock timestamps.

use crate::adapters::{AdapterFactory, AuthMethod as AdapterAuth};
use crate::connectors::{
    DestinationConnector, MySQLConfig, MySQLConnector, ObjectStorageConfig,
    ObjectStorageDestination, ObjectStorageSource, PostgreSQLConfig, PostgreSQLConnector,
    Record as ConnRecord, SourceConnector,
};
use crate::entity::{Entity, EntityType};
use crate::governance::{
    ComplianceEngine, ComplianceRule, DefaultComplianceEngine, DefaultSchemaEvolution,
    SchemaChange, SchemaChangeType, SchemaEvolution,
};
use crate::idempotency::{self, IdempotencyLedger};
use crate::lineage::{LineageNode, LineageNodeKind, LineageStore};
use chrono::Utc;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Where `execute_sync` reads real records from.
#[derive(Debug, Clone)]
pub enum SourceSpec {
    Postgres(PostgreSQLConfig),
    MySQL(MySQLConfig),
    S3(ObjectStorageConfig),
}

/// Where `execute_sync` writes real records to.
#[derive(Debug, Clone)]
pub enum DestinationSpec {
    Postgres(PostgreSQLConfig),
    MySQL(MySQLConfig),
    S3(ObjectStorageConfig),
    Webhook {
        config: HashMap<String, serde_json::Value>,
        auth: AdapterAuth,
    },
    Salesforce {
        config: HashMap<String, serde_json::Value>,
        auth: AdapterAuth,
    },
    HubSpot {
        config: HashMap<String, serde_json::Value>,
        auth: AdapterAuth,
    },
    Marketo {
        config: HashMap<String, serde_json::Value>,
        auth: AdapterAuth,
    },
}

#[derive(Debug, Clone, Default)]
pub struct ExecuteOptions {
    /// Cap on records read from the source. `None` reads everything.
    pub limit: Option<u64>,
    /// Compliance rules (PII masking / removal / truncation / encryption-flagging)
    /// applied to every record before it's written to the destination.
    pub compliance_rules: Vec<ComplianceRule>,
    /// When true, records are read from the real source and run through the
    /// real compliance engine exactly as normal, but `write_to_destination`
    /// is never called -- no HTTP request, database write, or object-storage
    /// put reaches the destination. `ExecutionResult::dry_run_preview` holds
    /// the exact post-compliance payloads that *would* have been sent, so a
    /// caller can audit them before committing to a real run.
    pub dry_run: bool,
    /// When set, enables real schema-drift detection: the last-seen
    /// field-name/type shape for this source->destination pair is persisted
    /// to a SQLite file at this path and diffed against every run's records.
    /// `None` disables schema-drift checking (matches prior behavior, and
    /// avoids creating a file a caller didn't ask for).
    pub schema_store_path: Option<PathBuf>,
    /// When set, enables real record-level idempotency: a SQLite ledger at
    /// this path records which exact record content was already
    /// successfully sent to which destination, so a re-run (e.g. after a
    /// mid-batch crash) skips records that are already synced instead of
    /// re-sending them, and still sends a record whose content genuinely
    /// changed even if its id was seen before. Only applies to the
    /// adapter-based destinations (webhook/Salesforce/HubSpot/Marketo) --
    /// `None` disables idempotency checking (matches prior behavior).
    pub idempotency_store_path: Option<PathBuf>,
}

/// The real result of a sync: actual counts, actual timestamps -- never
/// fabricated. `compliance_violations` lists any rule violations found *after*
/// `apply_rules` ran (e.g. an `Encrypt` rule, which this engine doesn't
/// implement, so it is honestly reported as unresolved rather than silently
/// treated as compliant).
#[derive(Debug, Clone, Serialize)]
pub struct ExecutionResult {
    pub run_id: String,
    pub rows_read: u64,
    pub rows_written: u64,
    pub rows_failed: u64,
    /// Set (from `ExecuteOptions::dry_run`) whenever this run skipped the
    /// real destination write. `rows_written`/`rows_failed` are always 0 for
    /// a dry run -- nothing was actually attempted -- so this flag is how a
    /// caller tells "0 written because dry run" apart from "0 written
    /// because everything failed".
    pub dry_run: bool,
    /// Only populated when `dry_run` is true: the exact post-compliance
    /// payload for every record that *would* have been sent to the
    /// destination, as compact JSON. Empty otherwise.
    pub dry_run_preview: Vec<String>,
    /// Records skipped because a real `IdempotencyLedger` (see
    /// `ExecuteOptions::idempotency_store_path`) already had this exact
    /// content marked as synced to this destination. Always 0 when
    /// idempotency checking is disabled.
    pub rows_skipped_idempotent: u64,
    /// Only populated when `ExecuteOptions::schema_store_path` is set: a
    /// human-readable description of every field-name/type change detected
    /// against the last-known shape for this source->destination pair.
    /// Empty when no store is configured, or when the shape hasn't changed.
    pub schema_changes: Vec<String>,
    pub compliance_violations: Vec<String>,
    pub started_at: chrono::DateTime<Utc>,
    pub completed_at: chrono::DateTime<Utc>,
    pub duration_ms: i64,
}

/// Run one real sync: source read -> compliance -> destination write -> lineage.
pub async fn execute_sync(
    source: SourceSpec,
    destination: DestinationSpec,
    options: ExecuteOptions,
    lineage: &LineageStore,
) -> crate::Result<ExecutionResult> {
    let (records, source_node) = read_from_source(&source, options.limit).await?;
    execute_with_records(records, source_node, destination, options, lineage).await
}

async fn execute_with_records(
    records: Vec<ConnRecord>,
    source_node: LineageNode,
    destination: DestinationSpec,
    options: ExecuteOptions,
    lineage: &LineageStore,
) -> crate::Result<ExecutionResult> {
    let run_id = uuid::Uuid::new_v4().to_string();
    let started_at = Utc::now();
    let rows_read = records.len() as u64;

    let compliance_engine = DefaultComplianceEngine::new(options.compliance_rules.clone());
    let mut compliant_records = Vec::with_capacity(records.len());
    let mut compliance_violations = Vec::new();
    for record in &records {
        let mut entity = record_to_entity(record);
        compliance_engine.apply_rules(&mut entity).await?;
        let check = compliance_engine.check_compliance(&entity).await?;
        if !check.compliant {
            compliance_violations.extend(check.violations);
        }
        compliant_records.push(entity_to_record(record, &entity));
    }

    // Real schema-drift detection: compares the shape actually being sent
    // this run (post-compliance, so masked/removed fields don't produce
    // spurious "Removed" noise) against the last-known shape for this
    // source->destination pair. Runs before the write (and even in dry-run)
    // so drift surfaces as an explicit finding, not just per-record HTTP
    // errors after the fact.
    let schema_changes = if let Some(store_path) = &options.schema_store_path {
        let schema_key = format!(
            "{}->{}",
            source_node.id,
            destination_schema_key(&destination)
        );
        let evolution =
            DefaultSchemaEvolution::with_sqlite_store("auto".to_string(), schema_key, store_path)?;
        let shape_entity = batch_shape_entity(&compliant_records);
        evolution
            .detect_changes(&shape_entity)
            .await?
            .iter()
            .map(describe_schema_change)
            .collect()
    } else {
        Vec::new()
    };

    let (rows_written, rows_failed, rows_skipped_idempotent, dry_run_preview, destination_node) =
        if options.dry_run {
            let preview = compliant_records
                .iter()
                .map(|r| r.data.to_string())
                .collect();
            (0, 0, 0, preview, None)
        } else {
            let (written, failed, skipped, node) = write_to_destination(
                &destination,
                &compliant_records,
                options.idempotency_store_path.as_deref(),
            )
            .await?;
            (written, failed, skipped, Vec::new(), Some(node))
        };

    let completed_at = Utc::now();
    if let Some(destination_node) = destination_node {
        lineage.record_sync(
            &run_id,
            source_node,
            destination_node,
            rows_written,
            started_at,
            completed_at,
        );
    }

    Ok(ExecutionResult {
        run_id,
        rows_read,
        rows_written,
        rows_failed,
        dry_run: options.dry_run,
        dry_run_preview,
        rows_skipped_idempotent,
        schema_changes,
        compliance_violations,
        started_at,
        completed_at,
        duration_ms: (completed_at - started_at).num_milliseconds(),
    })
}

/// A stable identifier for a destination target, used to scope the
/// persisted schema shape so two different destinations tracked against the
/// same store file never collide. Built from config only (no live call),
/// mirroring the id-construction already done per-arm in `write_to_destination`.
fn destination_schema_key(destination: &DestinationSpec) -> String {
    match destination {
        DestinationSpec::Postgres(cfg) => {
            format!("postgres:{}:{}/{}", cfg.host, cfg.database, cfg.table)
        }
        DestinationSpec::MySQL(cfg) => format!("mysql:{}:{}/{}", cfg.host, cfg.database, cfg.table),
        DestinationSpec::S3(cfg) => format!("s3:{}/{}", cfg.bucket, cfg.path),
        DestinationSpec::Webhook { config, .. } => format!(
            "webhook:{}",
            config
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
        ),
        DestinationSpec::Salesforce { config, .. } => format!(
            "salesforce:{}:{}",
            config
                .get("instance_url")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown"),
            config
                .get("object")
                .and_then(|v| v.as_str())
                .unwrap_or("Contact")
        ),
        DestinationSpec::HubSpot { config, .. } => format!(
            "hubspot:{}",
            config
                .get("object")
                .and_then(|v| v.as_str())
                .unwrap_or("contacts")
        ),
        DestinationSpec::Marketo { config, .. } => format!(
            "marketo:{}",
            config
                .get("api_host")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
        ),
    }
}

/// Merges every record's fields into one synthetic entity (each field takes
/// the value from the first record that defines it) so schema-drift
/// detection sees the shape of the whole batch, not just one record --
/// otherwise an optional field absent from record 1 but present on record 2
/// would spuriously look like a removal.
fn batch_shape_entity(records: &[ConnRecord]) -> Entity {
    let mut merged = serde_json::Map::new();
    for record in records {
        if let Some(obj) = record.data.as_object() {
            for (key, value) in obj {
                merged.entry(key.clone()).or_insert_with(|| value.clone());
            }
        }
    }
    let mut entity = Entity::new(EntityType::Custom("batch_shape".to_string()), "id", "batch");
    entity.attributes = serde_json::Value::Object(merged);
    entity
}

fn describe_schema_change(change: &SchemaChange) -> String {
    match change.change_type {
        SchemaChangeType::Added => format!(
            "Field '{}' added (type: {})",
            change.field_name,
            change.new_type.as_deref().unwrap_or("unknown")
        ),
        SchemaChangeType::Removed => format!(
            "Field '{}' removed (was: {})",
            change.field_name,
            change.old_type.as_deref().unwrap_or("unknown")
        ),
        SchemaChangeType::TypeChanged => format!(
            "Field '{}' changed type: {} -> {}",
            change.field_name,
            change.old_type.as_deref().unwrap_or("unknown"),
            change.new_type.as_deref().unwrap_or("unknown")
        ),
        SchemaChangeType::Renamed => format!("Field '{}' renamed", change.field_name),
    }
}

fn record_to_entity(record: &ConnRecord) -> Entity {
    let mut entity = Entity::new(
        EntityType::Custom("record".to_string()),
        "id",
        record.id.clone(),
    );
    entity.attributes = record.data.clone();
    entity
}

fn entity_to_record(original: &ConnRecord, entity: &Entity) -> ConnRecord {
    let mut rec = original.clone();
    rec.data = entity.attributes.clone();
    rec
}

async fn read_from_source(
    source: &SourceSpec,
    limit: Option<u64>,
) -> crate::Result<(Vec<ConnRecord>, LineageNode)> {
    match source {
        SourceSpec::Postgres(cfg) => {
            let connector = PostgreSQLConnector::new(cfg.clone());
            let records = match limit {
                Some(l) => SourceConnector::read_batch(&connector, 0, l).await?,
                None => SourceConnector::read_all(&connector).await?,
            };
            let node = LineageNode::new(
                format!("postgres:{}:{}/{}", cfg.host, cfg.database, cfg.table),
                LineageNodeKind::Source,
                "postgres",
                format!("{}.{}", cfg.database, cfg.table),
            );
            Ok((records, node))
        }
        SourceSpec::MySQL(cfg) => {
            let connector = MySQLConnector::new(cfg.clone());
            let records = match limit {
                Some(l) => SourceConnector::read_batch(&connector, 0, l).await?,
                None => SourceConnector::read_all(&connector).await?,
            };
            let node = LineageNode::new(
                format!("mysql:{}:{}/{}", cfg.host, cfg.database, cfg.table),
                LineageNodeKind::Source,
                "mysql",
                format!("{}.{}", cfg.database, cfg.table),
            );
            Ok((records, node))
        }
        SourceSpec::S3(cfg) => {
            let connector = ObjectStorageSource {
                config: cfg.clone(),
            };
            let records = match limit {
                Some(l) => SourceConnector::read_batch(&connector, 0, l).await?,
                None => SourceConnector::read_all(&connector).await?,
            };
            let node = LineageNode::new(
                format!("s3:{}/{}", cfg.bucket, cfg.path),
                LineageNodeKind::Source,
                "s3",
                format!("s3://{}/{}", cfg.bucket, cfg.path),
            );
            Ok((records, node))
        }
    }
}

async fn write_to_destination(
    destination: &DestinationSpec,
    records: &[ConnRecord],
    idempotency_store_path: Option<&Path>,
) -> crate::Result<(u64, u64, u64, LineageNode)> {
    match destination {
        DestinationSpec::Postgres(cfg) => {
            let connector = PostgreSQLConnector::new(cfg.clone());
            let written = DestinationConnector::write_batch(&connector, records).await? as u64;
            let node = LineageNode::new(
                format!("postgres:{}:{}/{}", cfg.host, cfg.database, cfg.table),
                LineageNodeKind::Destination,
                "postgres",
                format!("{}.{}", cfg.database, cfg.table),
            );
            Ok((
                written,
                (records.len() as u64).saturating_sub(written),
                0,
                node,
            ))
        }
        DestinationSpec::MySQL(cfg) => {
            let connector = MySQLConnector::new(cfg.clone());
            let written = DestinationConnector::write_batch(&connector, records).await? as u64;
            let node = LineageNode::new(
                format!("mysql:{}:{}/{}", cfg.host, cfg.database, cfg.table),
                LineageNodeKind::Destination,
                "mysql",
                format!("{}.{}", cfg.database, cfg.table),
            );
            Ok((
                written,
                (records.len() as u64).saturating_sub(written),
                0,
                node,
            ))
        }
        DestinationSpec::S3(cfg) => {
            let connector = ObjectStorageDestination {
                config: cfg.clone(),
            };
            let written = DestinationConnector::write_batch(&connector, records).await? as u64;
            let node = LineageNode::new(
                format!("s3:{}/{}", cfg.bucket, cfg.path),
                LineageNodeKind::Destination,
                "s3",
                format!("s3://{}/{}", cfg.bucket, cfg.path),
            );
            Ok((
                written,
                (records.len() as u64).saturating_sub(written),
                0,
                node,
            ))
        }
        DestinationSpec::Webhook { config, auth } => {
            let (written, failed, skipped) = write_via_adapter(
                "webhook",
                config.clone(),
                auth.clone(),
                records.to_vec(),
                idempotency_store_path.map(Path::to_path_buf),
                destination_schema_key(destination),
            )
            .await?;
            let node = LineageNode::new(
                format!(
                    "webhook:{}",
                    config
                        .get("url")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                ),
                LineageNodeKind::Destination,
                "webhook",
                config
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("webhook")
                    .to_string(),
            );
            Ok((written, failed, skipped, node))
        }
        DestinationSpec::Salesforce { config, auth } => {
            let (written, failed, skipped) = write_via_adapter(
                "salesforce",
                config.clone(),
                auth.clone(),
                records.to_vec(),
                idempotency_store_path.map(Path::to_path_buf),
                destination_schema_key(destination),
            )
            .await?;
            let object = config
                .get("object")
                .and_then(|v| v.as_str())
                .unwrap_or("Contact");
            let node = LineageNode::new(
                format!(
                    "salesforce:{}:{}",
                    config
                        .get("instance_url")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown"),
                    object
                ),
                LineageNodeKind::Destination,
                "salesforce",
                object.to_string(),
            );
            Ok((written, failed, skipped, node))
        }
        DestinationSpec::HubSpot { config, auth } => {
            let (written, failed, skipped) = write_via_adapter(
                "hubspot",
                config.clone(),
                auth.clone(),
                records.to_vec(),
                idempotency_store_path.map(Path::to_path_buf),
                destination_schema_key(destination),
            )
            .await?;
            let object = config
                .get("object")
                .and_then(|v| v.as_str())
                .unwrap_or("contacts");
            let node = LineageNode::new(
                format!("hubspot:{object}"),
                LineageNodeKind::Destination,
                "hubspot",
                object.to_string(),
            );
            Ok((written, failed, skipped, node))
        }
        DestinationSpec::Marketo { config, auth } => {
            let (written, failed, skipped) = write_via_adapter(
                "marketo",
                config.clone(),
                auth.clone(),
                records.to_vec(),
                idempotency_store_path.map(Path::to_path_buf),
                destination_schema_key(destination),
            )
            .await?;
            let node = LineageNode::new(
                format!(
                    "marketo:{}",
                    config
                        .get("api_host")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                ),
                LineageNodeKind::Destination,
                "marketo",
                "leads".to_string(),
            );
            Ok((written, failed, skipped, node))
        }
    }
}

/// Bridge into the synchronous `DestinationAdapter` trait (webhook/Salesforce/
/// HubSpot/Marketo use `reqwest::blocking`) from this async executor via
/// `spawn_blocking`, so a blocking HTTP call never stalls the Tokio runtime's
/// worker threads. When `idempotency_store_path` is set, each record is
/// checked against (and, on success, recorded into) a real
/// `IdempotencyLedger` before being sent -- a record whose exact content was
/// already successfully synced to `destination_key` is skipped rather than
/// re-sent, which is what makes a re-run after a mid-batch crash safe (and
/// gives every adapter, including the webhook adapter which has no upsert
/// semantics of its own, real duplicate protection).
async fn write_via_adapter(
    adapter_type: &'static str,
    config: HashMap<String, serde_json::Value>,
    auth: AdapterAuth,
    records: Vec<ConnRecord>,
    idempotency_store_path: Option<PathBuf>,
    destination_key: String,
) -> crate::Result<(u64, u64, u64)> {
    tokio::task::spawn_blocking(move || -> crate::Result<(u64, u64, u64)> {
        let adapter = AdapterFactory::create_adapter(adapter_type, &config, &auth)?;
        let ledger = idempotency_store_path
            .as_deref()
            .map(IdempotencyLedger::open)
            .transpose()?;

        let mut written = 0u64;
        let mut failed = 0u64;
        let mut skipped = 0u64;
        for record in &records {
            if let Some(ledger) = &ledger {
                let hash = idempotency::content_hash(&record.data);
                if ledger.already_synced(&destination_key, &record.id, &hash)? {
                    skipped += 1;
                    continue;
                }
                let entity = record_to_entity(record);
                match adapter.upsert(&entity, &[]) {
                    Ok(result) if result.success => {
                        ledger.mark_synced(&destination_key, &record.id, &hash)?;
                        written += 1;
                    }
                    _ => failed += 1,
                }
            } else {
                let entity = record_to_entity(record);
                match adapter.upsert(&entity, &[]) {
                    Ok(result) if result.success => written += 1,
                    _ => failed += 1,
                }
            }
        }
        Ok((written, failed, skipped))
    })
    .await
    .map_err(|e| crate::Error::Internal(format!("destination adapter task panicked: {e}")))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governance::{RuleAction, RuleType};
    use crate::testing::MockHttpServer;
    use serde_json::json;

    fn source_node() -> LineageNode {
        LineageNode::new(
            "test:source",
            LineageNodeKind::Source,
            "test",
            "test source",
        )
    }

    fn sample_records() -> Vec<ConnRecord> {
        vec![
            ConnRecord {
                id: "1".to_string(),
                data: json!({"id": 1, "email": "alice@example.com", "name": "Alice"}),
                metadata: crate::connectors::RecordMetadata {
                    source: "test".to_string(),
                    source_timestamp: None,
                    received_at: chrono::Utc::now().to_rfc3339(),
                    operation: crate::connectors::RecordOperation::Insert,
                },
            },
            ConnRecord {
                id: "2".to_string(),
                data: json!({"id": 2, "email": "bob@example.com", "name": "Bob"}),
                metadata: crate::connectors::RecordMetadata {
                    source: "test".to_string(),
                    source_timestamp: None,
                    received_at: chrono::Utc::now().to_rfc3339(),
                    operation: crate::connectors::RecordOperation::Insert,
                },
            },
        ]
    }

    #[tokio::test]
    async fn execute_with_records_writes_real_http_requests_and_records_real_lineage() {
        let server = MockHttpServer::start(200, "{}");
        let mut config = HashMap::new();
        config.insert("url".to_string(), json!(server.base_url.clone()));

        let destination = DestinationSpec::Webhook {
            config,
            auth: AdapterAuth::Bearer {
                token: "tok".to_string(),
            },
        };
        let lineage = LineageStore::new();

        let result = execute_with_records(
            sample_records(),
            source_node(),
            destination,
            ExecuteOptions::default(),
            &lineage,
        )
        .await
        .unwrap();

        assert_eq!(result.rows_read, 2);
        assert_eq!(result.rows_written, 2);
        assert_eq!(result.rows_failed, 0);
        assert_eq!(server.requests().len(), 2, "one real HTTP call per record");

        // Real lineage: one edge for this run, with the real record count.
        let edges = lineage.edges_for_run(&result.run_id);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].record_count, 2);
        assert!(edges[0].completed_at >= edges[0].started_at);
    }

    #[tokio::test]
    async fn execute_with_records_masks_pii_before_it_ever_leaves_the_process() {
        let server = MockHttpServer::start(200, "{}");
        let mut config = HashMap::new();
        config.insert("url".to_string(), json!(server.base_url.clone()));

        let destination = DestinationSpec::Webhook {
            config,
            auth: AdapterAuth::Bearer {
                token: "tok".to_string(),
            },
        };
        let lineage = LineageStore::new();

        let masking_rule = ComplianceRule::new(
            "email_masking".to_string(),
            RuleType::PiiMasking,
            vec!["email".to_string()],
            RuleAction::Mask("****".to_string()),
        );

        execute_with_records(
            sample_records(),
            source_node(),
            destination,
            ExecuteOptions {
                limit: None,
                compliance_rules: vec![masking_rule],
                dry_run: false,
                schema_store_path: None,
                idempotency_store_path: None,
            },
            &lineage,
        )
        .await
        .unwrap();

        for req in server.requests() {
            let body: serde_json::Value = serde_json::from_str(&req.body).unwrap();
            assert_eq!(
                body["email"],
                json!("****"),
                "raw PII must never reach the destination"
            );
        }
    }

    #[tokio::test]
    async fn execute_with_records_honestly_reports_unimplemented_encryption_as_a_violation() {
        let server = MockHttpServer::start(200, "{}");
        let mut config = HashMap::new();
        config.insert("url".to_string(), json!(server.base_url.clone()));
        let destination = DestinationSpec::Webhook {
            config,
            auth: AdapterAuth::Bearer {
                token: "tok".to_string(),
            },
        };
        let lineage = LineageStore::new();

        let encrypt_rule = ComplianceRule::new(
            "email_encryption".to_string(),
            RuleType::Compliance,
            vec!["email".to_string()],
            RuleAction::Encrypt,
        );

        let result = execute_with_records(
            sample_records(),
            source_node(),
            destination,
            ExecuteOptions {
                limit: None,
                compliance_rules: vec![encrypt_rule],
                dry_run: false,
                schema_store_path: None,
                idempotency_store_path: None,
            },
            &lineage,
        )
        .await
        .unwrap();

        assert!(
            !result.compliance_violations.is_empty(),
            "an Encrypt rule with no real encryption implementation must be reported, not silently passed"
        );
    }

    #[tokio::test]
    async fn dry_run_never_calls_the_destination_and_previews_the_real_payload() {
        let server = MockHttpServer::start(200, "{}");
        let mut config = HashMap::new();
        config.insert("url".to_string(), json!(server.base_url.clone()));

        let destination = DestinationSpec::Webhook {
            config,
            auth: AdapterAuth::Bearer {
                token: "tok".to_string(),
            },
        };
        let lineage = LineageStore::new();

        let result = execute_with_records(
            sample_records(),
            source_node(),
            destination,
            ExecuteOptions {
                limit: None,
                compliance_rules: vec![],
                dry_run: true,
                schema_store_path: None,
                idempotency_store_path: None,
            },
            &lineage,
        )
        .await
        .unwrap();

        assert!(result.dry_run);
        assert_eq!(result.rows_read, 2);
        assert_eq!(
            result.rows_written, 0,
            "dry run must never report a real write"
        );
        assert_eq!(result.rows_failed, 0);
        assert_eq!(
            server.requests().len(),
            0,
            "dry run must make zero real requests to the destination"
        );

        assert_eq!(result.dry_run_preview.len(), 2);
        let first: serde_json::Value = serde_json::from_str(&result.dry_run_preview[0]).unwrap();
        assert_eq!(first["email"], json!("alice@example.com"));

        // No real sync happened, so no lineage edge should be fabricated for it.
        assert!(lineage.edges_for_run(&result.run_id).is_empty());
    }

    #[tokio::test]
    async fn dry_run_preview_reflects_compliance_masking() {
        let server = MockHttpServer::start(200, "{}");
        let mut config = HashMap::new();
        config.insert("url".to_string(), json!(server.base_url.clone()));
        let destination = DestinationSpec::Webhook {
            config,
            auth: AdapterAuth::Bearer {
                token: "tok".to_string(),
            },
        };
        let lineage = LineageStore::new();

        let masking_rule = ComplianceRule::new(
            "email_masking".to_string(),
            RuleType::PiiMasking,
            vec!["email".to_string()],
            RuleAction::Mask("****".to_string()),
        );

        let result = execute_with_records(
            sample_records(),
            source_node(),
            destination,
            ExecuteOptions {
                limit: None,
                compliance_rules: vec![masking_rule],
                dry_run: true,
                schema_store_path: None,
                idempotency_store_path: None,
            },
            &lineage,
        )
        .await
        .unwrap();

        assert_eq!(server.requests().len(), 0);
        for preview in &result.dry_run_preview {
            let body: serde_json::Value = serde_json::from_str(preview).unwrap();
            assert_eq!(
                body["email"],
                json!("****"),
                "the preview should show what would actually be sent, including masking"
            );
        }
    }

    #[tokio::test]
    async fn schema_drift_is_detected_and_persisted_across_separate_runs() {
        let dir = std::env::temp_dir().join(format!("pyreverseetl_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store_path = dir.join("schema.db");

        let server = MockHttpServer::start(200, "{}");
        let mut config = HashMap::new();
        config.insert("url".to_string(), json!(server.base_url.clone()));
        let destination = || DestinationSpec::Webhook {
            config: config.clone(),
            auth: AdapterAuth::Bearer {
                token: "tok".to_string(),
            },
        };
        let lineage = LineageStore::new();

        // First run: nothing recorded yet for this source->destination pair,
        // so this is a first observation -- no drift reported.
        let first = execute_with_records(
            sample_records(),
            source_node(),
            destination(),
            ExecuteOptions {
                limit: None,
                compliance_rules: vec![],
                dry_run: false,
                schema_store_path: Some(store_path.clone()),
                idempotency_store_path: None,
            },
            &lineage,
        )
        .await
        .unwrap();
        assert!(first.schema_changes.is_empty());

        // Second run: same shape as before -- still no drift.
        let second = execute_with_records(
            sample_records(),
            source_node(),
            destination(),
            ExecuteOptions {
                limit: None,
                compliance_rules: vec![],
                dry_run: false,
                schema_store_path: Some(store_path.clone()),
                idempotency_store_path: None,
            },
            &lineage,
        )
        .await
        .unwrap();
        assert!(second.schema_changes.is_empty());

        // Third run: a genuinely new field appears -- must be detected as
        // real drift, persisted in the SQLite file at store_path (a fresh
        // process pointed at the same file would see the same history).
        let mut records_with_new_field = sample_records();
        for record in &mut records_with_new_field {
            record.data["signup_source"] = json!("referral");
        }
        let third = execute_with_records(
            records_with_new_field,
            source_node(),
            destination(),
            ExecuteOptions {
                limit: None,
                compliance_rules: vec![],
                dry_run: false,
                schema_store_path: Some(store_path.clone()),
                idempotency_store_path: None,
            },
            &lineage,
        )
        .await
        .unwrap();

        assert_eq!(third.schema_changes.len(), 1);
        assert!(third.schema_changes[0].contains("signup_source"));
        assert!(third.schema_changes[0].contains("added"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn idempotent_rerun_skips_already_synced_records_and_makes_no_new_requests() {
        let dir = std::env::temp_dir().join(format!("pyreverseetl_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let ledger_path = dir.join("idempotency.db");

        let server = MockHttpServer::start(200, "{}");
        let mut config = HashMap::new();
        config.insert("url".to_string(), json!(server.base_url.clone()));
        let destination = || DestinationSpec::Webhook {
            config: config.clone(),
            auth: AdapterAuth::Bearer {
                token: "tok".to_string(),
            },
        };
        let lineage = LineageStore::new();

        let first = execute_with_records(
            sample_records(),
            source_node(),
            destination(),
            ExecuteOptions {
                limit: None,
                compliance_rules: vec![],
                dry_run: false,
                schema_store_path: None,
                idempotency_store_path: Some(ledger_path.clone()),
            },
            &lineage,
        )
        .await
        .unwrap();
        assert_eq!(first.rows_written, 2);
        assert_eq!(first.rows_skipped_idempotent, 0);
        assert_eq!(
            server.requests().len(),
            2,
            "first run sends both records for real"
        );

        // Simulate a re-run with the exact same records (e.g. after a crash
        // that killed the process before it could tell the caller it
        // finished, so the caller retries the whole batch).
        let second = execute_with_records(
            sample_records(),
            source_node(),
            destination(),
            ExecuteOptions {
                limit: None,
                compliance_rules: vec![],
                dry_run: false,
                schema_store_path: None,
                idempotency_store_path: Some(ledger_path.clone()),
            },
            &lineage,
        )
        .await
        .unwrap();

        assert_eq!(second.rows_written, 0);
        assert_eq!(
            second.rows_skipped_idempotent, 2,
            "both records were already synced with identical content"
        );
        assert_eq!(
            server.requests().len(),
            2,
            "the re-run must make zero new real HTTP requests"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn idempotency_still_resends_a_record_whose_content_genuinely_changed() {
        let dir = std::env::temp_dir().join(format!("pyreverseetl_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let ledger_path = dir.join("idempotency.db");

        let server = MockHttpServer::start(200, "{}");
        let mut config = HashMap::new();
        config.insert("url".to_string(), json!(server.base_url.clone()));
        let destination = || DestinationSpec::Webhook {
            config: config.clone(),
            auth: AdapterAuth::Bearer {
                token: "tok".to_string(),
            },
        };
        let lineage = LineageStore::new();

        execute_with_records(
            sample_records(),
            source_node(),
            destination(),
            ExecuteOptions {
                limit: None,
                compliance_rules: vec![],
                dry_run: false,
                schema_store_path: None,
                idempotency_store_path: Some(ledger_path.clone()),
            },
            &lineage,
        )
        .await
        .unwrap();
        assert_eq!(server.requests().len(), 2);

        // Same record ids, but record 1's content genuinely changed --
        // must be sent again, not skipped as a false-positive duplicate.
        let mut changed = sample_records();
        changed[0].data["email"] = json!("alice-new-email@example.com");

        let result = execute_with_records(
            changed,
            source_node(),
            destination(),
            ExecuteOptions {
                limit: None,
                compliance_rules: vec![],
                dry_run: false,
                schema_store_path: None,
                idempotency_store_path: Some(ledger_path.clone()),
            },
            &lineage,
        )
        .await
        .unwrap();

        assert_eq!(
            result.rows_written, 1,
            "the genuinely changed record must be sent"
        );
        assert_eq!(
            result.rows_skipped_idempotent, 1,
            "the unchanged record is still skipped"
        );
        assert_eq!(
            server.requests().len(),
            3,
            "one new real request for the changed record"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn idempotency_disabled_by_default_resends_every_record_every_run() {
        let server = MockHttpServer::start(200, "{}");
        let mut config = HashMap::new();
        config.insert("url".to_string(), json!(server.base_url.clone()));
        let destination = || DestinationSpec::Webhook {
            config: config.clone(),
            auth: AdapterAuth::Bearer {
                token: "tok".to_string(),
            },
        };
        let lineage = LineageStore::new();

        for _ in 0..2 {
            let result = execute_with_records(
                sample_records(),
                source_node(),
                destination(),
                ExecuteOptions::default(),
                &lineage,
            )
            .await
            .unwrap();
            assert_eq!(result.rows_written, 2);
            assert_eq!(result.rows_skipped_idempotent, 0);
        }

        assert_eq!(
            server.requests().len(),
            4,
            "with no idempotency store configured, every run resends every record (prior behavior)"
        );
    }
}
