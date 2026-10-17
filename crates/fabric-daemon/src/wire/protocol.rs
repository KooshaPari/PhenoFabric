//! Wire protocol message types, parsing, and serialization.
//!
//! Defines the JSON wire format used between the daemon and its clients,
//! and provides the top-level message dispatcher.

use crate::coordinator::Coordinator;

use fabric_frame_transport::validation::{validate_message, validation_error_response};

use super::handlers;
use tracing::warn;

/// Process a single wire message and return an optional response.
pub async fn process_message(message: &str, coordinator: &Coordinator) -> Option<String> {
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
        "heartbeat" | "Heartbeat" => Some(r#"{"type":"heartbeat_ack","status":"ok"}"#.into()),
        "health_check" | "HealthCheck" => {
            let health = coordinator.health();
            Some(health.to_json())
        }
        "probe_request" | "ProbeRequest" => Some(coordinator.topology_snapshot()),
        "topology_request" | "TopologyRequest" => Some(coordinator.topology_snapshot()),
        "routes_request" | "RoutesRequest" => Some(coordinator.plans_snapshot()),
        "capabilities_request" | "CapabilitiesRequest" => Some(coordinator.capabilities_snapshot()),
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
                validated
                    .value
                    .get("from")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
            ))
        }
        "compile_request" | "CompileRequest" => {
            handlers::handle_compile_request(&validated.value, coordinator)
        }
        "save_config" | "SaveConfig" => handlers::handle_save_config(&validated.value, coordinator),
        // --- Network introspection (fabric network status|stun|tailscale) ---
        // These probe the daemon host rather than any shared state, so they
        // need no coordinator access. They do real network I/O, hence `.await`.
        "network_status_request" | "NetworkStatusRequest" => {
            super::network_handlers::handle_network_status_request().await
        }
        "network_stun_request" | "NetworkStunRequest" => {
            super::network_handlers::handle_network_stun_request().await
        }
        "network_tailscale_request" | "NetworkTailscaleRequest" => {
            super::network_handlers::handle_network_tailscale_request().await
        }
        // --- Surface plane ---
        "surface_lease_request" | "SurfaceLeaseRequest" => {
            super::surface_handlers::handle_surface_lease_request(&validated.value, coordinator)
        }
        // --- Auth (public routes, pre-auth bootstrap) ---
        "auth_start" | "AuthStart" => super::auth_handlers::handle_auth_start(coordinator).await,
        "auth_complete" | "AuthComplete" => {
            super::auth_handlers::handle_auth_complete(&validated.value, coordinator).await
        }
        "auth_email" | "AuthEmail" => {
            super::auth_handlers::handle_auth_email(&validated.value, coordinator).await
        }
        "auth_status" | "AuthStatus" => super::auth_handlers::handle_auth_status(coordinator).await,
        "auth_verify" | "AuthVerify" => {
            super::auth_handlers::handle_auth_verify(&validated.value, coordinator).await
        }
        "auth_logout" | "AuthLogout" => super::auth_handlers::handle_auth_logout(coordinator).await,
        // --- Frame plane (fabric-frame-transport message types) ---
        // These are the JSON projection of the frame protocol's `SessionInit`,
        // `FrameData`, and `FrameAck` messages. The daemon wire server is
        // line-delimited JSON while the frame transport is length-prefixed
        // binary, so `frame_handlers` deserializes each line into the real
        // transport types rather than inventing a parallel message shape.
        //
        // `session_ack`, `frame_ack_response`, and `frame_error` are response
        // types the server emits; they are never accepted as requests.
        "session_init" | "SessionInit" => Some(super::frame_handlers::handle_session_init(
            &validated.value,
            coordinator,
        )),
        "frame_data" | "FrameData" => Some(super::frame_handlers::handle_frame_data(
            &validated.value,
            coordinator,
        )),
        "frame_ack" | "FrameAck" => Some(super::frame_handlers::handle_frame_ack(
            &validated.value,
            coordinator,
        )),
        // Session teardown. Same wiring as the frame messages above: the line
        // is validated here, then handed to a frame handler that owns the
        // session-state transition.
        "session_close" | "SessionClose" => Some(super::frame_handlers::handle_session_close(
            &validated.value,
            coordinator,
        )),
        "session_cancel" | "SessionCancel" => Some(super::frame_handlers::handle_session_cancel(
            &validated.value,
            coordinator,
        )),
        // Resume. A reconnecting client re-presents its session id to continue
        // the same session; the handler lives in its own module so
        // `frame_handlers` stays under the project's line ceiling.
        "session_resume" | "SessionResume" => Some(super::frame_resume::handle_session_resume(
            &validated.value,
            coordinator,
        )),
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

    #[tokio::test]
    async fn process_heartbeat() {
        let coord = make_coordinator();
        let msg = r#"{"type":"heartbeat"}"#;
        let resp = process_message(msg, &coord).await.unwrap();
        assert!(resp.contains("heartbeat_ack"));
    }

    #[tokio::test]
    async fn process_health_check() {
        let coord = make_coordinator();
        let msg = r#"{"type":"health_check"}"#;
        let resp = process_message(msg, &coord).await.unwrap();
        assert!(resp.contains("healthy"));
    }

    #[tokio::test]
    async fn process_probe_request_returns_topology() {
        let coord = make_coordinator();
        let msg = r#"{"type":"probe_request"}"#;
        let resp = process_message(msg, &coord).await.unwrap();
        assert!(resp.contains("probe_response"));
        assert!(resp.contains("topology_epoch"));
        assert!(resp.contains("node_count"));
        assert!(resp.contains("edge_count"));
    }

    #[tokio::test]
    async fn process_topology_request() {
        let coord = make_coordinator();
        let msg = r#"{"type":"topology_request"}"#;
        let resp = process_message(msg, &coord).await.unwrap();
        assert!(resp.contains("probe_response"));
        assert!(resp.contains("nodes"));
    }

    #[tokio::test]
    async fn process_routes_request() {
        let coord = make_coordinator();
        let msg = r#"{"type":"routes_request"}"#;
        let resp = process_message(msg, &coord).await.unwrap();
        assert!(resp.contains("routes_response"));
        assert!(resp.contains("routes"));
    }

    #[tokio::test]
    async fn process_capabilities_request() {
        let coord = make_coordinator();
        let msg = r#"{"type":"capabilities_request"}"#;
        let resp = process_message(msg, &coord).await.unwrap();
        assert!(resp.contains("capabilities_response"));
        assert!(resp.contains("capabilities"));
    }

    #[tokio::test]
    async fn process_invalid_json() {
        let coord = make_coordinator();
        let resp = process_message("not json", &coord).await.unwrap();
        assert!(resp.contains("validation_error") || resp.contains("INVALID_JSON"));
    }

    #[tokio::test]
    async fn process_unknown_type() {
        let coord = make_coordinator();
        let msg = r#"{"type":"foo_bar"}"#;
        let resp = process_message(msg, &coord).await.unwrap();
        assert!(resp.contains("UNKNOWN_TYPE"));
    }

    #[tokio::test]
    async fn process_missing_type_field() {
        let coord = make_coordinator();
        let msg = r#"{"foo":"bar"}"#;
        let resp = process_message(msg, &coord).await.unwrap();
        assert!(resp.contains("MISSING_TYPE"));
    }

    #[tokio::test]
    async fn process_compile_request_missing_source_rejected() {
        let coord = make_coordinator();
        let msg = r#"{"type":"compile_request","destination":"b"}"#;
        let resp = process_message(msg, &coord).await.unwrap();
        assert!(resp.contains("MISSING_FIELD"));
    }

    #[tokio::test]
    async fn process_webrtc_offer_missing_target_rejected() {
        let coord = make_coordinator();
        let msg = r#"{"type":"webrtc_offer","sdp":"v=0..."}"#;
        let resp = process_message(msg, &coord).await.unwrap();
        assert!(resp.contains("MISSING_FIELD"));
    }

    #[tokio::test]
    async fn process_compile_request_wrong_type_rejected() {
        let coord = make_coordinator();
        let msg = r#"{"type":"compile_request","source":123,"destination":"b"}"#;
        let resp = process_message(msg, &coord).await.unwrap();
        assert!(resp.contains("WRONG_TYPE"));
    }

    // --- The four previously-unhandled message types ---
    //
    // Regression guards: these all used to return UNKNOWN_TYPE from
    // validation, before the dispatcher was ever consulted.

    #[tokio::test]
    async fn network_status_request_is_handled() {
        let coord = make_coordinator();
        let resp = process_message(r#"{"type":"network_status_request"}"#, &coord)
            .await
            .unwrap();
        assert!(
            !resp.contains("unknown_message") && !resp.contains("UNKNOWN_TYPE"),
            "network_status_request must not fall through: {resp}"
        );
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        for section in ["upnp", "stun", "tailscale"] {
            assert!(v[section]["status"].is_string(), "{section}.status missing");
        }
    }

    #[tokio::test]
    async fn network_stun_request_is_handled() {
        let coord = make_coordinator();
        let resp = process_message(r#"{"type":"network_stun_request"}"#, &coord)
            .await
            .unwrap();
        assert!(!resp.contains("UNKNOWN_TYPE"), "fell through: {resp}");
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert!(v.get("error").is_none(), "must not be a wire error: {resp}");
        assert!(v["external_port"].is_number());
    }

    #[tokio::test]
    async fn network_tailscale_request_is_handled() {
        let coord = make_coordinator();
        let resp = process_message(r#"{"type":"network_tailscale_request"}"#, &coord)
            .await
            .unwrap();
        assert!(!resp.contains("UNKNOWN_TYPE"), "fell through: {resp}");
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert!(v.get("error").is_none(), "must not be a wire error: {resp}");
        assert!(v["peers"].is_array());
    }

    #[tokio::test]
    async fn surface_lease_request_is_handled() {
        let coord = make_coordinator();
        let msg = r#"{"type":"surface_lease_request","name":"primary-display","protocol":"posix","locality_floor":"L5Loopback","requires_rt":false}"#;
        let resp = process_message(msg, &coord).await.unwrap();
        assert!(!resp.contains("UNKNOWN_TYPE"), "fell through: {resp}");
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert!(v["handle"].is_string());
        assert_eq!(v["state"], "Pending");
    }

    #[tokio::test]
    async fn surface_lease_request_missing_name_is_a_validation_error() {
        let coord = make_coordinator();
        let msg = r#"{"type":"surface_lease_request","protocol":"posix","locality_floor":"L5"}"#;
        let resp = process_message(msg, &coord).await.unwrap();
        assert!(resp.contains("MISSING_FIELD"), "got: {resp}");
    }

    #[tokio::test]
    async fn unknown_type_still_falls_through() {
        // The new arms must not accidentally swallow unrelated types.
        let coord = make_coordinator();
        let resp = process_message(r#"{"type":"totally_bogus"}"#, &coord)
            .await
            .unwrap();
        assert!(resp.contains("UNKNOWN_TYPE"), "got: {resp}");
    }

    // --- Frame plane ---
    //
    // These prove the three frame message types reach a handler instead of
    // falling through to `unknown_message`. Before the frame arms existed
    // they were rejected as `UNKNOWN_TYPE` by validation, because they were
    // absent from `REQUEST_MESSAGE_TYPES`.

    /// A valid `session_init` line, as the frame streamer sends it.
    fn session_init_line() -> &'static str {
        r#"{"type":"session_init","version":1,"preferred_codec":"Rgba","width":64,"height":48,"target_fps":60,"max_latency_ms":33,"client_id":"protocol-test"}"#
    }

    #[tokio::test]
    async fn session_init_is_dispatched_to_a_handler() {
        let coord = make_coordinator();
        let resp = process_message(session_init_line(), &coord).await.unwrap();
        assert!(!resp.contains("UNKNOWN_TYPE"), "fell through: {resp}");
        assert!(!resp.contains("unknown_message"), "fell through: {resp}");
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["type"], "session_ack");
        assert_eq!(v["session_id"], 1);
        assert_eq!(v["width"], 64);
        assert_eq!(v["height"], 48);
        assert_eq!(v["codec"], "Rgba");
    }

    #[tokio::test]
    async fn session_init_pascal_case_is_dispatched() {
        let coord = make_coordinator();
        let line = r#"{"type":"SessionInit","version":1,"preferred_codec":"Av1","width":32,"height":32,"target_fps":30,"max_latency_ms":33,"client_id":"c"}"#;
        let resp = process_message(line, &coord).await.unwrap();
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["type"], "session_ack");
        assert_eq!(v["codec"], "Av1");
    }

    #[tokio::test]
    async fn frame_data_is_dispatched_for_a_known_session() {
        let coord = make_coordinator();
        process_message(session_init_line(), &coord).await.unwrap();

        let frame = r#"{"type":"frame_data","session_id":1,"seq":0,"pts_us":0,"dts_us":0,"is_keyframe":true,"codec":"Rgba","width":64,"height":48,"payload_len":12288,"duration_us":16667}"#;
        let resp = process_message(frame, &coord).await.unwrap();
        assert!(!resp.contains("UNKNOWN_TYPE"), "fell through: {resp}");
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["type"], "frame_ack");
        assert_eq!(v["status"], "ok");
        assert_eq!(v["seq"], 0);
        assert_eq!(v["frame_count"], 1);
        assert_eq!(v["frames_received"], 12288);
    }

    #[tokio::test]
    async fn frame_data_without_a_session_init_is_an_unknown_session_error() {
        let coord = make_coordinator();
        let frame =
            r#"{"type":"frame_data","session_id":7,"seq":0,"codec":"Rgba","payload_len":16}"#;
        let resp = process_message(frame, &coord).await.unwrap();
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["type"], "frame_error");
        assert_eq!(v["error"], "unknown_session");
        assert_eq!(v["session_id"], 7);
    }

    #[tokio::test]
    async fn frame_ack_is_dispatched_for_a_known_session() {
        let coord = make_coordinator();
        process_message(session_init_line(), &coord).await.unwrap();
        let frame =
            r#"{"type":"frame_data","session_id":1,"seq":1,"codec":"Rgba","payload_len":32}"#;
        process_message(frame, &coord).await.unwrap();

        let ack =
            r#"{"type":"frame_ack","session_id":1,"seq":1,"rtt_us":4200,"recv_pts_us":16667}"#;
        let resp = process_message(ack, &coord).await.unwrap();
        assert!(!resp.contains("UNKNOWN_TYPE"), "fell through: {resp}");
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["type"], "frame_ack_response");
        assert_eq!(v["acked"], true);
        assert_eq!(v["rtt_us"], 4200);
    }

    #[tokio::test]
    async fn frame_data_missing_seq_is_a_validation_error() {
        let coord = make_coordinator();
        process_message(session_init_line(), &coord).await.unwrap();
        let frame = r#"{"type":"frame_data","session_id":1,"codec":"Rgba"}"#;
        let resp = process_message(frame, &coord).await.unwrap();
        assert!(resp.contains("MISSING_FIELD"), "got: {resp}");
    }

    #[tokio::test]
    async fn session_init_missing_client_id_is_a_validation_error() {
        let coord = make_coordinator();
        let line = r#"{"type":"session_init","version":1,"preferred_codec":"Rgba","width":64,"height":48,"target_fps":60,"max_latency_ms":33}"#;
        let resp = process_message(line, &coord).await.unwrap();
        assert!(resp.contains("MISSING_FIELD"), "got: {resp}");
    }

    #[tokio::test]
    async fn frame_data_with_a_non_integer_seq_is_a_validation_error() {
        // A float seq would silently become 0 if validation did not reject it.
        let coord = make_coordinator();
        process_message(session_init_line(), &coord).await.unwrap();
        let frame =
            r#"{"type":"frame_data","session_id":1,"seq":1.5,"codec":"Rgba","payload_len":8}"#;
        let resp = process_message(frame, &coord).await.unwrap();
        assert!(resp.contains("WRONG_TYPE"), "got: {resp}");
    }

    #[tokio::test]
    async fn frame_types_do_not_displace_the_other_arms() {
        // Adding the frame arms must not change how any pre-existing type is
        // dispatched.
        let coord = make_coordinator();
        let resp = process_message(r#"{"type":"heartbeat"}"#, &coord)
            .await
            .unwrap();
        assert!(resp.contains("heartbeat_ack"), "got: {resp}");
        let resp = process_message(r#"{"type":"health_check"}"#, &coord)
            .await
            .unwrap();
        assert!(resp.contains("healthy"), "got: {resp}");
    }

    #[tokio::test]
    async fn session_resume_is_dispatched_to_a_handler() {
        // Without a dispatch arm this would fall through to `unknown_message`
        // even though validation accepts the type -- the exact failure mode
        // that hid the frame path before it landed.
        let coord = make_coordinator();
        let resp = process_message(r#"{"type":"session_resume","session_id":1}"#, &coord)
            .await
            .unwrap();
        assert!(!resp.contains("unknown_message"), "fell through: {resp}");
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["type"], "frame_error");
        assert_eq!(v["error"], "unknown_session");
    }

    #[tokio::test]
    async fn session_resume_pascal_case_is_dispatched() {
        let coord = make_coordinator();
        process_message(session_init_line(), &coord).await.unwrap();
        let resp = process_message(r#"{"type":"SessionResume","session_id":1}"#, &coord)
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["type"], "session_resume_ack");
        assert_eq!(v["status"], "ok");
    }

    #[tokio::test]
    async fn session_resume_missing_session_id_is_a_validation_error() {
        let coord = make_coordinator();
        let resp = process_message(r#"{"type":"session_resume"}"#, &coord)
            .await
            .unwrap();
        assert!(resp.contains("MISSING_FIELD"), "got: {resp}");
    }
}
