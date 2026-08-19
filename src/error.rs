//! Error types for velocity-msi

use thiserror::Error;

/// Result type for MSI operations
pub type Result<T> = std::result::Result<T, MsiError>;

/// Errors that can occur during MSI generation
#[derive(Error, Debug)]
pub enum MsiError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Table not found: {0}")]
    TableNotFound(String),

    #[error("Table already exists: {0}")]
    TableAlreadyExists(String),

    #[error("Column count mismatch: expected {expected}, got {actual}")]
    ColumnCountMismatch { expected: usize, actual: usize },

    #[error("Invalid column name: {0}")]
    InvalidColumnName(String),

    #[error("Invalid table name: {0}")]
    InvalidTableName(String),

    #[error("Primary key required for column: {0}")]
    PrimaryKeyRequired(String),

    #[error("String too long: {len} > {max}")]
    StringTooLong { len: usize, max: usize },

    #[error("Encoding error: {0}")]
    EncodingError(String),

    #[error("CFB error: {0}")]
    CfbError(String),
}
