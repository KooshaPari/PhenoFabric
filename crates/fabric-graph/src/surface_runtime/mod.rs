//! Surface-plane runtime registry (PF-WP-030, spec 024).
//!
//! Provides an in-memory registry of active [`crate::surface::SurfaceLease`]s so the failover
//! hook can invalidate leases when a node fails, and so the wire transport
//! can enumerate active surfaces.
//!
//! ## Design rules
//!
//! - Single-threaded, in-memory; no persistence (R3 concern).
//! - `insert` is idempotent (overwrites if handle already present).
//! - `notify_node_failure` returns the list of invalidations — the caller
//!   is responsible for external notification (event log emission, Go-side
//!   handle drop).
//! - `bind_with_topology` (in `surface_ops`) replaces the
//!   `derive_endpoint_for_step` placeholder with a real topology lookup.

mod registry;
mod types;

pub use registry::SurfaceRegistry;
pub use types::*;
