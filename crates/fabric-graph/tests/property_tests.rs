//! Property-based tests for the Fabric graph compiler and route planning.
//!
//! Uses `proptest` to verify structural invariants that must hold for
//! arbitrary topologies and intents.

use fabric_graph::compile::{compile, compile_all};
use fabric_graph::multihop::{compile_multihop, builtin_stages};
use fabric_graph::model::{
    Edge, EdgeId, Intent, IntentId, IntentRequirements, Node, NodeId,
    Topology, TopologyMeta,
};
use fabric_graph::LocalityTier;
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Strategies for generating random topology graphs
// ---------------------------------------------------------------------------

fn arb_locality_tier() -> impl Strategy<Value = LocalityTier> {
    prop_oneof![
        Just(LocalityTier::L0SameProcess),
        Just(LocalityTier::L1SameNuma),
        Just(LocalityTier::L2CrossNumaShm),
        Just(LocalityTier::L3PcieP2P),
        Just(LocalityTier::L4Rdma),
        Just(LocalityTier::L5Loopback),
        Just(LocalityTier::L6Lan),
        Just(LocalityTier::L7Wan),
        Just(LocalityTier::L8Oob),
    ]
}

/// Generate a topology with 1..=10 nodes and 0..=15 edges.
fn arb_topology() -> impl Strategy<Value = Topology> {
    (1u32..=10, 0u32..=15).prop_flat_map(|(n_nodes, n_edges)| {
        let node_ids: Vec<String> = (0..n_nodes).map(|i| format!("node-{i}")).collect();
        (
            Just(node_ids.clone()),
            prop::collection::vec(arb_locality_tier(), n_nodes as usize..=n_nodes as usize),
            prop::collection::vec(
                (0u32..n_nodes, 0u32..n_nodes, arb_locality_tier()),
                0..=n_edges as usize,
            ),
        )
    }).prop_map(|(node_ids, tiers, edge_defs)| {
        let mut topo = Topology::new();
        topo.meta = TopologyMeta {
            name: "proptest-topo".into(),
            ..Default::default()
        };
        // Epoch starts at 0, add_node bumps it.
        for (id, tier) in node_ids.iter().zip(tiers) {
            topo.add_node(Node::new(NodeId::new(id), tier));
        }
        // Add edges (only between existing nodes, dedup by edge ID).
        let mut edge_counter = 0u32;
        for (from_idx, to_idx, tier) in edge_defs {
            let from_id = &node_ids[from_idx as usize % node_ids.len()];
            let to_id = &node_ids[to_idx as usize % node_ids.len()];
            if from_id == to_id {
                continue;
            }
            let edge_id = EdgeId::new(format!("e-{edge_counter}"));
            edge_counter += 1;
            let _ = topo.add_edge(Edge::new(
                edge_id,
                NodeId::new(from_id),
                NodeId::new(to_id),
                tier,
            ));
        }
        topo
    })
}

fn arb_intent_no_expiry() -> impl Strategy<Value = Intent> {
    "[a-z]{1,10}".prop_map(|name| Intent {
        id: IntentId::new(),
        name,
        requirements: IntentRequirements::default(),
        preferred_node: None,
        min_trust: fabric_graph::model::TrustLevel::default(),
        expires_at: None,
        tags: vec![],
    })
}

// ---------------------------------------------------------------------------
// Property: compiling a valid topology always produces a plan or a clear error
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn compile_always_terminates(topo in arb_topology(), intent in arb_intent_no_expiry()) {
        // compile() must return either Ok(RoutePlan) or Err(CompileError), never panic.
        let result = compile(&topo, &intent);
        match result {
            Ok(plan) => {
                // A successful plan must have at least one step.
                prop_assert!(!plan.steps.is_empty(), "plan must have at least one step");
            }
            Err(e) => {
                // The error must be one of the known variants.
                let msg = format!("{e}");
                prop_assert!(
                    msg.contains("no candidate")
                        || msg.contains("empty")
                        || msg.contains("expired")
                        || msg.contains("epoch"),
                    "unexpected error: {e}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Property: route plans always reference nodes that exist in the topology
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn plan_references_valid_nodes(topo in arb_topology(), intent in arb_intent_no_expiry()) {
        if let Ok(plan) = compile(&topo, &intent) {
            for step in &plan.steps {
                prop_assert!(
                    topo.nodes.contains_key(&step.node),
                    "plan step references unknown node {:?}",
                    step.node
                );
                if let Some(ref edge_id) = step.via_edge {
                    prop_assert!(
                        topo.edges.contains_key(edge_id),
                        "plan step references unknown edge {:?}",
                        edge_id
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Property: compile_all returns candidates or empty, never panics
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn compile_all_always_terminates(topo in arb_topology(), intent in arb_intent_no_expiry()) {
        let result = compile_all(&topo, &intent);
        prop_assert!(result.is_ok() || result.is_err());
    }
}

// ---------------------------------------------------------------------------
// Property: multihop compile with N nodes produces plans with <= N steps
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn multihop_steps_bounded_by_node_count(
        topo in arb_topology(),
        src in "[a-z]{1,5}",
        dst in "[a-z]{1,5}",
    ) {
        // Only test if source and destination exist in the topology
        let src_id = NodeId::new(&src);
        let dst_id = NodeId::new(&dst);
        if !topo.nodes.contains_key(&src_id) || !topo.nodes.contains_key(&dst_id) {
            return Ok(());
        }
        if src == dst {
            return Ok(());
        }

        let catalog = builtin_stages();
        let intent = Intent {
            id: IntentId::new(),
            name: "multihop-test".into(),
            requirements: IntentRequirements::default(),
            preferred_node: None,
            min_trust: fabric_graph::model::TrustLevel::default(),
            expires_at: None,
            tags: vec![],
        };

        let result = compile_multihop(&topo, &src_id, &dst_id, &intent, &catalog);
        match result {
            Ok(multihop) => {
                let n = topo.nodes.len();
                prop_assert!(
                    multihop.primary.steps.len() <= n,
                    "multihop plan has {} steps but topology has {} nodes",
                    multihop.primary.steps.len(),
                    n
                );
                // Plan must have at least 2 steps (source + destination).
                prop_assert!(
                    multihop.primary.steps.len() >= 2,
                    "multihop plan must have >= 2 steps, got {}",
                    multihop.primary.steps.len()
                );
            }
            Err(_) => {
                // No path found or other error — acceptable.
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Property: plan references all valid nodes (added a node -> no worse routes)
// This is tested by verifying that compilation results are consistent.
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn plan_nodes_are_subset_of_topology(topo in arb_topology(), intent in arb_intent_no_expiry()) {
        if let Ok(plan) = compile(&topo, &intent) {
            for step in &plan.steps {
                prop_assert!(
                    topo.nodes.contains_key(&step.node),
                    "plan references node {:?} not in topology",
                    step.node
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Property: adding nodes never decreases the number of valid routes
// (tested by compiling before and after adding a node)
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn adding_nodes_preserves_validity(
        topo in arb_topology(),
        extra_tier in arb_locality_tier(),
        intent in arb_intent_no_expiry(),
    ) {
        // Compile with original topology.
        let result_before = compile(&topo, &intent);

        // Add a new isolated node and compile again.
        let mut topo_plus = topo.clone();
        let new_node_id = NodeId::new(format!("extra-{}", topo_plus.nodes.len()));
        topo_plus.add_node(Node::new(new_node_id.clone(), extra_tier));

        let result_after = compile(&topo_plus, &intent);

        // If original succeeded, the expanded topology should also succeed
        // (the new node doesn't break existing routes).
        if let Ok(_plan_before) = result_before {
            prop_assert!(
                result_after.is_ok(),
                "compiling after adding a node should still succeed"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Property: removing an edge never increases the number of valid routes
// (tested by checking that the number of compile candidates doesn't increase)
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn removing_edge_does_not_increase_candidates(
        topo in arb_topology(),
        intent in arb_intent_no_expiry(),
    ) {
        if topo.edges.is_empty() {
            return Ok(());
        }

        // Compile with original topology.
        let candidates_before = compile_all(&topo, &intent)
            .map(|r| r.candidates.len())
            .unwrap_or(0);

        // Remove one edge.
        let edge_to_remove = topo.edges.keys().next().unwrap().clone();
        let mut topo_minus = topo.clone();
        topo_minus.edges.remove(&edge_to_remove);
        // Bump epoch manually to match.
        topo_minus.epoch.0 += 1;

        let candidates_after = compile_all(&topo_minus, &intent)
            .map(|r| r.candidates.len())
            .unwrap_or(0);

        prop_assert!(
            candidates_after <= candidates_before,
            "removing an edge should not increase candidates: before={}, after={}",
            candidates_before,
            candidates_after
        );
    }
}
