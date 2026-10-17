//! Tests for the frame-plane field validation.
//!
//! Split out of `validation.rs` so both files stay under the 500-line ceiling.

use super::super::validation::validate_message;

const INIT: &str = r#"{"type":"session_init","version":1,"preferred_codec":"Rgba","width":64,"height":48,"target_fps":60,"max_latency_ms":33,"client_id":"c"}"#;

#[test]
fn valid_session_init() {
    assert!(validate_message(INIT).is_ok());
}

#[test]
fn session_init_missing_client_id_rejected() {
    let err = validate_message(r#"{"type":"session_init","version":1}"#).unwrap_err();
    assert_eq!(err.code, "MISSING_FIELD");
    assert!(err.message.contains("client_id"));
}

#[test]
fn session_init_non_string_client_id_rejected() {
    let msg = r#"{"type":"session_init","client_id":42}"#;
    assert_eq!(validate_message(msg).unwrap_err().code, "WRONG_TYPE");
}

#[test]
fn session_init_negative_dimension_rejected() {
    // A negative width would silently become 0 via `as u32`.
    let msg = r#"{"type":"session_init","client_id":"c","width":-1}"#;
    assert_eq!(validate_message(msg).unwrap_err().code, "WRONG_TYPE");
}

#[test]
fn session_init_float_dimension_rejected() {
    let msg = r#"{"type":"session_init","client_id":"c","height":48.5}"#;
    assert_eq!(validate_message(msg).unwrap_err().code, "WRONG_TYPE");
}

#[test]
fn valid_frame_data() {
    let msg = r#"{"type":"frame_data","session_id":1,"seq":3,"pts_us":50001,"dts_us":49000,"is_keyframe":true,"codec":"Rgba","width":64,"height":48,"payload_len":12288,"duration_us":16667}"#;
    assert!(validate_message(msg).is_ok());
}

#[test]
fn frame_data_missing_seq_rejected() {
    let msg = r#"{"type":"frame_data","session_id":1,"codec":"Rgba"}"#;
    assert_eq!(validate_message(msg).unwrap_err().code, "MISSING_FIELD");
}

#[test]
fn frame_data_string_seq_rejected() {
    let msg = r#"{"type":"frame_data","seq":"3","codec":"Rgba"}"#;
    assert_eq!(validate_message(msg).unwrap_err().code, "WRONG_TYPE");
}

#[test]
fn frame_data_float_seq_rejected() {
    let msg = r#"{"type":"frame_data","seq":1.5,"codec":"Rgba"}"#;
    assert_eq!(validate_message(msg).unwrap_err().code, "WRONG_TYPE");
}

#[test]
fn frame_data_negative_seq_rejected() {
    let msg = r#"{"type":"frame_data","seq":-1,"codec":"Rgba"}"#;
    assert_eq!(validate_message(msg).unwrap_err().code, "WRONG_TYPE");
}

#[test]
fn frame_data_non_boolean_keyframe_rejected() {
    let msg = r#"{"type":"frame_data","seq":0,"is_keyframe":"yes"}"#;
    assert_eq!(validate_message(msg).unwrap_err().code, "WRONG_TYPE");
}

#[test]
fn frame_data_payload_len_over_u32_max_rejected() {
    // MAX_FRAME_SIZE is 16 MiB, far below u32::MAX, but the field must
    // still be rejected if it does not fit a u32 at all.
    let msg = r#"{"type":"frame_data","seq":0,"payload_len":4294967296}"#;
    assert_eq!(validate_message(msg).unwrap_err().code, "WRONG_TYPE");
}

#[test]
fn valid_frame_ack() {
    let msg = r#"{"type":"frame_ack","session_id":1,"seq":3,"rtt_us":4200,"recv_pts_us":50001}"#;
    assert!(validate_message(msg).is_ok());
}

#[test]
fn frame_ack_missing_seq_rejected() {
    let msg = r#"{"type":"frame_ack","rtt_us":4200}"#;
    assert_eq!(validate_message(msg).unwrap_err().code, "MISSING_FIELD");
}

#[test]
fn frame_types_accept_pascal_case() {
    assert!(validate_message(&INIT.replace("session_init", "SessionInit")).is_ok());
    assert!(validate_message(r#"{"type":"FrameData","seq":0}"#).is_ok());
    assert!(validate_message(r#"{"type":"FrameAck","seq":0}"#).is_ok());
}

#[test]
fn response_types_are_not_valid_requests() {
    for t in ["session_ack", "frame_error", "frame_ack_response"] {
        let msg = format!(r#"{{"type":"{t}","seq":0}}"#);
        assert_eq!(
            validate_message(&msg).unwrap_err().code,
            "UNKNOWN_TYPE",
            "{t} is a response type"
        );
    }
}
