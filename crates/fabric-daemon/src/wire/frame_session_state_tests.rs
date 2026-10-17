//! Unit tests for the frame-streaming session state.
//!
//! Split out of `frame_session_state.rs` to keep both files well under the
//! 500-line ceiling.

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

#[test]
fn new_state_is_empty_and_starts_at_one() {
    let state = FrameSessionState::new();
    assert_eq!(state.len(), 0);
    assert!(!state.contains(1));
    assert!(state.get(1).is_none());
    assert!(state.known_session_ids().is_empty());
}

#[test]
fn session_ids_are_allocated_incrementally_from_one() {
    let mut state = FrameSessionState::new();
    let a = state.insert_session(&init("a"));
    let b = state.insert_session(&init("b"));
    let c = state.insert_session(&init("c"));
    assert_eq!(a.session_id, 1);
    assert_eq!(b.session_id, 2);
    assert_eq!(c.session_id, 3);
    assert_eq!(state.len(), 3);
}

#[test]
fn insert_session_records_every_session_init_field() {
    let mut state = FrameSessionState::new();
    let session = state.insert_session(&init("client-x"));
    assert_eq!(session.client_id, "client-x");
    assert_eq!(session.codec, Codec::Rgba);
    assert_eq!(session.width, 64);
    assert_eq!(session.height, 48);
    assert_eq!(session.fps, 60);
    assert_eq!(session.max_latency_ms, 33);
    assert_eq!(session.version, PROTOCOL_VERSION);
    assert_eq!(session.frame_count, 0);
    assert_eq!(session.frames_received, 0);
    assert!(session.last_seq.is_none());
}

#[test]
fn known_session_ids_is_sorted_ascending() {
    let mut state = FrameSessionState::new();
    state.insert_session(&init("a"));
    state.insert_session(&init("b"));
    state.insert_session(&init("c"));
    assert_eq!(state.known_session_ids(), vec![1, 2, 3]);
}

#[test]
fn update_session_returns_none_for_unknown_id() {
    let mut state = FrameSessionState::new();
    state.insert_session(&init("a"));
    let result = state.update_session(99, |_| panic!("must not be called"));
    assert!(result.is_none());
}

#[test]
fn update_session_returns_post_mutation_counters() {
    let mut state = FrameSessionState::new();
    state.insert_session(&init("a"));

    let counters = state
        .update_session(1, |session| {
            session.last_seq = Some(7);
            session.frame_count += 1;
            session.frames_received += 500;
        })
        .expect("session 1 exists");

    assert_eq!(counters.frame_count, 1);
    assert_eq!(counters.frames_received, 500);
    assert_eq!(counters.last_seq, Some(7));
    assert_eq!(counters.ack_count, 0);
}

#[test]
fn update_session_counts_distinct_acked_sequences() {
    let mut state = FrameSessionState::new();
    state.insert_session(&init("a"));

    for seq in 0..3 {
        state
            .update_session(1, |session| {
                session.acks.insert(seq, seq * 10);
            })
            .expect("session 1 exists");
    }
    // Re-acking the same sequence must not inflate the count.
    state
        .update_session(1, |session| {
            session.acks.insert(1, 999);
        })
        .expect("session 1 exists");

    let counters = state.get(1).expect("session 1 exists").counters();
    assert_eq!(counters.ack_count, 3);
    assert_eq!(state.get(1).unwrap().acks.get(&1), Some(&999));
}

#[test]
fn sessions_are_independent() {
    let mut state = FrameSessionState::new();
    state.insert_session(&init("a"));
    state.insert_session(&init("b"));

    state
        .update_session(1, |session| {
            session.frame_count += 1;
            session.frames_received += 10;
        })
        .expect("session 1 exists");
    state
        .update_session(1, |session| session.frame_count += 1)
        .expect("session 1 exists");
    state
        .update_session(2, |session| session.frames_received += 99)
        .expect("session 2 exists");

    let one = state.get(1).expect("session 1 exists");
    let two = state.get(2).expect("session 2 exists");
    assert_eq!(one.frame_count, 2);
    assert_eq!(one.frames_received, 10);
    assert_eq!(one.client_id, "a");
    assert_eq!(two.frame_count, 0);
    assert_eq!(two.frames_received, 99);
    assert_eq!(two.client_id, "b");
}

#[test]
fn counters_reflect_a_full_frame_and_ack_cycle() {
    let mut state = FrameSessionState::new();
    state.insert_session(&init("a"));

    state
        .update_session(1, |s| {
            s.last_seq = Some(3);
            s.frame_count += 1;
            s.frames_received += 1024;
        })
        .expect("session 1 exists");
    let after_frame = state
        .update_session(1, |s| {
            s.acks.insert(3, 250);
        })
        .expect("session 1 exists");

    assert_eq!(after_frame.frame_count, 1);
    assert_eq!(after_frame.frames_received, 1024);
    assert_eq!(after_frame.ack_count, 1);
    assert_eq!(after_frame.last_seq, Some(3));
}

// ---------------------------------------------------------------------------
// Session lifecycle: activity, teardown, expiry, cap
// ---------------------------------------------------------------------------

/// A session created at `now_ms` records it as its activity time.
#[test]
fn new_session_records_its_activity_time() {
    let mut state = FrameSessionState::new();
    state.insert_session_at(&init("a"), 1_000);
    assert_eq!(state.get(1).expect("session 1").last_activity_ms, 1_000);
}

/// `touch_session` refreshes the idle clock and reports the post-mutation
/// counters, exactly as `update_session` does.
#[test]
fn touch_session_refreshes_activity_and_returns_counters() {
    let mut state = FrameSessionState::new();
    state.insert_session_at(&init("a"), 1_000);

    let counters = state
        .touch_session(1, 5_000, |session| {
            session.frame_count += 1;
            session.frames_received += 100;
        })
        .expect("session 1 exists");

    assert_eq!(state.get(1).expect("session 1").last_activity_ms, 5_000);
    assert_eq!(counters.frame_count, 1);
    assert_eq!(counters.frames_received, 100);
}

/// `touch_session` on an unknown id must not touch anything and must report
/// the miss, so the handler can turn it into an explicit error.
#[test]
fn touch_session_returns_none_for_unknown_id() {
    let mut state = FrameSessionState::new();
    assert!(state
        .touch_session(42, 1_000, |_| panic!("must not be called"))
        .is_none());
}

/// Closing removes the session from the map: this is the whole of teardown.
#[test]
fn close_session_removes_the_session() {
    let mut state = FrameSessionState::new();
    state.insert_session(&init("a"));
    assert_eq!(state.len(), 1);

    let outcome = state.close_session(1).expect("session 1 exists");

    assert_eq!(outcome.session_id, 1);
    assert_eq!(outcome.reason, TeardownReason::Closed);
    assert_eq!(outcome.client_id, "a");
    assert_eq!(state.len(), 0);
    assert!(!state.contains(1));
    assert!(state.get(1).is_none());
    assert!(state.is_empty());
}

/// A closed session cannot be closed again. The second attempt is a miss, not
/// a second success, so a client can tell its close landed.
#[test]
fn close_session_twice_is_a_miss() {
    let mut state = FrameSessionState::new();
    state.insert_session(&init("a"));
    assert!(state.close_session(1).is_some());
    assert!(
        state.close_session(1).is_none(),
        "a double close must report the miss"
    );
}

/// Cancelling removes the session and records its own distinct reason.
#[test]
fn cancel_session_removes_the_session() {
    let mut state = FrameSessionState::new();
    state.insert_session(&init("mid-transfer"));
    assert_eq!(state.len(), 1);

    let outcome = state.cancel_session(1).expect("session 1 exists");

    assert_eq!(outcome.reason, TeardownReason::Cancelled);
    assert_eq!(outcome.client_id, "mid-transfer");
    assert_eq!(state.len(), 0);
    assert!(state.get(1).is_none());
}

/// Cancellation releases everything the session accumulated: the entry is gone
/// and its recorded acks went with it.
#[test]
fn cancel_session_releases_partial_state() {
    let mut state = FrameSessionState::new();
    state.insert_session(&init("a"));
    state
        .touch_session(1, 10, |session| {
            session.last_seq = Some(4);
            session.frame_count += 5;
            session.frames_received += 5_000;
            session.acks.insert(4, 1_200);
            session.acks.insert(3, 1_100);
        })
        .expect("session 1 exists");

    // The partial state exists before the cancel...
    let before = state.get(1).expect("session 1 exists");
    assert_eq!(before.acks.len(), 2);
    assert_eq!(before.frame_count, 5);

    // ...and the teardown hands back its final counters rather than losing
    // them, then drops the entry entirely.
    let outcome = state.cancel_session(1).expect("session 1 exists");
    assert_eq!(outcome.counters.frame_count, 5);
    assert_eq!(outcome.counters.frames_received, 5_000);
    assert_eq!(outcome.counters.ack_count, 2);
    assert_eq!(outcome.counters.last_seq, Some(4));

    assert!(state.get(1).is_none(), "the session must not remain");
    assert_eq!(state.len(), 0, "the slot must be released");
}

/// A sweep removes exactly the sessions that have been idle for at least the
/// timeout, and leaves live ones alone.
#[test]
fn expire_idle_sessions_removes_only_idle_sessions() {
    let mut state = FrameSessionState::new();
    state.insert_session_at(&init("old"), 1_000);
    state.insert_session_at(&init("new"), 9_500);

    // Session 1 is 10_000 ms idle, session 2 is 500 ms idle.
    let expired = state.expire_idle_sessions(11_000, 10_000);

    assert_eq!(expired, vec![1]);
    assert!(!state.contains(1), "the idle session must be gone");
    assert!(state.contains(2), "the active session must survive");
    assert_eq!(state.len(), 1);
}

/// A session exactly at the timeout has been idle for `>= idle_timeout_ms`,
/// so it is swept. The boundary is inclusive deliberately.
#[test]
fn expire_idle_sessions_sweeps_at_the_boundary() {
    let mut state = FrameSessionState::new();
    state.insert_session_at(&init("a"), 1_000);

    // One millisecond short of the timeout: kept.
    assert!(state.expire_idle_sessions(10_999, 10_000).is_empty());
    assert!(state.contains(1));

    // Exactly at the timeout: swept.
    assert_eq!(state.expire_idle_sessions(11_000, 10_000), vec![1]);
    assert!(!state.contains(1));
}

/// Activity keeps a session alive: a session touched just before the sweep is
/// not idle, however old it was when it was created.
#[test]
fn activity_refreshes_the_idle_deadline() {
    let mut state = FrameSessionState::new();
    state.insert_session_at(&init("a"), 0);

    // Long idle, then activity: the clock restarts from the touch.
    state
        .touch_session(1, 9_000, |s| s.frame_count += 1)
        .expect("session 1 exists");

    assert!(
        state.expire_idle_sessions(10_000, 10_000).is_empty(),
        "a session touched at 9000 is only 1000 ms idle at 10000"
    );
    assert!(state.contains(1));
}

/// The sweep returns ids ascending so a caller can log deterministically.
#[test]
fn expire_idle_sessions_returns_ids_ascending() {
    let mut state = FrameSessionState::new();
    for _ in 0..4 {
        state.insert_session_at(&init("a"), 0);
    }
    assert_eq!(
        state.expire_idle_sessions(100_000, 10_000),
        vec![1, 2, 3, 4]
    );
}

/// A clock that steps backwards must not make sessions look infinitely idle.
/// Without the saturating subtraction, every session would be swept at once.
#[test]
fn expire_idle_sessions_tolerates_a_backwards_clock() {
    let mut state = FrameSessionState::new();
    state.insert_session_at(&init("a"), 50_000);

    // now_ms is behind last_activity_ms; saturating_sub yields 0.
    let expired = state.expire_idle_sessions(1_000, 10_000);

    assert!(
        expired.is_empty(),
        "a backwards clock must not evict live sessions: {expired:?}"
    );
    assert!(state.contains(1));
}

/// The teardown reason is remembered so a later `frame_data` can report that
/// the session was closed rather than claiming it never existed.
#[test]
fn teardown_reason_records_why_a_session_went_away() {
    let mut state = FrameSessionState::new();
    state.insert_session(&init("a"));
    state.insert_session(&init("b"));
    state.insert_session_at(&init("c"), 0);

    state.close_session(1).expect("session 1 exists");
    state.cancel_session(2).expect("session 2 exists");
    state.expire_idle_sessions(100_000, 10_000);

    assert_eq!(state.teardown_reason(1), Some(TeardownReason::Closed));
    assert_eq!(state.teardown_reason(2), Some(TeardownReason::Cancelled));
    assert_eq!(state.teardown_reason(3), Some(TeardownReason::Expired));
    // An id that was never issued has no reason.
    assert_eq!(state.teardown_reason(999), None);
}

/// The tombstone history is bounded: an unbounded one would grow for the
/// lifetime of the daemon, which is exactly the leak the cap exists to stop.
#[test]
fn teardown_history_is_bounded() {
    let mut state = FrameSessionState::new();
    // More teardowns than the history can hold.
    for _ in 0..(TEARDOWN_HISTORY + 20) {
        let id = state.insert_session(&init("a")).session_id;
        state.close_session(id).expect("session exists");
    }

    // The most recent teardown is still known...
    let last = state.insert_session(&init("a")).session_id;
    state.close_session(last).expect("session exists");
    assert_eq!(state.teardown_reason(last), Some(TeardownReason::Closed));

    // ...but the oldest has aged out and reads as "never existed", which is
    // the safe fallback: an id this daemon no longer remembers.
    assert_eq!(
        state.teardown_reason(1),
        None,
        "the oldest tombstone must have aged out"
    );
    assert!(state.is_empty(), "teardown must not leave sessions behind");
}

/// The cap reports room while below the limit and none once it is reached.
#[test]
fn session_capacity_tracks_the_cap() {
    let mut state = FrameSessionState::new();
    assert_eq!(state.session_capacity(), MAX_CONCURRENT_SESSIONS);
    assert!(state.has_session_capacity());

    for _ in 0..MAX_CONCURRENT_SESSIONS {
        state.insert_session(&init("a"));
    }

    assert!(!state.has_session_capacity(), "the cap must be reached");
    assert_eq!(state.len(), MAX_CONCURRENT_SESSIONS);
}

/// Closing a session frees a slot under the cap: the cap limits *concurrent*
/// sessions, so teardown is how a long-lived client makes room.
#[test]
fn closing_frees_a_slot_under_the_cap() {
    let mut state = FrameSessionState::new();
    for _ in 0..MAX_CONCURRENT_SESSIONS {
        state.insert_session(&init("a"));
    }
    assert!(!state.has_session_capacity());

    state.close_session(1).expect("session 1 exists");

    assert!(
        state.has_session_capacity(),
        "a freed slot must be reusable"
    );
}

/// Ids are not reused after teardown, so a stale `frame_data` can never land
/// on a different session that happens to reuse the number.
#[test]
fn teardown_does_not_reuse_session_ids() {
    let mut state = FrameSessionState::new();
    let first = state.insert_session(&init("a"));
    state
        .close_session(first.session_id)
        .expect("session exists");

    let second = state.insert_session(&init("b"));

    assert_eq!(first.session_id, 1);
    assert_eq!(
        second.session_id, 2,
        "a closed id must not be handed out again"
    );
}
