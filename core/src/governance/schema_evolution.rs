/// Schema Evolution Detection
///
/// Detects and handles upstream schema changes:
/// - Field additions/removals
/// - Type changes
/// - Field renames
/// - Mapping migration

use crate::{Entity, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

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

/// Default schema evolution implementation
pub struct DefaultSchemaEvolution {
    current_version: String,
}

impl DefaultSchemaEvolution {
    pub fn new(current_version: String) -> Self {
        Self { current_version }
    }
}

#[async_trait]
impl SchemaEvolution for DefaultSchemaEvolution {
    async fn detect_changes(&self, _entity: &Entity) -> Result<Vec<SchemaChange>> {
        // By default, no changes detected
        // In production, would compare against registered schema
        Ok(vec![])
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
        let evolution = MockSchemaEvolution::no_changes();
        let entity = Entity {
            id: "test".to_string(),
            data: serde_json::json!({}),
            metadata: Default::default(),
        };

        let changes = evolution.detect_changes(&entity).await.unwrap();
        assert!(changes.is_empty());
    }

    #[tokio::test]
    async fn test_schema_changes_detected() {
        let changes = vec![SchemaChange {
            field_name: "created_at".to_string(),
            change_type: SchemaChangeType::Added,
            old_type: None,
            new_type: Some("datetime".to_string()),
            details: None,
        }];

        let evolution = MockSchemaEvolution::new(changes.clone());
        let entity = Entity {
            id: "test".to_string(),
            data: serde_json::json!({}),
            metadata: Default::default(),
        };

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
}
