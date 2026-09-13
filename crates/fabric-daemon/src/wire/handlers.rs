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
}
