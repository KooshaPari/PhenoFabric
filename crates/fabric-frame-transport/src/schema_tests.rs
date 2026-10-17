//! Tests for the wire-protocol schema registry.
//!
//! Split out of `schema.rs` so both files stay under the 500-line ceiling.

use super::*;

#[test]
fn schema_is_valid_json() {
    let parsed: serde_json::Value = serde_json::from_str(WIRE_PROTOCOL_SCHEMA)
        .expect("WIRE_PROTOCOL_SCHEMA must be valid JSON");
    assert!(parsed.is_object());
}

#[test]
fn validate_empty_object_fails() {
    let result = validate_against_schema(r#"{}"#, "wire");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, "MISSING_TYPE");
}

#[test]
fn validate_non_object_fails() {
    let result = validate_against_schema(r#""hello""#, "wire");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, "NOT_OBJECT");
}

#[test]
fn validate_invalid_json_fails() {
    let result = validate_against_schema("not json", "wire");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, "INVALID_JSON");
}

#[test]
fn validate_unknown_type_fails() {
    let result = validate_against_schema(r#"{"type":"foo"}"#, "wire");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, "UNKNOWN_TYPE");
}

#[test]
fn validate_heartbeat_passes() {
    let result = validate_against_schema(r#"{"type":"heartbeat"}"#, "wire");
    assert!(result.is_ok());
}

#[test]
fn validate_health_check_passes() {
    let result = validate_against_schema(r#"{"type":"health_check"}"#, "wire");
    assert!(result.is_ok());
}

#[test]
fn validate_compile_request_missing_source_fails() {
    let result = validate_against_schema(r#"{"type":"compile_request","destination":"b"}"#, "wire");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, "MISSING_FIELD");
}

#[test]
fn validate_compile_request_missing_destination_fails() {
    let result = validate_against_schema(r#"{"type":"compile_request","source":"a"}"#, "wire");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, "MISSING_FIELD");
}

#[test]
fn validate_compile_request_passes() {
    let result = validate_against_schema(
        r#"{"type":"compile_request","source":"a","destination":"b"}"#,
        "wire",
    );
    assert!(result.is_ok());
}

#[test]
fn validate_compile_request_wrong_type_fails() {
    let result = validate_against_schema(
        r#"{"type":"compile_request","source":123,"destination":"b"}"#,
        "wire",
    );
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, "WRONG_TYPE");
}

#[test]
fn validate_webrtc_offer_passes() {
    let result = validate_against_schema(
        r#"{"type":"webrtc_offer","target":"n1","sdp":"v=0..."}"#,
        "wire",
    );
    assert!(result.is_ok());
}

#[test]
fn validate_webrtc_offer_missing_target_fails() {
    let result = validate_against_schema(r#"{"type":"webrtc_offer","sdp":"v=0..."}"#, "wire");
    assert!(result.is_err());
}

#[test]
fn validate_compile_request_null_source_fails() {
    let result = validate_against_schema(
        r#"{"type":"compile_request","source":null,"destination":"b"}"#,
        "wire",
    );
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, "NULL_FIELD");
}

#[test]
fn validate_save_config_passes() {
    let result = validate_against_schema(
        r#"{"type":"save_config","overrides":{"server":{"listen":"0.0.0.0:5555"}}}"#,
        "wire",
    );
    assert!(result.is_ok());
}

#[test]
fn validate_save_config_empty_passes() {
    // save_config has no required fields.
    let result = validate_against_schema(r#"{"type":"save_config"}"#, "wire");
    assert!(result.is_ok());
}

// --- network introspection + surface lease ---

#[test]
fn network_requests_pass_schema() {
    for t in [
        "network_status_request",
        "network_stun_request",
        "network_tailscale_request",
    ] {
        let result = validate_against_schema(&format!(r#"{{"type":"{t}"}}"#), "wire");
        assert!(result.is_ok(), "{t} should pass schema validation");
    }
}

#[test]
fn surface_lease_request_passes_schema() {
    let result = validate_against_schema(
        r#"{"type":"surface_lease_request","name":"primary-display","protocol":"posix","locality_floor":"L5","requires_rt":true}"#,
        "wire",
    );
    assert!(result.is_ok());
}

#[test]
fn surface_lease_request_missing_name_fails_schema() {
    let result = validate_against_schema(
        r#"{"type":"surface_lease_request","protocol":"posix","locality_floor":"L5"}"#,
        "wire",
    );
    let err = result.unwrap_err();
    assert_eq!(err.code, "MISSING_FIELD");
    assert!(err.message.contains("name"));
}

#[test]
fn surface_lease_request_requires_rt_wrong_type_fails_schema() {
    let result = validate_against_schema(
        r#"{"type":"surface_lease_request","name":"d","protocol":"posix","locality_floor":"L5","requires_rt":"yes"}"#,
        "wire",
    );
    assert_eq!(result.unwrap_err().code, "WRONG_TYPE");
}

mod frame_types_tests {
    use super::*;

    #[test]
    fn frame_types_are_known() {
        for t in [
            "session_init",
            "session_close",
            "session_cancel",
            "SessionInit",
            "frame_data",
            "FrameData",
            "frame_ack",
            "FrameAck",
        ] {
            assert!(is_known_type(t), "{t} should be a known type");
        }
    }

    #[test]
    fn session_init_requires_client_id() {
        assert_eq!(required_fields("session_init"), &["client_id"]);
        assert_eq!(required_fields("SessionInit"), &["client_id"]);
    }

    #[test]
    fn teardown_types_require_session_id() {
        assert_eq!(required_fields("session_close"), &["session_id"]);
        assert_eq!(required_fields("session_cancel"), &["session_id"]);
    }

    #[test]
    fn valid_teardown_passes_schema() {
        for line in [
            r#"{"type":"session_close","session_id":1}"#,
            r#"{"type":"session_cancel","session_id":7}"#,
        ] {
            assert!(
                validate_against_schema(line, "wire").is_ok(),
                "should validate: {line}"
            );
        }
    }

    #[test]
    fn teardown_without_session_id_fails_schema() {
        for line in [
            r#"{"type":"session_close"}"#,
            r#"{"type":"session_cancel"}"#,
        ] {
            let err =
                validate_against_schema(line, "wire").expect_err("teardown must name its session");
            assert_eq!(err.code, "MISSING_FIELD", "for {line}: {err}");
        }
    }

    #[test]
    fn teardown_null_session_id_fails_schema() {
        let err = validate_against_schema(r#"{"type":"session_close","session_id":null}"#, "wire")
            .expect_err("null session_id must be rejected");
        assert_eq!(err.code, "NULL_FIELD");
    }

    #[test]
    fn frame_data_and_ack_require_seq() {
        assert_eq!(required_fields("frame_data"), &["seq"]);
        assert_eq!(required_fields("frame_ack"), &["seq"]);
    }

    #[test]
    fn session_init_with_client_id_passes_schema() {
        let result = validate_against_schema(
            r#"{"type":"session_init","version":1,"preferred_codec":"Rgba","width":64,"height":48,"target_fps":60,"max_latency_ms":33,"client_id":"c"}"#,
            "wire",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn session_init_missing_client_id_fails_schema() {
        let result = validate_against_schema(r#"{"type":"session_init","version":1}"#, "wire");
        assert_eq!(result.unwrap_err().code, "MISSING_FIELD");
    }

    #[test]
    fn frame_data_missing_seq_fails_schema() {
        let result = validate_against_schema(r#"{"type":"frame_data","codec":"Rgba"}"#, "wire");
        assert_eq!(result.unwrap_err().code, "MISSING_FIELD");
    }

    #[test]
    fn frame_types_do_not_shadow_response_types() {
        // `session_ack` and `frame_error` are server-emitted only. They must
        // not be accepted as request types.
        for t in ["session_ack", "frame_error", "frame_ack_response"] {
            assert!(!is_known_type(t), "{t} is a response type, not a request");
        }
    }
}

#[cfg(test)]
mod auth_types_tests {
    use super::*;

    #[test]
    fn auth_start_is_known_and_requires_nothing() {
        for t in ["auth_start", "AuthStart"] {
            assert!(is_known_type(t), "{t} should be a known type");
            assert!(required_fields(t).is_empty(), "{t} requires no fields");
        }
    }

    #[test]
    fn auth_complete_requires_code() {
        assert!(is_known_type("auth_complete"));
        assert_eq!(required_fields("auth_complete"), &["code"]);
    }

    #[test]
    fn auth_email_requires_email() {
        assert!(is_known_type("auth_email"));
        assert_eq!(required_fields("auth_email"), &["email"]);
    }

    #[test]
    fn auth_status_is_known_and_requires_nothing() {
        for t in ["auth_status", "AuthStatus"] {
            assert!(is_known_type(t), "{t} should be a known type");
            assert!(required_fields(t).is_empty(), "{t} requires no fields");
        }
    }

    #[test]
    fn auth_verify_requires_email_and_code() {
        assert!(is_known_type("auth_verify"));
        assert_eq!(required_fields("auth_verify"), &["email", "code"]);
    }

    #[test]
    fn auth_logout_is_known_and_requires_nothing() {
        for t in ["auth_logout", "AuthLogout"] {
            assert!(is_known_type(t), "{t} should be a known type");
            assert!(required_fields(t).is_empty(), "{t} requires no fields");
        }
    }

    #[test]
    fn unknown_still_rejected() {
        assert!(!is_known_type("auth_destroy"));
        assert!(!is_known_type("definitely_not_a_type"));
    }

    #[test]
    fn network_requests_are_known_and_require_nothing() {
        for t in [
            "network_status_request",
            "network_stun_request",
            "network_tailscale_request",
        ] {
            assert!(is_known_type(t), "{t} should be a known type");
            assert!(required_fields(t).is_empty(), "{t} requires no fields");
        }
    }

    #[test]
    fn surface_lease_request_is_known_and_requires_three_fields() {
        assert!(is_known_type("surface_lease_request"));
        assert!(is_known_type("SurfaceLeaseRequest"));
        assert_eq!(
            required_fields("surface_lease_request"),
            &["name", "protocol", "locality_floor"]
        );
    }
}
