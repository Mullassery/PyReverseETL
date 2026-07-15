use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Type of change detected
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    /// New entity created
    Created,
    /// Existing entity updated
    Updated,
    /// Entity deleted
    Deleted,
}

/// Represents a single entity change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    /// Entity identifier
    pub entity_id: String,
    /// Type of change
    pub change_type: ChangeType,
    /// Previous state (None for Created)
    pub before: Option<Value>,
    /// Current state
    pub after: Value,
    /// When change occurred
    pub timestamp: DateTime<Utc>,
    /// List of fields that changed
    pub changed_fields: Vec<String>,
}

/// Detects changes in entities by comparing before/after states
pub struct ChangeDetector {
    previous_state: Arc<Mutex<HashMap<String, Value>>>,
}

impl ChangeDetector {
    /// Create new change detector
    pub fn new() -> Self {
        Self {
            previous_state: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Detect changes for a new entity state
    pub async fn detect(&self, entity_id: String, current: Value) -> Option<Change> {
        let mut state = self.previous_state.lock().await;

        let (before, change_type, changed_fields) = match state.get(&entity_id) {
            None => {
                // New entity
                (None, ChangeType::Created, vec![])
            }
            Some(prev) => {
                // Existing entity - compare
                let fields = Self::compare_values(prev, &current);
                if fields.is_empty() {
                    // No changes
                    return None;
                }
                (Some(prev.clone()), ChangeType::Updated, fields)
            }
        };

        state.insert(entity_id.clone(), current.clone());

        Some(Change {
            entity_id,
            change_type,
            before,
            after: current,
            timestamp: Utc::now(),
            changed_fields,
        })
    }

    /// Detect deletion of an entity
    pub async fn detect_deletion(&self, entity_id: String) -> Option<Change> {
        let mut state = self.previous_state.lock().await;

        if let Some(before) = state.remove(&entity_id) {
            Some(Change {
                entity_id,
                change_type: ChangeType::Deleted,
                before: Some(before.clone()),
                after: Value::Null,
                timestamp: Utc::now(),
                changed_fields: vec![],
            })
        } else {
            None
        }
    }

    /// Compare two JSON values and return list of changed field paths
    pub fn compare_values(before: &Value, after: &Value) -> Vec<String> {
        let mut changed = Vec::new();

        match (before, after) {
            (Value::Object(before_obj), Value::Object(after_obj)) => {
                // Check for modified and new fields
                for (key, after_val) in after_obj {
                    if let Some(before_val) = before_obj.get(key) {
                        if before_val != after_val {
                            changed.push(key.clone());
                        }
                    } else {
                        changed.push(key.clone());
                    }
                }
                // Check for deleted fields
                for key in before_obj.keys() {
                    if !after_obj.contains_key(key) {
                        changed.push(key.clone());
                    }
                }
            }
            _ => {
                if before != after {
                    changed.push("_root".to_string());
                }
            }
        }

        changed
    }

    /// Reset detector state (clear all previous states)
    pub async fn reset(&self) {
        self.previous_state.lock().await.clear();
    }
}

impl Default for ChangeDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_detect_created_entity() {
        let detector = ChangeDetector::new();
        let entity_data = json!({"id": "1", "name": "Alice"});

        let change = detector.detect("1".to_string(), entity_data).await;

        assert!(change.is_some());
        let change = change.unwrap();
        assert_eq!(change.change_type, ChangeType::Created);
        assert_eq!(change.entity_id, "1");
        assert_eq!(change.before, None);
        assert_eq!(change.changed_fields.len(), 0);
    }

    #[tokio::test]
    async fn test_detect_updated_entity() {
        let detector = ChangeDetector::new();
        let data1 = json!({"id": "1", "name": "Alice", "email": "alice@example.com"});
        let data2 = json!({"id": "1", "name": "Alice Updated", "email": "alice@example.com"});

        // First detection creates
        detector.detect("1".to_string(), data1).await;

        // Second detection should show update
        let change = detector.detect("1".to_string(), data2).await;

        assert!(change.is_some());
        let change = change.unwrap();
        assert_eq!(change.change_type, ChangeType::Updated);
        assert!(change.changed_fields.contains(&"name".to_string()));
        assert!(!change.changed_fields.contains(&"email".to_string()));
    }

    #[tokio::test]
    async fn test_detect_deleted_entity() {
        let detector = ChangeDetector::new();
        let entity_data = json!({"id": "1", "name": "Alice"});

        detector.detect("1".to_string(), entity_data).await;
        let change = detector.detect_deletion("1".to_string()).await;

        assert!(change.is_some());
        let change = change.unwrap();
        assert_eq!(change.change_type, ChangeType::Deleted);
        assert_eq!(change.entity_id, "1");
        assert_eq!(change.after, Value::Null);
    }

    #[test]
    fn test_compare_values_simple() {
        let before = json!({"name": "Alice", "age": 30});
        let after = json!({"name": "Alice", "age": 31});

        let changed = ChangeDetector::compare_values(&before, &after);

        assert_eq!(changed.len(), 1);
        assert!(changed.contains(&"age".to_string()));
    }

    #[test]
    fn test_compare_values_new_field() {
        let before = json!({"name": "Alice"});
        let after = json!({"name": "Alice", "email": "alice@example.com"});

        let changed = ChangeDetector::compare_values(&before, &after);

        assert!(changed.contains(&"email".to_string()));
    }

    #[test]
    fn test_compare_values_deleted_field() {
        let before = json!({"name": "Alice", "email": "alice@example.com"});
        let after = json!({"name": "Alice"});

        let changed = ChangeDetector::compare_values(&before, &after);

        assert!(changed.contains(&"email".to_string()));
    }
}
