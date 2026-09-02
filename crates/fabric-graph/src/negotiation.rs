//! Negotiation algorithm: match intents against a topology to produce ranked candidates.
//!
//! This module implements the core negotiation loop:
//! 1. Filter nodes that satisfy hard requirements (tags, locality, trust)
//! 2. Score each candidate node on soft dimensions (locality, latency, capability, trust)
//! 3. Sort by composite score and return ranked results

use crate::model::{EdgeId, Intent, IntentRequirements, Node, NodeId, ScoreBreakdown, Topology, TrustLevel};
use crate::score::{score_capability, score_locality, score_trust, ScoringWeights};
use std::collections::BTreeMap;

/// The result of a negotiation: ranked candidates with scores and reasoning.
#[derive(Debug, Clone)]
pub struct NegotiationResult {
    /// Ranked list of candidates (best first).
    pub candidates: Vec<NegotiatedCandidate>,
    /// The original intent.
    pub intent: Intent,
    /// Number of nodes that passed hard filters but were rejected for soft scoring.
    pub filtered_count: usize,
    /// Number of nodes that failed hard filters.
    pub rejected_hard_count: usize,
}

/// A single negotiated candidate route.
#[derive(Debug, Clone)]
pub struct NegotiatedCandidate {
    /// The destination node.
    pub node: NodeId,
    /// Score breakdown for this candidate.
    pub score: ScoreBreakdown,
    /// Why this candidate was ranked where it is.
    pub reason: String,
    /// Whether this candidate has a real-time island.
    pub has_rt_island: bool,
    /// Whether this candidate matches the intent's preferred_node hint.
    /// When true, the candidate is sorted ahead of non-preferred candidates
    /// regardless of composite score (see SPEC PF-FR-043).
    pub is_preferred: bool,
}

/// Run the negotiation algorithm for a single intent against a topology.
pub fn negotiate(topology: &Topology, intent: &Intent) -> NegotiationResult {
    let weights = ScoringWeights::default();
    let mut candidates = Vec::new();
    let mut rejected_hard = 0;
    let mut filtered_soft = 0;

    for (node_id, node) in &topology.nodes {
        // === HARD FILTERS ===
        if !passes_hard_filters(node, &intent.requirements, intent.min_trust) {
            rejected_hard += 1;
            continue;
        }

        // === SOFT SCORING ===
        let locality_score = score_locality(node, &intent.requirements);
        let capability_score = score_capability(node, &intent.requirements);
        let trust_score = score_trust(node);

        // For latency, we check all incident edges
        let latency_score = best_latency_score(node, &topology.edges, &intent.requirements);

        // Preferred node bonus (RT-locality requirement, see SPEC PF-FR-043)
        let is_preferred = intent
            .preferred_node
            .as_ref()
            .map(|p| p == node_id)
            .unwrap_or(false);

        let breakdown = ScoreBreakdown::new(locality_score, latency_score, capability_score, trust_score);
        let reason = format_reason(node, &breakdown, &intent.requirements);

        let has_rt_island = node.tags.contains(&"rt-island".to_string())
            || intent.requirements.requires_rt_island;

        candidates.push(NegotiatedCandidate {
            node: node_id.clone(),
            score: breakdown,
            reason,
            has_rt_island,
            is_preferred,
        });
    }

    // Sort: preferred first, then by composite score (descending)
    candidates.sort_by(|a, b| {
        b.is_preferred
            .cmp(&a.is_preferred)
            .then_with(|| {
                b.score
                    .composite
                    .partial_cmp(&a.score.composite)
                    .unwrap_or(std::cmp::Ordering::Less)
            })
    });

    // Assign ranks
    for (i, cand) in candidates.iter_mut().enumerate() {
        // We need to update the score rank — but ScoreBreakdown doesn't have rank.
        // Instead, we'll track rank in the candidate.
    }

    filtered_soft = candidates.len();

    NegotiationResult {
        candidates,
        intent: intent.clone(),
        filtered_count: filtered_soft,
        rejected_hard_count: rejected_hard,
    }
}

/// Hard-filter check: does this node pass all non-negotiable requirements?
fn passes_hard_filters(node: &Node, reqs: &IntentRequirements, min_trust: crate::model::TrustLevel) -> bool {
    // Required tags
    for tag in &reqs.required_tags {
        if !node.tags.contains(tag) {
            return false;
        }
    }

    // Max locality tier
    if let Some(max) = reqs.max_locality_tier {
        if (node.locality_tier.as_f64()) > max {
            return false;
        }
    }

    // Trust level
    if !node.meets_trust(min_trust) {
        return false;
    }

    // RT island requirement
    if reqs.requires_rt_island && !node.tags.contains(&"rt-island".to_string()) {
        return false;
    }

    true
}

/// Find the best (lowest latency) incident edge score for a node.
fn best_latency_score(
    node: &Node,
    edges: &BTreeMap<EdgeId, crate::model::Edge>,
    reqs: &IntentRequirements,
) -> f64 {
    let mut best: f64 = 0.5; // default neutral score
    for edge in edges.values() {
        if edge.from != node.id && edge.to != node.id {
            continue;
        }
        if !edge.up {
            continue;
        }
        let Some(metrics) = &edge.metrics else {
            continue;
        };
        let Some(latency_us) = metrics.latency_us else {
            continue;
        };
        let Some(max_latency) = reqs.max_latency_us else {
            // No constraint — this edge is fine
            best = best.max(0.75_f64);
            continue;
        };
        if latency_us > max_latency {
            continue;
        }
        let s = (1.0 - (latency_us / max_latency)).clamp(0.0_f64, 1.0_f64);
        best = best.max(s);
    }
    best
}

fn format_reason(
    node: &Node,
    breakdown: &ScoreBreakdown,
    _reqs: &IntentRequirements,
) -> String {
    let tier = node.locality_tier.as_f64();
    let tier_label = format!("tier-{:.0}", tier);
    let trust_label = format!(
        "{:?}",
        node.capabilities
            .first()
            .map(|c| c.trust)
            .unwrap_or(crate::model::TrustLevel::Untrusted)
    );
    format!(
        "{} {} score={:.2}",
        tier_label,
        trust_label,
        breakdown.composite,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CapabilityRef, EdgeId, IntentRequirements, IntentId, NodeId, RoutePlanId, TopologyEpoch};
    use chrono::Utc;
    use fabric_capability::locality::LocalityTier;

    fn make_topo() -> Topology {
        let mut topo = Topology::new();
        topo.add_node(
            Node::new(NodeId::new("gpu-0"), LocalityTier::L1)
                .with_tag("rt-island")
                .with_capability(CapabilityRef::new("sha256:gpu0".to_string()).with_trust(crate::model::TrustLevel::Attested)),
        );
        topo.add_node(
            Node::new(NodeId::new("cpu-0"), LocalityTier::L2)
                .with_capability(CapabilityRef::new("sha256:cpu0".to_string()).with_trust(crate::model::TrustLevel::Bootstrap)),
        );
        topo.add_node(
            Node::new(NodeId::new("far"), LocalityTier::L7)
                .with_capability(CapabilityRef::new("sha256:far".to_string()).with_trust(crate::model::TrustLevel::Untrusted)),
        );
        topo
    }

    fn make_intent() -> Intent {
        Intent {
            id: IntentId::new(),
            name: "ML training".to_string(),
            requirements: IntentRequirements {
                required_tags: vec!["rt-island".to_string()],
                max_locality_tier: Some(5.0),
                ..Default::default()
            },
            preferred_node: None,
            min_trust: crate::model::TrustLevel::Bootstrap,
            expires_at: None,
            tags: vec![],
        }
    }

    #[test]
    fn test_negotiate_filters_unprobed() {
        let mut topo = Topology::new();
        topo.add_node(Node::new(NodeId::new("unprobed"), LocalityTier::L2));

        let intent = Intent {
            id: IntentId::new(),
            name: "test".to_string(),
            requirements: IntentRequirements::default(),
            preferred_node: None,
            min_trust: crate::model::TrustLevel::Untrusted,
            expires_at: None,
            tags: vec![],
        };

        let result = negotiate(&topo, &intent);
        // Unprobed nodes pass hard filters but score low
        assert_eq!(result.rejected_hard_count, 0);
    }

    #[test]
    fn test_negotiate_rt_island_required() {
        let mut topo = Topology::new();
        topo.add_node(Node::new(NodeId::new("gpu-0"), LocalityTier::L1).with_tag("rt-island"));
        topo.add_node(Node::new(NodeId::new("cpu-0"), LocalityTier::L2));

        let mut intent = make_intent();
        intent.requirements.requires_rt_island = true;

        let result = negotiate(&topo, &intent);
        assert_eq!(result.rejected_hard_count, 1); // cpu-0 rejected
        assert_eq!(result.candidates.len(), 1);
        assert_eq!(result.candidates[0].node.0, "gpu-0");
    }

    #[test]
    fn test_negotiate_max_locality_tier() {
        let mut topo = Topology::new();
        topo.add_node(Node::new(NodeId::new("local"), LocalityTier::L1).with_tag("rt-island"));
        topo.add_node(Node::new(NodeId::new("regional"), LocalityTier::L5).with_tag("rt-island"));
        topo.add_node(Node::new(NodeId::new("far"), LocalityTier::L8).with_tag("rt-island"));

        let mut intent = make_intent();
        intent.requirements.max_locality_tier = Some(5.0);

        let result = negotiate(&topo, &intent);
        assert_eq!(result.candidates.len(), 2); // local + regional; far rejected
    }

    #[test]
    fn test_negotiate_ranking() {
        let topo = make_topo();
        let intent = make_intent();
        let result = negotiate(&topo, &intent);

        assert_eq!(result.candidates.len(), 1); // only gpu-0 has rt-island
        assert_eq!(result.candidates[0].node.0, "gpu-0");
        assert!(result.candidates[0].score.composite > 0.0);
    }

    #[test]
    fn test_negotiate_empty_topology() {
        let topo = Topology::new();
        let intent = make_intent();
        let result = negotiate(&topo, &intent);
        assert!(result.candidates.is_empty());
        assert_eq!(result.rejected_hard_count, 0);
    }
}
