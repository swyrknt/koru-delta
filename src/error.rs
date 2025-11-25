/// Error types for KoruDelta operations.
///
/// This module provides a comprehensive error hierarchy that covers all failure
/// modes in the database. All errors are well-typed and can be pattern-matched
/// for precise error handling.
use thiserror::Error;

/// The main error type for KoruDelta operations.
///
/// All fallible operations in KoruDelta return `Result<T, DeltaError>`.
/// This provides a unified error handling interface across the entire API.
#[derive(Error, Debug)]
pub enum DeltaError {
    /// Key not found in the specified namespace
    #[error("Key '{key}' not found in namespace '{namespace}'")]
    KeyNotFound {
        /// The namespace that was queried
        namespace: String,
        /// The key that was not found
        key: String,
    },

    /// No value exists at the specified timestamp
    #[error("No value for key '{key}' in namespace '{namespace}' at timestamp {timestamp}")]
    NoValueAtTimestamp {
        /// The namespace that was queried
        namespace: String,
        /// The key that was queried
        key: String,
        /// The timestamp that was queried
        timestamp: i64,
    },

    /// Serialization error when converting data to/from JSON
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Invalid data format or structure
    #[error("Invalid data: {reason}")]
    InvalidData {
        /// Description of why the data is invalid
        reason: String,
    },

    /// Internal error from the distinction engine
    #[error("Engine error: {0}")]
    EngineError(String),

    /// Storage operation failed
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Time-related error (invalid timestamp, time travel to future, etc.)
    #[error("Time error: {0}")]
    TimeError(String),
}

/// Result type alias for KoruDelta operations.
///
/// This is a convenience alias for `Result<T, DeltaError>` that makes
/// function signatures more concise throughout the codebase.
pub type DeltaResult<T> = Result<T, DeltaError>;
