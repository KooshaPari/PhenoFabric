//! Unit tests for the wire server's per-connection frame-session tracking.
//!
//! Split out of `mod.rs` to keep that file well under the 500-line ceiling.
//!
//! `note_session_in_line` is what lets a dropped connection mark its sessions
//! dormant instead of leaving them silently "active". It is a best-effort
//! classifier over untrusted input, so the cases below pin both what it must
//! record and what it must ignore -- an over-broad classifier would mark
//! unrelated sessions dormant, and an under-broad one would leave them
//! attached forever.

use super::*;

/// Convenience: feed one line to a fresh collector and return what it noted.
fn noted(line: &str) -> Vec<u64> {
    let mut sessions = Vec::new();
    note_session_in_line(line, &mut sessions);
    sessions
}

#[test]
fn frame_data_notes_its_session() {
    assert_eq!(
        noted(r#"{"type":"frame_data","session_id":7,"seq":0,"codec":"Rgba"}"#),
        vec![7]
    );
}

#[test]
fn frame_ack_notes_its_session() {
    assert_eq!(
        noted(r#"{"type":"frame_ack","session_id":3,"seq":1,"rtt_us":100}"#),
        vec![3]
    );
}

#[test]
fn session_resume_notes_its_session() {
    // A resume names a session, so the connection is carrying it and a drop
    // should mark it dormant.
    assert_eq!(
        noted(r#"{"type":"session_resume","session_id":9}"#),
        vec![9]
    );
}

/// Both casings are accepted on the wire, so both must be recognized here --
/// otherwise a PascalCase client would never get its session marked dormant.
#[test]
fn pascal_case_frame_types_are_noted() {
    assert_eq!(
        noted(r#"{"type":"FrameData","session_id":4,"seq":0}"#),
        vec![4]
    );
    assert_eq!(
        noted(r#"{"type":"FrameAck","session_id":5,"seq":0}"#),
        vec![5]
    );
    assert_eq!(noted(r#"{"type":"SessionResume","session_id":6}"#), vec![6]);
}

/// `session_init` assigns the id server-side, so the request itself names no
/// session. Recording nothing is correct: the id appears in the next frame.
#[test]
fn session_init_notes_nothing() {
    assert!(noted(
        r#"{"type":"session_init","client_id":"c","width":64,"height":48,"target_fps":60}"#
    )
    .is_empty());
}

/// Teardown names a session but removes it, so there is nothing left to mark
/// dormant. Counting these would only make the drop path re-flag a dead id.
#[test]
fn teardown_notes_nothing() {
    assert!(noted(r#"{"type":"session_close","session_id":1}"#).is_empty());
    assert!(noted(r#"{"type":"session_cancel","session_id":1}"#).is_empty());
}

/// Non-frame messages say nothing about sessions. A client that only ever
/// heartbeats must not mark anything dormant.
#[test]
fn non_frame_messages_note_nothing() {
    for line in [
        r#"{"type":"heartbeat"}"#,
        r#"{"type":"health_check"}"#,
        r#"{"type":"topology_request"}"#,
        r#"{"type":"surface_lease_request","name":"n","protocol":"posix","locality_floor":"L5"}"#,
    ] {
        assert!(noted(line).is_empty(), "{line} must not note a session");
    }
}

/// The collector must survive hostile input rather than panicking: the wire
/// server reads arbitrary bytes off a socket.
#[test]
fn malformed_input_notes_nothing_without_panicking() {
    for line in [
        "not json at all",
        "",
        "[1,2,3]",
        r#"{"no_type":true}"#,
        r#"{"type":42}"#,
        r#"{"type":"frame_data","session_id":"7"}"#,
        r#"{"type":"frame_data","session_id":-1}"#,
        r#"{"type":"frame_data","session_id":1.5}"#,
    ] {
        assert!(noted(line).is_empty(), "{line} must not note a session");
    }
}

/// A connection that streams thousands of frames must not accumulate thousands
/// of entries: one occurrence of an id is all the drop path needs.
#[test]
fn repeated_frames_note_each_session_once() {
    let mut sessions = Vec::new();
    for seq in 0..1_000 {
        note_session_in_line(
            &format!(r#"{{"type":"frame_data","session_id":7,"seq":{seq}}}"#),
            &mut sessions,
        );
    }
    assert_eq!(sessions, vec![7]);
}

/// Several sessions on one connection are each noted once, in arrival order.
/// Order is preserved here (sorting happens later, in
/// `mark_disconnected_many`) so the collector stays a faithful record of what
/// the connection actually carried.
#[test]
fn multiple_sessions_are_all_noted_in_order() {
    let mut sessions = Vec::new();
    for session_id in [3, 1, 3, 2, 1] {
        note_session_in_line(
            &format!(r#"{{"type":"frame_data","session_id":{session_id},"seq":0}}"#),
            &mut sessions,
        );
    }
    assert_eq!(sessions, vec![3, 1, 2], "duplicates collapse, order kept");
}
