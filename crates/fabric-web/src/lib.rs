//! fabric-web: Shared types and API contracts for the Fabric web frontend.
//!
//! This crate defines the API request/response types used between the
//! daemon's wire server HTTP API and the browser-based SPA frontend.
//! The actual Leptos SPA is built separately with trunk + WASM.

use fabric_graph::model::TrustLevel;
use fabric_tray::DaemonStatus;
use serde::{Deserialize, Serialize};

// --- API Response Types ---

/// Topology response for the web frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyResponse {
    pub nodes: Vec<TopologyNodeView>,
    pub edges: Vec<TopologyEdgeView>,
}

/// Simplified node view for web display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyNodeView {
    pub id: String,
    pub label: Option<String>,
    pub locality: String,
    pub cap_count: usize,
    pub tags: Vec<String>,
}

/// Simplified edge view for web display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyEdgeView {
    pub id: String,
    pub from: String,
    pub to: String,
    pub locality: String,
    pub up: bool,
}

/// Routes response for the web frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutesResponse {
    pub routes: Vec<RoutePlanView>,
}

/// Route plan view for web display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutePlanView {
    pub intent: String,
    pub steps: usize,
    pub cost: f64,
    pub trust_level: String,
    pub file_name: String,
}

/// Capabilities response for the web frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitiesResponse {
    pub capabilities: Vec<CapabilityView>,
}

/// Capability descriptor view for web display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityView {
    pub node_name: String,
    pub descriptor_id: String,
    pub trust: String,
}

/// Health status response for the web frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub daemon_status: String,
    pub daemon_healthy: bool,
    pub node_count: usize,
    pub edge_count: usize,
    pub cap_count: usize,
    pub route_count: usize,
}

// --- Conversion helpers ---

impl TopologyNodeView {
    /// Create from a fabric-graph Node.
    pub fn from_node(node: &fabric_graph::model::Node) -> Self {
        Self {
            id: node.id.0.clone(),
            label: node.label.clone(),
            locality: format!("{:?}", node.locality_tier),
            cap_count: node.capabilities.len(),
            tags: node.tags.clone(),
        }
    }
}

impl TopologyEdgeView {
    /// Create from a fabric-graph Edge.
    pub fn from_edge(edge: &fabric_graph::model::Edge) -> Self {
        Self {
            id: edge.id.0.clone(),
            from: edge.from.0.clone(),
            to: edge.to.0.clone(),
            locality: format!("{:?}", edge.locality_tier),
            up: edge.up,
        }
    }
}

impl CapabilityView {
    /// Create from a node and capability ref.
    pub fn from_node_cap(
        node: &fabric_graph::model::Node,
        cap: &fabric_graph::model::CapabilityRef,
    ) -> Self {
        Self {
            node_name: node.label.clone().unwrap_or_else(|| node.id.0.clone()),
            descriptor_id: cap.descriptor_id.clone(),
            trust: trust_label(cap.trust).to_string(),
        }
    }
}

impl HealthResponse {
    /// Create from daemon status and workspace data.
    pub fn from_workspace(
        status: DaemonStatus,
        node_count: usize,
        edge_count: usize,
        cap_count: usize,
        route_count: usize,
    ) -> Self {
        Self {
            daemon_status: status.label().to_string(),
            daemon_healthy: status.is_running(),
            node_count,
            edge_count,
            cap_count,
            route_count,
        }
    }
}

/// Format trust level as a display string.
pub fn trust_label(level: TrustLevel) -> &'static str {
    match level {
        TrustLevel::Untrusted => "Untrusted",
        TrustLevel::Bootstrap => "Bootstrap",
        TrustLevel::Attested => "Attested",
        TrustLevel::Audited => "Audited",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_label_all_variants() {
        assert_eq!(trust_label(TrustLevel::Untrusted), "Untrusted");
        assert_eq!(trust_label(TrustLevel::Bootstrap), "Bootstrap");
        assert_eq!(trust_label(TrustLevel::Attested), "Attested");
        assert_eq!(trust_label(TrustLevel::Audited), "Audited");
    }

    #[test]
    fn health_response_from_workspace() {
        let h = HealthResponse::from_workspace(
            DaemonStatus::Healthy,
            5,
            8,
            3,
            2,
        );
        assert!(h.daemon_healthy);
        assert_eq!(h.node_count, 5);
        assert_eq!(h.edge_count, 8);
        assert_eq!(h.cap_count, 3);
        assert_eq!(h.route_count, 2);
        assert_eq!(h.daemon_status, "Running");
    }

    #[test]
    fn health_response_stopped() {
        let h = HealthResponse::from_workspace(
            DaemonStatus::Stopped,
            0,
            0,
            0,
            0,
        );
        assert!(!h.daemon_healthy);
        assert_eq!(h.daemon_status, "Stopped");
    }

    #[test]
    fn topology_node_view_serialization() {
        let view = TopologyNodeView {
            id: "node-1".to_string(),
            label: Some("test-node".to_string()),
            locality: "L6Lan".to_string(),
            cap_count: 2,
            tags: vec!["gpu".to_string()],
        };
        let json = serde_json::to_string(&view).unwrap();
        let parsed: TopologyNodeView = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "node-1");
        assert_eq!(parsed.cap_count, 2);
    }

    #[test]
    fn serialization_roundtrip() {
        let health = HealthResponse {
            daemon_status: "Running".to_string(),
            daemon_healthy: true,
            node_count: 3,
            edge_count: 5,
            cap_count: 2,
            route_count: 1,
        };
        let json = serde_json::to_string(&health).unwrap();
        let parsed: HealthResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.daemon_healthy, true);
        assert_eq!(parsed.node_count, 3);
    }
}
