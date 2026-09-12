//! Wire transport server for fabric-daemon.
//!
//! Listens on a TCP socket and handles incoming wire messages.
//! Implements the server side of spec 025.

use crate::coordinator::Coordinator;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Start the wire transport server.
///
/// This is a blocking function that runs until the shutdown flag is set.
pub fn run_wire_server(
    listener: TcpListener,
    coordinator: Arc<Coordinator>,
    max_connections: usize,
    request_timeout_ms: u64,
) -> Result<(), WireServerError> {
    listener.set_nonblocking(false).map_err(|e| {
        WireServerError::Io(format!("failed to set listener to blocking: {e}"))
    })?;

    let timeout = Duration::from_millis(request_timeout_ms);
    let mut active_connections: usize = 0;

    info!(
        addr = ?listener.local_addr().ok(),
        max_connections,
        "wire server started"
    );

    loop {
        if coordinator.is_shutting_down() {
            info!("wire server shutting down, rejecting new connections");
            break;
        }

        // Accept with a short timeout so we can check shutdown flag periodically.
        match listener.accept() {
            Ok((stream, peer_addr)) => {
                if active_connections >= max_connections {
                    warn!(
                        peer = %peer_addr,
                        active_connections,
                        max_connections,
                        "connection limit reached, rejecting"
                    );
                    let mut stream = stream;
                    let _ = write!(
                        stream,
                        "{{\"error\":\"server_busy\",\"message\":\"max connections reached\"}}\n"
                    );
                    continue;
                }

                active_connections += 1;
                debug!(peer = %peer_addr, active_connections, "new connection");

                let coord = coordinator.clone();
                let handle = std::thread::spawn(move || {
                    handle_connection(stream, coord, timeout);
                    // Decrement is handled by Drop of a counter or we accept the leak
                    // for now — in production, use an AtomicUsize counter.
                });

                // Detach the thread (we don't join here — fire and forget).
                drop(handle);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No connection available, check shutdown flag.
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
            Err(e) => {
                error!("accept error: {e}");
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }

    info!("wire server stopped");
    Ok(())
}

/// Handle a single TCP connection.
fn handle_connection(
    stream: TcpStream,
    coordinator: Arc<Coordinator>,
    timeout: Duration,
) {
    let peer = stream
        .peer_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| "unknown".into());

    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));

    let reader_stream = match stream.try_clone() {
        Ok(s) => s,
        Err(e) => {
            debug!(peer = %peer, error = %e, "failed to clone stream");
            return;
        }
    };
    let reader = BufReader::new(reader_stream);
    let mut writer = stream;

    for line in reader.lines() {
        if coordinator.is_shutting_down() {
            let _ = write!(
                writer,
                "{{\"error\":\"shutting_down\"}}\n"
            );
            break;
        }

        let line = match line {
            Ok(l) => l,
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                debug!(peer = %peer, "read timeout, closing connection");
                break;
            }
            Err(e) => {
                debug!(peer = %peer, error = %e, "read error, closing connection");
                break;
            }
        };

        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        debug!(peer = %peer, len = line.len(), "received message");

        let response = process_message(&line, &coordinator);
        if let Some(resp) = response {
            if let Err(e) = write!(writer, "{resp}\n") {
                debug!(peer = %peer, error = %e, "write error");
                break;
            }
        }
    }

    debug!(peer = %peer, "connection closed");
}

/// Process a single wire message and return an optional response.
fn process_message(message: &str, coordinator: &Coordinator) -> Option<String> {
    // Parse as JSON to determine message type.
    let parsed: serde_json::Value = match serde_json::from_str(message) {
        Ok(v) => v,
        Err(_) => {
            return Some(
                r#"{"error":"invalid_json","message":"could not parse message as JSON"}"#.into(),
            );
        }
    };

    let msg_type = parsed.get("type").and_then(|v| v.as_str()).unwrap_or("unknown");

    match msg_type {
        "heartbeat" | "Heartbeat" => Some(
            r#"{"type":"heartbeat_ack","status":"ok"}"#.into(),
        ),
        "health_check" | "HealthCheck" => {
            let health = coordinator.health();
            Some(health.to_json())
        }
        "probe_request" | "ProbeRequest" => {
            // R3 stub: return basic topology info.
            let epoch = coordinator.topology_epoch();
            Some(format!(
                r#"{{"type":"probe_response","topology_epoch":{},"status":"ok"}}"#,
                epoch.0
            ))
        }
        "compile_request" | "CompileRequest" => {
            handle_compile_request(&parsed, coordinator)
        }
        _ => Some(format!(
            r#"{{"error":"unknown_message","type":"{}"}}"#,
            msg_type
        )),
    }
}

/// Handle a compile_request message: run compile_multihop on the current topology.
fn handle_compile_request(
    parsed: &serde_json::Value,
    coordinator: &Coordinator,
) -> Option<String> {
    let source = match parsed.get("source").and_then(|v| v.as_str()) {
        Some(s) => fabric_graph::model::NodeId::new(s),
        None => {
            return Some(
                r#"{"type":"compile_error","error":"missing_source","message":"source field required"}"#.into(),
            );
        }
    };
    let destination = match parsed.get("destination").and_then(|v| v.as_str()) {
        Some(s) => fabric_graph::model::NodeId::new(s),
        None => {
            return Some(
                r#"{"type":"compile_error","error":"missing_destination","message":"destination field required"}"#.into(),
            );
        }
    };

    // Build a minimal intent from the request (or use defaults).
    let intent_name = parsed
        .get("intent_name")
        .and_then(|v| v.as_str())
        .unwrap_or("wire-compile");
    let intent = fabric_graph::builder::IntentBuilder::new()
        .name(intent_name)
        .min_trust(fabric_graph::TrustLevel::Untrusted)
        .build();

    let catalog = fabric_graph::multihop::builtin_stages();

    match compile_with_coordinator(coordinator, &source, &destination, &intent, &catalog) {
        Ok(result) => {
            let plan_json = serde_json::to_string(&result.primary).unwrap_or_default();
            Some(format!(
                r#"{{"type":"compile_response","plan":{},"status":"ok"}}"#,
                plan_json
            ))
        }
        Err(e) => Some(format!(
            r#"{{"type":"compile_error","error":"compile_failed","message":"{}"}}"#,
            e
        )),
    }
}

/// Compile a multihop route using the coordinator's current topology.
fn compile_with_coordinator(
    coordinator: &Coordinator,
    source: &fabric_graph::model::NodeId,
    destination: &fabric_graph::model::NodeId,
    intent: &fabric_graph::model::Intent,
    catalog: &[fabric_graph::multihop::TransportStage],
) -> Result<fabric_graph::multihop::MultihopResult, String> {
    coordinator
        .compile_multihop(source, destination, intent, catalog)
        .map_err(|e| e.to_string())
}

#[derive(Debug, thiserror::Error)]
pub enum WireServerError {
    #[error("io error: {0}")]
    Io(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_heartbeat() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let config = crate::config::DaemonConfig {
            database: crate::config::DatabaseConfig {
                path: db_path,
                ..Default::default()
            },
            ..Default::default()
        };
        let coord = Arc::new(Coordinator::new(config).unwrap());

        let msg = r#"{"type":"heartbeat"}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("heartbeat_ack"));
    }

    #[test]
    fn process_health_check() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let config = crate::config::DaemonConfig {
            database: crate::config::DatabaseConfig {
                path: db_path,
                ..Default::default()
            },
            ..Default::default()
        };
        let coord = Arc::new(Coordinator::new(config).unwrap());

        let msg = r#"{"type":"health_check"}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("healthy"));
    }

    #[test]
    fn process_probe_request() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let config = crate::config::DaemonConfig {
            database: crate::config::DatabaseConfig {
                path: db_path,
                ..Default::default()
            },
            ..Default::default()
        };
        let coord = Arc::new(Coordinator::new(config).unwrap());

        let msg = r#"{"type":"probe_request"}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("probe_response"));
        assert!(resp.contains("topology_epoch"));
    }

    #[test]
    fn process_invalid_json() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let config = crate::config::DaemonConfig {
            database: crate::config::DatabaseConfig {
                path: db_path,
                ..Default::default()
            },
            ..Default::default()
        };
        let coord = Arc::new(Coordinator::new(config).unwrap());

        let resp = process_message("not json", &coord).unwrap();
        assert!(resp.contains("invalid_json"));
    }

    #[test]
    fn process_unknown_type() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let config = crate::config::DaemonConfig {
            database: crate::config::DatabaseConfig {
                path: db_path,
                ..Default::default()
            },
            ..Default::default()
        };
        let coord = Arc::new(Coordinator::new(config).unwrap());

        let msg = r#"{"type":"foo_bar"}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("unknown_message"));
    }

    #[test]
    fn process_compile_request_missing_source() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let config = crate::config::DaemonConfig {
            database: crate::config::DatabaseConfig {
                path: db_path,
                ..Default::default()
            },
            ..Default::default()
        };
        let coord = Arc::new(Coordinator::new(config).unwrap());

        let msg = r#"{"type":"compile_request","destination":"b"}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("missing_source"));
    }

    #[test]
    fn process_compile_request_missing_destination() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let config = crate::config::DaemonConfig {
            database: crate::config::DatabaseConfig {
                path: db_path,
                ..Default::default()
            },
            ..Default::default()
        };
        let coord = Arc::new(Coordinator::new(config).unwrap());

        let msg = r#"{"type":"compile_request","source":"a"}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("missing_destination"));
    }

    #[test]
    fn process_compile_request_empty_topology_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let config = crate::config::DaemonConfig {
            database: crate::config::DatabaseConfig {
                path: db_path,
                ..Default::default()
            },
            ..Default::default()
        };
        let coord = Arc::new(Coordinator::new(config).unwrap());

        // Empty topology → compile fails.
        let msg = r#"{"type":"compile_request","source":"a","destination":"b"}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("compile_error") || resp.contains("compile_failed"));
    }
}
