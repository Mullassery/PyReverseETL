use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConnectorError {
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Schema detection failed: {0}")]
    SchemaDetectionFailed(String),

    #[error("Read operation failed: {0}")]
    ReadFailed(String),

    #[error("Write operation failed: {0}")]
    WriteFailed(String),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Connector not found: {0}")]
    NotFound(String),

    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type ConnectorResult<T> = std::result::Result<T, ConnectorError>;
