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
