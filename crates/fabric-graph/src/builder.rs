//! Ergonomic builders for topologies, intents, and route plans.

use crate::model::{
    CapabilityRef, Edge, EdgeId, Intent, IntentId, IntentRequirements, LinkMetrics, Node,
    NodeId, RoutePlan, RoutePlanId, RouteStep, Topology, TopologyEpoch, TopologyMeta, TrustLevel,
};
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;

/// A builder for [`Topology`] that allows incremental construction.
///
/// This is the recommended way to construct a topology in tests and CLI tools.
#[derive(Debug, Default, Clone)]
pub struct TopologyBuilder {
    inner: Topology,
}

impl TopologyBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.inner.meta.name = name.into();
        self
    }

    pub fn add(mut self, node: Node) -> Self {
        self.inner.add_node(node);
        self
    }

    pub fn add_simple_node(
        mut self,
        id: &str,
        tier: fabric_capability::locality::LocalityTier,
    ) -> Self {
        self.inner.add_node(Node::new(NodeId::new(id), tier));
        self
    }

    pub fn add_with_label(
        mut self,
        id: &str,
        label: &str,
        tier: fabric_capability::locality::LocalityTier,
    ) -> Self {
        self.inner.add_node(
            Node::new(NodeId::new(id), tier).with_label(label),
        );
        self
    }

    pub fn add_with_cap(
        mut self,
        id: &str,
        tier: fabric_capability::locality::LocalityTier,
        capability_id: &str,
    ) -> Self {
        self.inner.add_node(
            Node::new(NodeId::new(id), tier)
                .with_capability(CapabilityRef::new(format!("sha256:{}", capability_id))),
        );
        self
    }

    pub fn add_with_tag(
        mut self,
        id: &str,
        tier: fabric_capability::locality::LocalityTier,
        tag: &str,
    ) -> Self {
        self.inner.add_node(
            Node::new(NodeId::new(id), tier).with_tag(tag),
        );
        self
    }

    pub fn connect(
        mut self,
        from: &str,
        to: &str,
        tier: fabric_capability::locality::LocalityTier,
    ) -> Self {
        let edge = Edge::new(
            EdgeId::new(format!("{}-{}", from, to)),
            NodeId::new(from),
            NodeId::new(to),
            tier,
        );
        self.inner.add_edge(edge).expect("nodes must exist");
        self
    }

    pub fn connect_with_metrics(
        mut self,
        from: &str,
        to: &str,
        tier: fabric_capability::locality::LocalityTier,
        metrics: LinkMetrics,
    ) -> Self {
        let edge = Edge::new(
            EdgeId::new(format!("{}-{}", from, to)),
            NodeId::new(from),
            NodeId::new(to),
            tier,
        )
        .with_metrics(metrics);
        self.inner.add_edge(edge).expect("nodes must exist");
        self
    }

    pub fn build(self) -> Topology {
        self.inner
    }
}

/// A builder for [`Intent`].
#[derive(Debug, Clone)]
pub struct IntentBuilder {
    name: String,
    requirements: IntentRequirements,
    preferred_node: Option<NodeId>,
    min_trust: TrustLevel,
    expires_at: Option<DateTime<Utc>>,
    tags: Vec<String>,
}

impl Default for IntentBuilder {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            requirements: IntentRequirements::default(),
            preferred_node: None,
            min_trust: TrustLevel::Untrusted,
            expires_at: None,
            tags: Vec::new(),
        }
    }
}

impl IntentBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn min_trust(mut self, trust: TrustLevel) -> Self {
        self.min_trust = trust;
        self
    }

    pub fn max_locality(mut self, tier: f64) -> Self {
        self.requirements.max_locality_tier = Some(tier);
        self
    }

    pub fn require_tag(mut self, tag: impl Into<String>) -> Self {
        self.requirements.required_tags.push(tag.into());
        self
    }

    pub fn prefer_node(mut self, node_id: impl Into<String>) -> Self {
        self.preferred_node = Some(NodeId::new(node_id));
        self
    }

    pub fn require_gpu(mut self, count: u32) -> Self {
        self.requirements.min_gpu_count = Some(count);
        self
    }

    pub fn require_rt_island(mut self) -> Self {
        self.requirements.requires_rt_island = true;
        self.requirements.required_tags.push("rt-island".to_string());
        self
    }

    pub fn require_cpu_arch(mut self, arch: impl Into<String>) -> Self {
        self.requirements.cpu_arch.push(arch.into());
        self
    }

    pub fn require_min_ram(mut self, bytes: u64) -> Self {
        self.requirements.min_ram_bytes = Some(bytes);
        self
    }

    pub fn max_latency_us(mut self, latency: f64) -> Self {
        self.requirements.max_latency_us = Some(latency);
        self
    }

    pub fn expires_in(mut self, seconds: i64) -> Self {
        self.expires_at = Some(Utc::now() + chrono::Duration::seconds(seconds));
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn build(self) -> Intent {
        Intent {
            id: IntentId::new(),
            name: self.name,
            requirements: self.requirements,
            preferred_node: self.preferred_node,
            min_trust: self.min_trust,
            expires_at: self.expires_at,
            tags: self.tags,
        }
    }
}

/// Build a [`RouteStep`] for tests or CLI scripts.
pub fn make_step(node_id: &str, action: &str) -> RouteStep {
    RouteStep {
        node: NodeId::new(node_id),
        capability_id: None,
        via_edge: None,
        action: action.to_string(),
    }
}

/// Build a [`RoutePlan`] directly for tests.
pub fn make_plan(steps: Vec<RouteStep>, epoch: TopologyEpoch) -> RoutePlan {
    let now = Utc::now();
    RoutePlan {
        id: RoutePlanId::new(),
        intent_id: IntentId::new(),
        topology_epoch: epoch,
        steps,
        estimated_latency_us: None,
        score: None,
        compiled_at: now,
        expires_at: now + chrono::Duration::hours(1),
        tags: Vec::new(),
    }
}

/// A multi-intent request (sequence of intents to be planned together).
#[derive(Debug, Clone)]
pub struct MultiIntent {
    pub name: String,
    pub intents: Vec<Intent>,
}

impl MultiIntent {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            intents: Vec::new(),
        }
    }

    pub fn add(mut self, intent: Intent) -> Self {
        self.intents.push(intent);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fabric_capability::locality::LocalityTier;

    #[test]
    fn test_topology_builder_simple() {
        let topo = TopologyBuilder::new()
            .with_name("test")
            .add_simple_node("a", LocalityTier::L1)
            .add_simple_node("b", LocalityTier::L2)
            .connect("a", "b", LocalityTier::L1)
            .build();

        assert_eq!(topo.meta.name, "test");
        assert_eq!(topo.node_count(), 2);
        assert_eq!(topo.edge_count(), 1);
    }

    #[test]
    fn test_topology_builder_with_cap() {
        let topo = TopologyBuilder::new()
            .add_with_cap("gpu", LocalityTier::L1, "abc123")
            .build();

        let node = topo.node(&NodeId::new("gpu")).unwrap();
        assert_eq!(node.capabilities.len(), 1);
        assert_eq!(node.capabilities[0].descriptor_id, "sha256:abc123");
    }

    #[test]
    fn test_topology_builder_connect_with_metrics() {
        let topo = TopologyBuilder::new()
            .add_simple_node("a", LocalityTier::L1)
            .add_simple_node("b", LocalityTier::L1)
            .connect_with_metrics("a", "b", LocalityTier::L1, LinkMetrics {
                latency_us: Some(10.0),
                bandwidth_bps: Some(1_000_000_000),
                packet_loss: Some(0.0),
                jitter_us: Some(1.0),
            })
            .build();

        let edge = topo.edge(&EdgeId::new("a-b")).unwrap();
        assert!(edge.metrics.is_some());
    }

    #[test]
    fn test_intent_builder_full() {
        let intent = IntentBuilder::new()
            .name("ML training")
            .min_trust(TrustLevel::Attested)
            .max_locality(3.0)
            .require_tag("rt-island")
            .require_gpu(1)
            .max_latency_us(100.0)
            .expires_in(3600)
            .with_tag("priority:high")
            .build();

        assert_eq!(intent.name, "ML training");
        assert_eq!(intent.min_trust, TrustLevel::Attested);
        assert_eq!(intent.requirements.required_tags, vec!["rt-island"]);
        assert_eq!(intent.requirements.min_gpu_count, Some(1));
        assert!(intent.expires_at.is_some());
    }

    #[test]
    fn test_intent_builder_require_rt_island() {
        let intent = IntentBuilder::new()
            .require_rt_island()
            .build();
        assert!(intent.requirements.requires_rt_island);
        assert!(intent.requirements.required_tags.contains(&"rt-island".to_string()));
    }

    #[test]
    fn test_make_step() {
        let step = make_step("a", "execute");
        assert_eq!(step.node.0, "a");
        assert_eq!(step.action, "execute");
    }

    #[test]
    fn test_multi_intent() {
        let multi = MultiIntent::new("batch")
            .add(IntentBuilder::new().name("step-1").build())
            .add(IntentBuilder::new().name("step-2").build());
        assert_eq!(multi.intents.len(), 2);
    }
}
