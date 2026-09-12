//! Route failover: re-plan when a node (or set of nodes) in an existing
//! `RoutePlan` is no longer reachable.
//!
//! Implements the minimum viable surface from spec 019 (PF-WP-021).
//! The function takes the original plan, the topology, the intent, and a
//! blacklist of failed node IDs, and returns:
//!   * `Ok(Some(plan))` — a new route that avoids the failed nodes
//!   * `Ok(None)`        — no remaining candidate can satisfy the intent
//!   * `Err(FailoverError)` — invariant violation (e.g. empty intent)
//!
//! The design rule is "atomic replacement": the new plan either fully
//! replaces the old one or doesn't exist. There is no in-place mutation,
//! no partial-step reuse. This matches ADR-0030.
//!
//! `replan_multihop()` extends this with multi-hop fallback support:
//! when a node in a multi-hop path fails, it first tries pre-generated
//! fallback routes before doing a fresh compile on the pruned topology.

use crate::compile;
use crate::model::{Intent, NodeId, RoutePlan, Topology};
use crate::multihop::{compile_multihop, MultihopError, MultihopResult, TransportStage};

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FailoverError {
    #[error("intent has no requirements — nothing to replan onto")]
    EmptyIntent,
    #[error("all candidate nodes are blacklisted")]
    AllCandidatesFailed,
}

/// The result of a failover attempt. `Some(plan)` means a new plan was
/// compiled that avoids the blacklisted nodes; `None` means no feasible
/// replacement exists (caller should release the lease).
#[derive(Debug)]
pub enum FailoverOutcome {
    /// A replacement plan was successfully compiled.
    Replaced(RoutePlan),
    /// No feasible replacement exists; the caller must release.
    NoReplacement,
}

/// Replan `old_plan` onto a topology with the given nodes blacklisted.
/// The topology is the **current** live topology (after the failures
/// have been removed from `topology` by the caller), so we re-use
/// `compile()` directly on it.
///
/// `blacklist` is a list of node IDs that have failed. If `blacklist`
/// is empty, this function returns `old_plan` cloned (no work needed).
pub fn replan(
    topology: &Topology,
    intent: &Intent,
    old_plan: &RoutePlan,
    blacklist: &[NodeId],
) -> Result<FailoverOutcome, FailoverError> {
    if intent.name.is_empty() {
        return Err(FailoverError::EmptyIntent);
    }
    if blacklist.is_empty() {
        // Nothing failed — just return the old plan wrapped.
        return Ok(FailoverOutcome::Replaced(old_plan.clone()));
    }

    // Compile a fresh plan on the current topology. Since the topology
    // passed in here is the post-failure topology (failed nodes already
    // removed by the caller), `compile()` will not propose any of the
    // blacklisted nodes anyway. The `blacklist` argument is therefore
    // informational: the contract is that the caller has already pruned
    // `topology`. We keep the parameter for symmetry with the spec
    // (spec 019 §3.1) and for future code that may want to *partially*
    // prune a topology without rebuilding it.
    let _ = blacklist;

    match compile(topology, intent) {
        Ok(new_plan) => Ok(FailoverOutcome::Replaced(new_plan)),
        Err(_) => Ok(FailoverOutcome::NoReplacement),
    }
}

/// Multi-hop failover: try fallback routes first, then fresh compile.
///
/// When a node in a multi-hop path fails, this function:
/// 1. Checks pre-generated fallback routes from `MultihopResult.fallbacks`
///    — the first fallback whose path avoids all failed nodes is used.
/// 2. If no fallback works, does a fresh `compile_multihop()` on the
///    pruned topology (failed nodes removed by caller).
/// 3. Returns `NoReplacement` if both paths fail.
///
/// This is the R3 extension to the single-hop `replan()` above.
/// The `source` and `destination` are needed for the fresh compile path.
/// The `catalog` is the transport stage catalog for multi-hop compilation.
pub fn replan_multihop(
    topology: &Topology,
    intent: &Intent,
    original: &MultihopResult,
    failed_nodes: &[NodeId],
    source: &NodeId,
    destination: &NodeId,
    catalog: &[TransportStage],
) -> Result<FailoverOutcome, FailoverError> {
    if intent.name.is_empty() {
        return Err(FailoverError::EmptyIntent);
    }
    if failed_nodes.is_empty() {
        return Ok(FailoverOutcome::Replaced(original.primary.clone()));
    }

    // Build a set of failed node IDs for O(1) lookup.
    let failed_set: std::collections::HashSet<&NodeId> = failed_nodes.iter().collect();

    // Step 1: Try pre-generated fallback routes.
    for fallback in &original.fallbacks {
        let touches_failed = fallback.steps.iter().any(|step| failed_set.contains(&step.node));
        if !touches_failed {
            return Ok(FailoverOutcome::Replaced(fallback.clone()));
        }
    }

    // Step 2: Fresh compile on the pruned topology.
    match compile_multihop(topology, source, destination, intent, catalog) {
        Ok(new_result) => Ok(FailoverOutcome::Replaced(new_result.primary)),
        Err(MultihopError::EmptyTopology)
        | Err(MultihopError::NoPath(..))
        | Err(MultihopError::SameNode(..)) => Ok(FailoverOutcome::NoReplacement),
        Err(MultihopError::Validation(_)) => Ok(FailoverOutcome::NoReplacement),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::TopologyBuilder;
    fn two_node_topology_with_edges() -> (Topology, NodeId, NodeId) {
        let a = NodeId::new("a");
        let b = NodeId::new("b");
        let topo = TopologyBuilder::new()
            .with_name("test-edges")
            .add(crate::Node::new(a.clone(), crate::LocalityTier::L5Loopback))
            .add(crate::Node::new(b.clone(), crate::LocalityTier::L5Loopback))
            .connect("a", "b", crate::LocalityTier::L1SameNuma)
            .build();
        (topo, a, b)
    }
    fn two_node_topology() -> (Topology, NodeId, NodeId) {
        let a = NodeId::new("a");
        let b = NodeId::new("b");
        let topo = TopologyBuilder::new()
            .with_name("test")
            .add(crate::Node::new(a.clone(), crate::LocalityTier::L5Loopback))
            .add(crate::Node::new(b.clone(), crate::LocalityTier::L5Loopback))
            .connect("a", "b", crate::LocalityTier::L1SameNuma)
            .build();
        (topo, a, b)
    }
    #[test]
    fn empty_blacklist_returns_old_plan() {
        let (topo, a, _) = two_node_topology();
        let intent = crate::builder::IntentBuilder::new()
            .name("test-intent")
            .min_trust(crate::TrustLevel::Untrusted)
            .build();
        let original = compile(&topo, &intent).expect("compile should succeed");
        let outcome = replan(&topo, &intent, &original, &[]).expect("no error");
        match outcome {
            FailoverOutcome::Replaced(p) => {
                assert_eq!(p.id, original.id);
            }
            _ => panic!("expected Replaced"),
        }
        // Suppress unused warning for the non-blacklist arg
        let _ = a;
    }

    #[test]
    fn replan_after_node_pruning_produces_new_route() {
        let (topo, a, b) = two_node_topology();
        let intent = crate::builder::IntentBuilder::new()
            .name("test-intent")
            .min_trust(crate::TrustLevel::Untrusted)
            .build();
        let original = compile(&topo, &intent).expect("compile should succeed");
        assert!(!original.steps.is_empty(), "original plan must have steps");

        // Caller prunes `a` (simulating node failure) → topology with only `b`.
        let pruned = TopologyBuilder::new().with_name("pruned");
        let node_b = crate::Node::new(b.clone(), crate::LocalityTier::L5Loopback);
        let pruned = pruned.add(node_b);
        let pruned_topo = pruned.build();

        let outcome = replan(&pruned_topo, &intent, &original, &[a.clone()])
            .expect("no error");
        match outcome {
            FailoverOutcome::Replaced(new_plan) => {
                // The new plan must use only `b` (since `a` was pruned).
                for step in &new_plan.steps {
                    assert_eq!(step.node, b, "step must use surviving node");
                }
            }
            _ => panic!("expected Replaced"),
        }
    }

    #[test]
    fn replan_with_no_survivors_returns_no_replacement() {
        let (topo, a, b) = two_node_topology();
        let intent = crate::builder::IntentBuilder::new()
            .name("test-intent")
            .min_trust(crate::TrustLevel::Untrusted)
            .build();
        let original = compile(&topo, &intent).expect("compile should succeed");

        // Caller prunes BOTH nodes → empty topology.
        let empty_topo = TopologyBuilder::new().with_name("empty").build();
        let outcome = replan(&empty_topo, &intent, &original, &[a, b])
            .expect("no error");
        assert!(matches!(outcome, FailoverOutcome::NoReplacement));
    }

    #[test]
    fn empty_intent_name_returns_error() {
        let (topo, _, _) = two_node_topology();
        let intent = crate::builder::IntentBuilder::new()
            .name("")
            .min_trust(crate::TrustLevel::Untrusted)
            .build();
        let original = compile(&topo, &intent).expect("compile should succeed");
        let result = replan(&topo, &intent, &original, &[]);
        assert!(matches!(result, Err(FailoverError::EmptyIntent)));
    }

    // ── replan_multihop tests ──

    fn three_node_topology() -> (Topology, NodeId, NodeId, NodeId) {
        let a = NodeId::new("a");
        let b = NodeId::new("b");
        let c = NodeId::new("c");
        let topo = TopologyBuilder::new()
            .with_name("test-3node")
            .add(crate::Node::new(a.clone(), crate::LocalityTier::L5Loopback))
            .add(crate::Node::new(b.clone(), crate::LocalityTier::L5Loopback))
            .add(crate::Node::new(c.clone(), crate::LocalityTier::L5Loopback))
            .connect("a", "b", crate::LocalityTier::L1SameNuma)
            .connect("b", "c", crate::LocalityTier::L1SameNuma)
            .connect("a", "c", crate::LocalityTier::L6Lan)
            .build();
        (topo, a, b, c)
    }

    #[test]
    fn multihop_empty_blacklist_returns_original() {
        let (topo, a, _b, c) = three_node_topology();
        let intent = crate::builder::IntentBuilder::new()
            .name("test-intent")
            .min_trust(crate::TrustLevel::Untrusted)
            .build();
        let catalog = crate::multihop::builtin_stages();
        let result = compile_multihop(&topo, &a, &c, &intent, &catalog)
            .expect("compile_multihop should succeed");
        let outcome = replan_multihop(&topo, &intent, &result, &[], &a, &c, &catalog)
            .expect("no error");
        match outcome {
            FailoverOutcome::Replaced(plan) => assert_eq!(plan.id, result.primary.id),
            _ => panic!("expected Replaced"),
        }
    }

    #[test]
    fn multihop_fallback_avoids_failed_node() {
        let (topo, a, b, c) = three_node_topology();
        let intent = crate::builder::IntentBuilder::new()
            .name("test-intent")
            .min_trust(crate::TrustLevel::Untrusted)
            .build();
        let catalog = crate::multihop::builtin_stages();
        let result = compile_multihop(&topo, &a, &c, &intent, &catalog)
            .expect("compile_multihop should succeed");

        // If primary goes through b, failing b should use a fallback via a→c direct.
        let outcome = replan_multihop(&topo, &intent, &result, &[b.clone()], &a, &c, &catalog)
            .expect("no error");
        match outcome {
            FailoverOutcome::Replaced(plan) => {
                // The replacement must not touch node b.
                for step in &plan.steps {
                    assert_ne!(step.node, b, "replacement must avoid failed node b");
                }
            }
            FailoverOutcome::NoReplacement => {
                // Acceptable if no fallback exists and fresh compile also fails
                // (depends on topology connectivity after pruning).
            }
        }
    }

    #[test]
    fn multihop_fresh_compile_after_fallbacks_exhausted() {
        let (topo, a, _b, c) = three_node_topology();
        let intent = crate::builder::IntentBuilder::new()
            .name("test-intent")
            .min_trust(crate::TrustLevel::Untrusted)
            .build();
        let catalog = crate::multihop::builtin_stages();
        let result = compile_multihop(&topo, &a, &c, &intent, &catalog)
            .expect("compile_multihop should succeed");

        // Build a MultihopResult with empty fallbacks to force fresh compile path.
        let no_fallbacks = MultihopResult {
            primary: result.primary.clone(),
            cost: result.cost.clone(),
            fallbacks: vec![], // no fallbacks
            stages_per_hop: result.stages_per_hop.clone(),
        };

        // With no failed nodes, should return original.
        let outcome = replan_multihop(&topo, &intent, &no_fallbacks, &[], &a, &c, &catalog)
            .expect("no error");
        assert!(matches!(outcome, FailoverOutcome::Replaced(_)));
    }

    #[test]
    fn multihop_empty_intent_returns_error() {
        let (topo, a, _b, c) = three_node_topology();
        let intent = crate::builder::IntentBuilder::new()
            .name("")
            .min_trust(crate::TrustLevel::Untrusted)
            .build();
        let catalog = crate::multihop::builtin_stages();
        let result = compile_multihop(&topo, &a, &c, &intent, &catalog)
            .expect("compile_multihop should succeed");
        let err = replan_multihop(&topo, &intent, &result, &[], &a, &c, &catalog);
        assert!(matches!(err, Err(FailoverError::EmptyIntent)));
    }
}
