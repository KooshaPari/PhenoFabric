//! Multi-node federation for fabric-daemon.
//!
//! Allows multiple daemon instances to share topology information,
//! enabling a unified view across a fleet of nodes.
//!
//! Each node is assigned a federation_id prefix to avoid ID collisions
//! when merging topologies from different daemons.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

use crate::coordinator::Coordinator;
use crate::config::FederationConfig;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Merge strategy for federated topologies.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum MergeStrategy {
    /// Merge all peers, deduplicating by prefixed node ID.
    MergeAll,
    /// Local topology wins on conflicts.
    LocalPrimary,
    /// Peer topology wins on conflicts.
    PeerPrimary,
}

impl Default for MergeStrategy {
    fn default() -> Self {
        Self::MergeAll
    }
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

// ---------------------------------------------------------------------------
// FederationState
// ---------------------------------------------------------------------------

/// Internal state shared between the sync thread and the coordinator.
pub struct FederationState {
    /// Unique prefix for this daemon (e.g. "node-1").
    pub federation_id: String,
    /// Cached peer topologies keyed by source address.
    pub peer_snapshots: Mutex<HashMap<String, TopologySnapshot>>,
    /// Last sync epoch (monotonically increasing).
    pub last_sync_epoch: AtomicU64,
    /// Whether the sync thread should stop.
    pub shutdown: Arc<AtomicBool>,
}

impl FederationState {
    /// Create a new federation state from config.
    pub fn new(config: &FederationConfig, federation_id: String) -> Self {
        Self {
            federation_id,
            peer_snapshots: Mutex::new(HashMap::new()),
            last_sync_epoch: AtomicU64::new(0),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get the last sync epoch.
    pub fn last_sync_epoch(&self) -> u64 {
        self.last_sync_epoch.load(Ordering::Relaxed)
    }

    /// Get the federation ID.
    pub fn federation_id(&self) -> &str {
        &self.federation_id
    }

    /// Cache a peer snapshot.
    pub fn cache_snapshot(&self, snapshot: TopologySnapshot) {
        let mut snapshots = self.peer_snapshots.lock().unwrap();
        snapshots.insert(snapshot.source_addr.clone(), snapshot);
    }

    /// Get all cached snapshots.
    pub fn cached_snapshots(&self) -> Vec<TopologySnapshot> {
        let snapshots = self.peer_snapshots.lock().unwrap();
        snapshots.values().cloned().collect()
    }

    /// Merge local topology with all cached peer snapshots.
    pub fn merge_topologies(
        &self,
        local_json: &str,
        strategy: &MergeStrategy,
    ) -> MergedTopology {
        let peers = self.cached_snapshots();
        merge_topologies_from_json(local_json, &peers, strategy)
    }
}

// ---------------------------------------------------------------------------
// Topology sync functions
// ---------------------------------------------------------------------------

/// Fetch topology from a peer daemon via TCP wire protocol.
///
/// Connects to `addr`, sends a `topology_request` message, and parses the
/// response. Returns `None` on timeout or connection failure.
pub fn sync_topology(addr: &str) -> Result<TopologySnapshot, FederationError> {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpStream;

    let timeout = Duration::from_secs(5);
    let stream = TcpStream::connect(addr).map_err(|e| {
        FederationError::ConnectionFailed(format!(
            "failed to connect to peer {addr}: {e}"
        ))
    })?;

    stream.set_read_timeout(Some(timeout)).ok();
    stream.set_write_timeout(Some(timeout)).ok();

    let mut writer = stream.try_clone().map_err(|e| {
        FederationError::ConnectionFailed(format!("clone stream: {e}"))
    })?;

    // Send topology request.
    let request = r#"{"type":"topology_request"}"#;
    writeln!(writer, "{request}").map_err(|e| {
        FederationError::ConnectionFailed(format!("write request: {e}"))
    })?;
    writer.flush().ok();

    // Read response.
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        let line = line.map_err(|e| {
            FederationError::ParseError(format!("read response: {e}"))
        })?;
        if line.is_empty() {
            continue;
        }
        return parse_topology_response(addr, &line);
    }

    Err(FederationError::NoResponse(format!(
        "peer {addr} returned empty response"
    )))
}

/// Parse a topology_response (probe_response) from a peer into a snapshot.
fn parse_topology_response(
    addr: &str,
    json: &str,
) -> Result<TopologySnapshot, FederationError> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| {
        FederationError::ParseError(format!("invalid JSON: {e}"))
    })?;

    let epoch = v.get("topology_epoch").and_then(|v| v.as_u64()).unwrap_or(0);
    let node_count = v.get("node_count").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let edge_count = v.get("edge_count").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

    let mut nodes = HashMap::new();
    if let Some(node_list) = v.get("nodes").and_then(|v| v.as_array()) {
        for n in node_list {
            let id = n.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if id.is_empty() {
                continue;
            }
            nodes.insert(
                id.clone(),
                NodeEntry {
                    id,
                    label: n.get("label").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    locality: n.get("locality").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    cap_count: n.get("cap_count").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
                    tags: n
                        .get("tags")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|t| t.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default(),
                    federation_id: String::new(), // filled by merge
                },
            );
        }
    }

    let mut edges = HashMap::new();
    if let Some(edge_list) = v.get("edges").and_then(|v| v.as_array()) {
        for e in edge_list {
            let id = e.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if id.is_empty() {
                continue;
            }
            edges.insert(
                id.clone(),
                EdgeEntry {
                    id,
                    from: e.get("from").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    to: e.get("to").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    locality: e.get("locality").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    federation_id: String::new(),
                },
            );
        }
    }

    Ok(TopologySnapshot {
        federation_id: String::new(),
        source_addr: addr.to_string(),
        topology_epoch: epoch,
        node_count,
        edge_count,
        nodes,
        edges,
    })
}

/// Merge a local topology (as JSON) with peer snapshots.
///
/// Nodes and edges from peers are prefixed with their `federation_id` to
/// avoid ID collisions across daemons.
pub fn merge_topologies_from_json(
    local_json: &str,
    peers: &[TopologySnapshot],
    strategy: &MergeStrategy,
) -> MergedTopology {
    let mut all_nodes: Vec<NodeEntry> = Vec::new();
    let mut all_edges: Vec<EdgeEntry> = Vec::new();
    let mut federation_ids: Vec<String> = Vec::new();
    let mut max_epoch: u64 = 0;

    // Parse local topology.
    let local: serde_json::Value = serde_json::from_str(local_json).unwrap_or_default();
    let local_epoch = local
        .get("topology_epoch")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    max_epoch = max_epoch.max(local_epoch);

    if let Some(node_list) = local.get("nodes").and_then(|v| v.as_array()) {
        for n in node_list {
            let id = n.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if id.is_empty() {
                continue;
            }
            all_nodes.push(NodeEntry {
                id: id.clone(),
                label: n.get("label").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                locality: n.get("locality").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                cap_count: n.get("cap_count").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
                tags: n
                    .get("tags")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|t| t.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
                federation_id: String::new(),
            });
        }
    }

    if let Some(edge_list) = local.get("edges").and_then(|v| v.as_array()) {
        for e in edge_list {
            let id = e.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if id.is_empty() {
                continue;
            }
            all_edges.push(EdgeEntry {
                id,
                from: e.get("from").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                to: e.get("to").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                locality: e.get("locality").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                federation_id: String::new(),
            });
        }
    }

    // Merge peer snapshots.
    for peer in peers {
        if peer.federation_id.is_empty() {
            continue;
        }
        federation_ids.push(peer.federation_id.clone());
        max_epoch = max_epoch.max(peer.topology_epoch);

        match strategy {
            MergeStrategy::MergeAll => {
                // Prefix peer IDs to avoid collisions.
                for (_, mut node) in peer.nodes.clone() {
                    node.id = format!("{}/{}", peer.federation_id, node.id);
                    node.federation_id = peer.federation_id.clone();
                    all_nodes.push(node);
                }
                for (_, mut edge) in peer.edges.clone() {
                    edge.id = format!("{}/{}", peer.federation_id, edge.id);
                    edge.from = format!("{}/{}", peer.federation_id, edge.from);
                    edge.to = format!("{}/{}", peer.federation_id, edge.to);
                    edge.federation_id = peer.federation_id.clone();
                    all_edges.push(edge);
                }
            }
            MergeStrategy::LocalPrimary => {
                // Only add peer nodes that don't exist locally (by unprefixed name).
                let local_ids: std::collections::HashSet<&str> = all_nodes
                    .iter()
                    .map(|n| n.id.as_str())
                    .collect();
                for (_, mut node) in peer.nodes.clone() {
                    if !local_ids.contains(node.id.as_str()) {
                        node.id = format!("{}/{}", peer.federation_id, node.id);
                        node.federation_id = peer.federation_id.clone();
                        all_nodes.push(node);
                    }
                }
                let local_edge_ids: std::collections::HashSet<&str> = all_edges
                    .iter()
                    .map(|e| e.id.as_str())
                    .collect();
                for (_, mut edge) in peer.edges.clone() {
                    if !local_edge_ids.contains(edge.id.as_str()) {
                        edge.id = format!("{}/{}", peer.federation_id, edge.id);
                        edge.from = format!("{}/{}", peer.federation_id, edge.from);
                        edge.to = format!("{}/{}", peer.federation_id, edge.to);
                        edge.federation_id = peer.federation_id.clone();
                        all_edges.push(edge);
                    }
                }
            }
            MergeStrategy::PeerPrimary => {
                // Peer nodes overwrite local nodes with the same base name.
                // For simplicity in this merged view, we always add the peer version
                // (prefixed) since the local node has no prefix.
                for (_, mut node) in peer.nodes.clone() {
                    node.id = format!("{}/{}", peer.federation_id, node.id);
                    node.federation_id = peer.federation_id.clone();
                    all_nodes.push(node);
                }
                for (_, mut edge) in peer.edges.clone() {
                    edge.id = format!("{}/{}", peer.federation_id, edge.id);
                    edge.from = format!("{}/{}", peer.federation_id, edge.from);
                    edge.to = format!("{}/{}", peer.federation_id, edge.to);
                    edge.federation_id = peer.federation_id.clone();
                    all_edges.push(edge);
                }
            }
        }
    }

    MergedTopology {
        nodes: all_nodes,
        edges: all_edges,
        peer_count: peers.len(),
        max_epoch,
        federation_ids,
    }
}

// ---------------------------------------------------------------------------
// Sync loop
// ---------------------------------------------------------------------------

/// Spawn the federation sync thread.
///
/// Periodically fetches topology from all configured peers and caches the
/// results. The coordinator can then merge them on demand.
pub fn spawn_sync_thread(
    state: Arc<FederationState>,
    config: FederationConfig,
) {
    let interval = Duration::from_secs(config.sync_interval_s);
    let shutdown = state.shutdown.clone();

    thread::spawn(move || {
        info!(
            interval_s = config.sync_interval_s,
            peers = config.peers.len(),
            "federation sync thread started"
        );

        loop {
            if shutdown.load(Ordering::Relaxed) {
                info!("federation sync thread stopping");
                break;
            }

            for peer_addr in &config.peers {
                match sync_topology(peer_addr) {
                    Ok(mut snapshot) => {
                        snapshot.federation_id = state.federation_id.clone();
                        debug!(
                            peer = %peer_addr,
                            epoch = snapshot.topology_epoch,
                            nodes = snapshot.node_count,
                            "fetched peer topology"
                        );
                        state.cache_snapshot(snapshot);
                    }
                    Err(e) => {
                        warn!(peer = %peer_addr, error = %e, "failed to sync peer topology");
                    }
                }
            }

            state
                .last_sync_epoch
                .fetch_add(1, Ordering::Relaxed);

            thread::sleep(interval);
        }
    });
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_peer_snapshot(
        federation_id: &str,
        addr: &str,
        epoch: u64,
        node_names: &[&str],
    ) -> TopologySnapshot {
        let mut nodes = HashMap::new();
        for name in node_names {
            nodes.insert(
                name.to_string(),
                NodeEntry {
                    id: name.to_string(),
                    label: format!("Label {name}"),
                    locality: "LAN".into(),
                    cap_count: 1,
                    tags: vec![],
                    federation_id: federation_id.to_string(),
                },
            );
        }
        TopologySnapshot {
            federation_id: federation_id.to_string(),
            source_addr: addr.to_string(),
            topology_epoch: epoch,
            node_count: node_names.len(),
            edge_count: 0,
            nodes,
            edges: HashMap::new(),
        }
    }

    #[test]
    fn merge_all_combines_nodes() {
        let local_json = r#"{
            "type": "probe_response",
            "topology_epoch": 1,
            "topology_name": "local",
            "node_count": 1,
            "edge_count": 0,
            "nodes": [{"id": "local-a", "label": "A", "locality": "LAN", "cap_count": 0, "tags": []}],
            "edges": []
        }"#;

        let peer = make_peer_snapshot("peer-1", "10.0.0.2:9400", 2, &["remote-x"]);

        let result = merge_topologies_from_json(
            local_json,
            &[peer],
            &MergeStrategy::MergeAll,
        );

        assert_eq!(result.nodes.len(), 2, "should have 1 local + 1 peer node");
        assert_eq!(result.max_epoch, 2);
        assert_eq!(result.peer_count, 1);
        assert!(result.federation_ids.contains(&"peer-1".to_string()));

        // Peer node should be prefixed.
        let peer_node = result
            .nodes
            .iter()
            .find(|n| n.federation_id == "peer-1")
            .unwrap();
        assert!(peer_node.id.starts_with("peer-1/"));
    }

    #[test]
    fn merge_local_primary_skips_duplicate_names() {
        let local_json = r#"{
            "type": "probe_response",
            "topology_epoch": 1,
            "topology_name": "local",
            "node_count": 1,
            "edge_count": 0,
            "nodes": [{"id": "shared-node", "label": "Local", "locality": "LAN", "cap_count": 0, "tags": []}],
            "edges": []
        }"#;

        // Peer has a node with same base name "shared-node".
        let peer = make_peer_snapshot(
            "peer-2",
            "10.0.0.3:9400",
            5,
            &["shared-node"],
        );

        let result = merge_topologies_from_json(
            local_json,
            &[peer],
            &MergeStrategy::LocalPrimary,
        );

        // LocalPrimary should only keep the local copy for existing names.
        let local_nodes: Vec<&NodeEntry> = result
            .nodes
            .iter()
            .filter(|n| n.federation_id.is_empty())
            .collect();
        assert_eq!(local_nodes.len(), 1);
        assert_eq!(local_nodes[0].id, "shared-node");

        // Peer version should NOT be added (duplicate).
        let peer_nodes: Vec<&NodeEntry> = result
            .nodes
            .iter()
            .filter(|n| n.federation_id == "peer-2")
            .collect();
        assert_eq!(
            peer_nodes.len(),
            0,
            "peer node with same name should be skipped"
        );
    }

    #[test]
    fn merge_all_edges_are_prefixed() {
        let local_json = r#"{
            "type": "probe_response",
            "topology_epoch": 1,
            "topology_name": "local",
            "node_count": 0,
            "edge_count": 1,
            "nodes": [],
            "edges": [{"id": "edge-1", "from": "a", "to": "b", "locality": "LAN"}]
        }"#;

        let mut peer = TopologySnapshot {
            federation_id: "peer-3".into(),
            source_addr: "10.0.0.4:9400".into(),
            topology_epoch: 3,
            node_count: 0,
            ..Default::default()
        };
        peer.edges.insert(
            "edge-2".into(),
            EdgeEntry {
                id: "edge-2".into(),
                from: "x".into(),
                to: "y".into(),
                locality: "WAN".into(),
                federation_id: "peer-3".into(),
            },
        );

        let result = merge_topologies_from_json(
            local_json,
            &[peer],
            &MergeStrategy::MergeAll,
        );

        assert_eq!(result.edges.len(), 2);
        let peer_edge = result
            .edges
            .iter()
            .find(|e| e.federation_id == "peer-3")
            .unwrap();
        assert!(peer_edge.id.starts_with("peer-3/"));
        assert!(peer_edge.from.starts_with("peer-3/"));
        assert!(peer_edge.to.starts_with("peer-3/"));
    }

    #[test]
    fn merge_empty_peers() {
        let local_json = r#"{
            "type": "probe_response",
            "topology_epoch": 1,
            "topology_name": "local",
            "node_count": 1,
            "edge_count": 0,
            "nodes": [{"id": "a", "label": "A", "locality": "LAN", "cap_count": 0, "tags": []}],
            "edges": []
        }"#;

        let result = merge_topologies_from_json(
            local_json,
            &[],
            &MergeStrategy::MergeAll,
        );

        assert_eq!(result.nodes.len(), 1);
        assert_eq!(result.edges.len(), 0);
        assert_eq!(result.peer_count, 0);
        assert!(result.federation_ids.is_empty());
    }

    #[test]
    fn merge_multiple_peers() {
        let local_json = r#"{
            "type": "probe_response",
            "topology_epoch": 1,
            "topology_name": "local",
            "node_count": 1,
            "edge_count": 0,
            "nodes": [{"id": "local", "label": "L", "locality": "LAN", "cap_count": 0, "tags": []}],
            "edges": []
        }"#;

        let peer1 = make_peer_snapshot("node-a", "10.0.0.1:9400", 2, &["a1", "a2"]);
        let peer2 = make_peer_snapshot("node-b", "10.0.0.2:9400", 3, &["b1"]);

        let result = merge_topologies_from_json(
            local_json,
            &[peer1, peer2],
            &MergeStrategy::MergeAll,
        );

        assert_eq!(result.nodes.len(), 4, "1 local + 2 peer-1 + 1 peer-2");
        assert_eq!(result.max_epoch, 3);
        assert_eq!(result.peer_count, 2);
    }

    #[test]
    fn federation_state_cache_and_retrieve() {
        let state = FederationState::new(
            &FederationConfig::default(),
            "test-node".into(),
        );

        let snapshot = make_peer_snapshot("peer", "1.2.3.4:9400", 10, &["n1"]);
        state.cache_snapshot(snapshot);

        let cached = state.cached_snapshots();
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].source_addr, "1.2.3.4:9400");
    }

    #[test]
    fn federation_state_sync_epoch() {
        let state = FederationState::new(
            &FederationConfig::default(),
            "node".into(),
        );
        assert_eq!(state.last_sync_epoch(), 0);
        state
            .last_sync_epoch
            .fetch_add(1, Ordering::Relaxed);
        assert_eq!(state.last_sync_epoch(), 1);
    }

    #[test]
    fn parse_topology_response_valid() {
        let json = r#"{
            "type": "probe_response",
            "status": "ok",
            "topology_epoch": 42,
            "topology_name": "test-topo",
            "node_count": 2,
            "edge_count": 1,
            "nodes": [
                {"id": "n1", "label": "Node 1", "locality": "LAN", "cap_count": 3, "tags": ["gpu"]},
                {"id": "n2", "label": "Node 2", "locality": "WAN", "cap_count": 0, "tags": []}
            ],
            "edges": [
                {"id": "e1", "from": "n1", "to": "n2", "locality": "WAN"}
            ]
        }"#;

        let snapshot = parse_topology_response("10.0.0.1:9400", json).unwrap();
        assert_eq!(snapshot.topology_epoch, 42);
        assert_eq!(snapshot.node_count, 2);
        assert_eq!(snapshot.nodes.len(), 2);
        assert_eq!(snapshot.edges.len(), 1);
        assert_eq!(snapshot.source_addr, "10.0.0.1:9400");
    }

    #[test]
    fn parse_topology_response_invalid_json() {
        let result = parse_topology_response("addr", "not-json");
        assert!(result.is_err());
    }

    #[test]
    fn parse_topology_response_empty_body() {
        let json = r#"{"type": "probe_response"}"#;
        let snapshot = parse_topology_response("addr", json).unwrap();
        assert_eq!(snapshot.topology_epoch, 0);
        assert!(snapshot.nodes.is_empty());
    }
}
