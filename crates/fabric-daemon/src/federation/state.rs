//! Internal federation state shared between the sync thread and the coordinator.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use super::merge::merge_topologies_from_json;
use super::types::{MergeStrategy, MergedTopology, TopologySnapshot};
use crate::config::FederationConfig;

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
    pub fn new(_config: &FederationConfig, federation_id: String) -> Self {
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
    pub fn merge_topologies(&self, local_json: &str, strategy: &MergeStrategy) -> MergedTopology {
        let peers = self.cached_snapshots();
        merge_topologies_from_json(local_json, &peers, strategy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_peer_snapshot(
        federation_id: &str,
        addr: &str,
        epoch: u64,
        node_names: &[&str],
    ) -> TopologySnapshot {
        use super::super::types::NodeEntry;
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
    fn federation_state_cache_and_retrieve() {
        let state = FederationState::new(&FederationConfig::default(), "test-node".into());

        let snapshot = make_peer_snapshot("peer", "1.2.3.4:9400", 10, &["n1"]);
        state.cache_snapshot(snapshot);

        let cached = state.cached_snapshots();
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].source_addr, "1.2.3.4:9400");
    }

    #[test]
    fn federation_state_sync_epoch() {
        let state = FederationState::new(&FederationConfig::default(), "node".into());
        assert_eq!(state.last_sync_epoch(), 0);
        state.last_sync_epoch.fetch_add(1, Ordering::Relaxed);
        assert_eq!(state.last_sync_epoch(), 1);
    }
}
