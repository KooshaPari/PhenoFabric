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
pub mod model;
pub mod negotiation;
pub mod planner;
pub mod surface;
pub mod surface_ops;
pub mod lease_fsm;
pub mod decision;
pub mod score;

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
