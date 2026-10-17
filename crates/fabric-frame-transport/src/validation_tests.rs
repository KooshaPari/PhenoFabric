//! Tests for wire-protocol message validation.
//!
//! Split out of `validation.rs` so both files stay under the 500-line ceiling.

use super::*;
use crate::schema::{is_known_type, required_fields};

#[test]
fn valid_heartbeat() {
    let result = validate_message(r#"{"type":"heartbeat"}"#);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().msg_type, "heartbeat");
}

#[test]
fn valid_health_check() {
    let result = validate_message(r#"{"type":"health_check"}"#);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().msg_type, "health_check");
}

#[test]
fn valid_topology_request() {
    let result = validate_message(r#"{"type":"topology_request"}"#);
    assert!(result.is_ok());
}

#[test]
fn valid_routes_request() {
    let result = validate_message(r#"{"type":"routes_request"}"#);
    assert!(result.is_ok());
}

#[test]
fn valid_capabilities_request() {
    let result = validate_message(r#"{"type":"capabilities_request"}"#);
    assert!(result.is_ok());
}

#[test]
fn valid_probe_request() {
    let result = validate_message(r#"{"type":"probe_request"}"#);
    assert!(result.is_ok());
}

#[test]
fn valid_compile_request() {
    let result = validate_message(r#"{"type":"compile_request","source":"a","destination":"b"}"#);
    assert!(result.is_ok());
}

#[test]
fn valid_compile_request_with_optional_fields() {
    let result = validate_message(
        r#"{"type":"compile_request","source":"a","destination":"b","intent_name":"my-intent"}"#,
    );
    assert!(result.is_ok());
}

#[test]
fn valid_webrtc_offer() {
    let result = validate_message(r#"{"type":"webrtc_offer","target":"n1","sdp":"v=0..."}"#);
    assert!(result.is_ok());
}

#[test]
fn valid_webrtc_answer() {
    let result = validate_message(r#"{"type":"webrtc_answer","target":"n1","sdp":"v=0..."}"#);
    assert!(result.is_ok());
}

#[test]
fn valid_webrtc_ice() {
    let result =
        validate_message(r#"{"type":"webrtc_ice","from":"browser","candidate":"candidate:1"}"#);
    assert!(result.is_ok());
}

#[test]
fn invalid_json() {
    let result = validate_message("not json at all");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, "INVALID_JSON");
}

#[test]
fn not_object() {
    let result = validate_message(r#""just a string""#);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, "NOT_OBJECT");
}

#[test]
fn missing_type_field() {
    let result = validate_message(r#"{"foo":"bar"}"#);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, "MISSING_TYPE");
}

#[test]
fn type_not_string() {
    let result = validate_message(r#"{"type":123}"#);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, "MISSING_TYPE");
}

#[test]
fn unknown_message_type() {
    let result = validate_message(r#"{"type":"unknown_thing"}"#);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, "UNKNOWN_TYPE");
}

#[test]
fn compile_request_missing_source() {
    let result = validate_message(r#"{"type":"compile_request","destination":"b"}"#);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, "MISSING_FIELD");
    assert!(err.message.contains("source"));
}

#[test]
fn compile_request_missing_destination() {
    let result = validate_message(r#"{"type":"compile_request","source":"a"}"#);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, "MISSING_FIELD");
    assert!(err.message.contains("destination"));
}

#[test]
fn compile_request_wrong_type_source() {
    let result = validate_message(r#"{"type":"compile_request","source":123,"destination":"b"}"#);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, "WRONG_TYPE");
}

#[test]
fn compile_request_wrong_type_destination() {
    let result = validate_message(r#"{"type":"compile_request","source":"a","destination":true}"#);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, "WRONG_TYPE");
}

#[test]
fn compile_request_null_source() {
    let result = validate_message(r#"{"type":"compile_request","source":null,"destination":"b"}"#);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, "NULL_FIELD");
}

#[test]
fn webrtc_offer_missing_target() {
    let result = validate_message(r#"{"type":"webrtc_offer","sdp":"v=0..."}"#);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, "MISSING_FIELD");
}

#[test]
fn webrtc_offer_missing_sdp() {
    let result = validate_message(r#"{"type":"webrtc_offer","target":"n1"}"#);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, "MISSING_FIELD");
}

#[test]
fn webrtc_offer_wrong_type_target() {
    let result = validate_message(r#"{"type":"webrtc_offer","target":42,"sdp":"v=0..."}"#);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, "WRONG_TYPE");
}

#[test]
fn webrtc_ice_missing_from() {
    let result = validate_message(r#"{"type":"webrtc_ice","candidate":"c1"}"#);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, "MISSING_FIELD");
}

#[test]
fn webrtc_ice_missing_candidate() {
    let result = validate_message(r#"{"type":"webrtc_ice","from":"browser"}"#);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, "MISSING_FIELD");
}

#[test]
fn validation_error_response_format() {
    let err = ValidationError {
        message: "test error".into(),
        code: "TEST_CODE".into(),
    };
    let resp = validation_error_response(&err);
    assert!(resp.contains("TEST_CODE"));
    assert!(resp.contains("test error"));
    assert!(resp.contains("validation_error"));
}

#[test]
fn case_insensitive_types_still_work() {
    // PascalCase types should also be valid.
    assert!(validate_message(r#"{"type":"Heartbeat"}"#).is_ok());
    assert!(validate_message(r#"{"type":"HealthCheck"}"#).is_ok());
    assert!(validate_message(r#"{"type":"TopologyRequest"}"#).is_ok());
    assert!(validate_message(r#"{"type":"RoutesRequest"}"#).is_ok());
    assert!(validate_message(r#"{"type":"CapabilitiesRequest"}"#).is_ok());
    assert!(validate_message(r#"{"type":"ProbeRequest"}"#).is_ok());
}

#[test]
fn valid_save_config() {
    let result = validate_message(
        r#"{"type":"save_config","overrides":{"server":{"listen":"0.0.0.0:5555"}}}"#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().msg_type, "save_config");
}

#[test]
fn valid_save_config_empty() {
    // save_config has no required fields.
    let result = validate_message(r#"{"type":"save_config"}"#);
    assert!(result.is_ok());
}

#[test]
fn save_config_wrong_type_overrides() {
    let result = validate_message(r#"{"type":"save_config","overrides":"not-an-object"}"#);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, "WRONG_TYPE");
}

// --- network introspection + surface lease (added with the daemon arms) ---

#[test]
fn network_requests_are_known_and_parameterless() {
    for t in [
        "network_status_request",
        "network_stun_request",
        "network_tailscale_request",
    ] {
        assert!(is_known_type(t), "{t} should be a known type");
        assert!(
            validate_message(&format!(r#"{{"type":"{t}"}}"#)).is_ok(),
            "{t} should validate with no other fields"
        );
    }
}

#[test]
fn network_requests_pascal_case_accepted() {
    assert!(validate_message(r#"{"type":"NetworkStatusRequest"}"#).is_ok());
    assert!(validate_message(r#"{"type":"NetworkStunRequest"}"#).is_ok());
    assert!(validate_message(r#"{"type":"NetworkTailscaleRequest"}"#).is_ok());
}

#[test]
fn surface_lease_request_requires_named_fields() {
    let msg = r#"{"type":"surface_lease_request","name":"display-0","protocol":"posix","locality_floor":"L5"}"#;
    assert!(validate_message(msg).is_ok());
    assert_eq!(
        required_fields("surface_lease_request"),
        &["name", "protocol", "locality_floor"]
    );
}

#[test]
fn surface_lease_request_missing_name_rejected() {
    let msg = r#"{"type":"surface_lease_request","protocol":"posix","locality_floor":"L5"}"#;
    let err = validate_message(msg).unwrap_err();
    assert_eq!(err.code, "MISSING_FIELD");
    assert!(err.message.contains("name"));
}

#[test]
fn surface_lease_request_wrong_type_protocol_rejected() {
    let msg = r#"{"type":"surface_lease_request","name":"d","protocol":7,"locality_floor":"L5"}"#;
    assert_eq!(validate_message(msg).unwrap_err().code, "WRONG_TYPE");
}

#[test]
fn surface_lease_request_requires_rt_is_boolean() {
    let msg = r#"{"type":"surface_lease_request","name":"d","protocol":"posix","locality_floor":"L5","requires_rt":"yes"}"#;
    assert_eq!(validate_message(msg).unwrap_err().code, "WRONG_TYPE");

    let ok = r#"{"type":"surface_lease_request","name":"d","protocol":"posix","locality_floor":"L5","requires_rt":true}"#;
    assert!(validate_message(ok).is_ok());
}
