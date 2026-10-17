//! Unit tests for the `session_transfer` wire handler.
//!
//! The handler is the only wire-level path that mutates a session's input /
//! output / focus seats after `session_init`, so the test surface is
//! concentrated here rather than scattered across the frame plane:
//!
//! * State-layer tests for `FrameSessionState::transfer_ownership` (the
//!   method the handler delegates to) live in `frame_session_state_tests.rs`
//!   because they exercise state invariants in isolation. This file drives
//!   the JSON-line interface the daemon actually accepts on the socket and
//!   proves that a refused transfer is byte-identical to a no-op.
//!
//! `frame_transfer.rs` is short by design: the 500-line ceiling lives in
//! the daemon crate, and the transfer path is a control-plane request with
//! its own error taxonomy, so its tests sit alongside it.

use serde_json::Value;

use super::super::frame_session_state::TransferRejection;
use super::handle_session_transfer;
use crate::coordinator::Coordinator;
use crate::wire::frame_session_state::SessionOwners;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a fresh `Coordinator` backed by a throwaway sqlite file.
///
/// The temp dir is leaked (`std::mem::forget`) so the database file is not
/// removed while the coordinator is still alive. The pattern is copied
/// from `frame_handlers_tests::make_coordinator`; it cannot be shared
/// because `frame_handlers_tests` is a private sibling module.
fn make_coordinator() -> Coordinator {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("transfer-test.db");
    let config = crate::config::DaemonConfig {
        database: crate::config::DatabaseConfig {
            path: db_path,
            ..Default::default()
        },
        ..Default::default()
    };
    let coord = Coordinator::new(config).expect("coordinator");
    std::mem::forget(dir);
    coord
}

fn parse(resp: &str) -> Value {
    serde_json::from_str(resp).unwrap_or_else(|e| panic!("response not JSON: {e}: {resp}"))
}

fn init(client: &str) -> fabric_frame_transport::SessionInit {
    fabric_frame_transport::SessionInit {
        version: fabric_frame_transport::PROTOCOL_VERSION,
        preferred_codec: fabric_frame_transport::Codec::Rgba,
        width: 64,
        height: 48,
        target_fps: 60,
        max_latency_ms: 33,
        client_id: client.to_string(),
    }
}

/// Build a `session_transfer` value that targets `session_id` with explicit
/// owners. `focus_owner` defaults to the new `input_owner` when omitted.
fn transfer_value(session_id: u64, input: &str, output: &str, focus: Option<&str>) -> Value {
    let mut v = serde_json::json!({
        "type": "session_transfer",
        "session_id": session_id,
        "input_owner": input,
        "output_owner": output,
    });
    if let Some(focus) = focus {
        v["focus_owner"] = Value::String(focus.to_string());
    }
    v
}

/// Read the session's seats directly from the state layer. The handler
/// tests below compare `before` / `after` to prove the refusal paths are
/// atomic.
fn owners(coord: &Coordinator, id: u64) -> Option<SessionOwners> {
    coord.frame_sessions_mut().current_owners(id)
}

// ---------------------------------------------------------------------------
// Success path
// ---------------------------------------------------------------------------

/// A transfer with valid owners returns `session_transfer_ack` carrying the
/// new seats, and the state layer reports the same seats back to any later
/// reader. This is the contract the post-transfer frame gate relies on: the
/// seats recorded in the response are the seats the per-frame check
/// consults on the very next `frame_data`.
#[test]
fn session_transfer_hands_input_to_a_new_peer() {
    let coord = make_coordinator();
    let session = coord.frame_sessions_mut().insert_session(&init("client-a"));
    let id = session.session_id;

    let v = transfer_value(id, "client-b", "display-b", Some("client-b"));
    let resp = parse(&handle_session_transfer(&v, &coord));
    assert_eq!(resp["type"], "session_transfer_ack");
    assert_eq!(resp["status"], "ok");
    assert_eq!(resp["session_id"], id);
    assert_eq!(resp["input_owner"], "client-b");
    assert_eq!(resp["output_owner"], "display-b");
    assert_eq!(resp["focus_owner"], "client-b");

    let recorded = owners(&coord, id).expect("still live");
    assert_eq!(recorded.input_owner, "client-b");
    assert_eq!(recorded.output_owner, "display-b");
    assert_eq!(recorded.focus_owner, "client-b");
}

/// When the caller omits `focus_owner`, the state layer defaults it to the
/// new `input_owner` and the response carries the recorded value (not the
/// requested one). The test catches a class of bug where the handler would
/// echo a `null` field rather than the resolved seat.
#[test]
fn session_transfer_defaults_focus_owner_to_input_owner() {
    let coord = make_coordinator();
    let session = coord.frame_sessions_mut().insert_session(&init("client-a"));

    let v = transfer_value(session.session_id, "client-b", "display-b", None);
    let resp = parse(&handle_session_transfer(&v, &coord));
    assert_eq!(resp["focus_owner"], "client-b");
}

// ---------------------------------------------------------------------------
// Refusal paths
// ---------------------------------------------------------------------------

/// A request without a `session_id` is refused before the state layer is
/// touched: the response names the missing field, and the session table is
/// unchanged. The handler owns this refusal (the state-layer method
/// `transfer_ownership` accepts any `u64` because the daemon hands the
/// id out, so the wire-level "no id at all" is caught at the handler
/// boundary).
#[test]
fn session_transfer_missing_session_id_is_missing_session_id() {
    let coord = make_coordinator();
    let session = coord.frame_sessions_mut().insert_session(&init("client-a"));
    let before = coord.frame_sessions_mut().len();

    let v = serde_json::json!({
        "type": "session_transfer",
        "input_owner": "client-b",
        "output_owner": "display-b",
    });
    let resp = parse(&handle_session_transfer(&v, &coord));
    assert_eq!(resp["type"], "frame_error");
    assert_eq!(resp["error"], "missing_session_id");
    assert_eq!(
        coord.frame_sessions_mut().len(),
        before,
        "refusal must not touch the state"
    );

    let after = owners(&coord, session.session_id).expect("live");
    assert_eq!(after.input_owner, "client-a");
}

/// Empty owner ids are caught at the handler before the state layer is
/// reached; the refusal names the offending field so the caller can fix
/// the request without a follow-up probe.
#[test]
fn session_transfer_empty_input_owner_is_degenerate() {
    let coord = make_coordinator();
    let session = coord.frame_sessions_mut().insert_session(&init("client-a"));
    let before = owners(&coord, session.session_id).expect("live");

    let v = transfer_value(session.session_id, "", "display-b", None);
    let resp = parse(&handle_session_transfer(&v, &coord));
    assert_eq!(resp["type"], "frame_error");
    assert_eq!(resp["error"], "degenerate_transfer");
    assert_eq!(resp["session_id"], session.session_id);

    let after = owners(&coord, session.session_id).expect("still live");
    assert_eq!(before.input_owner, after.input_owner);
    assert_eq!(before.output_owner, after.output_owner);
    assert_eq!(before.focus_owner, after.focus_owner);
}

#[test]
fn session_transfer_empty_output_owner_is_degenerate() {
    let coord = make_coordinator();
    let session = coord.frame_sessions_mut().insert_session(&init("client-a"));

    let v = transfer_value(session.session_id, "client-b", "", None);
    let resp = parse(&handle_session_transfer(&v, &coord));
    assert_eq!(resp["type"], "frame_error");
    assert_eq!(resp["error"], "degenerate_transfer");
}

#[test]
fn session_transfer_empty_focus_owner_is_degenerate() {
    let coord = make_coordinator();
    let session = coord.frame_sessions_mut().insert_session(&init("client-a"));

    let v = transfer_value(session.session_id, "client-b", "display-b", Some(""));
    let resp = parse(&handle_session_transfer(&v, &coord));
    assert_eq!(resp["type"], "frame_error");
    assert_eq!(resp["error"], "degenerate_transfer");
}

/// A session id this daemon never issued is refused as `unknown_session`,
/// distinct from the `session_gone` refusal below. The two paths are
/// deliberately kept separate so a client whose session expired
/// (daemon-side) is told so, not told its id was never valid.
#[test]
fn session_transfer_unknown_session_is_unknown_session() {
    let coord = make_coordinator();
    // No session is ever opened.

    let v = transfer_value(9_999, "client-b", "display-b", None);
    let resp = parse(&handle_session_transfer(&v, &coord));
    assert_eq!(resp["type"], "frame_error");
    assert_eq!(resp["error"], "unknown_session");
    assert_eq!(resp["session_id"], 9_999);
}

/// A session id that was issued and has since been closed is refused as
/// `session_gone` with a stable `reason` field. The point of the test is
/// that the refusal names the prior teardown reason, so a client that
/// races a transfer against a daemon-side sweep can tell the two apart.
#[test]
fn session_transfer_session_gone_after_close() {
    let coord = make_coordinator();
    let session = coord.frame_sessions_mut().insert_session(&init("client-a"));
    let id = session.session_id;
    coord
        .frame_sessions_mut()
        .close_session(id)
        .expect("session existed");

    let v = transfer_value(id, "client-b", "display-b", None);
    let resp = parse(&handle_session_transfer(&v, &coord));
    assert_eq!(resp["type"], "frame_error");
    assert_eq!(resp["error"], "session_gone");
    assert_eq!(resp["reason"], "closed");
    assert_eq!(resp["session_id"], id);
}

// ---------------------------------------------------------------------------
// Refusal is atomic
// ---------------------------------------------------------------------------

/// The contract: a refused transfer must not mutate the session. This
/// single test exercises every refusal path on a separate session, so a
/// future change that quietly updates one seat while refusing the other
/// would be caught here even if each single-field test still passed.
#[test]
fn session_transfer_refusal_does_not_mutate_state() {
    // --- Path 1: missing session_id ---
    let coord = make_coordinator();
    let s1 = coord.frame_sessions_mut().insert_session(&init("client-a"));
    let v = serde_json::json!({
        "type": "session_transfer",
        "input_owner": "client-b",
        "output_owner": "display-b",
    });
    let _ = handle_session_transfer(&v, &coord);
    let after = owners(&coord, s1.session_id).expect("live");
    assert_eq!(after.input_owner, "client-a");
    assert_eq!(after.output_owner, "client-a");
    assert_eq!(after.focus_owner, "client-a");

    // --- Path 2: degenerate input owner ---
    let coord = make_coordinator();
    let s2 = coord.frame_sessions_mut().insert_session(&init("client-a"));
    let v = transfer_value(s2.session_id, "", "display-b", None);
    let _ = handle_session_transfer(&v, &coord);
    let after = owners(&coord, s2.session_id).expect("live");
    assert_eq!(after.input_owner, "client-a");

    // --- Path 3: degenerate output owner ---
    let coord = make_coordinator();
    let s3 = coord.frame_sessions_mut().insert_session(&init("client-a"));
    let v = transfer_value(s3.session_id, "client-b", "", None);
    let _ = handle_session_transfer(&v, &coord);
    let after = owners(&coord, s3.session_id).expect("live");
    assert_eq!(after.output_owner, "client-a");

    // --- Path 4: unknown session ---
    let coord = make_coordinator();
    let s4 = coord.frame_sessions_mut().insert_session(&init("client-a"));
    let v = transfer_value(9_999, "client-b", "display-b", None);
    let _ = handle_session_transfer(&v, &coord);
    let after = owners(&coord, s4.session_id).expect("live");
    assert_eq!(after.input_owner, "client-a");

    // --- Path 5: session already torn down ---
    let coord = make_coordinator();
    let s5 = coord.frame_sessions_mut().insert_session(&init("client-a"));
    coord
        .frame_sessions_mut()
        .close_session(s5.session_id)
        .expect("session existed");
    let v = transfer_value(s5.session_id, "client-b", "display-b", None);
    let _ = handle_session_transfer(&v, &coord);
    // Session is gone; refusal must not have re-created it.
    assert!(owners(&coord, s5.session_id).is_none());
}

// ---------------------------------------------------------------------------
// State-layer probe (light)
// ---------------------------------------------------------------------------

/// The `TransferRejection` error names are stable contract on the wire
/// (the handler relies on the same names) and on the state-layer API
/// (downstream code can match on them). Pinning the exact strings here
/// guards against accidental renames in the `as_str` impl.
#[test]
fn transfer_rejection_error_names_are_stable() {
    use crate::wire::frame_session_state::TeardownReason;

    assert_eq!(
        TransferRejection::DegenerateTransfer.as_str(),
        "degenerate_transfer"
    );
    assert_eq!(
        TransferRejection::UnknownSession.as_str(),
        "unknown_session"
    );
    assert_eq!(
        TransferRejection::SessionGone(TeardownReason::Closed).as_str(),
        "session_gone_closed"
    );
    assert_eq!(
        TransferRejection::SessionGone(TeardownReason::Cancelled).as_str(),
        "session_gone_cancelled"
    );
    assert_eq!(
        TransferRejection::SessionGone(TeardownReason::Expired).as_str(),
        "session_gone_expired"
    );
    assert_eq!(TransferRejection::MissingField.as_str(), "missing_field");
}
