use serde::{Deserialize, Serialize};

/// Integration with PyStreamPDF for PDF-based data sources.
///
/// PyReverseETL can activate data extracted from PDFs using PyStreamPDF
/// intelligence engine for selective retrieval and token efficiency.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamPDFSource {
    /// PyStreamPDF API base URL
    pub api_url: String,
    /// PDF file path or identifier
    pub pdf_path: String,
    /// Query/prompt to extract specific data from PDF
    pub extraction_query: String,
    /// Maximum tokens to retrieve
    pub max_tokens: Option<u32>,
    /// Confidence threshold for extracted data (0.0 - 1.0)
    pub min_confidence: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamPDFConfig {
    /// PyStreamPDF API base URL (e.g., http://localhost:8002)
    pub api_url: String,
    /// Default extraction mode
    pub extraction_mode: ExtractionMode,
    /// Default token budget per query
    pub default_max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtractionMode {
    Selective,  // Extract only relevant sections
    Full,       // Extract entire PDF (less efficient)
    Semantic,   // Semantic search-based extraction
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedData {
    pub source_pdf: String,
    pub extracted_fields: std::collections::HashMap<String, String>,
    pub confidence_scores: std::collections::HashMap<String, f32>,
    pub tokens_used: u32,
}

impl StreamPDFSource {
    pub fn new(
        api_url: impl Into<String>,
        pdf_path: impl Into<String>,
        extraction_query: impl Into<String>,
    ) -> Self {
        StreamPDFSource {
            api_url: api_url.into(),
            pdf_path: pdf_path.into(),
            extraction_query: extraction_query.into(),
            max_tokens: Some(2000),  // Default token limit
            min_confidence: Some(0.8),
        }
    }

    pub fn with_max_tokens(mut self, tokens: u32) -> Self {
        self.max_tokens = Some(tokens);
        self
    }

    pub fn with_min_confidence(mut self, confidence: f32) -> Self {
        self.min_confidence = Some(confidence);
        self
    }

    pub fn token_efficient(mut self) -> Self {
        self.max_tokens = Some(500);  // Minimize token usage
        self.min_confidence = Some(0.9);  // Higher confidence threshold
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streampdf_source_creation() {
        let source = StreamPDFSource::new(
            "http://localhost:8002",
            "/data/invoice.pdf",
            "Extract invoice number and amount",
        );
        assert_eq!(source.pdf_path, "/data/invoice.pdf");
        assert_eq!(source.max_tokens, Some(2000));
    }

    #[test]
    fn test_streampdf_token_efficient() {
        let source = StreamPDFSource::new(
            "http://localhost:8002",
            "/data/invoice.pdf",
            "Extract invoice number",
        )
        .token_efficient();

        assert_eq!(source.max_tokens, Some(500));
        assert_eq!(source.min_confidence, Some(0.9));
    }

    #[test]
    fn test_streampdf_config_modes() {
        let config = StreamPDFConfig {
            api_url: "http://localhost:8002".to_string(),
            extraction_mode: ExtractionMode::Selective,
            default_max_tokens: Some(1000),
        };

        match config.extraction_mode {
            ExtractionMode::Selective => {
                // Token-efficient selective extraction
            }
            _ => panic!("Wrong extraction mode"),
        }
    }
}
