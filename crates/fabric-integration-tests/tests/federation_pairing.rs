//! End-to-end two-daemon pairing over the real wire protocol.
//!
//! These tests start two in-process daemons via [`start_two_daemons`],
//! then exercise the `federation_pair_request` wire message. The flow
//! is the documented user path:
//!
//! 1. The caller opens a single TCP connection to daemon A and writes
//!    a `federation_pair_request` line carrying a list of peer
//!    addresses (here, daemon B's address).
//! 2. Daemon A's wire handler dials each peer in turn and ships the
//!    local `federation_sync_request`.
//! 3. The peer's `federation_sync_request` handler merges the local
//!    snapshot with what A sent, returning the merged view.
//! 4. Daemon A wraps the per-peer outcomes into a
//!    `federation_pair_response` and returns it on the same connection.
//!
//! What this proves:
//! * the new `federation_pair_request` request type is accepted by
//!   the schema validator,
//! * the wire handler dials the configured peer, ships the local
//!   snapshot, and reads back a `federation_sync_response`,
//! * the merged view from the peer is exposed through the
//!   `federation_pair_response` payload that the caller can read.
//!
//! What this does NOT prove:
//! * cross-machine semantics (both daemons share the same host),
//! * NAT traversal / peer discovery (the test points at a real local
//!   listener; no STUN, UPnP, or Tailscale is involved),
//! * that the merged view is cached on A (it is reported once and
//!   discarded; A's `coordinator.topology_snapshot()` continues to
//!   return the local view).
//!
//! All tests in this file share a static [`PAIRING_GUARD`] mutex to
//! serialise pair-handler invocations across threads, which avoids the
//! cross-test flake pattern this repo has seen when a process-global
//! port is bound or shared; the harness still binds ephemeral ports
//! per daemon so the guard only protects against the dispatcher's
//! internal `Mutex<DaemonConfig>` contention.

use fabric_integration_tests::harness::{send_and_receive, start_two_daemons, stop_daemon};
use std::sync::Mutex;

/// Serialise pair-handler invocations across the file's tests so two
/// concurrently-running tests cannot race on the dispatcher's
/// `Mutex<DaemonConfig>` (the pair handler locks it briefly to read
/// the `federation` config block).
static PAIRING_GUARD: Mutex<()> = Mutex::new(());

/// Happy path: send a `federation_pair_request` to daemon A pointing
/// at daemon B, expect one `PairOutcome` with `reachable = true` and
/// the merged view containing both A's prefixed node and B's local
/// node.
#[test]
fn federation_pair_request_pairs_two_loopback_daemons() {
    let _guard = PAIRING_GUARD.lock().unwrap_or_else(|e| e.into_inner());

    let (a, b) = start_two_daemons();

    // Build the request: pair A with B. The handler on A dials B, ships
    // A's local snapshot, and returns the merged view.
    let request = format!(
        r#"{{"type":"federation_pair_request","federation_id":"peer-A","merge_strategy":"MergeAll","peers":["{}"]}}"#,
        b.addr
    );
    let response = send_and_receive(&a.addr, &request);

    let parsed: serde_json::Value = serde_json::from_str(&response)
        .unwrap_or_else(|e| panic!("response not valid JSON: {e}; raw={response}"));

    assert_eq!(
        parsed["type"], "federation_pair_response",
        "expected federation_pair_response, got: {response}"
    );

    let results = parsed["results"]
        .as_array()
        .expect("results must be an array");
    assert_eq!(
        results.len(),
        1,
        "exactly one peer attempt expected; got: {response}"
    );

    let outcome = &results[0];
    assert_eq!(
        outcome["peer"], b.addr,
        "outcome.peer should match the requested peer; got: {outcome}"
    );
    assert_eq!(
        outcome["reachable"], true,
        "loopback pair should be reachable; got: {outcome}"
    );
    assert_eq!(
        outcome["peer_count"], 1,
        "peer B should report peer_count=1 after merging in A; got: {outcome}"
    );
    assert_eq!(
        outcome["error"],
        serde_json::Value::Null,
        "successful pair must have error=null; got: {outcome}"
    );

    // The merged view from B should contain A's node prefixed with
    // peer-A and B's local node (unprefixed, because B was the local
    // side of the merge).
    let merged_ids: Vec<String> = outcome["merged_node_ids"]
        .as_array()
        .expect("merged_node_ids must be an array")
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    assert!(
        merged_ids.iter().any(|id| id.starts_with("peer-A/")),
        "merged view must contain A's node prefixed with peer-A/; got: {merged_ids:?}"
    );
    assert!(
        merged_ids.iter().any(|id| id == "node_b"),
        "merged view must contain B's local node_b; got: {merged_ids:?}"
    );

    // federation_ids should record A's federation_id as a peer that
    // B merged in.
    let fed_ids: Vec<String> = outcome["federation_ids"]
        .as_array()
        .expect("federation_ids must be an array")
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    assert!(
        fed_ids.iter().any(|v| v == "peer-A"),
        "federation_ids must record the peer-A prefix; got: {fed_ids:?}"
    );

    stop_daemon(a);
    stop_daemon(b);
}

/// Failure path: send a `federation_pair_request` to a daemon asking
/// it to pair with an address that nothing listens on (port 1 on
/// loopback is reserved and almost always closed). The handler must
/// return a `PairOutcome` with `reachable = false` and an error
/// string, NOT a process-level panic.
#[test]
fn federation_pair_request_reports_unreachable_peer() {
    let _guard = PAIRING_GUARD.lock().unwrap_or_else(|e| e.into_inner());

    let (a, _b) = start_two_daemons();

    // 127.0.0.1:1 is the IANA "tcpmux" port; we treat it as a
    // guaranteed-unreachable loopback address for this test. If by
    // extreme bad luck something is listening, the test still has the
    // 5-second per-peer timeout as a backstop, so it can take a moment
    // but will not hang the suite.
    let unreachable = "127.0.0.1:1";
    let request = format!(
        r#"{{"type":"federation_pair_request","federation_id":"peer-A","merge_strategy":"MergeAll","peers":["{unreachable}"]}}"#
    );
    let response = send_and_receive(&a.addr, &request);

    let parsed: serde_json::Value = serde_json::from_str(&response)
        .unwrap_or_else(|e| panic!("response not valid JSON: {e}; raw={response}"));

    assert_eq!(
        parsed["type"], "federation_pair_response",
        "expected federation_pair_response even on failure; got: {response}"
    );

    let results = parsed["results"]
        .as_array()
        .expect("results must be an array");
    assert_eq!(results.len(), 1, "exactly one peer attempt expected");

    let outcome = &results[0];
    assert_eq!(
        outcome["peer"], unreachable,
        "outcome.peer should echo the requested address"
    );
    assert_eq!(
        outcome["reachable"], false,
        "unreachable peer must be reported as reachable=false; got: {outcome}"
    );
    assert!(
        outcome["error"].as_str().is_some(),
        "unreachable peer must surface an error string; got: {outcome}"
    );

    stop_daemon(a);
    stop_daemon(_b);
}

/// Schema rejection: a `federation_pair_request` without the required
/// `peers` field is rejected by the validator before the handler
/// runs.
#[test]
fn federation_pair_request_missing_peers_is_a_validation_error() {
    let _guard = PAIRING_GUARD.lock().unwrap_or_else(|e| e.into_inner());

    let (a, b) = start_two_daemons();
    let request = r#"{"type":"federation_pair_request","federation_id":"peer-A"}"#;
    let response = send_and_receive(&a.addr, request);
    assert!(
        response.contains("MISSING_FIELD") || response.contains("missing_peers"),
        "missing peers should be rejected as a validation or handler error; got: {response}"
    );
    stop_daemon(a);
    stop_daemon(b);
}

/// Schema rejection: an empty `peers` array is rejected at the
/// handler level because pairing with nobody is a no-op. The schema
/// accepts the empty array (peers is required but length is not
/// enforced by the schema), so the handler emits
/// `federation_pair_error` with code `missing_peers`.
#[test]
fn federation_pair_request_empty_peers_is_a_handler_error() {
    let _guard = PAIRING_GUARD.lock().unwrap_or_else(|e| e.into_inner());

    let (a, b) = start_two_daemons();
    let request = r#"{"type":"federation_pair_request","peers":[]}"#;
    let response = send_and_receive(&a.addr, request);
    assert!(
        response.contains("missing_peers") || response.contains("invalid_peer"),
        "empty peers should be rejected; got: {response}"
    );
    stop_daemon(a);
    stop_daemon(b);
}
