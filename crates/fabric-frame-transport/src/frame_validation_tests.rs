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
    let msg = r#"{"type":"frame_data","session_id":1,"seq":3,"pts_us":50001,"dts_us":49000,"is_keyframe":true,"codec":"Rgba","width":64,"height":48,"payload_len":12288,"duration_us":16667,"sender_id":"peer-1"}"#;
    assert!(validate_message(msg).is_ok());
}

#[test]
fn frame_data_missing_sender_id_rejected() {
    // The wire protocol makes `sender_id` a required field on every frame
    // so the daemon can attribute input to a specific peer without
    // trusting the sender's self-declaration alone.
    let msg = r#"{"type":"frame_data","session_id":1,"seq":3,"codec":"Rgba","width":64,"height":48,"payload_len":12288}"#;
    let err = validate_message(msg).unwrap_err();
    assert_eq!(err.code, "MISSING_FIELD");
    assert!(err.message.contains("sender_id"));
}

#[test]
fn frame_data_non_string_sender_id_rejected() {
    let msg = r#"{"type":"frame_data","sender_id":42,"seq":0,"codec":"Rgba"}"#;
    assert_eq!(validate_message(msg).unwrap_err().code, "WRONG_TYPE");
}

#[test]
fn frame_data_missing_seq_rejected() {
    let msg = r#"{"type":"frame_data","session_id":1,"codec":"Rgba","sender_id":"peer-1"}"#;
    assert_eq!(validate_message(msg).unwrap_err().code, "MISSING_FIELD");
}

#[test]
fn frame_data_string_seq_rejected() {
    let msg = r#"{"type":"frame_data","seq":"3","codec":"Rgba","sender_id":"peer-1"}"#;
    assert_eq!(validate_message(msg).unwrap_err().code, "WRONG_TYPE");
}

#[test]
fn frame_data_float_seq_rejected() {
    let msg = r#"{"type":"frame_data","seq":1.5,"codec":"Rgba","sender_id":"peer-1"}"#;
    assert_eq!(validate_message(msg).unwrap_err().code, "WRONG_TYPE");
}

#[test]
fn frame_data_negative_seq_rejected() {
    let msg = r#"{"type":"frame_data","seq":-1,"codec":"Rgba","sender_id":"peer-1"}"#;
    assert_eq!(validate_message(msg).unwrap_err().code, "WRONG_TYPE");
}

#[test]
fn frame_data_non_boolean_keyframe_rejected() {
    let msg = r#"{"type":"frame_data","seq":0,"is_keyframe":"yes","sender_id":"peer-1"}"#;
    assert_eq!(validate_message(msg).unwrap_err().code, "WRONG_TYPE");
}

#[test]
fn frame_data_payload_len_over_u32_max_rejected() {
    // MAX_FRAME_SIZE is 16 MiB, far below u32::MAX, but the field must
    // still be rejected if it does not fit a u32 at all.
    let msg = r#"{"type":"frame_data","seq":0,"payload_len":4294967296,"sender_id":"peer-1"}"#;
    assert_eq!(validate_message(msg).unwrap_err().code, "WRONG_TYPE");
}

#[test]
fn valid_frame_ack() {
    let msg = r#"{"type":"frame_ack","session_id":1,"seq":3,"rtt_us":4200,"recv_pts_us":50001,"sender_id":"peer-1"}"#;
    assert!(validate_message(msg).is_ok());
}

#[test]
fn frame_ack_missing_sender_id_rejected() {
    let msg = r#"{"type":"frame_ack","session_id":1,"seq":3,"rtt_us":4200}"#;
    let err = validate_message(msg).unwrap_err();
    assert_eq!(err.code, "MISSING_FIELD");
    assert!(err.message.contains("sender_id"));
}

#[test]
fn frame_ack_missing_seq_rejected() {
    let msg = r#"{"type":"frame_ack","session_id":1,"rtt_us":4200,"sender_id":"peer-1"}"#;
    assert_eq!(validate_message(msg).unwrap_err().code, "MISSING_FIELD");
}

#[test]
fn valid_session_transfer() {
    let msg = r#"{"type":"session_transfer","session_id":3,"input_owner":"new-in","output_owner":"new-out","focus_owner":"new-focus"}"#;
    assert!(validate_message(msg).is_ok());
}

#[test]
fn valid_session_transfer_minimal() {
    // `focus_owner` is optional; the daemon defaults it to `input_owner`.
    let msg = r#"{"type":"session_transfer","session_id":3,"input_owner":"new-in","output_owner":"new-out"}"#;
    assert!(validate_message(msg).is_ok());
}

#[test]
fn session_transfer_missing_input_owner_rejected() {
    let msg = r#"{"type":"session_transfer","session_id":3,"output_owner":"new-out"}"#;
    let err = validate_message(msg).unwrap_err();
    assert_eq!(err.code, "MISSING_FIELD");
    assert!(err.message.contains("input_owner"));
}

#[test]
fn session_transfer_missing_output_owner_rejected() {
    let msg = r#"{"type":"session_transfer","session_id":3,"input_owner":"new-in"}"#;
    let err = validate_message(msg).unwrap_err();
    assert_eq!(err.code, "MISSING_FIELD");
    assert!(err.message.contains("output_owner"));
}

#[test]
fn session_transfer_focus_owner_must_be_string() {
    let msg = r#"{"type":"session_transfer","session_id":3,"input_owner":"in","output_owner":"out","focus_owner":7}"#;
    assert_eq!(validate_message(msg).unwrap_err().code, "WRONG_TYPE");
}

#[test]
fn valid_session_close() {
    assert!(validate_message(r#"{"type":"session_close","session_id":3}"#).is_ok());
}

#[test]
fn valid_session_cancel() {
    assert!(validate_message(r#"{"type":"session_cancel","session_id":3}"#).is_ok());
}

#[test]
fn valid_session_resume() {
    assert!(validate_message(r#"{"type":"session_resume","session_id":3}"#).is_ok());
}

#[test]
fn session_resume_accepts_pascal_case() {
    assert!(validate_message(r#"{"type":"SessionResume","session_id":1}"#).is_ok());
}

#[test]
fn session_resume_missing_session_id_rejected() {
    let err = validate_message(r#"{"type":"session_resume"}"#)
        .expect_err("a resume must name its session");
    assert_eq!(err.code, "MISSING_FIELD");
    assert!(err.message.contains("session_id"));
}

#[test]
fn session_resume_null_session_id_rejected() {
    let err = validate_message(r#"{"type":"session_resume","session_id":null}"#)
        .expect_err("null rejected");
    assert_eq!(err.code, "NULL_FIELD");
}

/// A string, float, negative, or boolean `session_id` must be `WRONG_TYPE`,
/// exactly as for the teardown messages. Without this the handler's `as_u64`
/// would read `"3"` as absent and report the misleading `missing_session_id`
/// instead of naming the real problem.
#[test]
fn session_resume_non_integer_session_id_rejected() {
    for bad in [r#""3""#, "1.5", "-1", "true"] {
        let line = format!(r#"{{"type":"session_resume","session_id":{bad}}}"#);
        let err = validate_message(&line).expect_err("must be an unsigned integer");
        assert_eq!(err.code, "WRONG_TYPE", "for {line}: {err}");
    }
}

#[test]
fn session_resume_ack_is_not_a_valid_request() {
    // The resume response is a server->client type; accepting it as a request
    // would let a client confuse its own ack for a command.
    let msg = r#"{"type":"session_resume_ack","session_id":1}"#;
    assert_eq!(
        validate_message(msg).unwrap_err().code,
        "UNKNOWN_TYPE",
        "session_resume_ack is a response type"
    );
}

#[test]
fn teardown_missing_session_id_rejected() {
    for line in [
        r#"{"type":"session_close"}"#,
        r#"{"type":"session_cancel"}"#,
    ] {
        let err = validate_message(line).expect_err("must name its session");
        assert_eq!(err.code, "MISSING_FIELD", "for {line}");
    }
}

#[test]
fn teardown_null_session_id_rejected() {
    let err = validate_message(r#"{"type":"session_close","session_id":null}"#)
        .expect_err("null rejected");
    assert_eq!(err.code, "NULL_FIELD");
}

// A string or float session_id must be WRONG_TYPE, not silently read as
// "absent" by the handler's `as_u64`, which would report the misleading
// `missing_session_id` instead.
#[test]
fn teardown_non_integer_session_id_rejected() {
    for bad in [r#""3""#, "1.5", "-1", "true"] {
        let line = format!(r#"{{"type":"session_close","session_id":{bad}}}"#);
        let err = validate_message(&line).expect_err("must be an unsigned integer");
        assert_eq!(err.code, "WRONG_TYPE", "for {line}: {err}");
    }
}

#[test]
fn teardown_types_accept_pascal_case() {
    assert!(validate_message(r#"{"type":"SessionClose","session_id":1}"#).is_ok());
    assert!(validate_message(r#"{"type":"SessionCancel","session_id":1}"#).is_ok());
}

#[test]
fn frame_types_accept_pascal_case() {
    assert!(validate_message(&INIT.replace("session_init", "SessionInit")).is_ok());
    assert!(validate_message(r#"{"type":"FrameData","seq":0,"sender_id":"x"}"#).is_ok());
    assert!(validate_message(r#"{"type":"FrameAck","seq":0,"sender_id":"x"}"#).is_ok());
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
