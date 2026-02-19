//! Error types for tzbucket-core.
//!
//! This module defines the error types used throughout the library,
//! with specific error categories for parsing, timezone handling,
//! policy violations, and runtime issues.

use thiserror::Error;

/// The main error type for tzbucket operations.
#[derive(Debug, Error)]
pub enum TzBucketError {
    /// Invalid timezone name provided.
    #[error("Invalid timezone: {0}")]
    InvalidTimezone(String),

    /// Error parsing timestamp input.
    #[error("Parse error: {0}")]
    ParseError(String),

    /// DST policy violation (nonexistent or ambiguous time with error policy).
    #[error("Policy error: {0}")]
    PolicyError(String),

    /// Runtime or I/O error.
    #[error("Runtime error: {0}")]
    RuntimeError(String),
}

/// Result type alias for tzbucket operations.
pub type Result<T> = std::result::Result<T, TzBucketError>;
