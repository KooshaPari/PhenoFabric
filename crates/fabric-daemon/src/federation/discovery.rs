//! Peer discovery, topology fetching, and the background sync loop.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tracing::{debug, info, warn};

use super::state::FederationState;
use super::types::{EdgeEntry, FederationError, NodeEntry, TopologySnapshot};
use crate::config::FederationConfig;

/// Fetch topology from a peer daemon via TCP wire protocol.
///
/// Connects to `addr`, sends a `topology_request` message, and parses the
/// response. Returns `Err` on timeout or connection failure.
pub fn sync_topology(addr: &str) -> Result<TopologySnapshot, FederationError> {
    let timeout = Duration::from_secs(5);
    let stream = TcpStream::connect(addr).map_err(|e| {
        FederationError::ConnectionFailed(format!("failed to connect to peer {addr}: {e}"))
    })?;

    stream.set_read_timeout(Some(timeout)).ok();
    stream.set_write_timeout(Some(timeout)).ok();

    let mut writer = stream
        .try_clone()
        .map_err(|e| FederationError::ConnectionFailed(format!("clone stream: {e}")))?;

    // Send topology request.
    let request = r#"{"type":"topology_request"}"#;
    writeln!(writer, "{request}")
        .map_err(|e| FederationError::ConnectionFailed(format!("write request: {e}")))?;
    writer.flush().ok();

    // Read response.
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        let line = line.map_err(|e| FederationError::ParseError(format!("read response: {e}")))?;
        if line.is_empty() {
            continue;
        }
        return parse_topology_response(addr, &line);
    }

    Err(FederationError::NoResponse(format!(
        "peer {addr} returned empty response"
    )))
}

/// Parse a topology_response (probe_response) from a peer into a snapshot.
pub(crate) fn parse_topology_response(
    addr: &str,
    json: &str,
) -> Result<TopologySnapshot, FederationError> {
    let v: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| FederationError::ParseError(format!("invalid JSON: {e}")))?;

    let epoch = v
        .get("topology_epoch")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let node_count = v.get("node_count").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let edge_count = v.get("edge_count").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

    let mut nodes = HashMap::new();
    if let Some(node_list) = v.get("nodes").and_then(|v| v.as_array()) {
        for n in node_list {
            let id = n
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if id.is_empty() {
                continue;
            }
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
                    federation_id: String::new(), // filled by merge
                },
            );
        }
    }

    let mut edges = HashMap::new();
    if let Some(edge_list) = v.get("edges").and_then(|v| v.as_array()) {
        for e in edge_list {
            let id = e
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if id.is_empty() {
                continue;
            }
            edges.insert(
                id.clone(),
                EdgeEntry {
                    id,
                    from: e
                        .get("from")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    to: e
                        .get("to")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    locality: e
                        .get("locality")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    federation_id: String::new(),
                },
            );
        }
    }

    Ok(TopologySnapshot {
        federation_id: String::new(),
        source_addr: addr.to_string(),
        topology_epoch: epoch,
        node_count,
        edge_count,
        nodes,
        edges,
    })
}

/// Spawn the federation sync thread.
///
/// Periodically fetches topology from all configured peers and caches the
/// results. The coordinator can then merge them on demand.
pub fn spawn_sync_thread(state: Arc<FederationState>, config: FederationConfig) {
    let interval = Duration::from_secs(config.sync_interval_s);
    let shutdown = state.shutdown.clone();

    thread::spawn(move || {
        info!(
            interval_s = config.sync_interval_s,
            peers = config.peers.len(),
            "federation sync thread started"
        );

        loop {
            if shutdown.load(Ordering::Relaxed) {
                info!("federation sync thread stopping");
                break;
            }

            for peer_addr in &config.peers {
                match sync_topology(peer_addr) {
                    Ok(mut snapshot) => {
                        snapshot.federation_id = state.federation_id.clone();
                        debug!(
                            peer = %peer_addr,
                            epoch = snapshot.topology_epoch,
                            nodes = snapshot.node_count,
                            "fetched peer topology"
                        );
                        state.cache_snapshot(snapshot);
                    }
                    Err(e) => {
                        warn!(peer = %peer_addr, error = %e, "failed to sync peer topology");
                    }
                }
            }

            state.last_sync_epoch.fetch_add(1, Ordering::Relaxed);

            thread::sleep(interval);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_topology_response_valid() {
        let json = r#"{
            "type": "probe_response",
            "status": "ok",
            "topology_epoch": 42,
            "topology_name": "test-topo",
            "node_count": 2,
            "edge_count": 1,
            "nodes": [
                {"id": "n1", "label": "Node 1", "locality": "LAN", "cap_count": 3, "tags": ["gpu"]},
                {"id": "n2", "label": "Node 2", "locality": "WAN", "cap_count": 0, "tags": []}
            ],
            "edges": [
                {"id": "e1", "from": "n1", "to": "n2", "locality": "WAN"}
            ]
        }"#;

        let snapshot = parse_topology_response("10.0.0.1:9400", json).unwrap();
        assert_eq!(snapshot.topology_epoch, 42);
        assert_eq!(snapshot.node_count, 2);
        assert_eq!(snapshot.nodes.len(), 2);
        assert_eq!(snapshot.edges.len(), 1);
        assert_eq!(snapshot.source_addr, "10.0.0.1:9400");
    }

    #[test]
    fn parse_topology_response_invalid_json() {
        let result = parse_topology_response("addr", "not-json");
        assert!(result.is_err());
    }

    #[test]
    fn parse_topology_response_empty_body() {
        let json = r#"{"type": "probe_response"}"#;
        let snapshot = parse_topology_response("addr", json).unwrap();
        assert_eq!(snapshot.topology_epoch, 0);
        assert!(snapshot.nodes.is_empty());
    }
}
