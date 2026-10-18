//! User-facing pairing path: dial a peer daemon over TCP, ship a
//! `federation_sync_request` carrying the local topology, and report the
//! merged view returned by the peer.
//!
//! This is the runtime pairing path that turns two `fabric-daemon`
//! instances into a federated pair. The mechanics are the same code
//! path the integration test exercises: open a plain TCP stream, write
//! a newline-delimited JSON request, read the response line, parse it.
//!
//! Two entry points are exported:
//!
//! * [`pair_with_peers`] is the library-level driver. It takes a snapshot
//!   of the local topology (as the `probe_response` JSON shape) plus a
//!   list of peer addresses, and returns a [`Vec<PairOutcome>`]
//!   describing each attempt. No I/O surfaces (files, network listeners)
//!   are touched beyond the outbound TCP connections.
//! * The wire handler in `wire::handlers::handle_federation_pair_request`
//!   wraps `pair_with_peers` and exposes it through the daemon's wire
//!   protocol as the `federation_pair_request` message. That is the
//!   documented user path; see `docs/acceptance/A2-TWO-DEVICE-PAIRING.md`.
//!
//! ## What this proves vs what it does not
//!
//! `pair_with_peers` proves the wire exchange end-to-end: the local
//! daemon reaches the peer, the peer's wire handler parses the request,
//! and the merged topology view comes back. It does NOT prove:
//!
//! * that the pair survives a physical-machine hop (the integration test
//!   runs both daemons in the same process on loopback),
//! * NAT traversal or peer discovery (no STUN/UPnP/Tailscale is invoked;
//!   `static_peers` is the address list and nothing else),
//! * that the merged view is persisted or observable by future queries
//!   (the pair outcome is reported once and not cached on this side; the
//!   peer's `federation_sync_request` handler is stateless across calls,
//!   so "pairing" here is a one-shot data exchange, not a long-lived
//!   session).
//!
//! Honest loopback pairing is the right scope for this lane; expanding
//! to cross-machine semantics is a separate increment.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::merge::merge_topologies_from_json;
use super::types::{MergeStrategy, NodeEntry, TopologySnapshot};

/// The outcome of pairing with one peer address.
///
/// Constructed and returned by [`pair_with_peers`]; serialised into the
/// `federation_pair_response` payload by the wire handler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairOutcome {
    /// Peer address the daemon tried to reach (verbatim from the request).
    pub peer: String,
    /// Whether the daemon successfully connected, sent the request, and
    /// parsed a `federation_sync_response` line. Failures populate
    /// [`Self::error`] instead.
    pub reachable: bool,
    /// Number of peer snapshots the peer merged in. `None` when
    /// [`Self::reachable`] is false.
    pub peer_count: Option<usize>,
    /// Highest topology epoch observed in the merged view. `None` when
    /// [`Self::reachable`] is false.
    pub max_epoch: Option<u64>,
    /// Federation IDs the peer reported having merged in. Empty when
    /// [`Self::reachable`] is false.
    pub federation_ids: Vec<String>,
    /// IDs of the nodes the peer returned in its merged view (already
    /// prefixed with their federation_id by the merge logic). Empty when
    /// [`Self::reachable`] is false.
    pub merged_node_ids: Vec<String>,
    /// Free-text failure description. `None` on success.
    pub error: Option<String>,
}

/// Dial each peer in `peers`, exchange a topology sync, and return one
/// [`PairOutcome`] per attempt in input order.
///
/// The function never panics on a single bad address: a connection
/// refused, a parse error, or a timeout produces a [`PairOutcome`] with
/// `reachable = false` and an `error` string. The whole call is
/// synchronous and uses `std::net` so it can be driven from a regular
/// std thread without a tokio runtime.
///
/// # Arguments
///
/// * `local_snapshot_json` -- the local daemon's `probe_response` JSON,
///   as returned by `coordinator.topology_snapshot()`. It is parsed for
///   the local epoch / node count but the per-node details come from
///   the wire round-trip to the peer.
/// * `peers` -- `host:port` strings identifying each peer daemon's wire
///   listener. Loopback is fine; nothing here assumes a routable IP.
/// * `federation_id` -- the federation_id prefix the peer should apply
///   to this daemon's nodes when merging. Equivalent to the
///   `federation_id` field on a `TopologySnapshot` carried in
///   `federation_sync_request`.
/// * `merge_strategy` -- strategy the peer should apply when merging.
///   `MergeAll`, `LocalPrimary`, `PeerPrimary`.
/// * `timeout` -- per-operation TCP read/write timeout. Keep this short
///   so a single dead peer does not stall pairing.
pub fn pair_with_peers(
    local_snapshot_json: &str,
    peers: &[String],
    federation_id: &str,
    merge_strategy: &MergeStrategy,
    timeout: Duration,
) -> Vec<PairOutcome> {
    let mut outcomes = Vec::with_capacity(peers.len());
    for peer in peers {
        outcomes.push(pair_one(
            local_snapshot_json,
            peer,
            federation_id,
            merge_strategy,
            timeout,
        ));
    }
    outcomes
}

fn pair_one(
    local_snapshot_json: &str,
    peer: &str,
    federation_id: &str,
    merge_strategy: &MergeStrategy,
    timeout: Duration,
) -> PairOutcome {
    // Connect to the peer.
    let stream = match TcpStream::connect(peer) {
        Ok(s) => s,
        Err(e) => {
            return PairOutcome {
                peer: peer.to_string(),
                reachable: false,
                peer_count: None,
                max_epoch: None,
                federation_ids: Vec::new(),
                merged_node_ids: Vec::new(),
                error: Some(format!("connect failed: {e}")),
            };
        }
    };

    if let Err(e) = stream.set_read_timeout(Some(timeout)) {
        return PairOutcome {
            peer: peer.to_string(),
            reachable: false,
            peer_count: None,
            max_epoch: None,
            federation_ids: Vec::new(),
            merged_node_ids: Vec::new(),
            error: Some(format!("set read timeout: {e}")),
        };
    }
    if let Err(e) = stream.set_write_timeout(Some(timeout)) {
        return PairOutcome {
            peer: peer.to_string(),
            reachable: false,
            peer_count: None,
            max_epoch: None,
            federation_ids: Vec::new(),
            merged_node_ids: Vec::new(),
            error: Some(format!("set write timeout: {e}")),
        };
    }

    // Build the peer snapshot from the local probe_response JSON. The
    // shape the wire handler expects is `TopologySnapshot`, not
    // `probe_response`, so we convert here. We use a minimal view: the
    // peer does not need our full probe metadata (active_leases,
    // active_plans, status, name) -- only topology_epoch and the
    // per-node entries -- so we emit a TopologySnapshot with the
    // federation_id the caller supplied and source_addr set to the peer
    // we are contacting (the peer's merge logic uses that for
    // bookkeeping).
    let peer_snapshot = match build_peer_snapshot(local_snapshot_json, federation_id, peer) {
        Ok(s) => s,
        Err(e) => {
            return PairOutcome {
                peer: peer.to_string(),
                reachable: false,
                peer_count: None,
                max_epoch: None,
                federation_ids: Vec::new(),
                merged_node_ids: Vec::new(),
                error: Some(format!("build peer snapshot: {e}")),
            };
        }
    };

    // Serialise the request envelope. We rebuild it from scratch rather
    // than embedding a pre-serialised string so the schema validator on
    // the peer sees `peer_snapshot` as a JSON object, not a string.
    let strategy_str = match merge_strategy {
        MergeStrategy::MergeAll => "MergeAll",
        MergeStrategy::LocalPrimary => "LocalPrimary",
        MergeStrategy::PeerPrimary => "PeerPrimary",
    };
    let request_value = serde_json::json!({
        "type": "federation_sync_request",
        "strategy": strategy_str,
        "peer_snapshot": serde_json::to_value(&peer_snapshot).unwrap_or_default(),
    });
    let request_line = format!("{}\n", request_value);

    // Send.
    let mut writer = match stream.try_clone() {
        Ok(w) => w,
        Err(e) => {
            return PairOutcome {
                peer: peer.to_string(),
                reachable: false,
                peer_count: None,
                max_epoch: None,
                federation_ids: Vec::new(),
                merged_node_ids: Vec::new(),
                error: Some(format!("clone stream: {e}")),
            };
        }
    };
    if let Err(e) = writer.write_all(request_line.as_bytes()) {
        return PairOutcome {
            peer: peer.to_string(),
            reachable: false,
            peer_count: None,
            max_epoch: None,
            federation_ids: Vec::new(),
            merged_node_ids: Vec::new(),
            error: Some(format!("write request: {e}")),
        };
    }
    if let Err(e) = writer.flush() {
        return PairOutcome {
            peer: peer.to_string(),
            reachable: false,
            peer_count: None,
            max_epoch: None,
            federation_ids: Vec::new(),
            merged_node_ids: Vec::new(),
            error: Some(format!("flush: {e}")),
        };
    }

    // Read first non-empty response line.
    let reader = BufReader::new(stream);
    let mut lines = reader.lines();
    let response_line = loop {
        match lines.next() {
            Some(Ok(line)) => {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    break trimmed.to_string();
                }
            }
            Some(Err(e)) => {
                return PairOutcome {
                    peer: peer.to_string(),
                    reachable: false,
                    peer_count: None,
                    max_epoch: None,
                    federation_ids: Vec::new(),
                    merged_node_ids: Vec::new(),
                    error: Some(format!("read response: {e}")),
                };
            }
            None => {
                return PairOutcome {
                    peer: peer.to_string(),
                    reachable: false,
                    peer_count: None,
                    max_epoch: None,
                    federation_ids: Vec::new(),
                    merged_node_ids: Vec::new(),
                    error: Some("peer closed without responding".to_string()),
                };
            }
        }
    };

    parse_pair_response(peer, &response_line)
}

/// Parse the peer's `federation_sync_response` line and turn it into a
/// [`PairOutcome`]. Errors become `reachable = false` with an `error`
/// string; a successful parse populates the merged-view fields.
fn parse_pair_response(peer: &str, response_line: &str) -> PairOutcome {
    let value: serde_json::Value = match serde_json::from_str(response_line) {
        Ok(v) => v,
        Err(e) => {
            return PairOutcome {
                peer: peer.to_string(),
                reachable: false,
                peer_count: None,
                max_epoch: None,
                federation_ids: Vec::new(),
                merged_node_ids: Vec::new(),
                error: Some(format!("response not valid JSON: {e}")),
            };
        }
    };

    let kind = value.get("type").and_then(|v| v.as_str()).unwrap_or("");
    if kind != "federation_sync_response" {
        return PairOutcome {
            peer: peer.to_string(),
            reachable: false,
            peer_count: None,
            max_epoch: None,
            federation_ids: Vec::new(),
            merged_node_ids: Vec::new(),
            error: Some(format!(
                "unexpected response type '{kind}' (expected federation_sync_response): {response_line}"
            )),
        };
    }

    let peer_count = value
        .get("peer_count")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize);
    let max_epoch = value.get("max_epoch").and_then(|v| v.as_u64());
    let federation_ids: Vec<String> = value
        .get("federation_ids")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let merged_node_ids: Vec<String> = value
        .get("nodes")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|n| n.get("id").and_then(|i| i.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default();

    PairOutcome {
        peer: peer.to_string(),
        reachable: true,
        peer_count,
        max_epoch,
        federation_ids,
        merged_node_ids,
        error: None,
    }
}

/// Build a `TopologySnapshot` from the local daemon's `probe_response`
/// JSON.
///
/// The wire handler at the other end expects a `TopologySnapshot`, not
/// a `probe_response`, so we walk the JSON once and assemble the
/// narrower type. `federation_id` and `source_addr` come from the
/// caller; node/edge counts and per-node entries come from the local
/// snapshot.
fn build_peer_snapshot(
    local_json: &str,
    federation_id: &str,
    source_addr: &str,
) -> Result<TopologySnapshot, String> {
    let value: serde_json::Value =
        serde_json::from_str(local_json).map_err(|e| format!("parse local snapshot: {e}"))?;

    let topology_epoch = value
        .get("topology_epoch")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let mut nodes: HashMap<String, NodeEntry> = HashMap::new();
    if let Some(arr) = value.get("nodes").and_then(|v| v.as_array()) {
        for n in arr {
            let id = match n.get("id").and_then(|v| v.as_str()) {
                Some(s) if !s.is_empty() => s.to_string(),
                _ => continue,
            };
            nodes.insert(
                id.clone(),
                NodeEntry {
                    id,
                    label: n
                        .get("label")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    locality: n
                        .get("locality")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    cap_count: n.get("cap_count").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
                    tags: n
                        .get("tags")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|t| t.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default(),
                    federation_id: federation_id.to_string(),
                },
            );
        }
    }

    let node_count = nodes.len();
    let edge_count = value
        .get("edges")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    Ok(TopologySnapshot {
        federation_id: federation_id.to_string(),
        source_addr: source_addr.to_string(),
        topology_epoch,
        node_count,
        edge_count,
        nodes,
        edges: HashMap::new(),
    })
}

/// Render a [`Vec<PairOutcome>`] as the `federation_pair_response`
/// payload the wire handler writes back to the caller.
pub fn write_pair_response(results: &[PairOutcome]) -> String {
    serde_json::json!({
        "type": "federation_pair_response",
        "results": results,
    })
    .to_string()
}

/// Apply the same merge the peer's `federation_sync_request` handler
/// would. Exposed for tests that want to assert what the merged view
/// should contain without standing up a second daemon.
#[allow(dead_code)]
pub fn merge_for_test(
    local_snapshot_json: &str,
    peer: &TopologySnapshot,
    strategy: &MergeStrategy,
) -> super::types::MergedTopology {
    merge_topologies_from_json(local_snapshot_json, std::slice::from_ref(peer), strategy)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local_snapshot_with_node(name: &str) -> String {
        serde_json::json!({
            "type": "probe_response",
            "status": "ok",
            "topology_epoch": 7,
            "topology_name": "test",
            "node_count": 1,
            "edge_count": 0,
            "nodes": [{
                "id": name,
                "label": "Local",
                "locality": "LAN",
                "cap_count": 0,
                "tags": [],
            }],
            "edges": [],
        })
        .to_string()
    }

    #[test]
    fn build_peer_snapshot_round_trip() {
        let snap = build_peer_snapshot(
            &local_snapshot_with_node("node_a"),
            "node-A",
            "127.0.0.1:9400",
        )
        .expect("snapshot");
        assert_eq!(snap.federation_id, "node-A");
        assert_eq!(snap.source_addr, "127.0.0.1:9400");
        assert_eq!(snap.topology_epoch, 7);
        assert_eq!(snap.node_count, 1);
        assert!(snap.nodes.contains_key("node_a"));
        let node_a = &snap.nodes["node_a"];
        assert_eq!(node_a.label, "Local");
        assert_eq!(node_a.locality, "LAN");
        assert_eq!(node_a.federation_id, "node-A");
    }

    #[test]
    fn build_peer_snapshot_missing_local_json_is_error() {
        let result = build_peer_snapshot("not json", "node-A", "127.0.0.1:9400");
        assert!(result.is_err(), "non-JSON input must surface as error");
    }

    #[test]
    fn pair_outcome_serialises_with_required_fields() {
        let outcome = PairOutcome {
            peer: "127.0.0.1:9400".into(),
            reachable: true,
            peer_count: Some(1),
            max_epoch: Some(7),
            federation_ids: vec!["node-A".into()],
            merged_node_ids: vec!["node_b".into(), "node-A/node_a".into()],
            error: None,
        };
        let json = serde_json::to_string(&outcome).expect("serialise");
        assert!(json.contains("\"reachable\":true"));
        assert!(json.contains("\"peer_count\":1"));
        assert!(json.contains("\"federation_ids\":[\"node-A\"]"));
        assert!(json.contains("\"merged_node_ids\":[\"node_b\",\"node-A/node_a\"]"));
        assert!(json.contains("\"error\":null"));
    }

    #[test]
    fn parse_pair_response_success() {
        let resp = r#"{"type":"federation_sync_response","nodes":[{"id":"node_b"},{"id":"node-A/node_a"}],"edges":[],"peer_count":1,"max_epoch":9,"federation_ids":["node-A"]}"#;
        let outcome = parse_pair_response("127.0.0.1:9400", resp);
        assert!(outcome.reachable);
        assert_eq!(outcome.peer_count, Some(1));
        assert_eq!(outcome.max_epoch, Some(9));
        assert_eq!(outcome.federation_ids, vec!["node-A"]);
        assert_eq!(outcome.merged_node_ids, vec!["node_b", "node-A/node_a"]);
        assert!(outcome.error.is_none());
    }

    #[test]
    fn parse_pair_response_wrong_type_is_error() {
        let resp = r#"{"type":"compile_error","error":"nope"}"#;
        let outcome = parse_pair_response("127.0.0.1:9400", resp);
        assert!(!outcome.reachable);
        assert!(outcome
            .error
            .as_deref()
            .unwrap_or("")
            .contains("unexpected response type"));
    }

    #[test]
    fn parse_pair_response_invalid_json_is_error() {
        let outcome = parse_pair_response("127.0.0.1:9400", "not json");
        assert!(!outcome.reachable);
        assert!(outcome
            .error
            .as_deref()
            .unwrap_or("")
            .contains("not valid JSON"));
    }

    #[test]
    fn write_pair_response_wraps_results() {
        let outcomes = vec![PairOutcome {
            peer: "127.0.0.1:9400".into(),
            reachable: true,
            peer_count: Some(1),
            max_epoch: Some(7),
            federation_ids: vec!["node-A".into()],
            merged_node_ids: vec!["node_b".into()],
            error: None,
        }];
        let line = write_pair_response(&outcomes);
        assert!(line.contains("\"type\":\"federation_pair_response\""));
        assert!(line.contains("\"results\""));
        assert!(line.contains("\"peer\":\"127.0.0.1:9400\""));
    }
}
