//! Multi-node federation for fabric-daemon.
//!
//! Allows multiple daemon instances to share topology information,
//! enabling a unified view across a fleet of nodes.
//!
//! Each node is assigned a federation_id prefix to avoid ID collisions
//! when merging topologies from different daemons.
//!
//! Federation is wired to the wire protocol via `federation_sync_request`
//! in `wire::protocol::process_message`; the handler in
//! `wire::handlers::handle_federation_sync_request` consumes the peer's
//! `TopologySnapshot`, calls `merge_topologies_from_json` against the
//! local snapshot read from `coordinator.topology_snapshot()`, and
//! returns the merged view as `federation_sync_response`.
//!
//! The background sync machinery in `discovery` and `state` is still only
//! reachable from integration tests; the wire arm IS the sync trigger
//! for cross-daemon topology exchange. The module-level `dead_code`
//! allow below is left in place so the binary target still compiles when
//! no caller invokes `sync_topology` / `FederationState::spawn_sync_thread`
//! directly -- they remain public API for the test target and a future
//! background-sync configuration that will be wired through
//! `FederationConfig`.
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
