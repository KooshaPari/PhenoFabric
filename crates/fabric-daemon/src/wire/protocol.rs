//! Wire protocol message types, parsing, and serialization.
//!
//! Defines the JSON wire format used between the daemon and its clients,
//! and provides the top-level message dispatcher.

use crate::coordinator::Coordinator;

use fabric_frame_transport::validation::{validate_message, validation_error_response};

use super::handlers;
use tracing::warn;

/// Process a single wire message and return an optional response.
pub fn process_message(message: &str, coordinator: &Coordinator) -> Option<String> {
    // Validate the incoming message before processing.
    let validated = match validate_message(message) {
        Ok(v) => v,
        Err(e) => {
            warn!(
                code = %e.code,
                message = %e.message,
                "wire protocol validation failed"
            );
            return Some(validation_error_response(&e));
        }
    };

    let msg_type = &validated.msg_type;

    match msg_type.as_str() {
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
            handlers::handle_webrtc_offer(&validated.value, coordinator)
        }
        "webrtc_answer" | "WebRTCAnswer" => {
            handlers::handle_webrtc_answer(&validated.value, coordinator)
        }
        "webrtc_ice" | "WebRTCIce" => {
            // ICE candidate relay -- acknowledge receipt.
            Some(format!(
                r#"{{"type":"webrtc_ice_ack","status":"ok","from":"{}"}}"#,
                validated.value.get("from").and_then(|v| v.as_str()).unwrap_or("unknown")
            ))
        }
        "compile_request" | "CompileRequest" => {
            handlers::handle_compile_request(&validated.value, coordinator)
        }
        "save_config" | "SaveConfig" => {
            handlers::handle_save_config(&validated.value, coordinator)
        }
        _ => Some(format!(
            r#"{{"error":"unknown_message","type":"{}"}}"#,
            msg_type
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn make_coordinator() -> Arc<Coordinator> {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let config = crate::config::DaemonConfig {
            database: crate::config::DatabaseConfig {
                path: db_path,
                ..Default::default()
            },
            ..Default::default()
        };
        Arc::new(Coordinator::new(config).unwrap())
    }

    #[test]
    fn process_heartbeat() {
        let coord = make_coordinator();
        let msg = r#"{"type":"heartbeat"}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("heartbeat_ack"));
    }

    #[test]
    fn process_health_check() {
        let coord = make_coordinator();
        let msg = r#"{"type":"health_check"}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("healthy"));
    }

    #[test]
    fn process_probe_request_returns_topology() {
        let coord = make_coordinator();
        let msg = r#"{"type":"probe_request"}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("probe_response"));
        assert!(resp.contains("topology_epoch"));
        assert!(resp.contains("node_count"));
        assert!(resp.contains("edge_count"));
    }

    #[test]
    fn process_topology_request() {
        let coord = make_coordinator();
        let msg = r#"{"type":"topology_request"}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("probe_response"));
        assert!(resp.contains("nodes"));
    }

    #[test]
    fn process_routes_request() {
        let coord = make_coordinator();
        let msg = r#"{"type":"routes_request"}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("routes_response"));
        assert!(resp.contains("routes"));
    }

    #[test]
    fn process_capabilities_request() {
        let coord = make_coordinator();
        let msg = r#"{"type":"capabilities_request"}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("capabilities_response"));
        assert!(resp.contains("capabilities"));
    }

    #[test]
    fn process_invalid_json() {
        let coord = make_coordinator();
        let resp = process_message("not json", &coord).unwrap();
        assert!(resp.contains("validation_error") || resp.contains("INVALID_JSON"));
    }

    #[test]
    fn process_unknown_type() {
        let coord = make_coordinator();
        let msg = r#"{"type":"foo_bar"}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("UNKNOWN_TYPE"));
    }

    #[test]
    fn process_missing_type_field() {
        let coord = make_coordinator();
        let msg = r#"{"foo":"bar"}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("MISSING_TYPE"));
    }

    #[test]
    fn process_compile_request_missing_source_rejected() {
        let coord = make_coordinator();
        let msg = r#"{"type":"compile_request","destination":"b"}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("MISSING_FIELD"));
    }

    #[test]
    fn process_webrtc_offer_missing_target_rejected() {
        let coord = make_coordinator();
        let msg = r#"{"type":"webrtc_offer","sdp":"v=0..."}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("MISSING_FIELD"));
    }

    #[test]
    fn process_compile_request_wrong_type_rejected() {
        let coord = make_coordinator();
        let msg = r#"{"type":"compile_request","source":123,"destination":"b"}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("WRONG_TYPE"));
    }
}
