/// Connector Configuration and Connection Pool Management

use super::{SourceConnector, DestinationConnector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Unified connector configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorConfig {
    /// Unique connector ID
    pub id: String,
    /// Connector type: source or destination
    pub connector_type: ConnectorType,
    /// Implementation: postgres, mysql, s3, api, etc.
    pub implementation: String,
    /// Connection parameters
    pub params: HashMap<String, String>,
    /// Optional tags for organization
    pub tags: Vec<String>,
    /// Whether this is the default connector
    pub is_default: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConnectorType {
    #[serde(rename = "source")]
    Source,
    #[serde(rename = "destination")]
    Destination,
}

impl ConnectorConfig {
    /// Create new source connector config
    pub fn source(id: &str, implementation: &str) -> Self {
        Self {
            id: id.to_string(),
            connector_type: ConnectorType::Source,
            implementation: implementation.to_string(),
            params: Default::default(),
            tags: vec![],
            is_default: false,
        }
    }

    /// Create new destination connector config
    pub fn destination(id: &str, implementation: &str) -> Self {
        Self {
            id: id.to_string(),
            connector_type: ConnectorType::Destination,
            implementation: implementation.to_string(),
            params: Default::default(),
            tags: vec![],
            is_default: false,
        }
    }

    /// Add parameter
    pub fn with_param(mut self, key: &str, value: &str) -> Self {
        self.params.insert(key.to_string(), value.to_string());
        self
    }

    /// Add tag
    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }

    /// Mark as default
    pub fn as_default(mut self) -> Self {
        self.is_default = true;
        self
    }
}

/// Connection pool for managing connector instances
pub struct ConnectionPool {
    configs: Arc<std::sync::Mutex<HashMap<String, ConnectorConfig>>>,
    source_instances: Arc<std::sync::Mutex<HashMap<String, Arc<dyn std::any::Any + Send + Sync>>>>,
    destination_instances: Arc<std::sync::Mutex<HashMap<String, Arc<dyn std::any::Any + Send + Sync>>>>,
}

impl ConnectionPool {
    /// Create new connection pool
    pub fn new() -> Self {
        Self {
            configs: Arc::new(std::sync::Mutex::new(HashMap::new())),
            source_instances: Arc::new(std::sync::Mutex::new(HashMap::new())),
            destination_instances: Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }

    /// Register connector configuration
    pub fn register(&self, config: ConnectorConfig) -> crate::Result<()> {
        let mut configs = self.configs.lock().map_err(|e| {
            crate::Error::Internal(format!("Failed to acquire config lock: {}", e))
        })?;
        configs.insert(config.id.clone(), config);
        Ok(())
    }

    /// Get connector configuration
    pub fn get_config(&self, id: &str) -> crate::Result<Option<ConnectorConfig>> {
        let configs = self.configs.lock().map_err(|e| {
            crate::Error::Internal(format!("Failed to acquire config lock: {}", e))
        })?;
        Ok(configs.get(id).cloned())
    }

    /// List all configurations
    pub fn list_configs(&self, connector_type: Option<ConnectorType>) -> crate::Result<Vec<ConnectorConfig>> {
        let configs = self.configs.lock().map_err(|e| {
            crate::Error::Internal(format!("Failed to acquire config lock: {}", e))
        })?;

        let result = configs
            .values()
            .filter(|c| connector_type.is_none() || c.connector_type == connector_type.unwrap())
            .cloned()
            .collect();

        Ok(result)
    }

    /// Find configs by tag
    pub fn find_by_tag(&self, tag: &str) -> crate::Result<Vec<ConnectorConfig>> {
        let configs = self.configs.lock().map_err(|e| {
            crate::Error::Internal(format!("Failed to acquire config lock: {}", e))
        })?;

        let result = configs
            .values()
            .filter(|c| c.tags.contains(&tag.to_string()))
            .cloned()
            .collect();

        Ok(result)
    }

    /// Get default source connector
    pub fn get_default_source(&self) -> crate::Result<Option<ConnectorConfig>> {
        let configs = self.configs.lock().map_err(|e| {
            crate::Error::Internal(format!("Failed to acquire config lock: {}", e))
        })?;

        Ok(configs
            .values()
            .find(|c| c.is_default && c.connector_type == ConnectorType::Source)
            .cloned())
    }

    /// Get default destination connector
    pub fn get_default_destination(&self) -> crate::Result<Option<ConnectorConfig>> {
        let configs = self.configs.lock().map_err(|e| {
            crate::Error::Internal(format!("Failed to acquire config lock: {}", e))
        })?;

        Ok(configs
            .values()
            .find(|c| c.is_default && c.connector_type == ConnectorType::Destination)
            .cloned())
    }
}

impl Default for ConnectionPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connector_config_source() {
        let config = ConnectorConfig::source("pg_prod", "postgres")
            .with_param("host", "db.example.com")
            .with_param("database", "analytics")
            .with_tag("production")
            .with_tag("replicated")
            .as_default();

        assert_eq!(config.id, "pg_prod");
        assert_eq!(config.connector_type, ConnectorType::Source);
        assert_eq!(config.implementation, "postgres");
        assert!(config.is_default);
        assert_eq!(config.tags.len(), 2);
        assert_eq!(config.params.get("host").unwrap(), "db.example.com");
    }

    #[test]
    fn test_connector_config_destination() {
        let config = ConnectorConfig::destination("bq_prod", "bigquery")
            .with_param("project_id", "my-project")
            .with_param("dataset", "analytics");

        assert_eq!(config.id, "bq_prod");
        assert_eq!(config.connector_type, ConnectorType::Destination);
        assert_eq!(config.implementation, "bigquery");
        assert!(!config.is_default);
    }

    #[test]
    fn test_connection_pool() -> crate::Result<()> {
        let pool = ConnectionPool::new();

        let source_config = ConnectorConfig::source("pg1", "postgres")
            .with_tag("prod")
            .as_default();
        pool.register(source_config.clone())?;

        let dest_config = ConnectorConfig::destination("bq1", "bigquery")
            .with_tag("prod");
        pool.register(dest_config)?;

        // Get specific config
        let retrieved = pool.get_config("pg1")?;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "pg1");

        // List all
        let all = pool.list_configs(None)?;
        assert_eq!(all.len(), 2);

        // List by type
        let sources = pool.list_configs(Some(ConnectorType::Source))?;
        assert_eq!(sources.len(), 1);

        // Find by tag
        let prod_configs = pool.find_by_tag("prod")?;
        assert_eq!(prod_configs.len(), 2);

        // Get default
        let default = pool.get_default_source()?;
        assert!(default.is_some());
        assert_eq!(default.unwrap().id, "pg1");

        Ok(())
    }
}
