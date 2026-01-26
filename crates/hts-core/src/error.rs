//! Error types for HTS-Core operations.

use thiserror::Error;

/// Errors that can occur in HTS-Core operations.
#[derive(Debug, Error)]
pub enum HtsError {
    /// Error reading or writing files.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Error from Polars operations.
    #[error("Polars error: {0}")]
    Polars(#[from] polars::error::PolarsError),

    /// Error parsing time period string.
    #[error("Invalid period format: {0}")]
    InvalidPeriod(String),

    /// Error in hierarchy specification.
    #[error("Hierarchy error: {0}")]
    Hierarchy(String),

    /// Column not found in DataFrame.
    #[error("Column not found: {0}")]
    ColumnNotFound(String),
}

/// Result type alias for HTS-Core operations.
pub type Result<T> = std::result::Result<T, HtsError>;
