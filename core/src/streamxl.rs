use serde::{Deserialize, Serialize};

/// Integration with StreamXL for spreadsheet-based data sources.
///
/// PyReverseETL can activate data from StreamXL queries, models, and sheets.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamXLSource {
    /// StreamXL API base URL
    pub api_url: String,
    /// Sheet/table name to query
    pub sheet_name: String,
    /// Optional query to filter/transform data
    pub query: Option<String>,
    /// Column mapping to entity fields
    pub column_mapping: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamXLConfig {
    /// StreamXL API base URL (e.g., http://localhost:8001)
    pub api_url: String,
    /// Default sheet access method (direct, query, model)
    pub access_method: StreamXLAccessMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamXLAccessMethod {
    Direct,     // Read sheet directly
    Query,      // Execute query
    Model,      // Use StreamXL model
}

impl StreamXLSource {
    pub fn new(api_url: impl Into<String>, sheet_name: impl Into<String>) -> Self {
        StreamXLSource {
            api_url: api_url.into(),
            sheet_name: sheet_name.into(),
            query: None,
            column_mapping: std::collections::HashMap::new(),
        }
    }

    pub fn with_query(mut self, query: impl Into<String>) -> Self {
        self.query = Some(query.into());
        self
    }

    pub fn add_column_mapping(mut self, excel_column: impl Into<String>, field: impl Into<String>) -> Self {
        self.column_mapping.insert(excel_column.into(), field.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streamxl_source_creation() {
        let source = StreamXLSource::new("http://localhost:8001", "customers");
        assert_eq!(source.sheet_name, "customers");
    }

    #[test]
    fn test_streamxl_source_with_mapping() {
        let source = StreamXLSource::new("http://localhost:8001", "customers")
            .add_column_mapping("A", "customer_id")
            .add_column_mapping("B", "email")
            .add_column_mapping("C", "lifetime_value");

        assert_eq!(source.column_mapping.len(), 3);
    }
}
