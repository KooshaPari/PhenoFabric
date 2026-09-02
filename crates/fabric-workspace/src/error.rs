//! Error types for fabric-workspace.

use std::fmt;

/// All errors produced by fabric-workspace.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// I/O error reading or writing the on-disk workspace store.
    #[error("workspace store I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Workspace or lease with the given id already exists.
    #[error("workspace {id} already exists")]
    AlreadyExists {
        /// The duplicate id.
        id: String,
    },

    /// No workspace found with the given id.
    #[error("workspace {id} not found")]
    NotFound {
        /// The missing id.
        id: String,
    },

    /// A lease transitions was rejected (e.g. terminal → active).
    #[error("invalid transition on lease {lease_id}: {transition:?}")]
    InvalidTransition {
        /// The lease that rejected the transition.
        lease_id: String,
        /// The transition that was attempted.
        transition: crate::lease::Transition,
    },

    /// A workspace claim conflicts with an active seat on the same (host, tier).
    #[error("seat conflict on {host}/{tier}: held by workspace {held_by}")]
    SeatConflict {
        /// Host identifier (`CapabilityDescriptor.node_id`).
        host: String,
        /// Locality tier string ("L0", "L1", ...).
        tier: String,
        /// The workspace that currently holds the seat.
        held_by: String,
    },

    /// A general conflict (e.g. duplicate workspace name, locked file).
    #[error("conflict: {0}")]
    Conflict(String),

    /// Workspace JSON (de)serialization error.
    #[error("workspace JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Workspace store directory is not configured.
    #[error("workspace store path not configured; pass --store-dir or set FABRIC_WORKSPACE_DIR")]
    StoreDirNotSet,
}

/// Crate-local result alias.
pub type Result<T> = std::result::Result<T, Error>;

/// Format an optional string with a fallback for display contexts.
pub(crate) fn display_opt(opt: &Option<String>, fallback: &str) -> String {
    opt.as_deref()
        .map(|s| s.to_string())
        .unwrap_or_else(|| fallback.to_string())
}

/// Helper for displaying transitions in error messages.
pub(crate) struct DisplayTransition(pub(crate) crate::lease::Transition);

impl fmt::Display for DisplayTransition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
