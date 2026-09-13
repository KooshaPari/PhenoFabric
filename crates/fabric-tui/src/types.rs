//! Data types for fabric-tui wire protocol and DB responses.

use serde::Deserialize;

#[derive(Debug, Default, Clone, Deserialize)]
pub struct TopologyResponse {
    pub nodes: Vec<TopoNode>,
    pub edges: Vec<TopoEdge>,
    pub epoch: u64,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct TopoNode {
    pub id: String,
    pub label: Option<String>,
    pub locality: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct TopoEdge {
    pub from: String,
    pub to: String,
    pub locality: String,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct HealthResponse {
    pub daemon_healthy: bool,
    pub uptime_s: u64,
    pub node_count: usize,
    pub edge_count: usize,
    pub cap_count: usize,
    pub route_count: usize,
    pub lease_count: usize,
    pub epoch: u64,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct RoutesResponse {
    pub routes: Vec<RouteInfo>,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct RouteInfo {
    pub id: String,
    pub steps: usize,
    pub source: String,
    pub destination: String,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct LeasesResponse {
    pub leases: Vec<LeaseInfo>,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct LeaseInfo {
    pub handle: String,
    pub protocol: String,
    pub state: String,
    pub name: String,
}
