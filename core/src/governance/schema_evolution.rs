/// Schema Evolution Detection
///
/// Detects and handles upstream schema changes:
/// - Field additions/removals
/// - Type changes
/// - Field renames
/// - Mapping migration
///
/// This is deliberately scoped to *structural* shape tracking (does this
/// record's field-name/type shape still match what was last seen for this
/// source/destination pair) -- not statistical data-quality drift
/// (distribution shifts, quality scores), which `governance/quality_gate.rs`
/// correctly leaves to a real StatGuardian integration (see the note in
/// `error.rs`). `DefaultSchemaEvolution::detect_changes` used to
/// unconditionally return `Ok(vec![])` ("would compare against registered
/// schema" per its own comment, with no such comparison ever implemented);
/// it now persists the last-seen shape (in-memory or SQLite) and does a
/// real field-by-field diff against it.
use crate::{Entity, Result};
use async_trait::async_trait;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

/// Type of schema change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchemaChangeType {
    Added,
    Removed,
    TypeChanged,
    Renamed,
}

/// Individual schema change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaChange {
    pub field_name: String,
    pub change_type: SchemaChangeType,
    pub old_type: Option<String>,
    pub new_type: Option<String>,
    pub details: Option<String>,
}

/// Schema evolution trait - detects and handles schema changes
#[async_trait]
pub trait SchemaEvolution: Send + Sync {
    /// Detect schema changes in entity
    async fn detect_changes(&self, entity: &Entity) -> Result<Vec<SchemaChange>>;

    /// Get current schema version
    async fn get_schema_version(&self) -> Result<String>;

    /// Check if schema version is compatible
    async fn is_compatible(&self, current: &str, required: &str) -> Result<bool>;
}

/// Mock schema evolution for testing
#[cfg(test)]
pub struct MockSchemaEvolution {
    changes: Vec<SchemaChange>,
}

#[cfg(test)]
impl MockSchemaEvolution {
    pub fn new(changes: Vec<SchemaChange>) -> Self {
        Self { changes }
    }

    pub fn no_changes() -> Self {
        Self { changes: vec![] }
    }
}

#[cfg(test)]
#[async_trait]
impl SchemaEvolution for MockSchemaEvolution {
    async fn detect_changes(&self, _entity: &Entity) -> Result<Vec<SchemaChange>> {
        Ok(self.changes.clone())
    }

    async fn get_schema_version(&self) -> Result<String> {
        Ok("v1.0.0".to_string())
    }

    async fn is_compatible(&self, current: &str, required: &str) -> Result<bool> {
        // Simple version comparison: same or newer is compatible
        Ok(current >= required)
    }
}

/// A field name -> simplified JSON type-tag shape, e.g.
/// `{"email": "string", "age": "number"}`. Deliberately coarse (JSON's own
/// type system, not a destination-specific one) since this is a "does the
/// shape a source is producing still look like before" check, not a
/// destination-schema-compatibility check.
type FieldShape = HashMap<String, String>;

fn infer_shape(value: &Value) -> FieldShape {
    let mut shape = FieldShape::new();
    if let Some(obj) = value.as_object() {
        for (key, val) in obj {
            shape.insert(key.clone(), json_type_tag(val));
        }
    }
    shape
}

fn json_type_tag(value: &Value) -> String {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
    .to_string()
}

/// Diffs `previous` against `current`, producing one `SchemaChange` per
/// added/removed/type-changed field. Field renames aren't detected here (a
/// rename is indistinguishable from a remove+add without a stable field-id
/// to track across the boundary) -- they surface as one `Removed` + one
/// `Added` `SchemaChange`, which is still actionable (and honest) even
/// though it doesn't identify the rename as a single event.
fn diff_shapes(previous: &FieldShape, current: &FieldShape) -> Vec<SchemaChange> {
    let mut changes = Vec::new();

    for (field, current_type) in current {
        match previous.get(field) {
            None => changes.push(SchemaChange {
                field_name: field.clone(),
                change_type: SchemaChangeType::Added,
                old_type: None,
                new_type: Some(current_type.clone()),
                details: None,
            }),
            Some(previous_type) if previous_type != current_type => {
                changes.push(SchemaChange {
                    field_name: field.clone(),
                    change_type: SchemaChangeType::TypeChanged,
                    old_type: Some(previous_type.clone()),
                    new_type: Some(current_type.clone()),
                    details: None,
                });
            }
            Some(_) => {}
        }
    }

    for (field, previous_type) in previous {
        if !current.contains_key(field) {
            changes.push(SchemaChange {
                field_name: field.clone(),
                change_type: SchemaChangeType::Removed,
                old_type: Some(previous_type.clone()),
                new_type: None,
                details: None,
            });
        }
    }

    changes.sort_by(|a, b| a.field_name.cmp(&b.field_name));
    changes
}

enum ShapeStore {
    /// Not persisted across process restarts -- fine for tests and for
    /// short-lived callers that only care about drift *within* one process
    /// lifetime, but a fresh process starts with no prior shape (so the
    /// first `detect_changes` call always reports every field as `Added`).
    InMemory(Mutex<HashMap<String, FieldShape>>),
    /// Persisted to a real SQLite file, so drift is detected across
    /// separate `pyreverseetl execute` invocations, which is the actual
    /// real-world shape of this problem (each sync run is a fresh process).
    Sqlite(Mutex<Connection>),
}

/// Default schema evolution implementation: persists the last-seen field
/// shape (in-memory or SQLite-backed) and diffs each new entity against it.
pub struct DefaultSchemaEvolution {
    current_version: String,
    schema_key: String,
    store: ShapeStore,
}

impl DefaultSchemaEvolution {
    /// In-memory: real diffing, but no persistence across process restarts.
    pub fn new(current_version: String) -> Self {
        Self {
            current_version,
            schema_key: "default".to_string(),
            store: ShapeStore::InMemory(Mutex::new(HashMap::new())),
        }
    }

    /// SQLite-backed: real diffing, persisted across process restarts.
    /// `schema_key` scopes the stored shape (e.g. `"postgres:orders->hubspot:contacts"`)
    /// so tracking multiple source/destination pairs against the same file
    /// doesn't collide.
    pub fn with_sqlite_store(
        current_version: String,
        schema_key: String,
        path: &Path,
    ) -> Result<Self> {
        let conn = Connection::open(path)?;
        Self::init_table(&conn)?;
        Ok(Self {
            current_version,
            schema_key,
            store: ShapeStore::Sqlite(Mutex::new(conn)),
        })
    }

    /// SQLite-backed but in an in-memory database -- for tests that want to
    /// exercise the real SQLite code path without touching disk.
    #[cfg(test)]
    pub fn with_sqlite_in_memory(current_version: String, schema_key: String) -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::init_table(&conn)?;
        Ok(Self {
            current_version,
            schema_key,
            store: ShapeStore::Sqlite(Mutex::new(conn)),
        })
    }

    fn init_table(conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_shapes (
                schema_key TEXT PRIMARY KEY,
                shape_json TEXT NOT NULL
            )",
            [],
        )?;
        Ok(())
    }

    fn load_shape(&self) -> Result<FieldShape> {
        match &self.store {
            ShapeStore::InMemory(map) => Ok(map
                .lock()
                .unwrap()
                .get(&self.schema_key)
                .cloned()
                .unwrap_or_default()),
            ShapeStore::Sqlite(conn) => {
                let conn = conn.lock().unwrap();
                let stored: Option<String> = conn
                    .query_row(
                        "SELECT shape_json FROM schema_shapes WHERE schema_key = ?1",
                        params![self.schema_key],
                        |row| row.get(0),
                    )
                    .optional()?;
                Ok(match stored {
                    Some(json) => serde_json::from_str(&json)?,
                    None => FieldShape::new(),
                })
            }
        }
    }

    fn save_shape(&self, shape: &FieldShape) -> Result<()> {
        match &self.store {
            ShapeStore::InMemory(map) => {
                map.lock()
                    .unwrap()
                    .insert(self.schema_key.clone(), shape.clone());
                Ok(())
            }
            ShapeStore::Sqlite(conn) => {
                let conn = conn.lock().unwrap();
                let json = serde_json::to_string(shape)?;
                conn.execute(
                    "INSERT INTO schema_shapes (schema_key, shape_json) VALUES (?1, ?2)
                     ON CONFLICT(schema_key) DO UPDATE SET shape_json = excluded.shape_json",
                    params![self.schema_key, json],
                )?;
                Ok(())
            }
        }
    }
}

#[async_trait]
impl SchemaEvolution for DefaultSchemaEvolution {
    async fn detect_changes(&self, entity: &Entity) -> Result<Vec<SchemaChange>> {
        let previous = self.load_shape()?;
        let current = infer_shape(&entity.attributes);

        let changes = if previous.is_empty() {
            // Nothing recorded yet for this schema_key: this is the first
            // observation, not "every field just appeared" -- there's
            // nothing to diff against yet, so report no changes but still
            // persist the shape so the *next* call has something real to
            // compare against.
            Vec::new()
        } else {
            diff_shapes(&previous, &current)
        };

        if !current.is_empty() {
            self.save_shape(&current)?;
        }

        Ok(changes)
    }

    async fn get_schema_version(&self) -> Result<String> {
        Ok(self.current_version.clone())
    }

    async fn is_compatible(&self, current: &str, required: &str) -> Result<bool> {
        Ok(current >= required)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_no_schema_changes() {
        use crate::entity::EntityType;
        let evolution = MockSchemaEvolution::no_changes();
        let entity = Entity::new(EntityType::Custom("test".to_string()), "id", "test");

        let changes = evolution.detect_changes(&entity).await.unwrap();
        assert!(changes.is_empty());
    }

    #[tokio::test]
    async fn test_schema_changes_detected() {
        use crate::entity::EntityType;
        let changes = vec![SchemaChange {
            field_name: "created_at".to_string(),
            change_type: SchemaChangeType::Added,
            old_type: None,
            new_type: Some("datetime".to_string()),
            details: None,
        }];

        let evolution = MockSchemaEvolution::new(changes.clone());
        let entity = Entity::new(EntityType::Custom("test".to_string()), "id", "test");

        let detected = evolution.detect_changes(&entity).await.unwrap();
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].field_name, "created_at");
    }

    #[tokio::test]
    async fn test_schema_version() {
        let evolution = MockSchemaEvolution::no_changes();
        let version = evolution.get_schema_version().await.unwrap();
        assert_eq!(version, "v1.0.0");
    }

    #[tokio::test]
    async fn test_version_compatibility() {
        let evolution = MockSchemaEvolution::no_changes();
        let compatible = evolution.is_compatible("v2.0.0", "v1.0.0").await.unwrap();
        assert!(compatible);

        let incompatible = evolution.is_compatible("v0.5.0", "v1.0.0").await.unwrap();
        assert!(!incompatible);
    }

    fn entity_with(attrs: Value) -> Entity {
        use crate::entity::EntityType;
        let mut entity = Entity::new(EntityType::Custom("test".to_string()), "id", "rec1");
        entity.attributes = attrs;
        entity
    }

    #[tokio::test]
    async fn default_evolution_reports_no_changes_on_first_observation() {
        let evolution = DefaultSchemaEvolution::new("v1".to_string());
        let entity = entity_with(serde_json::json!({"email": "a@b.com", "age": 30}));

        let changes = evolution.detect_changes(&entity).await.unwrap();

        assert!(changes.is_empty(), "nothing to diff against yet");
    }

    #[tokio::test]
    async fn default_evolution_detects_added_field() {
        let evolution = DefaultSchemaEvolution::new("v1".to_string());
        let first = entity_with(serde_json::json!({"email": "a@b.com"}));
        evolution.detect_changes(&first).await.unwrap();

        let second = entity_with(serde_json::json!({"email": "a@b.com", "phone": "555-1234"}));
        let changes = evolution.detect_changes(&second).await.unwrap();

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field_name, "phone");
        assert_eq!(changes[0].change_type, SchemaChangeType::Added);
    }

    #[tokio::test]
    async fn default_evolution_detects_removed_field() {
        let evolution = DefaultSchemaEvolution::new("v1".to_string());
        let first = entity_with(serde_json::json!({"email": "a@b.com", "phone": "555-1234"}));
        evolution.detect_changes(&first).await.unwrap();

        let second = entity_with(serde_json::json!({"email": "a@b.com"}));
        let changes = evolution.detect_changes(&second).await.unwrap();

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field_name, "phone");
        assert_eq!(changes[0].change_type, SchemaChangeType::Removed);
    }

    #[tokio::test]
    async fn default_evolution_detects_type_change() {
        let evolution = DefaultSchemaEvolution::new("v1".to_string());
        let first = entity_with(serde_json::json!({"age": 30}));
        evolution.detect_changes(&first).await.unwrap();

        let second = entity_with(serde_json::json!({"age": "thirty"}));
        let changes = evolution.detect_changes(&second).await.unwrap();

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field_name, "age");
        assert_eq!(changes[0].change_type, SchemaChangeType::TypeChanged);
        assert_eq!(changes[0].old_type.as_deref(), Some("number"));
        assert_eq!(changes[0].new_type.as_deref(), Some("string"));
    }

    #[tokio::test]
    async fn default_evolution_no_changes_when_shape_is_stable() {
        let evolution = DefaultSchemaEvolution::new("v1".to_string());
        let first = entity_with(serde_json::json!({"email": "a@b.com", "age": 30}));
        evolution.detect_changes(&first).await.unwrap();

        let second = entity_with(serde_json::json!({"email": "c@d.com", "age": 40}));
        let changes = evolution.detect_changes(&second).await.unwrap();

        assert!(changes.is_empty());
    }

    #[tokio::test]
    async fn sqlite_backed_evolution_persists_across_instances() {
        // Real SQLite persistence: two separate `DefaultSchemaEvolution`
        // instances sharing the same schema_key/connection-equivalent must
        // see each other's shape, exactly like two separate `pyreverseetl
        // execute` process invocations would via a real file.
        let dir = std::env::temp_dir().join(format!("pyreverseetl_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("schema.db");

        let first_process = DefaultSchemaEvolution::with_sqlite_store(
            "v1".to_string(),
            "src->dst".to_string(),
            &db_path,
        )
        .unwrap();
        first_process
            .detect_changes(&entity_with(serde_json::json!({"email": "a@b.com"})))
            .await
            .unwrap();
        drop(first_process);

        let second_process = DefaultSchemaEvolution::with_sqlite_store(
            "v1".to_string(),
            "src->dst".to_string(),
            &db_path,
        )
        .unwrap();
        let changes = second_process
            .detect_changes(&entity_with(
                serde_json::json!({"email": "a@b.com", "new_field": true}),
            ))
            .await
            .unwrap();

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field_name, "new_field");
        assert_eq!(changes[0].change_type, SchemaChangeType::Added);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn different_schema_keys_do_not_collide_in_the_same_file() {
        let dir = std::env::temp_dir().join(format!("pyreverseetl_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("schema.db");

        let evolution_a =
            DefaultSchemaEvolution::with_sqlite_store("v1".to_string(), "a".to_string(), &db_path)
                .unwrap();
        let evolution_b =
            DefaultSchemaEvolution::with_sqlite_store("v1".to_string(), "b".to_string(), &db_path)
                .unwrap();

        // Seed key "a" with a shape that has "x"; key "b" has never been seen.
        evolution_a
            .detect_changes(&entity_with(serde_json::json!({"x": 1})))
            .await
            .unwrap();

        // key "b" observing a record with "x" for the first time must be
        // treated as a first observation (no changes), not diffed against
        // key "a"'s stored shape.
        let changes_b = evolution_b
            .detect_changes(&entity_with(serde_json::json!({"x": 1})))
            .await
            .unwrap();
        assert!(changes_b.is_empty(), "key 'b' must not see key 'a's shape");

        // key "a" observing a genuinely new field must still be detected
        // correctly after "b" has also written to the same file.
        let changes_a = evolution_a
            .detect_changes(&entity_with(serde_json::json!({"x": 1, "y": 2})))
            .await
            .unwrap();
        assert_eq!(changes_a.len(), 1);
        assert_eq!(changes_a[0].field_name, "y");

        std::fs::remove_dir_all(&dir).ok();
    }
}
