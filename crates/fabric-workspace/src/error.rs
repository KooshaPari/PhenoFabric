// Copyright 2026 Phenotype authors
//! Workspace errors and types.

use thiserror::Error;

/// Errors from the fabric-workspace crate.
#[derive(Debug, Error)]
pub enum Error {
    /// The requested workspace was not found.
    #[error("workspace not found: {0}")]
    NotFound(String),

    /// A seat is already held by another workspace.
    #[error("seat conflict: seat '{seat_id}' held by '{holder}'")]
    SeatConflict {
        /// The conflicting seat identifier.
        seat_id: String,
        /// The holder of the conflicting seat.
        holder: String,
        /// The locality tier of the conflicting seat.
        locality_tier: String,
    },

    /// Conflict in workspace operation.
    #[error("conflict in workspace '{workspace_id}': {message}")]
    Conflict {
        /// The workspace identifier involved.
        workspace_id: String,
        /// Human-readable conflict description.
        message: String,
    },

    /// The seat lease has expired.
    #[error("lease expired: {0}")]
    LeaseExpired(String),

    /// A workspace with this ID already exists.
    #[error("workspace already exists: {0}")]
    AlreadyExists(String),

    /// Serialization or deserialization failed.
    #[error("serialization failed: {0}")]
    Serialization(String),

    /// An I/O error occurred.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The workspace is in an invalid state for the requested operation.
    #[error("invalid state: {0}")]
    InvalidState(String),

    /// The workspace is in a terminal state and cannot accept new operations.
    #[error("workspace is terminal: {0}")]
    Terminal(String),
}

/// Result type for fabric-workspace operations.
pub type Result<T> = std::result::Result<T, Error>;
