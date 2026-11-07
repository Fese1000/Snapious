//! Shared error type of the core.

use thiserror::Error;

/// Core errors. Messages are meant for users (German).
#[derive(Debug, Error, PartialEq)]
pub enum Error {
    /// Invalid input (wrong value, empty sequence, unknown name).
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    /// Call in the wrong state.
    #[error("Invalid state: {0}")]
    InvalidState(String),
    /// Position or range outside the sequence.
    #[error("Out of range: {0}")]
    OutOfRange(String),
    /// Error while reading or writing files.
    #[error("File error: {0}")]
    Io(String),
    /// An operation failed (message for the UI).
    #[error("{0}")]
    Operation(String),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e.to_string())
    }
}

/// Result with the core error type.
pub type Result<T> = std::result::Result<T, Error>;

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Error::Io(format!("Database: {}", e))
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Io(format!("JSON: {}", e))
    }
}
