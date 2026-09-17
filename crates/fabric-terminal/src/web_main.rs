//! tf-web: Multi-tenant web server for terminal viewing
//!
//! Serves a REST API and WebSocket terminal streaming endpoint.
//! Agents connect via API key, view pane content in real-time.

use anyhow::{Context, Result};
use axum::{
    Json, Router,
    extract::{
        Path, Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;
use tokio::sync::RwLock as TokioRwLock;
use tower_http::cors::{Any, CorsLayer};

#[cfg(feature = "self-update")]
pub mod self_update;
mod tmux_capture;

/// Shared application state
struct AppState {
    /// Registered agent API keys: api_key -> agent_name
    agents: TokioRwLock<HashMap<String, AgentInfo>>,
    /// Active WebSocket connections: pane_id -> list of sender handles
    connections: TokioRwLock<HashMap<String, Vec<tokio::sync::mpsc::UnboundedSender<String>>>>,
    /// Cached pane content for REST API
    pane_cache: RwLock<HashMap<String, CachedPane>>,
    /// Broadcast channel for pushing pane updates to WebSocket subscribers
    /// Receivers get the pane_id string; they fetch content from pane_cache.
    broadcast_tx: tokio::sync::broadcast::Sender<String>,
    /// tmux socket path
    socket_path: String,
    /// Path to tf-mux binary (for reference; capture loop re-resolves via TMUX_BIN)
    #[allow(dead_code)]
    tmux_bin: std::path::PathBuf,
    /// Path to persistence file for pane cache
    pane_cache_path: std::path::PathBuf,
}

#[derive(Clone, Serialize)]
#[allow(dead_code)]
struct AgentInfo {
    name: String,
    created_at: String,
    #[serde(skip_serializing)]
    api_key: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct CachedPane {
    pane_id: String,
    workspace_id: String,
    tab_id: String,
    lines: Vec<String>,
    width: u32,
    height: u32,
    cursor_row: u32,
    cursor_col: u32,
    updated_at: u64,
    source: String,
}
// === REST API Models ===
#[derive(Serialize)]
struct ApiResponse<T: Serialize> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}
#[derive(Serialize)]
struct PaneResponse {
    pane_id: String,
    workspace_id: String,
    tab_id: String,
    source: String,
}

#[derive(Serialize)]
struct LayoutResponse {
    panes: Vec<PaneResponse>,
    width: u32,
    height: u32,
}
#[derive(Deserialize)]
struct CaptureQuery {
    format: Option<String>,
}
#[derive(Deserialize)]
struct RegisterAgentRequest {
    name: String,
}
#[derive(Serialize)]
struct RegisterAgentResponse {
    api_key: String,
    agent_name: String,
}
#[derive(Deserialize)]
struct SendMessageRequest {
    text: String,
}
// === Ingest API Models ===
#[derive(Deserialize)]
struct IngestRequest {
    panes: Vec<IngestPane>,
}
#[derive(Deserialize)]
struct IngestPane {
    pane_id: String,
    title: String,
    lines: Vec<String>,
    width: u32,
    height: u32,
    source: String,
}
// === API Key Auth ===
/// Validate API key from Authorization header
async fn validate_api_key(
    state: &AppState,
    headers: &axum::http::HeaderMap,
) -> Result<String, StatusCode> {
    let key = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let agents = state.agents.read().await;
    agents
        .get(key)
        .map(|a| a.name.clone())
        .ok_or(StatusCode::UNAUTHORIZED)
}
/// Save pane cache to disk.
fn save_pane_cache(state: &AppState) {
    let cache = state.pane_cache.read().unwrap();
    let panes: Vec<&CachedPane> = cache.values().collect();
    let path = &state.pane_cache_path;
    match serde_json::to_string_pretty(&panes) {
        Ok(json) => {
            if let Err(e) = std::fs::write(path, &json) {
                tracing::warn!(error = %e, path = %path.display(), "Failed to save pane cache");
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "Failed to serialize pane cache");
        }
    }
}

// === Handler: Health ===
async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "tf-web",
        "version": "0.2.0"
    }))
}

// === Handler: Register Agent ===

async fn register_agent(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterAgentRequest>,
) -> Result<Json<ApiResponse<RegisterAgentResponse>>, StatusCode> {
    let api_key = format!("tf_{}", uuid_simple());
    let agent = AgentInfo {
        name: req.name.clone(),
        created_at: chrono_now(),
        api_key: api_key.clone(),
    };

    state.agents.write().await.insert(api_key.clone(), agent);
    tracing::info!("Registered agent: {}", req.name);
    Ok(Json(ApiResponse {
        success: true,
        data: Some(RegisterAgentResponse {
            api_key,
            agent_name: req.name,
        }),
        error: None,
    }))
}
// === Handler: List Panes ===
async fn list_panes(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ApiResponse<Vec<PaneResponse>>>, StatusCode> {
    let _agent = validate_api_key(&state, &headers).await?;

    // Return cached panes
    let cache = state.pane_cache.read().unwrap();
    let panes: Vec<PaneResponse> = cache
        .values()
        .map(|p| PaneResponse {
            pane_id: p.pane_id.clone(),
            workspace_id: p.workspace_id.clone(),
            tab_id: p.tab_id.clone(),
            source: p.source.clone(),
        })
        .collect();
    Ok(Json(ApiResponse {
        success: true,
        data: Some(panes),
        error: None,
    }))
}
// === Handler: Get Layout ===
async fn get_layout(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ApiResponse<LayoutResponse>>, StatusCode> {
    let _agent = validate_api_key(&state, &headers).await?;

    let cache = state.pane_cache.read().unwrap();
    let panes: Vec<PaneResponse> = cache
        .values()
        .map(|p| PaneResponse {
            pane_id: p.pane_id.clone(),
            workspace_id: p.workspace_id.clone(),
            tab_id: p.tab_id.clone(),
            source: p.source.clone(),
        })
        .collect();
    Ok(Json(ApiResponse {
        success: true,
        data: Some(LayoutResponse {
            panes,
            width: 80,
            height: 24,
        }),
        error: None,
    }))
}
// === Handler: Capture Pane ===
async fn capture_pane(
    State(state): State<Arc<AppState>>,
    Path(pane_id): Path<String>,
    headers: axum::http::HeaderMap,
    Query(query): Query<CaptureQuery>,
) -> Result<Response, StatusCode> {
    let _agent = validate_api_key(&state, &headers).await?;

    let cache = state.pane_cache.read().unwrap();
    let pane = cache.get(&pane_id).ok_or(StatusCode::NOT_FOUND)?;

    let format = query.format.as_deref().unwrap_or("text");

    match format {
        "json" => Ok(Json(ApiResponse {
            success: true,
            data: Some(pane.clone()),
            error: None,
        })
        .into_response()),
        _ => {
            let text = pane.lines.join("\n");
            Ok(text.into_response())
        }
    }
}
// === Handler: Send Keys to Pane ===
async fn send_keys(
    State(state): State<Arc<AppState>>,
    Path(pane_id): Path<String>,
    headers: axum::http::HeaderMap,
    Json(req): Json<SendMessageRequest>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let agent = validate_api_key(&state, &headers).await?;

    tracing::info!(
        "Agent '{}' sending keys to pane {}: {:?}",
        agent,
        pane_id,
        req.text
    );

    // In production: execute tmux send-keys via subprocess
    // For now: just log and broadcast to connected WebSockets
    let connections = state.connections.read().await;
    if let Some(senders) = connections.get(&pane_id) {
        let msg = serde_json::json!({
            "type": "input",
            "pane_id": pane_id,
            "text": req.text,
            "agent": agent,
        });
        let msg_str = serde_json::to_string(&msg).unwrap_or_default();
        for sender in senders {
            let _ = sender.send(msg_str.clone());
        }
    }
    Ok(Json(ApiResponse {
        success: true,
        data: Some(format!("Sent to pane {}", pane_id)),
        error: None,
    }))
}
// === Handler: Ingest Panes ===
async fn ingest_panes(
    State(state): State<Arc<AppState>>,
    Json(req): Json<IngestRequest>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let now = now_epoch_secs();
    let mut cache = state.pane_cache.write().unwrap();
    let count = req.panes.len();
    let mut pane_ids: Vec<String> = Vec::with_capacity(count);

    for pane in &req.panes {
        let cached = CachedPane {
            pane_id: pane.pane_id.clone(),
            workspace_id: "remote".to_string(),
            tab_id: pane.title.clone(),
            lines: pane.lines.clone(),
            width: pane.width,
            height: pane.height,
            cursor_row: 0,
            cursor_col: 0,
            updated_at: now,
            source: pane.source.clone(),
        };
        pane_ids.push(pane.pane_id.clone());
        cache.insert(pane.pane_id.clone(), cached);
    }

    drop(cache); // release lock before broadcast

    // Persist to disk
    save_pane_cache(&state);

    // Broadcast pane IDs so WebSocket subscribers can pull updated content
    for pane_id in &pane_ids {
        let _ = state.broadcast_tx.send(pane_id.clone());
    }

    tracing::info!("Ingested {} panes from external source", count);
    Ok(Json(ApiResponse {
        success: true,
        data: Some(format!("Ingested {} panes", count)),
        error: None,
    }))
}
// === Handler: WebSocket Terminal ===
async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(pane_id): Path<String>,
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Response, StatusCode> {
    // Validate API key from query param (WebSocket can't set headers)
    let _api_key = headers
        .get("sec-websocket-protocol")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // Also check query param
    // For simplicity, accept WebSocket connections for now
    // TODO: proper auth via query param
    let state = state.clone();
    let pane_id = pane_id.clone();

    Ok(ws.on_upgrade(move |socket| handle_ws(socket, pane_id, state)))
}

async fn handle_ws(socket: WebSocket, pane_id: String, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to broadcast channel BEFORE sending initial content
    let mut broadcast_rx = state.broadcast_tx.subscribe();

    // Send initial pane content
    {
        let msg = {
            let cache = state.pane_cache.read().unwrap();
            cache.get(&pane_id).map(|pane| {
                serde_json::json!({
                    "type": "screen",
                    "pane_id": pane_id,
                    "lines": pane.lines,
                    "width": pane.width,
                    "height": pane.height,
                    "cursor_row": pane.cursor_row,
                    "cursor_col": pane.cursor_col,
                })
            })
        };
        if let Some(msg) = msg
            && let Ok(text) = serde_json::to_string(&msg)
        {
            let _ = sender.send(Message::Text(text.into())).await;
        }
    }

    // Spawn task to forward broadcast updates to this WebSocket
    let bc_state = state.clone();
    let bc_pane_id = pane_id.clone();
    let send_task = tokio::spawn(async move {
        loop {
            match broadcast_rx.recv().await {
                Ok(updated_pane_id) => {
                    // Only push if this broadcast is for our pane (or "all")
                    if updated_pane_id != bc_pane_id && updated_pane_id != "all" {
                        continue;
                    }
                    let msg = {
                        let cache = bc_state.pane_cache.read().unwrap();
                        cache.get(&bc_pane_id).map(|pane| {
                            serde_json::json!({
                                "type": "screen",
                                "pane_id": bc_pane_id,
                                "lines": pane.lines,
                                "width": pane.width,
                                "height": pane.height,
                                "cursor_row": pane.cursor_row,
                                "cursor_col": pane.cursor_col,
                            })
                        })
                    };
                    if let Some(msg) = msg
                        && let Ok(text) = serde_json::to_string(&msg)
                        && sender.send(Message::Text(text.into())).await.is_err()
                    {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("WebSocket missed {} broadcasts", n);
                }
                Err(_) => break, // channel closed
            }
        }
    });

    // Handle incoming messages (user input from browser)
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    tracing::debug!("WS input on {}: {}", pane_id, text);
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }
}
// === Handler: Frontend ===
async fn frontend() -> Html<&'static str> {
    Html(include_str!("web_index.html"))
}
// === Stale Pane Cleanup ===

/// Panes not updated in this many seconds are pruned from cache.
const STALE_PANE_TTL_SECS: u64 = 300; // 5 minutes

/// Remove panes whose `updated_at` is older than STALE_PANE_TTL_SECS.
/// Returns the number of panes removed.
fn cleanup_stale_panes(state: &AppState) -> usize {
    let mut cache = state.pane_cache.write().unwrap();
    let now = now_epoch_secs();
    let stale: Vec<String> = cache
        .iter()
        .filter(|(_, p)| now.saturating_sub(p.updated_at) > STALE_PANE_TTL_SECS)
        .map(|(id, _)| id.clone())
        .collect();
    let count = stale.len();
    for id in &stale {
        cache.remove(id);
    }
    if count > 0 {
        drop(cache); // release lock before disk write
        save_pane_cache(state);
        tracing::info!(count = count, "Pruned stale panes from cache");
    }
    count
}

/// Background task: prune stale panes every 60 seconds.
fn spawn_stale_pane_cleaner(state: Arc<AppState>) {
    std::thread::Builder::new()
        .name("pane-cleanup".into())
        .spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_secs(60));
                let removed = cleanup_stale_panes(&state);
                if removed > 0 {
                    tracing::debug!(
                        remaining = { state.pane_cache.read().unwrap().len() },
                        "Stale pane cleanup cycle"
                    );
                }
            }
        })
        .expect("Failed to spawn pane cleanup thread");
}

// === Helpers ===
fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!("{:x}{:x}", t.as_secs(), t.subsec_nanos())
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!("{}", t.as_secs())
}

fn now_epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// === Main ===
#[tokio::main]
async fn main() -> Result<()> {
    // Handle self-update and --version before any initialization
    {
        let args: Vec<String> = std::env::args().collect();
        if args.len() > 1 {
            match args[1].as_str() {
                #[cfg(feature = "self-update")]
                "self-update" => {
                    self_update::run("tf-web")?;
                    return Ok(());
                }
                "--version" | "-V" => {
                    println!("tf-web {}", env!("CARGO_PKG_VERSION"));
                    return Ok(());
                }
                _ => {}
            }
        }
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tf_web=info".into()),
        )
        .init();

    let port: u16 = std::env::var("TF_WEB_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .context("Invalid TF_WEB_PORT")?;
    let socket_path = std::env::var("TF_TMUX_SOCKET").unwrap_or_else(|_| {
        // Auto-detect: try common tmux socket locations
        let uid = {
            let output = std::process::Command::new("id").arg("-u").output();
            output
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .and_then(|s| s.trim().parse::<u32>().ok())
                .unwrap_or(1000)
        };
        let candidates = [
            format!("/tmp/tmux-{}/default", uid),
            format!("/private/tmp/tmux-{}/default", uid),
        ];
        for path in &candidates {
            if std::path::Path::new(path).exists() {
                return path.clone();
            }
        }
        candidates[0].clone() // best guess: Linux path
    });
    let tmux_bin = tmux_capture::find_tf_mux_binary().unwrap_or_else(|| {
        tracing::warn!(
            "tf-mux not found; capture loop will be disabled. \
                 Set TMUX_BIN env var to enable."
        );
        PathBuf::from("tf-mux") // placeholder; loop won't run
    });
    // Broadcast channel for pushing pane updates to WebSocket clients (capacity 256)
    let (broadcast_tx, _) = tokio::sync::broadcast::channel::<String>(256);

    let poll_ms: u64 = std::env::var("TF_POLL_MS")
        .unwrap_or_else(|_| "500".to_string())
        .parse()
        .context("Invalid TF_POLL_MS")?;
    let state = Arc::new(AppState {
        agents: TokioRwLock::new(HashMap::new()),
        connections: TokioRwLock::new(HashMap::new()),
        pane_cache: RwLock::new(HashMap::new()),
        broadcast_tx,
        socket_path,
        tmux_bin,
        pane_cache_path: std::env::var("TF_PANE_CACHE")
            .ok()
            .map(|s| std::path::PathBuf::from(s.trim()))
            .unwrap_or_else(|| {
                // Default: next to the executable
                std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|d| d.join("tf-web-panes.json")))
                    .unwrap_or_else(|| std::env::temp_dir().join("tf-web-panes.json"))
            }),
    });

    // Load persisted pane cache from disk.
    {
        let path = &state.pane_cache_path;
        if path.exists() {
            match std::fs::read_to_string(path) {
                Ok(data) => {
                    match serde_json::from_str::<Vec<CachedPane>>(&data) {
                        Ok(panes) => {
                            let before = panes.len();
                            let mut cache = state.pane_cache.write().unwrap();
                            let now = now_epoch_secs();
                            // Filter out stale panes on load
                            let fresh: Vec<CachedPane> = panes
                                .into_iter()
                                .filter(|p| now.saturating_sub(p.updated_at) <= STALE_PANE_TTL_SECS)
                                .collect();
                            let pruned = before - fresh.len();
                            for pane in fresh {
                                cache.insert(pane.pane_id.clone(), pane);
                            }
                            let count = cache.len();
                            drop(cache);
                            if pruned > 0 {
                                tracing::info!(
                                    loaded = count,
                                    pruned = pruned,
                                    "Loaded persisted pane cache (pruned stale)"
                                );
                            } else {
                                tracing::info!(count = count, "Loaded persisted pane cache");
                            }
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "Failed to parse pane cache file");
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to read pane cache file");
                }
            }
        }
    }
    // Spawn background stale pane cleaner (prunes panes not updated in 5 min)
    spawn_stale_pane_cleaner(state.clone());
    // Spawn background tmux capture loop on a dedicated OS thread
    let capture_state = state.clone();
    std::thread::Builder::new()
        .name("tmux-capture".into())
        .spawn(move || {
            tmux_capture::run_capture_loop_blocking(capture_state, poll_ms);
        })
        .expect("Failed to spawn capture thread");
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        // Frontend
        .route("/", get(frontend))
        // Health
        .route("/health", get(health))
        // Agent management
        .route("/api/agents", post(register_agent))
        // Pane operations
        .route("/api/panes", get(list_panes))
        .route("/api/layout", get(get_layout))
        .route("/api/panes/{pane_id}/capture", get(capture_pane))
        .route("/api/panes/{pane_id}/send", post(send_keys))
        // Ingest endpoint
        .route("/api/ingest", post(ingest_panes))
        // WebSocket terminal
        .route("/ws/terminal/{pane_id}", get(ws_handler))
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("tf-web listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
// === Tests ===
#[cfg(test)]
mod tests {
    use super::*;

    fn test_state() -> Arc<AppState> {
        let (broadcast_tx, _) = tokio::sync::broadcast::channel::<String>(256);
        Arc::new(AppState {
            agents: TokioRwLock::new(HashMap::new()),
            connections: TokioRwLock::new(HashMap::new()),
            pane_cache: RwLock::new(HashMap::new()),
            broadcast_tx,
            socket_path: "/tmp/test-tmux".to_string(),
            tmux_bin: PathBuf::from("/nonexistent/tf-mux"),
            pane_cache_path: std::env::temp_dir().join("tf-test-panes.json"),
        })
    }
    #[tokio::test]
    async fn test_health_endpoint() {
        let resp = health().await;
        let body: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&resp.0).unwrap()).unwrap();
        assert_eq!(body["status"], "ok");
        assert_eq!(body["service"], "tf-web");
        assert_eq!(body["version"], "0.2.0");
    }
    #[tokio::test]
    async fn test_register_agent() {
        let state = test_state();
        let req = RegisterAgentRequest {
            name: "test-agent".to_string(),
        };
        let resp = register_agent(State(state.clone()), Json(req))
            .await
            .unwrap();
        let body = resp.0;
        assert!(body.success);
        let data = body.data.unwrap();
        assert_eq!(data.agent_name, "test-agent");
        assert!(data.api_key.starts_with("tf_"));
        assert!(data.api_key.len() > 10);
        // Verify agent was stored
        let agents = state.agents.read().await;
        assert!(agents.contains_key(&data.api_key));
        assert_eq!(agents[&data.api_key].name, "test-agent");
    }

    #[tokio::test]
    async fn test_ingest_single_pane() {
        let state = test_state();
        let req = IngestRequest {
            panes: vec![IngestPane {
                pane_id: "win-459012".to_string(),
                title: "PowerShell".to_string(),
                lines: vec!["line1".to_string(), "line2".to_string()],
                width: 80,
                height: 24,
                source: "windows".to_string(),
            }],
        };
        let resp = ingest_panes(State(state.clone()), Json(req)).await.unwrap();
        let body = resp.0;
        assert!(body.success);
        assert_eq!(body.data.unwrap(), "Ingested 1 panes");

        // Verify pane was stored in cache
        let cache = state.pane_cache.read().unwrap();
        assert_eq!(cache.len(), 1);
        let pane = cache.get("win-459012").unwrap();
        assert_eq!(pane.pane_id, "win-459012");
        assert_eq!(pane.tab_id, "PowerShell");
        assert_eq!(pane.lines, vec!["line1", "line2"]);
        assert_eq!(pane.width, 80);
        assert_eq!(pane.height, 24);
        assert_eq!(pane.source, "windows");
        assert_eq!(pane.workspace_id, "remote");
    }

    #[tokio::test]
    async fn test_ingest_multiple_panes() {
        let state = test_state();
        let req = IngestRequest {
            panes: vec![
                IngestPane {
                    pane_id: "win-100".to_string(),
                    title: "Terminal".to_string(),
                    lines: vec!["hello".to_string()],
                    width: 40,
                    height: 1,
                    source: "windows".to_string(),
                },
                IngestPane {
                    pane_id: "win-200".to_string(),
                    title: "Editor".to_string(),
                    lines: vec!["code".to_string(), "here".to_string()],
                    width: 120,
                    height: 2,
                    source: "macos".to_string(),
                },
            ],
        };
        let resp = ingest_panes(State(state.clone()), Json(req)).await.unwrap();
        let body = resp.0;
        assert!(body.success);
        assert_eq!(body.data.unwrap(), "Ingested 2 panes");

        let cache = state.pane_cache.read().unwrap();
        assert_eq!(cache.len(), 2);
        assert_eq!(cache["win-100"].source, "windows");
        assert_eq!(cache["win-200"].source, "macos");
    }

    #[tokio::test]
    async fn test_ingest_overwrites_existing_pane() {
        let state = test_state();
        // First ingest
        let req1 = IngestRequest {
            panes: vec![IngestPane {
                pane_id: "win-1".to_string(),
                title: "Old".to_string(),
                lines: vec!["old".to_string()],
                width: 40,
                height: 1,
                source: "windows".to_string(),
            }],
        };
        let _ = ingest_panes(State(state.clone()), Json(req1))
            .await
            .unwrap();
        // Second ingest overwrites same pane_id
        let req2 = IngestRequest {
            panes: vec![IngestPane {
                pane_id: "win-1".to_string(),
                title: "New".to_string(),
                lines: vec!["new".to_string()],
                width: 80,
                height: 2,
                source: "windows".to_string(),
            }],
        };
        let _ = ingest_panes(State(state.clone()), Json(req2))
            .await
            .unwrap();

        let cache = state.pane_cache.read().unwrap();
        assert_eq!(cache.len(), 1);
        assert_eq!(cache["win-1"].tab_id, "New");
        assert_eq!(cache["win-1"].lines, vec!["new"]);
    }

    #[tokio::test]
    async fn test_list_panes_after_ingest() {
        let state = test_state();
        let req = IngestRequest {
            panes: vec![IngestPane {
                pane_id: "win-999".to_string(),
                title: "MyTab".to_string(),
                lines: vec![],
                width: 80,
                height: 24,
                source: "windows".to_string(),
            }],
        };
        let _ = ingest_panes(State(state.clone()), Json(req)).await.unwrap();

        // Register an agent and get API key for auth
        let reg_resp = register_agent(
            State(state.clone()),
            Json(RegisterAgentRequest {
                name: "test".to_string(),
            }),
        )
        .await
        .unwrap();
        let api_key = reg_resp.0.data.unwrap().api_key;
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            "authorization",
            format!("Bearer {}", api_key).parse().unwrap(),
        );

        let resp = list_panes(State(state), headers).await.unwrap();
        let body = resp.0;
        assert!(body.success);
        let panes = body.data.unwrap();
        assert_eq!(panes.len(), 1);
        assert_eq!(panes[0].pane_id, "win-999");
        assert_eq!(panes[0].source, "windows");
    }

    #[test]
    fn test_stale_pane_cleanup() {
        let state = test_state();
        let now = now_epoch_secs();

        // Insert a fresh pane and a stale pane (> 5 min old)
        {
            let mut cache = state.pane_cache.write().unwrap();
            cache.insert(
                "fresh-pane".to_string(),
                CachedPane {
                    pane_id: "fresh-pane".to_string(),
                    workspace_id: "test-ws".to_string(),
                    tab_id: "Tab1".to_string(),
                    lines: vec!["line1".to_string()],
                    width: 80,
                    height: 24,
                    cursor_row: 0,
                    cursor_col: 0,
                    source: "windows".to_string(),
                    updated_at: now, // just updated
                },
            );
            cache.insert(
                "stale-pane".to_string(),
                CachedPane {
                    pane_id: "stale-pane".to_string(),
                    workspace_id: "test-ws".to_string(),
                    tab_id: "Tab2".to_string(),
                    lines: vec!["old".to_string()],
                    width: 80,
                    height: 24,
                    cursor_row: 0,
                    cursor_col: 0,
                    source: "windows".to_string(),
                    updated_at: now - 600, // 10 minutes ago (beyond 5 min TTL)
                },
            );
        }

        let removed = cleanup_stale_panes(&state);
        assert_eq!(removed, 1, "should remove exactly 1 stale pane");

        let cache = state.pane_cache.read().unwrap();
        assert_eq!(cache.len(), 1, "only fresh pane should remain");
        assert!(cache.contains_key("fresh-pane"));
        assert!(!cache.contains_key("stale-pane"));
    }
}
