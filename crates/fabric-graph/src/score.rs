//! Scoring functions for route candidates.
//!
//! Each candidate route is scored on four dimensions:
//! - **Locality** — how physically close are the endpoints?
//! - **Latency** — measured link latency
//! - **Capability** — how well does the destination meet the intent's requirements?
//! - **Trust** — the trust level of the capability descriptors
//!
//! The composite score is a weighted sum of these four dimensions.

use crate::model::{Intent, IntentRequirements, LinkMetrics, Node, RouteStep, Score, ScoreBreakdown, TrustLevel};
use std::collections::HashMap;

/// Weights for the four scoring dimensions.
///
/// These are the defaults; they can be overridden via [`ScoringWeights::custom`].
#[derive(Debug, Clone)]
pub struct ScoringWeights {
    pub locality: f64,
    pub latency: f64,
    pub capability: f64,
    pub trust: f64,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            locality: 0.35,
            latency: 0.30,
            capability: 0.25,
            trust: 0.10,
        }
    }
}

impl ScoringWeights {
    pub fn custom(locality: f64, latency: f64, capability: f64, trust: f64) -> Self {
        Self {
            locality,
            latency,
            capability,
            trust,
        }
    }
}

/// Score a single route step against an intent.
pub(crate) fn score_step(
    step: &RouteStep,
    nodes: &HashMap<String, Node>,
    edges: &HashMap<String, crate::model::Edge>,
    requirements: &IntentRequirements,
    weights: &ScoringWeights,
) -> Option<Score> {
    let node = nodes.get(&step.node.0)?;

    let locality_score = score_locality(node, requirements);
    let latency_score = score_latency(step, edges, requirements);
    let capability_score = score_capability(node, requirements);
    let trust_score = score_trust(node);

    let breakdown = ScoreBreakdown::new(
        locality_score,
        latency_score,
        capability_score,
        trust_score,
    );

    let rank = 0;
    let reason = format_reason(
        &breakdown,
        node,
        requirements,
        locality_score,
        latency_score,
        capability_score,
        trust_score,
    );

    Some(Score {
        breakdown,
        rank,
        reason,
    })
}

pub(crate) fn score_locality(node: &Node, requirements: &IntentRequirements) -> f64 {
    let tier = node.locality_tier.as_f64();
    if let Some(max) = requirements.max_locality_tier {
        if tier > max {
            return 0.0;
        }
        // Score is 1.0 at max tier, 0.0 at tier 0 (best possible)
        let range = max - 0.0;
        if range == 0.0 {
            return 1.0;
        }
        return 1.0 - (tier / range);
    }
    // No constraint: use tier directly (L0=best, L8=worst)
    let normalized: f64 = 1.0 - (tier / 8.0).min(1.0);
    normalized.clamp(0.0, 1.0)
}

fn score_latency(step: &RouteStep, edges: &HashMap<String, crate::model::Edge>, requirements: &IntentRequirements) -> f64 {
    let Some(edge_id) = &step.via_edge else {
        // Direct hop — best possible latency
        return 1.0;
    };
    let Some(edge) = edges.get(&edge_id.0) else {
        return 0.5;
    };
    let Some(metrics) = &edge.metrics else {
        return 0.5;
    };
    let Some(latency_us) = metrics.latency_us else {
        return 0.5;
    };
    let Some(max_latency) = requirements.max_latency_us else {
        return 1.0; // No constraint — neutral
    };
    if latency_us > max_latency {
        return 0.0;
    }
    (1.0 - (latency_us / max_latency)).clamp(0.0, 1.0)
}

pub(crate) fn score_capability(node: &Node, requirements: &IntentRequirements) -> f64 {
    // Simple count-based scoring: does the node have the minimum required capabilities?
    let has_gpu = node.capabilities.iter().any(|c| c.descriptor_id.contains("gpu"));
    let has_cpu = !node.capabilities.is_empty();

    let mut score: f64 = 0.0;
    // CPU
    if !requirements.cpu_arch.is_empty() || requirements.min_ram_bytes.is_some() {
        if has_cpu {
            score += 0.5;
        }
    } else {
        score += 0.5;
    }

    // GPU
    if let Some(min_gpu) = requirements.min_gpu_count {
        if has_gpu && min_gpu >= 1 {
            score += 0.5;
        }
    } else {
        score += 0.25; // Neutral bonus for having GPU when not required
    }

    score.clamp(0.0, 1.0)
}

pub(crate) fn score_trust(node: &Node) -> f64 {
    if node.capabilities.is_empty() {
        return 0.5; // Unknown trust for unprobed nodes
    }
    let max_trust = node
        .capabilities
        .iter()
        .map(|c| c.trust)
        .max()
        .unwrap_or(TrustLevel::Untrusted);
    match max_trust {
        TrustLevel::Untrusted => 0.1,
        TrustLevel::Bootstrap => 0.4,
        TrustLevel::Attested => 0.75,
        TrustLevel::Audited => 1.0,
    }
}

fn format_reason(
    breakdown: &ScoreBreakdown,
    node: &Node,
    requirements: &IntentRequirements,
    locality_score: f64,
    latency_score: f64,
    capability_score: f64,
    trust_score: f64,
) -> String {
    let mut reasons = Vec::new();
    if locality_score >= 0.8 {
        reasons.push(format!("high locality (tier {:.0})", node.locality_tier.as_f64()));
    } else if locality_score < 0.3 {
        reasons.push(format!("low locality (tier {:.0})", node.locality_tier.as_f64()));
    }
    if latency_score >= 0.8 {
        reasons.push("low latency".to_string());
    }
    if capability_score >= 0.8 {
        reasons.push("strong capability match".to_string());
    }
    if trust_score >= 0.8 {
        reasons.push(format!("{:?}", node.capabilities.first().map(|c| c.trust).unwrap_or(TrustLevel::Untrusted)));
    }
    if reasons.is_empty() {
        reasons.push("acceptable".to_string());
    }
    format!(
        "score={:.2} ({})",
        breakdown.composite,
        reasons.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CapabilityRef, EdgeId, NodeId, RouteStep};
    use fabric_capability::locality::LocalityTier;
    use std::collections::HashMap;

    fn make_node(id: &str, tier: u8, trust: TrustLevel) -> Node {
        let cap = CapabilityRef::new(format!("sha256:{}", id)).with_trust(trust);
        Node::new(NodeId::new(id), LocalityTier::from_index(tier).unwrap_or(LocalityTier::L5Loopback))
            .with_capability(cap)
    }

    fn make_step(node_id: &str) -> RouteStep {
        RouteStep {
            node: NodeId::new(node_id),
            capability_id: None,
            via_edge: None,
            action: "execute".to_string(),
        }
    }

    #[test]
    fn test_score_locality_within_limit() {
        let node = make_node("gpu-0", 2, TrustLevel::Attested);
        let reqs = IntentRequirements {
            max_locality_tier: Some(3.0),
            ..Default::default()
        };
        let score = score_locality(&node, &reqs);
        assert!((score - 0.333).abs() < 0.01, "score={}", score);
    }

    #[test]
    fn test_score_locality_exceeds_limit() {
        let node = make_node("far-node", 5, TrustLevel::Attested);
        let reqs = IntentRequirements {
            max_locality_tier: Some(3.0),
            ..Default::default()
        };
        assert_eq!(score_locality(&node, &reqs), 0.0);
    }

    #[test]
    fn test_score_latency_within_budget() {
        let step = RouteStep {
            node: NodeId::new("a"),
            capability_id: None,
            via_edge: Some(EdgeId::new("a-b")),
            action: "route".to_string(),
        };
        let edge = crate::model::Edge::new(
            EdgeId::new("a-b"),
            NodeId::new("a"),
            NodeId::new("b"),
            LocalityTier::L2,
        )
        .with_metrics(LinkMetrics {
            latency_us: Some(50.0),
            bandwidth_bps: Some(1_000_000_000),
            packet_loss: Some(0.0),
            jitter_us: Some(5.0),
        });
        let mut edges = HashMap::new();
        edges.insert("a-b".to_string(), edge);

        let reqs = IntentRequirements {
            max_latency_us: Some(100.0),
            ..Default::default()
        };
        let score = score_latency(&step, &edges, &reqs);
        assert!((score - 0.5).abs() < 0.01, "score={}", score);
    }

    #[test]
    fn test_score_latency_exceeds_budget() {
        let step = RouteStep {
            node: NodeId::new("a"),
            capability_id: None,
            via_edge: Some(EdgeId::new("a-b")),
            action: "route".to_string(),
        };
        let edge = crate::model::Edge::new(
            EdgeId::new("a-b"),
            NodeId::new("a"),
            NodeId::new("b"),
            LocalityTier::L2,
        )
        .with_metrics(LinkMetrics {
            latency_us: Some(150.0),
            bandwidth_bps: Some(1_000_000_000),
            packet_loss: Some(0.0),
            jitter_us: Some(5.0),
        });
        let mut edges = HashMap::new();
        edges.insert("a-b".to_string(), edge);

        let reqs = IntentRequirements {
            max_latency_us: Some(100.0),
            ..Default::default()
        };
        assert_eq!(score_latency(&step, &edges, &reqs), 0.0);
    }

    #[test]
    fn test_score_trust_attested() {
        let node = make_node("a", 2, TrustLevel::Attested);
        assert!((score_trust(&node) - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_score_trust_audited() {
        let node = make_node("a", 2, TrustLevel::Audited);
        assert_eq!(score_trust(&node), 1.0);
    }

    #[test]
    fn test_score_trust_empty() {
        let node = Node::new(NodeId::new("unprobed"), LocalityTier::L2);
        assert_eq!(score_trust(&node), 0.5);
    }

    #[test]
    fn test_scoring_weights_default() {
        let w = ScoringWeights::default();
        assert!((w.locality - 0.35).abs() < 0.001);
        assert!((w.latency - 0.30).abs() < 0.001);
        assert!((w.capability - 0.25).abs() < 0.001);
        assert!((w.trust - 0.10).abs() < 0.001);
    }

    #[test]
    fn test_score_breakdown_composite() {
        let breakdown =
            ScoreBreakdown::new(1.0, 1.0, 1.0, 1.0);
        // 0.35 + 0.30 + 0.25 + 0.10 = 1.0
        assert!((breakdown.composite - 1.0).abs() < 0.001);
    }
}
