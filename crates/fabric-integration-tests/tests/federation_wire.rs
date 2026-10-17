//! Cross-daemon federation sync over the real wire protocol.
//!
//! These tests start two in-process daemons via [`start_two_daemons`],
//! then exercise the `federation_sync_request` wire message end-to-end:
//! peer A builds a JSON snapshot of its own topology, ships it to peer B
//! over a real TCP connection, and B's wire handler merges it against
//! B's local snapshot. The merged view is returned as
//! `federation_sync_response`.
//!
//! Honest loopback test: both daemons are in the same process and share
//! the same host; what this proves is that the wire arm reaches the
//! merge function with a serialised peer snapshot. Cross-machine semantics
//! (peer trust, address validation, retries) are intentionally out of scope
//! for this lane and live behind the `FederationConfig` switch.

use fabric_daemon::federation::{sync_topology, MergeStrategy, TopologySnapshot};
use fabric_integration_tests::harness::{send_and_receive, start_two_daemons, stop_daemon};
use std::collections::HashMap;

/// Build a `federation_sync_request` line carrying the given peer snapshot
/// and merge strategy.
fn federation_sync_request_line(peer: &TopologySnapshot, strategy: &MergeStrategy) -> String {
    let strategy_str = match strategy {
        MergeStrategy::MergeAll => "MergeAll",
        MergeStrategy::LocalPrimary => "LocalPrimary",
        MergeStrategy::PeerPrimary => "PeerPrimary",
    };
    // Embed the peer snapshot as a nested object. We rebuild the
    // envelope from scratch rather than embedding a pre-serialised
    // string so the schema validator sees the `peer_snapshot` field as
    // an object (not a string).
    serde_json::json!({
        "type": "federation_sync_request",
        "strategy": strategy_str,
        "peer_snapshot": serde_json::to_value(peer).expect("serialise peer snapshot"),
    })
    .to_string()
}

/// Fetch daemon `addr`'s topology via the existing `sync_topology` TCP
/// path, set its `federation_id` to the supplied prefix, and return it
/// ready to be carried as `peer_snapshot`.
fn snapshot_for_peer(addr: &str, federation_id: &str) -> TopologySnapshot {
    let mut snap = sync_topology(addr).expect("sync_topology to peer should succeed");
    snap.federation_id = federation_id.to_string();
    snap
}

#[test]
fn federation_sync_request_merges_peer_into_local_via_wire() {
    let (a, b) = start_two_daemons();

    // Fetch A's view of itself and assign an explicit federation_id so
    // the merge function prefixes A's nodes with "peer-A/".
    let peer_snapshot = snapshot_for_peer(&a.addr, "peer-A");
    assert!(
        !peer_snapshot.nodes.is_empty(),
        "peer-A snapshot must include A's local nodes (got {:?})",
        peer_snapshot.nodes,
    );

    // B receives the request: it pulls its own local snapshot and merges
    // it with the peer's. Send the request and read the response.
    let request = federation_sync_request_line(&peer_snapshot, &MergeStrategy::MergeAll);
    let response = send_and_receive(&b.addr, &request);

    // Parse and assert the merged view.
    let parsed: serde_json::Value = serde_json::from_str(&response)
        .unwrap_or_else(|e| panic!("response not valid JSON: {e}; raw={response}"));
    assert_eq!(
        parsed["type"], "federation_sync_response",
        "expected federation_sync_response, got: {response}"
    );

    // B contributed node_b (no federation_id). A's node_a should appear
    // prefixed with peer-A (under MergeAll).
    let merged_ids: Vec<String> = parsed["nodes"]
        .as_array()
        .expect("nodes should be an array")
        .iter()
        .filter_map(|n| n["id"].as_str().map(String::from))
        .collect();
    assert!(
        merged_ids.iter().any(|id| id == "node_b"),
        "merged view must include B's local node_b; got: {merged_ids:?}"
    );
    assert!(
        merged_ids.iter().any(|id| id.starts_with("peer-A/")),
        "merged view must include A's nodes prefixed with peer-A/; got: {merged_ids:?}"
    );

    // peer_count reflects how many peer snapshots were merged in.
    assert_eq!(
        parsed["peer_count"].as_u64(),
        Some(1),
        "peer_count should be 1 after a single federation_sync_request"
    );
    // federation_ids carries the prefix the peer identified itself with.
    let fed_ids = parsed["federation_ids"]
        .as_array()
        .expect("federation_ids should be an array");
    assert!(
        fed_ids.iter().any(|v| v == "peer-A"),
        "federation_ids should contain peer-A; got: {fed_ids:?}"
    );

    stop_daemon(a);
    stop_daemon(b);
}

#[test]
fn federation_sync_request_with_local_primary_skips_local_dups() {
    let (a, b) = start_two_daemons();

    // Build a peer snapshot for A that contains a node whose unprefixed
    // id is "node_b" -- the same name B already has. With LocalPrimary,
    // B's local copy should win, so the merged view should not include
    // any "peer-A/node_b" entry.
    let mut peer_snapshot = sync_topology(&a.addr).expect("sync_topology to A");
    peer_snapshot.federation_id = "peer-A".to_string();
    // Clear A's existing nodes; we only care about the collision case.
    let mut nodes: HashMap<String, fabric_daemon::federation::NodeEntry> = HashMap::new();
    nodes.insert(
        "node_b".to_string(),
        fabric_daemon::federation::NodeEntry {
            id: "node_b".to_string(),
            label: "Remote Node B".to_string(),
            locality: "LAN".to_string(),
            cap_count: 0,
            tags: vec![],
            federation_id: "peer-A".to_string(),
        },
    );
    peer_snapshot.nodes = nodes;
    peer_snapshot.node_count = 1;

    let request = federation_sync_request_line(&peer_snapshot, &MergeStrategy::LocalPrimary);
    let response = send_and_receive(&b.addr, &request);
    let parsed: serde_json::Value = serde_json::from_str(&response).expect("parse response");

    let merged_ids: Vec<String> = parsed["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|n| n["id"].as_str().map(String::from))
        .collect();

    // LocalPrimary drops peer nodes whose unprefixed name already exists
    // locally. B's local "node_b" is the winner; no "peer-A/node_b" is added.
    assert!(
        merged_ids.iter().any(|id| id == "node_b"),
        "merged view must include the local node_b; got: {merged_ids:?}"
    );
    assert!(
        !merged_ids.iter().any(|id| id == "peer-A/node_b"),
        "LocalPrimary should drop the duplicate; got: {merged_ids:?}"
    );

    stop_daemon(a);
    stop_daemon(b);
}

#[test]
fn federation_sync_request_missing_peer_snapshot_is_a_validation_error() {
    // The schema validator rejects a request without `peer_snapshot` with
    // MISSING_FIELD before the handler is ever invoked.
    let (a, b) = start_two_daemons();
    let request = r#"{"type":"federation_sync_request","strategy":"MergeAll"}"#;
    let response = send_and_receive(&b.addr, request);
    assert!(
        response.contains("MISSING_FIELD"),
        "missing peer_snapshot should be a validation error; got: {response}"
    );
    stop_daemon(a);
    stop_daemon(b);
}
