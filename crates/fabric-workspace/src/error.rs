// Copyright 2026 Phenotype authors
//! Workspace errors and types.

use thiserror::Error;

/// Errors from the fabric-workspace crate.
#[derive(Debug, Error)]
pub enum Error {
    #[error("workspace not found: {0}")]
    NotFound(String),

    #[error("seat conflict: seat '{seat_id}' held by '{holder}'")]
    SeatConflict {
        seat_id: String,
        holder: String,
        locality_tier: String,
    },

    #[error("lease expired: {0}")]
    LeaseExpired(String),

    #[error("workspace already exists: {0}")]
    AlreadyExists(String),

    #[error("serialization failed: {0}")]
    Serialization(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid state: {0}")]
    InvalidState(String),

    #[error("workspace is terminal: {0}")]
    Terminal(String),
}

/// Result type for fabric-workspace operations.
pub type Result<T> = std::result::Result<T, Error>;
