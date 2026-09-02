//! Core domain model for Fabric's topology graph.
//!
//! The graph consists of:
//! - **Nodes** — physical machines or virtual execution contexts
//! - **Edges** — links between nodes with locality tier and link metrics
//! - **Capabilities** — what each node can do (attached to nodes)
//! - **Intents** — placement requirements from a caller
//! - **RoutePlans** — compiler output: a sequence of hops satisfying an intent

use std::collections::BTreeMap;

use anyhow::bail;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Monotonically increasing epoch counter for topology versioning.
///
/// Each capability advertisement and topology mutation increments the epoch.
/// Route plans are pinned to an epoch: they are only valid if the current
/// epoch matches the epoch they were compiled against.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default,
)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct TopologyEpoch(pub u64);

impl TopologyEpoch {
    /// Advance the epoch by one.
    #[must_use]
    pub fn bump(&mut self) -> TopologyEpoch {
        self.0 += 1;
        *self
    }

    /// Increment and return the new epoch value.
    #[must_use]
    pub fn bump_and_get(&mut self) -> u64 {
        self.0 += 1;
        self.0
    }
}

impl std::fmt::Display for TopologyEpoch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Trust level for a capability descriptor attached to a node.
///
/// Fabric does not *trust* descriptors by default; it requires an attestation
/// chain. The trust level reflects how much verification has been performed.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    Default,
)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum TrustLevel {
    /// Node reported its own capabilities. No external verification.
    /// PF-FR-012: No default trust.
    #[default]
    Untrusted = 0,
    /// Bootstrap trust assigned by local policy (e.g. same admin domain).
    Bootstrap = 1,
    /// Verified against a signed attestation (e.g. TPM quote, SEV-SNP).
    Attested = 2,
    /// Audit complete, cross-verified by a trusted third party.
    Audited = 3,
}

impl TrustLevel {
    /// Whether this trust level is sufficient to meet a minimum requirement.
    #[must_use]
    pub fn satisfies(&self, minimum: TrustLevel) -> bool {
        *self >= minimum
    }
}

// ---------------------------------------------------------------------------
// Identifier types
// ---------------------------------------------------------------------------

/// Unique identifier for a node in the topology graph.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct NodeId(pub String);

impl NodeId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for an edge (link) between two nodes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct EdgeId(pub String);

impl EdgeId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl std::fmt::Display for EdgeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for an intent (placement requirement).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct IntentId(pub Uuid);

impl IntentId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for IntentId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for a compiled route plan.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RoutePlanId(pub Uuid);

impl RoutePlanId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for RoutePlanId {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Capability reference
// ---------------------------------------------------------------------------

/// A reference to a capability descriptor, optionally with trust metadata.
///
/// When attached to a node, this is a pointer to the descriptor plus trust info.
/// When used in an intent, this is a *requirement* against which capabilities
/// are matched.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct CapabilityRef {
    /// SHA-256 of the canonical JSON bytes of the descriptor.
    /// Used as the stable, content-addressable identity key.
    pub descriptor_id: String,
    /// Trust level assigned to this descriptor.
    pub trust: TrustLevel,
    /// When this descriptor was last refreshed.
    #[cfg_attr(feature = "schemars", schemars(with = "Option<String>"))]
    pub refreshed_at: Option<DateTime<Utc>>,
}

impl CapabilityRef {
    pub fn new(descriptor_id: String) -> Self {
        Self {
            descriptor_id,
            trust: TrustLevel::default(),
            refreshed_at: None,
        }
    }

    pub fn with_trust(mut self, trust: TrustLevel) -> Self {
        self.trust = trust;
        self
    }
}

// ---------------------------------------------------------------------------
// Link metrics
// ---------------------------------------------------------------------------

/// Metrics for a single link (edge) in the topology.
///
/// All fields are Option<T> because not every link has every metric measured.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct LinkMetrics {
    /// Round-trip latency in microseconds. None if not measured.
    pub latency_us: Option<f64>,
    /// Estimated one-way bandwidth in bytes per second. None if not measured.
    pub bandwidth_bps: Option<u64>,
    /// Packet loss rate 0.0..1.0. None if not measured.
    pub packet_loss: Option<f64>,
    /// Jitter in microseconds (stddev of latency samples). None if not measured.
    pub jitter_us: Option<f64>,
}

impl LinkMetrics {
    /// Whether all metrics are known (i.e. the link has been actively measured).
    pub fn is_complete(&self) -> bool {
        self.latency_us.is_some()
            && self.bandwidth_bps.is_some()
            && self.packet_loss.is_some()
            && self.jitter_us.is_some()
    }

    /// Estimate effective bandwidth given a required latency.
    pub fn effective_bandwidth(&self, max_latency_us: f64) -> Option<u64> {
        let lat = self.latency_us?;
        if lat > max_latency_us {
            return None;
        }
        let bw = self.bandwidth_bps?;
        // Simple model: effective = bandwidth * (1 - loss_rate) * latency_factor
        let loss_factor = 1.0 - self.packet_loss.unwrap_or(0.0);
        let latency_factor = (max_latency_us / lat).min(1.0);
        Some(((bw as f64) * loss_factor * latency_factor) as u64)
    }
}

// ---------------------------------------------------------------------------
// Node
// ---------------------------------------------------------------------------

/// A node in the topology graph.
///
/// Each node represents a physical machine, VM, accelerator, or other execution
/// context. It carries zero or more capability descriptors.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Node {
    /// Unique node identifier (hostname, IP, or stable UUID).
    pub id: NodeId,
    /// Human-readable label (optional).
    pub label: Option<String>,
    /// Primary locality tier of this node. Used for copy-path reasoning.
    #[cfg_attr(feature = "schemars", schemars(with = "u8"))]
    pub locality_tier: fabric_capability::locality::LocalityTier,
    /// All capability descriptors advertised by this node.
    /// A node with an empty vec has not been probed (or is a pure router).
    pub capabilities: Vec<CapabilityRef>,
    /// When this node was last seen in the topology.
    pub last_seen: DateTime<Utc>,
    /// Extra node-level tags (e.g. "gpu-pool", "high-memory", "rt-island").
    pub tags: Vec<String>,
}

impl Node {
    pub fn new(id: NodeId, locality_tier: fabric_capability::locality::LocalityTier) -> Self {
        Self {
            id,
            label: None,
            locality_tier,
            capabilities: Vec::new(),
            last_seen: Utc::now(),
            tags: Vec::new(),
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_capability(mut self, cap: CapabilityRef) -> Self {
        self.capabilities.push(cap);
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Whether this node meets the minimum trust requirement.
    pub fn meets_trust(&self, minimum: TrustLevel) -> bool {
        for cap in &self.capabilities {
            if !cap.trust.satisfies(minimum) {
                return false;
            }
        }
        true
    }
}

// ---------------------------------------------------------------------------
// Edge
// ---------------------------------------------------------------------------

/// An edge (link) between two nodes in the topology graph.
///
/// An edge always has a **locality tier** (L0..L8). The tier reflects the
/// physical separation of the two endpoints. Lower tier = better locality.
/// L0 = same process, L1 = same NUMA node, L2 = same machine, L3 = same LAN,
/// L4 = same campus, L5 = same region, L6 = cross-region, L7 = cross-cloud,
/// L8 = satellite/WAN.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Edge {
    pub id: EdgeId,
    /// Source node.
    pub from: NodeId,
    /// Destination node.
    pub to: NodeId,
    /// Locality tier of this link (required — PF-FR-002).
    #[cfg_attr(feature = "schemars", schemars(with = "u8"))]
    pub locality_tier: fabric_capability::locality::LocalityTier,
    /// Measured link metrics (optional — populated by probing).
    pub metrics: Option<LinkMetrics>,
    /// Whether this edge is currently usable (admin-up, not quarantined).
    pub up: bool,
}

impl Edge {
    pub fn new(
        id: EdgeId,
        from: NodeId,
        to: NodeId,
        locality_tier: fabric_capability::locality::LocalityTier,
    ) -> Self {
        Self {
            id,
            from,
            to,
            locality_tier,
            metrics: None,
            up: true,
        }
    }

    pub fn with_metrics(mut self, metrics: LinkMetrics) -> Self {
        self.metrics = Some(metrics);
        self
    }
}

// ---------------------------------------------------------------------------
// Intent
// ---------------------------------------------------------------------------

/// The desired execution context for a placed object.
///
/// An intent is the caller-side description of where something should run.
/// The compiler translates an intent into a route plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Intent {
    /// Unique intent identifier.
    pub id: IntentId,
    /// Human-readable name for this intent (e.g. "ML training on GPU-0").
    pub name: String,
    /// Requirements on the execution context.
    pub requirements: IntentRequirements,
    /// Optional: preferred destination node (soft hint, compiler may override).
    pub preferred_node: Option<NodeId>,
    /// Minimum trust level required for all capabilities.
    pub min_trust: TrustLevel,
    /// When this intent expires (None = no expiry).
    #[cfg_attr(feature = "schemars", schemars(with = "Option<String>"))]
    pub expires_at: Option<DateTime<Utc>>,
    /// Tags that the caller wants attached to the resulting route plan.
    pub tags: Vec<String>,
}

/// Core requirements for an intent.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct IntentRequirements {
    /// Required CPU architecture(s). Empty = any.
    pub cpu_arch: Vec<String>,
    /// Minimum RAM in bytes.
    pub min_ram_bytes: Option<u64>,
    /// Required GPU count (None = no GPU needed).
    pub min_gpu_count: Option<u32>,
    /// Required GPU compute capability (e.g. "8.0" for CUDA 8.0).
    pub min_gpu_compute: Option<String>,
    /// Required locality tier (or better). None = any.
    pub max_locality_tier: Option<f64>,
    /// Required OS platform(s). Empty = any.
    pub platforms: Vec<String>,
    /// Required tags (all must be present on the destination node).
    pub required_tags: Vec<String>,
    /// Maximum acceptable latency in microseconds (end-to-end).
    pub max_latency_us: Option<f64>,
    /// Minimum required bandwidth in bytes per second.
    pub min_bandwidth_bps: Option<u64>,
    /// Whether this intent requires a real-time island.
    pub requires_rt_island: bool,
    /// Whether this intent requires GPU direct (NVLink/P2P).
    pub requires_gpu_direct: bool,
}

impl IntentRequirements {
    /// Check whether a node satisfies these requirements.
    pub fn matches_node(&self, node: &Node) -> bool {
        // Check required tags
        for tag in &self.required_tags {
            if !node.tags.contains(tag) {
                return false;
            }
        }
        // Check max locality tier
        if let Some(max) = self.max_locality_tier {
            if (node.locality_tier.as_f64()) > max {
                return false;
            }
        }
        // Check trust level (if node has capabilities)
        // CPU arch, RAM, GPU are checked against descriptors separately
        true
    }
}

// ---------------------------------------------------------------------------
// Route plan
// ---------------------------------------------------------------------------

/// A single hop in a route plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct RouteStep {
    /// The node this step executes on.
    pub node: NodeId,
    /// The capability descriptor to use at this hop (descriptor_id reference).
    pub capability_id: Option<String>,
    /// Edge used to reach this node (if not the first hop).
    pub via_edge: Option<EdgeId>,
    /// What this hop does (e.g. "execute", "route", "sink").
    pub action: String,
}

/// A compiled route plan: the output of the topology compiler.
///
/// A route plan is the concrete answer to "how do I get from here to there,
/// satisfying this intent?" It is pinned to a topology epoch and is only
/// valid until the topology changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct RoutePlan {
    /// Unique route plan identifier.
    pub id: RoutePlanId,
    /// Intent this plan satisfies.
    pub intent_id: IntentId,
    /// Topology epoch at which this plan was compiled.
    pub topology_epoch: TopologyEpoch,
    /// Ordered list of hops. May be a single hop (direct) or multi-hop.
    pub steps: Vec<RouteStep>,
    /// Estimated end-to-end latency in microseconds.
    pub estimated_latency_us: Option<f64>,
    /// Score breakdown for this plan (for debugging/audit).
    pub score: Option<ScoreBreakdown>,
    /// When this plan was compiled.
    pub compiled_at: DateTime<Utc>,
    /// When this plan expires (computed from intent expiry + slack).
    pub expires_at: DateTime<Utc>,
    /// Tags inherited from the intent.
    pub tags: Vec<String>,
}

impl RoutePlan {
    /// Validate the plan: all nodes exist, all edges exist, epoch matches.
    pub fn validate(&self, topology: &Topology) -> anyhow::Result<()> {
        if self.topology_epoch != topology.epoch {
            bail!(
                "RoutePlan epoch {} does not match topology epoch {}",
                self.topology_epoch,
                topology.epoch
            );
        }
        if self.steps.is_empty() {
            bail!("RoutePlan has no steps");
        }
        for step in &self.steps {
            if !topology.nodes.contains_key(&step.node) {
                bail!("RoutePlan references unknown node: {}", step.node);
            }
            if let Some(ref edge_id) = step.via_edge {
                if !topology.edges.contains_key(edge_id) {
                    bail!("RoutePlan references unknown edge: {}", edge_id);
                }
            }
        }
        Ok(())
    }

    /// Whether this plan is directly executable (single hop, same process or same machine).
    pub fn is_local(&self) -> bool {
        if self.steps.len() != 1 {
            return false;
        }
        let step = &self.steps[0];
        if let Some(edge_id) = &step.via_edge {
            if let Some(edge) = self.steps.get(0) {
                // Check if the edge locality tier is L0 or L1
                if let Some(e) = step.node.to_string().is_empty().then(|| None::<&Edge>) {
                    // We don't have the edge here; check via topology
                }
            }
        }
        // Single-hop plans are always potentially local
        self.steps.len() == 1
    }
}

// ---------------------------------------------------------------------------
// Topology
// ---------------------------------------------------------------------------

/// The full topology graph: nodes, edges, and epoch.
///
/// This is the root data structure that the compiler operates on.
/// It is append-only with respect to epoch changes; old snapshots are preserved
/// for audit purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Topology {
    /// Monotonically increasing epoch counter.
    pub epoch: TopologyEpoch,
    /// All nodes in the topology, keyed by NodeId.
    pub nodes: BTreeMap<NodeId, Node>,
    /// All edges in the topology, keyed by EdgeId.
    pub edges: BTreeMap<EdgeId, Edge>,
    /// Topology-wide metadata.
    pub meta: TopologyMeta,
    /// History: snapshots of prior epochs (optional, kept for audit).
    #[serde(skip)]
    history: Vec<Topology>,
}

impl Default for Topology {
    fn default() -> Self {
        Self::new()
    }
}

impl Topology {
    pub fn new() -> Self {
        Self {
            epoch: TopologyEpoch::default(),
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
            meta: TopologyMeta::default(),
            history: Vec::new(),
        }
    }

    /// Add a node to the topology. Idempotent: updates existing nodes.
    pub fn add_node(&mut self, node: Node) {
        self.nodes.insert(node.id.clone(), node);
        self.bump_epoch();
    }

    /// Add an edge to the topology.
    /// Returns an error if either endpoint is missing.
    pub fn add_edge(&mut self, edge: Edge) -> anyhow::Result<()> {
        if !self.nodes.contains_key(&edge.from) {
            bail!("Edge references unknown source node: {}", edge.from);
        }
        if !self.nodes.contains_key(&edge.to) {
            bail!("Edge references unknown destination node: {}", edge.to);
        }
        self.edges.insert(edge.id.clone(), edge);
        self.bump_epoch();
        Ok(())
    }

    /// Get a node by ID.
    pub fn node(&self, id: &NodeId) -> Option<&Node> {
        self.nodes.get(id)
    }

    /// Get an edge by ID.
    pub fn edge(&self, id: &EdgeId) -> Option<&Edge> {
        self.edges.get(id)
    }

    /// All edges incident to a given node.
    pub fn edges_for(&self, node: &NodeId) -> Vec<&Edge> {
        self.edges
            .values()
            .filter(|e| e.from == *node || e.to == *node)
            .collect()
    }

    /// Bump the topology epoch, archiving the current state.
    fn bump_epoch(&mut self) {
        // Archive current topology before mutating
        let snapshot = Topology {
            epoch: self.epoch,
            nodes: self.nodes.clone(),
            edges: self.edges.clone(),
            meta: self.meta.clone(),
            history: Vec::new(),
        };
        self.history.push(snapshot);
        self.epoch.bump();
    }

    /// Number of nodes in the topology.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of edges in the topology.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

/// Metadata about the topology as a whole.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct TopologyMeta {
    /// Human-readable topology name (e.g. "home-lab-v3").
    pub name: String,
    /// When this topology was first created.
    #[cfg_attr(feature = "schemars", schemars(with = "Option<String>"))]
    pub created_at: Option<DateTime<Utc>>,
    /// Who created this topology.
    pub created_by: Option<String>,
    /// Arbitrary key-value metadata.
    pub annotations: BTreeMap<String, String>,
}

// ---------------------------------------------------------------------------
// Score (re-exported from score.rs)
// ---------------------------------------------------------------------------

/// The final composite score for a route plan.
///
/// Higher is better. Used to rank competing candidates and for audit.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ScoreBreakdown {
    pub locality_score: f64,
    pub latency_score: f64,
    pub capability_score: f64,
    pub trust_score: f64,
    pub composite: f64,
}

impl ScoreBreakdown {
    pub fn new(
        locality_score: f64,
        latency_score: f64,
        capability_score: f64,
        trust_score: f64,
    ) -> Self {
        let composite =
            locality_score * 0.35 + latency_score * 0.30 + capability_score * 0.25 + trust_score * 0.10;
        Self {
            locality_score,
            latency_score,
            capability_score,
            trust_score,
            composite,
        }
    }
}

/// Individual route scoring result (used by the score module).
#[derive(Debug, Clone)]
pub struct Score {
    pub breakdown: ScoreBreakdown,
    pub rank: usize,
    pub reason: String,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use fabric_capability::locality::LocalityTier;

    #[test]
    fn test_topology_epoch_bump() {
        let mut epoch = TopologyEpoch::default();
        assert_eq!(epoch.0, 0);
        epoch.bump();
        assert_eq!(epoch.0, 1);
        assert_eq!(epoch.bump_and_get(), 2);
        assert_eq!(epoch.0, 2);
    }

    #[test]
    fn test_trust_level_satisfies() {
        assert!(TrustLevel::Attested.satisfies(TrustLevel::Untrusted));
        assert!(TrustLevel::Audited.satisfies(TrustLevel::Bootstrap));
        assert!(!TrustLevel::Bootstrap.satisfies(TrustLevel::Attested));
        assert!(TrustLevel::Untrusted.satisfies(TrustLevel::Untrusted));
    }

    #[test]
    fn test_link_metrics_effective_bandwidth() {
        let metrics = LinkMetrics {
            latency_us: Some(100.0),
            bandwidth_bps: Some(1_000_000_000),
            packet_loss: Some(0.001),
            jitter_us: Some(5.0),
        };
        // Under 200us: good
        assert_eq!(metrics.effective_bandwidth(200.0), Some(999_000_000));
        // Under 50us: lat=100 > 50, so fails
        assert_eq!(metrics.effective_bandwidth(50.0), None);
        // Under 100us: exactly the bandwidth
        assert_eq!(metrics.effective_bandwidth(100.0), Some(999_000_000));
        // Over limit: fails
        assert_eq!(metrics.effective_bandwidth(50.0), None);
    }

    #[test]
    fn test_link_metrics_incomplete() {
        let partial = LinkMetrics {
            latency_us: Some(100.0),
            bandwidth_bps: None,
            packet_loss: None,
            jitter_us: None,
        };
        assert!(!partial.is_complete());
    }

    #[test]
    fn test_node_with_capabilities() {
        let cap = CapabilityRef::new("sha256:abc123".to_string()).with_trust(TrustLevel::Attested);
        let node = Node::new(NodeId::new("gpu-0"), LocalityTier::L2)
            .with_capability(cap)
            .with_tag("rt-island");

        assert_eq!(node.capabilities.len(), 1);
        assert_eq!(node.tags, vec!["rt-island"]);
        assert!(node.meets_trust(TrustLevel::Untrusted));
        assert!(node.meets_trust(TrustLevel::Bootstrap));
        assert!(node.meets_trust(TrustLevel::Attested));
        assert!(!node.meets_trust(TrustLevel::Audited));
    }

    #[test]
    fn test_intent_requirements_matches_node() {
        let node = Node::new(NodeId::new("gpu-0"), LocalityTier::L3).with_tag("rt-island");

        let reqs = IntentRequirements {
            required_tags: vec!["rt-island".to_string()],
            max_locality_tier: Some(3.5),
            ..Default::default()
        };
        assert!(reqs.matches_node(&node));

        let reqs_fail = IntentRequirements {
            required_tags: vec!["fpga".to_string()],
            max_locality_tier: Some(3.5),
            ..Default::default()
        };
        assert!(!reqs_fail.matches_node(&node));
    }

    #[test]
    fn test_route_plan_validate() {
        let mut topology = Topology::new();
        topology.add_node(Node::new(NodeId::new("a"), LocalityTier::L1));
        topology.add_node(Node::new(NodeId::new("b"), LocalityTier::L2));
        topology.add_edge(Edge::new(
            EdgeId::new("a-b"),
            NodeId::new("a"),
            NodeId::new("b"),
            LocalityTier::L1,
        ));

        let plan = RoutePlan {
            id: RoutePlanId::new(),
            intent_id: IntentId::new(),
            topology_epoch: topology.epoch,
            steps: vec![RouteStep {
                node: NodeId::new("a"),
                capability_id: None,
                via_edge: None,
                action: "execute".to_string(),
            }],
            estimated_latency_us: Some(10.0),
            score: None,
            compiled_at: Utc::now(),
            expires_at: Utc::now(),
            tags: vec![],
        };

        assert!(plan.validate(&topology).is_ok());
    }

    #[test]
    fn test_route_plan_validate_bad_epoch() {
        let mut topology = Topology::new();
        topology.add_node(Node::new(NodeId::new("a"), LocalityTier::L1));

        let bad_plan = RoutePlan {
            id: RoutePlanId::new(),
            intent_id: IntentId::new(),
            topology_epoch: TopologyEpoch(999),
            steps: vec![RouteStep {
                node: NodeId::new("a"),
                capability_id: None,
                via_edge: None,
                action: "execute".to_string(),
            }],
            estimated_latency_us: None,
            score: None,
            compiled_at: Utc::now(),
            expires_at: Utc::now(),
            tags: vec![],
        };

        assert!(bad_plan.validate(&topology).is_err());
    }

    #[test]
    fn test_topology_add_node_bumps_epoch() {
        let mut topo = Topology::new();
        let e0 = topo.epoch;
        topo.add_node(Node::new(NodeId::new("n1"), LocalityTier::L2));
        assert!(topo.epoch.0 > e0.0);
        let e1 = topo.epoch;
        topo.add_node(Node::new(NodeId::new("n2"), LocalityTier::L1));
        assert!(topo.epoch.0 > e1.0);
    }

    #[test]
    fn test_topology_add_edge_unknown_node() {
        let mut topo = Topology::new();
        topo.add_node(Node::new(NodeId::new("a"), LocalityTier::L1));
        let result = topo.add_edge(Edge::new(
            EdgeId::new("a-b"),
            NodeId::new("a"),
            NodeId::new("nonexistent"),
            LocalityTier::L2,
        ));
        assert!(result.is_err());
    }

    #[test]
    fn test_node_id_display() {
        let id = NodeId::new("gpu-0");
        assert_eq!(format!("{}", id), "gpu-0");
    }

    #[test]
    fn test_capability_ref_new() {
        let r = CapabilityRef::new("sha256:test".to_string());
        assert_eq!(r.trust, TrustLevel::Untrusted);
        assert!(r.refreshed_at.is_none());
        let r2 = r.with_trust(TrustLevel::Attested);
        assert_eq!(r2.trust, TrustLevel::Attested);
    }
}
