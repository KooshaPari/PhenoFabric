//! Individual protocol message handlers for the wire server.

use crate::coordinator::Coordinator;

/// Handle a compile_request message: run compile_multihop on the current topology.
pub(crate) fn handle_compile_request(
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
pub(crate) fn handle_webrtc_offer(
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
                r#"{"type":"webrtc_error","error":"missing_sdp","message":"sdp field required"}"#
                    .into(),
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
pub(crate) fn handle_webrtc_answer(
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

/// Handle a save_config message: update in-memory config and persist to disk.
///
/// Accepts either:
/// - A full config replacement via `"config": { ... }` (parsed as DaemonConfig)
/// - Partial overrides via `"overrides": { ... }` (merged onto current config)
///
/// Returns the updated config snapshot on success.
pub(crate) fn handle_save_config(
    parsed: &serde_json::Value,
    coordinator: &Coordinator,
) -> Option<String> {
    if let Some(config_value) = parsed.get("config") {
        // Full config replacement.
        match serde_json::from_value::<crate::config::DaemonConfig>(config_value.clone()) {
            Ok(new_config) => {
                coordinator.update_config(new_config);
                Some(format!(
                    r#"{{"type":"save_config_response","status":"ok","config":{}}}"#,
                    coordinator.config_snapshot()
                ))
            }
            Err(e) => Some(format!(
                r#"{{"type":"save_config_error","error":"invalid_config","message":"{}"}}"#,
                e
            )),
        }
    } else if let Some(overrides) = parsed.get("overrides") {
        // Partial override.
        match coordinator.apply_config_overrides(overrides) {
            Ok(()) => Some(format!(
                r#"{{"type":"save_config_response","status":"ok","config":{}}}"#,
                coordinator.config_snapshot()
            )),
            Err(e) => Some(format!(
                r#"{{"type":"save_config_error","error":"apply_failed","message":"{}"}}"#,
                e
            )),
        }
    } else {
        Some(
            r#"{"type":"save_config_error","error":"missing_payload","message":"provide either config or overrides field"}"#.into(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::protocol::process_message;
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
    fn process_webrtc_offer_requires_target() {
        let coord = make_coordinator();
        let msg = r#"{"type":"webrtc_offer"}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("MISSING_FIELD") || resp.contains("missing_target"));
    }

    #[test]
    fn process_webrtc_offer_with_target() {
        let coord = make_coordinator();
        let msg = r#"{"type":"webrtc_offer","target":"node-1","sdp":"v=0..."}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("webrtc_answer"));
        assert!(resp.contains("relay_pending"));
    }

    #[test]
    fn process_webrtc_ice() {
        let coord = make_coordinator();
        let msg = r#"{"type":"webrtc_ice","from":"browser","candidate":"candidate:..."}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("webrtc_ice_ack"));
        assert!(resp.contains("browser"));
    }

    #[test]
    fn process_probe_request_with_nodes() {
        let coord = make_coordinator();

        // Add a node to the topology.
        let topo = fabric_graph::builder::TopologyBuilder::new()
            .with_name("test-topo")
            .add(
                fabric_graph::Node::new(
                    fabric_graph::model::NodeId::new("n1"),
                    fabric_graph::LocalityTier::L5Loopback,
                )
                .with_label("Node One"),
            )
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
    fn process_compile_request_missing_source() {
        let coord = make_coordinator();
        let msg = r#"{"type":"compile_request","destination":"b"}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("MISSING_FIELD") || resp.contains("missing_source"));
    }

    #[test]
    fn process_compile_request_missing_destination() {
        let coord = make_coordinator();
        let msg = r#"{"type":"compile_request","source":"a"}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("MISSING_FIELD") || resp.contains("missing_destination"));
    }

    #[test]
    fn process_compile_request_empty_topology_returns_error() {
        let coord = make_coordinator();
        // Empty topology -> compile fails.
        let msg = r#"{"type":"compile_request","source":"a","destination":"b"}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("compile_error") || resp.contains("compile_failed"));
    }

    #[test]
    fn process_save_config_overrides() {
        let coord = make_coordinator();
        let msg = r#"{"type":"save_config","overrides":{"server":{"listen":"0.0.0.0:5555"},"logging":{"level":"trace"}}}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("save_config_response"));
        assert!(resp.contains("ok"));
        assert!(resp.contains("0.0.0.0:5555"));
        assert!(resp.contains("trace"));
    }

    #[test]
    fn process_save_config_missing_payload() {
        let coord = make_coordinator();
        let msg = r#"{"type":"save_config"}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("save_config_error"));
        assert!(resp.contains("missing_payload"));
    }

    #[test]
    fn process_save_config_full_replacement() {
        let coord = make_coordinator();
        let msg = r#"{"type":"save_config","config":{"server":{"listen":"0.0.0.0:8888","max_connections":32,"request_timeout_ms":10000},"database":{"path":"state.db","wal_mode":true,"flush_interval_ms":1000},"topology":{"auto_probe":false,"probe_interval_s":60,"epoch_persistence":true},"leases":{"default_ttl_s":3600,"max_ttl_s":86400,"renewal_window_s":300,"fairness_policy":"FairShare"},"logging":{"level":"debug","format":"compact","file":null},"auth":{"enabled":false,"workos_client_id":"","workos_client_secret":"","workos_redirect_uri":"","infisical_client_id":"","infisical_client_secret":"","infisical_project_id":"","jwt_secret":null,"public_routes":["health_check","status_check"]}}}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("save_config_response"));
        assert!(resp.contains("ok"));
        // Verify the config was actually updated.
        let snapshot = coord.config_snapshot();
        let parsed: serde_json::Value = serde_json::from_str(&snapshot).unwrap();
        assert_eq!(parsed["server"]["listen"], "0.0.0.0:8888");
        assert_eq!(parsed["logging"]["level"], "debug");
    }

    #[test]
    fn process_save_config_persists_to_file() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let cfg_path = dir.path().join("daemon.toml");

        let config = crate::config::DaemonConfig {
            database: crate::config::DatabaseConfig {
                path: db_path,
                ..Default::default()
            },
            ..Default::default()
        };
        let coord = Arc::new(Coordinator::new(config).unwrap());
        coord.set_config_path(cfg_path.clone());

        let msg = r#"{"type":"save_config","overrides":{"server":{"listen":"0.0.0.0:9999"}}}"#;
        let resp = process_message(msg, &coord).unwrap();
        assert!(resp.contains("save_config_response"));

        // Verify the file was written.
        assert!(cfg_path.exists());
        let content = std::fs::read_to_string(&cfg_path).unwrap();
        assert!(content.contains("0.0.0.0:9999"));
    }
}
