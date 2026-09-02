//! Error type for the capability crate.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    /// The platform is not supported by the requested probe.
    #[error("unsupported platform: {0}")]
    Unsupported(String),

    /// The capability descriptor failed JSON schema validation.
    #[error("schema validation failed: {0}")]
    Schema(String),

    /// A cryptographic operation failed.
    #[error("crypto error: {0}")]
    Crypto(String),

    /// Signature verification failed.
    #[error("signature verification failed: {0}")]
    Signature(String),

    /// A probed value was malformed or unreadable.
    #[error("malformed probe output at {0}: {1}")]
    Malformed(String, String),

    /// I/O error during a probe.
    #[error("i/o error at {0}: {1}")]
    Io(String, std::io::Error),

    /// A probe exceeded its time budget.
    #[error("probe '{0}' exceeded budget of {1}ms (actual: {2}ms)")]
    BudgetExceeded(String, u64, u64),

    /// Serialization / deserialization error.
    #[error("serialization error: {0}")]
    Serde(String),
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Serde(e.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io("io".to_string(), e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
