//! Application state, tab navigation, and data refresh logic.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use serde::Deserialize;

use crate::types::*;

// ---------------------------------------------------------------------------
// Tab state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Dashboard,
    Topology,
    Routes,
    Leases,
}

impl Tab {
    pub fn all() -> &'static [Tab] {
        &[Tab::Dashboard, Tab::Topology, Tab::Routes, Tab::Leases]
    }

    pub fn title(&self) -> &str {
        match self {
            Tab::Dashboard => "Dashboard",
            Tab::Topology => "Topology",
            Tab::Routes => "Routes",
            Tab::Leases => "Leases",
        }
    }

    pub fn key(&self) -> char {
        match self {
            Tab::Dashboard => '1',
            Tab::Topology => '2',
            Tab::Routes => '3',
            Tab::Leases => '4',
        }
    }
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

pub struct App {
    pub running: bool,
    pub active_tab: Tab,
    pub topology: TopologyResponse,
    pub health: HealthResponse,
    pub routes: RoutesResponse,
    pub leases: LeasesResponse,
    pub selected_row: usize,
    pub last_refresh: Instant,
    pub connect_addr: String,
    pub error_msg: Option<String>,
    pub db_path: Option<String>,
}

impl App {
    pub fn new(connect_addr: String, db_path: Option<String>) -> Self {
        Self {
            running: true,
            active_tab: Tab::Dashboard,
            topology: TopologyResponse::default(),
            health: HealthResponse::default(),
            routes: RoutesResponse::default(),
            leases: LeasesResponse::default(),
            selected_row: 0,
            last_refresh: Instant::now() - Duration::from_secs(10), // force initial refresh
            connect_addr,
            error_msg: None,
            db_path,
        }
    }

    pub fn refresh(&mut self) {
        let db_path = self.db_path.clone();
        if let Some(ref path) = db_path {
            self.refresh_from_db(path);
        } else {
            self.refresh_from_daemon();
        }
        self.last_refresh = Instant::now();
    }

    fn refresh_from_daemon(&mut self) {
        self.error_msg = None;

        // Health
        if let Ok(h) = fetch_daemon_json::<HealthResponse>(&self.connect_addr, "health_check") {
            self.health = h;
        } else {
            self.error_msg = Some(format!("Cannot reach daemon at {}", self.connect_addr));
            return;
        }

        // Topology
        if let Ok(t) = fetch_daemon_json::<TopologyResponse>(&self.connect_addr, "topology_request")
        {
            self.topology = t;
        }

        // Routes
        if let Ok(r) = fetch_daemon_json::<RoutesResponse>(&self.connect_addr, "routes_request") {
            self.routes = r;
        }

        // Leases
        if let Ok(l) = fetch_daemon_json::<LeasesResponse>(&self.connect_addr, "leases_request") {
            self.leases = l;
        }
    }

    fn refresh_from_db(&mut self, path: &str) {
        self.error_msg = None;
        match fabric_persist::Persist::open(path) {
            Ok(persist) => {
                if let Ok(Some(topo)) = persist.load_topology() {
                    self.topology = TopologyResponse {
                        nodes: topo
                            .nodes
                            .values()
                            .map(|n| TopoNode {
                                id: n.id.0.clone(),
                                label: n.label.clone(),
                                locality: format!("{:?}", n.locality_tier),
                                tags: n.tags.clone(),
                            })
                            .collect(),
                        edges: topo
                            .edges
                            .values()
                            .map(|e| TopoEdge {
                                from: e.from.0.clone(),
                                to: e.to.0.clone(),
                                locality: format!("{:?}", e.locality_tier),
                            })
                            .collect(),
                        epoch: topo.epoch.0,
                    };
                    self.health.node_count = self.topology.nodes.len();
                    self.health.edge_count = self.topology.edges.len();
                    self.health.epoch = self.topology.epoch;
                }
                if let Ok(state) = persist.recover_state() {
                    self.health.lease_count = state.active_leases.len();
                    self.health.route_count = state.active_plans.len();
                    self.leases = LeasesResponse {
                        leases: state
                            .active_leases
                            .iter()
                            .map(|l| LeaseInfo {
                                handle: format!("{}", l.handle.0),
                                protocol: format!("{:?}", l.spec.protocol),
                                state: format!("{:?}", l.state),
                                name: l.spec.name.clone(),
                            })
                            .collect(),
                    };
                }
            }
            Err(e) => {
                self.error_msg = Some(format!("DB error: {e}"));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Wire protocol helpers
// ---------------------------------------------------------------------------

fn fetch_daemon_json<T: for<'de> Deserialize<'de>>(addr: &str, msg_type: &str) -> Result<T, String> {
    let mut stream =
        TcpStream::connect(addr).map_err(|e| format!("connect failed: {e}"))?;
    stream.set_read_timeout(Some(Duration::from_secs(3))).ok();
    stream.set_write_timeout(Some(Duration::from_secs(3))).ok();

    let msg = format!("{{\"type\":\"{}\"}}\n", msg_type);
    stream
        .write_all(msg.as_bytes())
        .map_err(|e| format!("write failed: {e}"))?;

    let mut reader = BufReader::new(&stream);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|e| format!("read failed: {e}"))?;

    serde_json::from_str(&line).map_err(|e| format!("parse failed: {e}"))
}
