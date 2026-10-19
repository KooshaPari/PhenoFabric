//! Unit tests for the `session_resume` wire handler.
//!
//! These drive the handler directly against a real `Coordinator`, the same way
//! `frame_handlers_tests.rs` does for the frame messages, so what is asserted
//! here is the exact response line a client would read off the socket.

use super::super::frame_session_state::{SessionState, DEFAULT_IDLE_TIMEOUT_MS};
use super::*;
use fabric_frame_transport::PROTOCOL_VERSION;
use std::sync::Arc;

/// The idle timeout the daemon's sweep actually applies.
///
/// `frame_handlers::IDLE_TIMEOUT_MS` is private to that module and this test
/// needs the same bound, so it reads the underlying public constant rather than
/// duplicating the number or widening visibility for a test.
const IDLE_TIMEOUT_MS: u64 = DEFAULT_IDLE_TIMEOUT_MS;

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
    // The session's input owner is the opener's `client_id`, so the default
    // sender here is the same id that the tests pass to `init_value`.
    // Tests that need to drive a different peer should construct a frame
    // by hand.
    frame_value_with_sender(
        session_id,
        seq,
        payload_len,
        "client-a",
        payload_b64_for(payload_len),
    )
}

fn frame_value_with_sender(
    session_id: u64,
    seq: u64,
    payload_len: u32,
    sender_id: &str,
    payload_b64: &str,
) -> Value {
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
        "sender_id": sender_id,
        "payload_b64": payload_b64,
    })
}

/// Base64-encode `payload_len` zero bytes. See the same-name helper in
/// `frame_handlers_tests.rs` for the rationale. Duplicated here so the
/// resume tests can drive the wire without a `base64` dependency.
fn payload_b64_for(payload_len: u32) -> &'static str {
    match payload_len {
        0 => "",
        12288 => rgba_64x48_zero_b64(),
        _ => {
            let n = payload_len as usize;
            let full = n / 3;
            let rem = n % 3;
            let mut s = String::with_capacity((full + (rem != 0) as usize) * 4);
            for _ in 0..full {
                s.push_str("AAAA");
            }
            match rem {
                0 => {}
                1 => s.push_str("AA=="),
                2 => s.push_str("AAA="),
                _ => unreachable!(),
            }
            Box::leak(s.into_boxed_str())
        }
    }
}

fn rgba_64x48_zero_b64() -> &'static str {
    use std::sync::OnceLock;
    static CACHE: OnceLock<String> = OnceLock::new();
    let s = CACHE.get_or_init(|| "A".repeat(16_384));
    s.as_str()
}

fn resume_value(session_id: u64) -> Value {
    serde_json::json!({"type": "session_resume", "session_id": session_id})
}

fn parse(resp: &str) -> Value {
    serde_json::from_str(resp).unwrap_or_else(|e| panic!("response not JSON: {e}: {resp}"))
}

/// Open a session, stream `frames` frames into it, then mark it disconnected --
/// the full client-side story of "stream, lose the connection".
fn stream_then_disconnect(coord: &Arc<Coordinator>, client: &str, frames: u64) -> u64 {
    let ack = parse(&super::super::frame_handlers::handle_session_init(
        &init_value(client),
        coord,
    ));
    assert_eq!(ack["type"], "session_ack");
    let session_id = ack["session_id"].as_u64().expect("session_id");

    for seq in 0..frames {
        let f = parse(&super::super::frame_handlers::handle_frame_data(
            &frame_value(session_id, seq, 12288),
            coord,
        ));
        assert_eq!(f["status"], "ok", "frame {seq} should land: {f}");
    }

    coord.frame_sessions_mut().mark_disconnected(session_id);
    session_id
}

// ---------------------------------------------------------------------------
// The happy path
// ---------------------------------------------------------------------------

/// A resume of a disconnected session succeeds and reports the session's
/// carried-forward accounting.
#[test]
fn resume_of_a_disconnected_session_succeeds() {
    let coord = make_coordinator();
    let session_id = stream_then_disconnect(&coord, "client-a", 3);

    let v = parse(&handle_session_resume(&resume_value(session_id), &coord));

    assert_eq!(v["type"], "session_resume_ack");
    assert_eq!(v["status"], "ok");
    assert_eq!(v["session_id"], session_id);
    assert_eq!(v["client_id"], "client-a");
    assert_eq!(v["state"], "active");
    assert_eq!(v["was_disconnected"], true);
    assert_eq!(v["frame_count"], 3);
    // 3 * 12,288 = 36,864: the three decoded frame bodies the session
    // accumulated before being marked disconnected, not the 100 * 3 = 300
    // the client would have sent before the wire enforced the codec
    // shape.
    assert_eq!(v["frames_received"], 36864);
    assert_eq!(v["last_seq"], 2);
}

/// The full round trip the task asks for: disconnect, reconnect, resume, and
/// keep sending frames -- with the sequence continuing rather than restarting.
#[test]
fn disconnect_then_reconnect_and_resume_continues_the_sequence() {
    let coord = make_coordinator();
    let session_id = stream_then_disconnect(&coord, "client-a", 3);

    // --- Reconnect: resume ---
    let resumed = parse(&handle_session_resume(&resume_value(session_id), &coord));
    assert_eq!(resumed["status"], "ok");
    assert_eq!(resumed["last_seq"], 2, "the daemon had accepted seq 0..2");

    // --- Keep streaming from where it left off, not from zero ---
    let next = parse(&super::super::frame_handlers::handle_frame_data(
        &frame_value(session_id, 3, 12288),
        &coord,
    ));
    assert_eq!(next["type"], "frame_ack", "got: {next}");
    assert_eq!(next["status"], "ok", "got: {next}");
    assert_eq!(next["seq"], 3);
    assert_eq!(
        next["frame_count"], 4,
        "accounting continued across the resume: {next}"
    );
    assert_eq!(
        next["frames_received"], 49152,
        "byte accounting continued too: {next}"
    );
    assert_eq!(next["last_seq"], 3);
}

/// The session is genuinely re-attached, not merely permitted to keep sending.
#[test]
fn resume_reports_the_session_as_active_again() {
    let coord = make_coordinator();
    let session_id = stream_then_disconnect(&coord, "client-a", 1);
    assert_eq!(
        coord.frame_sessions_mut().get(session_id).unwrap().state,
        SessionState::Disconnected
    );

    handle_session_resume(&resume_value(session_id), &coord);

    assert_eq!(
        coord.frame_sessions_mut().get(session_id).unwrap().state,
        SessionState::Active
    );
    assert!(
        coord
            .frame_sessions_mut()
            .disconnected_session_ids()
            .is_empty(),
        "no session should be left dormant after a successful resume"
    );
}

/// Resuming a session that was never marked dormant still succeeds -- the client
/// may have lost its `session_resume` response and retried. It reports
/// `was_disconnected: false` so both ends can see the difference.
#[test]
fn resume_of_an_undropped_session_is_idempotent() {
    let coord = make_coordinator();
    let session_id = stream_then_disconnect(&coord, "client-a", 1);
    // Put it back to active, as if the drop were never observed.
    coord
        .frame_sessions_mut()
        .resume_session(session_id, 1)
        .expect("session exists");

    let first = parse(&handle_session_resume(&resume_value(session_id), &coord));
    assert_eq!(first["status"], "ok");
    assert_eq!(first["was_disconnected"], false);

    // Retrying is not an error and does not double-count.
    let second = parse(&handle_session_resume(&resume_value(session_id), &coord));
    assert_eq!(second["status"], "ok");
    assert_eq!(second["frame_count"], first["frame_count"]);
}

// ---------------------------------------------------------------------------
// Refusals
// ---------------------------------------------------------------------------

/// Resume after a close is refused, and the message says *why* and what to do.
#[test]
fn resume_after_close_is_rejected() {
    let coord = make_coordinator();
    let session_id = stream_then_disconnect(&coord, "client-a", 2);
    super::super::frame_handlers::handle_session_close(
        &serde_json::json!({"type": "session_close", "session_id": session_id}),
        &coord,
    );

    let v = parse(&handle_session_resume(&resume_value(session_id), &coord));

    assert_eq!(v["type"], "frame_error");
    assert_eq!(v["error"], "session_gone");
    assert_eq!(v["reason"], "closed");
    assert_eq!(v["session_id"], session_id);
    let message = v["message"].as_str().unwrap();
    assert!(
        message.contains("cannot be resumed") && message.contains("session_init"),
        "the error should say resume is refused and what to do instead: {message}"
    );
    assert_eq!(
        coord.frame_sessions_mut().len(),
        0,
        "a refused resume must not resurrect the session"
    );
}

/// Resume after a cancel is refused with the cancel reason.
#[test]
fn resume_after_cancel_is_rejected() {
    let coord = make_coordinator();
    let session_id = stream_then_disconnect(&coord, "client-a", 2);
    super::super::frame_handlers::handle_session_cancel(
        &serde_json::json!({"type": "session_cancel", "session_id": session_id}),
        &coord,
    );

    let v = parse(&handle_session_resume(&resume_value(session_id), &coord));
    assert_eq!(v["error"], "session_gone");
    assert_eq!(v["reason"], "cancelled");
}

/// A disconnected session that then expires cannot be resumed. This is the
/// interaction between the new dormant state and the existing sweep: the sweep
/// reclaims dormant sessions on the same schedule as active ones.
#[test]
fn resume_after_expiry_of_a_disconnected_session_is_rejected() {
    let coord = make_coordinator();
    let session_id = stream_then_disconnect(&coord, "client-a", 2);
    assert_eq!(
        coord.frame_sessions_mut().disconnected_session_ids(),
        vec![session_id],
        "precondition: the session is dormant"
    );

    // Age it past the idle timeout and let the real sweep reclaim it.
    {
        let mut state = coord.frame_sessions_mut();
        let session = state
            .get_mut_for_test(session_id)
            .expect("session should still exist");
        session.last_activity_ms = session.last_activity_ms.saturating_sub(IDLE_TIMEOUT_MS * 2);
    }
    assert_eq!(
        super::super::frame_handlers::sweep_expired_frame_sessions(&coord),
        vec![session_id],
        "a disconnected session must still expire"
    );

    let v = parse(&handle_session_resume(&resume_value(session_id), &coord));
    assert_eq!(v["type"], "frame_error");
    assert_eq!(v["error"], "session_gone");
    assert_eq!(v["reason"], "expired");
}

/// An unknown id is rejected cleanly as `unknown_session`, distinct from a
/// session this daemon tore down.
#[test]
fn resume_with_an_unknown_session_id_is_rejected() {
    let coord = make_coordinator();
    let v = parse(&handle_session_resume(&resume_value(4242), &coord));
    assert_eq!(v["type"], "frame_error");
    assert_eq!(v["error"], "unknown_session");
    assert_eq!(v["session_id"], 4242);
}

/// A resume with no session id cannot identify its target.
#[test]
fn resume_without_session_id_is_rejected() {
    let coord = make_coordinator();
    let v = parse(&handle_session_resume(
        &serde_json::json!({"type": "session_resume"}),
        &coord,
    ));
    assert_eq!(v["type"], "frame_error");
    assert_eq!(v["error"], "missing_session_id");
}

/// A refused resume leaves no half-open state: with no sessions at all, every
/// refusal path must leave the table empty.
#[test]
fn refused_resume_leaves_no_half_open_state() {
    let coord = make_coordinator();

    assert_eq!(
        parse(&handle_session_resume(&resume_value(1), &coord))["error"],
        "unknown_session"
    );

    assert!(
        coord.frame_sessions_mut().is_empty(),
        "a refused resume must not create a session"
    );
    assert_eq!(coord.frame_sessions_mut().len(), 0);
}

/// A failed resume must not disturb the sessions that do exist.
#[test]
fn refused_resume_does_not_disturb_a_live_session() {
    let coord = make_coordinator();
    let session_id = stream_then_disconnect(&coord, "client-a", 2);

    handle_session_resume(&resume_value(9999), &coord);

    assert_eq!(coord.frame_sessions_mut().len(), 1);
    let live = parse(&super::super::frame_handlers::handle_frame_data(
        &frame_value(session_id, 3, 12288),
        &coord,
    ));
    assert_eq!(
        live["status"], "ok",
        "the real session is untouched: {live}"
    );
    assert_eq!(live["frame_count"], 3);
}

/// A wrong id must not be able to hijack a real session: resuming 9999 and then
/// streaming into it lands nowhere, and the real session's counters are exactly
/// what they were.
#[test]
fn a_wrong_id_cannot_touch_the_real_session() {
    let coord = make_coordinator();
    let session_id = stream_then_disconnect(&coord, "client-a", 2);

    handle_session_resume(&resume_value(9999), &coord);
    let stray = parse(&super::super::frame_handlers::handle_frame_data(
        &frame_value(9999, 0, 12288),
        &coord,
    ));
    assert_eq!(stray["error"], "unknown_session");

    let before = coord
        .frame_sessions_mut()
        .get(session_id)
        .expect("real session")
        .counters();
    assert_eq!(before.frame_count, 2, "no stray frame was accounted");
}

/// A session id is never reissued, so a stale id cannot land on a *new*
/// session. This is the property that makes "refused resume is final" hold.
#[test]
fn a_resumed_id_is_not_reissued_to_a_new_session() {
    let coord = make_coordinator();
    let session_id = stream_then_disconnect(&coord, "client-a", 1);
    super::super::frame_handlers::handle_session_close(
        &serde_json::json!({"type": "session_close", "session_id": session_id}),
        &coord,
    );

    let fresh = parse(&super::super::frame_handlers::handle_session_init(
        &init_value("client-b"),
        &coord,
    ));
    let fresh_id = fresh["session_id"].as_u64().expect("session_id");

    assert_ne!(fresh_id, session_id, "a closed id must not be reused");
    let v = parse(&handle_session_resume(&resume_value(session_id), &coord));
    assert_eq!(v["error"], "session_gone", "the stale id stays dead: {v}");
}
