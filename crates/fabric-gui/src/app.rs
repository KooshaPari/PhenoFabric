//! Application state, wire-protocol types, and data refresh for fabric-gui.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde::Deserialize;

// Tab navigation

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab { Dashboard, Topology, Routes, Leases }

impl Tab {
    pub const ALL: &'static [Tab] = &[Tab::Dashboard, Tab::Topology, Tab::Routes, Tab::Leases];
    pub fn title(self) -> &'static str {
        match self { Tab::Dashboard => "Dashboard", Tab::Topology => "Topology",
                     Tab::Routes => "Routes", Tab::Leases => "Leases" }
    }
    pub fn index(self) -> usize {
        match self { Tab::Dashboard => 0, Tab::Topology => 1, Tab::Routes => 2, Tab::Leases => 3 }
    }
}

// Wire-protocol response types (mirrors fabric-tui/src/types.rs)

#[derive(Debug, Default, Clone, Deserialize)]
pub struct TopologyResponse { pub nodes: Vec<TopoNode>, pub edges: Vec<TopoEdge>, pub epoch: u64 }
#[derive(Debug, Default, Clone, Deserialize)]
pub struct TopoNode { pub id: String, pub label: Option<String>, pub locality: String, pub tags: Vec<String> }
#[derive(Debug, Default, Clone, Deserialize)]
pub struct TopoEdge { pub from: String, pub to: String, pub locality: String }
#[derive(Debug, Default, Clone, Deserialize)]
pub struct HealthResponse {
    pub daemon_healthy: bool, pub uptime_s: u64, pub node_count: usize, pub edge_count: usize,
    pub cap_count: usize, pub route_count: usize, pub lease_count: usize, pub epoch: u64,
}
#[derive(Debug, Default, Clone, Deserialize)]
pub struct RoutesResponse { pub routes: Vec<RouteInfo> }
#[derive(Debug, Default, Clone, Deserialize)]
pub struct RouteInfo { pub id: String, pub steps: usize, pub source: String, pub destination: String }
#[derive(Debug, Default, Clone, Deserialize)]
pub struct LeasesResponse { pub leases: Vec<LeaseInfo> }
#[derive(Debug, Default, Clone, Deserialize)]
pub struct LeaseInfo { pub handle: String, pub protocol: String, pub state: String, pub name: String }

// Application state

enum RefreshMsg { Data(GuiData), Error(String) }

#[derive(Debug, Default, Clone)]
pub struct GuiData {
    pub health: HealthResponse, pub topology: TopologyResponse,
    pub routes: RoutesResponse, pub leases: LeasesResponse,
}

pub struct GuiApp {
    pub active_tab: Tab, pub data: GuiData, pub connect_addr: String,
    pub db_path: Option<String>, pub error_msg: Option<String>,
    pub last_refresh: Instant, pub refresh_interval: Duration, pub dark_mode: bool,
    rx: Option<mpsc::Receiver<RefreshMsg>>, refresh_in_flight: bool,
}

impl GuiApp {
    pub fn new(connect_addr: String, db_path: Option<String>) -> Self {
        Self {
            active_tab: Tab::Dashboard, data: GuiData::default(), connect_addr, db_path,
            error_msg: None, last_refresh: Instant::now() - Duration::from_secs(10),
            refresh_interval: Duration::from_secs(5), dark_mode: true,
            rx: None, refresh_in_flight: false,
        }
    }

    pub fn refresh(&mut self) {
        if self.refresh_in_flight { return; }
        let addr = self.connect_addr.clone();
        let db = self.db_path.clone();
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        self.refresh_in_flight = true;
        std::thread::spawn(move || {
            let msg = match fetch_all(&addr, db.as_deref()) {
                Ok(d) => RefreshMsg::Data(d), Err(e) => RefreshMsg::Error(e),
            };
            let _ = tx.send(msg);
        });
    }

    pub fn poll_refresh(&mut self) {
        let Some(rx) = self.rx.take() else { return };
        match rx.try_recv() {
            Ok(RefreshMsg::Data(d)) => { self.data = d; self.error_msg = None; self.last_refresh = Instant::now(); self.refresh_in_flight = false; }
            Ok(RefreshMsg::Error(e)) => { self.error_msg = Some(e); self.last_refresh = Instant::now(); self.refresh_in_flight = false; }
            Err(mpsc::TryRecvError::Empty) => { self.rx = Some(rx); }
            Err(mpsc::TryRecvError::Disconnected) => { self.error_msg = Some("refresh thread crashed".into()); self.refresh_in_flight = false; }
        }
    }

    pub fn should_auto_refresh(&self) -> bool { self.last_refresh.elapsed() >= self.refresh_interval }
    pub fn toggle_dark_mode(&mut self) { self.dark_mode = !self.dark_mode; }

    pub fn parse_args() -> (String, Option<String>) {
        let mut connect = "127.0.0.1:9400".to_string();
        let mut db = None;
        let args: Vec<String> = std::env::args().skip(1).collect();
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--connect" | "-c" => { if let Some(v) = args.get(i+1) { connect = v.clone(); i += 2; } else { i += 1; } }
                "--db" | "-d" => { if let Some(v) = args.get(i+1) { db = Some(v.clone()); i += 2; } else { i += 1; } }
                _ => i += 1,
            }
        }
        (connect, db)
    }

    pub fn handle_keys(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            for event in &i.events {
                if let egui::Event::Key { pressed: true, key, .. } = event {
                    match key {
                        egui::Key::Tab => { let n = (self.active_tab.index()+1) % Tab::ALL.len(); self.active_tab = Tab::ALL[n]; }
                        egui::Key::R => self.refresh(), _ => {}
                    }
                }
                if let egui::Event::Text(text) = event {
                    match text.as_str() {
                        "1" => self.active_tab = Tab::Dashboard, "2" => self.active_tab = Tab::Topology,
                        "3" => self.active_tab = Tab::Routes, "4" => self.active_tab = Tab::Leases, _ => {}
                    }
                }
            }
        });
    }

    pub fn configure_visuals(&self, ctx: &egui::Context) {
        ctx.set_visuals(if self.dark_mode { egui::Visuals::dark() } else { egui::Visuals::light() });
    }
}

// Data fetching

fn fetch_all(addr: &str, db_path: Option<&str>) -> Result<GuiData, String> {
    let health = fetch_daemon_json::<HealthResponse>(addr, "health_check").ok();
    let topology = fetch_daemon_json::<TopologyResponse>(addr, "topology_request").ok();
    let routes = fetch_daemon_json::<RoutesResponse>(addr, "routes_request").ok();
    let leases = fetch_daemon_json::<LeasesResponse>(addr, "leases_request").ok();
    if health.is_some() || topology.is_some() {
        return Ok(GuiData {
            health: health.unwrap_or_default(), topology: topology.unwrap_or_default(),
            routes: routes.unwrap_or_default(), leases: leases.unwrap_or_default(),
        });
    }
    if let Some(path) = db_path { return fetch_from_db(path); }
    Err(format!("Cannot reach daemon at {addr} and no DB path given"))
}

fn fetch_daemon_json<T: for<'de> Deserialize<'de>>(addr: &str, msg_type: &str) -> Result<T, String> {
    let mut stream = TcpStream::connect(addr).map_err(|e| format!("connect: {e}"))?;
    stream.set_read_timeout(Some(Duration::from_secs(3))).ok();
    stream.set_write_timeout(Some(Duration::from_secs(3))).ok();
    stream.write_all(format!("{{\"type\":\"{msg_type}\"}}\n").as_bytes()).map_err(|e| format!("write: {e}"))?;
    let mut reader = BufReader::new(&stream);
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|e| format!("read: {e}"))?;
    serde_json::from_str(&line).map_err(|e| format!("parse: {e}"))
}

fn fetch_from_db(path: &str) -> Result<GuiData, String> {
    let persist = fabric_persist::Persist::open(path).map_err(|e| format!("db open: {e}"))?;
    let mut data = GuiData::default();
    if let Ok(Some(topo)) = persist.load_topology() {
        data.topology = TopologyResponse {
            epoch: topo.epoch.0,
            nodes: topo.nodes.values().map(|n| TopoNode {
                id: n.id.0.clone(), label: n.label.clone(),
                locality: n.locality_tier.short_code().to_string(), tags: n.tags.clone(),
            }).collect(),
            edges: topo.edges.values().map(|e| TopoEdge {
                from: e.from.0.clone(), to: e.to.0.clone(), locality: e.locality_tier.short_code().to_string(),
            }).collect(),
        };
        data.health.node_count = data.topology.nodes.len();
        data.health.edge_count = data.topology.edges.len();
        data.health.epoch = data.topology.epoch;
    }
    if let Ok(state) = persist.recover_state() {
        data.health.lease_count = state.active_leases.len();
        data.health.route_count = state.active_plans.len();
        data.leases = LeasesResponse { leases: state.active_leases.iter().map(|l| LeaseInfo {
            handle: format!("{}", l.handle.0), protocol: format!("{:?}", l.spec.protocol),
            state: format!("{:?}", l.state), name: l.spec.name.clone(),
        }).collect() };
        data.routes = RoutesResponse { routes: state.active_plans.iter().map(|p| RouteInfo {
            id: format!("{}", p.id.0), steps: p.steps.len(),
            source: p.steps.first().map(|s| s.node.0.clone()).unwrap_or_default(),
            destination: p.steps.last().map(|s| s.node.0.clone()).unwrap_or_default(),
        }).collect() };
    }
    Ok(data)
}
