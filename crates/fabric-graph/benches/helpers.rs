//! Benchmark helpers: topology and node builders for criterion benchmarks.
//!
//! Provides reusable functions to construct topologies of various sizes
//! for benchmarking the graph compiler, negotiation, and failover paths.

use fabric_graph::model::{
    Edge, EdgeId, Intent, IntentId, IntentRequirements, LinkMetrics, Node, NodeId, Topology,
    TopologyEpoch, TopologyMeta, TrustLevel,
};
use fabric_graph::LocalityTier;

/// Build a fully-connected topology with `n` nodes.
///
/// Every node gets a unique ID (`node-0`, `node-1`, ...) and edges are
/// created between every pair of nodes (directed, both directions) at L6 LAN.
pub fn build_mesh_topology(n: usize) -> Topology {
    let mut topo = Topology::new();
    topo.meta = TopologyMeta {
        name: format!("bench-mesh-{n}"),
        ..Default::default()
    };

    let tiers = [
        LocalityTier::L1SameNuma,
        LocalityTier::L2CrossNumaShm,
        LocalityTier::L3PcieP2P,
        LocalityTier::L5Loopback,
        LocalityTier::L6Lan,
    ];

    for i in 0..n {
        let tier = tiers[i % tiers.len()];
        let node_id = NodeId::new(format!("node-{i}"));
        let mut node = Node::new(node_id.clone(), tier);
        node.label = Some(format!("bench-node-{i}"));
        topo.add_node(node);
    }

    let node_ids: Vec<NodeId> = (0..n).map(|i| NodeId::new(format!("node-{i}"))).collect();

    // Create bidirectional edges between consecutive nodes (chain)
    for i in 0..n.saturating_sub(1) {
        let edge_id = EdgeId::new(format!("e-{}-{}", i, i + 1));
        let edge = Edge::new(
            edge_id,
            node_ids[i].clone(),
            node_ids[i + 1].clone(),
            LocalityTier::L6Lan,
        )
        .with_metrics(LinkMetrics {
            latency_us: Some(50.0 + (i as f64) * 10.0),
            bandwidth_bps: Some(1_000_000_000),
            packet_loss: Some(0.0),
            jitter_us: Some(2.0),
        });
        topo.add_edge(edge).unwrap();
    }

    // For larger topologies, also create some skip edges for alternative paths
    if n > 4 {
        for i in 0..n.saturating_sub(2) {
            let edge_id = EdgeId::new(format!("e-{}-{}-skip", i, i + 2));
            let edge = Edge::new(
                edge_id,
                node_ids[i].clone(),
                node_ids[i + 2].clone(),
                LocalityTier::L7Wan,
            );
            topo.add_edge(edge).unwrap();
        }
    }

    topo
}

/// Build a linear chain topology: node-0 -> node-1 -> ... -> node-(n-1).
///
/// Each consecutive pair is connected by a directed edge at L6 LAN with metrics.
pub fn build_chain_topology(n: usize) -> Topology {
    let mut topo = Topology::new();
    topo.meta = TopologyMeta {
        name: format!("bench-chain-{n}"),
        ..Default::default()
    };

    for i in 0..n {
        let tier = if i == 0 {
            LocalityTier::L2CrossNumaShm
        } else {
            LocalityTier::L6Lan
        };
        let node_id = NodeId::new(format!("node-{i}"));
        topo.add_node(Node::new(node_id, tier));
    }

    for i in 0..n.saturating_sub(1) {
        let edge_id = EdgeId::new(format!("e-{}-{}", i, i + 1));
        let edge = Edge::new(
            edge_id,
            NodeId::new(format!("node-{i}")),
            NodeId::new(format!("node-{}", i + 1)),
            LocalityTier::L6Lan,
        );
        topo.add_edge(edge).unwrap();
    }

    topo
}

/// Build a 4-node chain topology with edges at different locality tiers.
///
///   node-0 (L2) --L2--> node-1 (L3) --L6--> node-2 (L6) --L6--> node-3 (L7)
pub fn build_4node_chain() -> Topology {
    let mut topo = Topology::new();
    topo.meta = TopologyMeta {
        name: "bench-4node-chain".into(),
        ..Default::default()
    };

    let nodes = vec![
        ("node-0", LocalityTier::L2CrossNumaShm),
        ("node-1", LocalityTier::L3PcieP2P),
        ("node-2", LocalityTier::L6Lan),
        ("node-3", LocalityTier::L7Wan),
    ];

    for (id, tier) in &nodes {
        topo.add_node(Node::new(NodeId::new(*id), *tier));
    }

    let edges = vec![
        ("e-0-1", "node-0", "node-1", LocalityTier::L2CrossNumaShm),
        ("e-1-2", "node-1", "node-2", LocalityTier::L6Lan),
        ("e-2-3", "node-2", "node-3", LocalityTier::L6Lan),
    ];

    for (eid, from, to, tier) in &edges {
        topo.add_edge(Edge::new(
            EdgeId::new(*eid),
            NodeId::new(*from),
            NodeId::new(*to),
            *tier,
        ))
        .unwrap();
    }

    topo
}

/// Create a simple intent that accepts any node.
pub fn any_node_intent() -> Intent {
    Intent {
        id: IntentId::new(),
        name: "bench-any-node".into(),
        requirements: IntentRequirements::default(),
        preferred_node: None,
        min_trust: TrustLevel::Untrusted,
        tags: vec![],
        expires_at: None,
    }
}

/// Create an intent that requires a specific tag.
pub fn tag_intent(tag: &str) -> Intent {
    Intent {
        id: IntentId::new(),
        name: format!("bench-require-{tag}"),
        requirements: IntentRequirements {
            required_tags: vec![tag.to_string()],
            ..Default::default()
        },
        preferred_node: None,
        min_trust: TrustLevel::Untrusted,
        tags: vec![],
        expires_at: None,
    }
}

/// Create an intent with a preferred node hint.
pub fn preferred_node_intent(node_id: &str) -> Intent {
    Intent {
        id: IntentId::new(),
        name: "bench-preferred".into(),
        requirements: IntentRequirements::default(),
        preferred_node: Some(NodeId::new(node_id)),
        min_trust: TrustLevel::Untrusted,
        tags: vec![],
        expires_at: None,
    }
}
