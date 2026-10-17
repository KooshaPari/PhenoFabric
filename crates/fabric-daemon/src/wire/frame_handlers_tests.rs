//! Unit tests for the frame-plane wire handlers.
//!
//! Split out of `frame_handlers.rs` so both files stay well under the 500-line
//! ceiling. Behavior under test is unchanged; only the file layout differs.

use super::super::frame_session_state::MAX_CONCURRENT_SESSIONS;
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

/// Rewind a session's recorded activity far enough that the next sweep takes it.
///
/// The session's `last_activity_ms` is seeded from the daemon's real wall
/// clock, so waiting out the 120 s production timeout would make this test
/// take two minutes and still prove less than moving the recorded time
/// directly: the sweep's decision depends only on `now_ms -
/// last_activity_ms`, which is what this rewinds.
fn age_session_past_idle_timeout(coord: &Arc<Coordinator>, session_id: u64) {
    let mut state = coord.frame_sessions_mut();
    let session = state
        .get_mut_for_test(session_id)
        .unwrap_or_else(|| panic!("session {session_id} should exist"));
    // Rewind by twice the timeout so the subtraction is unambiguously past the
    // deadline regardless of how much wall time the test itself consumed.
    session.last_activity_ms = session.last_activity_ms.saturating_sub(IDLE_TIMEOUT_MS * 2);
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

// ---------------------------------------------------------------------------
// Session teardown: close
// ---------------------------------------------------------------------------

fn close_value(session_id: u64) -> Value {
    serde_json::json!({"type": "session_close", "session_id": session_id})
}

fn cancel_value(session_id: u64) -> Value {
    serde_json::json!({"type": "session_cancel", "session_id": session_id})
}

/// Build a real `SessionInit` from the JSON test fixture, for driving the
/// state layer directly.
fn init_value_to_init(client: &str) -> SessionInit {
    serde_json::from_value(init_value(client)).expect("test init must deserialize")
}

/// A close removes the session from the coordinator's table.
#[test]
fn session_close_removes_the_session() {
    let coord = make_coordinator();
    handle_session_init(&init_value("client-a"), &coord);

    let v = parse(&handle_session_close(&close_value(1), &coord));
    assert_eq!(v["type"], "session_close_ack");
    assert_eq!(v["status"], "ok");
    assert_eq!(v["session_id"], 1);
    assert_eq!(v["reason"], "closed");
    assert_eq!(v["client_id"], "client-a");

    assert_eq!(
        coord.frame_sessions_mut().len(),
        0,
        "close must remove the session from the map"
    );
}

/// The close response reports the counters the session had accumulated, so
/// the client's final accounting is not lost at teardown.
#[test]
fn session_close_reports_final_counters() {
    let coord = make_coordinator();
    handle_session_init(&init_value("client-a"), &coord);
    handle_frame_data(&frame_value(1, 0, 100), &coord);
    handle_frame_data(&frame_value(1, 1, 250), &coord);

    let v = parse(&handle_session_close(&close_value(1), &coord));
    assert_eq!(v["frame_count"], 2);
    assert_eq!(v["frames_received"], 350);
    assert_eq!(v["last_seq"], 1);
}

/// A frame after a close fails cleanly, naming the teardown as the reason.
/// It must not be silently accepted and must not panic.
#[test]
fn frame_data_after_close_fails_cleanly() {
    let coord = make_coordinator();
    handle_session_init(&init_value("client-a"), &coord);
    handle_session_close(&close_value(1), &coord);

    let v = parse(&handle_frame_data(&frame_value(1, 0, 1024), &coord));
    assert_eq!(v["type"], "frame_error");
    assert_eq!(v["error"], "session_gone");
    assert_eq!(v["reason"], "closed");
    assert_eq!(v["session_id"], 1);
    let message = v["message"].as_str().unwrap();
    assert!(
        message.contains("closed") && message.contains("session_init"),
        "the error should say what happened and what to do: {message}"
    );
}

/// A frame ack after a close fails the same way.
#[test]
fn frame_ack_after_close_fails_cleanly() {
    let coord = make_coordinator();
    handle_session_init(&init_value("client-a"), &coord);
    handle_session_close(&close_value(1), &coord);

    let ack = serde_json::json!({"type":"frame_ack","session_id":1,"seq":0,"rtt_us":1});
    let v = parse(&handle_frame_ack(&ack, &coord));
    assert_eq!(v["error"], "session_gone");
    assert_eq!(v["reason"], "closed");
}

/// Closing twice is reported, not silently accepted as a second success.
#[test]
fn session_close_twice_reports_the_miss() {
    let coord = make_coordinator();
    handle_session_init(&init_value("client-a"), &coord);
    let first = parse(&handle_session_close(&close_value(1), &coord));
    assert_eq!(first["status"], "ok");

    let second = parse(&handle_session_close(&close_value(1), &coord));
    assert_eq!(second["type"], "frame_error");
    assert_eq!(second["error"], "session_gone");
    assert_eq!(second["reason"], "closed");
}

/// Closing an id this daemon never issued is `unknown_session`, distinct from
/// a session the daemon tore down.
#[test]
fn session_close_for_unknown_session_is_unknown_session() {
    let coord = make_coordinator();
    let v = parse(&handle_session_close(&close_value(4242), &coord));
    assert_eq!(v["type"], "frame_error");
    assert_eq!(v["error"], "unknown_session");
    assert_eq!(v["session_id"], 4242);
}

/// A close without a session id cannot identify its target.
#[test]
fn session_close_without_session_id_is_an_error() {
    let coord = make_coordinator();
    let v = parse(&handle_session_close(
        &serde_json::json!({"type": "session_close"}),
        &coord,
    ));
    assert_eq!(v["type"], "frame_error");
    assert_eq!(v["error"], "missing_session_id");
}

/// Closing one session leaves the others live: sessions are independent.
#[test]
fn session_close_leaves_other_sessions_live() {
    let coord = make_coordinator();
    handle_session_init(&init_value("a"), &coord);
    handle_session_init(&init_value("b"), &coord);

    handle_session_close(&close_value(1), &coord);

    assert_eq!(coord.frame_sessions_mut().len(), 1);
    let live = parse(&handle_frame_data(&frame_value(2, 0, 64), &coord));
    assert_eq!(
        live["type"], "frame_ack",
        "session 2 must still work: {live}"
    );
}

// ---------------------------------------------------------------------------
// Session teardown: cancel
// ---------------------------------------------------------------------------

/// Cancelling mid-transfer removes the session, so it is not left active.
#[test]
fn session_cancel_removes_the_session() {
    let coord = make_coordinator();
    handle_session_init(&init_value("client-a"), &coord);
    // Mid-transfer: some frames already landed.
    handle_frame_data(&frame_value(1, 0, 100), &coord);
    handle_frame_data(&frame_value(1, 1, 100), &coord);

    let v = parse(&handle_session_cancel(&cancel_value(1), &coord));
    assert_eq!(v["type"], "session_close_ack");
    assert_eq!(v["status"], "ok");
    assert_eq!(v["reason"], "cancelled");
    assert_eq!(v["session_id"], 1);
    assert_eq!(v["frame_count"], 2, "final counters are reported");

    assert_eq!(
        coord.frame_sessions_mut().len(),
        0,
        "cancel must not leave the session active"
    );
}

/// Cancellation releases the session's partial state.
#[test]
fn session_cancel_releases_partial_state() {
    let coord = make_coordinator();
    handle_session_init(&init_value("client-a"), &coord);
    handle_frame_data(&frame_value(1, 0, 100), &coord);
    let ack = serde_json::json!({"type":"frame_ack","session_id":1,"seq":0,"rtt_us":900});
    handle_frame_ack(&ack, &coord);

    {
        let state = coord.frame_sessions_mut();
        let session = state.get(1).expect("session 1 exists");
        assert_eq!(session.acks.len(), 1, "recorded RTT exists before cancel");
        assert_eq!(session.frame_count, 1);
    }

    parse(&handle_session_cancel(&cancel_value(1), &coord));

    // The whole entry, including the recorded RTT, is gone.
    let state = coord.frame_sessions_mut();
    assert!(state.get(1).is_none(), "no active session remains");
    assert!(state.is_empty(), "the slot is released");
}

/// A frame after a cancel fails cleanly.
#[test]
fn frame_data_after_cancel_fails_cleanly() {
    let coord = make_coordinator();
    handle_session_init(&init_value("client-a"), &coord);
    handle_frame_data(&frame_value(1, 0, 100), &coord);
    handle_session_cancel(&cancel_value(1), &coord);

    let v = parse(&handle_frame_data(&frame_value(1, 1, 1024), &coord));
    assert_eq!(v["type"], "frame_error");
    assert_eq!(v["error"], "session_gone");
    assert_eq!(v["reason"], "cancelled");
}

/// Cancelling twice is reported.
#[test]
fn session_cancel_twice_reports_the_miss() {
    let coord = make_coordinator();
    handle_session_init(&init_value("client-a"), &coord);
    assert_eq!(
        parse(&handle_session_cancel(&cancel_value(1), &coord))["status"],
        "ok"
    );

    let second = parse(&handle_session_cancel(&cancel_value(1), &coord));
    assert_eq!(second["error"], "session_gone");
    assert_eq!(second["reason"], "cancelled");
}

/// A cancel without a session id cannot identify its target.
#[test]
fn session_cancel_without_session_id_is_an_error() {
    let coord = make_coordinator();
    let v = parse(&handle_session_cancel(
        &serde_json::json!({"type": "session_cancel"}),
        &coord,
    ));
    assert_eq!(v["error"], "missing_session_id");
}

// ---------------------------------------------------------------------------
// Idle expiry
// ---------------------------------------------------------------------------

/// Drive a session's idle clock into the past, then sweep.
///
/// The clock is moved rather than slept on: the daemon's real clock cannot be
/// wound back from outside, and a test that waited 120 s to prove an expiry
/// would prove nothing a direct clock adjustment does not.
#[test]
fn sweep_removes_an_idle_session() {
    let coord = make_coordinator();
    handle_session_init(&init_value("client-a"), &coord);
    assert_eq!(coord.frame_sessions_mut().len(), 1);

    // Nothing is idle yet: a fresh session survives a sweep.
    assert!(
        sweep_expired_frame_sessions(&coord).is_empty(),
        "a fresh session must not be swept"
    );
    assert_eq!(coord.frame_sessions_mut().len(), 1);

    // Age it past the timeout. `last_activity_ms` is seeded from the daemon
    // clock, so rewinding it by more than the timeout is equivalent to
    // waiting.
    age_session_past_idle_timeout(&coord, 1);

    let expired = sweep_expired_frame_sessions(&coord);
    assert_eq!(expired, vec![1]);
    assert_eq!(coord.frame_sessions_mut().len(), 0);
}

/// A frame after an expiry fails cleanly and says the session expired, so a
/// client can tell a daemon-side reap from its own mistake.
#[test]
fn frame_data_after_expiry_fails_cleanly() {
    let coord = make_coordinator();
    handle_session_init(&init_value("client-a"), &coord);
    handle_frame_data(&frame_value(1, 0, 100), &coord);
    age_session_past_idle_timeout(&coord, 1);
    assert_eq!(sweep_expired_frame_sessions(&coord), vec![1]);

    let v = parse(&handle_frame_data(&frame_value(1, 1, 1024), &coord));
    assert_eq!(v["type"], "frame_error");
    assert_eq!(v["error"], "session_gone");
    assert_eq!(v["reason"], "expired");
    assert_eq!(v["session_id"], 1);
}

/// A frame ack after an expiry fails the same way.
#[test]
fn frame_ack_after_expiry_fails_cleanly() {
    let coord = make_coordinator();
    handle_session_init(&init_value("client-a"), &coord);
    age_session_past_idle_timeout(&coord, 1);
    sweep_expired_frame_sessions(&coord);

    let ack = serde_json::json!({"type":"frame_ack","session_id":1,"seq":0,"rtt_us":1});
    assert_eq!(parse(&handle_frame_ack(&ack, &coord))["reason"], "expired");
}

/// Only idle sessions are swept; an active one survives.
#[test]
fn sweep_keeps_sessions_that_are_still_active() {
    let coord = make_coordinator();
    handle_session_init(&init_value("active"), &coord);
    handle_session_init(&init_value("idle"), &coord);

    // Session 2 is aged; session 1 keeps streaming, refreshing its clock.
    age_session_past_idle_timeout(&coord, 2);
    for _ in 0..3 {
        handle_frame_data(&frame_value(1, 0, 64), &coord);
    }

    let expired = sweep_expired_frame_sessions(&coord);

    assert_eq!(
        expired,
        vec![2],
        "only the idle session should go: {expired:?}"
    );
    assert!(coord.frame_sessions_mut().contains(1));
}

/// After a session is swept, its id is not reused, so a late frame cannot land
/// on a different session that happens to get the same number.
#[test]
fn expired_session_id_is_not_reused() {
    let coord = make_coordinator();
    handle_session_init(&init_value("a"), &coord);
    age_session_past_idle_timeout(&coord, 1);
    sweep_expired_frame_sessions(&coord);

    let next = parse(&handle_session_init(&init_value("b"), &coord));
    assert_eq!(next["session_id"], 2, "the expired id must not come back");
}

// ---------------------------------------------------------------------------
// Session cap
// ---------------------------------------------------------------------------

/// Fill the session table to the cap.
fn fill_to_session_cap(coord: &Arc<Coordinator>) {
    // Fill through the state layer, not the handler. `insert_session` is the
    // raw allocation path and does not apply the handler's capacity check, so
    // this lands on exactly the cap rather than one short of it. The cap's
    // enforcement is asserted separately, below, against the handler.
    let mut state = coord.frame_sessions_mut();
    for _ in 0..MAX_CONCURRENT_SESSIONS {
        state.insert_session(&init_value_to_init("filler"));
    }
    assert_eq!(state.len(), MAX_CONCURRENT_SESSIONS);
}

/// The (N+1)th session is rejected with a clear, distinct error.
#[test]
fn session_cap_rejects_the_session_after_the_limit() {
    let coord = make_coordinator();
    fill_to_session_cap(&coord);

    let v = parse(&handle_session_init(&init_value("one-too-many"), &coord));

    assert_eq!(v["type"], "frame_error");
    assert_eq!(v["error"], "session_limit_reached");
    let message = v["message"].as_str().unwrap();
    assert!(
        message.contains(&MAX_CONCURRENT_SESSIONS.to_string()) && message.contains("close"),
        "the error should state the cap and the way out: {message}"
    );
}

/// A rejected session is not inserted: the cap bounds the table rather than
/// admitting a session the daemon cannot serve.
#[test]
fn session_cap_does_not_insert_the_rejected_session() {
    let coord = make_coordinator();
    fill_to_session_cap(&coord);

    handle_session_init(&init_value("one-too-many"), &coord);

    assert_eq!(coord.frame_sessions_mut().len(), MAX_CONCURRENT_SESSIONS);
}

/// Teardown makes room under the cap, so a long-lived client is not locked out
/// once it has cycled through sessions.
#[test]
fn session_cap_room_is_reclaimed_by_teardown() {
    let coord = make_coordinator();
    fill_to_session_cap(&coord);
    assert_eq!(
        parse(&handle_session_init(&init_value("blocked"), &coord))["error"],
        "session_limit_reached"
    );

    // Closing one session frees exactly one slot.
    handle_session_close(&close_value(1), &coord);

    let v = parse(&handle_session_init(&init_value("after-close"), &coord));
    assert_eq!(v["type"], "session_ack", "a freed slot must be usable: {v}");
    assert_eq!(coord.frame_sessions_mut().len(), MAX_CONCURRENT_SESSIONS);
}

/// Cancelling also frees a slot, and the expiry sweep is the third path that
/// reclaims one.
#[test]
fn session_cap_room_is_reclaimed_by_cancel_and_expiry() {
    let coord = make_coordinator();

    // --- Freed by cancel ---
    fill_to_session_cap(&coord);
    handle_session_cancel(&cancel_value(1), &coord);
    assert_eq!(
        parse(&handle_session_init(&init_value("after-cancel"), &coord))["type"],
        "session_ack"
    );

    // --- Freed by the expiry sweep ---
    // The `session_init` above already took the slot the cancel freed, so the
    // table is back at exactly the cap. No top-up: one more would overshoot.
    assert_eq!(coord.frame_sessions_mut().len(), MAX_CONCURRENT_SESSIONS);

    // Which session the sweep takes is not fixed, so age every session rather
    // than guessing which id the previous steps consumed.
    let ids: Vec<u64> = coord.frame_sessions_mut().known_session_ids();
    for id in &ids {
        age_session_past_idle_timeout(&coord, *id);
    }
    let expired = sweep_expired_frame_sessions(&coord);
    assert_eq!(
        expired.len(),
        MAX_CONCURRENT_SESSIONS,
        "the sweep must take every aged session: {expired:?}"
    );
    assert_eq!(coord.frame_sessions_mut().len(), 0);

    let v = parse(&handle_session_init(&init_value("after-expiry"), &coord));
    assert_eq!(v["type"], "session_ack", "a swept slot must be usable: {v}");
}
