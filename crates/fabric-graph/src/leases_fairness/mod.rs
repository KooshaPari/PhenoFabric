//! Multi-tenant fairness for surface leases (PF-WP-022 v2, spec 022).
//!
//! Extends the single-tenant `rebind_or_fail` (spec 020) with a fairness
//! queue that decides *who gets capacity next* when many tenants contend
//! for the same host resources. This is the R1 closeout — closes the
//! "last 5%" of R1 by adding the multi-tenant arbitration that the
//! R2 surface-plane runtime will need.
//!
//! ## Reader's guide (ADR-0028)
//!
//! Read these modules before changing this file:
//!
//! - `crate::leases` — `rebind_or_fail`, `SurfaceLease`, `LeaseState`
//! - `crate::surface` — `SurfaceSpec`, `SurfaceSpecError`
//! - `crate::surface_ops` — `new_lease`, `is_terminal`
//!
//! ## Algorithm summary (spec 022 §4)
//!
//! - **Fifo**: pure round-robin; rotation order = insertion order.
//! - **FairShare**: pick tenant with max deficit; ties broken FIFO.
//! - **PriorityWeighted**: serve by priority group; within group FIFO.
//!   Deny if higher-priority tenant is waiting.
//! - **WeightedRoundRobin**: each tenant gets `weight` slots per rotation;
//!   rotation cursor advances after each grant.

mod pardon;
mod queue;
mod types;

pub use pardon::*;
pub use queue::FairnessQueue;
pub use types::*;
