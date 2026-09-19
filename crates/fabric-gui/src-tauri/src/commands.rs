//! Tauri command handlers — bridge between the frontend and the daemon.
//!
//! Each `#[tauri::command]` is callable from JavaScript via
//! `invoke('command_name', { ... })`. They lock the shared `DaemonManager`
//! and delegate to the async wire-protocol client in `daemon`.

use tauri::State;

use crate::daemon;
use crate::types::*;
use crate::AppState;

/// The session token to present on protected requests.
///
/// Returns `None` when no login has happened or the token's reported lifetime
/// has elapsed, in which case the request goes out unauthenticated and the
/// daemon answers with an auth error the UI can act on.
async fn session_token(state: &AppState) -> Option<String> {
    let guard = state.session.lock().await;
    match guard.as_ref() {
        Some(token) if !token.is_expired() => Some(token.access_token.clone()),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Data commands — fetch from daemon wire protocol
// ---------------------------------------------------------------------------

/// Fetch daemon health status via TCP wire protocol.
#[tauri::command]
pub async fn get_health(state: State<'_, AppState>) -> Result<HealthResponse, String> {
    let addr = state.daemon.lock().await.listen_addr().to_string();
    let token = session_token(&state).await;
    daemon::fetch_health(&addr, token.as_deref()).await
}

/// Fetch network topology graph.
#[tauri::command]
pub async fn get_topology(state: State<'_, AppState>) -> Result<TopologyResponse, String> {
    let addr = state.daemon.lock().await.listen_addr().to_string();
    let token = session_token(&state).await;
    daemon::fetch_topology(&addr, token.as_deref()).await
}

/// Fetch active routing plans.
#[tauri::command]
pub async fn get_routes(state: State<'_, AppState>) -> Result<RoutesResponse, String> {
    let addr = state.daemon.lock().await.listen_addr().to_string();
    let token = session_token(&state).await;
    daemon::fetch_routes(&addr, token.as_deref()).await
}

/// Fetch active surface leases.
#[tauri::command]
pub async fn get_leases(state: State<'_, AppState>) -> Result<LeasesResponse, String> {
    let addr = state.daemon.lock().await.listen_addr().to_string();
    let token = session_token(&state).await;
    daemon::fetch_leases(&addr, token.as_deref()).await
}

/// Fetch network connectivity status (Tailscale, UPnP, NAT).
#[tauri::command]
pub async fn get_network(state: State<'_, AppState>) -> Result<NetworkStatus, String> {
    let addr = state.daemon.lock().await.listen_addr().to_string();
    let token = session_token(&state).await;
    daemon::fetch_network(&addr, token.as_deref()).await
}

/// Fetch streaming statistics (frames, latency, codec).
#[tauri::command]
pub async fn get_streaming(state: State<'_, AppState>) -> Result<StreamingStats, String> {
    let addr = state.daemon.lock().await.listen_addr().to_string();
    let token = session_token(&state).await;
    daemon::fetch_streaming(&addr, token.as_deref()).await
}

/// Fetch authentication status.
#[tauri::command]
pub async fn get_auth(state: State<'_, AppState>) -> Result<AuthStatus, String> {
    let addr = state.daemon.lock().await.listen_addr().to_string();
    let token = session_token(&state).await;
    daemon::fetch_auth(&addr, token.as_deref()).await
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
    let token = session_token(&state).await;
    daemon::fetch_all_data(&addr, token.as_deref()).await
}

// ---------------------------------------------------------------------------
// Auth lifecycle commands
// ---------------------------------------------------------------------------

/// Start WorkOS OAuth flow — returns authorization URL.
#[tauri::command]
pub async fn start_auth(state: State<'_, AppState>) -> Result<AuthStartResponse, String> {
    let daemon = state.daemon.lock().await;
    let addr = daemon.listen_addr().to_string();

    // Try to get auth URL from daemon's OAuth provider
    match daemon::fetch_auth_start(&addr).await {
        Ok(resp) => Ok(resp),
        Err(_) => {
            // Fallback: build the authorize URL from this build's WorkOS
            // settings. `response_type=code` is required - WorkOS rejects the
            // request outright without it and lands on a generic error page.
            Ok(AuthStartResponse {
                url: format!(
                    "https://api.workos.com/user_management/authorize?client_id={}&redirect_uri={}&response_type=code&provider=authkit",
                    query_encode(daemon::WORKOS_CLIENT_ID),
                    query_encode(daemon::WORKOS_REDIRECT_URI),
                ),
                state: uuid::Uuid::new_v4().to_string(),
            })
        }
    }
}

/// Percent-encode a value for use in a query string (RFC 3986 unreserved set).
///
/// The client id and redirect URI are the only interpolated values, and the
/// redirect URI contains `:` and `/`, which must be encoded for the query to
/// survive URL parsing.
fn query_encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// Complete WorkOS AuthKit login — exchange code for tokens.
#[tauri::command]
pub async fn complete_auth(
    state: State<'_, AppState>,
    code: String,
    code_verifier: Option<String>,
) -> Result<AuthStatus, String> {
    let addr = state.daemon.lock().await.listen_addr().to_string();
    let completion = daemon::fetch_complete_auth(&addr, &code, code_verifier.as_deref()).await?;

    // Hold the token so later requests can authenticate, and return the status
    // the frontend already consumes.
    let status = completion.status.clone();
    *state.session.lock().await = daemon::SessionToken::from_completion(&completion);
    Ok(status)
}

/// Sign out: clear the daemon session and the locally held token.
#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> Result<AuthStatus, String> {
    let addr = state.daemon.lock().await.listen_addr().to_string();
    let status = daemon::fetch_logout(&addr).await?;
    *state.session.lock().await = None;
    Ok(status)
}

/// Start passwordless email auth — send magic link.
#[tauri::command]
pub async fn start_email_auth(
    state: State<'_, AppState>,
    email: String,
) -> Result<EmailAuthResponse, String> {
    let daemon = state.daemon.lock().await;
    let addr = daemon.listen_addr().to_string();
    daemon::fetch_email_auth(&addr, &email).await
}

/// Verify a Magic Auth code — completes passwordless login.
#[tauri::command]
pub async fn verify_email_auth(
    state: State<'_, AppState>,
    email: String,
    code: String,
) -> Result<AuthStatus, String> {
    let addr = state.daemon.lock().await.listen_addr().to_string();
    let completion = daemon::fetch_verify_email_auth(&addr, &email, &code).await?;
    let status = completion.status.clone();
    *state.session.lock().await = daemon::SessionToken::from_completion(&completion);
    Ok(status)
}

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
pub async fn get_daemon_status(state: State<'_, AppState>) -> Result<DaemonStatusResponse, String> {
    let daemon = state.daemon.lock().await;
    Ok(daemon.status_snapshot())
}

/// Start a one-shot HTTP listener for the OAuth callback.
/// Returns the port the listener is bound to. The frontend should construct
/// the auth URL with `redirect_uri=http://localhost:{port}/auth/callback`.
#[tauri::command]
pub fn start_auth_listener(app: tauri::AppHandle) -> Result<u16, String> {
    crate::auth_callback::start_listener(app)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_encode_encodes_reserved_characters() {
        // The redirect URI's `:` and `/` must be encoded or the query string
        // will not survive URL parsing.
        assert_eq!(
            query_encode("http://localhost:5173/auth/callback"),
            "http%3A%2F%2Flocalhost%3A5173%2Fauth%2Fcallback"
        );
    }

    #[test]
    fn query_encode_leaves_unreserved_characters_alone() {
        assert_eq!(query_encode("client_01K4-A.b~c"), "client_01K4-A.b~c");
    }
}
