//! Tailscale mesh network integration.
//!
//! Wraps the Tailscale CLI to discover peers, query node status, and
//! coordinate mesh networking for the Fabric daemon.

use serde::{Deserialize, Serialize};
use tokio::process::Command;

use super::NetworkError;

/// Default path to the Tailscale CLI binary.
const DEFAULT_TAILSCALE_PATH: &str = "tailscale";

/// A peer on the Tailscale network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TailscalePeer {
    /// Hostname of the peer.
    pub hostname: String,
    /// Tailscale IP address (e.g., "100.x.y.z").
    pub ip: String,
    /// Whether the peer is currently online.
    pub online: bool,
    /// Operating system of the peer.
    pub os: String,
    /// Tailscale public key of the peer.
    pub public_key: String,
}

/// Status of the local Tailscale node and its peers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TailscaleStatus {
    /// The local node information.
    pub self_node: TailscalePeer,
    /// All known peers on the tailnet.
    pub peers: Vec<TailscalePeer>,
}

/// Tailscale CLI wrapper for mesh networking operations.
///
/// # Example
///
/// ```no_run
/// use fabric_daemon::network::tailscale::TailscaleClient;
///
/// # async fn example() -> Result<(), fabric_daemon::network::NetworkError> {
/// let client = TailscaleClient::new();
/// let status = client.get_status().await?;
/// println!("Online: {}, Peers: {}", status.self_node.online, status.peers.len());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Default, Clone)]
pub struct TailscaleClient {
    /// Path to the tailscale binary.
    binary_path: String,
}

impl TailscaleClient {
    /// Creates a new Tailscale client using the default binary path.
    pub fn new() -> Self {
        Self {
            binary_path: DEFAULT_TAILSCALE_PATH.into(),
        }
    }

    /// Creates a new Tailscale client with a custom binary path.
    pub fn with_binary_path(path: impl Into<String>) -> Self {
        Self {
            binary_path: path.into(),
        }
    }

    /// Returns the local node status and all known peers.
    pub async fn get_status(&self) -> Result<TailscaleStatus, NetworkError> {
        let json = self.run_status_command().await?;
        parse_status_json(&json)
    }

    /// Discovers all peers on the tailnet.
    pub async fn discover_peers(&self) -> Result<Vec<TailscalePeer>, NetworkError> {
        let status = self.get_status().await?;
        Ok(status.peers)
    }

    /// Checks if the Tailscale daemon is running and connected.
    pub async fn is_connected(&self) -> Result<bool, NetworkError> {
        let output = Command::new(&self.binary_path)
            .args(["status", "--json"])
            .output()
            .await
            .map_err(|e| NetworkError::TailscaleCli {
                message: format!("Failed to execute tailscale: {e}"),
            })?;

        // tailscale exits 0 when connected, non-zero otherwise.
        Ok(output.status.success())
    }

    /// Returns the local Tailscale IP address.
    pub async fn local_ip(&self) -> Result<String, NetworkError> {
        let status = self.get_status().await?;
        Ok(status.self_node.ip)
    }

    /// Runs `tailscale status --json` and returns the raw output.
    async fn run_status_command(&self) -> Result<String, NetworkError> {
        let output = Command::new(&self.binary_path)
            .args(["status", "--json"])
            .output()
            .await
            .map_err(|e| NetworkError::TailscaleCli {
                message: format!("Failed to execute tailscale: {e}"),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(NetworkError::TailscaleCli {
                message: format!(
                    "tailscale status failed (exit {}): {stderr}",
                    output.status.code().unwrap_or(-1)
                ),
            });
        }

        String::from_utf8(output.stdout).map_err(|e| NetworkError::TailscaleCli {
            message: format!("Invalid UTF-8 in tailscale output: {e}"),
        })
    }
}

/// Raw JSON structure from `tailscale status --json`.
///
/// The Tailscale CLI outputs a `SelfV1` and `Peer` map. We parse the
/// relevant fields into our own types.
#[derive(Debug, Deserialize)]
struct RawStatus {
    #[serde(rename = "Self")]
    self_node: Option<RawPeer>,
    #[serde(rename = "Peer")]
    peers: Option<std::collections::HashMap<String, RawPeer>>,
}

#[derive(Debug, Deserialize)]
struct RawPeer {
    #[serde(rename = "HostName", default)]
    hostname: String,
    #[serde(rename = "TailscaleIPs", default)]
    tailscale_ips: Option<Vec<String>>,
    #[serde(rename = "Online", default)]
    online: bool,
    #[serde(rename = "OS", default)]
    os: String,
    #[serde(rename = "PublicKey", default)]
    public_key: String,
    #[serde(rename = "ID", default)]
    _id: u64,
}

/// Parses the raw tailscale status JSON into our types.
fn parse_status_json(json: &str) -> Result<TailscaleStatus, NetworkError> {
    let raw: RawStatus = serde_json::from_str(json).map_err(|e| NetworkError::TailscaleCli {
        message: format!("Failed to parse tailscale status JSON: {e}"),
    })?;

    let self_node = raw.self_node.ok_or_else(|| NetworkError::TailscaleCli {
        message: "No Self node in tailscale status".into(),
    })?;

    let self_node = convert_peer(self_node)?;

    let peers = raw
        .peers
        .unwrap_or_default()
        .into_values()
        .filter_map(|p| convert_peer(p).ok())
        .collect();

    Ok(TailscaleStatus { self_node, peers })
}

/// Converts a raw peer entry into a `TailscalePeer`.
fn convert_peer(raw: RawPeer) -> Result<TailscalePeer, NetworkError> {
    let ip = raw
        .tailscale_ips
        .and_then(|ips| ips.first().cloned())
        .unwrap_or_default();

    Ok(TailscalePeer {
        hostname: raw.hostname,
        ip,
        online: raw.online,
        os: raw.os,
        public_key: raw.public_key,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_status_json_basic() {
        let json = r#"{
            "Self": {
                "HostName": "my-node",
                "TailscaleIPs": ["100.64.0.1"],
                "Online": true,
                "OS": "linux",
                "PublicKey": "abc123",
                "ID": 1
            },
            "Peer": {
                "node2": {
                    "HostName": "other-node",
                    "TailscaleIPs": ["100.64.0.2"],
                    "Online": true,
                    "OS": "darwin",
                    "PublicKey": "def456",
                    "ID": 2
                }
            }
        }"#;

        let status = parse_status_json(json).unwrap();
        assert_eq!(status.self_node.hostname, "my-node");
        assert_eq!(status.self_node.ip, "100.64.0.1");
        assert!(status.self_node.online);
        assert_eq!(status.peers.len(), 1);
        assert_eq!(status.peers[0].hostname, "other-node");
        assert_eq!(status.peers[0].ip, "100.64.0.2");
    }

    #[test]
    fn parse_status_json_no_peers() {
        let json = r#"{
            "Self": {
                "HostName": "solo",
                "TailscaleIPs": ["100.64.0.1"],
                "Online": false,
                "OS": "linux",
                "PublicKey": "key1",
                "ID": 1
            }
        }"#;

        let status = parse_status_json(json).unwrap();
        assert_eq!(status.self_node.hostname, "solo");
        assert!(!status.self_node.online);
        assert!(status.peers.is_empty());
    }

    #[test]
    fn parse_status_json_invalid() {
        assert!(parse_status_json("not json").is_err());
    }

    #[test]
    fn parse_status_json_missing_self() {
        let json = r#"{"Peer": {}}"#;
        assert!(parse_status_json(json).is_err());
    }

    #[test]
    fn peer_serializes() {
        let peer = TailscalePeer {
            hostname: "test".into(),
            ip: "100.64.0.1".into(),
            online: true,
            os: "linux".into(),
            public_key: "key".into(),
        };
        let json = serde_json::to_string(&peer).unwrap();
        assert!(json.contains("\"hostname\""));
    }

    #[test]
    fn client_construction() {
        let c = TailscaleClient::new();
        assert_eq!(c.binary_path, DEFAULT_TAILSCALE_PATH);

        let c = TailscaleClient::with_binary_path("/usr/local/bin/tailscale");
        assert_eq!(c.binary_path, "/usr/local/bin/tailscale");
    }

    #[test]
    fn convert_peer_missing_ip() {
        let raw = RawPeer {
            hostname: "node".into(),
            tailscale_ips: None,
            online: true,
            os: "linux".into(),
            public_key: "k".into(),
            _id: 1,
        };
        let peer = convert_peer(raw).unwrap();
        assert_eq!(peer.ip, "");
    }

    #[test]
    fn parse_status_multiple_peers() {
        let json = r#"{
            "Self": {
                "HostName": "hub",
                "TailscaleIPs": ["100.64.0.1"],
                "Online": true,
                "OS": "linux",
                "PublicKey": "pk1",
                "ID": 1
            },
            "Peer": {
                "n2": {
                    "HostName": "peer2",
                    "TailscaleIPs": ["100.64.0.2"],
                    "Online": true,
                    "OS": "darwin",
                    "PublicKey": "pk2",
                    "ID": 2
                },
                "n3": {
                    "HostName": "peer3",
                    "TailscaleIPs": ["100.64.0.3"],
                    "Online": false,
                    "OS": "windows",
                    "PublicKey": "pk3",
                    "ID": 3
                }
            }
        }"#;

        let status = parse_status_json(json).unwrap();
        assert_eq!(status.peers.len(), 2);
    }
}
