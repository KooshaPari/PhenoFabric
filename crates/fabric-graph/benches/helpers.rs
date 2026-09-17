//! Benchmark helpers: topology and node builders for criterion benchmarks.
//!
//! Provides reusable functions to construct topologies of various sizes
//! for benchmarking the graph compiler, negotiation, and failover paths.

use fabric_graph::model::{
    CapabilityRef, Edge, EdgeId, Intent, IntentId, IntentRequirements, LinkMetrics, Node, NodeId,
    Topology, TopologyMeta, TrustLevel,
};
use fabric_graph::{score_locality, FairnessPolicy, FairnessQueue, LocalityTier, TenantId};

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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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

// ---------------------------------------------------------------------------
// Enhanced helpers for new benchmarks
// ---------------------------------------------------------------------------

/// Build a topology with `n` nodes, each with a CPU capability descriptor.
/// Edges form a chain + skip graph. Some nodes carry GPU capabilities.
pub fn build_capability_topology(n: usize) -> Topology {
    let mut topo = Topology::new();
    topo.meta = TopologyMeta {
        name: format!("bench-capability-{n}"),
        ..Default::default()
    };

    let tiers = [
        LocalityTier::L1SameNuma,
        LocalityTier::L2CrossNumaShm,
        LocalityTier::L3PcieP2P,
        LocalityTier::L6Lan,
        LocalityTier::L6Lan,
    ];

    for i in 0..n {
        let tier = tiers[i % tiers.len()];
        let node_id = NodeId::new(format!("node-{i}"));
        let cap_id = format!("sha256:cpu-{i}");
        let mut node = Node::new(node_id, tier)
            .with_capability(CapabilityRef::new(cap_id).with_trust(TrustLevel::Attested));
        // Every 3rd node gets a GPU capability
        if i % 3 == 0 {
            node = node.with_capability(
                CapabilityRef::new(format!("sha256:gpu-{i}")).with_trust(TrustLevel::Attested),
            );
        }
        node.label = Some(format!("bench-node-{i}"));
        topo.add_node(node);
    }

    let node_ids: Vec<NodeId> = (0..n).map(|i| NodeId::new(format!("node-{i}"))).collect();

    // Chain edges
    for i in 0..n.saturating_sub(1) {
        let edge = Edge::new(
            EdgeId::new(format!("e-{}-{}", i, i + 1)),
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

    // Skip edges for alternative paths
    if n > 4 {
        for i in 0..n.saturating_sub(2) {
            let edge = Edge::new(
                EdgeId::new(format!("e-{}-{}-skip", i, i + 2)),
                node_ids[i].clone(),
                node_ids[i + 2].clone(),
                LocalityTier::L6Lan,
            );
            topo.add_edge(edge).unwrap();
        }
    }

    topo
}

/// Build a batch of `n` distinct intents, each targeting a different node.
pub fn build_intent_batch(n: usize) -> Vec<Intent> {
    (0..n)
        .map(|i| Intent {
            id: IntentId::new(),
            name: format!("bench-intent-{i}"),
            requirements: IntentRequirements::default(),
            preferred_node: Some(NodeId::new(format!("node-{}", i % 20))),
            min_trust: TrustLevel::Untrusted,
            tags: vec![],
            expires_at: None,
        })
        .collect()
}

/// Build a topology with `n` nodes, all at the same locality tier,
/// useful for scoring benchmarks where we want to measure scoring throughput
/// without locality variance dominating.
pub fn build_flat_topology(n: usize, tier: LocalityTier) -> Topology {
    let mut topo = Topology::new();
    topo.meta = TopologyMeta {
        name: format!("bench-flat-{n}"),
        ..Default::default()
    };

    for i in 0..n {
        let node = Node::new(NodeId::new(format!("node-{i}")), tier).with_capability(
            CapabilityRef::new(format!("sha256:cap-{i}")).with_trust(TrustLevel::Attested),
        );
        topo.add_node(node);
    }

    let node_ids: Vec<NodeId> = (0..n).map(|i| NodeId::new(format!("node-{i}"))).collect();
    for i in 0..n.saturating_sub(1) {
        let edge = Edge::new(
            EdgeId::new(format!("e-{}-{}", i, i + 1)),
            node_ids[i].clone(),
            node_ids[i + 1].clone(),
            tier,
        )
        .with_metrics(LinkMetrics {
            latency_us: Some(100.0),
            bandwidth_bps: Some(1_000_000_000),
            packet_loss: Some(0.0),
            jitter_us: Some(1.0),
        });
        topo.add_edge(edge).unwrap();
    }

    topo
}

/// Benchmark score_locality throughput across all nodes in a topology.
pub fn bench_score_locality_throughput(topo: &Topology, intent: &Intent) {
    for node in topo.nodes.values() {
        let _score = score_locality(node, &intent.requirements);
    }
}

/// Create a pre-filled fairness queue with `n` tenants, each having
/// made `acquire_count` acquire calls.
pub fn build_fairness_queue(tenant_count: usize, acquire_count: u32) -> FairnessQueue {
    let mut queue = FairnessQueue::new(FairnessPolicy::FairShare { weight: 1 });
    for t in 0..tenant_count {
        let tenant = TenantId::new(format!("tenant-{t}"));
        for _ in 0..acquire_count {
            queue.try_acquire(tenant.clone(), 1);
        }
    }
    queue
}
