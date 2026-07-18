// Individual connector test definitions and results
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use crate::connectors::Capability;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorTest {
    pub name: String,
    pub connector_id: String,
    pub test_type: TestType,
    pub expected_capability: Capability,
    pub test_data: Option<Vec<serde_json::Value>>,
    pub assertions: Vec<Assertion>,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TestType {
    /// Can establish connection?
    Connection,
    /// Can detect schema from source?
    SchemaDetection,
    /// Can read data from source?
    Read,
    /// Can write single record to destination?
    Write,
    /// Can write batch of records?
    BatchWrite,
    /// Does rate limiting work correctly?
    RateLimit,
    /// Does error recovery work (retries, backoff)?
    ErrorRecovery,
    /// Circuit breaker pattern?
    CircuitBreaker,
    /// Full end-to-end integration?
    Integration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assertion {
    pub name: String,
    pub assertion_type: AssertionType,
    pub expected: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssertionType {
    /// Result equals expected value
    Equals,
    /// Result contains substring
    Contains,
    /// Result matches regex
    Matches,
    /// Numeric comparison (>, <, >=, <=, ==)
    Numeric(String),
    /// Custom assertion function
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_name: String,
    pub connector_id: String,
    pub test_type: TestType,
    pub passed: bool,
    pub duration_ms: u64,
    pub error: Option<String>,
    pub metrics: TestMetrics,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TestMetrics {
    /// Records processed in test
    pub records_processed: u64,
    /// Bytes transferred
    pub bytes_transferred: u64,
    /// Average latency (ms)
    pub latency_ms: f64,
    /// Throughput (records/sec)
    pub throughput_rps: f64,
    /// Error rate (0.0 - 1.0)
    pub error_rate: f64,
    /// Memory used (MB)
    pub memory_used_mb: f64,
}

impl ConnectorTest {
    pub fn new(
        name: impl Into<String>,
        connector_id: impl Into<String>,
        test_type: TestType,
        expected_capability: Capability,
    ) -> Self {
        Self {
            name: name.into(),
            connector_id: connector_id.into(),
            test_type,
            expected_capability,
            test_data: None,
            assertions: Vec::new(),
            timeout_seconds: 30,
        }
    }

    pub fn with_data(mut self, data: Vec<serde_json::Value>) -> Self {
        self.test_data = Some(data);
        self
    }

    pub fn with_assertion(mut self, assertion: Assertion) -> Self {
        self.assertions.push(assertion);
        self
    }

    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }
}

impl TestResult {
    pub fn success(
        test_name: impl Into<String>,
        connector_id: impl Into<String>,
        test_type: TestType,
        duration_ms: u64,
    ) -> Self {
        Self {
            test_name: test_name.into(),
            connector_id: connector_id.into(),
            test_type,
            passed: true,
            duration_ms,
            error: None,
            metrics: TestMetrics::default(),
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn failure(
        test_name: impl Into<String>,
        connector_id: impl Into<String>,
        test_type: TestType,
        duration_ms: u64,
        error: impl Into<String>,
    ) -> Self {
        Self {
            test_name: test_name.into(),
            connector_id: connector_id.into(),
            test_type,
            passed: false,
            duration_ms,
            error: Some(error.into()),
            metrics: TestMetrics::default(),
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn with_metrics(mut self, metrics: TestMetrics) -> Self {
        self.metrics = metrics;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connector_test_creation() {
        let test = ConnectorTest::new(
            "connection_test",
            "postgres",
            TestType::Connection,
            Capability::Read,
        );
        assert_eq!(test.name, "connection_test");
        assert_eq!(test.connector_id, "postgres");
        assert_eq!(test.test_type, TestType::Connection);
    }

    #[test]
    fn test_result_success() {
        let result = TestResult::success("test", "postgres", TestType::Connection, 100);
        assert!(result.passed);
        assert_eq!(result.duration_ms, 100);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_result_failure() {
        let result = TestResult::failure(
            "test",
            "postgres",
            TestType::Connection,
            100,
            "Connection timeout",
        );
        assert!(!result.passed);
        assert!(result.error.is_some());
    }
}
