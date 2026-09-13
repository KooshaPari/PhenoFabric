//! Core domain model for Fabric's topology graph.
//!
//! The graph consists of:
//! - **Nodes** -- physical machines or virtual execution contexts
//! - **Edges** -- links between nodes with locality tier and link metrics
//! - **Capabilities** -- what each node can do (attached to nodes)
//! - **Intents** -- placement requirements from a caller
//! - **RoutePlans** -- compiler output: a sequence of hops satisfying an intent

mod edges;
mod ids;
mod intent;
mod nodes;
mod routing;
mod types;

// Re-export all public items from sub-modules for backward compatibility.
pub use edges::*;
pub use ids::*;
pub use intent::*;
pub use nodes::*;
pub use routing::*;
pub use types::*;
