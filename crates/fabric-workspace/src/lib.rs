//! # fabric-workspace
//!
//! Persistent seat-lease management for Fabric.  A *workspace* is a
//! [ADR-0025](https://phenotype-fabric/adr/0025) instantiation of a
//! compiled [RoutePlan] — it binds the route's steps to a real seat
//! (a `LocalityTier` instance on a concrete host), tracks the seat's
//! state through a 5-state FSM, and persists the binding to disk so
//! that other Fabric-aware tools (thegent, ShareCLI, IDE-bridge) can
//! observe the running seat.
//!
//! ## Lifecycle
//!
//! ```text
//! Pending ──acquire──► Active ──release──► Completed
//!                       │                    ▲
//!                       │                    │
//!                       └──fail──► Failed    │
//!                       │                    │
//!                       └──revoke──► Revoked  │
//! ```
//!
//! The 5 states are persisted to `~/.local/share/fabric/workspaces/<id>.json`.
//! All state transitions are append-only — completed/failed/revoked workspaces
//! are never deleted, only marked terminal.
//!
//! ## Conflict detection
//!
//! A workspace is *conflicting* if it claims a seat that is already claimed by
//! another active workspace. The detection is a single pass over the active
//! set; conflict resolution is **operator-driven** (the
//! `fabric workspace list --conflicts` command shows the overlap, and
//! `fabric workspace release <id>` makes room).
//!
//! ## Stability
//!
//! R0 release: STABLE for the seat-lease model itself (Pending→Active→terminal),
//! EXPERIMENTAL for the on-disk format (may change before R1).

#![deny(missing_docs)]
#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]

pub mod error;
pub mod lease;
pub mod state;

pub use error::{Error, Result};
pub use lease::{Lease, LeaseId, LeaseState, Transition, TransitionError};
pub use state::Workspace;
pub use state::{Workspace, WorkspaceStore, WorkspaceStoreOptions};
