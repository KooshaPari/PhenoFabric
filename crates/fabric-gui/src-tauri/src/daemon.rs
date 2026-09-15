//! Async daemon process manager and TCP wire-protocol client.
//!
//! Communicates with fabric-daemon over TCP using the same JSON-line protocol
//! as the existing egui GUI: send `{"type":"<msg_type>"}\n`, read JSON response.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde::de::DeserializeOwned;

use crate::types::*;

// ---------------------------------------------------------------------------
// Constants (matches daemon_manager.rs in fabric-gui)
// ---------------------------------------------------------------------------

const MAX_RESTARTS: u32 = 5;
const BASE_BACKOFF_SECS: u64 = 1;
const MAX_BACKOFF_SECS: u64 = 30;
const HEALTH_POLL_INTERVAL: Duration = Duration::from_secs(5);
const HEALTH_CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
const TCP_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_LOG_LINES: usize = 500;

// ---------------------------------------------------------------------------
// Daemon configuration
// ---------------------------------------------------------------------------

pub struct DaemonConfig {
    pub daemon_path: Option<String>,
    pub config_path: Option<String>,
    pub db_path: Option<String>,
    pub listen_addr: String,
}

impl DaemonConfig {
    pub fn default_config() -> Self {
        Self {
            daemon_path: None,
            config_path: None,
            db_path: None,
            listen_addr: "127.0.0.1:9400".into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Daemon lifecycle state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum DaemonLifecycle {
    NotStarted,
    Starting { started_at: Instant },
    Running { pid: u32, started_at: Instant },
    Failed { error: String, last_attempt: Instant },
    Stopped,
}

impl DaemonLifecycle {
    pub fn label(&self) -> &str {
        match self {
            Self::NotStarted => "Not Started",
            Self::Starting { .. } => "Starting",
            Self::Running { .. } => "Running",
            Self::Failed { .. } => "Failed",
            Self::Stopped => "Stopped",
        }
    }

    pub fn is_healthy(&self) -> bool {
        matches!(self, Self::Running { .. })
    }

    pub fn pid(&self) -> Option<u32> {
        if let Self::Running { pid, .. } = self {
            Some(*pid)
        } else {
            None
        }
    }

    pub fn uptime_secs(&self) -> Option<u64> {
        if let Self::Running { started_at, .. } = self {
            Some(started_at.elapsed().as_secs())
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// DaemonManager
// ---------------------------------------------------------------------------

pub struct DaemonManager {
    config: DaemonConfig,
    lifecycle: DaemonLifecycle,
    child: Option<Child>,
    restart_count: u32,
    last_health_check: Instant,
    log_buffer: Vec<String>,
}

impl DaemonManager {
    /// Create a new daemon manager with the given configuration.
    pub fn new(config: DaemonConfig) -> Self {
        Self {
            config,
            lifecycle: DaemonLifecycle::NotStarted,
            child: None,
            restart_count: 0,
            last_health_check: Instant::now() - HEALTH_POLL_INTERVAL,
            log_buffer: Vec::new(),
        }
    }

    /// Current listen address.
    pub fn listen_addr(&self) -> &str {
        &self.config.listen_addr
    }

    /// Whether the daemon is healthy.
    pub fn is_healthy(&self) -> bool {
        self.lifecycle.is_healthy()
    }

    /// Snapshot of current status for the frontend.
    pub fn status_snapshot(&self) -> DaemonStatusResponse {
        DaemonStatusResponse {
            state: self.lifecycle.label().to_string(),
            pid: self.lifecycle.pid(),
            uptime_secs: self.lifecycle.uptime_secs(),
            restart_count: self.restart_count,
            listen_addr: self.config.listen_addr.clone(),
        }
    }

    /// Recent log lines.
    pub fn recent_logs(&self) -> Vec<LogEntry> {
        self.log_buffer
            .iter()
            .enumerate()
            .map(|(_i, msg)| LogEntry {
                timestamp: chrono::Local::now()
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string(),
                level: if msg.contains("error") || msg.contains("fail") {
                    "ERROR".into()
                } else if msg.contains("warn") {
                    "WARN".into()
                } else {
                    "INFO".into()
                },
                message: msg.clone(),
            })
            .collect()
    }

    // -- Process lifecycle ---------------------------------------------------

    /// Resolve the path to the fabric-daemon binary.
    fn resolve_daemon_path(&self) -> Result<String, String> {
        if let Some(ref path) = self.config.daemon_path {
            return Ok(path.clone());
        }

        // Look next to the GUI binary
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let name = if cfg!(windows) {
                    "fabric-daemon.exe"
                } else {
                    "fabric-daemon"
                };
                let candidate = dir.join(name);
                if candidate.exists() {
                    return Ok(candidate.to_string_lossy().to_string());
                }
            }
        }

        // Search PATH
        let name = if cfg!(windows) {
            "fabric-daemon.exe"
        } else {
            "fabric-daemon"
        };
        if let Ok(path_var) = std::env::var("PATH") {
            for dir in path_var.split(':') {
                let candidate = std::path::PathBuf::from(dir).join(name);
                if candidate.exists() {
                    return Ok(candidate.to_string_lossy().to_string());
                }
            }
        }

        Err("fabric-daemon binary not found in PATH or next to GUI binary".into())
    }

    /// Start the daemon process.
    pub fn start(&mut self) -> Result<(), String> {
        if matches!(
            self.lifecycle,
            DaemonLifecycle::Running { .. } | DaemonLifecycle::Starting { .. }
        ) {
            return Err("Daemon is already running or starting".into());
        }

        let exe_path = self.resolve_daemon_path()?;
        let mut cmd = Command::new(&exe_path);
        cmd.arg("start");
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        if let Some(ref config) = self.config.config_path {
            cmd.arg("--config").arg(config);
        }
        if let Some(ref db) = self.config.db_path {
            cmd.arg("--db").arg(db);
        }
        cmd.arg("--listen").arg(&self.config.listen_addr);

        self.push_log(format!("[daemon] launching: {exe_path}"));
        let child = cmd.spawn().map_err(|e| {
            let msg = format!("Failed to spawn daemon: {e}");
            self.push_log(format!("[daemon] {msg}"));
            self.lifecycle = DaemonLifecycle::Failed {
                error: msg.clone(),
                last_attempt: Instant::now(),
            };
            msg
        })?;

        let pid = child.id();
        self.push_log(format!("[daemon] started with PID {pid}"));
        self.lifecycle = DaemonLifecycle::Starting {
            started_at: Instant::now(),
        };
        self.child = Some(child);
        Ok(())
    }

    /// Stop the daemon process gracefully, then force-kill if needed.
    pub fn stop(&mut self) -> Result<(), String> {
        if !matches!(
            self.lifecycle,
            DaemonLifecycle::Running { .. } | DaemonLifecycle::Starting { .. }
        ) {
            self.lifecycle = DaemonLifecycle::Stopped;
            return Ok(());
        }

        self.push_log("[daemon] sending SIGTERM / terminating".into());

        let mut child = match self.child.take() {
            Some(c) => c,
            None => {
                self.lifecycle = DaemonLifecycle::Stopped;
                return Ok(());
            }
        };

        if let Err(e) = child.kill() {
            self.push_log(format!("[daemon] kill error: {e}"));
        }

        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            match child.try_wait() {
                Ok(Some(_)) => {
                    self.push_log("[daemon] stopped gracefully".into());
                    self.lifecycle = DaemonLifecycle::Stopped;
                    self.restart_count = 0;
                    return Ok(());
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(100)),
                Err(e) => {
                    self.push_log(format!("[daemon] wait error: {e}"));
                    break;
                }
            }
        }

        let _ = child.kill();
        let _ = child.wait();

        self.lifecycle = DaemonLifecycle::Stopped;
        self.restart_count = 0;
        self.push_log("[daemon] stopped".into());
        Ok(())
    }

    /// Restart with exponential backoff.
    pub fn restart(&mut self) -> Result<(), String> {
        self.stop()?;
        self.restart_count += 1;

        let delay_secs = std::cmp::min(
            BASE_BACKOFF_SECS * 2u64.pow(self.restart_count.saturating_sub(1)),
            MAX_BACKOFF_SECS,
        );
        self.push_log(format!(
            "[daemon] restart attempt {}/{MAX_RESTARTS} in {delay_secs}s",
            self.restart_count
        ));
        std::thread::sleep(Duration::from_secs(delay_secs));

        self.start()
    }

    /// Health check via TCP wire protocol.
    fn check_health_tcp(&self, addr: &str) -> bool {
        let parsed: std::net::SocketAddr = match addr.parse() {
            Ok(a) => a,
            Err(_) => return false,
        };

        let stream = match TcpStream::connect_timeout(&parsed, HEALTH_CONNECT_TIMEOUT) {
            Ok(s) => s,
            Err(_) => return false,
        };

        stream
            .set_read_timeout(Some(HEALTH_CONNECT_TIMEOUT))
            .ok();
        stream
            .set_write_timeout(Some(HEALTH_CONNECT_TIMEOUT))
            .ok();

        let mut stream = stream;
        if stream
            .write_all(b"{\"type\":\"health_check\"}\n")
            .is_err()
        {
            return false;
        }

        let reader = BufReader::new(&stream);
        for line in reader.lines().take(1) {
            if let Ok(l) = line {
                if l.contains("\"daemon_healthy\":true") || l.contains("\"daemon_healthy\": true") {
                    return true;
                }
            }
        }
        false
    }

    /// Poll health and manage transitions. Call periodically.
    pub fn poll(&mut self) {
        if self.last_health_check.elapsed() < HEALTH_POLL_INTERVAL {
            return;
        }
        self.last_health_check = Instant::now();

        let addr = self.config.listen_addr.clone();
        let state = self.lifecycle.clone();
        match state {
            DaemonLifecycle::Starting { started_at } => {
                let healthy = self.check_health_tcp(&addr);
                if healthy || started_at.elapsed() > Duration::from_secs(10) {
                    if healthy {
                        let pid = self.child.as_ref().map(|c| c.id()).unwrap_or(0);
                        self.lifecycle = DaemonLifecycle::Running {
                            pid,
                            started_at,
                        };
                        self.push_log("[daemon] became healthy".into());
                        self.restart_count = 0;
                    } else if let Some(ref mut child) = self.child {
                        match child.try_wait() {
                            Ok(Some(status)) => {
                                let msg = format!("Daemon exited with status: {status}");
                                self.push_log(format!("[daemon] {msg}"));
                                self.lifecycle = DaemonLifecycle::Failed {
                                    error: msg,
                                    last_attempt: Instant::now(),
                                };
                                self.child = None;
                            }
                            _ => {}
                        }
                    }
                }
            }
            DaemonLifecycle::Running { .. } => {
                if !self.check_health_tcp(&addr) {
                    self.push_log("[daemon] health check failed".into());
                    if let Some(ref mut child) = self.child {
                        match child.try_wait() {
                            Ok(Some(status)) => {
                                let msg = format!("Daemon exited: {status}");
                                self.push_log(format!("[daemon] {msg}"));
                                self.lifecycle = DaemonLifecycle::Failed {
                                    error: msg,
                                    last_attempt: Instant::now(),
                                };
                                self.child = None;
                            }
                            Ok(None) => {
                                self.push_log("[daemon] health degraded, monitoring".into());
                            }
                            Err(_) => {}
                        }
                    } else {
                        self.lifecycle = DaemonLifecycle::Failed {
                            error: "Process handle lost".into(),
                            last_attempt: Instant::now(),
                        };
                    }
                }
            }
            DaemonLifecycle::Failed { .. } => {
                // Auto-restart if under limit (handled by Tauri commands)
            }
            _ => {}
        }
    }

    fn push_log(&mut self, line: String) {
        self.log_buffer.push(line);
        if self.log_buffer.len() > MAX_LOG_LINES {
            let excess = self.log_buffer.len() - MAX_LOG_LINES;
            self.log_buffer.drain(..excess);
        }
    }
}

impl Drop for DaemonManager {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

// ---------------------------------------------------------------------------
// TCP wire-protocol client
// ---------------------------------------------------------------------------

/// Connect to the daemon at `addr`, send a JSON message with the given
/// `msg_type`, and deserialize the JSON response.
///
/// Protocol: send `{"type":"<msg_type>"}\n`, read one line of JSON.
fn fetch_daemon_json<T: DeserializeOwned>(addr: &str, msg_type: &str) -> Result<T, String> {
    let parsed: std::net::SocketAddr = addr
        .parse()
        .map_err(|e| format!("invalid address '{addr}': {e}"))?;

    let mut stream =
        TcpStream::connect_timeout(&parsed, TCP_TIMEOUT).map_err(|e| format!("connect: {e}"))?;

    stream
        .set_read_timeout(Some(TCP_TIMEOUT))
        .map_err(|e| format!("set read timeout: {e}"))?;
    stream
        .set_write_timeout(Some(TCP_TIMEOUT))
        .map_err(|e| format!("set write timeout: {e}"))?;

    let request = format!("{{\"type\":\"{msg_type}\"}}\n");
    stream
        .write_all(request.as_bytes())
        .map_err(|e| format!("write: {e}"))?;

    let mut reader = BufReader::new(&stream);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|e| format!("read: {e}"))?;

    if line.trim().is_empty() {
        return Err("daemon returned empty response".into());
    }

    serde_json::from_str(&line).map_err(|e| format!("parse: {e}"))
}

/// Fetch a specific data type from the daemon.
pub async fn fetch_health(addr: &str) -> Result<HealthResponse, String> {
    tokio::task::spawn_blocking({
        let addr = addr.to_string();
        move || fetch_daemon_json::<HealthResponse>(&addr, "health_check")
    })
    .await
    .map_err(|e| format!("task join: {e}"))?
}

pub async fn fetch_topology(addr: &str) -> Result<TopologyResponse, String> {
    tokio::task::spawn_blocking({
        let addr = addr.to_string();
        move || fetch_daemon_json::<TopologyResponse>(&addr, "topology_request")
    })
    .await
    .map_err(|e| format!("task join: {e}"))?
}

pub async fn fetch_routes(addr: &str) -> Result<RoutesResponse, String> {
    tokio::task::spawn_blocking({
        let addr = addr.to_string();
        move || fetch_daemon_json::<RoutesResponse>(&addr, "routes_request")
    })
    .await
    .map_err(|e| format!("task join: {e}"))?
}

pub async fn fetch_leases(addr: &str) -> Result<LeasesResponse, String> {
    tokio::task::spawn_blocking({
        let addr = addr.to_string();
        move || fetch_daemon_json::<LeasesResponse>(&addr, "leases_request")
    })
    .await
    .map_err(|e| format!("task join: {e}"))?
}

/// Fetch network status. Attempts a "network_status" request;
/// falls back to building from health data.
pub async fn fetch_network(addr: &str) -> Result<NetworkStatus, String> {
    tokio::task::spawn_blocking({
        let addr = addr.to_string();
        move || {
            // Try the network-specific wire-protocol message first
            match fetch_daemon_json::<NetworkStatus>(&addr, "network_status") {
                Ok(status) => Ok(status),
                Err(_) => {
                    // Fallback: derive from health check connectivity
                    let health = fetch_daemon_json::<HealthResponse>(&addr, "health_check");
                    Ok(NetworkStatus {
                        daemon_connected: health.is_ok(),
                        ..NetworkStatus::default()
                    })
                }
            }
        }
    })
    .await
    .map_err(|e| format!("task join: {e}"))?
}

/// Fetch streaming stats. Attempts a "streaming_stats" request;
/// falls back to defaults.
pub async fn fetch_streaming(addr: &str) -> Result<StreamingStats, String> {
    tokio::task::spawn_blocking({
        let addr = addr.to_string();
        move || match fetch_daemon_json::<StreamingStats>(&addr, "streaming_stats") {
            Ok(stats) => Ok(stats),
            Err(_) => Ok(StreamingStats::default()),
        }
    })
    .await
    .map_err(|e| format!("task join: {e}"))?
}

/// Fetch auth status. Attempts an "auth_status" request;
/// falls back to defaults.
pub async fn fetch_auth(addr: &str) -> Result<AuthStatus, String> {
    tokio::task::spawn_blocking({
        let addr = addr.to_string();
        move || match fetch_daemon_json::<AuthStatus>(&addr, "auth_status") {
            Ok(status) => Ok(status),
            Err(_) => Ok(AuthStatus::default()),
        }
    })
    .await
    .map_err(|e| format!("task join: {e}"))?
}

/// Fetch all data from daemon in parallel.
pub async fn fetch_all_data(addr: &str) -> Result<GuiData, String> {
    let (health, topology, routes, leases, network, streaming, auth) = tokio::join!(
        fetch_health(addr),
        fetch_topology(addr),
        fetch_routes(addr),
        fetch_leases(addr),
        fetch_network(addr),
        fetch_streaming(addr),
        fetch_auth(addr),
    );

    Ok(GuiData {
        health: health.unwrap_or_default(),
        topology: topology.unwrap_or_default(),
        routes: routes.unwrap_or_default(),
        leases: leases.unwrap_or_default(),
        network: network.unwrap_or_default(),
        streaming: streaming.unwrap_or_default(),
        auth: auth.unwrap_or_default(),
        logs: vec![],
        settings: SettingsState::default(),
    })
}
