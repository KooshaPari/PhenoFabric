//! Regression test for the daemon health probe.
//!
//! # The defect
//!
//! `fabric-daemon health` used to read lines from the daemon socket in a
//! loop until EOF. The daemon's wire server deliberately KEEPS a
//! connection open so a client can send more messages over it, so EOF
//! never arrived. The probe printed the correct healthy JSON and then
//! blocked forever.
//!
//! That broke the Docker `HEALTHCHECK`, which runs exactly this command:
//! every health check hit the container timeout and killed a perfectly
//! healthy daemon, so the container was reported "unhealthy" and an
//! orchestrator would restart it in a loop. This shipped in v0.1.5.
//!
//! # What this test asserts
//!
//! The probe must TERMINATE. Not "print the right JSON" -- print the
//! right JSON *and exit*. A probe that prints correctly and then hangs is
//! exactly the bug, so the exit is the thing under test.
//!
//! Run with: `cargo test -p fabric-daemon --test health_probe`

use fabric_daemon::config::DaemonConfig;
use fabric_daemon::coordinator::Coordinator;
use fabric_daemon::wire::run_wire_server;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Start the daemon wire server on an ephemeral port.
///
/// Mirrors the harness in `fabric-integration-tests`' `daemon_lifecycle`
/// test: a real coordinator, a real listener, a real accept loop.
fn start_daemon() -> (Arc<Coordinator>, String, Arc<AtomicBool>) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("health_probe.db");

    let config = DaemonConfig {
        database: fabric_daemon::config::DatabaseConfig {
            path: db_path,
            ..Default::default()
        },
        server: fabric_daemon::config::ServerConfig {
            listen: "127.0.0.1:0".into(),
            callback_listen_addr: "127.0.0.1:0".into(),
            max_connections: 16,
            request_timeout_ms: 5000,
        },
        ..Default::default()
    };

    let coordinator = Arc::new(Coordinator::new(config.clone()).expect("new coordinator"));
    let listener = std::net::TcpListener::bind(&config.server.listen).expect("bind ephemeral");
    let addr = listener.local_addr().expect("local_addr").to_string();

    let coord = coordinator.clone();
    let shutdown = coordinator.shutdown_flag();

    thread::spawn(move || {
        let auth = Arc::new(fabric_daemon::auth::AuthMiddleware::new(
            fabric_daemon::auth::AuthMiddlewareConfig::default(),
        ));
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("build runtime"),
        );
        let _ = run_wire_server(listener, coord, 16, 5000, auth, runtime);
    });

    (coordinator, addr, shutdown)
}

/// Locate the `fabric-daemon` binary built by cargo.
///
/// Search order mirrors `fabric-integration-tests`' `locate_daemon_binary`:
/// cargo's `CARGO_BIN_EXE_*` first, then the target directory, then the
/// standard workspace layout.
fn locate_daemon_binary() -> Option<PathBuf> {
    fn exists(p: &std::path::Path) -> bool {
        p.exists() && p.is_file()
    }

    if let Ok(p) = std::env::var("CARGO_BIN_EXE_fabric-daemon") {
        let p = PathBuf::from(p);
        if exists(&p) {
            return Some(p);
        }
    }

    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            manifest
                .parent()
                .and_then(|p| p.parent())
                .map(|root| root.join("target"))
        })?;

    for profile in ["debug", "release"] {
        let candidate = target_dir.join(profile).join("fabric-daemon");
        if exists(&candidate) {
            return Some(candidate);
        }
    }
    None
}

/// Run the daemon's `health` subcommand against `addr` and return how long
/// it took to exit, plus its exit status and stdout.
///
/// The call is bounded: if the probe hangs (the v0.1.5 defect) the child
/// is killed and the elapsed time is reported, so the caller can assert
/// on the hang instead of blocking the test suite forever.
fn run_health_probe(binary: &PathBuf, addr: &str) -> (Duration, Option<i32>, String) {
    let start = Instant::now();
    let mut child = Command::new(binary)
        .arg("health")
        .arg("--connect")
        .arg(addr)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn fabric-daemon health");

    // Poll for exit rather than `wait()`, so a hanging probe is detected
    // and killed instead of blocking the test run indefinitely.
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        match child.try_wait().expect("try_wait") {
            Some(status) => {
                let elapsed = start.elapsed();
                let mut out = String::new();
                if let Some(mut stdout) = child.stdout.take() {
                    use std::io::Read;
                    let _ = stdout.read_to_string(&mut out);
                }
                return (elapsed, status.code(), out);
            }
            None => {
                if Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return (start.elapsed(), None, String::new());
                }
                thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// The health probe must exit on its own.
///
/// This is the regression test for the v0.1.5 defect: the probe used to
/// print healthy JSON and then block until it was killed, which made
/// every Docker health check time out.
#[test]
fn health_probe_terminates_instead_of_hanging() {
    let Some(binary) = locate_daemon_binary() else {
        panic!(
            "could not locate the fabric-daemon binary; build it first \
             (cargo build -p fabric-daemon)"
        );
    };

    let (_coordinator, addr, _shutdown) = start_daemon();

    // The daemon needs a moment to accept connections.
    let connect_deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if TcpStream::connect(&addr).is_ok() {
            break;
        }
        assert!(
            Instant::now() < connect_deadline,
            "daemon did not become ready on {addr}"
        );
        thread::sleep(Duration::from_millis(50));
    }

    let (elapsed, code, stdout) = run_health_probe(&binary, &addr);

    assert!(
        code.is_some(),
        "the health probe HUNG: it did not exit within 20s (elapsed {:.1}s). \
         The Docker HEALTHCHECK runs this command, so a hanging probe makes \
         a healthy container report as unhealthy.",
        elapsed.as_secs_f64()
    );

    assert_eq!(
        code,
        Some(0),
        "the health probe exited non-zero (elapsed {:.1}s). stdout: {:?}",
        elapsed.as_secs_f64(),
        stdout
    );

    assert!(
        stdout.contains("\"status\":\"healthy\""),
        "the health probe did not report a healthy daemon. stdout: {:?}",
        stdout
    );

    // The probe should be fast. It is a single request/response over
    // loopback; a slow exit means it is still streaming.
    assert!(
        elapsed < Duration::from_secs(10),
        "the health probe took {:.1}s to exit, which suggests it is waiting \
         for EOF instead of reading one response line",
        elapsed.as_secs_f64()
    );
}

/// The probe must fail (non-zero) when no daemon is listening.
///
/// A probe that reports "healthy" for a dead daemon is worse than one
/// that hangs: it lets a broken deployment look fine.
#[test]
fn health_probe_fails_when_no_daemon_is_listening() {
    let Some(binary) = locate_daemon_binary() else {
        panic!(
            "could not locate the fabric-daemon binary; build it first \
             (cargo build -p fabric-daemon)"
        );
    };

    // Bind and immediately drop, so the port is almost certainly free.
    let port = {
        let l = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
        l.local_addr().expect("local_addr").port()
    };

    let (elapsed, code, _) = run_health_probe(&binary, &format!("127.0.0.1:{port}"));

    assert!(
        code.is_some(),
        "the health probe hung against a dead daemon (elapsed {:.1}s)",
        elapsed.as_secs_f64()
    );
    assert_ne!(
        code,
        Some(0),
        "the health probe reported success against a daemon that is not listening"
    );
}

/// The daemon must answer a health check on the connection it keeps open.
///
/// This is the server-side half of the contract the probe depends on: the
/// daemon replies with a health response AND keeps the connection open.
/// A probe that waited for EOF would hang against this server, which is
/// why the response has to be a single line that a reader can take and
/// stop at.
#[test]
fn daemon_answers_health_check_on_a_kept_open_connection() {
    let (_coordinator, addr, _shutdown) = start_daemon();

    let connect_deadline = Instant::now() + Duration::from_secs(10);
    let stream = loop {
        match TcpStream::connect(&addr) {
            Ok(s) => break s,
            Err(_) => {
                assert!(
                    Instant::now() < connect_deadline,
                    "daemon did not become ready on {addr}"
                );
                thread::sleep(Duration::from_millis(50));
            }
        }
    };

    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    // Clone the handle so the test can write a second request while the
    // reader holds its borrow. `try_clone` shares the same socket.
    let mut writer = stream.try_clone().expect("clone stream");

    let msg = r#"{"type":"health_check"}"#;
    writer.write_all(msg.as_bytes()).expect("write");
    writer.write_all(b"\n").expect("write newline");
    writer.flush().expect("flush");

    let mut reader = BufReader::new(&stream);
    let mut line = String::new();
    reader.read_line(&mut line).expect("read line");

    assert!(
        line.contains("\"status\":\"healthy\""),
        "the daemon did not answer with a healthy response. got: {:?}",
        line
    );

    // The daemon must NOT close the connection after answering -- that is
    // what made the old streaming probe wait forever. Assert the
    // connection is still usable for a second request.
    let msg2 = r#"{"type":"health_check"}"#;
    writer.write_all(msg2.as_bytes()).expect("second write");
    writer.write_all(b"\n").expect("second write newline");
    writer.flush().expect("second flush");

    let mut line2 = String::new();
    reader.read_line(&mut line2).expect("second read line");
    assert!(
        line2.contains("\"status\":\"healthy\""),
        "the daemon did not answer a second health check on the same \
         connection. got: {:?}",
        line2
    );
}
