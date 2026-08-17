use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyreverseetl_core::adapters::AuthMethod as AdapterAuth;
use pyreverseetl_core::connectors::{
    MySQLConfig, ObjectStorageConfig, PostgreSQLConfig, TableFormat,
};
use pyreverseetl_core::destination::DestinationType;
use pyreverseetl_core::entity::EntityType;
use pyreverseetl_core::governance::{ComplianceRule, RuleAction, RuleType};
use pyreverseetl_core::workflow::SourceType;
use pyreverseetl_core::{
    execute_sync, Activation, Destination, DestinationSpec, Entity, ExecuteOptions, LineageStore,
    SourceSpec, SyncRun, Workflow,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;

#[pyclass]
pub struct PyWorkflow {
    inner: Workflow,
}

#[pymethods]
impl PyWorkflow {
    #[staticmethod]
    #[pyo3(signature = (name, owner, source_type, table_name=None))]
    pub fn new(name: &str, owner: &str, source_type: &str, table_name: Option<&str>) -> Self {
        let source = match source_type {
            "table" => SourceType::Table {
                table_name: table_name.unwrap_or("default").to_string(),
            },
            _ => SourceType::Table {
                table_name: table_name.unwrap_or("default").to_string(),
            },
        };
        PyWorkflow {
            inner: Workflow::new(name, owner, source),
        }
    }

    #[getter]
    pub fn id(&self) -> String {
        self.inner.id.clone()
    }

    #[getter]
    pub fn name(&self) -> String {
        self.inner.name.clone()
    }

    pub fn with_description(&mut self, desc: &str) -> PyResult<()> {
        self.inner = self.inner.clone().with_description(desc);
        Ok(())
    }

    pub fn add_mapping(&mut self, source: &str, dest: &str) -> PyResult<()> {
        self.inner = self.inner.clone().add_mapping(source, dest);
        Ok(())
    }

    pub fn set_enabled(&mut self, enabled: bool) -> PyResult<()> {
        self.inner = self.inner.clone().set_enabled(enabled);
        Ok(())
    }
}

#[pyclass]
pub struct PyDestination {
    inner: Destination,
}

#[pymethods]
impl PyDestination {
    #[staticmethod]
    pub fn new(name: &str, dest_type: &str) -> Self {
        let dtype = match dest_type {
            "salesforce" => DestinationType::Salesforce,
            "hubspot" => DestinationType::HubSpot,
            "kafka" => DestinationType::Kafka,
            "webhook" => DestinationType::Webhook,
            _ => DestinationType::Webhook,
        };
        PyDestination {
            inner: Destination::new(name, dtype),
        }
    }

    #[getter]
    pub fn id(&self) -> String {
        self.inner.id.clone()
    }

    #[getter]
    pub fn name(&self) -> String {
        self.inner.name.clone()
    }

    pub fn set_enabled(&mut self, enabled: bool) -> PyResult<()> {
        self.inner = self.inner.clone().set_enabled(enabled);
        Ok(())
    }
}

#[pyclass]
pub struct PyActivation {
    inner: Activation,
}

#[pymethods]
impl PyActivation {
    #[staticmethod]
    pub fn new(name: &str, workflow_id: &str, owner: &str) -> Self {
        PyActivation {
            inner: Activation::new(name, workflow_id, owner),
        }
    }

    #[getter]
    pub fn id(&self) -> String {
        self.inner.id.clone()
    }

    #[getter]
    pub fn name(&self) -> String {
        self.inner.name.clone()
    }

    pub fn add_destination(&mut self, dest_id: &str) -> PyResult<()> {
        self.inner = self.inner.clone().add_destination(dest_id);
        Ok(())
    }

    pub fn set_enabled(&mut self, enabled: bool) -> PyResult<()> {
        self.inner = self.inner.clone().set_enabled(enabled);
        Ok(())
    }
}

#[pyclass]
pub struct PyEntity {
    inner: Entity,
}

#[pymethods]
impl PyEntity {
    #[staticmethod]
    pub fn new(entity_type: &str, key_field: &str, id: &str) -> Self {
        let etype = match entity_type {
            "customer" => EntityType::Customer,
            "account" => EntityType::Account,
            "company" => EntityType::Company,
            _ => EntityType::Customer,
        };
        PyEntity {
            inner: Entity::new(etype, key_field, id),
        }
    }

    #[getter]
    pub fn id(&self) -> String {
        self.inner.id.clone()
    }

    pub fn add_trait(&mut self, name: &str, value: &str) -> PyResult<()> {
        self.inner = self.inner.clone().add_trait(name, serde_json::json!(value));
        Ok(())
    }

    pub fn add_attribute(&mut self, key: &str, value: &str) -> PyResult<()> {
        self.inner = self
            .inner
            .clone()
            .add_attribute(key, serde_json::json!(value));
        Ok(())
    }
}

#[pyclass]
pub struct PySyncRun {
    inner: SyncRun,
}

#[pymethods]
impl PySyncRun {
    #[staticmethod]
    pub fn new(workflow_id: &str, activation_id: &str) -> Self {
        PySyncRun {
            inner: SyncRun::new(workflow_id, activation_id),
        }
    }

    #[getter]
    pub fn id(&self) -> String {
        self.inner.id.clone()
    }

    #[getter]
    pub fn status(&self) -> String {
        self.inner.status.to_string()
    }

    pub fn mark_running(&mut self) -> PyResult<()> {
        self.inner.mark_running();
        Ok(())
    }

    pub fn mark_success(&mut self) -> PyResult<()> {
        self.inner.mark_success();
        Ok(())
    }

    pub fn record_processed(&mut self, count: u64) -> PyResult<()> {
        self.inner.record_processed(count);
        Ok(())
    }
}

// -- Real sync engine bindings --------------------------------------------
//
// Everything below wires the Python layer to `pyreverseetl_core::execute_sync`
// -- the real engine that reads from a live source connector, applies real
// compliance rules, writes through a real destination connector/adapter, and
// records real lineage. `python/pyreverseetl/cli.py`'s `execute_activation()`
// used to be a standalone in-memory dict simulator that never reached this
// crate at all; `run_sync` is what it calls now.

fn py_err(e: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(e.to_string())
}

fn parse_json(s: &str) -> PyResult<Value> {
    serde_json::from_str(s).map_err(py_err)
}

fn get_str<'a>(v: &'a Value, key: &str) -> PyResult<&'a str> {
    v.get(key)
        .and_then(|x| x.as_str())
        .ok_or_else(|| py_err(format!("missing required string field '{key}'")))
}

fn get_str_opt<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(|x| x.as_str())
}

fn get_u16(v: &Value, key: &str, default: u16) -> u16 {
    v.get(key)
        .and_then(|x| x.as_u64())
        .map(|n| n as u16)
        .unwrap_or(default)
}

fn build_object_storage_config(config: &Value) -> PyResult<ObjectStorageConfig> {
    let bucket = get_str(config, "bucket")?;
    let path = get_str(config, "path")?;
    let mut cfg = ObjectStorageConfig::s3(bucket, path);
    if get_str_opt(config, "format") == Some("csv") {
        cfg = cfg.with_format(TableFormat::csv());
    }
    if let (Some(endpoint), Some(access_key), Some(secret_key)) = (
        get_str_opt(config, "endpoint"),
        get_str_opt(config, "access_key"),
        get_str_opt(config, "secret_key"),
    ) {
        cfg = cfg.with_minio(endpoint, access_key, secret_key);
    }
    if let Some(region) = get_str_opt(config, "region") {
        cfg.region = region.to_string();
    }
    Ok(cfg)
}

fn build_source_spec(source_type: &str, config: &Value) -> PyResult<SourceSpec> {
    match source_type {
        "postgres" => {
            let mut cfg = PostgreSQLConfig::new(
                get_str(config, "host")?,
                get_u16(config, "port", 5432),
                get_str(config, "database")?,
                get_str(config, "username")?,
                get_str(config, "password")?,
                get_str(config, "table")?,
            );
            if let Some(c) = get_str_opt(config, "incremental_column") {
                cfg = cfg.with_incremental_column(c);
            }
            Ok(SourceSpec::Postgres(cfg))
        }
        "mysql" => {
            let mut cfg = MySQLConfig::new(
                get_str(config, "host")?,
                get_u16(config, "port", 3306),
                get_str(config, "database")?,
                get_str(config, "username")?,
                get_str(config, "password")?,
                get_str(config, "table")?,
            );
            if let Some(c) = get_str_opt(config, "incremental_column") {
                cfg = cfg.with_incremental_column(c);
            }
            Ok(SourceSpec::MySQL(cfg))
        }
        "s3" => Ok(SourceSpec::S3(build_object_storage_config(config)?)),
        other => Err(py_err(format!(
            "unsupported source_type '{other}' (supported: postgres, mysql, s3)"
        ))),
    }
}

fn build_auth(config: &Value) -> PyResult<AdapterAuth> {
    let auth = config
        .get("auth")
        .ok_or_else(|| py_err("missing 'auth' object in destination config"))?;
    match get_str(auth, "type")? {
        "bearer" => Ok(AdapterAuth::Bearer {
            token: get_str(auth, "token")?.to_string(),
        }),
        "api_key" => Ok(AdapterAuth::ApiKey {
            key: get_str(auth, "key")?.to_string(),
        }),
        "basic" => Ok(AdapterAuth::Basic {
            username: get_str(auth, "username")?.to_string(),
            password: get_str(auth, "password")?.to_string(),
        }),
        "oauth" => Ok(AdapterAuth::OAuth {
            client_id: get_str(auth, "client_id")?.to_string(),
            client_secret: get_str(auth, "client_secret")?.to_string(),
            refresh_token: get_str_opt(auth, "refresh_token").map(|s| s.to_string()),
        }),
        other => Err(py_err(format!("unsupported auth type '{other}'"))),
    }
}

fn json_object_to_map(v: &Value) -> HashMap<String, Value> {
    v.as_object()
        .map(|o| o.clone().into_iter().collect())
        .unwrap_or_default()
}

fn build_destination_spec(destination_type: &str, config: &Value) -> PyResult<DestinationSpec> {
    match destination_type {
        "postgres" => {
            let mut cfg = PostgreSQLConfig::new(
                get_str(config, "host")?,
                get_u16(config, "port", 5432),
                get_str(config, "database")?,
                get_str(config, "username")?,
                get_str(config, "password")?,
                get_str(config, "table")?,
            );
            if let Some(k) = get_str_opt(config, "upsert_key") {
                cfg = cfg.with_upsert_key(k);
            }
            Ok(DestinationSpec::Postgres(cfg))
        }
        "mysql" => {
            let mut cfg = MySQLConfig::new(
                get_str(config, "host")?,
                get_u16(config, "port", 3306),
                get_str(config, "database")?,
                get_str(config, "username")?,
                get_str(config, "password")?,
                get_str(config, "table")?,
            );
            if let Some(k) = get_str_opt(config, "upsert_key") {
                cfg = cfg.with_upsert_key(k);
            }
            Ok(DestinationSpec::MySQL(cfg))
        }
        "s3" => Ok(DestinationSpec::S3(build_object_storage_config(config)?)),
        "webhook" => Ok(DestinationSpec::Webhook {
            config: json_object_to_map(config),
            auth: build_auth(config)?,
        }),
        "salesforce" => Ok(DestinationSpec::Salesforce {
            config: json_object_to_map(config),
            auth: build_auth(config)?,
        }),
        "hubspot" => Ok(DestinationSpec::HubSpot {
            config: json_object_to_map(config),
            auth: build_auth(config)?,
        }),
        "marketo" => Ok(DestinationSpec::Marketo {
            config: json_object_to_map(config),
            auth: build_auth(config)?,
        }),
        other => Err(py_err(format!(
            "unsupported destination_type '{other}' (supported: postgres, mysql, s3, webhook, salesforce, hubspot, marketo)"
        ))),
    }
}

fn build_compliance_rules(json: Option<&str>) -> PyResult<Vec<ComplianceRule>> {
    let Some(json) = json else {
        return Ok(Vec::new());
    };
    let value = parse_json(json)?;
    let arr = value
        .as_array()
        .ok_or_else(|| py_err("compliance_rules must be a JSON array"))?;
    arr.iter()
        .map(|rule| {
            let id = get_str(rule, "id")?.to_string();
            let rule_type = match get_str(rule, "rule_type")? {
                "pii_masking" => RuleType::PiiMasking,
                "retention" => RuleType::Retention,
                "compliance" => RuleType::Compliance,
                "custom" => RuleType::Custom,
                other => return Err(py_err(format!("unknown rule_type '{other}'"))),
            };
            let target_fields = rule
                .get("target_fields")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let action_obj = rule
                .get("action")
                .ok_or_else(|| py_err("compliance rule missing 'action'"))?;
            let action = match get_str(action_obj, "type")? {
                "mask" => RuleAction::Mask(
                    get_str_opt(action_obj, "pattern")
                        .unwrap_or("****")
                        .to_string(),
                ),
                "remove" => RuleAction::Remove,
                "truncate" => RuleAction::Truncate(
                    action_obj
                        .get("length")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(10) as usize,
                ),
                "encrypt" => RuleAction::Encrypt,
                other => return Err(py_err(format!("unknown action type '{other}'"))),
            };
            Ok(ComplianceRule::new(id, rule_type, target_fields, action))
        })
        .collect()
}

static LINEAGE: OnceLock<LineageStore> = OnceLock::new();

fn lineage_store() -> &'static LineageStore {
    LINEAGE.get_or_init(LineageStore::new)
}

/// The real result of one sync run: real counts, real timestamps.
#[pyclass]
pub struct PySyncResult {
    #[pyo3(get)]
    pub run_id: String,
    #[pyo3(get)]
    pub rows_read: u64,
    #[pyo3(get)]
    pub rows_written: u64,
    #[pyo3(get)]
    pub rows_failed: u64,
    #[pyo3(get)]
    pub compliance_violations: Vec<String>,
    #[pyo3(get)]
    pub started_at: String,
    #[pyo3(get)]
    pub completed_at: String,
    #[pyo3(get)]
    pub duration_ms: i64,
}

/// Run a real sync: read from `source_type`/`source_config` (JSON string),
/// apply `compliance_rules` (JSON string, optional), write to
/// `destination_type`/`destination_config` (JSON string), and record a real
/// lineage edge. Supported `source_type`: `postgres`, `mysql`, `s3`.
/// Supported `destination_type`: all of those plus `webhook`, `salesforce`,
/// `hubspot`, `marketo`.
#[pyfunction]
#[pyo3(signature = (source_type, source_config, destination_type, destination_config, limit=None, compliance_rules=None))]
pub fn run_sync(
    source_type: &str,
    source_config: &str,
    destination_type: &str,
    destination_config: &str,
    limit: Option<u64>,
    compliance_rules: Option<&str>,
) -> PyResult<PySyncResult> {
    let source_cfg_value = parse_json(source_config)?;
    let dest_cfg_value = parse_json(destination_config)?;

    let source = build_source_spec(source_type, &source_cfg_value)?;
    let destination = build_destination_spec(destination_type, &dest_cfg_value)?;
    let rules = build_compliance_rules(compliance_rules)?;

    let options = ExecuteOptions {
        limit,
        compliance_rules: rules,
    };

    let runtime = tokio::runtime::Runtime::new().map_err(py_err)?;
    let result = runtime
        .block_on(execute_sync(source, destination, options, lineage_store()))
        .map_err(py_err)?;

    Ok(PySyncResult {
        run_id: result.run_id,
        rows_read: result.rows_read,
        rows_written: result.rows_written,
        rows_failed: result.rows_failed,
        compliance_violations: result.compliance_violations,
        started_at: result.started_at.to_rfc3339(),
        completed_at: result.completed_at.to_rfc3339(),
        duration_ms: result.duration_ms,
    })
}

/// Export the real lineage graph (accumulated across every `run_sync` call in
/// this process) as JSON: nodes are systems connectors have actually touched,
/// edges are real sync runs with real record counts and timestamps.
#[pyfunction]
pub fn lineage_json() -> String {
    lineage_store().to_json().to_string()
}

/// Export the real lineage graph as Graphviz DOT for visualization.
#[pyfunction]
pub fn lineage_dot() -> String {
    lineage_store().to_dot()
}

#[pymodule]
fn _core(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_class::<PyWorkflow>()?;
    m.add_class::<PyDestination>()?;
    m.add_class::<PyActivation>()?;
    m.add_class::<PyEntity>()?;
    m.add_class::<PySyncRun>()?;
    m.add_class::<PySyncResult>()?;
    m.add_function(wrap_pyfunction!(run_sync, m)?)?;
    m.add_function(wrap_pyfunction!(lineage_json, m)?)?;
    m.add_function(wrap_pyfunction!(lineage_dot, m)?)?;
    Ok(())
}
