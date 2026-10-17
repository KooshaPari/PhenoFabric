//! Unit tests for the frame-plane wire handlers.
//!
//! Split out of `frame_handlers.rs` so both files stay well under the 500-line
//! ceiling. Behavior under test is unchanged; only the file layout differs.

use super::*;
use fabric_frame_transport::PROTOCOL_VERSION;
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
    // Keep the temp dir alive for the duration of the coordinator.
    let coord = Coordinator::new(config).unwrap();
    std::mem::forget(dir);
    Arc::new(coord)
}

fn init_value(client: &str) -> Value {
    serde_json::json!({
        "type": "session_init",
        "version": PROTOCOL_VERSION,
        "preferred_codec": "Rgba",
        "width": 64,
        "height": 48,
        "target_fps": 60,
        "max_latency_ms": 33,
        "client_id": client,
    })
}

fn frame_value(session_id: u64, seq: u64, payload_len: u32) -> Value {
    serde_json::json!({
        "type": "frame_data",
        "session_id": session_id,
        "seq": seq,
        "pts_us": seq * 16_667,
        "dts_us": seq * 16_667,
        "is_keyframe": seq == 0,
        "codec": "Rgba",
        "width": 64,
        "height": 48,
        "payload_len": payload_len,
        "duration_us": 16_667,
    })
}

/// Parse a handler response as a map, asserting it is valid JSON.
fn parse(resp: &str) -> Value {
    serde_json::from_str(resp).unwrap_or_else(|e| panic!("response not JSON: {e}: {resp}"))
}

#[test]
fn session_init_returns_session_ack() {
    let coord = make_coordinator();
    let resp = handle_session_init(&init_value("client-a"), &coord);
    let v = parse(&resp);
    assert_eq!(v["type"], "session_ack");
    assert_eq!(v["session_id"], 1);
    assert_eq!(v["codec"], "Rgba");
    assert_eq!(v["width"], 64);
    assert_eq!(v["height"], 48);
    assert_eq!(v["fps"], 60);
    assert!(v["server_time_ms"].is_number());
}

#[test]
fn session_ack_matches_transport_session_ack_shape() {
    // The response must deserialize into the real transport type.
    let coord = make_coordinator();
    let resp = handle_session_init(&init_value("client-a"), &coord);
    let v = parse(&resp);
    let ack: SessionAck = serde_json::from_value(v).expect("must match SessionAck");
    assert_eq!(ack.session_id, 1);
    assert_eq!(ack.codec, Codec::Rgba);
}

#[test]
fn session_init_zero_dimensions_rejected() {
    let coord = make_coordinator();
    let mut v = init_value("client-a");
    v["width"] = serde_json::json!(0);
    let resp = handle_session_init(&v, &coord);
    let parsed = parse(&resp);
    assert_eq!(parsed["type"], "frame_error");
    assert_eq!(parsed["error"], "invalid_session_init");
}

#[test]
fn session_init_zero_fps_rejected() {
    let coord = make_coordinator();
    let mut v = init_value("client-a");
    v["target_fps"] = serde_json::json!(0);
    let resp = handle_session_init(&v, &coord);
    assert_eq!(parse(&resp)["error"], "invalid_session_init");
}

#[test]
fn session_init_missing_client_id_rejected() {
    let coord = make_coordinator();
    let mut v = init_value("client-a");
    v.as_object_mut().unwrap().remove("client_id");
    let resp = handle_session_init(&v, &coord);
    assert_eq!(parse(&resp)["error"], "invalid_session_init");
}

#[test]
fn frame_data_for_unknown_session_is_an_error() {
    let coord = make_coordinator();
    let resp = handle_frame_data(&frame_value(42, 0, 1024), &coord);
    let v = parse(&resp);
    assert_eq!(v["type"], "frame_error");
    assert_eq!(v["error"], "unknown_session");
    assert_eq!(v["session_id"], 42);
}

#[test]
fn unknown_session_error_names_established_sessions() {
    let coord = make_coordinator();
    handle_session_init(&init_value("client-a"), &coord);
    let resp = handle_frame_data(&frame_value(99, 0, 1024), &coord);
    let v = parse(&resp);
    let message = v["message"].as_str().unwrap();
    assert!(
        message.contains('1'),
        "error should name session 1, got: {message}"
    );
}

#[test]
fn unknown_session_error_distinguishes_no_sessions_from_wrong_id() {
    // "no sessions at all" and "wrong id" are different client bugs, so
    // the diagnostic must read differently in the two cases.
    let coord = make_coordinator();
    let empty = parse(&handle_frame_data(&frame_value(5, 0, 8), &coord));
    assert!(
        empty["message"].as_str().unwrap().contains("none"),
        "with no sessions the error should say none: {empty}"
    );
    assert_eq!(empty["active_sessions"].as_array().unwrap().len(), 0);

    handle_session_init(&init_value("client-a"), &coord);
    let wrong_id = parse(&handle_frame_data(&frame_value(5, 0, 8), &coord));
    assert!(
        !wrong_id["message"].as_str().unwrap().contains("none"),
        "with a live session the error should name it, not say none: {wrong_id}"
    );
    let active = wrong_id["active_sessions"].as_array().unwrap();
    assert_eq!(active.len(), 1, "one session is live: {wrong_id}");
}

#[test]
fn frame_data_roundtrip_accumulates_counters() {
    let coord = make_coordinator();
    handle_session_init(&init_value("client-a"), &coord);

    let first = parse(&handle_frame_data(&frame_value(1, 0, 100), &coord));
    assert_eq!(first["type"], "frame_ack");
    assert_eq!(first["status"], "ok");
    assert_eq!(first["seq"], 0);
    assert_eq!(first["frame_count"], 1);
    assert_eq!(first["frames_received"], 100);
    assert_eq!(first["last_seq"], 0);

    let second = parse(&handle_frame_data(&frame_value(1, 1, 250), &coord));
    assert_eq!(second["frame_count"], 2);
    assert_eq!(second["frames_received"], 350);
    assert_eq!(second["last_seq"], 1);
}

#[test]
fn frame_data_without_session_id_is_an_error() {
    let coord = make_coordinator();
    handle_session_init(&init_value("client-a"), &coord);
    let mut v = frame_value(1, 0, 10);
    v.as_object_mut().unwrap().remove("session_id");
    let resp = handle_frame_data(&v, &coord);
    assert_eq!(parse(&resp)["error"], "missing_session_id");
}

#[test]
fn frame_data_without_codec_is_an_error() {
    let coord = make_coordinator();
    handle_session_init(&init_value("client-a"), &coord);
    let mut v = frame_value(1, 0, 10);
    v.as_object_mut().unwrap().remove("codec");
    let resp = handle_frame_data(&v, &coord);
    assert_eq!(parse(&resp)["error"], "missing_codec");
}

#[test]
fn frame_data_with_unknown_codec_is_an_error() {
    let coord = make_coordinator();
    handle_session_init(&init_value("client-a"), &coord);
    let mut v = frame_value(1, 0, 10);
    v["codec"] = serde_json::json!("Vp9");
    let resp = handle_frame_data(&v, &coord);
    let parsed = parse(&resp);
    assert_eq!(parsed["error"], "invalid_codec");
    assert!(parsed["message"].as_str().unwrap().contains("Vp9"));
}

#[test]
fn frame_data_oversize_payload_rejected() {
    let coord = make_coordinator();
    handle_session_init(&init_value("client-a"), &coord);
    let too_big = fabric_frame_transport::MAX_FRAME_SIZE as u32 + 1;
    let resp = handle_frame_data(&frame_value(1, 0, too_big), &coord);
    assert_eq!(parse(&resp)["error"], "frame_too_large");
}

#[test]
fn frame_ack_for_unknown_session_is_an_error() {
    let coord = make_coordinator();
    let v = serde_json::json!({"type":"frame_ack","session_id":7,"seq":0,"rtt_us":100});
    let resp = handle_frame_ack(&v, &coord);
    let parsed = parse(&resp);
    assert_eq!(parsed["error"], "unknown_session");
}

#[test]
fn frame_ack_records_rtt() {
    let coord = make_coordinator();
    handle_session_init(&init_value("client-a"), &coord);
    handle_frame_data(&frame_value(1, 3, 64), &coord);

    let ack = serde_json::json!({
        "type": "frame_ack",
        "session_id": 1,
        "seq": 3,
        "rtt_us": 1234,
        "recv_pts_us": 3 * 16_667,
    });
    let resp = handle_frame_ack(&ack, &coord);
    let v = parse(&resp);
    assert_eq!(v["type"], "frame_ack_response");
    assert_eq!(v["acked"], true);
    assert_eq!(v["rtt_us"], 1234);
    assert_eq!(v["recorded_rtt_us"], 1234);
    assert_eq!(v["ack_count"], 1);
    assert_eq!(v["frame_count"], 1);
    assert_eq!(v["last_seq"], 3);
}

#[test]
fn frame_ack_before_any_frame_is_not_acked() {
    // `acked` must mean "the daemon received a frame at or past this seq",
    // not merely "the ack was recorded". An ack for a frame the daemon
    // never saw must report false.
    let coord = make_coordinator();
    handle_session_init(&init_value("client-a"), &coord);

    let ack = serde_json::json!({
        "type": "frame_ack",
        "session_id": 1,
        "seq": 0,
        "rtt_us": 500,
        "recv_pts_us": 0,
    });
    let v = parse(&handle_frame_ack(&ack, &coord));
    assert_eq!(v["acked"], false);
    // It is still recorded, so the RTT is not lost.
    assert_eq!(v["recorded_rtt_us"], 500);
    assert_eq!(v["frame_count"], 0);
    assert!(v["last_seq"].is_null(), "no frame was received");
}

#[test]
fn frame_ack_for_seq_beyond_last_frame_is_not_acked() {
    let coord = make_coordinator();
    handle_session_init(&init_value("client-a"), &coord);
    handle_frame_data(&frame_value(1, 2, 64), &coord);

    let ack = serde_json::json!({"type":"frame_ack","session_id":1,"seq":9,"rtt_us":1});
    let v = parse(&handle_frame_ack(&ack, &coord));
    assert_eq!(v["acked"], false, "seq 9 was never received");

    let in_range = serde_json::json!({"type":"frame_ack","session_id":1,"seq":1,"rtt_us":2});
    let v2 = parse(&handle_frame_ack(&in_range, &coord));
    assert_eq!(v2["acked"], true, "seq 1 is at or below last_seq 2");
}

#[test]
fn frame_ack_without_session_id_is_an_error() {
    let coord = make_coordinator();
    handle_session_init(&init_value("client-a"), &coord);
    let v = serde_json::json!({"type":"frame_ack","seq":0,"rtt_us":1});
    let resp = handle_frame_ack(&v, &coord);
    assert_eq!(parse(&resp)["error"], "missing_session_id");
}

#[test]
fn sessions_are_numbered_incrementally() {
    let coord = make_coordinator();
    let a = parse(&handle_session_init(&init_value("a"), &coord));
    let b = parse(&handle_session_init(&init_value("b"), &coord));
    assert_eq!(a["session_id"], 1);
    assert_eq!(b["session_id"], 2);
}

#[test]
fn session_state_tracks_two_independent_sessions() {
    let coord = make_coordinator();
    handle_session_init(&init_value("a"), &coord);
    handle_session_init(&init_value("b"), &coord);

    handle_frame_data(&frame_value(1, 0, 10), &coord);
    handle_frame_data(&frame_value(1, 1, 10), &coord);
    handle_frame_data(&frame_value(2, 0, 99), &coord);

    let state = coord.frame_sessions_mut();
    let s1 = state.get(1).unwrap();
    let s2 = state.get(2).unwrap();
    assert_eq!(s1.frame_count, 2);
    assert_eq!(s1.frames_received, 20);
    assert_eq!(s1.client_id, "a");
    assert_eq!(s2.frame_count, 1);
    assert_eq!(s2.frames_received, 99);
    assert_eq!(s2.client_id, "b");
}

#[test]
fn codec_parsing_accepts_transport_names() {
    for (name, expected) in [
        ("Hevc", Codec::Hevc),
        ("Av1", Codec::Av1),
        ("Nv12", Codec::Nv12),
        ("Rgba", Codec::Rgba),
    ] {
        let v = Value::String(name.to_string());
        assert_eq!(parse_codec(&v).unwrap(), expected, "{name} should parse");
    }
}

#[test]
fn frame_ack_response_matches_transport_frame_ack_fields() {
    // The response carries the real transport FrameAck fields under their
    // transport names, so a client reading them needs no translation.
    let coord = make_coordinator();
    handle_session_init(&init_value("client-a"), &coord);
    handle_frame_data(&frame_value(1, 5, 32), &coord);

    let ack = serde_json::json!({
        "type": "frame_ack",
        "session_id": 1,
        "seq": 5,
        "rtt_us": 999,
        "recv_pts_us": 5 * 16_667,
    });
    let v = parse(&handle_frame_ack(&ack, &coord));
    let expected = FrameAck {
        seq: 5,
        rtt_us: 999,
        recv_pts_us: 5 * 16_667,
    };
    assert_eq!(v["seq"], expected.seq);
    assert_eq!(v["rtt_us"], expected.rtt_us);
    assert_eq!(v["recv_pts_us"], expected.recv_pts_us);
}

#[test]
fn protocol_version_constant_is_preserved() {
    // The transport version this daemon reports on its session_ack.
    assert_eq!(PROTOCOL_VERSION, 1);
}
