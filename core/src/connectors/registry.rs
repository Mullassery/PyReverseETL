/// Connector Registry and Discovery
///
/// Central registry for discovering, registering, and managing all available connectors

use super::{SourceConnector, DestinationConnector, ConnectorConfig, ConnectionPool};
use std::collections::HashMap;

/// Built-in connector descriptors
pub struct BuiltInConnectors;

impl BuiltInConnectors {
    /// Get all built-in source connectors
    pub fn sources() -> Vec<ConnectorDescriptor> {
        vec![
            ConnectorDescriptor {
                id: "postgres".to_string(),
                name: "PostgreSQL".to_string(),
                description: "Read data from PostgreSQL databases".to_string(),
                connector_type: "source".to_string(),
                capabilities: vec!["read", "incremental_read", "schema_detection", "batch"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                required_params: vec!["host", "port", "user", "password", "database"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            },
            ConnectorDescriptor {
                id: "mysql".to_string(),
                name: "MySQL".to_string(),
                description: "Read data from MySQL databases".to_string(),
                connector_type: "source".to_string(),
                capabilities: vec!["read", "incremental_read", "schema_detection", "batch"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                required_params: vec!["host", "port", "user", "password", "database"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            },
            ConnectorDescriptor {
                id: "csv".to_string(),
                name: "CSV Files".to_string(),
                description: "Read data from local or remote CSV files".to_string(),
                connector_type: "source".to_string(),
                capabilities: vec!["read", "schema_detection", "batch"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                required_params: vec!["path"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            },
            ConnectorDescriptor {
                id: "api".to_string(),
                name: "REST API".to_string(),
                description: "Read data from HTTP REST endpoints".to_string(),
                connector_type: "source".to_string(),
                capabilities: vec!["read", "incremental_read", "batch", "stream"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                required_params: vec!["url"].iter().map(|s| s.to_string()).collect(),
            },
            ConnectorDescriptor {
                id: "s3".to_string(),
                name: "Amazon S3".to_string(),
                description: "Read data from Amazon S3 buckets".to_string(),
                connector_type: "source".to_string(),
                capabilities: vec!["read", "schema_detection", "batch"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                required_params: vec!["bucket", "key", "region"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            },
            ConnectorDescriptor {
                id: "gcs".to_string(),
                name: "Google Cloud Storage".to_string(),
                description: "Read data from Google Cloud Storage buckets".to_string(),
                connector_type: "source".to_string(),
                capabilities: vec!["read", "schema_detection", "batch"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                required_params: vec!["bucket", "path"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            },
        ]
    }

    /// Get all built-in destination connectors
    pub fn destinations() -> Vec<ConnectorDescriptor> {
        vec![
            ConnectorDescriptor {
                id: "postgres".to_string(),
                name: "PostgreSQL".to_string(),
                description: "Write data to PostgreSQL databases".to_string(),
                connector_type: "destination".to_string(),
                capabilities: vec!["write", "batch", "schema_detection"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                required_params: vec!["host", "port", "user", "password", "database"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            },
            ConnectorDescriptor {
                id: "mysql".to_string(),
                name: "MySQL".to_string(),
                description: "Write data to MySQL databases".to_string(),
                connector_type: "destination".to_string(),
                capabilities: vec!["write", "batch", "schema_detection"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                required_params: vec!["host", "port", "user", "password", "database"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            },
            ConnectorDescriptor {
                id: "snowflake".to_string(),
                name: "Snowflake".to_string(),
                description: "Write data to Snowflake data warehouse".to_string(),
                connector_type: "destination".to_string(),
                capabilities: vec!["write", "batch", "schema_detection", "stream"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                required_params: vec!["account", "user", "password", "warehouse", "database"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            },
            ConnectorDescriptor {
                id: "bigquery".to_string(),
                name: "Google BigQuery".to_string(),
                description: "Write data to Google BigQuery".to_string(),
                connector_type: "destination".to_string(),
                capabilities: vec!["write", "batch", "schema_detection", "stream"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                required_params: vec!["project_id", "dataset"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            },
            ConnectorDescriptor {
                id: "redshift".to_string(),
                name: "Amazon Redshift".to_string(),
                description: "Write data to Amazon Redshift".to_string(),
                connector_type: "destination".to_string(),
                capabilities: vec!["write", "batch", "schema_detection"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                required_params: vec!["host", "port", "user", "password", "database"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            },
            ConnectorDescriptor {
                id: "http".to_string(),
                name: "HTTP/Webhook".to_string(),
                description: "Send data to HTTP endpoints".to_string(),
                connector_type: "destination".to_string(),
                capabilities: vec!["write", "batch", "stream"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                required_params: vec!["url"].iter().map(|s| s.to_string()).collect(),
            },
            ConnectorDescriptor {
                id: "s3".to_string(),
                name: "Amazon S3".to_string(),
                description: "Write data to Amazon S3 buckets".to_string(),
                connector_type: "destination".to_string(),
                capabilities: vec!["write", "batch"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                required_params: vec!["bucket", "key", "region"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            },
        ]
    }
}

/// Connector descriptor for discovery
#[derive(Debug, Clone)]
pub struct ConnectorDescriptor {
    /// Unique connector ID
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description of what it does
    pub description: String,
    /// Type: source or destination
    pub connector_type: String,
    /// List of capabilities (read, write, stream, batch, etc.)
    pub capabilities: Vec<String>,
    /// Required configuration parameters
    pub required_params: Vec<String>,
}

/// Connector registry for discovery and lookup
pub struct ConnectorRegistry {
    descriptors: HashMap<String, ConnectorDescriptor>,
    pool: ConnectionPool,
}

impl ConnectorRegistry {
    /// Create new registry with built-in connectors
    pub fn new() -> Self {
        let mut descriptors = HashMap::new();

        // Add source connectors
        for descriptor in BuiltInConnectors::sources() {
            let key = format!("source:{}", descriptor.id);
            descriptors.insert(key, descriptor);
        }

        // Add destination connectors
        for descriptor in BuiltInConnectors::destinations() {
            let key = format!("destination:{}", descriptor.id);
            descriptors.insert(key, descriptor);
        }

        Self {
            descriptors,
            pool: ConnectionPool::new(),
        }
    }

    /// Get connector descriptor
    pub fn get_descriptor(&self, connector_type: &str, id: &str) -> Option<ConnectorDescriptor> {
        let key = format!("{}:{}", connector_type, id);
        self.descriptors.get(&key).cloned()
    }

    /// List all available source connectors
    pub fn list_sources(&self) -> Vec<ConnectorDescriptor> {
        self.descriptors
            .values()
            .filter(|d| d.connector_type == "source")
            .cloned()
            .collect()
    }

    /// List all available destination connectors
    pub fn list_destinations(&self) -> Vec<ConnectorDescriptor> {
        self.descriptors
            .values()
            .filter(|d| d.connector_type == "destination")
            .cloned()
            .collect()
    }

    /// Search connectors by capability
    pub fn find_by_capability(&self, capability: &str) -> Vec<ConnectorDescriptor> {
        self.descriptors
            .values()
            .filter(|d| d.capabilities.contains(&capability.to_string()))
            .cloned()
            .collect()
    }

    /// Get the connection pool
    pub fn pool(&self) -> &ConnectionPool {
        &self.pool
    }

    /// Get the connection pool (mutable)
    pub fn pool_mut(&mut self) -> &mut ConnectionPool {
        &mut self.pool
    }
}

impl Default for ConnectorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_built_in_sources() {
        let sources = BuiltInConnectors::sources();
        assert!(!sources.is_empty());
        assert!(sources.iter().any(|s| s.id == "postgres"));
        assert!(sources.iter().any(|s| s.id == "api"));
        assert!(sources.iter().any(|s| s.id == "s3"));
    }

    #[test]
    fn test_built_in_destinations() {
        let dests = BuiltInConnectors::destinations();
        assert!(!dests.is_empty());
        assert!(dests.iter().any(|d| d.id == "snowflake"));
        assert!(dests.iter().any(|d| d.id == "bigquery"));
        assert!(dests.iter().any(|d| d.id == "http"));
    }

    #[test]
    fn test_registry_creation() {
        let registry = ConnectorRegistry::new();
        assert!(!registry.list_sources().is_empty());
        assert!(!registry.list_destinations().is_empty());
    }

    #[test]
    fn test_registry_lookup() {
        let registry = ConnectorRegistry::new();

        let postgres_source = registry.get_descriptor("source", "postgres");
        assert!(postgres_source.is_some());
        assert_eq!(postgres_source.unwrap().name, "PostgreSQL");

        let snowflake_dest = registry.get_descriptor("destination", "snowflake");
        assert!(snowflake_dest.is_some());
        assert_eq!(snowflake_dest.unwrap().name, "Snowflake");
    }

    #[test]
    fn test_find_by_capability() {
        let registry = ConnectorRegistry::new();

        let stream_capable = registry.find_by_capability("stream");
        assert!(!stream_capable.is_empty());
        assert!(stream_capable.iter().any(|c| c.id == "api" || c.id == "snowflake"));
    }
}
