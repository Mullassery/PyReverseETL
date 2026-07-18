// Connector testing framework
// Tests all 50 core connectors for functionality, reliability, and performance

pub mod connector_test;
pub mod harness;
pub mod test_data;
pub mod metrics;

pub use connector_test::{ConnectorTest, TestType, TestResult, Assertion};
pub use harness::{ConnectorTestHarness, ConnectorTestReport};
pub use test_data::{TestDatabase, TestDataGenerator};
pub use metrics::ConnectorMetrics;
