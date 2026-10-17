//! Frame-session disconnect / reconnect / resume over a real daemon.
//!
//! Split out of `two_node.rs` to keep both files under the 500-line ceiling.
//! These are the integration counterpart to the unit tests in
//! `crates/fabric-daemon/src/wire/frame_resume_tests.rs`: the unit tests drive
//! the handler directly, these drive it through a real TCP socket against a
//! real `run_wire_server`, including the connection-drop path that only exists
//! inside the server's accept loop.
//!
//! Run: `cargo test -p fabric-integration-tests --test frame_disconnect_resume`

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

use fabric_integration_tests::harness::{start_two_daemons, stop_daemon};

/// A `session_init` line, identical to the one the core tests use.
const INIT_LINE: &str = r#"{"type":"session_init","version":1,"preferred_codec":"Rgba","width":64,"height":48,"target_fps":60,"max_latency_ms":33,"client_id":"teardown-client"}"#;

/// One `frame_data` line. 64*48*4 = 12288 bytes of RGBA per frame.
fn frame_line(session_id: u64, seq: u64) -> String {
    format!(
        r#"{{"type":"frame_data","session_id":{session_id},"seq":{seq},"pts_us":{seq},"dts_us":{seq},"is_keyframe":true,"codec":"Rgba","width":64,"height":48,"payload_len":12288,"duration_us":16667}}"#
    )
}

/// Open a connection and return a request/response exchange closure.
///
/// Mirrors `two_node.rs`'s helper of the same name. Duplicated rather than
/// shared because each integration test file is its own crate, and a test-only
/// helper is not worth promoting into the harness for two call sites.
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
// Test 10: disconnect / reconnect / resume over a real daemon
// ===========================================================================

/// A client that loses its TCP connection mid-session can reconnect, resume the
/// same session, and continue its frame sequence from where it stopped.
///
/// This is the disconnect/reconnect acceptance path end-to-end. Unlike
/// `frame_session_state_survives_reconnection` (which only shows state outlives
/// a socket), this drives the whole cycle:
///
///   1. `session_init` on connection A -> `session_ack` with a session id.
///   2. Frames on connection A; the daemon accounts them.
///   3. Connection A is **dropped at the TCP level** -- the socket is closed
///      without any `session_close`, which is exactly what a client crash or a
///      severed link looks like to the daemon.
///   4. The daemon's wire server observes the drop and marks the session
///      dormant. It must **not** destroy it.
///   5. Connection B resumes with `session_resume` and gets the accounting it
///      must continue from.
///   6. Frames continue on connection B at the next sequence number, and the
///      counters carry forward rather than restarting.
#[test]
fn frame_session_disconnect_reconnect_and_resume_over_real_daemon() {
    let (_a, b) = start_two_daemons();

    // --- 1. Open a session on connection A ---
    let mut conn_a = TcpStream::connect(&b.addr).expect("connect A");
    conn_a.set_nodelay(true).expect("set TCP_NODELAY");
    conn_a
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("set read timeout");
    let mut reader_a = BufReader::new(conn_a.try_clone().expect("clone A"));

    let ack = exchange_line(&mut conn_a, &mut reader_a, INIT_LINE);
    assert_eq!(ack["type"], "session_ack", "got: {ack}");
    let session_id = ack["session_id"].as_u64().expect("session_id");

    // --- 2. Stream a few frames ---
    for seq in 0..3 {
        let f = exchange_line(&mut conn_a, &mut reader_a, &frame_line(session_id, seq));
        assert_eq!(f["type"], "frame_ack", "frame {seq} should land: {f}");
        assert_eq!(f["frame_count"], seq + 1);
    }

    // --- 3. Drop the connection mid-session, with no close ---
    // `drop` closes the socket, so the daemon's reader sees EOF and runs its
    // connection-drop path.
    drop(reader_a);
    drop(conn_a);

    // The drop is observed by the wire server on its own thread, so this is a
    // race with a background handler. Poll over the wire rather than reading
    // coordinator state: `frame_sessions_mut` is crate-private, and widening it
    // for a test would be a worse trade than proving the same property the way
    // a client observes it.
    //
    // A `session_resume` reports `was_disconnected`, which is exactly the
    // signal that the daemon's drop path ran. Polling the resume is safe to
    // repeat because resume is idempotent.
    let mut probe = frame_exchange(&b.addr);
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let resumed = loop {
        let attempt = probe(&format!(
            r#"{{"type":"session_resume","session_id":{session_id}}}"#
        ));
        if attempt["type"] == "session_resume_ack" && attempt["was_disconnected"] == true {
            break attempt;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the daemon should mark session {session_id} disconnected within 5s of the \
             drop; last resume attempt was: {attempt}"
        );
        std::thread::sleep(Duration::from_millis(20));
    };

    // The disconnect policy, observed from outside the daemon: the session was
    // NOT destroyed by the drop, and its accounting is intact and carried
    // forward to the client.
    assert_eq!(resumed["status"], "ok");
    assert_eq!(resumed["session_id"], session_id);
    assert_eq!(resumed["client_id"], "teardown-client", "its own session");
    assert_eq!(
        resumed["last_seq"], 2,
        "the daemon had accepted seq 0..2 before the drop: {resumed}"
    );
    assert_eq!(resumed["frame_count"], 3);
    assert_eq!(resumed["frames_received"], 3 * 12_288);

    // --- 4. Reconnect properly and continue the session ---
    let mut conn_b = TcpStream::connect(&b.addr).expect("reconnect B");
    conn_b.set_nodelay(true).expect("set TCP_NODELAY");
    conn_b
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("set read timeout");
    let mut reader_b = BufReader::new(conn_b.try_clone().expect("clone B"));

    // Resuming again from the new connection is idempotent: the session was
    // already re-attached by the probe, and the client that lost the first
    // response must be able to retry.
    let resumed = exchange_line(
        &mut conn_b,
        &mut reader_b,
        &format!(r#"{{"type":"session_resume","session_id":{session_id}}}"#),
    );
    assert_eq!(
        resumed["type"], "session_resume_ack",
        "resume must succeed over a real daemon: {resumed}"
    );
    assert_eq!(resumed["status"], "ok");
    assert_eq!(
        resumed["was_disconnected"], false,
        "already re-attached by the probe, so no new drop was observed: {resumed}"
    );

    // --- 5. Continue streaming from the next sequence number ---
    let next = exchange_line(&mut conn_b, &mut reader_b, &frame_line(session_id, 3));
    assert_eq!(next["type"], "frame_ack", "got: {next}");
    assert_eq!(next["status"], "ok");
    assert_eq!(
        next["frame_count"], 4,
        "sequencing continued across the resume: {next}"
    );
    assert_eq!(
        next["frames_received"],
        4 * 12_288,
        "byte accounting continued too: {next}"
    );
    assert_eq!(next["last_seq"], 3);

    // --- 6. And it can still be closed cleanly from the new connection ---
    // Teardown semantics must keep working across a reconnect: this is a
    // session the client resumed, and closing it must report the post-resume
    // counters and be final.
    let closed = exchange_line(
        &mut conn_b,
        &mut reader_b,
        &format!(r#"{{"type":"session_close","session_id":{session_id}}}"#),
    );
    assert_eq!(closed["type"], "session_close_ack", "got: {closed}");
    assert_eq!(closed["reason"], "closed");
    assert_eq!(
        closed["frame_count"], 4,
        "the post-resume frames are counted by the close: {closed}"
    );

    // --- 7. A closed session is not resumable, over the real wire ---
    let after_close = exchange_line(
        &mut conn_b,
        &mut reader_b,
        &format!(r#"{{"type":"session_resume","session_id":{session_id}}}"#),
    );
    assert_eq!(after_close["type"], "frame_error", "got: {after_close}");
    assert_eq!(after_close["error"], "session_gone");
    assert_eq!(after_close["reason"], "closed");

    stop_daemon(b);
}

/// A session that was closed can never be resumed, even by a client that
/// reconnects afterwards. Teardown is final.
#[test]
fn session_resume_after_close_over_real_daemon_is_rejected() {
    let (_a, b) = start_two_daemons();
    let mut exchange = frame_exchange(&b.addr);

    let ack = exchange(INIT_LINE);
    let session_id = ack["session_id"].as_u64().expect("session_id");
    assert_eq!(exchange(&frame_line(session_id, 0))["status"], "ok");
    assert_eq!(
        exchange(&format!(
            r#"{{"type":"session_close","session_id":{session_id}}}"#
        ))["status"],
        "ok"
    );

    // A fresh connection tries to resume the dead id.
    let mut reconnect = frame_exchange(&b.addr);
    let resumed = reconnect(&format!(
        r#"{{"type":"session_resume","session_id":{session_id}}}"#
    ));

    assert_eq!(resumed["type"], "frame_error", "got: {resumed}");
    assert_eq!(
        resumed["error"], "session_gone",
        "a closed session must not be resumable: {resumed}"
    );
    assert_eq!(resumed["reason"], "closed");

    // And the refusal created nothing.
    let health = reconnect(r#"{"type":"health_check"}"#);
    assert_eq!(health["status"], "healthy", "the daemon is unharmed");

    stop_daemon(b);
}

/// A resume naming an id this daemon never issued is rejected cleanly over the
/// wire, with no half-open state left behind.
#[test]
fn session_resume_with_unknown_id_over_real_daemon_is_rejected() {
    let (_a, b) = start_two_daemons();
    let mut exchange = frame_exchange(&b.addr);

    // A real session exists, so "wrong id" is not conflated with "no sessions".
    let ack = exchange(INIT_LINE);
    let session_id = ack["session_id"].as_u64().expect("session_id");

    let bogus = exchange(r#"{"type":"session_resume","session_id":9999}"#);
    assert_eq!(bogus["type"], "frame_error", "got: {bogus}");
    assert_eq!(bogus["error"], "unknown_session");
    assert_eq!(bogus["session_id"], 9999);

    // The real session is entirely unaffected by the refused resume.
    let live = exchange(&frame_line(session_id, 0));
    assert_eq!(live["type"], "frame_ack", "got: {live}");
    assert_eq!(live["frame_count"], 1, "counters did not shift: {live}");

    // A resume with no session id at all is a validation error, not a crash.
    let missing = exchange(r#"{"type":"session_resume"}"#);
    assert_eq!(missing["type"], "validation_error", "got: {missing}");

    stop_daemon(b);
}

/// Send one line and read exactly one response line over `stream`.
///
/// A local helper rather than the `frame_exchange` closure because these tests
/// need the raw socket back to drop it deliberately mid-session, and a closure
/// that owns the socket cannot be dropped on its own.
fn exchange_line(
    stream: &mut TcpStream,
    reader: &mut BufReader<TcpStream>,
    request: &str,
) -> serde_json::Value {
    stream
        .write_all(request.as_bytes())
        .and_then(|_| stream.write_all(b"\n"))
        .and_then(|_| stream.flush())
        .expect("send request");
    let mut line = String::new();
    let read = reader.read_line(&mut line).expect("read response");
    assert_ne!(read, 0, "daemon closed the connection");
    serde_json::from_str(line.trim()).unwrap_or_else(|e| panic!("bad JSON ({e}): {line}"))
}
