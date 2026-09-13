//! End-to-end smoke tests for the fabric-daemon wire protocol.
//!
//! Starts the daemon in-process on a random TCP port, sends wire protocol
//! messages, and verifies the responses.
//!
//! Run with: `cargo test -p fabric-integration-tests --test e2e_smoke`

use fabric_daemon::config::DaemonConfig;
use fabric_daemon::coordinator::Coordinator;
use fabric_daemon::wire::run_wire_server;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Start the daemon wire server on a random port, return (coordinator, addr,
/// shutdown_flag).
fn start_daemon() -> (Arc<Coordinator>, String, Arc<AtomicBool>) {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("e2e_smoke.db");

    let config = DaemonConfig {
        database: fabric_daemon::config::DatabaseConfig {
            path: db_path,
            ..Default::default()
        },
        server: fabric_daemon::config::ServerConfig {
            listen: "127.0.0.1:0".into(), // random port
            max_connections: 16,
            request_timeout_ms: 5000,
        },
        ..Default::default()
    };

    let coordinator = Arc::new(Coordinator::new(config.clone()).unwrap());

    // Bind to port 0 to get a random available port.
    let listener = std::net::TcpListener::bind(&config.server.listen).unwrap();
    let addr = listener.local_addr().unwrap().to_string();

    let coord = coordinator.clone();
    let shutdown = coordinator.shutdown_flag();
    let flag = shutdown.clone();

    thread::spawn(move || {
        let _ = run_wire_server(listener, coord, 16, 5000);
        // Signal completion (not needed for tests, but clean).
    });

    (coordinator, addr, flag)
}

/// Send a raw line to the daemon and read the response line.
fn send_and_receive(addr: &str, message: &str) -> String {
    let mut stream = TcpStream::connect(addr).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .unwrap();

    // Send message with newline terminator.
    write!(stream, "{}\n", message).unwrap();
    stream.flush().unwrap();

    // Read response line.
    let reader = BufReader::new(stream.try_clone().unwrap());
    let mut lines = reader.lines();
    lines
        .next()
        .expect("no response from daemon")
        .expect("io error reading response")
}

/// Wait until the daemon is accepting connections (retry loop).
fn wait_for_daemon(addr: &str, max_wait: Duration) {
    let start = Instant::now();
    loop {
        match TcpStream::connect(addr) {
            Ok(_) => return,
            Err(_) if start.elapsed() < max_wait => {
                thread::sleep(Duration::from_millis(50));
            }
            Err(e) => panic!("daemon did not become ready: {e}"),
        }
    }
}

// ===========================================================================
// Smoke tests
// ===========================================================================

#[test]
fn e2e_daemon_health_check() {
    let (_coord, addr, _flag) = start_daemon();
    wait_for_daemon(&addr, Duration::from_secs(5));

    let msg = r#"{"type":"health_check"}"#;
    let resp = send_and_receive(&addr, msg);

    let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(parsed["status"], "healthy");
    assert!(parsed["uptime_s"].as_u64().is_some());
    assert!(parsed["topology_epoch"].as_u64().is_some());
    assert!(parsed["active_leases"].as_u64().is_some());
    assert!(parsed["active_plans"].as_u64().is_some());
}

#[test]
fn e2e_daemon_topology_request() {
    let (coord, addr, _flag) = start_daemon();
    wait_for_daemon(&addr, Duration::from_secs(5));

    // Add a node to the topology so we get meaningful data.
    let topo = fabric_graph::builder::TopologyBuilder::new()
        .with_name("e2e-smoke")
        .add_simple_node("smoke-node-1", fabric_graph::LocalityTier::L1SameNuma)
        .add_simple_node("smoke-node-2", fabric_graph::LocalityTier::L6Lan)
        .build();
    coord.set_topology(topo).unwrap();

    let msg = r#"{"type":"topology_request"}"#;
    let resp = send_and_receive(&addr, msg);

    let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(parsed["type"], "probe_response");
    assert_eq!(parsed["topology_name"], "e2e-smoke");
    assert_eq!(parsed["node_count"], 2);
    assert!(parsed["nodes"].as_array().unwrap().len() == 2);
}

#[test]
fn e2e_daemon_routes_request() {
    let (_coord, addr, _flag) = start_daemon();
    wait_for_daemon(&addr, Duration::from_secs(5));

    let msg = r#"{"type":"routes_request"}"#;
    let resp = send_and_receive(&addr, msg);

    let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(parsed["type"], "routes_response");
    assert!(parsed["routes"].as_array().is_some());
}

#[test]
fn e2e_daemon_invalid_message() {
    let (_coord, addr, _flag) = start_daemon();
    wait_for_daemon(&addr, Duration::from_secs(5));

    // Send non-JSON data.
    let resp = send_and_receive(&addr, "not valid json at all");
    assert!(resp.contains("INVALID_JSON") || resp.contains("invalid_json"));
}

#[test]
fn e2e_daemon_unknown_message_type() {
    let (_coord, addr, _flag) = start_daemon();
    wait_for_daemon(&addr, Duration::from_secs(5));

    let msg = r#"{"type":"completely_unknown_type"}"#;
    let resp = send_and_receive(&addr, msg);
    assert!(resp.contains("UNKNOWN_TYPE") || resp.contains("unknown_message"));
}

#[test]
fn e2e_daemon_heartbeat() {
    let (_coord, addr, _flag) = start_daemon();
    wait_for_daemon(&addr, Duration::from_secs(5));

    let msg = r#"{"type":"heartbeat"}"#;
    let resp = send_and_receive(&addr, msg);

    let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(parsed["type"], "heartbeat_ack");
    assert_eq!(parsed["status"], "ok");
}

#[test]
fn e2e_daemon_capabilities_request() {
    let (_coord, addr, _flag) = start_daemon();
    wait_for_daemon(&addr, Duration::from_secs(5));

    let msg = r#"{"type":"capabilities_request"}"#;
    let resp = send_and_receive(&addr, msg);

    let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(parsed["type"], "capabilities_response");
    assert!(parsed["capabilities"].as_array().is_some());
}
