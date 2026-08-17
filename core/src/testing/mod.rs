// Connector testing framework
// Tests all 50 core connectors for functionality, reliability, and performance

pub mod connector_test;
pub mod harness;
pub mod metrics;
pub mod mock_http;
pub mod test_data;

pub use connector_test::{Assertion, ConnectorTest, TestResult, TestType};
pub use harness::{ConnectorTestHarness, ConnectorTestReport};
pub use metrics::ConnectorMetrics;
pub use mock_http::{MockHttpServer, RecordedRequest};
pub use test_data::{TestDataGenerator, TestDatabase};
