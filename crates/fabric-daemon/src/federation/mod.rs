//! Multi-node federation for fabric-daemon.
//!
//! Allows multiple daemon instances to share topology information,
//! enabling a unified view across a fleet of nodes.
//!
//! Each node is assigned a federation_id prefix to avoid ID collisions
//! when merging topologies from different daemons.

mod discovery;
mod merge;
mod state;
pub mod types;

pub use discovery::{spawn_sync_thread, sync_topology};
pub use merge::merge_topologies_from_json;
pub use state::FederationState;
pub use types::{
    EdgeEntry, FederationError, FederationStatus, MergeStrategy, MergedTopology, NodeEntry,
    PeerStatus, TopologySnapshot,
};
