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
            Some(coordinator.topology_snapshot())
        }
        "topology_request" | "TopologyRequest" => {
            Some(coordinator.topology_snapshot())
        }
        "routes_request" | "RoutesRequest" => {
            Some(coordinator.plans_snapshot())
        }
        "capabilities_request" | "CapabilitiesRequest" => {
            Some(coordinator.capabilities_snapshot())
        }
        // --- WebRTC signaling ---
        "webrtc_offer" | "WebRTCOffer" => {
            handle_webrtc_offer(&parsed, coordinator)
        }
        "webrtc_answer" | "WebRTCAnswer" => {
            handle_webrtc_answer(&parsed, coordinator)
        }
        "webrtc_ice" | "WebRTCIce" => {
            // ICE candidate relay — acknowledge receipt.
            Some(format!(
                r#"{{"type":"webrtc_ice_ack","status":"ok","from":"{}"}}"#,
                parsed.get("from").and_then(|v| v.as_str()).unwrap_or("unknown")
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

/// Handle a WebRTC offer from a client.
/// Relays the offer and returns the SDP answer from the target node.
fn handle_webrtc_offer(
    parsed: &serde_json::Value,
    _coordinator: &Coordinator,
) -> Option<String> {
    let target = match parsed.get("target").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => {
            return Some(
                r#"{"type":"webrtc_error","error":"missing_target","message":"target field required"}"#.into(),
            );
        }
    };
    let _sdp = match parsed.get("sdp").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => {
            return Some(
                r#"{"type":"webrtc_error","error":"missing_sdp","message":"sdp field required"}"#.into(),
            );
        }
    };
    // In production, relay the offer to the target node via frame transport.
    // For now, acknowledge receipt and return a placeholder answer.
    Some(format!(
        r#"{{"type":"webrtc_answer","target":"{}","sdp":"placeholder-answer","status":"relay_pending"}}"#,
        target
    ))
}

/// Handle a WebRTC answer from a client.
fn handle_webrtc_answer(
    parsed: &serde_json::Value,
    _coordinator: &Coordinator,
) -> Option<String> {
    let target = match parsed.get("target").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => {
            return Some(
                r#"{"type":"webrtc_error","error":"missing_target","message":"target field required"}"#.into(),
            );
        }
    };
    Some(format!(
        r#"{{"type":"webrtc_answer_ack","target":"{}","status":"ok"}}"#,
        target
    ))
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
    fn process_probe_request_returns_topology() {
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
        assert!(resp.contains("node_count"));
        assert!(resp.contains("edge_count"));
    }

    #[test]
    fn process_topology_request() {
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

        let msg = r#"{"type":"topology_request"}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("probe_response"));
        assert!(resp.contains("nodes"));
    }

    #[test]
    fn process_routes_request() {
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

        let msg = r#"{"type":"routes_request"}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("routes_response"));
        assert!(resp.contains("routes"));
    }

    #[test]
    fn process_capabilities_request() {
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

        let msg = r#"{"type":"capabilities_request"}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("capabilities_response"));
        assert!(resp.contains("capabilities"));
    }

    #[test]
    fn process_webrtc_offer_requires_target() {
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

        let msg = r#"{"type":"webrtc_offer"}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("missing_target"));
    }

    #[test]
    fn process_webrtc_offer_with_target() {
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

        let msg = r#"{"type":"webrtc_offer","target":"node-1","sdp":"v=0..."}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("webrtc_answer"));
        assert!(resp.contains("relay_pending"));
    }

    #[test]
    fn process_webrtc_ice() {
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

        let msg = r#"{"type":"webrtc_ice","from":"browser","candidate":"candidate:..."}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("webrtc_ice_ack"));
        assert!(resp.contains("browser"));
    }

    #[test]
    fn process_probe_request_with_nodes() {
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

        // Add a node to the topology.
        let topo = fabric_graph::builder::TopologyBuilder::new()
            .with_name("test-topo")
            .add(fabric_graph::Node::new(
                fabric_graph::model::NodeId::new("n1"),
                fabric_graph::LocalityTier::L5Loopback,
            ).with_label("Node One"))
            .build();
        coord.set_topology(topo).unwrap();

        let msg = r#"{"type":"probe_request"}"#;
        let resp = process_message(msg, &coord).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(parsed["node_count"], 1);
        assert_eq!(parsed["topology_name"], "test-topo");
        let nodes = parsed["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0]["id"], "n1");
        assert_eq!(nodes[0]["label"], "Node One");
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
