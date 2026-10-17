//! Multi-node federation for fabric-daemon.
//!
//! Allows multiple daemon instances to share topology information,
//! enabling a unified view across a fleet of nodes.
//!
//! Each node is assigned a federation_id prefix to avoid ID collisions
//! when merging topologies from different daemons.
//!
//! Federation is not wired to any wire message type yet (the daemon has no
//! federation arm in `wire/protocol.rs`), so in the binary target these items
//! are unreferenced. They are kept compiling because `lib.rs` exports the
//! module as public API and its tests cover the topology-merge logic.
//! Remove this allow when the first federation wire arm lands.
#![allow(dead_code, unused_imports)]

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
