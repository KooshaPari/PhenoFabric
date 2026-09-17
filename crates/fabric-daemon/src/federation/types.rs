//! Shared types for multi-node federation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Merge strategy for federated topologies.
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum MergeStrategy {
    /// Merge all peers, deduplicating by prefixed node ID.
    #[default]
    MergeAll,
    /// Local topology wins on conflicts.
    LocalPrimary,
    /// Peer topology wins on conflicts.
    PeerPrimary,
}

/// A simplified topology snapshot fetched from a peer daemon.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TopologySnapshot {
    /// The daemon's federation ID (prefix).
    pub federation_id: String,
    /// The address this snapshot was fetched from.
    pub source_addr: String,
    /// Topology epoch on the peer.
    pub topology_epoch: u64,
    /// Number of nodes reported by the peer.
    pub node_count: usize,
    /// Number of edges reported by the peer.
    pub edge_count: usize,
    /// Raw node entries (id -> label).
    pub nodes: HashMap<String, NodeEntry>,
    /// Raw edge entries (id -> edge).
    pub edges: HashMap<String, EdgeEntry>,
}

/// A single node entry from a peer.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeEntry {
    pub id: String,
    pub label: String,
    pub locality: String,
    pub cap_count: usize,
    pub tags: Vec<String>,
    pub federation_id: String,
}

/// A single edge entry from a peer.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EdgeEntry {
    pub id: String,
    pub from: String,
    pub to: String,
    pub locality: String,
    pub federation_id: String,
}

/// Merged topology result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedTopology {
    /// Combined nodes from all federated daemons.
    pub nodes: Vec<NodeEntry>,
    /// Combined edges from all federated daemons.
    pub edges: Vec<EdgeEntry>,
    /// Number of peers that contributed.
    pub peer_count: usize,
    /// The highest epoch seen across all peers.
    pub max_epoch: u64,
    /// Per-peer federation IDs that were merged.
    pub federation_ids: Vec<String>,
}

/// Status of federation, returned by the federation_status message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationStatus {
    pub enabled: bool,
    pub federation_id: String,
    pub peer_count: usize,
    pub sync_interval_s: u64,
    pub merge_strategy: MergeStrategy,
    pub last_sync_epoch: u64,
    pub peers: Vec<PeerStatus>,
}

/// Status of a single peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerStatus {
    pub addr: String,
    pub reachable: bool,
    pub last_sync_epoch: u64,
    pub last_sync_age_s: u64,
}

/// Errors that can occur during federation operations.
#[derive(Debug, thiserror::Error)]
pub enum FederationError {
    #[error("connection failed: {0}")]
    ConnectionFailed(String),
    #[error("no response: {0}")]
    NoResponse(String),
    #[error("parse error: {0}")]
    ParseError(String),
}
