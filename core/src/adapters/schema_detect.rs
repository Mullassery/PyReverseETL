use super::{DestinationSchema, FieldType};
use crate::Entity;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// Automatic schema detection from entity data
pub struct SchemaDetector;

impl SchemaDetector {
    /// Detect schema from a collection of entities
    pub fn detect_from_entities(entities: &[Entity]) -> DestinationSchema {
        let mut field_types: HashMap<String, Vec<FieldType>> = HashMap::new();
        let mut field_counts: HashMap<String, usize> = HashMap::new();

        for entity in entities {
            Self::analyze_entity(&entity, &mut field_types, &mut field_counts);
        }

        // Aggregate detected types and determine most likely types
        let fields = field_types
            .into_iter()
            .map(|(field_name, types)| {
                let aggregated_type = Self::aggregate_types(&types);
                (field_name, aggregated_type)
            })
            .collect();

        DestinationSchema {
            fields,
            required_fields: Vec::new(),
            max_batch_size: 1000,
        }
    }

    /// Analyze a single entity for field types
    fn analyze_entity(
        entity: &Entity,
        field_types: &mut HashMap<String, Vec<FieldType>>,
        field_counts: &mut HashMap<String, usize>,
    ) {
        // Analyze attributes
        if let serde_json::Value::Object(attrs) = &entity.attributes {
            for (name, value) in attrs {
                let field_key = name.clone();
                let detected_type = Self::infer_type(value);
                field_types.entry(field_key.clone()).or_insert_with(Vec::new).push(detected_type);
                *field_counts.entry(field_key).or_insert(0) += 1;
            }
        }

        // Analyze traits
        if let serde_json::Value::Object(traits) = &entity.traits {
            for (name, value) in traits {
                let field_key = format!("trait_{}", name);
                let detected_type = Self::infer_type(value);
                field_types.entry(field_key.clone()).or_insert_with(Vec::new).push(detected_type);
                *field_counts.entry(field_key).or_insert(0) += 1;
            }
        }
    }

    /// Infer the type of a JSON value
    pub fn infer_type(value: &Value) -> FieldType {
        match value {
            Value::Null => FieldType::String { max_length: None },
            Value::Bool(_) => FieldType::Boolean,
            Value::Number(n) => {
                if n.is_f64() {
                    FieldType::Float
                } else {
                    FieldType::Integer
                }
            }
            Value::String(s) => Self::infer_string_type(s),
            Value::Array(_) => FieldType::Custom("array".to_string()),
            Value::Object(_) => FieldType::Custom("object".to_string()),
        }
    }

    /// Infer string type from content
    fn infer_string_type(s: &str) -> FieldType {
        if s.is_empty() {
            return FieldType::String {
                max_length: Some(255),
            };
        }

        // Check for email
        if s.contains('@') && s.contains('.') && s.len() > 5 {
            if is_valid_email(s) {
                return FieldType::Email;
            }
        }

        // Check for URL
        if s.starts_with("http://") || s.starts_with("https://") || s.starts_with("ftp://") {
            if is_valid_url(s) {
                return FieldType::Url;
            }
        }

        // Check for ISO 8601 datetime
        if is_iso8601_datetime(s) {
            return FieldType::DateTime;
        }

        // Default to string with estimated max length
        let max_len = (s.len() as f64 * 1.2).ceil() as u32;
        let max_len = std::cmp::max(50, std::cmp::min(max_len, 65536));
        FieldType::String {
            max_length: Some(max_len),
        }
    }

    /// Aggregate multiple detected types into a single type
    fn aggregate_types(types: &[FieldType]) -> FieldType {
        if types.is_empty() {
            return FieldType::String { max_length: None };
        }

        let type_counts = Self::count_types(types);

        // Return the most common type
        type_counts
            .into_iter()
            .max_by_key(|(_field_type, count)| *count)
            .map(|(field_type, _count)| field_type)
            .unwrap_or(FieldType::String { max_length: None })
    }

    /// Count occurrences of each type
    fn count_types(types: &[FieldType]) -> Vec<(FieldType, usize)> {
        let mut type_strings: HashMap<String, usize> = HashMap::new();

        for t in types {
            let key = format!("{:?}", t);
            *type_strings.entry(key).or_insert(0) += 1;
        }

        // Reconstruct types with counts (note: this is simplified)
        types
            .iter()
            .map(|t| (t.clone(), *type_strings.get(&format!("{:?}", t)).unwrap_or(&1)))
            .collect()
    }

    /// Suggest required fields based on field presence
    pub fn suggest_required_fields(entities: &[Entity], presence_threshold: f64) -> Vec<String> {
        if entities.is_empty() {
            return Vec::new();
        }

        let mut field_presence: HashMap<String, usize> = HashMap::new();
        let mut all_fields = HashSet::new();

        for entity in entities {
            if let serde_json::Value::Object(attrs) = &entity.attributes {
                for name in attrs.keys() {
                    *field_presence.entry(name.clone()).or_insert(0) += 1;
                    all_fields.insert(name.clone());
                }
            }
        }

        let entity_count = entities.len() as f64;
        field_presence
            .into_iter()
            .filter(|(_field, count)| {
                let presence = *count as f64 / entity_count;
                presence >= presence_threshold
            })
            .map(|(field, _count)| field)
            .collect()
    }

    /// Generate statistics about field usage
    pub fn generate_statistics(entities: &[Entity]) -> FieldStatistics {
        let mut field_stats: HashMap<String, FieldStats> = HashMap::new();

        for entity in entities {
            if let serde_json::Value::Object(attrs) = &entity.attributes {
                for (name, value) in attrs {
                    let stats = field_stats.entry(name.clone()).or_insert(FieldStats {
                        name: name.clone(),
                        count: 0,
                        null_count: 0,
                        types_detected: HashSet::new(),
                    });

                    stats.count += 1;
                    if value.is_null() {
                        stats.null_count += 1;
                    } else {
                        let detected_type = Self::infer_type(value);
                        stats.types_detected.insert(format!("{:?}", detected_type));
                    }
                }
            }
        }

        FieldStatistics {
            total_entities: entities.len(),
            fields: field_stats.into_values().collect(),
        }
    }
}

/// Field statistics for analysis
#[derive(Debug, Clone)]
pub struct FieldStats {
    pub name: String,
    pub count: usize,
    pub null_count: usize,
    pub types_detected: HashSet<String>,
}

/// Collection of field statistics
#[derive(Debug, Clone)]
pub struct FieldStatistics {
    pub total_entities: usize,
    pub fields: Vec<FieldStats>,
}

impl FieldStatistics {
    /// Get field presence percentage
    pub fn field_presence(&self, field_name: &str) -> f64 {
        if self.total_entities == 0 {
            return 0.0;
        }

        self.fields
            .iter()
            .find(|f| f.name == field_name)
            .map(|f| (f.count as f64 / self.total_entities as f64) * 100.0)
            .unwrap_or(0.0)
    }

    /// Get null percentage for field
    pub fn null_percentage(&self, field_name: &str) -> f64 {
        if self.total_entities == 0 {
            return 0.0;
        }

        self.fields
            .iter()
            .find(|f| f.name == field_name)
            .map(|f| (f.null_count as f64 / f.count as f64) * 100.0)
            .unwrap_or(0.0)
    }
}

// Email validation helper (basic)
fn is_valid_email(s: &str) -> bool {
    let parts: Vec<&str> = s.split('@').collect();
    parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() && parts[1].contains('.')
}

// URL validation helper (basic)
fn is_valid_url(s: &str) -> bool {
    s.contains("://") && s.len() > 10
}

// ISO 8601 datetime validation (basic)
fn is_iso8601_datetime(s: &str) -> bool {
    (s.len() >= 10) && s.chars().nth(4).map(|c| c == '-').unwrap_or(false) && (s.contains('T') || s.contains(' '))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::EntityType;
    use serde_json::json;

    #[test]
    fn test_infer_type_email() {
        let value = json!("test@example.com");
        match SchemaDetector::infer_type(&value) {
            FieldType::Email => {}
            t => panic!("Expected Email, got {:?}", t),
        }
    }

    #[test]
    fn test_infer_type_url() {
        let value = json!("https://example.com/page");
        match SchemaDetector::infer_type(&value) {
            FieldType::Url => {}
            t => panic!("Expected Url, got {:?}", t),
        }
    }

    #[test]
    fn test_infer_type_datetime() {
        let value = json!("2026-07-15T10:30:00Z");
        match SchemaDetector::infer_type(&value) {
            FieldType::DateTime => {}
            t => panic!("Expected DateTime, got {:?}", t),
        }
    }

    #[test]
    fn test_infer_type_integer() {
        let value = json!(42);
        match SchemaDetector::infer_type(&value) {
            FieldType::Integer => {}
            t => panic!("Expected Integer, got {:?}", t),
        }
    }

    #[test]
    fn test_infer_type_float() {
        let value = json!(3.14);
        match SchemaDetector::infer_type(&value) {
            FieldType::Float => {}
            t => panic!("Expected Float, got {:?}", t),
        }
    }

    #[test]
    fn test_infer_type_boolean() {
        let value = json!(true);
        match SchemaDetector::infer_type(&value) {
            FieldType::Boolean => {}
            t => panic!("Expected Boolean, got {:?}", t),
        }
    }

    #[test]
    fn test_detect_from_entities() {
        let mut entity = Entity::new(EntityType::Customer, "id", "cust_1");
        entity = entity.add_attribute("email", json!("test@example.com"));
        entity = entity.add_attribute("revenue", json!(5000.50));

        let schema = SchemaDetector::detect_from_entities(&[entity]);
        assert!(schema.fields.contains_key("email"));
        assert!(schema.fields.contains_key("revenue"));
    }

    #[test]
    fn test_suggest_required_fields() {
        let entity1 = Entity::new(EntityType::Customer, "id", "cust_1")
            .add_attribute("email", json!("test1@example.com"))
            .add_attribute("name", json!("Customer 1"));
        let entity2 = Entity::new(EntityType::Customer, "id", "cust_2")
            .add_attribute("email", json!("test2@example.com"));

        let required = SchemaDetector::suggest_required_fields(&[entity1, entity2], 0.8);
        assert!(required.contains(&"email".to_string()));
        assert!(!required.contains(&"name".to_string()));
    }

    #[test]
    fn test_field_statistics() {
        let entity = Entity::new(EntityType::Customer, "id", "cust_1")
            .add_attribute("email", json!("test@example.com"));

        let stats = SchemaDetector::generate_statistics(&[entity]);
        assert_eq!(stats.total_entities, 1);
        assert!(stats.fields.iter().any(|f| f.name == "email"));
    }
}
