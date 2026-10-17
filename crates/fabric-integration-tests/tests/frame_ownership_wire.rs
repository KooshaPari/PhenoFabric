//! Wire-level proof that frame-session ownership is enforced over a real daemon.
//!
//! Unit tests in `crates/fabric-daemon/src/wire/` exercise the handler in
//! isolation. This file drives the same gates end-to-end through
//! `run_wire_server` so the test sees what a real client sees, not what the
//! handler returns when called from inside the crate:
//!
//! 1. A `frame_data` whose `sender_id` does not match the session's current
//!    input owner is rejected with `input_owner_mismatch`. The frame does not
//!    increment counters.
//! 2. A `session_transfer` re-points the input seat to a new peer. After the
//!    transfer, the previous owner is locked out and the new owner can drive
//!    frames.
//! 3. The transfer is refused with a distinct error for an empty
//!    `input_owner`, and the live session is left untouched.
//! 4. The transfer is refused with `unknown_session` for an id the daemon
//!    never issued.
//! 5. A foreign `frame_ack` is also rejected with `input_owner_mismatch`,
//!    because the ownership gate is symmetric: only the current input owner
//!    can ack frames either.
//!
//! The file stays under the project's 500-line ceiling. The
//! `frame_exchange` helper is duplicated from `frame_disconnect_resume.rs`
//! because each integration test file is its own crate and a test-only helper
//! is not worth promoting into the harness for two call sites.
//!
//! Run: `cargo test -p fabric-integration-tests --test frame_ownership_wire`

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

use fabric_integration_tests::harness::{start_two_daemons, stop_daemon};

/// `session_init` line for the opener. The `client_id` becomes the session's
/// initial input owner at the state layer, so the matching `frame_data`
/// below must use the same id.
const INIT_LINE: &str = r#"{"type":"session_init","version":1,"preferred_codec":"Rgba","width":64,"height":48,"target_fps":60,"max_latency_ms":33,"client_id":"opener"}"#;

/// One `frame_data` line carrying `sender_id = opener`. Matches the input
/// owner recorded at `session_init`, so the ownership gate accepts it.
fn frame_with_sender(session_id: u64, seq: u64, sender: &str) -> String {
    format!(
        r#"{{"type":"frame_data","session_id":{session_id},"seq":{seq},"pts_us":{seq},"dts_us":{seq},"is_keyframe":true,"codec":"Rgba","width":64,"height":48,"payload_len":12288,"duration_us":16667,"sender_id":"{sender}"}}"#
    )
}

/// One `frame_ack` line carrying `sender_id = opener`. The ownership gate on
/// `frame_ack` is symmetric with `frame_data`.
fn ack_with_sender(session_id: u64, seq: u64, sender: &str) -> String {
    format!(
        r#"{{"type":"frame_ack","session_id":{session_id},"seq":{seq},"rtt_us":1234,"recv_pts_us":{seq},"sender_id":"{sender}"}}"#
    )
}

/// Open a connection and return a closure that sends one line and reads one
/// response. The closure owns the socket.
fn frame_exchange(addr: &str) -> impl FnMut(&str) -> serde_json::Value {
    let mut stream = TcpStream::connect(addr).expect("connect to daemon wire server");
    stream.set_nodelay(true).expect("set TCP_NODELAY");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("set read timeout");
    let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));

    move |request: &str| -> serde_json::Value {
        stream.write_all(request.as_bytes()).expect("write request");
        stream.write_all(b"\n").expect("write newline");
        stream.flush().expect("flush request");
        let mut line = String::new();
        let read = reader.read_line(&mut line).expect("read response");
        assert_ne!(read, 0, "daemon closed the connection");
        serde_json::from_str(line.trim())
            .unwrap_or_else(|e| panic!("response was not JSON ({e}): {line}"))
    }
}

// ===========================================================================
// Test 1: foreign sender_id is refused on the wire
// ===========================================================================

/// A `frame_data` whose `sender_id` is not the session's current input owner
/// is rejected with `input_owner_mismatch`. The error names both the
/// declared sender and the actual owner, so a client that mistyped its id
/// can diagnose itself, and the daemon's accounting is unchanged.
#[test]
fn frame_data_with_foreign_sender_id_over_real_daemon_is_input_owner_mismatch() {
    let (_a, b) = start_two_daemons();
    let mut exchange = frame_exchange(&b.addr);

    let ack = exchange(INIT_LINE);
    let session_id = ack["session_id"].as_u64().expect("session_id");

    // The opener's frame lands: it matches the input owner recorded at
    // session_init.
    let ok = exchange(&frame_with_sender(session_id, 0, "opener"));
    assert_eq!(ok["type"], "frame_ack", "got: {ok}");
    assert_eq!(ok["status"], "ok");
    assert_eq!(ok["frame_count"], 1, "the opener's frame was counted: {ok}");

    // A foreign sender is rejected. The error is `input_owner_mismatch`,
    // not `unknown_session` -- the daemon can see the session, but the
    // declared sender is not the owner on file.
    let rejected = exchange(&frame_with_sender(session_id, 1, "intruder"));
    assert_eq!(rejected["type"], "frame_error", "got: {rejected}");
    assert_eq!(
        rejected["error"], "input_owner_mismatch",
        "foreign sender_id must be rejected on the wire: {rejected}"
    );
    assert_eq!(rejected["session_id"], session_id);
    assert_eq!(
        rejected["current_input_owner"], "opener",
        "the daemon must report who actually owns the session, not the offender's id"
    );
    assert_eq!(rejected["declared_sender_id"], "intruder");

    // The refused frame did not shift counters. The next frame from the
    // real owner should still be seq 2 (the rejected one was seq 1), and
    // the running totals must show 1+1 = 2 frames, not 3.
    let after = exchange(&frame_with_sender(session_id, 1, "opener"));
    assert_eq!(after["type"], "frame_ack", "got: {after}");
    assert_eq!(after["status"], "ok");
    assert_eq!(
        after["frame_count"], 2,
        "the foreign frame must not increment the counter: {after}"
    );
    assert_eq!(
        after["frames_received"],
        2 * 12_288,
        "byte accounting must reflect only the two accepted frames"
    );

    stop_daemon(b);
}

// ===========================================================================
// Test 2: post-transfer lockout
// ===========================================================================

/// After a `session_transfer` re-points the input seat to a new peer, the
/// previous owner is locked out and the new owner is accepted. The lockout
/// is enforced by the same `input_owner_mismatch` gate as a foreign sender,
/// but the *trigger* is the transfer, not a bad id from a new connection:
/// the daemon must atomically re-seat the input owner, with no frame in
/// flight under the old owner being allowed to land afterwards.
#[test]
fn session_transfer_locks_out_previous_owner_over_real_daemon() {
    let (_a, b) = start_two_daemons();
    let mut exchange = frame_exchange(&b.addr);

    let ack = exchange(INIT_LINE);
    let session_id = ack["session_id"].as_u64().expect("session_id");

    // Opener drives one frame successfully (sanity check the gate lets
    // them in at all).
    let opener_first = exchange(&frame_with_sender(session_id, 0, "opener"));
    assert_eq!(opener_first["type"], "frame_ack", "got: {opener_first}");
    assert_eq!(opener_first["frame_count"], 1);

    // Hand the input seat to "newcomer". `session_transfer` is bearer-grade,
    // so a peer that knows the session id can request a transfer; the new
    // seats are validated by the daemon.
    let transfer = exchange(&format!(
        r#"{{"type":"session_transfer","session_id":{session_id},"input_owner":"newcomer","output_owner":"opener"}}"#
    ));
    assert_eq!(transfer["type"], "session_transfer_ack", "got: {transfer}");
    assert_eq!(transfer["status"], "ok");
    assert_eq!(transfer["session_id"], session_id);
    assert_eq!(transfer["input_owner"], "newcomer");
    assert_eq!(transfer["output_owner"], "opener");
    // `focus_owner` was not specified, so the state layer defaults it to
    // the new input owner. The ack must echo that default, not silently
    // leave it null.
    assert_eq!(
        transfer["focus_owner"], "newcomer",
        "the focus seat must default to the new input owner"
    );

    // The previous owner is now locked out. The error must be
    // `input_owner_mismatch` (not `unknown_session` and not
    // `degenerate_transfer`): the session still exists, and the request
    // itself is well-formed, but the declared sender is no longer the
    // current input owner.
    let locked_out = exchange(&frame_with_sender(session_id, 1, "opener"));
    assert_eq!(locked_out["type"], "frame_error", "got: {locked_out}");
    assert_eq!(
        locked_out["error"], "input_owner_mismatch",
        "the previous owner must be refused after transfer: {locked_out}"
    );
    assert_eq!(locked_out["session_id"], session_id);
    assert_eq!(locked_out["current_input_owner"], "newcomer");
    assert_eq!(locked_out["declared_sender_id"], "opener");

    // The new owner can drive the same seq number. The fact that the
    // rejected frame used seq 1 must not have advanced the daemon's
    // expected-next-seq counter -- a refused frame is a no-op.
    let newcomer_first = exchange(&frame_with_sender(session_id, 1, "newcomer"));
    assert_eq!(newcomer_first["type"], "frame_ack", "got: {newcomer_first}");
    assert_eq!(newcomer_first["status"], "ok");
    assert_eq!(
        newcomer_first["frame_count"], 2,
        "only two frames should be counted: opener seq 0, newcomer seq 1"
    );
    assert_eq!(newcomer_first["last_seq"], 1);

    stop_daemon(b);
}

// ===========================================================================
// Test 3: degenerate transfer leaves the session untouched
// ===========================================================================

/// A `session_transfer` that names an empty `input_owner` is refused with
/// `degenerate_transfer`. The state after a refused transfer must be
/// byte-identical to the state before it: the prior owner can still drive
/// frames, and the counters still hold.
#[test]
fn session_transfer_with_empty_input_owner_over_real_daemon_is_degenerate_transfer() {
    let (_a, b) = start_two_daemons();
    let mut exchange = frame_exchange(&b.addr);

    let ack = exchange(INIT_LINE);
    let session_id = ack["session_id"].as_u64().expect("session_id");

    // The opener drives one frame; this both sanity-checks the gate and
    // gives the post-refusal test a non-zero starting counter.
    let opener_first = exchange(&frame_with_sender(session_id, 0, "opener"));
    assert_eq!(opener_first["frame_count"], 1);

    // Empty `input_owner` is refused.
    let refused = exchange(&format!(
        r#"{{"type":"session_transfer","session_id":{session_id},"input_owner":"","output_owner":"newcomer"}}"#
    ));
    assert_eq!(refused["type"], "frame_error", "got: {refused}");
    assert_eq!(
        refused["error"], "degenerate_transfer",
        "an empty input_owner is a degenerate transfer, not an unknown session"
    );
    assert_eq!(refused["session_id"], session_id);

    // The opener can still drive frames. The transfer refusal was a
    // no-op: the input owner is still "opener".
    let still_opener = exchange(&frame_with_sender(session_id, 1, "opener"));
    assert_eq!(still_opener["type"], "frame_ack", "got: {still_opener}");
    assert_eq!(still_opener["status"], "ok");
    assert_eq!(
        still_opener["frame_count"], 2,
        "the refused transfer must not have shifted counters"
    );

    // And a foreign sender is still refused by the same gate as before:
    // the ownership state is unchanged.
    let foreign = exchange(&frame_with_sender(session_id, 2, "stranger"));
    assert_eq!(foreign["type"], "frame_error", "got: {foreign}");
    assert_eq!(foreign["error"], "input_owner_mismatch");
    assert_eq!(foreign["current_input_owner"], "opener");

    stop_daemon(b);
}

// ===========================================================================
// Test 4: transfer against an unknown session is reported
// ===========================================================================

/// A `session_transfer` that names a session this daemon never issued is
/// refused with `unknown_session`. The error is the same one `frame_data`
/// uses for a missing session, so a client that confuses transfer and
/// frame can still diagnose itself from a single error vocabulary.
#[test]
fn session_transfer_with_unknown_session_id_over_real_daemon_is_unknown_session() {
    let (_a, b) = start_two_daemons();
    let mut exchange = frame_exchange(&b.addr);

    // Open a real session so "unknown" is not conflated with "no sessions".
    let _ = exchange(INIT_LINE);

    let refused = exchange(
        r#"{"type":"session_transfer","session_id":9999,"input_owner":"alice","output_owner":"bob"}"#,
    );
    assert_eq!(refused["type"], "frame_error", "got: {refused}");
    assert_eq!(
        refused["error"], "unknown_session",
        "a transfer for a session this daemon never issued must be unknown_session: {refused}"
    );
    assert_eq!(refused["session_id"], 9999);

    stop_daemon(b);
}

// ===========================================================================
// Test 5: foreign frame_ack is also rejected
// ===========================================================================

/// The ownership gate is symmetric: a `frame_ack` whose `sender_id` does
/// not match the current input owner is rejected with
/// `input_owner_mismatch`. This protects the `last_seq` / RTT accounting
/// from being moved by a peer that has been locked out.
#[test]
fn frame_ack_with_foreign_sender_id_over_real_daemon_is_input_owner_mismatch() {
    let (_a, b) = start_two_daemons();
    let mut exchange = frame_exchange(&b.addr);

    let ack = exchange(INIT_LINE);
    let session_id = ack["session_id"].as_u64().expect("session_id");

    // The opener drives one frame, so the daemon has a real seq 0 to ack.
    let frame = exchange(&frame_with_sender(session_id, 0, "opener"));
    assert_eq!(frame["type"], "frame_ack", "got: {frame}");

    // The opener's own ack lands and is recorded.
    let opener_ack = exchange(&ack_with_sender(session_id, 0, "opener"));
    assert_eq!(
        opener_ack["type"], "frame_ack_response",
        "got: {opener_ack}"
    );
    assert_eq!(opener_ack["acked"], true);
    assert_eq!(opener_ack["ack_count"], 1);

    // A foreign ack is rejected. The error is the same as for a foreign
    // frame: the daemon enforces input-owner symmetry on both sides of the
    // flow.
    let foreign_ack = exchange(&ack_with_sender(session_id, 0, "intruder"));
    assert_eq!(foreign_ack["type"], "frame_error", "got: {foreign_ack}");
    assert_eq!(
        foreign_ack["error"], "input_owner_mismatch",
        "foreign sender_id on frame_ack must be refused: {foreign_ack}"
    );
    assert_eq!(foreign_ack["current_input_owner"], "opener");
    assert_eq!(foreign_ack["declared_sender_id"], "intruder");

    stop_daemon(b);
}
