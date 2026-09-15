//! Tauri command handlers — bridge between the frontend and the daemon.
//!
//! Each `#[tauri::command]` is callable from JavaScript via
//! `invoke('command_name', { ... })`. They lock the shared `DaemonManager`
//! and delegate to the async wire-protocol client in `daemon`.

use tauri::State;

use crate::daemon;
use crate::types::*;
use crate::AppState;

// ---------------------------------------------------------------------------
// Data commands — fetch from daemon wire protocol
// ---------------------------------------------------------------------------

/// Fetch daemon health status via TCP wire protocol.
#[tauri::command]
pub async fn get_health(state: State<'_, AppState>) -> Result<HealthResponse, String> {
    let addr = state.daemon.lock().await.listen_addr().to_string();
    daemon::fetch_health(&addr).await
}

/// Fetch network topology graph.
#[tauri::command]
pub async fn get_topology(state: State<'_, AppState>) -> Result<TopologyResponse, String> {
    let addr = state.daemon.lock().await.listen_addr().to_string();
    daemon::fetch_topology(&addr).await
}

/// Fetch active routing plans.
#[tauri::command]
pub async fn get_routes(state: State<'_, AppState>) -> Result<RoutesResponse, String> {
    let addr = state.daemon.lock().await.listen_addr().to_string();
    daemon::fetch_routes(&addr).await
}

/// Fetch active surface leases.
#[tauri::command]
pub async fn get_leases(state: State<'_, AppState>) -> Result<LeasesResponse, String> {
    let addr = state.daemon.lock().await.listen_addr().to_string();
    daemon::fetch_leases(&addr).await
}

/// Fetch network connectivity status (Tailscale, UPnP, NAT).
#[tauri::command]
pub async fn get_network(state: State<'_, AppState>) -> Result<NetworkStatus, String> {
    let addr = state.daemon.lock().await.listen_addr().to_string();
    daemon::fetch_network(&addr).await
}

/// Fetch streaming statistics (frames, latency, codec).
#[tauri::command]
pub async fn get_streaming(state: State<'_, AppState>) -> Result<StreamingStats, String> {
    let addr = state.daemon.lock().await.listen_addr().to_string();
    daemon::fetch_streaming(&addr).await
}

/// Fetch authentication status.
#[tauri::command]
pub async fn get_auth(state: State<'_, AppState>) -> Result<AuthStatus, String> {
    let addr = state.daemon.lock().await.listen_addr().to_string();
    daemon::fetch_auth(&addr).await
}

/// Fetch recent daemon log entries.
#[tauri::command]
pub async fn get_logs(state: State<'_, AppState>) -> Result<Vec<LogEntry>, String> {
    let daemon = state.daemon.lock().await;
    Ok(daemon.recent_logs())
}

/// Fetch current settings.
#[tauri::command]
pub async fn get_settings(_state: State<'_, AppState>) -> Result<SettingsState, String> {
    Ok(SettingsState::default())
}

/// Fetch all data in a single call (used for initial load / full refresh).
#[tauri::command]
pub async fn refresh_data(state: State<'_, AppState>) -> Result<GuiData, String> {
    let addr = state.daemon.lock().await.listen_addr().to_string();
    daemon::fetch_all_data(&addr).await
}

// ---------------------------------------------------------------------------
// Daemon lifecycle commands
// ---------------------------------------------------------------------------

/// Start the fabric-daemon process.
#[tauri::command]
pub async fn start_daemon(state: State<'_, AppState>) -> Result<DaemonStatusResponse, String> {
    let mut daemon = state.daemon.lock().await;
    daemon.start()?;
    Ok(daemon.status_snapshot())
}

/// Stop the fabric-daemon process.
#[tauri::command]
pub async fn stop_daemon(state: State<'_, AppState>) -> Result<DaemonStatusResponse, String> {
    let mut daemon = state.daemon.lock().await;
    daemon.stop()?;
    Ok(daemon.status_snapshot())
}

/// Restart the fabric-daemon with exponential backoff.
#[tauri::command]
pub async fn restart_daemon(state: State<'_, AppState>) -> Result<DaemonStatusResponse, String> {
    let mut daemon = state.daemon.lock().await;
    daemon.restart()?;
    Ok(daemon.status_snapshot())
}

/// Get current daemon lifecycle status.
#[tauri::command]
pub async fn get_daemon_status(
    state: State<'_, AppState>,
) -> Result<DaemonStatusResponse, String> {
    let daemon = state.daemon.lock().await;
    Ok(daemon.status_snapshot())
}
