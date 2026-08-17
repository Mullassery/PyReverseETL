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
use crate::governance::{ComplianceEngine, ComplianceRule, DefaultComplianceEngine};
use crate::lineage::{LineageNode, LineageNodeKind, LineageStore};
use chrono::Utc;
use serde::Serialize;
use std::collections::HashMap;

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

    let (rows_written, rows_failed, destination_node) =
        write_to_destination(&destination, &compliant_records).await?;

    let completed_at = Utc::now();
    lineage.record_sync(
        &run_id,
        source_node,
        destination_node,
        rows_written,
        started_at,
        completed_at,
    );

    Ok(ExecutionResult {
        run_id,
        rows_read,
        rows_written,
        rows_failed,
        compliance_violations,
        started_at,
        completed_at,
        duration_ms: (completed_at - started_at).num_milliseconds(),
    })
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
) -> crate::Result<(u64, u64, LineageNode)> {
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
                node,
            ))
        }
        DestinationSpec::Webhook { config, auth } => {
            let (written, failed) =
                write_via_adapter("webhook", config.clone(), auth.clone(), records.to_vec())
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
            Ok((written, failed, node))
        }
        DestinationSpec::Salesforce { config, auth } => {
            let (written, failed) =
                write_via_adapter("salesforce", config.clone(), auth.clone(), records.to_vec())
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
            Ok((written, failed, node))
        }
        DestinationSpec::HubSpot { config, auth } => {
            let (written, failed) =
                write_via_adapter("hubspot", config.clone(), auth.clone(), records.to_vec())
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
            Ok((written, failed, node))
        }
        DestinationSpec::Marketo { config, auth } => {
            let (written, failed) =
                write_via_adapter("marketo", config.clone(), auth.clone(), records.to_vec())
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
            Ok((written, failed, node))
        }
    }
}

/// Bridge into the synchronous `DestinationAdapter` trait (webhook/Salesforce/
/// HubSpot/Marketo use `reqwest::blocking`) from this async executor via
/// `spawn_blocking`, so a blocking HTTP call never stalls the Tokio runtime's
/// worker threads.
async fn write_via_adapter(
    adapter_type: &'static str,
    config: HashMap<String, serde_json::Value>,
    auth: AdapterAuth,
    records: Vec<ConnRecord>,
) -> crate::Result<(u64, u64)> {
    tokio::task::spawn_blocking(move || -> crate::Result<(u64, u64)> {
        let adapter = AdapterFactory::create_adapter(adapter_type, &config, &auth)?;
        let mut written = 0u64;
        let mut failed = 0u64;
        for record in &records {
            let entity = record_to_entity(record);
            match adapter.upsert(&entity, &[]) {
                Ok(result) if result.success => written += 1,
                _ => failed += 1,
            }
        }
        Ok((written, failed))
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
}
