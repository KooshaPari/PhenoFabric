//! Unit tests for disconnect and resume of frame sessions.
//!
//! Split out of `frame_session_state_tests.rs` to keep both files under the
//! 500-line ceiling. These cover the state layer only; the wire-level behavior
//! (the `session_resume` request and its errors) is tested in
//! `frame_resume_tests.rs`.

use super::*;
use fabric_frame_transport::PROTOCOL_VERSION;

fn init(client: &str) -> SessionInit {
    SessionInit {
        version: PROTOCOL_VERSION,
        preferred_codec: Codec::Rgba,
        width: 64,
        height: 48,
        target_fps: 60,
        max_latency_ms: 33,
        client_id: client.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Disconnect and resume
// ---------------------------------------------------------------------------

/// A new session starts attached.
#[test]
fn new_session_starts_active() {
    let mut state = FrameSessionState::new();
    state.insert_session(&init("a"));
    assert_eq!(state.get(1).expect("session 1").state, SessionState::Active);
}

/// A disconnect marks the session dormant but does **not** destroy it. This is
/// the disconnect policy in one assertion: the entry, its id, and its counters
/// all survive, so a reconnecting client still has something to resume.
#[test]
fn disconnect_marks_dormant_without_destroying_the_session() {
    let mut state = FrameSessionState::new();
    state.insert_session_at(&init("a"), 1_000);
    state
        .touch_session(1, 2_000, |session| {
            session.last_seq = Some(7);
            session.frame_count += 3;
            session.frames_received += 900;
        })
        .expect("session 1 exists");

    assert!(state.mark_disconnected(1));

    let session = state.get(1).expect("a disconnect must not destroy");
    assert_eq!(session.state, SessionState::Disconnected);
    assert_eq!(session.last_seq, Some(7), "accounting must survive");
    assert_eq!(session.frame_count, 3);
    assert_eq!(session.frames_received, 900);
    assert_eq!(state.len(), 1, "the session must still occupy its slot");
    assert_eq!(state.disconnected_session_ids(), vec![1]);
}

/// A disconnect must not extend the session's life: the idle clock keeps
/// running from the last real frame, so a disconnected session is reclaimed on
/// schedule rather than getting a fresh 120 s window for free.
#[test]
fn disconnect_does_not_refresh_the_idle_clock() {
    let mut state = FrameSessionState::new();
    state.insert_session_at(&init("a"), 1_000);
    state
        .touch_session(1, 5_000, |_| {})
        .expect("session 1 exists");

    state.mark_disconnected(1);

    assert_eq!(
        state.get(1).expect("session 1").last_activity_ms,
        5_000,
        "a disconnect is not activity"
    );
}

/// A disconnected session is still swept by the ordinary idle expiry -- there
/// is no separate reaper and no exemption for the dormant state.
#[test]
fn disconnected_session_still_expires() {
    let mut state = FrameSessionState::new();
    state.insert_session_at(&init("a"), 1_000);
    state.mark_disconnected(1);

    // Not yet idle.
    assert!(state.expire_idle_sessions(10_999, 10_000).is_empty());
    assert!(state.contains(1), "still inside the timeout");

    // Now idle: the dormant session is reclaimed like any other.
    assert_eq!(state.expire_idle_sessions(11_000, 10_000), vec![1]);
    assert!(!state.contains(1));
    assert_eq!(state.teardown_reason(1), Some(TeardownReason::Expired));
}

/// Disconnecting an unknown id is a no-op that reports the miss, so a drop
/// cannot resurrect or create anything.
#[test]
fn disconnect_of_unknown_session_reports_the_miss() {
    let mut state = FrameSessionState::new();
    state.insert_session(&init("a"));
    assert!(!state.mark_disconnected(4242));
    assert_eq!(state.len(), 1, "no session may be created by a disconnect");
}

/// Disconnecting twice is idempotent: the postcondition is "dormant" either way,
/// so a client that reconnects and drops again must not see an error.
#[test]
fn disconnect_is_idempotent() {
    let mut state = FrameSessionState::new();
    state.insert_session(&init("a"));
    assert!(state.mark_disconnected(1));
    assert!(state.mark_disconnected(1));
    assert_eq!(
        state.get(1).expect("session 1").state,
        SessionState::Disconnected
    );
}

/// The bulk disconnect marks exactly the live ids it was given and skips the
/// rest, so a session that has since expired cannot make the drop path fail.
#[test]
fn mark_disconnected_many_skips_ids_with_no_live_session() {
    let mut state = FrameSessionState::new();
    state.insert_session(&init("a"));
    state.insert_session(&init("b"));

    // Session 1 was torn down before the drop arrived.
    state.close_session(1).expect("session 1 exists");

    let marked = state.mark_disconnected_many(&[1, 2, 99]);

    assert_eq!(marked, vec![2], "only the live id is marked");
    assert!(state.get(1).is_none(), "a dead id must not be resurrected");
    assert_eq!(
        state.get(2).expect("session 2").state,
        SessionState::Disconnected
    );
}

/// The bulk disconnect reports ids ascending and dedups repeats, so a log line
/// is deterministic no matter what order the traffic arrived in.
#[test]
fn mark_disconnected_many_is_sorted_and_deduped() {
    let mut state = FrameSessionState::new();
    for _ in 0..3 {
        state.insert_session(&init("a"));
    }

    assert_eq!(state.mark_disconnected_many(&[3, 1, 2, 1]), vec![1, 2, 3]);
}

/// The core round trip: disconnect, then resume the same session.
#[test]
fn resume_reconnects_a_disconnected_session() {
    let mut state = FrameSessionState::new();
    state.insert_session_at(&init("a"), 1_000);
    state.mark_disconnected(1);

    let outcome = state.resume_session(1, 9_000).expect("resume succeeds");

    assert_eq!(outcome.session_id, 1);
    assert_eq!(
        outcome.client_id, "a",
        "the client confirms its own session"
    );
    assert!(outcome.was_disconnected, "the drop was recorded");
    assert_eq!(outcome.state, SessionState::Active);
    assert_eq!(
        state.get(1).expect("session 1").state,
        SessionState::Active,
        "the session is attached again"
    );
    assert_eq!(
        state.get(1).expect("session 1").last_activity_ms,
        9_000,
        "a resume is proof of life and refreshes the clock"
    );
}

/// Sequencing continues across a resume; it does not restart. The counters the
/// resume hands back are the ones the client must continue from.
#[test]
fn resume_carries_sequencing_forward() {
    let mut state = FrameSessionState::new();
    state.insert_session_at(&init("a"), 1_000);
    for seq in 0..5 {
        state
            .touch_session(1, 2_000, |session| {
                session.last_seq = Some(seq);
                session.frame_count += 1;
                session.frames_received += 100;
            })
            .expect("session 1 exists");
    }
    state
        .touch_session(1, 2_000, |session| {
            session.acks.insert(4, 1_234);
        })
        .expect("session 1 exists");

    state.mark_disconnected(1);
    let outcome = state.resume_session(1, 9_000).expect("resume succeeds");

    assert_eq!(outcome.counters.last_seq, Some(4), "not reset to None");
    assert_eq!(outcome.counters.frame_count, 5, "not reset to zero");
    assert_eq!(outcome.counters.frames_received, 500);
    assert_eq!(outcome.counters.ack_count, 1, "recorded RTTs survive");

    // And the next frame continues from there rather than colliding.
    let after = state
        .touch_session(1, 10_000, |session| {
            session.last_seq = Some(5);
            session.frame_count += 1;
        })
        .expect("session 1 exists");
    assert_eq!(after.last_seq, Some(5), "sequence advanced, not restarted");
    assert_eq!(after.frame_count, 6);
}

/// Resuming an already-attached session succeeds and reports that no drop had
/// been recorded. It is idempotent, so a client that lost the first response
/// can retry instead of being told its session vanished.
#[test]
fn resume_of_an_active_session_is_idempotent() {
    let mut state = FrameSessionState::new();
    state.insert_session_at(&init("a"), 1_000);

    let outcome = state.resume_session(1, 2_000).expect("resume succeeds");

    assert!(!outcome.was_disconnected, "no drop was recorded");
    assert_eq!(outcome.state, SessionState::Active);
}

/// Resuming after a close is refused with the reason, and the refusal changes
/// nothing. Teardown is final.
#[test]
fn resume_after_close_is_refused_with_the_reason() {
    let mut state = FrameSessionState::new();
    state.insert_session(&init("a"));
    state.close_session(1).expect("session 1 exists");

    let err = state.resume_session(1, 2_000).expect_err("must be refused");

    assert_eq!(err, ResumeRejection::Gone(TeardownReason::Closed));
    assert!(
        state.is_empty(),
        "a refused resume must not create a session"
    );
}

/// Same for cancel.
#[test]
fn resume_after_cancel_is_refused() {
    let mut state = FrameSessionState::new();
    state.insert_session(&init("a"));
    state.cancel_session(1).expect("session 1 exists");

    assert_eq!(
        state.resume_session(1, 2_000).expect_err("must be refused"),
        ResumeRejection::Gone(TeardownReason::Cancelled)
    );
    assert!(state.is_empty());
}

/// And for expiry: a session that was disconnected and then swept stays dead.
#[test]
fn resume_after_expiry_is_refused() {
    let mut state = FrameSessionState::new();
    state.insert_session_at(&init("a"), 0);
    state.mark_disconnected(1);
    state.expire_idle_sessions(100_000, 10_000);

    assert_eq!(
        state
            .resume_session(1, 100_000)
            .expect_err("must be refused"),
        ResumeRejection::Gone(TeardownReason::Expired)
    );
    assert!(state.is_empty());
}

/// An id this daemon never issued is refused as `Unknown`, not as `Gone`. The
/// distinction is the same one `frame_data` makes and for the same reason.
#[test]
fn resume_of_unknown_id_is_refused_as_unknown() {
    let mut state = FrameSessionState::new();
    assert_eq!(
        state
            .resume_session(4242, 1_000)
            .expect_err("must be refused"),
        ResumeRejection::Unknown
    );
    assert!(state.is_empty(), "no session may be created by a refusal");
}

/// A refused resume must not disturb the sessions that do exist. This is the
/// "no half-open state" property at the state layer.
#[test]
fn a_refused_resume_leaves_existing_sessions_untouched() {
    let mut state = FrameSessionState::new();
    state.insert_session_at(&init("a"), 1_000);
    state
        .touch_session(1, 2_000, |session| {
            session.last_seq = Some(3);
            session.frame_count += 4;
        })
        .expect("session 1 exists");

    assert!(state.resume_session(999, 3_000).is_err());

    let session = state.get(1).expect("session 1 must survive");
    assert_eq!(session.last_seq, Some(3), "accounting is untouched");
    assert_eq!(session.frame_count, 4);
    assert_eq!(session.last_activity_ms, 2_000, "clock is untouched");
    assert_eq!(state.len(), 1, "nothing was added or removed");
}

/// A disconnect and resume cycle can repeat indefinitely without drifting the
/// accounting. Guards against resume silently clearing a counter.
#[test]
fn repeated_disconnect_resume_cycles_preserve_accounting() {
    let mut state = FrameSessionState::new();
    state.insert_session_at(&init("a"), 0);

    for cycle in 1..=3u64 {
        state
            .touch_session(1, cycle * 1_000, |session| {
                session.frame_count += 1;
                session.frames_received += 10;
            })
            .expect("session 1 exists");
        state.mark_disconnected(1);
        let outcome = state
            .resume_session(1, cycle * 1_000 + 1)
            .expect("resume succeeds");
        assert!(outcome.was_disconnected, "cycle {cycle} recorded a drop");
        assert_eq!(outcome.counters.frame_count, cycle);
    }

    assert_eq!(state.get(1).expect("session 1").frames_received, 30);
}

/// A resume must not consume a new session id: it continues the existing one,
/// and the next `session_init` still gets a fresh id.
#[test]
fn resume_does_not_allocate_a_new_session_id() {
    let mut state = FrameSessionState::new();
    state.insert_session(&init("a"));
    state.mark_disconnected(1);

    state.resume_session(1, 2_000).expect("resume succeeds");

    let next = state.insert_session(&init("b"));
    assert_eq!(
        next.session_id, 2,
        "ids keep advancing, resume consumes none"
    );
}

/// The session-state wire names are stable, because a client parses them.
#[test]
fn session_state_names_are_stable() {
    assert_eq!(SessionState::Active.as_str(), "active");
    assert_eq!(SessionState::Disconnected.as_str(), "disconnected");
}
