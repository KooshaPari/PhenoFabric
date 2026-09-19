//! Async daemon process manager and TCP wire-protocol client.
//!
//! Communicates with fabric-daemon over TCP using the same JSON-line protocol
//! as the existing egui GUI: send `{"type":"<msg_type>"}\n`, read JSON response.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde::{de::DeserializeOwned, Deserialize};

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

/// WorkOS AuthKit client id for this desktop build.
///
/// A client id is a public identifier, not a credential. The daemon needs it to
/// exchange an authorization code, and the frontend uses the same value to
/// build the authorize URL. It is passed to the spawned daemon through the
/// environment so login works with no config file on disk.
pub const WORKOS_CLIENT_ID: &str = "client_01K4KYZR40RK7R9X3PPB5SEJ66";

/// Redirect URI the daemon advertises when it builds an authorize URL itself.
///
/// The GUI's real login flow listens on an ephemeral port and sends that URI
/// from the frontend, so this value only affects the daemon-side `auth_start`
/// path. It is the dashboard default and is registered, so the URL is accepted
/// instead of failing redirect-URI validation.
pub const WORKOS_REDIRECT_URI: &str = "http://localhost:5173/auth/callback";

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
    Starting {
        started_at: Instant,
    },
    Running {
        pid: u32,
        started_at: Instant,
    },
    Failed {
        error: String,
        last_attempt: Instant,
    },
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
            .map(|msg| LogEntry {
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
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
        // Pass the deployment-specific WorkOS settings to the daemon. Without
        // these the daemon has no client id and every auth message comes back
        // as workos_not_configured, so login can never complete.
        cmd.env("WORKOS_CLIENT_ID", WORKOS_CLIENT_ID);
        cmd.env("WORKOS_REDIRECT_URI", WORKOS_REDIRECT_URI);
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

        stream.set_read_timeout(Some(HEALTH_CONNECT_TIMEOUT)).ok();
        stream.set_write_timeout(Some(HEALTH_CONNECT_TIMEOUT)).ok();

        let mut stream = stream;
        if stream.write_all(b"{\"type\":\"health_check\"}\n").is_err() {
            return false;
        }

        let reader = BufReader::new(&stream);
        for l in reader.lines().take(1).flatten() {
            if l.contains("\"daemon_healthy\":true") || l.contains("\"daemon_healthy\": true") {
                return true;
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
                        self.lifecycle = DaemonLifecycle::Running { pid, started_at };
                        self.push_log("[daemon] became healthy".into());
                        self.restart_count = 0;
                    } else if let Some(ref mut child) = self.child {
                        if let Ok(Some(status)) = child.try_wait() {
                            let msg = format!("Daemon exited with status: {status}");
                            self.push_log(format!("[daemon] {msg}"));
                            self.lifecycle = DaemonLifecycle::Failed {
                                error: msg,
                                last_attempt: Instant::now(),
                            };
                            self.child = None;
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

/// Build the one-line JSON request for a wire message.
///
/// The token is attached as a top-level field when present, which is where the
/// daemon's middleware looks for it, and is JSON-escaped rather than
/// interpolated so a token containing quotes cannot corrupt the line.
fn build_request(msg_type: &str, token: Option<&str>) -> String {
    match token {
        Some(token) => format!(
            "{{\"type\":\"{msg_type}\",\"token\":{}}}\n",
            serde_json::to_string(token).unwrap_or_else(|_| "\"\"".into())
        ),
        None => format!("{{\"type\":\"{msg_type}\"}}\n"),
    }
}

/// Connect to the daemon at `addr`, send a JSON message with the given
/// `msg_type`, and deserialize the JSON response.
///
/// Protocol: send `{"type":"<msg_type>"}\n`, read one line of JSON.
fn fetch_daemon_json<T: DeserializeOwned>(
    addr: &str,
    msg_type: &str,
    token: Option<&str>,
) -> Result<T, String> {
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

    // Protected routes require a bearer token; the middleware reads it from
    // the top-level `token` field. Public routes ignore it, so it is always
    // attached when present.
    let request = build_request(msg_type, token);
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
pub async fn fetch_health(
    addr: &str,
    token: Option<&str>,
) -> Result<HealthResponse, String> {
    tokio::task::spawn_blocking({
        let addr = addr.to_string();
        let token = token.map(str::to_string);
        move || fetch_daemon_json::<HealthResponse>(&addr, "health_check", token.as_deref())
    })
    .await
    .map_err(|e| format!("task join: {e}"))?
}

pub async fn fetch_topology(
    addr: &str,
    token: Option<&str>,
) -> Result<TopologyResponse, String> {
    tokio::task::spawn_blocking({
        let addr = addr.to_string();
        let token = token.map(str::to_string);
        move || fetch_daemon_json::<TopologyResponse>(&addr, "topology_request", token.as_deref())
    })
    .await
    .map_err(|e| format!("task join: {e}"))?
}

pub async fn fetch_routes(
    addr: &str,
    token: Option<&str>,
) -> Result<RoutesResponse, String> {
    tokio::task::spawn_blocking({
        let addr = addr.to_string();
        let token = token.map(str::to_string);
        move || fetch_daemon_json::<RoutesResponse>(&addr, "routes_request", token.as_deref())
    })
    .await
    .map_err(|e| format!("task join: {e}"))?
}

pub async fn fetch_leases(
    addr: &str,
    token: Option<&str>,
) -> Result<LeasesResponse, String> {
    tokio::task::spawn_blocking({
        let addr = addr.to_string();
        let token = token.map(str::to_string);
        move || fetch_daemon_json::<LeasesResponse>(&addr, "leases_request", token.as_deref())
    })
    .await
    .map_err(|e| format!("task join: {e}"))?
}

/// Fetch network status. Attempts a "network_status" request;
/// falls back to building from health data.
pub async fn fetch_network(
    addr: &str,
    token: Option<&str>,
) -> Result<NetworkStatus, String> {
    tokio::task::spawn_blocking({
        let addr = addr.to_string();
        let token = token.map(str::to_string);
        move || {
            // Try the network-specific wire-protocol message first
            match fetch_daemon_json::<NetworkStatus>(&addr, "network_status", token.as_deref()) {
                Ok(status) => Ok(status),
                Err(_) => {
                    // Fallback: derive from health check connectivity
                    let health =
                        fetch_daemon_json::<HealthResponse>(&addr, "health_check", token.as_deref());
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
pub async fn fetch_streaming(
    addr: &str,
    token: Option<&str>,
) -> Result<StreamingStats, String> {
    tokio::task::spawn_blocking({
        let addr = addr.to_string();
        let token = token.map(str::to_string);
        move || match fetch_daemon_json::<StreamingStats>(&addr, "streaming_stats", token.as_deref()) {
            Ok(stats) => Ok(stats),
            Err(_) => Ok(StreamingStats::default()),
        }
    })
    .await
    .map_err(|e| format!("task join: {e}"))?
}

/// Fetch auth status. Attempts an "auth_status" request;
/// falls back to defaults.
pub async fn fetch_auth(addr: &str, token: Option<&str>) -> Result<AuthStatus, String> {
    tokio::task::spawn_blocking({
        let addr = addr.to_string();
        let token = token.map(str::to_string);
        move || match fetch_daemon_json::<AuthStatus>(&addr, "auth_status", token.as_deref()) {
            Ok(status) => Ok(status),
            Err(_) => Ok(AuthStatus::default()),
        }
    })
    .await
    .map_err(|e| format!("task join: {e}"))?
}

/// Fetch all data from daemon in parallel.
pub async fn fetch_all_data(addr: &str, token: Option<&str>) -> Result<GuiData, String> {
    let (health, topology, routes, leases, network, streaming, auth) = tokio::join!(
        fetch_health(addr, token),
        fetch_topology(addr, token),
        fetch_routes(addr, token),
        fetch_leases(addr, token),
        fetch_network(addr, token),
        fetch_streaming(addr, token),
        fetch_auth(addr, token),
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

// ---------------------------------------------------------------------------
// Auth wire protocol functions
// ---------------------------------------------------------------------------

/// Request WorkOS authorization URL from daemon.
pub async fn fetch_auth_start(addr: &str) -> Result<AuthStartResponse, String> {
    tokio::task::spawn_blocking({
        let addr = addr.to_string();
        move || {
            let mut stream = TcpStream::connect(&addr).map_err(|e| format!("connect: {e}"))?;
            stream
                .set_read_timeout(Some(TCP_TIMEOUT))
                .map_err(|e| format!("timeout: {e}"))?;

            let msg = serde_json::json!({"type": "auth_start"});
            writeln!(stream, "{}", msg).map_err(|e| format!("write: {e}"))?;

            let mut reader = BufReader::new(&stream);
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .map_err(|e| format!("read: {e}"))?;

            serde_json::from_str::<AuthStartResponse>(&line).map_err(|e| format!("parse: {e}"))
        }
    })
    .await
    .map_err(|e| format!("task join: {e}"))?
}

/// A completed login: the GUI's `AuthStatus` plus the access token.
///
/// The token is flattened out of the same response the daemon already sends,
/// so the `AuthStatus` shape the frontend consumes is unchanged.
#[derive(Debug, Clone, Deserialize)]
pub struct AuthCompletion {
    #[serde(flatten)]
    pub status: AuthStatus,
    /// WorkOS access token to present on later protected requests.
    #[serde(default)]
    pub access_token: Option<String>,
}

/// The GUI's session credential, held in memory for the process lifetime.
///
/// Deliberately not persisted: a restarted GUI logs in again. Writing a bearer
/// token to disk needs the OS keychain, which is a separate change.
#[derive(Debug, Clone)]
pub struct SessionToken {
    pub access_token: String,
    /// Expiry taken from the session lifetime the daemon reported.
    pub expires_at: std::time::SystemTime,
}

impl SessionToken {
    /// Build from a login response, or `None` when no token was returned.
    pub fn from_completion(completion: &AuthCompletion) -> Option<Self> {
        let access_token = completion.access_token.clone()?;
        let expires_at = std::time::SystemTime::now()
            + std::time::Duration::from_secs(completion.status.session_expiry_secs);
        Some(Self {
            access_token,
            expires_at,
        })
    }

    /// Whether the token is past the lifetime the daemon reported.
    pub fn is_expired(&self) -> bool {
        std::time::SystemTime::now() >= self.expires_at
    }
}

/// Exchange an AuthKit authorization code for tokens.
///
/// `code_verifier` is the PKCE verifier for the challenge used in the
/// authorize request. Forwarding it lets the daemon exchange the code as a
/// public client instead of relying on a configured client secret.
pub async fn fetch_complete_auth(
    addr: &str,
    code: &str,
    code_verifier: Option<&str>,
) -> Result<AuthCompletion, String> {
    tokio::task::spawn_blocking({
        let addr = addr.to_string();
        let code = code.to_string();
        let code_verifier = code_verifier.map(str::to_string);
        move || {
            let mut stream = TcpStream::connect(&addr).map_err(|e| format!("connect: {e}"))?;
            stream
                .set_read_timeout(Some(TCP_TIMEOUT))
                .map_err(|e| format!("timeout: {e}"))?;

            let mut msg = serde_json::json!({"type": "auth_complete", "code": code});
            if let Some(verifier) = code_verifier {
                msg["code_verifier"] = serde_json::Value::String(verifier);
            }
            writeln!(stream, "{}", msg).map_err(|e| format!("write: {e}"))?;

            let mut reader = BufReader::new(&stream);
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .map_err(|e| format!("read: {e}"))?;

            serde_json::from_str::<AuthCompletion>(&line).map_err(|e| format!("parse: {e}"))
        }
    })
    .await
    .map_err(|e| format!("task join: {e}"))?
}

/// Send passwordless email auth request.
pub async fn fetch_email_auth(addr: &str, email: &str) -> Result<EmailAuthResponse, String> {
    tokio::task::spawn_blocking({
        let addr = addr.to_string();
        let email = email.to_string();
        move || {
            let mut stream = TcpStream::connect(&addr).map_err(|e| format!("connect: {e}"))?;
            stream
                .set_read_timeout(Some(TCP_TIMEOUT))
                .map_err(|e| format!("timeout: {e}"))?;

            let msg = serde_json::json!({"type": "auth_email", "email": email});
            writeln!(stream, "{}", msg).map_err(|e| format!("write: {e}"))?;

            let mut reader = BufReader::new(&stream);
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .map_err(|e| format!("read: {e}"))?;

            serde_json::from_str::<EmailAuthResponse>(&line).map_err(|e| format!("parse: {e}"))
        }
    })
    .await
    .map_err(|e| format!("task join: {e}"))?
}

/// Verify a Magic Auth code and complete passwordless login.
///
/// Completes the flow started by `fetch_email_auth`: the daemon exchanges the
/// emailed code for tokens and returns the resulting `AuthStatus`.
pub async fn fetch_verify_email_auth(
    addr: &str,
    email: &str,
    code: &str,
) -> Result<AuthCompletion, String> {
    tokio::task::spawn_blocking({
        let addr = addr.to_string();
        let email = email.to_string();
        let code = code.to_string();
        move || {
            let mut stream = TcpStream::connect(&addr).map_err(|e| format!("connect: {e}"))?;
            stream
                .set_read_timeout(Some(TCP_TIMEOUT))
                .map_err(|e| format!("timeout: {e}"))?;

            let msg = serde_json::json!({"type": "auth_verify", "email": email, "code": code});
            writeln!(stream, "{}", msg).map_err(|e| format!("write: {e}"))?;

            let mut reader = BufReader::new(&stream);
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .map_err(|e| format!("read: {e}"))?;

            serde_json::from_str::<AuthCompletion>(&line).map_err(|e| format!("parse: {e}"))
        }
    })
    .await
    .map_err(|e| format!("task join: {e}"))?
}

/// Sign out: clear the daemon-side session.
pub async fn fetch_logout(addr: &str) -> Result<AuthStatus, String> {
    tokio::task::spawn_blocking({
        let addr = addr.to_string();
        move || {
            let mut stream = TcpStream::connect(&addr).map_err(|e| format!("connect: {e}"))?;
            stream
                .set_read_timeout(Some(TCP_TIMEOUT))
                .map_err(|e| format!("timeout: {e}"))?;

            let msg = serde_json::json!({"type": "auth_logout"});
            writeln!(stream, "{}", msg).map_err(|e| format!("write: {e}"))?;

            let mut reader = BufReader::new(&stream);
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .map_err(|e| format!("read: {e}"))?;

            serde_json::from_str::<AuthStatus>(&line).map_err(|e| format!("parse: {e}"))
        }
    })
    .await
    .map_err(|e| format!("task join: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The exact response the daemon sends on a successful login.
    const LOGIN_RESPONSE: &str = r#"{"type":"auth_complete_response","status":"ok","logged_in":true,"user_name":"Dev User","user_email":"dev@example.com","org_name":"org_456","roles":[],"session_expiry_secs":3600,"active_sessions":[],"access_token":"at_secret","user":{"id":"user_1","email":"dev@example.com","name":"Dev User","org_id":"org_456"}}"#;

    #[test]
    fn login_response_parses_with_token() {
        let c: AuthCompletion = serde_json::from_str(LOGIN_RESPONSE).unwrap();
        assert!(c.status.logged_in);
        assert_eq!(c.status.user_email, "dev@example.com");
        assert_eq!(c.status.session_expiry_secs, 3600);
        assert_eq!(c.access_token.as_deref(), Some("at_secret"));
    }

    #[test]
    fn session_token_takes_the_reported_lifetime() {
        let c: AuthCompletion = serde_json::from_str(LOGIN_RESPONSE).unwrap();
        let t = SessionToken::from_completion(&c).expect("token present");
        assert_eq!(t.access_token, "at_secret");
        assert!(!t.is_expired());
    }

    #[test]
    fn no_token_in_the_response_means_no_session() {
        let body = LOGIN_RESPONSE.replace(r#","access_token":"at_secret""#, "");
        let c: AuthCompletion = serde_json::from_str(&body).unwrap();
        assert!(c.access_token.is_none());
        assert!(SessionToken::from_completion(&c).is_none());
    }

    #[test]
    fn zero_lifetime_session_is_already_expired() {
        let body = LOGIN_RESPONSE.replace(r#""session_expiry_secs":3600"#, r#""session_expiry_secs":0"#);
        let c: AuthCompletion = serde_json::from_str(&body).unwrap();
        assert!(SessionToken::from_completion(&c).unwrap().is_expired());
    }

    #[test]
    fn request_carries_the_token_when_present() {
        let v: serde_json::Value =
            serde_json::from_str(build_request("topology_request", Some("at_secret")).trim()).unwrap();
        assert_eq!(v["type"], "topology_request");
        assert_eq!(v["token"], "at_secret");
    }

    #[test]
    fn request_omits_the_token_when_absent() {
        let v: serde_json::Value =
            serde_json::from_str(build_request("health_check", None).trim()).unwrap();
        assert_eq!(v["type"], "health_check");
        assert!(v.get("token").is_none());
    }

    #[test]
    fn token_is_escaped_not_interpolated() {
        // A token containing a quote must not produce a malformed line.
        let raw = build_request("topology_request", Some("a\"b"));
        let v: serde_json::Value = serde_json::from_str(raw.trim()).unwrap();
        assert_eq!(v["token"], "a\"b");
    }
}
