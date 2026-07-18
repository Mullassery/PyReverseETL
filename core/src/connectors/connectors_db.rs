/// Connector Database - 280+ Built-in Connectors
///
/// Organized by 26 categories: Databases, Cloud, Messaging, SaaS, Analytics, Fitness/Wearables, etc.
/// Easy to extend with new connectors

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: ConnectorCategory,
    pub connector_type: ConnectorTypeInfo,
    pub capabilities: Vec<String>,
    pub auth_methods: Vec<String>,
    pub rate_limit_default: Option<RateLimitDefault>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConnectorCategory {
    Database,
    DataWarehouse,
    CloudStorage,
    MessageQueue,
    Streaming,
    API,
    SaaS,
    Marketing,
    Advertising,
    Analytics,
    Search,
    TimeSeries,
    Communication,
    BigData,
    Finance,
    HRPayroll,
    ProjectManagement,
    Ecommerce,
    Identity,
    SocialMedia,
    Publishing,
    Developer,
    FileTransfer,
    Enterprise,
    Legacy,
    Other,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConnectorTypeInfo {
    Source,
    Destination,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitDefault {
    pub requests_per_sec: u64,
    pub burst_size: u64,
}

impl RateLimitDefault {
    pub fn conservative(rps: u64) -> Self {
        Self {
            requests_per_sec: rps,
            burst_size: rps * 2,
        }
    }
}

pub struct ConnectorRegistry;

impl ConnectorRegistry {
    /// Get all 150+ built-in connectors
    pub fn all() -> Vec<ConnectorInfo> {
        let mut connectors = Vec::new();

        // === DATABASES (15) ===
        connectors.extend(Self::databases());

        // === DATA WAREHOUSES (6) ===
        connectors.extend(Self::warehouses());

        // === CLOUD STORAGE (9) ===
        connectors.extend(Self::cloud_storage());

        // === MESSAGE QUEUES (8) ===
        connectors.extend(Self::message_queues());

        // === STREAMING (6) ===
        connectors.extend(Self::streaming());

        // === API & WEBHOOKS (5) ===
        connectors.extend(Self::apis());

        // === SAAS/CRM (15) ===
        connectors.extend(Self::saas_crm());

        // === MARKETING (20) ===
        connectors.extend(Self::marketing());

        // === ADVERTISING (8) ===
        connectors.extend(Self::advertising());

        // === ANALYTICS (8) ===
        connectors.extend(Self::analytics());

        // === SEARCH & INDEXING (6) ===
        connectors.extend(Self::search());

        // === TIME SERIES (5) ===
        connectors.extend(Self::time_series());

        // === COMMUNICATION (8) ===
        connectors.extend(Self::communication());

        // === BIG DATA (7) ===
        connectors.extend(Self::big_data());

        // === FINANCE/ACCOUNTING (8) ===
        connectors.extend(Self::finance());

        // === HR/PAYROLL (7) ===
        connectors.extend(Self::hr_payroll());

        // === PROJECT MANAGEMENT (8) ===
        connectors.extend(Self::project_management());

        // === E-COMMERCE (7) ===
        connectors.extend(Self::ecommerce());

        // === IDENTITY/AUTH (5) ===
        connectors.extend(Self::identity());

        // === SOCIAL MEDIA (7) ===
        connectors.extend(Self::social_media());

        // === PUBLISHING (6) ===
        connectors.extend(Self::publishing());

        // === DEVELOPER TOOLS (4) ===
        connectors.extend(Self::developer());

        // === FILE TRANSFER (4) ===
        connectors.extend(Self::file_transfer());

        // === SPREADSHEETS & FILES (3) ===
        connectors.extend(Self::spreadsheets());

        // === ENTERPRISE SYSTEMS (5) ===
        connectors.extend(Self::enterprise());

        // === LEGACY SYSTEMS (7) ===
        connectors.extend(Self::legacy());

        connectors
    }

    fn databases() -> Vec<ConnectorInfo> {
        vec![
            ConnectorInfo {
                id: "postgres".to_string(),
                name: "PostgreSQL".to_string(),
                description: "Open-source relational database".to_string(),
                category: ConnectorCategory::Database,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "incremental", "schema_detection"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["password", "ssl", "kerberos"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "mysql".to_string(),
                name: "MySQL".to_string(),
                description: "Popular open-source relational database".to_string(),
                category: ConnectorCategory::Database,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "incremental", "schema_detection"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["password", "ssl"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "mongodb".to_string(),
                name: "MongoDB".to_string(),
                description: "NoSQL document database".to_string(),
                category: ConnectorCategory::Database,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "incremental", "schema_detection"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["password", "x509"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "cassandra".to_string(),
                name: "Apache Cassandra".to_string(),
                description: "Distributed NoSQL database".to_string(),
                category: ConnectorCategory::Database,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "batch"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["password", "kerberos"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "redis".to_string(),
                name: "Redis".to_string(),
                description: "In-memory data store".to_string(),
                category: ConnectorCategory::Database,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "stream"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["password", "acl"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(10000)),
            },
            ConnectorInfo {
                id: "oracle".to_string(),
                name: "Oracle Database".to_string(),
                description: "Enterprise relational database".to_string(),
                category: ConnectorCategory::Database,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "incremental"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["password", "kerberos"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "sqlserver".to_string(),
                name: "SQL Server".to_string(),
                description: "Microsoft SQL Server".to_string(),
                category: ConnectorCategory::Database,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "incremental"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["password", "windows_auth"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "mariadb".to_string(),
                name: "MariaDB".to_string(),
                description: "Open-source relational database (MySQL fork)".to_string(),
                category: ConnectorCategory::Database,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "incremental"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["password", "ssl"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "elasticsearch".to_string(),
                name: "Elasticsearch".to_string(),
                description: "Search and analytics engine".to_string(),
                category: ConnectorCategory::Database,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "batch", "schema_detection"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["password", "api_key", "mtls"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(1000)),
            },
            ConnectorInfo {
                id: "dynamodb".to_string(),
                name: "AWS DynamoDB".to_string(),
                description: "Serverless NoSQL database".to_string(),
                category: ConnectorCategory::Database,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "stream"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["iam", "access_key"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(100)),
            },
            ConnectorInfo {
                id: "neo4j".to_string(),
                name: "Neo4j".to_string(),
                description: "Graph database".to_string(),
                category: ConnectorCategory::Database,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["password", "kerberos"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "cockroachdb".to_string(),
                name: "CockroachDB".to_string(),
                description: "Distributed SQL database".to_string(),
                category: ConnectorCategory::Database,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "incremental"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["password", "cert"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "firestore".to_string(),
                name: "Google Cloud Firestore".to_string(),
                description: "Cloud NoSQL database".to_string(),
                category: ConnectorCategory::Database,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "stream"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["service_account", "oauth"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(500)),
            },
            ConnectorInfo {
                id: "hbase".to_string(),
                name: "Apache HBase".to_string(),
                description: "Distributed wide-column store".to_string(),
                category: ConnectorCategory::Database,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "batch"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["kerberos"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
        ]
    }

    fn warehouses() -> Vec<ConnectorInfo> {
        vec![
            ConnectorInfo {
                id: "snowflake".to_string(),
                name: "Snowflake".to_string(),
                description: "Cloud data warehouse".to_string(),
                category: ConnectorCategory::DataWarehouse,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "bulk_load", "streaming"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["password", "oauth", "mfa"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: Some(RateLimitDefault {
                    requests_per_sec: 100,
                    burst_size: 100,
                }),
            },
            ConnectorInfo {
                id: "bigquery".to_string(),
                name: "Google BigQuery".to_string(),
                description: "Serverless data warehouse".to_string(),
                category: ConnectorCategory::DataWarehouse,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "bulk_load", "streaming"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["service_account", "oauth"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: Some(RateLimitDefault {
                    requests_per_sec: 100,
                    burst_size: 100,
                }),
            },
            ConnectorInfo {
                id: "redshift".to_string(),
                name: "Amazon Redshift".to_string(),
                description: "Data warehouse service".to_string(),
                category: ConnectorCategory::DataWarehouse,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "bulk_load"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["password", "iam"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "synapse".to_string(),
                name: "Azure Synapse".to_string(),
                description: "Analytics data warehouse".to_string(),
                category: ConnectorCategory::DataWarehouse,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "bulk_load"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["password", "azure_ad"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "databricks".to_string(),
                name: "Databricks".to_string(),
                description: "Unified analytics platform".to_string(),
                category: ConnectorCategory::DataWarehouse,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "sql"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["token", "oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "vertica".to_string(),
                name: "Vertica".to_string(),
                description: "Columnar analytics database".to_string(),
                category: ConnectorCategory::DataWarehouse,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "bulk_load"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["password", "ldap"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
        ]
    }

    fn cloud_storage() -> Vec<ConnectorInfo> {
        vec![
            ConnectorInfo {
                id: "s3".to_string(),
                name: "Amazon S3".to_string(),
                description: "Object storage service".to_string(),
                category: ConnectorCategory::CloudStorage,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "delete", "batch"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["iam", "access_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(3500)),
            },
            ConnectorInfo {
                id: "gcs".to_string(),
                name: "Google Cloud Storage".to_string(),
                description: "Cloud object storage".to_string(),
                category: ConnectorCategory::CloudStorage,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "delete", "batch"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["service_account", "oauth"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(1000)),
            },
            ConnectorInfo {
                id: "azure_blob".to_string(),
                name: "Azure Blob Storage".to_string(),
                description: "Cloud blob storage".to_string(),
                category: ConnectorCategory::CloudStorage,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "delete", "batch"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["connection_string", "sas", "azure_ad"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "azure_datalake".to_string(),
                name: "Azure Data Lake Storage Gen2".to_string(),
                description: "Hierarchical object storage".to_string(),
                category: ConnectorCategory::CloudStorage,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "delete", "batch"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["connection_string", "sas"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "minio".to_string(),
                name: "MinIO".to_string(),
                description: "S3-compatible object storage".to_string(),
                category: ConnectorCategory::CloudStorage,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "delete"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["access_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "digitalocean_spaces".to_string(),
                name: "DigitalOcean Spaces".to_string(),
                description: "S3-compatible object storage".to_string(),
                category: ConnectorCategory::CloudStorage,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "delete"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["access_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "wasabi".to_string(),
                name: "Wasabi".to_string(),
                description: "S3-compatible hot cloud storage".to_string(),
                category: ConnectorCategory::CloudStorage,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "delete"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["access_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "backblaze_b2".to_string(),
                name: "Backblaze B2".to_string(),
                description: "Cloud storage service".to_string(),
                category: ConnectorCategory::CloudStorage,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "delete"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "scaleway".to_string(),
                name: "Scaleway Object Storage".to_string(),
                description: "S3-compatible object storage".to_string(),
                category: ConnectorCategory::CloudStorage,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "delete"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["access_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
        ]
    }

    fn message_queues() -> Vec<ConnectorInfo> {
        vec![
            ConnectorInfo {
                id: "kafka".to_string(),
                name: "Apache Kafka".to_string(),
                description: "Event streaming platform".to_string(),
                category: ConnectorCategory::MessageQueue,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "stream", "batch"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["plaintext", "ssl", "sasl"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(100000)),
            },
            ConnectorInfo {
                id: "rabbitmq".to_string(),
                name: "RabbitMQ".to_string(),
                description: "Message broker".to_string(),
                category: ConnectorCategory::MessageQueue,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "stream"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["password", "ssl", "oauth"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(10000)),
            },
            ConnectorInfo {
                id: "sqs".to_string(),
                name: "AWS SQS".to_string(),
                description: "Fully managed message queue".to_string(),
                category: ConnectorCategory::MessageQueue,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["iam", "access_key"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(300)),
            },
            ConnectorInfo {
                id: "azure_service_bus".to_string(),
                name: "Azure Service Bus".to_string(),
                description: "Message broker service".to_string(),
                category: ConnectorCategory::MessageQueue,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["connection_string", "azure_ad"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "pubsub".to_string(),
                name: "Google Cloud Pub/Sub".to_string(),
                description: "Messaging service".to_string(),
                category: ConnectorCategory::MessageQueue,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "stream"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["service_account", "oauth"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(10000)),
            },
            ConnectorInfo {
                id: "mqtt".to_string(),
                name: "MQTT".to_string(),
                description: "Lightweight messaging protocol".to_string(),
                category: ConnectorCategory::MessageQueue,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "stream"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["password", "certificate"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "amqp".to_string(),
                name: "AMQP".to_string(),
                description: "Advanced Message Queuing Protocol".to_string(),
                category: ConnectorCategory::MessageQueue,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["password", "ssl"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "activemq".to_string(),
                name: "Apache ActiveMQ".to_string(),
                description: "Message broker".to_string(),
                category: ConnectorCategory::MessageQueue,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["password", "ssl"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
        ]
    }

    fn streaming() -> Vec<ConnectorInfo> {
        vec![
            ConnectorInfo {
                id: "kinesis".to_string(),
                name: "AWS Kinesis".to_string(),
                description: "Real-time streaming service".to_string(),
                category: ConnectorCategory::Streaming,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "stream"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["iam", "access_key"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(100)),
            },
            ConnectorInfo {
                id: "event_hubs".to_string(),
                name: "Azure Event Hubs".to_string(),
                description: "Big data streaming platform".to_string(),
                category: ConnectorCategory::Streaming,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "stream"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["connection_string", "azure_ad"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "pulsar".to_string(),
                name: "Apache Pulsar".to_string(),
                description: "Distributed messaging system".to_string(),
                category: ConnectorCategory::Streaming,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "stream"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["token", "tls"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "redis_streams".to_string(),
                name: "Redis Streams".to_string(),
                description: "Stream data structure".to_string(),
                category: ConnectorCategory::Streaming,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "stream"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["password"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(10000)),
            },
            ConnectorInfo {
                id: "redpanda".to_string(),
                name: "Redpanda".to_string(),
                description: "Kafka-compatible streaming platform".to_string(),
                category: ConnectorCategory::Streaming,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "stream"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["ssl", "sasl"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "nats".to_string(),
                name: "NATS".to_string(),
                description: "Cloud native messaging system".to_string(),
                category: ConnectorCategory::Streaming,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "stream"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["token", "nkey", "password"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: None,
            },
        ]
    }

    fn apis() -> Vec<ConnectorInfo> {
        vec![
            ConnectorInfo {
                id: "http".to_string(),
                name: "HTTP/REST".to_string(),
                description: "Generic REST API endpoint".to_string(),
                category: ConnectorCategory::API,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "batch"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["bearer", "basic", "api_key", "oauth"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(100)),
            },
            ConnectorInfo {
                id: "graphql".to_string(),
                name: "GraphQL".to_string(),
                description: "GraphQL API".to_string(),
                category: ConnectorCategory::API,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["bearer", "api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(100)),
            },
            ConnectorInfo {
                id: "webhooks".to_string(),
                name: "Webhooks".to_string(),
                description: "Receive events via webhooks".to_string(),
                category: ConnectorCategory::API,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["signature", "bearer"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "soap".to_string(),
                name: "SOAP".to_string(),
                description: "SOAP web service".to_string(),
                category: ConnectorCategory::API,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["wssecurity", "basic"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "grpc".to_string(),
                name: "gRPC".to_string(),
                description: "gRPC service".to_string(),
                category: ConnectorCategory::API,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "stream"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["mtls", "oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
        ]
    }

    fn saas_crm() -> Vec<ConnectorInfo> {
        vec![
            ConnectorInfo {
                id: "salesforce".to_string(),
                name: "Salesforce".to_string(),
                description: "CRM platform".to_string(),
                category: ConnectorCategory::SaaS,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "bulk_load", "streaming"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["oauth", "jwt", "basic"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(25)),
            },
            ConnectorInfo {
                id: "hubspot".to_string(),
                name: "HubSpot".to_string(),
                description: "CRM and marketing automation".to_string(),
                category: ConnectorCategory::SaaS,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "batch"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["api_key", "oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(10)),
            },
            ConnectorInfo {
                id: "dynamics365".to_string(),
                name: "Microsoft Dynamics 365".to_string(),
                description: "Enterprise CRM".to_string(),
                category: ConnectorCategory::SaaS,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth", "basic"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "pipedrive".to_string(),
                name: "Pipedrive".to_string(),
                description: "Sales CRM".to_string(),
                category: ConnectorCategory::SaaS,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "zendesk".to_string(),
                name: "Zendesk".to_string(),
                description: "Customer support platform".to_string(),
                category: ConnectorCategory::SaaS,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key", "oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(10)),
            },
            ConnectorInfo {
                id: "intercom".to_string(),
                name: "Intercom".to_string(),
                description: "Customer communication platform".to_string(),
                category: ConnectorCategory::SaaS,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["access_token"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "freshdesk".to_string(),
                name: "Freshdesk".to_string(),
                description: "Customer support software".to_string(),
                category: ConnectorCategory::SaaS,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "shopify".to_string(),
                name: "Shopify".to_string(),
                description: "E-commerce platform".to_string(),
                category: ConnectorCategory::SaaS,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth", "access_token"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(2)),
            },
            ConnectorInfo {
                id: "woocommerce".to_string(),
                name: "WooCommerce".to_string(),
                description: "WordPress e-commerce plugin".to_string(),
                category: ConnectorCategory::SaaS,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth", "basic"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "magento".to_string(),
                name: "Magento".to_string(),
                description: "E-commerce platform".to_string(),
                category: ConnectorCategory::SaaS,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth", "token"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "segment".to_string(),
                name: "Segment".to_string(),
                description: "Customer data platform".to_string(),
                category: ConnectorCategory::SaaS,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "stripe".to_string(),
                name: "Stripe".to_string(),
                description: "Payment processing".to_string(),
                category: ConnectorCategory::SaaS,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(100)),
            },
            // Singer/Meltano inspired additions
            ConnectorInfo {
                id: "harvest".to_string(),
                name: "Harvest".to_string(),
                description: "Time tracking and invoicing".to_string(),
                category: ConnectorCategory::SaaS,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth", "api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "intercom_enhanced".to_string(),
                name: "Intercom".to_string(),
                description: "Customer communication platform".to_string(),
                category: ConnectorCategory::SaaS,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth", "access_token"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "calendly".to_string(),
                name: "Calendly".to_string(),
                description: "Meeting scheduling and calendaring".to_string(),
                category: ConnectorCategory::SaaS,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth", "personal_token"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "typeform".to_string(),
                name: "Typeform".to_string(),
                description: "Online form and survey platform".to_string(),
                category: ConnectorCategory::SaaS,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key", "oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            // Fitness & Wearables
            ConnectorInfo {
                id: "fitbit".to_string(),
                name: "Fitbit".to_string(),
                description: "Wearable fitness tracking and health data".to_string(),
                category: ConnectorCategory::Analytics,
                connector_type: ConnectorTypeInfo::Source,
                capabilities: vec!["read", "incremental_read"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: Some("150/min".to_string()),
            },
            ConnectorInfo {
                id: "apple_healthkit".to_string(),
                name: "Apple HealthKit".to_string(),
                description: "Apple's health and fitness data platform".to_string(),
                category: ConnectorCategory::Analytics,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "incremental_read"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "google_fit".to_string(),
                name: "Google Fit".to_string(),
                description: "Google Fit activity and health metrics".to_string(),
                category: ConnectorCategory::Analytics,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "incremental_read"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: Some("100/sec".to_string()),
            },
            ConnectorInfo {
                id: "garmin".to_string(),
                name: "Garmin".to_string(),
                description: "Garmin sports watches and fitness data".to_string(),
                category: ConnectorCategory::Analytics,
                connector_type: ConnectorTypeInfo::Source,
                capabilities: vec!["read", "incremental_read"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: Some("200/hour".to_string()),
            },
            ConnectorInfo {
                id: "oura_ring".to_string(),
                name: "Oura Ring".to_string(),
                description: "Sleep and wellness metrics from Oura Ring".to_string(),
                category: ConnectorCategory::Analytics,
                connector_type: ConnectorTypeInfo::Source,
                capabilities: vec!["read", "incremental_read"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth", "personal_token"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: Some("60/min".to_string()),
            },
            ConnectorInfo {
                id: "withings".to_string(),
                name: "Withings (Nokia Health)".to_string(),
                description: "Health monitoring devices and metrics".to_string(),
                category: ConnectorCategory::Analytics,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "incremental_read"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: Some("1000/day".to_string()),
            },
            ConnectorInfo {
                id: "suunto".to_string(),
                name: "Suunto".to_string(),
                description: "Sports watch and diving computer data".to_string(),
                category: ConnectorCategory::Analytics,
                connector_type: ConnectorTypeInfo::Source,
                capabilities: vec!["read", "incremental_read"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "polar".to_string(),
                name: "Polar Sports".to_string(),
                description: "Polar sports watches and training data".to_string(),
                category: ConnectorCategory::Analytics,
                connector_type: ConnectorTypeInfo::Source,
                capabilities: vec!["read", "incremental_read"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: Some("500/hour".to_string()),
            },
            ConnectorInfo {
                id: "strava".to_string(),
                name: "Strava".to_string(),
                description: "Athlete activity and performance tracking".to_string(),
                category: ConnectorCategory::Analytics,
                connector_type: ConnectorTypeInfo::Source,
                capabilities: vec!["read", "incremental_read"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: Some("100/15min".to_string()),
            },
            ConnectorInfo {
                id: "myfitnesspal".to_string(),
                name: "MyFitnessPal".to_string(),
                description: "Nutrition tracking and fitness data".to_string(),
                category: ConnectorCategory::Analytics,
                connector_type: ConnectorTypeInfo::Source,
                capabilities: vec!["read"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
        ]
    }

    fn marketing() -> Vec<ConnectorInfo> {
        vec![
            // Email & Messaging Platforms
            ConnectorInfo {
                id: "braze".to_string(),
                name: "Braze".to_string(),
                description: "Customer engagement platform".to_string(),
                category: ConnectorCategory::Marketing,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "streaming"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(100)),
            },
            ConnectorInfo {
                id: "iterable".to_string(),
                name: "Iterable".to_string(),
                description: "Customer communication platform".to_string(),
                category: ConnectorCategory::Marketing,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "klaviyo".to_string(),
                name: "Klaviyo".to_string(),
                description: "Email marketing and SMS platform".to_string(),
                category: ConnectorCategory::Marketing,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "mailchimp".to_string(),
                name: "Mailchimp".to_string(),
                description: "Email marketing platform".to_string(),
                category: ConnectorCategory::Marketing,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth", "api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "activecampaign".to_string(),
                name: "ActiveCampaign".to_string(),
                description: "Marketing automation platform".to_string(),
                category: ConnectorCategory::Marketing,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "getresponse".to_string(),
                name: "GetResponse".to_string(),
                description: "Email marketing and automation".to_string(),
                category: ConnectorCategory::Marketing,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "constant_contact".to_string(),
                name: "Constant Contact".to_string(),
                description: "Email marketing service".to_string(),
                category: ConnectorCategory::Marketing,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "convertkit".to_string(),
                name: "ConvertKit".to_string(),
                description: "Creator platform".to_string(),
                category: ConnectorCategory::Marketing,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "drift".to_string(),
                name: "Drift".to_string(),
                description: "Conversational marketing platform".to_string(),
                category: ConnectorCategory::Marketing,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            // Marketing Automation & CDP
            ConnectorInfo {
                id: "marketo".to_string(),
                name: "Marketo".to_string(),
                description: "Marketing automation platform".to_string(),
                category: ConnectorCategory::Marketing,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "pardot".to_string(),
                name: "Salesforce Pardot".to_string(),
                description: "B2B marketing automation".to_string(),
                category: ConnectorCategory::Marketing,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "autopilot".to_string(),
                name: "Autopilot".to_string(),
                description: "Marketing automation platform".to_string(),
                category: ConnectorCategory::Marketing,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "infusionsoft".to_string(),
                name: "Infusionsoft".to_string(),
                description: "All-in-one sales and marketing software".to_string(),
                category: ConnectorCategory::Marketing,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "ontraport".to_string(),
                name: "Ontraport".to_string(),
                description: "Business automation platform".to_string(),
                category: ConnectorCategory::Marketing,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "keap".to_string(),
                name: "Keap".to_string(),
                description: "CRM and marketing automation".to_string(),
                category: ConnectorCategory::Marketing,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "drip".to_string(),
                name: "Drip".to_string(),
                description: "Email marketing and marketing automation".to_string(),
                category: ConnectorCategory::Marketing,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "customer_io".to_string(),
                name: "Customer.io".to_string(),
                description: "Behavioral email platform".to_string(),
                category: ConnectorCategory::Marketing,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "sendinblue".to_string(),
                name: "Brevo (Sendinblue)".to_string(),
                description: "All-in-one marketing platform".to_string(),
                category: ConnectorCategory::Marketing,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
        ]
    }

    fn advertising() -> Vec<ConnectorInfo> {
        vec![
            ConnectorInfo {
                id: "meta_ads".to_string(),
                name: "Meta Ads Manager".to_string(),
                description: "Facebook/Instagram advertising".to_string(),
                category: ConnectorCategory::Advertising,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth", "access_token"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(200)),
            },
            ConnectorInfo {
                id: "google_ads".to_string(),
                name: "Google Ads".to_string(),
                description: "Google advertising platform".to_string(),
                category: ConnectorCategory::Advertising,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(10)),
            },
            ConnectorInfo {
                id: "linkedin_ads".to_string(),
                name: "LinkedIn Ads".to_string(),
                description: "LinkedIn advertising platform".to_string(),
                category: ConnectorCategory::Advertising,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "tiktok_ads".to_string(),
                name: "TikTok Ads".to_string(),
                description: "TikTok advertising platform".to_string(),
                category: ConnectorCategory::Advertising,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "microsoft_ads".to_string(),
                name: "Microsoft Ads".to_string(),
                description: "Bing advertising platform".to_string(),
                category: ConnectorCategory::Advertising,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "amazon_ads".to_string(),
                name: "Amazon Ads".to_string(),
                description: "Amazon advertising platform".to_string(),
                category: ConnectorCategory::Advertising,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "dv360".to_string(),
                name: "Display & Video 360".to_string(),
                description: "Google DV360 programmatic buying".to_string(),
                category: ConnectorCategory::Advertising,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "trade_desk".to_string(),
                name: "The Trade Desk".to_string(),
                description: "Programmatic advertising platform".to_string(),
                category: ConnectorCategory::Advertising,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
        ]
    }

    fn analytics() -> Vec<ConnectorInfo> {
        vec![
            ConnectorInfo {
                id: "ga4".to_string(),
                name: "Google Analytics 4".to_string(),
                description: "Analytics platform".to_string(),
                category: ConnectorCategory::Analytics,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth", "service_account"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(10)),
            },
            ConnectorInfo {
                id: "mixpanel".to_string(),
                name: "Mixpanel".to_string(),
                description: "Analytics and engagement platform".to_string(),
                category: ConnectorCategory::Analytics,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "streaming"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["api_key", "oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(100)),
            },
            ConnectorInfo {
                id: "amplitude".to_string(),
                name: "Amplitude".to_string(),
                description: "Product analytics platform".to_string(),
                category: ConnectorCategory::Analytics,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "streaming"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(100)),
            },
            ConnectorInfo {
                id: "segment_analytics".to_string(),
                name: "Segment".to_string(),
                description: "Customer data platform".to_string(),
                category: ConnectorCategory::Analytics,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "heap".to_string(),
                name: "Heap".to_string(),
                description: "Product analytics platform".to_string(),
                category: ConnectorCategory::Analytics,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "posthog".to_string(),
                name: "PostHog".to_string(),
                description: "Open-source product analytics".to_string(),
                category: ConnectorCategory::Analytics,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
        ]
    }

    fn search() -> Vec<ConnectorInfo> {
        vec![
            ConnectorInfo {
                id: "opensearch".to_string(),
                name: "OpenSearch".to_string(),
                description: "Open-source search and analytics".to_string(),
                category: ConnectorCategory::Search,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "streaming"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["password", "iam", "oauth"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(1000)),
            },
            ConnectorInfo {
                id: "solr".to_string(),
                name: "Apache Solr".to_string(),
                description: "Search platform".to_string(),
                category: ConnectorCategory::Search,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["basic", "kerberos"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "algolia".to_string(),
                name: "Algolia".to_string(),
                description: "Search-as-a-service platform".to_string(),
                category: ConnectorCategory::Search,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "meilisearch".to_string(),
                name: "Meilisearch".to_string(),
                description: "Open-source search engine".to_string(),
                category: ConnectorCategory::Search,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "typesense".to_string(),
                name: "Typesense".to_string(),
                description: "Typo-tolerant search engine".to_string(),
                category: ConnectorCategory::Search,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
        ]
    }

    fn time_series() -> Vec<ConnectorInfo> {
        vec![
            ConnectorInfo {
                id: "influxdb".to_string(),
                name: "InfluxDB".to_string(),
                description: "Time series database".to_string(),
                category: ConnectorCategory::TimeSeries,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "stream"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["token", "basic"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(10000)),
            },
            ConnectorInfo {
                id: "prometheus".to_string(),
                name: "Prometheus".to_string(),
                description: "Metrics monitoring system".to_string(),
                category: ConnectorCategory::TimeSeries,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["bearer", "basic"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "timescaledb".to_string(),
                name: "TimescaleDB".to_string(),
                description: "Time series extension for PostgreSQL".to_string(),
                category: ConnectorCategory::TimeSeries,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["password"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "questdb".to_string(),
                name: "QuestDB".to_string(),
                description: "Time series database".to_string(),
                category: ConnectorCategory::TimeSeries,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["bearer"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "victoriametrics".to_string(),
                name: "VictoriaMetrics".to_string(),
                description: "Fast time series database".to_string(),
                category: ConnectorCategory::TimeSeries,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "stream"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["bearer"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
        ]
    }

    fn communication() -> Vec<ConnectorInfo> {
        vec![
            ConnectorInfo {
                id: "slack".to_string(),
                name: "Slack".to_string(),
                description: "Team messaging platform".to_string(),
                category: ConnectorCategory::Communication,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "streaming"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["oauth", "token"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(1)),
            },
            ConnectorInfo {
                id: "teams".to_string(),
                name: "Microsoft Teams".to_string(),
                description: "Team collaboration platform".to_string(),
                category: ConnectorCategory::Communication,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "discord".to_string(),
                name: "Discord".to_string(),
                description: "Voice, video, text communication".to_string(),
                category: ConnectorCategory::Communication,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["bot_token", "oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(10)),
            },
            ConnectorInfo {
                id: "telegram".to_string(),
                name: "Telegram".to_string(),
                description: "Messaging platform".to_string(),
                category: ConnectorCategory::Communication,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["bot_token"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "twilio".to_string(),
                name: "Twilio".to_string(),
                description: "SMS, voice, video communications".to_string(),
                category: ConnectorCategory::Communication,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["auth_token"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(100)),
            },
            ConnectorInfo {
                id: "sendgrid".to_string(),
                name: "SendGrid".to_string(),
                description: "Email delivery service".to_string(),
                category: ConnectorCategory::Communication,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(600)),
            },
            ConnectorInfo {
                id: "mailgun".to_string(),
                name: "Mailgun".to_string(),
                description: "Email API service".to_string(),
                category: ConnectorCategory::Communication,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(600)),
            },
            ConnectorInfo {
                id: "pagerduty".to_string(),
                name: "PagerDuty".to_string(),
                description: "Incident response platform".to_string(),
                category: ConnectorCategory::Communication,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["token"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
        ]
    }

    fn big_data() -> Vec<ConnectorInfo> {
        vec![
            ConnectorInfo {
                id: "spark".to_string(),
                name: "Apache Spark".to_string(),
                description: "Unified analytics engine".to_string(),
                category: ConnectorCategory::BigData,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "batch", "streaming"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["kerberos"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "flink".to_string(),
                name: "Apache Flink".to_string(),
                description: "Stream processing framework".to_string(),
                category: ConnectorCategory::BigData,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "streaming"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec![].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "hive".to_string(),
                name: "Apache Hive".to_string(),
                description: "Data warehouse infrastructure".to_string(),
                category: ConnectorCategory::BigData,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "batch"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["kerberos"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "impala".to_string(),
                name: "Apache Impala".to_string(),
                description: "Query engine for Hadoop".to_string(),
                category: ConnectorCategory::BigData,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["kerberos"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "delta_lake".to_string(),
                name: "Delta Lake".to_string(),
                description: "ACID table format for data lakes".to_string(),
                category: ConnectorCategory::BigData,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec![].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "iceberg".to_string(),
                name: "Apache Iceberg".to_string(),
                description: "Table format with hidden partitioning".to_string(),
                category: ConnectorCategory::BigData,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec![].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "hudi".to_string(),
                name: "Apache Hudi".to_string(),
                description: "Incremental processing framework for data lakes".to_string(),
                category: ConnectorCategory::BigData,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "incremental"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec![].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
        ]
    }

    /// Count total connectors
    pub fn count() -> usize {
        Self::all().len()
    }

    /// Get connectors by category
    pub fn by_category(category: ConnectorCategory) -> Vec<ConnectorInfo> {
        Self::all()
            .into_iter()
            .filter(|c| c.category == category)
            .collect()
    }
}

    fn finance() -> Vec<ConnectorInfo> {
        vec![
            ConnectorInfo {
                id: "quickbooks_online".to_string(),
                name: "QuickBooks Online".to_string(),
                description: "Cloud accounting software".to_string(),
                category: ConnectorCategory::Finance,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(100)),
            },
            ConnectorInfo {
                id: "xero".to_string(),
                name: "Xero".to_string(),
                description: "Online accounting software".to_string(),
                category: ConnectorCategory::Finance,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "netsuite".to_string(),
                name: "NetSuite".to_string(),
                description: "ERP and financial management".to_string(),
                category: ConnectorCategory::Finance,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth", "token"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "stripe_billing".to_string(),
                name: "Stripe Billing".to_string(),
                description: "Subscription and invoice management".to_string(),
                category: ConnectorCategory::Finance,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(100)),
            },
            ConnectorInfo {
                id: "freshbooks".to_string(),
                name: "FreshBooks".to_string(),
                description: "Invoicing and accounting software".to_string(),
                category: ConnectorCategory::Finance,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "wave".to_string(),
                name: "Wave".to_string(),
                description: "Accounting software for small business".to_string(),
                category: ConnectorCategory::Finance,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "sage".to_string(),
                name: "Sage".to_string(),
                description: "Enterprise accounting and ERP".to_string(),
                category: ConnectorCategory::Finance,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "chargebee".to_string(),
                name: "Chargebee".to_string(),
                description: "Subscription billing platform".to_string(),
                category: ConnectorCategory::Finance,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
        ]
    }

    fn hr_payroll() -> Vec<ConnectorInfo> {
        vec![
            ConnectorInfo {
                id: "workday".to_string(),
                name: "Workday".to_string(),
                description: "Enterprise HCM and payroll".to_string(),
                category: ConnectorCategory::HRPayroll,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "bamboohr".to_string(),
                name: "BambooHR".to_string(),
                description: "HR software".to_string(),
                category: ConnectorCategory::HRPayroll,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "adp".to_string(),
                name: "ADP".to_string(),
                description: "Payroll and HR services".to_string(),
                category: ConnectorCategory::HRPayroll,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "gusto".to_string(),
                name: "Gusto".to_string(),
                description: "Payroll and HR platform".to_string(),
                category: ConnectorCategory::HRPayroll,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "rippling".to_string(),
                name: "Rippling".to_string(),
                description: "HR, IT, and payroll software".to_string(),
                category: ConnectorCategory::HRPayroll,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "paychex".to_string(),
                name: "Paychex".to_string(),
                description: "Payroll and HR solutions".to_string(),
                category: ConnectorCategory::HRPayroll,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "ukg".to_string(),
                name: "UKG (Ultimate Kronos)".to_string(),
                description: "Workforce management and payroll".to_string(),
                category: ConnectorCategory::HRPayroll,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
        ]
    }

    fn project_management() -> Vec<ConnectorInfo> {
        vec![
            ConnectorInfo {
                id: "asana".to_string(),
                name: "Asana".to_string(),
                description: "Project and task management".to_string(),
                category: ConnectorCategory::ProjectManagement,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth", "api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "monday".to_string(),
                name: "Monday.com".to_string(),
                description: "Work management platform".to_string(),
                category: ConnectorCategory::ProjectManagement,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "notion".to_string(),
                name: "Notion".to_string(),
                description: "Workspace and knowledge base".to_string(),
                category: ConnectorCategory::ProjectManagement,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth", "api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "airtable".to_string(),
                name: "Airtable".to_string(),
                description: "Spreadsheet-database hybrid".to_string(),
                category: ConnectorCategory::ProjectManagement,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key", "oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(30)),
            },
            ConnectorInfo {
                id: "linear".to_string(),
                name: "Linear".to_string(),
                description: "Issue tracking for software teams".to_string(),
                category: ConnectorCategory::ProjectManagement,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "jira_cloud".to_string(),
                name: "Jira Cloud".to_string(),
                description: "Issue and project tracking".to_string(),
                category: ConnectorCategory::ProjectManagement,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth", "api_token"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "confluence".to_string(),
                name: "Confluence".to_string(),
                description: "Wiki and documentation platform".to_string(),
                category: ConnectorCategory::ProjectManagement,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth", "api_token"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "trello".to_string(),
                name: "Trello".to_string(),
                description: "Card-based project management".to_string(),
                category: ConnectorCategory::ProjectManagement,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth", "api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
        ]
    }

    fn ecommerce() -> Vec<ConnectorInfo> {
        vec![
            ConnectorInfo {
                id: "shopify_plus".to_string(),
                name: "Shopify Plus".to_string(),
                description: "Enterprise e-commerce platform".to_string(),
                category: ConnectorCategory::Ecommerce,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(2)),
            },
            ConnectorInfo {
                id: "bigcommerce".to_string(),
                name: "BigCommerce".to_string(),
                description: "E-commerce platform".to_string(),
                category: ConnectorCategory::Ecommerce,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "wix".to_string(),
                name: "Wix".to_string(),
                description: "Website builder and e-commerce".to_string(),
                category: ConnectorCategory::Ecommerce,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "lightspeed".to_string(),
                name: "Lightspeed".to_string(),
                description: "Omnichannel commerce platform".to_string(),
                category: ConnectorCategory::Ecommerce,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "ecwid".to_string(),
                name: "Ecwid".to_string(),
                description: "Shopping cart software".to_string(),
                category: ConnectorCategory::Ecommerce,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "webflow".to_string(),
                name: "Webflow".to_string(),
                description: "Web design and hosting platform".to_string(),
                category: ConnectorCategory::Ecommerce,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "square_online".to_string(),
                name: "Square Online".to_string(),
                description: "E-commerce and payments".to_string(),
                category: ConnectorCategory::Ecommerce,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
        ]
    }

    fn identity() -> Vec<ConnectorInfo> {
        vec![
            ConnectorInfo {
                id: "auth0".to_string(),
                name: "Auth0".to_string(),
                description: "Identity and access management".to_string(),
                category: ConnectorCategory::Identity,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key", "oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "okta".to_string(),
                name: "Okta".to_string(),
                description: "Identity and access management".to_string(),
                category: ConnectorCategory::Identity,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_token"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "azure_ad".to_string(),
                name: "Azure Active Directory".to_string(),
                description: "Identity and access management".to_string(),
                category: ConnectorCategory::Identity,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth", "service_principal"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "keycloak".to_string(),
                name: "Keycloak".to_string(),
                description: "Open-source identity platform".to_string(),
                category: ConnectorCategory::Identity,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_token"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "plaid".to_string(),
                name: "Plaid".to_string(),
                description: "Financial API and identity".to_string(),
                category: ConnectorCategory::Identity,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
        ]
    }

    fn social_media() -> Vec<ConnectorInfo> {
        vec![
            ConnectorInfo {
                id: "facebook".to_string(),
                name: "Facebook".to_string(),
                description: "Social network and data".to_string(),
                category: ConnectorCategory::SocialMedia,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "instagram".to_string(),
                name: "Instagram".to_string(),
                description: "Social media platform".to_string(),
                category: ConnectorCategory::SocialMedia,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "twitter".to_string(),
                name: "Twitter/X".to_string(),
                description: "Social network platform".to_string(),
                category: ConnectorCategory::SocialMedia,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth", "api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(300)),
            },
            ConnectorInfo {
                id: "pinterest".to_string(),
                name: "Pinterest".to_string(),
                description: "Social media platform".to_string(),
                category: ConnectorCategory::SocialMedia,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "snapchat".to_string(),
                name: "Snapchat".to_string(),
                description: "Social media platform".to_string(),
                category: ConnectorCategory::SocialMedia,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "tiktok_data".to_string(),
                name: "TikTok Data".to_string(),
                description: "Social media analytics".to_string(),
                category: ConnectorCategory::SocialMedia,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "youtube".to_string(),
                name: "YouTube".to_string(),
                description: "Video platform and analytics".to_string(),
                category: ConnectorCategory::SocialMedia,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
        ]
    }

    fn publishing() -> Vec<ConnectorInfo> {
        vec![
            ConnectorInfo {
                id: "medium".to_string(),
                name: "Medium".to_string(),
                description: "Publishing platform".to_string(),
                category: ConnectorCategory::Publishing,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth", "api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "wordpress".to_string(),
                name: "WordPress.com".to_string(),
                description: "Blogging and CMS platform".to_string(),
                category: ConnectorCategory::Publishing,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "substack".to_string(),
                name: "Substack".to_string(),
                description: "Newsletter platform".to_string(),
                category: ConnectorCategory::Publishing,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "ghost".to_string(),
                name: "Ghost".to_string(),
                description: "Publishing platform".to_string(),
                category: ConnectorCategory::Publishing,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "dev_to".to_string(),
                name: "Dev.to".to_string(),
                description: "Developer community platform".to_string(),
                category: ConnectorCategory::Publishing,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "hashnode".to_string(),
                name: "Hashnode".to_string(),
                description: "Developer blogging platform".to_string(),
                category: ConnectorCategory::Publishing,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["api_key", "oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
        ]
    }
}

    fn developer() -> Vec<ConnectorInfo> {
        vec![
            ConnectorInfo {
                id: "github".to_string(),
                name: "GitHub".to_string(),
                description: "Developer collaboration platform".to_string(),
                category: ConnectorCategory::Developer,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth", "personal_token"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(60)),
            },
            ConnectorInfo {
                id: "gitlab".to_string(),
                name: "GitLab".to_string(),
                description: "DevOps platform".to_string(),
                category: ConnectorCategory::Developer,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth", "personal_token"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "bitbucket".to_string(),
                name: "Bitbucket".to_string(),
                description: "Git repository hosting".to_string(),
                category: ConnectorCategory::Developer,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth", "app_password"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "duckdb".to_string(),
                name: "DuckDB".to_string(),
                description: "In-process analytical database".to_string(),
                category: ConnectorCategory::Developer,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "sql"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["file_path"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
        ]
    }

    fn file_transfer() -> Vec<ConnectorInfo> {
        vec![
            ConnectorInfo {
                id: "sftp".to_string(),
                name: "SFTP".to_string(),
                description: "SSH file transfer protocol".to_string(),
                category: ConnectorCategory::FileTransfer,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "delete"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["password", "ssh_key", "certificate"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "ftp".to_string(),
                name: "FTP".to_string(),
                description: "File transfer protocol".to_string(),
                category: ConnectorCategory::FileTransfer,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "delete"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["password"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "smb".to_string(),
                name: "SMB/CIFS".to_string(),
                description: "Windows file sharing".to_string(),
                category: ConnectorCategory::FileTransfer,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "delete"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["password", "ntlm"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "dropbox".to_string(),
                name: "Dropbox".to_string(),
                description: "Cloud file storage".to_string(),
                category: ConnectorCategory::FileTransfer,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "delete"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(150)),
            },
        ]
    }

    fn spreadsheets() -> Vec<ConnectorInfo> {
        vec![
            ConnectorInfo {
                id: "google_sheets".to_string(),
                name: "Google Sheets".to_string(),
                description: "Collaborative spreadsheets".to_string(),
                category: ConnectorCategory::Other,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth", "service_account"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: Some(RateLimitDefault::conservative(500)),
            },
            ConnectorInfo {
                id: "microsoft_excel".to_string(),
                name: "Microsoft Excel (OneDrive)".to_string(),
                description: "Excel files via OneDrive".to_string(),
                category: ConnectorCategory::Other,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "local_excel".to_string(),
                name: "Excel Files (Local)".to_string(),
                description: "Local .xlsx and .xls files".to_string(),
                category: ConnectorCategory::Other,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec![].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
        ]
    }

    fn enterprise() -> Vec<ConnectorInfo> {
        vec![
            ConnectorInfo {
                id: "sap_erp".to_string(),
                name: "SAP ERP".to_string(),
                description: "SAP Enterprise Resource Planning".to_string(),
                category: ConnectorCategory::Enterprise,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["basic", "sso"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "oracle_ebs".to_string(),
                name: "Oracle EBS".to_string(),
                description: "Oracle E-Business Suite".to_string(),
                category: ConnectorCategory::Enterprise,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["basic", "oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "netsuite_erp".to_string(),
                name: "NetSuite ERP".to_string(),
                description: "Oracle NetSuite (already covered in Finance)".to_string(),
                category: ConnectorCategory::Enterprise,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["oauth", "token"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "teradata".to_string(),
                name: "Teradata".to_string(),
                description: "Enterprise data warehouse".to_string(),
                category: ConnectorCategory::Enterprise,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["password", "ldap"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "greenplum".to_string(),
                name: "Greenplum".to_string(),
                description: "Distributed data warehouse".to_string(),
                category: ConnectorCategory::Enterprise,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["password"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
        ]
    }

    fn legacy() -> Vec<ConnectorInfo> {
        vec![
            ConnectorInfo {
                id: "db2".to_string(),
                name: "IBM DB2".to_string(),
                description: "IBM relational database".to_string(),
                category: ConnectorCategory::Legacy,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["password"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "sybase".to_string(),
                name: "Sybase ASE".to_string(),
                description: "Sybase adaptive server".to_string(),
                category: ConnectorCategory::Legacy,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["password"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "firebird".to_string(),
                name: "Firebird".to_string(),
                description: "Open source relational database".to_string(),
                category: ConnectorCategory::Legacy,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["password"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "h2".to_string(),
                name: "H2 Database".to_string(),
                description: "Embedded relational database".to_string(),
                category: ConnectorCategory::Legacy,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["password"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "couchdb".to_string(),
                name: "CouchDB".to_string(),
                description: "Document-oriented database".to_string(),
                category: ConnectorCategory::Legacy,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["password", "oauth"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "jms".to_string(),
                name: "JMS (Java Message Service)".to_string(),
                description: "Java message queue standard".to_string(),
                category: ConnectorCategory::Legacy,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write", "stream"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                auth_methods: vec!["basic", "ssl"].iter().map(|s| s.to_string()).collect(),
                rate_limit_default: None,
            },
            ConnectorInfo {
                id: "mainframe".to_string(),
                name: "Mainframe (AS/400, COBOL)".to_string(),
                description: "Legacy mainframe systems".to_string(),
                category: ConnectorCategory::Legacy,
                connector_type: ConnectorTypeInfo::Both,
                capabilities: vec!["read", "write"].iter().map(|s| s.to_string()).collect(),
                auth_methods: vec!["basic", "custom"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                rate_limit_default: None,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connector_registry_count() {
        let connectors = ConnectorRegistry::all();
        println!("Total connectors: {}", connectors.len());
        assert!(connectors.len() >= 130);
    }

    #[test]
    fn test_connectors_by_category() {
        let databases = ConnectorRegistry::by_category(ConnectorCategory::Database);
        assert!(!databases.is_empty());

        let warehouses = ConnectorRegistry::by_category(ConnectorCategory::DataWarehouse);
        assert!(!warehouses.is_empty());
    }

    #[test]
    fn test_connector_details() {
        let all = ConnectorRegistry::all();
        let postgres = all.iter().find(|c| c.id == "postgres").unwrap();

        assert_eq!(postgres.name, "PostgreSQL");
        assert_eq!(postgres.connector_type, ConnectorTypeInfo::Both);
        assert!(postgres.capabilities.contains(&"read".to_string()));
    }
}
