//! Tests for the OAuth callback listener.
//!
//! Twelve tests in three groups:
//!
//! 1. **Registry-level** (4): pure in-memory `record_state`/`take_state`
//!    semantics. No socket.
//!
//! 2. **HTTP-level** (4): real loopback sockets, end-to-end through the
//!    listener. These are the contract tests for the four documented
//!    callback outcomes (valid, wrong state, missing code, unknown
//!    state). They assert the listener reaches `handle_auth_complete` or
//!    short-circuits with 400 as appropriate, without ever performing a
//!    real WorkOS network call (the test coordinator has no WorkOS
//!    configured, so `auth_complete` returns an `auth_error` whose
//!    `message` field we use as a reach-confirmation marker).
//!
//! 3. **Auxiliary** (4): method dispatch, unknown path, query parser,
//!    HTML escaper. These pin small but easy-to-regress contracts.

use super::dispatch::{html_escape, render_error_page};
use super::registry::{clear_registry_for_tests, record_state, take_state};
use super::server::run_callback_server;
use crate::coordinator::Coordinator;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

/// Serializes tests that touch the process-wide state registry.
///
/// The registry is a `OnceLock<Mutex<HashMap<..>>>` shared by the whole
/// process, and several tests call `clear_registry_for_tests()`. Cargo runs
/// tests in parallel by default, so without this lock one test's clear wipes
/// another test's in-flight state and the suite fails intermittently -- it
/// passed locally and failed on CI, which is exactly the shape of a race.
/// Hold this guard for the whole test body. A poisoned lock is recovered
/// rather than propagated, so one failing test does not cascade.
#[cfg(test)]
fn serial() -> std::sync::MutexGuard<'static, ()> {
    static SERIAL: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    match SERIAL.get_or_init(|| std::sync::Mutex::new(())).lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;

/// Build a Coordinator without WorkOS configured.
///
/// With an empty `workos_client_id`, `coordinator.workos_provider()`
/// returns `None`, so `handle_auth_complete` short-circuits with
/// `auth_error` whose `message` field contains
/// `"set auth.workos_client_id in daemon config"`. No network call is
/// made. The four HTTP tests rely on this so they can observe
/// "reached / did not reach auth-complete" without performing a real
/// WorkOS exchange.
fn make_coordinator() -> Arc<Coordinator> {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    let config = crate::config::DaemonConfig {
        database: crate::config::DatabaseConfig {
            path: db_path,
            ..Default::default()
        },
        ..Default::default()
    };
    Arc::new(Coordinator::new(config).unwrap())
}

/// Spin up the callback listener on a kernel-assigned port and return
/// the bound address along with a shutdown flag the caller can flip
/// to stop the listener thread.
fn spawn_test_listener(coordinator: Arc<Coordinator>) -> (String, Arc<AtomicBool>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
    let addr = listener.local_addr().unwrap().to_string();
    let runtime = Arc::new(
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap(),
    );

    // Drive the shutdown flag directly so the test listener exits
    // when the test ends without waiting on the real daemon's
    // ctrlc handler.
    let shutdown = coordinator.shutdown_flag();

    std::thread::spawn(move || {
        run_callback_server(listener, coordinator, runtime, Duration::from_secs(2));
    });

    (addr, shutdown)
}

/// Send a single HTTP GET to the listener and return
/// `(status_code, response_body)`.
fn get(addr: &str, path_and_query: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(addr).expect("connect test listener");
    let req =
        format!("GET {path_and_query} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).expect("write request");

    let mut raw = String::new();
    stream.read_to_string(&mut raw).expect("read response");

    let mut parts = raw.splitn(2, "\r\n\r\n");
    let head = parts.next().unwrap_or("");
    let body = parts.next().unwrap_or("").to_string();
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);
    (status, body)
}

// -- Registry-level tests (no socket) -------------------------------

#[test]
fn take_state_round_trip() {
    // Use a unique state value so this test does not race with
    // other registry tests sharing the same global Mutex<HashMap>.
    let state = "state-round-trip-X9Q7";
    record_state(state);
    assert!(take_state(state));
    // Second take must fail: one-shot.
    assert!(!take_state(state));
}

#[test]
fn take_state_unknown_returns_false() {
    assert!(!take_state("never-issued-3F2K"));
}

#[test]
fn take_state_empty_is_rejected() {
    assert!(!take_state(""));
}

#[test]
fn record_empty_is_a_noop() {
    record_state("");
    // Empty is silently discarded; no entry exists to consume.
    assert!(!take_state(""));
}

// -- HTTP-level tests (real loopback sockets) ------------------------

/// A callback with the recorded state and a code reaches the auth-
/// complete handler. The coordinator has no WorkOS configured, so the
/// handler returns `workos_not_configured` (no network). The test
/// asserts the response body carries that diagnostic — proving the
/// handler was reached — and does NOT carry the state-rejection
/// marker that would mean validation short-circuited.
#[test]
fn valid_callback_reaches_auth_complete() {
    let _serial = serial();
    clear_registry_for_tests();
    let coord = make_coordinator();
    let (addr, shutdown) = spawn_test_listener(coord);

    record_state("good-state");
    let (status, body) = get(&addr, "/auth/callback?code=fake-code&state=good-state");

    shutdown.store(true, Ordering::Relaxed);

    assert_eq!(status, 500, "expected exchange-error page: {body}");
    // The wire `auth_complete` returns an `auth_error` whose
    // `message` field carries the diagnostic. We just need a
    // substring that only appears in the error path (i.e. proves
    // we got past state validation) and not in any state-rejection
    // page. `workos_client_id` is unique to the no-workos-config
    // branch.
    assert!(
        body.contains("workos_client_id"),
        "auth-complete must have been reached; body was: {body}"
    );
    assert!(
        !body.contains("Invalid or expired state"),
        "state validation must not have rejected; body was: {body}"
    );
}

/// A callback whose state does NOT match the issued one is rejected
/// with 400 and never reaches `handle_auth_complete`.
#[test]
fn wrong_state_rejected_with_400() {
    let _serial = serial();
    clear_registry_for_tests();
    let coord = make_coordinator();
    let (addr, shutdown) = spawn_test_listener(coord);

    record_state("issued-state");
    let (status, body) = get(&addr, "/auth/callback?code=fake-code&state=different-state");

    shutdown.store(true, Ordering::Relaxed);

    assert_eq!(status, 400, "expected 400 for wrong state: {body}");
    assert!(
        body.contains("Invalid or expired state"),
        "expected invalid-state copy; body was: {body}"
    );
    assert!(
        !body.contains("workos_not_configured"),
        "auth-complete must NOT have been reached; body was: {body}"
    );
}

/// A callback without a `code` is rejected with 400. The state is
/// not consumed, so a subsequent valid retry still works.
#[test]
fn missing_code_rejected_with_400() {
    let _serial = serial();
    clear_registry_for_tests();
    let coord = make_coordinator();
    let (addr, shutdown) = spawn_test_listener(coord);

    record_state("state-without-code");
    let (status, body) = get(&addr, "/auth/callback?state=state-without-code");

    shutdown.store(true, Ordering::Relaxed);

    assert_eq!(status, 400, "expected 400 for missing code: {body}");
    assert!(
        body.contains("Missing code parameter"),
        "expected missing-code copy; body was: {body}"
    );

    // The state must still be redeemable because validation
    // short-circuited before `take_state` ran. (Otherwise the user
    // would have to restart the entire login flow on a typo.)
    assert!(
        take_state("state-without-code"),
        "state must not be consumed by a malformed request"
    );
}

/// A callback for a state that was never issued (expired or never
/// existed) is rejected with 400.
#[test]
fn unknown_state_rejected_with_400() {
    let _serial = serial();
    clear_registry_for_tests();
    let coord = make_coordinator();
    let (addr, shutdown) = spawn_test_listener(coord);

    let (status, body) = get(&addr, "/auth/callback?code=fake-code&state=never-issued");

    shutdown.store(true, Ordering::Relaxed);

    assert_eq!(status, 400, "expected 400 for unknown state: {body}");
    assert!(
        body.contains("Invalid or expired state"),
        "expected invalid-state copy; body was: {body}"
    );
}

// -- Auxiliary tests --------------------------------------------------

#[test]
fn render_error_page_escapes_user_controllable_strings() {
    let body = render_error_page("Err & <boom>", "<script>alert(1)</script>");
    assert!(body.contains("Err &amp; &lt;boom&gt;"));
    assert!(body.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    // Make sure the raw, unescaped form does NOT survive.
    assert!(!body.contains("<script>alert(1)</script>"));
    assert!(!body.contains("<boom>"));
}

#[test]
fn html_escape_handles_special_characters() {
    assert_eq!(
        html_escape("<b>\"a\"&'b'</b>"),
        "&lt;b&gt;&quot;a&quot;&amp;&#39;b&#39;&lt;/b&gt;"
    );
}

#[test]
fn non_get_returns_405() {
    let _serial = serial();
    clear_registry_for_tests();
    let coord = make_coordinator();
    let (addr, shutdown) = spawn_test_listener(coord);
    let mut stream = TcpStream::connect(&addr).unwrap();
    stream
        .write_all(b"POST /auth/callback HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();
    let status = raw
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);
    shutdown.store(true, Ordering::Relaxed);
    assert_eq!(status, 405);
}

#[test]
fn unknown_path_returns_404() {
    let _serial = serial();
    clear_registry_for_tests();
    let coord = make_coordinator();
    let (addr, shutdown) = spawn_test_listener(coord);
    let (status, _body) = get(&addr, "/something-else");
    shutdown.store(true, Ordering::Relaxed);
    assert_eq!(status, 404);
}
