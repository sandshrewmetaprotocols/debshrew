//! Error types for debshrew
//!
//! This module defines the error types used throughout the debshrew project.

use thiserror::Error;

/// A specialized Result type for debshrew operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error type for debshrew operations
#[derive(Error, Debug)]
pub enum Error {
    /// Error occurred during serialization or deserialization
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Error occurred during bincode serialization or deserialization
    #[error("Bincode error: {0}")]
    Bincode(#[from] bincode::Error),

    /// Error occurred during hex encoding or decoding
    #[error("Hex error: {0}")]
    Hex(#[from] hex::FromHexError),

    /// Error occurred during I/O operations
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Error occurred during UTF-8 conversion
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    /// Error occurred during URL parsing
    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    /// Error occurred during regex operations
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    /// Error occurred during CDC message generation
    #[error("CDC message error: {0}")]
    CdcMessage(String),

    /// Error occurred during state management
    #[error("State error: {0}")]
    State(String),

    /// Error occurred during view access
    #[error("View access error: {0}")]
    ViewAccess(String),

    /// Error occurred during block processing
    #[error("Block processing error: {0}")]
    BlockProcessing(String),

    /// Error occurred during reorg handling
    #[error("Reorg handling error: {0}")]
    ReorgHandling(String),

    /// Error occurred during sink operations
    #[error("Sink error: {0}")]
    Sink(String),

    /// Error occurred during transform operations
    #[error("Transform error: {0}")]
    Transform(String),

    /// Error occurred during WASM operations
    #[error("WASM error: {0}")]
    Wasm(String),

    /// Error occurred during metashrew client operations
    #[error("Metashrew client error: {0}")]
    MetashrewClient(String),

    /// Generic error with a message
    #[error("{0}")]
    Generic(String),
}

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Error::Generic(s.to_string())
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::Generic(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_from_string() {
        let error = Error::from("test error");
        assert!(matches!(error, Error::Generic(_)));
        if let Error::Generic(msg) = error {
            assert_eq!(msg, "test error");
        }
    }

    #[test]
    fn test_error_display() {
        let error = Error::Generic("test error".to_string());
        assert_eq!(error.to_string(), "test error");

        let error = Error::CdcMessage("invalid message".to_string());
        assert_eq!(error.to_string(), "CDC message error: invalid message");
    }
}
