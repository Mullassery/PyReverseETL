use crate::adapters::{FieldMapping, Transformation};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// YAML-based field mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingConfig {
    pub mappings: Vec<MappingEntry>,
    pub dedup_field: Option<String>,
    pub batch_size: Option<u32>,
}

/// Single field mapping entry from YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingEntry {
    /// Source field name (from entity)
    pub source: String,
    /// Destination field name (in adapter/API)
    pub destination: String,
    /// Transformation to apply (optional)
    pub transform: Option<TransformConfig>,
    /// Whether this field is required
    pub required: Option<bool>,
}

/// Transformation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TransformConfig {
    /// Simple transformations (identity, uppercase, lowercase, timestamp)
    Simple(String),
    /// Decimal rounding: { round: 2 }
    RoundDecimals { round: u32 },
    /// Custom transformation: { custom: "expression" }
    Custom { custom: String },
}

impl MappingConfig {
    /// Parse YAML string into MappingConfig
    pub fn from_yaml(yaml_str: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml_str)
    }

    /// Load from YAML file path
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        Ok(Self::from_yaml(&content)?)
    }

    /// Convert to FieldMapping structs
    pub fn to_field_mappings(&self) -> Vec<FieldMapping> {
        self.mappings
            .iter()
            .map(|entry| {
                let transformation = entry.transform.as_ref().map(|t| t.to_transformation());
                FieldMapping {
                    source_field: entry.source.clone(),
                    destination_field: entry.destination.clone(),
                    transformation,
                    required: entry.required.unwrap_or(false),
                }
            })
            .collect()
    }

    /// Convert to YAML string
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }
}

impl TransformConfig {
    /// Convert to Transformation enum
    fn to_transformation(&self) -> Transformation {
        match self {
            TransformConfig::Simple(s) => match s.as_str() {
                "identity" => Transformation::Identity,
                "uppercase" => Transformation::Uppercase,
                "lowercase" => Transformation::Lowercase,
                "timestamp" => Transformation::ToTimestamp,
                _ => Transformation::Identity,
            },
            TransformConfig::RoundDecimals { round } => Transformation::RoundDecimals(*round),
            TransformConfig::Custom { custom } => Transformation::Custom(custom.clone()),
        }
    }
}

/// YAML examples for common destinations
pub mod examples {
    pub const SALESFORCE_MAPPING: &str = r#"
mappings:
  - source: customer_id
    destination: Id
    required: true
  - source: email
    destination: Email__c
    required: true
  - source: first_name
    destination: FirstName
    transform: identity
  - source: last_name
    destination: LastName
    transform: identity
  - source: ltv
    destination: LifetimeValue__c
    transform:
      round: 2
  - source: created_at
    destination: CreatedDate
    transform: timestamp

external_id_field: Email__c
batch_size: 10000
"#;

    pub const HUBSPOT_MAPPING: &str = r#"
mappings:
  - source: email
    destination: email
    required: true
  - source: first_name
    destination: firstname
    transform: identity
  - source: last_name
    destination: lastname
    transform: identity
  - source: phone
    destination: phone
    required: false
  - source: company
    destination: company
    transform: identity
  - source: lifecycle_stage
    destination: lifecyclestage
    required: false

dedup_field: email
batch_size: 100
"#;

    pub const MARKETO_MAPPING: &str = r#"
mappings:
  - source: email
    destination: email
    required: true
  - source: first_name
    destination: firstName
    transform: identity
  - source: last_name
    destination: lastName
    transform: identity
  - source: phone
    destination: phone
    required: false
  - source: company
    destination: company
    transform: identity
  - source: revenue
    destination: annualRevenue
    transform:
      round: 0

dedup_field: email
batch_size: 300
"#;

    pub const WEBHOOK_MAPPING: &str = r#"
mappings:
  - source: customer_id
    destination: id
    required: true
  - source: email
    destination: email
    required: true
  - source: name
    destination: name
    transform: identity
  - source: status
    destination: status
    transform: uppercase

batch_size: 1000
"#;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_salesforce_yaml() {
        let config = MappingConfig::from_yaml(examples::SALESFORCE_MAPPING).unwrap();
        assert_eq!(config.mappings.len(), 6);
        assert_eq!(config.mappings[0].source, "customer_id");
        assert_eq!(config.mappings[0].destination, "Id");
    }

    #[test]
    fn test_parse_hubspot_yaml() {
        let config = MappingConfig::from_yaml(examples::HUBSPOT_MAPPING).unwrap();
        assert_eq!(config.mappings.len(), 6);
        assert_eq!(config.dedup_field, Some("email".to_string()));
        assert_eq!(config.batch_size, Some(100));
    }

    #[test]
    fn test_parse_marketo_yaml() {
        let config = MappingConfig::from_yaml(examples::MARKETO_MAPPING).unwrap();
        assert_eq!(config.mappings.len(), 6);
        assert_eq!(config.batch_size, Some(300));
    }

    #[test]
    fn test_mapping_to_field_mappings() {
        let config = MappingConfig::from_yaml(examples::WEBHOOK_MAPPING).unwrap();
        let field_mappings = config.to_field_mappings();
        assert_eq!(field_mappings.len(), 4);
        assert_eq!(field_mappings[0].source_field, "customer_id");
        assert!(field_mappings[0].required);
    }

    #[test]
    fn test_transformation_identity() {
        let t = TransformConfig::Simple("identity".to_string());
        match t.to_transformation() {
            Transformation::Identity => {}
            _ => panic!("Expected Identity transformation"),
        }
    }

    #[test]
    fn test_transformation_round_decimals() {
        let t = TransformConfig::RoundDecimals { round: 2 };
        match t.to_transformation() {
            Transformation::RoundDecimals(n) => assert_eq!(n, 2),
            _ => panic!("Expected RoundDecimals transformation"),
        }
    }

    #[test]
    fn test_yaml_roundtrip() {
        let config = MappingConfig::from_yaml(examples::SALESFORCE_MAPPING).unwrap();
        let yaml_str = config.to_yaml().unwrap();
        let config2 = MappingConfig::from_yaml(&yaml_str).unwrap();
        assert_eq!(config.mappings.len(), config2.mappings.len());
    }
}
