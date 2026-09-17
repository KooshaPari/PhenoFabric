//! Wire server startup logic for the orchestrator.
//!
//! Binds a TCP listener and dispatches messages through the coordinator,
//! wrapping the synchronous `fabric_daemon::wire::run_wire_server` in a
//! Tokio-compatible spawn.

use std::net::TcpListener;
use std::sync::Arc;

use fabric_daemon::auth::AuthMiddleware;
use fabric_daemon::coordinator::Coordinator;
use tracing::info;

/// Start the wire server on a background thread, returning the listener's
/// local address for status reporting.
///
/// This is a thin wrapper that creates the `TcpListener`, logs binding,
/// and delegates to `fabric_daemon::wire::run_wire_server`.
pub fn start_wire_server(
    coordinator: Arc<Coordinator>,
    listen_addr: &str,
    max_connections: usize,
    request_timeout_ms: u64,
    auth: Arc<AuthMiddleware>,
    runtime: Arc<tokio::runtime::Runtime>,
) -> Result<(), WireServerError> {
    let listener =
        TcpListener::bind(listen_addr).map_err(|e| WireServerError::Bind(e.to_string()))?;

    info!(addr = %listen_addr, "wire server binding");

    // Run the wire server (blocking until shutdown flag is set on the coordinator).
    fabric_daemon::wire::run_wire_server(
        listener,
        coordinator,
        max_connections,
        request_timeout_ms,
        auth,
        runtime,
    )
    .map_err(|e| WireServerError::Runtime(e.to_string()))
}

/// Errors from the wire server.
#[derive(Debug, thiserror::Error)]
pub enum WireServerError {
    #[error("bind error: {0}")]
    Bind(String),
    #[error("runtime error: {0}")]
    Runtime(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use fabric_daemon::config::{DaemonConfig, DatabaseConfig, ServerConfig};

    fn make_coordinator() -> (Arc<Coordinator>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("serve_test.db");
        let config = DaemonConfig {
            database: DatabaseConfig {
                path: db_path,
                ..Default::default()
            },
            server: ServerConfig {
                listen: "127.0.0.1:0".into(),
                ..Default::default()
            },
            ..Default::default()
        };
        (Arc::new(Coordinator::new(config).unwrap()), dir)
    }

    #[test]
    fn start_wire_server_bind_success() {
        let (coord, _dir) = make_coordinator();
        // Bind to port 0 to get an ephemeral port.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        drop(listener);

        // The coordinator's shutdown flag lets us exit the server quickly.
        coord.shutdown();

        let auth = Arc::new(AuthMiddleware::new(Default::default()));
        let runtime = Arc::new(
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap(),
        );

        let result = start_wire_server(coord, &addr, 64, 1000, auth, runtime);
        // Should succeed (bind + immediate shutdown).
        assert!(result.is_ok());
    }

    #[test]
    fn wire_server_bad_bind_fails() {
        let (coord, _dir) = make_coordinator();
        // Try to bind to a port that is clearly invalid.
        let auth = Arc::new(AuthMiddleware::new(Default::default()));
        let runtime = Arc::new(
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap(),
        );
        let result = start_wire_server(coord, "127.0.0.1:99999", 64, 1000, auth, runtime);
        assert!(result.is_err());
    }
}
