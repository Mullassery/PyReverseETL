use std::fmt;

/// Adapter-specific error types
#[derive(Debug)]
pub enum AdapterError {
    /// Authentication failed
    AuthenticationFailed(String),
    /// Connection error
    ConnectionError(String),
    /// Unsupported destination type
    UnsupportedDestination(String),
    /// Invalid configuration
    InvalidConfiguration(String),
    /// Field mapping error
    FieldMappingError(String),
    /// Operation failed (upsert, delete, etc.)
    OperationFailed(String),
    /// Rate limit exceeded
    RateLimitExceeded { retry_after_ms: u32 },
    /// Validation error
    ValidationError(String),
    /// Network error
    NetworkError(String),
    /// Timeout
    Timeout,
    /// Schema not available
    SchemaNotAvailable,
    /// Batch size exceeded
    BatchSizeExceeded { max_size: u32, requested: u32 },
    /// Not implemented yet
    NotImplemented(String),
}

impl fmt::Display for AdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdapterError::AuthenticationFailed(msg) => write!(f, "Authentication failed: {}", msg),
            AdapterError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            AdapterError::UnsupportedDestination(dtype) => write!(f, "Unsupported destination: {}", dtype),
            AdapterError::InvalidConfiguration(msg) => write!(f, "Invalid configuration: {}", msg),
            AdapterError::FieldMappingError(msg) => write!(f, "Field mapping error: {}", msg),
            AdapterError::OperationFailed(msg) => write!(f, "Operation failed: {}", msg),
            AdapterError::RateLimitExceeded { retry_after_ms } => {
                write!(f, "Rate limit exceeded, retry after {}ms", retry_after_ms)
            }
            AdapterError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            AdapterError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            AdapterError::Timeout => write!(f, "Operation timed out"),
            AdapterError::SchemaNotAvailable => write!(f, "Destination schema not available"),
            AdapterError::BatchSizeExceeded { max_size, requested } => {
                write!(f, "Batch size exceeded: max {}, requested {}", max_size, requested)
            }
            AdapterError::NotImplemented(msg) => write!(f, "Not implemented: {}", msg),
        }
    }
}

impl std::error::Error for AdapterError {}

impl From<AdapterError> for crate::Error {
    fn from(err: AdapterError) -> Self {
        crate::Error::StorageError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authentication_failed_error() {
        let err = AdapterError::AuthenticationFailed("Invalid API key".to_string());
        assert_eq!(err.to_string(), "Authentication failed: Invalid API key");
    }

    #[test]
    fn test_unsupported_destination_error() {
        let err = AdapterError::UnsupportedDestination("unknown_platform".to_string());
        assert_eq!(err.to_string(), "Unsupported destination: unknown_platform");
    }

    #[test]
    fn test_rate_limit_error() {
        let err = AdapterError::RateLimitExceeded { retry_after_ms: 5000 };
        assert_eq!(err.to_string(), "Rate limit exceeded, retry after 5000ms");
    }

    #[test]
    fn test_batch_size_exceeded_error() {
        let err = AdapterError::BatchSizeExceeded {
            max_size: 100,
            requested: 150,
        };
        assert!(err.to_string().contains("Batch size exceeded"));
    }
}
