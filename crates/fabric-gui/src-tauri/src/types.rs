//! Shared data types for wire-protocol responses and application state.
//!
//! These types mirror the existing fabric-gui/src/app.rs types but add
//! `Serialize` for Tauri command responses.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Wire-protocol response types (mirrors fabric-tui/src/types.rs)
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TopologyResponse {
    pub nodes: Vec<TopoNode>,
    pub edges: Vec<TopoEdge>,
    pub epoch: u64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TopoNode {
    pub id: String,
    pub label: Option<String>,
    pub locality: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TopoEdge {
    pub from: String,
    pub to: String,
    pub locality: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RoutesResponse {
    pub routes: Vec<RouteInfo>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RouteInfo {
    pub id: String,
    pub steps: usize,
    pub source: String,
    pub destination: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct LeasesResponse {
    pub leases: Vec<LeaseInfo>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct LeaseInfo {
    pub handle: String,
    pub protocol: String,
    pub state: String,
    pub name: String,
}

// ---------------------------------------------------------------------------
// Network status types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkStatus {
    pub daemon_connected: bool,
    pub tailscale_connected: bool,
    pub upnp_active: bool,
    pub nat_type: String,
    pub public_ip: String,
    pub tailscale_peers: Vec<TailscalePeer>,
    pub upnp_mappings: Vec<UpnpMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TailscalePeer {
    pub hostname: String,
    pub ip: String,
    pub latency_ms: f64,
    pub online: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpnpMapping {
    pub protocol: String,
    pub internal_port: u16,
    pub external_port: u16,
    pub description: String,
}

// ---------------------------------------------------------------------------
// Streaming stats types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingStats {
    pub active_sessions: Vec<StreamSession>,
    pub frames_sent: u64,
    pub frames_dropped: u64,
    pub latency_ms: f64,
    pub bandwidth_mbps: f64,
    pub codec: String,
    pub resolution: String,
    pub fps: u32,
    pub bitrate_kbps: u32,
}

impl Default for StreamingStats {
    fn default() -> Self {
        Self {
            active_sessions: vec![],
            frames_sent: 0,
            frames_dropped: 0,
            latency_ms: 0.0,
            bandwidth_mbps: 0.0,
            codec: "H.264".into(),
            resolution: "1920x1080".into(),
            fps: 60,
            bitrate_kbps: 8000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamSession {
    pub id: String,
    pub target: String,
    pub codec: String,
    pub resolution: String,
    pub state: String,
}

// ---------------------------------------------------------------------------
// Auth status types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthStatus {
    pub logged_in: bool,
    pub user_name: String,
    pub user_email: String,
    pub org_name: String,
    pub roles: Vec<String>,
    pub session_expiry_secs: u64,
    pub active_sessions: Vec<AuthSession>,
}

impl Default for AuthStatus {
    fn default() -> Self {
        Self {
            logged_in: false,
            user_name: "-".into(),
            user_email: "-".into(),
            org_name: "-".into(),
            roles: vec![],
            session_expiry_secs: 0,
            active_sessions: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSession {
    pub session_id: String,
    pub device: String,
    pub created: String,
}

// ---------------------------------------------------------------------------
// Log entry type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

// ---------------------------------------------------------------------------
// Settings state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsState {
    pub theme: String,
    pub auto_refresh_secs: u64,
    pub startup_tab: String,
    pub daemon_address: String,
    pub tailscale_enabled: bool,
    pub upnp_enabled: bool,
    pub stun_server: String,
    pub default_codec: String,
    pub max_bitrate_kbps: u32,
    pub keyframe_interval: u32,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            theme: "Dark".into(),
            auto_refresh_secs: 5,
            startup_tab: "Dashboard".into(),
            daemon_address: "127.0.0.1:9400".into(),
            tailscale_enabled: true,
            upnp_enabled: true,
            stun_server: "stun.l.google.com:19302".into(),
            default_codec: "H.264".into(),
            max_bitrate_kbps: 10000,
            keyframe_interval: 2,
        }
    }
}

// ---------------------------------------------------------------------------
// Aggregate data payload for refresh_data command
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct GuiData {
    pub health: HealthResponse,
    pub topology: TopologyResponse,
    pub routes: RoutesResponse,
    pub leases: LeasesResponse,
    pub network: NetworkStatus,
    pub streaming: StreamingStats,
    pub auth: AuthStatus,
    pub logs: Vec<LogEntry>,
    pub settings: SettingsState,
}

// ---------------------------------------------------------------------------
// Auth start (OAuth URL generation)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthStartResponse {
    pub url: String,
    pub state: String,
}

// ---------------------------------------------------------------------------
// Email auth response
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAuthResponse {
    pub success: bool,
    pub message: String,
}

// ---------------------------------------------------------------------------
// Daemon status (serializable snapshot of daemon lifecycle)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStatusResponse {
    pub state: String,
    pub pid: Option<u32>,
    pub uptime_secs: Option<u64>,
    pub restart_count: u32,
    pub listen_addr: String,
}


