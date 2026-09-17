//! Topology exchange and merge helpers for the two-node harness.
//!
//! Uses the real `sync_topology()` TCP code path for deterministic
//! federation testing, then merges results via `merge_topologies_from_json`.

use super::DaemonInstance;
use fabric_daemon::federation::{sync_topology, MergeStrategy, MergedTopology, TopologySnapshot};

/// Fetch topology from `daemon_b` via TCP, then merge it into `daemon_a`'s
/// local topology view.
///
/// Returns a [`MergedTopology`] containing nodes and edges from both daemons.
///
/// # Arguments
///
/// * `daemon_a` -- the "local" daemon whose topologySnapshot serves as the
///   base for the merge.
/// * `daemon_b` -- the "peer" daemon whose topology is fetched via TCP.
/// * `strategy` -- merge strategy (`MergeAll`, `LocalPrimary`, or
///   `PeerPrimary`).
pub fn exchange_and_merge(
    daemon_a: &DaemonInstance,
    daemon_b: &DaemonInstance,
    strategy: &MergeStrategy,
) -> MergedTopology {
    // Fetch daemon_b's topology via the real TCP sync code path.
    let mut peer_snapshot: TopologySnapshot =
        sync_topology(&daemon_b.addr).expect("sync_topology to daemon_b should succeed");

    // The sync_topology response doesn't include a federation_id (it's set
    // by the federation sync thread in production). For testing, derive it
    // from the daemon's address so the merge logic can identify peer nodes.
    if peer_snapshot.federation_id.is_empty() {
        peer_snapshot.federation_id = daemon_b.addr.replace(":", "-").to_string();
    }

    // Get daemon_a's local topology as JSON (the probe_response format).
    let local_json = daemon_a.coordinator.topology_snapshot();

    // Merge.
    fabric_daemon::federation::merge_topologies_from_json(&local_json, &[peer_snapshot], strategy)
}

/// Assert that a merged topology has the expected node and edge counts.
///
/// # Panics
///
/// Panics with a descriptive message if counts don't match.
pub fn assert_merged_topology(
    merged: &MergedTopology,
    expected_node_count: usize,
    expected_edge_count: usize,
) {
    assert_eq!(
        merged.nodes.len(),
        expected_node_count,
        "expected {} nodes in merged topology, got {} (nodes: {:?})",
        expected_node_count,
        merged.nodes.len(),
        merged.nodes.iter().map(|n| &n.id).collect::<Vec<_>>(),
    );
    assert_eq!(
        merged.edges.len(),
        expected_edge_count,
        "expected {} edges in merged topology, got {} (edges: {:?})",
        expected_edge_count,
        merged.edges.len(),
        merged.edges.iter().map(|e| &e.id).collect::<Vec<_>>(),
    );
}

/// Assert that the merged topology contains nodes from at least
/// `expected_federation_count` distinct federation IDs.
pub fn assert_federation_ids(merged: &MergedTopology, expected_min_count: usize) {
    assert!(
        merged.federation_ids.len() >= expected_min_count,
        "expected at least {} federation IDs, got {:?}",
        expected_min_count,
        merged.federation_ids,
    );
}

/// Assert that the merged topology's max epoch is at least `min_epoch`.
pub fn assert_min_epoch(merged: &MergedTopology, min_epoch: u64) {
    assert!(
        merged.max_epoch >= min_epoch,
        "expected max_epoch >= {min_epoch}, got {}",
        merged.max_epoch,
    );
}

#[cfg(test)]
mod tests {
    use super::super::{build_2node_topology, start_two_daemons, stop_daemon};
    use super::*;

    #[test]
    fn exchange_and_merge_combines_two_nodes() {
        let (a, b) = start_two_daemons();

        let merged = exchange_and_merge(&a, &b, &MergeStrategy::MergeAll);
        // a has "node_a", b has "node_b" => 2 nodes total.
        assert_merged_topology(&merged, 2, 0);

        stop_daemon(a);
        stop_daemon(b);
    }

    #[test]
    fn exchange_and_merge_with_edges() {
        let (a, b) = start_two_daemons();

        // Give daemon_a a full 2-node topology with an edge.
        let topo = build_2node_topology();
        a.coordinator.set_topology(topo).unwrap();

        let merged = exchange_and_merge(&a, &b, &MergeStrategy::MergeAll);
        // a: "node_a" + "node_b" + edge, b: "node_b" => 3 nodes (peer prefixed), 1 edge.
        assert_merged_topology(&merged, 3, 1);

        stop_daemon(a);
        stop_daemon(b);
    }

    #[test]
    fn assert_federation_ids_catches_missing() {
        let (a, b) = start_two_daemons();

        let merged = exchange_and_merge(&a, &b, &MergeStrategy::MergeAll);
        assert_federation_ids(&merged, 1);

        stop_daemon(a);
        stop_daemon(b);
    }
}
