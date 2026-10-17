//! Two-node integration tests for the Fabric daemon (core).
//!
//! Tests 1-5 cover basic two-node scenarios: health check, topology exchange,
//! multihop compile, frame streaming, and heartbeat round-trip.
//!
//! Advanced tests (6-10) are in `two_node_advanced.rs`.
//!
//! Run all: `cargo test -p fabric-integration-tests --test two_node --test two_node_advanced`

use fabric_daemon::federation::MergeStrategy;
use fabric_frame_transport::Codec;
use fabric_graph::model::NodeId;
use fabric_graph::multihop::builtin_stages;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

use fabric_integration_tests::harness::federation::{
    assert_federation_ids, assert_merged_topology, assert_min_epoch, exchange_and_merge,
};
use fabric_integration_tests::harness::frame_streamer::stream_frames_between;
use fabric_integration_tests::harness::{
    build_2node_topology, send_and_receive, start_two_daemons, stop_daemon,
};
use fabric_integration_tests::make_intent;

// ===========================================================================
// Test 1: Two-Node Health Check
// ===========================================================================

/// Both daemons start, accept TCP connections, and report healthy.
#[test]
fn two_node_health_check() {
    let (a, b) = start_two_daemons();

    // Query health over the real TCP wire protocol.
    let msg = r#"{"type":"health_check"}"#;
    let resp_a = send_and_receive(&a.addr, msg);
    let resp_b = send_and_receive(&b.addr, msg);

    let parsed_a: serde_json::Value = serde_json::from_str(&resp_a).expect("parse health_a");
    let parsed_b: serde_json::Value = serde_json::from_str(&resp_b).expect("parse health_b");

    assert_eq!(parsed_a["status"], "healthy");
    assert_eq!(parsed_b["status"], "healthy");

    // Verify uptime is reported (always >= 0).
    assert!(parsed_a["uptime_s"].as_u64().is_some());
    assert!(parsed_b["uptime_s"].as_u64().is_some());

    // Verify topology_epoch is reported.
    assert!(parsed_a["topology_epoch"].as_u64().is_some());
    assert!(parsed_b["topology_epoch"].as_u64().is_some());

    // Different ports.
    assert_ne!(a.addr, b.addr);

    stop_daemon(a);
    stop_daemon(b);
}

// ===========================================================================
// Test 2: Two-Node Topology Exchange
// ===========================================================================

/// B fetches A's topology via sync_topology(), verify snapshot fields and
/// that merging produces the expected node/edge counts.
#[test]
fn two_node_topology_exchange() {
    let (a, b) = start_two_daemons();

    // Give daemon_a a full 2-node topology with an edge.
    let topo = build_2node_topology();
    a.coordinator
        .set_topology(topo)
        .expect("set topology for a");

    // B fetches A's topology over real TCP.
    let snapshot = fabric_daemon::federation::sync_topology(&a.addr)
        .expect("sync_topology to a should succeed");

    // The snapshot should contain node data from A.
    assert!(
        snapshot.node_count >= 1,
        "expected at least 1 node in snapshot from A, got {}",
        snapshot.node_count,
    );

    // Merge: A (node_a + node_b + edge) + B (node_b) => 3 nodes (peer prefixed), 1 edge.
    // Peer node_b is prefixed with federation_id to avoid ID collisions.
    let merged = exchange_and_merge(&a, &b, &MergeStrategy::MergeAll);
    assert_merged_topology(&merged, 3, 1);
    // federation_ids comes from peer snapshots only (local nodes have empty federation_id).
    assert_federation_ids(&merged, 1);

    // max_epoch should be >= 0 (baseline check).
    assert_min_epoch(&merged, 0);

    stop_daemon(a);
    stop_daemon(b);
}

// ===========================================================================
// Test 3: Two-Node Multihop Compile
// ===========================================================================

/// Compile a route from node_a to node_b using the 2-node topology.
/// Verify the plan has 2 steps (execute + receive).
#[test]
fn two_node_multihop_compile() {
    let (a, _b) = start_two_daemons();

    // Set a 2-node topology on daemon A.
    let topo = build_2node_topology();
    a.coordinator
        .set_topology(topo.clone())
        .expect("set topology");

    let intent = make_intent();
    let catalog = builtin_stages();

    let result = fabric_graph::multihop::compile_multihop(
        &topo,
        &NodeId::new("node_a"),
        &NodeId::new("node_b"),
        &intent,
        &catalog,
    )
    .expect("compile_multihop should succeed");

    // 2-node direct path: 2 steps (execute at source, receive at destination).
    assert_eq!(
        result.primary.steps.len(),
        2,
        "expected 2 steps, got {}",
        result.primary.steps.len(),
    );

    // Steps should be execute + receive.
    assert_eq!(result.primary.steps[0].action, "execute");
    assert_eq!(result.primary.steps[1].action, "receive");

    // Nodes should match our topology.
    assert_eq!(result.primary.steps[0].node, NodeId::new("node_a"));
    assert_eq!(result.primary.steps[1].node, NodeId::new("node_b"));

    // Cost should be non-negative.
    assert!(
        result.cost.latency_us >= 0.0,
        "latency cost should be >= 0, got {}",
        result.cost.latency_us,
    );

    // Stages per hop should be populated.
    assert_eq!(
        result.stages_per_hop.len(),
        1,
        "1 hop (direct edge) should produce 1 set of stages"
    );

    stop_daemon(a);
}

// ===========================================================================
// Test 4: Two-Node Frame Streaming
// ===========================================================================

/// Stream synthetic RGBA frames from a client to daemon B's wire server.
/// Verify all frames are sent and the session completes without error.
#[test]
fn two_node_frame_streaming() {
    let (_a, b) = start_two_daemons();

    let width = 64;
    let height = 48;
    let frame_count = 10;

    let result = stream_frames_between(
        &b.addr,
        "client-test",
        width,
        height,
        frame_count,
        Codec::Rgba,
    )
    .expect("stream_frames_between should succeed");

    assert_eq!(
        result.frames_sent, frame_count,
        "expected {} frames sent, got {}",
        frame_count, result.frames_sent,
    );

    // All frames should have been acknowledged (local ack in current impl).
    assert_eq!(
        result.frames_acknowledged, frame_count,
        "expected {} frames acknowledged, got {}",
        frame_count, result.frames_acknowledged,
    );

    // Latency is deliberately NOT asserted here.
    //
    // `avg_rtt_us` is wall-clock time around a blocking `read_line` that
    // waits for the server's rejection response (see
    // `harness/frame_streamer.rs`), not a true round-trip: the harness
    // acknowledges locally and the code itself notes a real RTT needs the
    // wire server to echo frames. That makes it a measurement of host load,
    // not of this code's correctness.
    //
    // Measured on an idle machine over 15 runs: min 107 us, median 383 us,
    // p90 797 us, max 5478 us. The previous `< 10_000` bound sat barely
    // above that idle maximum, so any concurrent load failed the test for
    // reasons unrelated to the code under test. Observed flaking was 2/8 on
    // the pre-change base commit 4796706, with none of the auth work present.
    //
    // The properties that ARE worth guarding are asserted above: every frame
    // sent, every frame acknowledged, and non-zero elapsed time. Those are
    // deterministic and they are what this test is actually for.

    // Session elapsed time should be non-zero (at least some wall time).
    assert!(result.elapsed > Duration::ZERO, "elapsed should be > 0");

    stop_daemon(b);
}

// ===========================================================================
// Test 5: Two-Node Heartbeat Round-Trip
// ===========================================================================

/// Send heartbeats to both daemons over real TCP connections.
/// Verify each responds with heartbeat_ack.
#[test]
fn two_node_heartbeat_roundtrip() {
    let (a, b) = start_two_daemons();

    let msg = r#"{"type":"heartbeat"}"#;

    let resp_a = send_and_receive(&a.addr, msg);
    let resp_b = send_and_receive(&b.addr, msg);

    // Parse both responses.
    let parsed_a: serde_json::Value =
        serde_json::from_str(&resp_a).expect("parse heartbeat ack from a");
    let parsed_b: serde_json::Value =
        serde_json::from_str(&resp_b).expect("parse heartbeat ack from b");

    assert_eq!(parsed_a["type"], "heartbeat_ack");
    assert_eq!(parsed_a["status"], "ok");

    assert_eq!(parsed_b["type"], "heartbeat_ack");
    assert_eq!(parsed_b["status"], "ok");

    // Verify both daemons are still healthy after heartbeats.
    let health_a = send_and_receive(&a.addr, r#"{"type":"health_check"}"#);
    let health_b = send_and_receive(&b.addr, r#"{"type":"health_check"}"#);

    let ha: serde_json::Value = serde_json::from_str(&health_a).unwrap();
    let hb: serde_json::Value = serde_json::from_str(&health_b).unwrap();
    assert_eq!(ha["status"], "healthy");
    assert_eq!(hb["status"], "healthy");

    stop_daemon(a);
    stop_daemon(b);
}

// ===========================================================================
// Test 6: Real Frame Round-Trip Through a Real Daemon
// ===========================================================================

/// A single frame must survive a full round trip through a real daemon:
/// `session_init` -> `session_ack`, `frame_data` -> `frame_ack`,
/// `frame_ack` -> `frame_ack_response`.
///
/// This is the A2 gate. Before the frame dispatcher landed, `session_init`
/// and `frame_data` were absent from `REQUEST_MESSAGE_TYPES`, so validation
/// rejected them with `UNKNOWN_TYPE` and this path did not exist. The test
/// asserts on the daemon's actual responses rather than on the mere fact that
/// bytes reached it, because an ignored response is what hid the gap.
#[test]
fn frame_round_trip_through_real_daemon() {
    let (_a, b) = start_two_daemons();

    let mut stream = TcpStream::connect(&b.addr).expect("connect to daemon b wire server");
    stream.set_nodelay(true).expect("set TCP_NODELAY");
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .expect("set read timeout");

    let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));

    // Send one request line and read exactly one response line.
    let mut exchange = |request: &str| -> serde_json::Value {
        stream.write_all(request.as_bytes()).expect("write request");
        stream.write_all(b"\n").expect("write newline");
        stream.flush().expect("flush request");

        let mut line = String::new();
        let read = reader.read_line(&mut line).expect("read response");
        assert_ne!(read, 0, "daemon closed the connection");
        serde_json::from_str(line.trim())
            .unwrap_or_else(|e| panic!("response was not JSON ({e}): {line}"))
    };

    // --- 1. Open a session ---
    let ack = exchange(
        r#"{"type":"session_init","version":1,"preferred_codec":"Rgba","width":64,"height":48,"target_fps":60,"max_latency_ms":33,"client_id":"round-trip-client"}"#,
    );
    assert_eq!(ack["type"], "session_ack", "got: {ack}");
    let session_id = ack["session_id"]
        .as_u64()
        .unwrap_or_else(|| panic!("session_ack carried no session_id: {ack}"));
    assert_eq!(session_id, 1, "first session should be id 1");
    assert_eq!(ack["codec"], "Rgba");
    assert_eq!(ack["width"], 64);
    assert_eq!(ack["height"], 48);

    // --- 2. Send one real frame for that session ---
    // 64*48*4 = 12288 bytes of RGBA.
    let frame = exchange(&format!(
        r#"{{"type":"frame_data","session_id":{session_id},"seq":0,"pts_us":0,"dts_us":0,"is_keyframe":true,"codec":"Rgba","width":64,"height":48,"payload_len":12288,"duration_us":16667}}"#
    ));
    assert_eq!(frame["type"], "frame_ack", "got: {frame}");
    assert_eq!(frame["status"], "ok", "daemon rejected the frame: {frame}");
    assert_eq!(frame["seq"], 0);
    assert_eq!(frame["session_id"], session_id);
    assert_eq!(
        frame["frame_count"], 1,
        "daemon should count exactly one frame"
    );
    assert_eq!(
        frame["frames_received"], 12288,
        "daemon should account the frame's payload bytes"
    );
    assert_eq!(frame["is_keyframe"], true);

    // --- 3. Acknowledge the frame ---
    let ack_response = exchange(&format!(
        r#"{{"type":"frame_ack","session_id":{session_id},"seq":0,"rtt_us":1234,"recv_pts_us":0}}"#
    ));
    assert_eq!(
        ack_response["type"], "frame_ack_response",
        "got: {ack_response}"
    );
    assert_eq!(
        ack_response["acked"], true,
        "the daemon did receive frame 0, so the ack must be honored"
    );
    assert_eq!(ack_response["rtt_us"], 1234);
    assert_eq!(ack_response["recorded_rtt_us"], 1234);
    assert_eq!(ack_response["ack_count"], 1);

    // --- 4. A frame for an unknown session is reported, not dropped ---
    let orphan = exchange(
        r#"{"type":"frame_data","session_id":4242,"seq":0,"codec":"Rgba","payload_len":16}"#,
    );
    assert_eq!(orphan["type"], "frame_error", "got: {orphan}");
    assert_eq!(
        orphan["error"], "unknown_session",
        "an unknown session must be an explicit error"
    );
    assert_eq!(orphan["session_id"], 4242);
    let active = orphan["active_sessions"]
        .as_array()
        .expect("error should list active sessions");
    assert!(
        active.contains(&serde_json::json!(session_id)),
        "error should name the live session: {orphan}"
    );

    stop_daemon(b);
}

/// Server-side state must persist across separate connections.
///
/// Session state lives on the coordinator, not on the socket, so a client can
/// reconnect and keep streaming into the same session. If the state were
/// per-connection, the second connection's frames would fail with
/// `unknown_session`.
#[test]
fn frame_session_state_survives_reconnection() {
    let (_a, b) = start_two_daemons();

    let session_id = {
        let mut stream = TcpStream::connect(&b.addr).expect("connect");
        stream.set_nodelay(true).expect("set TCP_NODELAY");
        let mut reader = BufReader::new(stream.try_clone().expect("clone"));
        stream
            .write_all(
                br#"{"type":"session_init","version":1,"preferred_codec":"Av1","width":32,"height":32,"target_fps":30,"max_latency_ms":33,"client_id":"reconnect"}"#,
            )
            .and_then(|_| stream.write_all(b"\n"))
            .and_then(|_| stream.flush())
            .expect("send session_init");
        let mut line = String::new();
        reader.read_line(&mut line).expect("read session_ack");
        let ack: serde_json::Value = serde_json::from_str(line.trim()).expect("parse ack");
        assert_eq!(ack["codec"], "Av1");
        ack["session_id"].as_u64().expect("session_id")
    };

    // Open a brand-new connection and keep using the session from it.
    let mut stream = TcpStream::connect(&b.addr).expect("reconnect");
    stream.set_nodelay(true).expect("set TCP_NODELAY");
    let mut reader = BufReader::new(stream.try_clone().expect("clone"));
    let frame = exchange_frame(&mut stream, &mut reader, session_id, 0);
    assert_eq!(
        frame["type"], "frame_ack",
        "session state did not survive the reconnect: {frame}"
    );
    assert_eq!(frame["frame_count"], 1);

    stop_daemon(b);
}

/// Send one frame line and parse the single response line.
fn exchange_frame(
    stream: &mut TcpStream,
    reader: &mut BufReader<TcpStream>,
    session_id: u64,
    seq: u64,
) -> serde_json::Value {
    let request = format!(
        r#"{{"type":"frame_data","session_id":{session_id},"seq":{seq},"codec":"Av1","width":32,"height":32,"payload_len":4096}}"#
    );
    stream
        .write_all(request.as_bytes())
        .and_then(|_| stream.write_all(b"\n"))
        .and_then(|_| stream.flush())
        .expect("send frame_data");
    let mut line = String::new();
    let read = reader.read_line(&mut line).expect("read frame_ack");
    assert_ne!(read, 0, "daemon closed the connection");
    serde_json::from_str(line.trim()).unwrap_or_else(|e| panic!("bad JSON ({e}): {line}"))
}

// ===========================================================================
// Test 8: session_close over a real daemon
// ===========================================================================

/// Open a connection to `addr` and return a request/response exchange closure.
///
/// Only the closure is returned: it borrows the connection it wrote to, and
/// no caller in these tests needs the raw socket, so handing it back would
/// only invite using it while the closure still owns it.
fn frame_exchange(addr: &str) -> impl FnMut(&str) -> serde_json::Value {
    let mut stream = TcpStream::connect(addr).expect("connect to daemon wire server");
    stream.set_nodelay(true).expect("set TCP_NODELAY");
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
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

const INIT_LINE: &str = r#"{"type":"session_init","version":1,"preferred_codec":"Rgba","width":64,"height":48,"target_fps":60,"max_latency_ms":33,"client_id":"teardown-client"}"#;

fn frame_line(session_id: u64, seq: u64) -> String {
    format!(
        r#"{{"type":"frame_data","session_id":{session_id},"seq":{seq},"pts_us":{seq},"dts_us":{seq},"is_keyframe":true,"codec":"Rgba","width":64,"height":48,"payload_len":12288,"duration_us":16667}}"#
    )
}

/// A full `session_init` -> frames -> `session_close` cycle over real TCP.
///
/// Asserts on the daemon's actual responses, including that a frame sent
/// after the close is rejected with `session_gone` rather than silently
/// accepted.
#[test]
fn session_close_over_real_daemon() {
    let (_a, b) = start_two_daemons();
    let mut exchange = frame_exchange(&b.addr);

    // --- Open ---
    let ack = exchange(INIT_LINE);
    assert_eq!(ack["type"], "session_ack", "got: {ack}");
    let session_id = ack["session_id"].as_u64().expect("session_id");

    // --- Stream a couple of frames so the close has counters to report ---
    let f0 = exchange(&frame_line(session_id, 0));
    assert_eq!(f0["type"], "frame_ack", "got: {f0}");
    let f1 = exchange(&frame_line(session_id, 1));
    assert_eq!(f1["frame_count"], 2);

    // --- Close ---
    let closed = exchange(&format!(
        r#"{{"type":"session_close","session_id":{session_id}}}"#
    ));
    assert_eq!(closed["type"], "session_close_ack", "got: {closed}");
    assert_eq!(closed["status"], "ok");
    assert_eq!(closed["reason"], "closed");
    assert_eq!(closed["session_id"], session_id);
    assert_eq!(
        closed["frame_count"], 2,
        "final counters are reported: {closed}"
    );
    assert_eq!(closed["frames_received"], 24_576);

    // --- A frame after the close is rejected cleanly ---
    let after = exchange(&frame_line(session_id, 2));
    assert_eq!(after["type"], "frame_error", "got: {after}");
    assert_eq!(
        after["error"], "session_gone",
        "a frame for a closed session must be an explicit error: {after}"
    );
    assert_eq!(after["reason"], "closed");

    // --- The daemon is still healthy and can host a new session ---
    let health = exchange(r#"{"type":"health_check"}"#);
    assert_eq!(health["status"], "healthy");
    let reack = exchange(INIT_LINE);
    assert_eq!(reack["type"], "session_ack");
    assert_ne!(
        reack["session_id"], session_id,
        "a closed id must not be reused"
    );

    stop_daemon(b);
}

/// Closing twice over the wire is reported, not accepted as a second success.
#[test]
fn session_close_twice_over_real_daemon() {
    let (_a, b) = start_two_daemons();
    let mut exchange = frame_exchange(&b.addr);

    let ack = exchange(INIT_LINE);
    let session_id = ack["session_id"].as_u64().expect("session_id");
    let close = format!(r#"{{"type":"session_close","session_id":{session_id}}}"#);

    assert_eq!(exchange(&close)["status"], "ok");
    let second = exchange(&close);
    assert_eq!(second["type"], "frame_error", "got: {second}");
    assert_eq!(second["error"], "session_gone");

    stop_daemon(b);
}

// ===========================================================================
// Test 9: session_cancel over a real daemon
// ===========================================================================

/// A session can be cancelled mid-transfer: the session stops being active,
/// its partial state is released, and the next frame for it fails cleanly.
#[test]
fn session_cancel_mid_transfer_over_real_daemon() {
    let (_a, b) = start_two_daemons();
    let mut exchange = frame_exchange(&b.addr);

    let ack = exchange(INIT_LINE);
    assert_eq!(ack["type"], "session_ack", "got: {ack}");
    let session_id = ack["session_id"].as_u64().expect("session_id");

    // --- Start the transfer, then abort part-way through ---
    for seq in 0..3 {
        let f = exchange(&frame_line(session_id, seq));
        assert_eq!(f["type"], "frame_ack", "frame {seq} should land: {f}");
    }

    let cancelled = exchange(&format!(
        r#"{{"type":"session_cancel","session_id":{session_id}}}"#
    ));
    assert_eq!(cancelled["type"], "session_close_ack", "got: {cancelled}");
    assert_eq!(cancelled["status"], "ok");
    assert_eq!(
        cancelled["reason"], "cancelled",
        "cancel must report its own reason, distinct from a close: {cancelled}"
    );
    assert_eq!(cancelled["frame_count"], 3, "partial progress is reported");

    // --- The session is no longer active ---
    let after = exchange(&frame_line(session_id, 3));
    assert_eq!(after["type"], "frame_error", "got: {after}");
    assert_eq!(
        after["error"], "session_gone",
        "a frame for a cancelled session must fail cleanly: {after}"
    );
    assert_eq!(after["reason"], "cancelled");

    // --- The daemon survives and can serve a new session ---
    let reack = exchange(INIT_LINE);
    assert_eq!(reack["type"], "session_ack", "got: {reack}");
    let new_session = reack["session_id"].as_u64().expect("session_id");
    let fresh = exchange(&frame_line(new_session, 0));
    assert_eq!(
        fresh["type"], "frame_ack",
        "a new session must start from zero: {fresh}"
    );
    assert_eq!(
        fresh["frame_count"], 1,
        "the new session's counters must not inherit the cancelled one's"
    );

    stop_daemon(b);
}

/// Cancelling one session leaves other sessions untouched.
#[test]
fn session_cancel_leaves_other_sessions_live() {
    let (_a, b) = start_two_daemons();
    let mut exchange = frame_exchange(&b.addr);

    let first = exchange(INIT_LINE);
    let first_id = first["session_id"].as_u64().expect("session_id");

    let second_init = INIT_LINE.replace("teardown-client", "second-client");
    let second = exchange(&second_init);
    let second_id = second["session_id"].as_u64().expect("session_id");
    assert_ne!(first_id, second_id);

    exchange(&format!(
        r#"{{"type":"session_cancel","session_id":{first_id}}}"#
    ));

    // The untouched session still carries frames.
    let live = exchange(&frame_line(second_id, 0));
    assert_eq!(
        live["type"], "frame_ack",
        "cancelling session {first_id} must not affect session {second_id}: {live}"
    );
    assert_eq!(live["session_id"], second_id);

    stop_daemon(b);
}
