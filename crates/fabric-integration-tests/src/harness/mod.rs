//! Two-node test harness for Fabric integration tests.
//!
//! Provides helpers for starting in-process daemon instances with
//! separate SQLite databases and random TCP ports, suitable for
//! testing federation, frame streaming, and multi-node scenarios.

pub mod federation;
pub mod frame_streamer;

use fabric_daemon::auth::middleware::AuthMiddlewareConfig;
use fabric_daemon::auth::AuthMiddleware;
use fabric_daemon::config::{DaemonConfig, DatabaseConfig, ServerConfig};
use fabric_daemon::coordinator::Coordinator;
use fabric_daemon::wire::run_wire_server;
use fabric_graph::model::{Edge, EdgeId, Node, NodeId, Topology};
use fabric_graph::LocalityTier;
use std::net::TcpListener;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// DaemonInstance
// ---------------------------------------------------------------------------

/// A running daemon instance with coordinator, wire server, and shutdown flag.
pub struct DaemonInstance {
    /// The coordinator (shared with the wire server thread).
    pub coordinator: Arc<Coordinator>,
    /// TCP address the wire server is bound to (e.g. "127.0.0.1:54321").
    pub addr: String,
    /// Shutdown flag (triggers wire server exit when set).
    pub shutdown: Arc<AtomicBool>,
    /// Handle to the wire server thread.
    pub _server_thread: JoinHandle<()>,
    /// Temp directory (kept alive for the duration of the instance).
    pub _dir: tempfile::TempDir,
}

impl DaemonInstance {
    /// Request graceful shutdown and join the wire server thread.
    pub fn stop(self) {
        self.coordinator.shutdown();
        // Give the wire server a moment to notice the flag.
        thread::sleep(Duration::from_millis(100));
        // Drop the coordinator clone inside the thread (it joins below).
        drop(self._server_thread.join());
    }
}

// ---------------------------------------------------------------------------
// start_daemon
// ---------------------------------------------------------------------------

/// Start a single daemon on a random port with a fresh temp database.
///
/// The wire server runs on a background thread and is ready once
/// [`wait_for_daemon`] succeeds against the returned address.
pub fn start_daemon(federation_id: &str) -> DaemonInstance {
    let dir = tempfile::tempdir().expect("create temp dir");
    let db_path = dir.path().join(format!("{federation_id}.db"));

    let config = DaemonConfig {
        database: DatabaseConfig {
            path: db_path,
            ..Default::default()
        },
        server: ServerConfig {
            listen: "127.0.0.1:0".into(),
            max_connections: 16,
            request_timeout_ms: 5000,
        },
        ..Default::default()
    };

    let coordinator = Arc::new(Coordinator::new(config.clone()).expect("create coordinator"));

    // Bind to port 0 for a random available port.
    let listener =
        TcpListener::bind(&config.server.listen).expect("bind wire server to 127.0.0.1:0");
    let addr = listener.local_addr().expect("local_addr").to_string();

    let coord = coordinator.clone();
    let shutdown = coordinator.shutdown_flag();
    let flag = shutdown.clone();

    let server_thread = thread::spawn(move || {
        let auth = Arc::new(AuthMiddleware::new(AuthMiddlewareConfig::default()));
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("create tokio runtime"),
        );
        let _ = run_wire_server(listener, coord, 16, 5000, auth, runtime);
    });

    DaemonInstance {
        coordinator,
        addr,
        shutdown: flag,
        _server_thread: server_thread,
        _dir: dir,
    }
}

// ---------------------------------------------------------------------------
// start_two_daemons
// ---------------------------------------------------------------------------

/// Start two daemons wired as federation peers with distinct topology nodes.
///
/// - `a` gets federation_id `"a"` with node `"node_a"`
/// - `b` gets federation_id `"b"` with node `"node_b"`
///
/// Returns `(daemon_a, daemon_b)`.
pub fn start_two_daemons() -> (DaemonInstance, DaemonInstance) {
    let a = start_daemon("a");
    let b = start_daemon("b");

    // Wait for both to accept connections.
    wait_for_daemon(&a.addr, Duration::from_secs(5));
    wait_for_daemon(&b.addr, Duration::from_secs(5));

    // Set initial topologies so each daemon reports a distinct node.
    let topo_a = build_single_node_topology("node_a", LocalityTier::L6Lan);
    a.coordinator
        .set_topology(topo_a)
        .expect("set topology for a");

    let topo_b = build_single_node_topology("node_b", LocalityTier::L6Lan);
    b.coordinator
        .set_topology(topo_b)
        .expect("set topology for b");

    (a, b)
}

// ---------------------------------------------------------------------------
// wait_for_daemon
// ---------------------------------------------------------------------------

/// Block until the daemon at `addr` accepts TCP connections, or panic after
/// `timeout`.
pub fn wait_for_daemon(addr: &str, timeout: Duration) {
    let start = Instant::now();
    loop {
        match std::net::TcpStream::connect(addr) {
            Ok(_) => return,
            Err(_) if start.elapsed() < timeout => {
                thread::sleep(Duration::from_millis(50));
            }
            Err(e) => panic!("daemon at {addr} did not become ready within {timeout:?}: {e}"),
        }
    }
}

// ---------------------------------------------------------------------------
// stop_daemon
// ---------------------------------------------------------------------------

/// Stop a daemon instance by requesting shutdown and joining the wire server
/// thread. Consumes the instance.
pub fn stop_daemon(instance: DaemonInstance) {
    instance.stop();
}

/// Send a JSON line to the daemon and read the response line.
pub fn send_and_receive(addr: &str, message: &str) -> String {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    let mut stream = TcpStream::connect(addr).expect("connect to daemon");
    stream.set_nodelay(true).expect("set TCP_NODELAY");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("set read timeout");
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .expect("set write timeout");

    writeln!(stream, "{message}").expect("write message");
    stream.flush().expect("flush");

    let reader = BufReader::new(stream.try_clone().expect("clone stream"));
    let mut lines = reader.lines();
    lines
        .next()
        .expect("no response from daemon")
        .expect("io error reading response")
}

// ---------------------------------------------------------------------------
// Topology builders
// ---------------------------------------------------------------------------

/// Build a single-node topology (used internally by `start_two_daemons`).
fn build_single_node_topology(node_id: &str, tier: LocalityTier) -> Topology {
    let mut topo = Topology::new();
    topo.add_node(Node::new(NodeId::new(node_id), tier));
    topo
}

/// Build a 2-node topology: `node_a --L6Lan--> node_b`.
///
/// Useful for testing multihop route compilation and cross-node frame
/// streaming.
pub fn build_2node_topology() -> Topology {
    build_2node_topology_with_tier(
        LocalityTier::L6Lan,
        LocalityTier::L6Lan,
        LocalityTier::L6Lan,
    )
}

/// Build a 2-node topology with explicit locality tiers.
///
/// Creates `node_a` with `tier_a`, `node_b` with `tier_b`, and an edge
/// between them with `edge_tier`.
pub fn build_2node_topology_with_tier(
    tier_a: LocalityTier,
    tier_b: LocalityTier,
    edge_tier: LocalityTier,
) -> Topology {
    let mut topo = Topology::new();
    topo.add_node(Node::new(NodeId::new("node_a"), tier_a));
    topo.add_node(Node::new(NodeId::new("node_b"), tier_b));
    topo.add_edge(Edge::new(
        EdgeId::new("edge-a-b"),
        NodeId::new("node_a"),
        NodeId::new("node_b"),
        edge_tier,
    ))
    .expect("add edge to 2-node topology");
    topo
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_and_stop_daemon() {
        let daemon = start_daemon("test");
        wait_for_daemon(&daemon.addr, Duration::from_secs(5));

        // Verify the coordinator is healthy.
        let health = daemon.coordinator.health();
        assert_eq!(health.status, "healthy");

        stop_daemon(daemon);
    }

    #[test]
    fn start_two_daemons_both_healthy() {
        let (a, b) = start_two_daemons();

        let ha = a.coordinator.health();
        let hb = b.coordinator.health();
        assert_eq!(ha.status, "healthy");
        assert_eq!(hb.status, "healthy");

        // Different addresses.
        assert_ne!(a.addr, b.addr);

        stop_daemon(a);
        stop_daemon(b);
    }

    #[test]
    fn build_2node_topology_has_two_nodes_and_one_edge() {
        let topo = build_2node_topology();
        assert_eq!(topo.nodes.len(), 2);
        assert_eq!(topo.edges.len(), 1);
        assert!(topo.nodes.contains_key(&NodeId::new("node_a")));
        assert!(topo.nodes.contains_key(&NodeId::new("node_b")));
    }
}
