//! # fabric-graph
//!
//! Topology graph model and route compiler for Phenotype Fabric.
//!
//! This crate is the **graph-native** core. It is read-only with respect to
//! execution; it produces route plans that the runtime then enacts.
//!
//! ## Modules
//!
//! - [`model`]: graph types — nodes, edges, capabilities, intents, topology epoch
//! - [`builder`]: ergonomic constructors and builders for graphs and intents
//! - [`negotiation`]: the negotiation algorithm — score routes against intents
//! - [`compile`]: produce route plans from intents against a topology
//! - [`score`]: scoring functions (locality, latency, capability, trust)
//! - [`planner`]: long-horizon planner for batch/sequence intents
//! - [`failover`]: re-plan around failed nodes when a route's steps fail (PF-WP-021, spec 019)
//! - [`surface`] / [`surface_ops`] / [`lease_fsm`]: surface plane types + lease FSM (PF-WP-015, spec 019)
//! - [`decision`]: Admit / AdmitWithNotes / Reject + Severity reducer (mirrors cmd/checker/decision.go)
//! - [`leases`]: single integration entry point that wires `failover::replan` to the surface plane (PF-WP-022, spec 020)
//! - [`leases_fairness`]: multi-tenant fairness queue + `pardon()` operator escape hatch (PF-WP-022 v2, spec 022)
//!
//! ## Non-negotiable invariants (PF-FR-002..005)
//!
//! 1. Every edge carries a locality tier (L0..L8). An edge without one is invalid.
//! 2. Every node carries a signed capability descriptor (or None for "unprobed").
//! 3. The topology epoch is a first-class property of every route plan. Plans
//!    pin to the epoch they were compiled against and refuse to be enacted on
//!    a different epoch without a re-negotiation step.
//! 4. The graph is the *truth* of physical layout. Anything not in the graph
//!    does not exist for placement.

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]

// Re-export fabric-capability's LocalityTier so callers don't need a separate dep.
pub use fabric_capability::LocalityTier;

pub mod builder;
pub mod compile;
pub mod failover;
pub mod leases_fairness;
pub mod model;
pub mod negotiation;
pub mod planner;
pub mod decision;
pub mod lease_fsm;
pub mod leases;
pub mod score;
pub mod surface;
pub mod surface_ops;

pub use crate::leases_fairness::{
    FairnessDecision, FairnessPolicy, FairnessQueue, FairnessSnapshot,
    PardonError, TenantAccounting, TenantId, DenyReason, pardon,
};

pub use crate::builder::IntentBuilder;
pub use crate::compile::{compile, compile_all};
pub use crate::model::{
    Edge, EdgeId, Intent, IntentRequirements, LinkMetrics, Node, NodeId,
    RoutePlan, RoutePlanId, RouteStep, Score, ScoreBreakdown, Topology,
    TopologyEpoch, TopologyMeta, TrustLevel,
};
pub use crate::negotiation::negotiate;
pub use crate::planner::plan_sequence;
pub use crate::score::ScoringWeights;
