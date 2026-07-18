// Test data generation and management for connector tests
use serde_json::{json, Value};
use std::collections::HashMap;

pub struct TestDatabase;

pub struct TestDataGenerator {
    dataset_size: usize,
}

#[derive(Debug, Clone)]
pub struct ConnectorMetrics {
    pub records_processed: u64,
    pub bytes_transferred: u64,
    pub latency_ms: f64,
    pub throughput_rps: f64,
    pub error_rate: f64,
    pub memory_used_mb: f64,
}

impl TestDataGenerator {
    pub fn new(size: usize) -> Self {
        Self { dataset_size: size }
    }

    /// Generate customer test data
    pub fn generate_customers(&self) -> Vec<Value> {
        (0..self.dataset_size)
            .map(|i| {
                json!({
                    "customer_id": i,
                    "name": format!("Customer {}", i),
                    "email": format!("customer{}@example.com", i),
                    "lifetime_value": (i as f64) * 100.50,
                    "created_at": "2024-01-01T00:00:00Z",
                    "status": if i % 2 == 0 { "active" } else { "inactive" },
                })
            })
            .collect()
    }

    /// Generate order test data
    pub fn generate_orders(&self) -> Vec<Value> {
        (0..self.dataset_size)
            .map(|i| {
                json!({
                    "order_id": i,
                    "customer_id": i % 100,
                    "amount": (i as f64) * 10.50,
                    "quantity": (i % 100) + 1,
                    "created_at": "2024-01-15T00:00:00Z",
                    "status": ["pending", "shipped", "delivered"][i % 3],
                })
            })
            .collect()
    }

    /// Generate event test data
    pub fn generate_events(&self) -> Vec<Value> {
        (0..self.dataset_size)
            .map(|i| {
                json!({
                    "event_id": format!("event_{}", i),
                    "user_id": i % 1000,
                    "event_type": ["click", "view", "purchase"][i % 3],
                    "timestamp": "2024-01-15T12:00:00Z",
                    "properties": {
                        "page": format!("/page/{}", i % 50),
                        "source": ["web", "mobile", "app"][i % 3],
                    }
                })
            })
            .collect()
    }

    /// Generate product test data
    pub fn generate_products(&self) -> Vec<Value> {
        (0..self.dataset_size.min(1000))
            .map(|i| {
                json!({
                    "product_id": i,
                    "name": format!("Product {}", i),
                    "price": (i as f64) * 29.99,
                    "sku": format!("SKU-{:06}", i),
                    "stock": (i % 1000) as i32,
                    "category": ["electronics", "clothing", "books"][i % 3],
                })
            })
            .collect()
    }

    /// Generate user profile test data
    pub fn generate_users(&self) -> Vec<Value> {
        (0..self.dataset_size)
            .map(|i| {
                json!({
                    "user_id": i,
                    "username": format!("user_{}", i),
                    "email": format!("user{}@example.com", i),
                    "age": 18 + (i % 60),
                    "location": ["US", "EU", "APAC"][i % 3],
                    "created_at": "2024-01-01T00:00:00Z",
                    "last_login": "2024-01-15T10:00:00Z",
                })
            })
            .collect()
    }

    /// Generate random records with schema
    pub fn generate_schema(
        &self,
        schema: HashMap<String, String>,
    ) -> Vec<Value> {
        (0..self.dataset_size)
            .map(|i| {
                let mut record = json!({});
                for (field, field_type) in &schema {
                    record[field] = match field_type.as_str() {
                        "integer" => json!(i),
                        "string" => json!(format!("value_{}", i)),
                        "float" => json!((i as f64) * 1.5),
                        "boolean" => json!(i % 2 == 0),
                        _ => json!(null),
                    };
                }
                record
            })
            .collect()
    }

    /// Generate small dataset (100 records)
    pub fn small(&self) -> Vec<Value> {
        Self::new(100).generate_customers()
    }

    /// Generate medium dataset (10K records)
    pub fn medium(&self) -> Vec<Value> {
        Self::new(10_000).generate_customers()
    }

    /// Generate large dataset (100K records)
    pub fn large(&self) -> Vec<Value> {
        Self::new(100_000).generate_customers()
    }
}

impl TestDatabase {
    pub fn new() -> Self {
        Self
    }

    /// Create test connection string
    pub fn postgres_url() -> String {
        std::env::var("TEST_POSTGRES_URL")
            .unwrap_or_else(|_| "postgresql://postgres:password@localhost/test_pyreverseetl".to_string())
    }

    pub fn mysql_url() -> String {
        std::env::var("TEST_MYSQL_URL")
            .unwrap_or_else(|_| "mysql://root:password@localhost/test_pyreverseetl".to_string())
    }

    pub fn mongodb_url() -> String {
        std::env::var("TEST_MONGODB_URL")
            .unwrap_or_else(|_| "mongodb://localhost:27017/test_pyreverseetl".to_string())
    }

    pub fn redis_url() -> String {
        std::env::var("TEST_REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".to_string())
    }

    pub fn s3_bucket() -> String {
        std::env::var("TEST_S3_BUCKET")
            .unwrap_or_else(|_| "test-pyreverseetl".to_string())
    }

    pub fn kafka_brokers() -> Vec<String> {
        vec!["localhost:9092".to_string()]
    }
}

impl Default for TestDataGenerator {
    fn default() -> Self {
        Self::new(1000)
    }
}

impl Default for TestDatabase {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectorMetrics {
    pub fn new() -> Self {
        Self {
            records_processed: 0,
            bytes_transferred: 0,
            latency_ms: 0.0,
            throughput_rps: 0.0,
            error_rate: 0.0,
            memory_used_mb: 0.0,
        }
    }

    pub fn with_records(mut self, count: u64) -> Self {
        self.records_processed = count;
        self
    }

    pub fn with_throughput(mut self, rps: f64) -> Self {
        self.throughput_rps = rps;
        self
    }

    pub fn with_latency(mut self, ms: f64) -> Self {
        self.latency_ms = ms;
        self
    }

    pub fn with_error_rate(mut self, rate: f64) -> Self {
        self.error_rate = rate;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_customer_generation() {
        let gen = TestDataGenerator::new(10);
        let customers = gen.generate_customers();
        assert_eq!(customers.len(), 10);
        assert!(customers[0]["customer_id"].is_number());
    }

    #[test]
    fn test_order_generation() {
        let gen = TestDataGenerator::new(50);
        let orders = gen.generate_orders();
        assert_eq!(orders.len(), 50);
        assert!(orders[0]["amount"].is_number());
    }

    #[test]
    fn test_event_generation() {
        let gen = TestDataGenerator::new(100);
        let events = gen.generate_events();
        assert_eq!(events.len(), 100);
        assert!(events[0]["event_id"].is_string());
    }

    #[test]
    fn test_product_generation() {
        let gen = TestDataGenerator::new(2000);
        let products = gen.generate_products();
        assert!(products.len() <= 1000);
    }

    #[test]
    fn test_test_database_urls() {
        let postgres = TestDatabase::postgres_url();
        assert!(postgres.contains("postgresql"));

        let mysql = TestDatabase::mysql_url();
        assert!(mysql.contains("mysql"));
    }

    #[test]
    fn test_metrics_builder() {
        let metrics = ConnectorMetrics::new()
            .with_records(1000)
            .with_throughput(500.0)
            .with_latency(50.0);

        assert_eq!(metrics.records_processed, 1000);
        assert_eq!(metrics.throughput_rps, 500.0);
        assert_eq!(metrics.latency_ms, 50.0);
    }
}
