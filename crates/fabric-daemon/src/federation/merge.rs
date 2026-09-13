//! Topology merge logic for federated daemons.

use super::types::{EdgeEntry, MergeStrategy, MergedTopology, NodeEntry, TopologySnapshot};

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
            let id = n.get("id").and_then(|v| v.as_str()).unwrap_or("" ).to_string();
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
                let local_ids: Vec<String> = all_nodes
                    .iter()
                    .map(|n| n.id.clone())
                    .collect();
                let local_ids_set: std::collections::HashSet<&str> = local_ids
                    .iter()
                    .map(|s| s.as_str())
                    .collect();
                for (_, mut node) in peer.nodes.clone() {
                    if !local_ids_set.contains(node.id.as_str()) {
                        node.id = format!("{}/{}", peer.federation_id, node.id);
                        node.federation_id = peer.federation_id.clone();
                        all_nodes.push(node);
                    }
                }
                let local_edge_ids: Vec<String> = all_edges
                    .iter()
                    .map(|e| e.id.clone())
                    .collect();
                let local_edge_ids_set: std::collections::HashSet<&str> = local_edge_ids
                    .iter()
                    .map(|s| s.as_str())
                    .collect();
                for (_, mut edge) in peer.edges.clone() {
                    if !local_edge_ids_set.contains(edge.id.as_str()) {
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
}
