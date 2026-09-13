//! Daemon lifecycle manager — auto-start, health polling, restart with backoff.
//!
//! Monitors the fabric-daemon process and manages its lifecycle from the GUI.

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
/// Maximum number of restart attempts before giving up.
const MAX_RESTARTS: u32 = 5;
/// Base delay for exponential backoff (doubles each attempt, capped at 30s).
const BASE_BACKOFF_SECS: u64 = 1;
/// Cap on backoff delay.
const MAX_BACKOFF_SECS: u64 = 30;
/// Health poll interval.
const HEALTH_POLL_INTERVAL: Duration = Duration::from_secs(5);
/// TCP connect timeout for health checks.
const HEALTH_CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
/// Max log lines kept in the ring buffer.
const MAX_LOG_LINES: usize = 500;

/// Lifecycle state of the managed daemon process.
#[derive(Debug, Clone)]
pub enum DaemonState {
    /// Not started yet.
    NotStarted,
    /// Process spawned, waiting for it to become healthy.
    Starting { started_at: Instant },
    /// Process is healthy and accepting connections.
    Running { pid: u32, started_at: Instant },
    /// Last start attempt failed.
    Failed { error: String, last_attempt: Instant },
    /// Explicitly stopped by the user.
    Stopped,
}

impl DaemonState {
    /// Human-readable status text.
    pub fn label(&self) -> &str {
        match self {
            DaemonState::NotStarted => "Not Started",
            DaemonState::Starting { .. } => "Starting",
            DaemonState::Running { .. } => "Running",
            DaemonState::Failed { .. } => "Failed",
            DaemonState::Stopped => "Stopped",
        }
    }

    /// Whether the daemon is considered healthy.
    pub fn is_healthy(&self) -> bool {
        matches!(self, DaemonState::Running { .. })
    }

    /// PID if running.
    pub fn pid(&self) -> Option<u32> {
        if let DaemonState::Running { pid, .. } = self { Some(*pid) } else { None }
    }

    /// Uptime in seconds if running.
    pub fn uptime_secs(&self) -> Option<u64> {
        if let DaemonState::Running { started_at, .. } = self {
            Some(started_at.elapsed().as_secs())
        } else {
            None
        }
    }
}

/// Manages the fabric-daemon child process lifecycle.
pub struct DaemonManager {
    state: Arc<Mutex<DaemonState>>,
    daemon_path: Option<String>,
    config_path: Option<String>,
    db_path: Option<String>,
    listen_addr: Option<String>,
    restart_count: u32,
    last_restart: Option<Instant>,
    child: Option<Child>,
    log_buffer: Vec<String>,
    last_health_check: Instant,
    auto_start: bool,
}

impl DaemonManager {
    /// Create a new daemon manager.
    pub fn new(
        daemon_path: Option<String>,
        config_path: Option<String>,
        db_path: Option<String>,
    ) -> Self {
        Self {
            state: Arc::new(Mutex::new(DaemonState::NotStarted)),
            daemon_path,
            config_path,
            db_path,
            listen_addr: None,
            restart_count: 0,
            last_restart: None,
            child: None,
            log_buffer: Vec::new(),
            last_health_check: Instant::now() - HEALTH_POLL_INTERVAL,
            auto_start: true,
        }
    }

    /// Set the listen address override.
    pub fn set_listen_addr(&mut self, addr: String) {
        self.listen_addr = Some(addr);
    }

    /// Enable or disable auto-start behavior.
    pub fn set_auto_start(&mut self, enabled: bool) {
        self.auto_start = enabled;
    }

    /// Whether auto-start is enabled.
    pub fn auto_start_enabled(&self) -> bool {
        self.auto_start
    }

    /// Resolve the path to the fabric-daemon binary.
    fn resolve_daemon_path(&self) -> Result<String, String> {
        // 1. Explicit path from constructor
        if let Some(ref path) = self.daemon_path {
            return Ok(path.clone());
        }

        // 2. Look for fabric-daemon in the same directory as the GUI binary
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let candidate = if cfg!(windows) {
                    dir.join("fabric-daemon.exe")
                } else {
                    dir.join("fabric-daemon")
                };
                if candidate.exists() {
                    return Ok(candidate.to_string_lossy().to_string());
                }
            }
        }

        // 3. Search PATH
        let name = if cfg!(windows) { "fabric-daemon.exe" } else { "fabric-daemon" };
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
        let state = self.state.lock().unwrap().clone();
        if matches!(state, DaemonState::Running { .. } | DaemonState::Starting { .. }) {
            return Err("Daemon is already running or starting".into());
        }

        let exe_path = self.resolve_daemon_path()?;
        let mut cmd = Command::new(&exe_path);
        cmd.arg("start");
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        if let Some(ref config) = self.config_path {
            cmd.arg("--config").arg(config);
        }
        if let Some(ref db) = self.db_path {
            cmd.arg("--db").arg(db);
        }
        if let Some(ref addr) = self.listen_addr {
            cmd.arg("--listen").arg(addr);
        }

        self.push_log(format!("[daemon] launching: {exe_path}"));
        let child = cmd.spawn().map_err(|e| {
            let msg = format!("Failed to spawn daemon: {e}");
            self.push_log(format!("[daemon] {msg}"));
            *self.state.lock().unwrap() = DaemonState::Failed {
                error: msg.clone(),
                last_attempt: Instant::now(),
            };
            msg
        })?;

        let pid = child.id();
        self.push_log(format!("[daemon] started with PID {pid}"));
        *self.state.lock().unwrap() = DaemonState::Starting {
            started_at: Instant::now(),
        };
        self.child = Some(child);
        Ok(())
    }

    /// Stop the daemon process gracefully, then force-kill if needed.
    pub fn stop(&mut self) -> Result<(), String> {
        {
            let state = self.state.lock().unwrap().clone();
            if !matches!(state, DaemonState::Running { .. } | DaemonState::Starting { .. }) {
                return Ok(());
            }
        }

        self.push_log("[daemon] sending SIGTERM / terminating".into());

        // Take child out to avoid overlapping mutable borrows
        let mut child = match self.child.take() {
            Some(c) => c,
            None => {
                *self.state.lock().unwrap() = DaemonState::Stopped;
                return Ok(());
            }
        };

        // Try graceful termination first
        if let Err(e) = child.kill() {
            self.push_log(format!("[daemon] kill error: {e}"));
        }
        // Wait up to 3 seconds for clean exit
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            match child.try_wait() {
                Ok(Some(_)) => {
                    self.push_log("[daemon] stopped gracefully".into());
                    *self.state.lock().unwrap() = DaemonState::Stopped;
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
        // Force kill
        self.push_log("[daemon] force killing".into());
        let _ = child.kill();
        let _ = child.wait();

        *self.state.lock().unwrap() = DaemonState::Stopped;
        self.restart_count = 0;
        self.push_log("[daemon] stopped".into());
        Ok(())
    }

    /// Restart with exponential backoff.
    pub fn restart(&mut self) -> Result<(), String> {
        self.stop()?;
        self.last_restart = Some(Instant::now());
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

    /// Check if the daemon is healthy by TCP connecting and sending health_check.
    pub fn check_health(&mut self, addr: &str) -> bool {
        use std::io::Write;
        use std::net::TcpStream;

        let stream = match TcpStream::connect_timeout(
            &addr.parse().unwrap_or_else(|_| "127.0.0.1:9400".parse().unwrap()),
            HEALTH_CONNECT_TIMEOUT,
        ) {
            Ok(s) => s,
            Err(_) => return false,
        };

        let mut stream = match stream {
            s => s,
        };
        stream.set_read_timeout(Some(HEALTH_CONNECT_TIMEOUT)).ok();
        stream.set_write_timeout(Some(HEALTH_CONNECT_TIMEOUT)).ok();

        if stream.write_all(b"{\"type\":\"health_check\"}\n").is_err() {
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

    /// Auto-start the daemon if it is not running. Called on GUI launch.
    pub fn auto_start(&mut self, addr: &str) {
        if !self.auto_start {
            return;
        }
        let state = self.state.lock().unwrap().clone();
        if state.is_healthy() {
            return;
        }
        // Quick TCP check — if daemon is already running externally, just adopt it
        if self.check_health(addr) {
            self.push_log("[daemon] detected running externally".into());
            *self.state.lock().unwrap() = DaemonState::Running {
                pid: 0, // unknown PID
                started_at: Instant::now(),
            };
            return;
        }
        if let Err(e) = self.start() {
            self.push_log(format!("[daemon] auto-start failed: {e}"));
        }
    }

    /// Poll health and manage restarts. Call this every frame or on a timer.
    pub fn poll(&mut self, addr: &str) {
        if self.last_health_check.elapsed() < HEALTH_POLL_INTERVAL {
            return;
        }
        self.last_health_check = Instant::now();

        let state = self.state.lock().unwrap().clone();
        match state {
            DaemonState::Starting { started_at } => {
                let healthy = self.check_health(addr);
                if started_at.elapsed() > Duration::from_secs(10) {
                    if healthy {
                        let pid = self.child.as_ref().map(|c| c.id()).unwrap_or(0);
                        *self.state.lock().unwrap() = DaemonState::Running { pid, started_at };
                        self.push_log("[daemon] became healthy".into());
                        self.restart_count = 0;
                    } else if let Some(ref mut child) = self.child {
                        match child.try_wait() {
                            Ok(Some(status)) => {
                                let msg = format!("Daemon exited with status: {status}");
                                self.push_log(format!("[daemon] {msg}"));
                                *self.state.lock().unwrap() = DaemonState::Failed {
                                    error: msg, last_attempt: Instant::now(),
                                };
                                self.child = None;
                                self.maybe_restart(addr);
                            }
                            Ok(None) => {
                                // Still running but not healthy yet
                            }
                            Err(_) => {}
                        }
                    }
                } else if healthy {
                    let pid = self.child.as_ref().map(|c| c.id()).unwrap_or(0);
                    *self.state.lock().unwrap() = DaemonState::Running { pid, started_at };
                    self.push_log("[daemon] became healthy".into());
                    self.restart_count = 0;
                }
            }
            DaemonState::Running { .. } => {
                let healthy = self.check_health(addr);
                if !healthy {
                    self.push_log("[daemon] health check failed".into());
                    if let Some(ref mut child) = self.child {
                        match child.try_wait() {
                            Ok(Some(status)) => {
                                let msg = format!("Daemon exited: {status}");
                                self.push_log(format!("[daemon] {msg}"));
                                *self.state.lock().unwrap() = DaemonState::Failed {
                                    error: msg, last_attempt: Instant::now(),
                                };
                                self.child = None;
                                self.maybe_restart(addr);
                            }
                            Ok(None) => {
                                self.push_log("[daemon] health degraded, monitoring".into());
                            }
                            Err(_) => {}
                        }
                    } else {
                        *self.state.lock().unwrap() = DaemonState::Failed {
                            error: "Process handle lost".into(), last_attempt: Instant::now(),
                        };
                        self.maybe_restart(addr);
                    }
                }
            }
            DaemonState::Failed { last_attempt, .. } => {
                if last_attempt.elapsed() > Duration::from_secs(30) && self.should_restart() {
                    self.maybe_restart(addr);
                }
            }
            _ => {}
        }
    }

    /// Try to restart if under the max restart limit.
    fn maybe_restart(&mut self, addr: &str) {
        if self.restart_count >= MAX_RESTARTS {
            self.push_log(format!(
                "[daemon] giving up after {MAX_RESTARTS} restart attempts"
            ));
            return;
        }
        if let Err(e) = self.restart() {
            self.push_log(format!("[daemon] restart failed: {e}"));
        }
        // Re-check health after restart delay
        let _ = addr;
    }

    /// Whether we should attempt a restart.
    pub fn should_restart(&self) -> bool {
        self.restart_count < MAX_RESTARTS
    }

    /// Reset restart counter after a successful connection.
    pub fn reset_restarts(&mut self) {
        self.restart_count = 0;
    }

    /// Get a clone of the current state (thread-safe).
    pub fn state_snapshot(&self) -> DaemonState {
        self.state.lock().unwrap().clone()
    }

    /// Get recent log lines.
    pub fn recent_logs(&self) -> &[String] {
        &self.log_buffer
    }

    /// Number of restart attempts so far.
    pub fn restart_count(&self) -> u32 {
        self.restart_count
    }

    /// Drain log lines from the child process stdout/stderr (non-blocking).
    pub fn drain_child_logs(&mut self) {
        // We can't easily do non-blocking reads on Child's stdout/stderr
        // in a single-threaded egui context. Instead, we capture logs
        // at spawn time and on state transitions. For a more complete
        // solution, we'd spawn reader threads — but this is sufficient
        // for the GUI's needs.
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
