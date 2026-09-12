//! Error types for the persistence layer.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum PersistError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("migration error: {0}")]
    Migration(String),

    #[error("recovery error: {0}")]
    Recovery(String),

    #[error("lock error: {0}")]
    Lock(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("not found: {0}")]
    NotFound(String),
}
