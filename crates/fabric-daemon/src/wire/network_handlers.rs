//! Wire handlers for the `fabric network` commands.
//!
//! Three request types, all of which are pure host introspection — the daemon
//! probes the machine it runs on and reports what it observed:
//!
//! - `network_status_request`  -> `{upnp:{...}, stun:{...}, tailscale:{...}}`
//! - `network_stun_request`    -> `{external_addr, external_port, mapped_addr, ...}`
//! - `network_tailscale_request` -> `{status, hostname, tailnet, peers:[...]}`
//!
//! The response field names below are dictated by the CLI's parser in
//! `crates/fabric-cli/src/commands/network.rs`; that file is the contract.
//! Anything the CLI reads with `.get(...)` must be present with exactly that
//! name and JSON type, and any field it treats as an error indicator must be
//! spelled the way it expects.
//!
//! ## Why the errors are values, not wire errors
//!
//! A daemon that cannot reach STUN or find a gateway is a *normal* condition,
//! not a protocol failure. The CLI distinguishes the two cases by whether the
//! response carries an `error` key (`wire_client::send_message` turns any such
//! response into a hard `DaemonError`). So each probe failure is reported as a
//! `status` string inside the payload, and the command still exits 0 and
//! prints what it knows.

use crate::network::stun::StunClient;
use crate::network::tailscale::TailscaleClient;
use crate::network::upnp::UPnPClient;

/// Status strings the CLI colour-codes as healthy
/// (`crates/fabric-cli/src/commands/network.rs`, `fmt_status`).
const STATUS_OK: &str = "ok";
const STATUS_UNAVAILABLE: &str = "unavailable";
const STATUS_ERROR: &str = "error";
const STATUS_DISABLED: &str = "disabled";

/// Handle `network_status_request`: probe all three network planes.
///
/// Returns the aggregate shape the CLI's `network status` renderer reads:
/// three sibling objects, each with a `status` string. `stun` additionally
/// carries `external_addr` (string) so the command can print the public IP.
pub(crate) async fn handle_network_status_request() -> Option<String> {
    let stun = probe_stun().await;
    let tailscale = probe_tailscale().await;

    let response = serde_json::json!({
        "type": "network_status_response",
        "upnp": probe_upnp().await,
        "stun": stun,
        "tailscale": tailscale,
    });
    Some(response.to_string())
}

/// Handle `network_stun_request`: run one STUN binding request.
///
/// Response fields (CLI `stun()`, `crates/fabric-cli/src/commands/network.rs`):
/// `external_addr` (string), `external_port` (number), `mapped_addr` (string).
/// `external_addr` and `external_port` are the same SocketAddr split apart,
/// because the CLI prints them on separate rows.
pub(crate) async fn handle_network_stun_request() -> Option<String> {
    let client = StunClient::new();
    match client.query_external_address().await {
        Ok(resp) => {
            let response = serde_json::json!({
                "type": "network_stun_response",
                "status": STATUS_OK,
                "external_addr": resp.external_addr.ip().to_string(),
                "external_port": resp.external_addr.port(),
                "mapped_addr": resp.mapped_addr.to_string(),
                "changed_addr": resp.changed_addr.map(|a| a.to_string()),
                "server": resp.server,
            });
            Some(response.to_string())
        }
        Err(e) => {
            // Not an `error` key: the CLI would turn that into DaemonError and
            // exit non-zero. Report it as a status so `fabric network stun`
            // still renders a result.
            warn_stun(&e);
            let response = serde_json::json!({
                "type": "network_stun_response",
                "status": STATUS_UNAVAILABLE,
                "external_addr": serde_json::Value::Null,
                "external_port": 0,
                "mapped_addr": serde_json::Value::Null,
                "detail": e.to_string(),
            });
            Some(response.to_string())
        }
    }
}

/// Handle `network_tailscale_request`: report the local node and its peers.
///
/// Response fields (CLI `tailscale()`, `crates/fabric-cli/src/commands/network.rs`):
/// `status` (string), `hostname` (string), `tailnet` (string), `peers` (array
/// of `{hostname, tailscale_ip, os, online}`).
///
/// Note the peer IP key is `tailscale_ip`, not `ip` — that is what the CLI
/// reads, even though the internal [`crate::network::tailscale::TailscalePeer`]
/// field is named `ip`.
pub(crate) async fn handle_network_tailscale_request() -> Option<String> {
    let probe = probe_tailscale().await;
    let response = serde_json::json!({
        "type": "network_tailscale_response",
        "status": probe["status"],
        "hostname": probe["hostname"],
        "tailnet": probe["tailnet"],
        "peer_count": probe["peer_count"],
        "peers": probe["peers"],
    });
    Some(response.to_string())
}

// -----------------------------------------------------------------------------
// Probes
// -----------------------------------------------------------------------------

/// STUN probe, shaped for embedding under either `network_status` or
/// `network_stun_request`. Always returns a `status` string.
async fn probe_stun() -> serde_json::Value {
    let client = StunClient::new();
    match client.query_external_address().await {
        Ok(resp) => serde_json::json!({
            "status": STATUS_OK,
            "external_addr": resp.external_addr.ip().to_string(),
            "external_port": resp.external_addr.port(),
            "mapped_addr": resp.mapped_addr.to_string(),
            "server": resp.server,
        }),
        Err(e) => {
            warn_stun(&e);
            serde_json::json!({
                "status": STATUS_UNAVAILABLE,
                "external_addr": serde_json::Value::Null,
                "detail": e.to_string(),
            })
        }
    }
}

/// UPnP gateway probe.
///
/// `network status` only reads `status` from this object, and `discover()`
/// reports absence as an error (no gateway on the LAN is the common case), so
/// absence maps to `unavailable` rather than an error response.
async fn probe_upnp() -> serde_json::Value {
    let client = UPnPClient::new();
    match client.discover().await {
        Ok(gw) => serde_json::json!({
            "status": STATUS_OK,
            "external_ip": gw.external_ip,
            "manufacturer": gw.manufacturer,
            "friendly_name": gw.friendly_name,
        }),
        Err(e) => {
            tracing::debug!(error = %e, "upnp: no gateway discovered");
            serde_json::json!({
                "status": STATUS_UNAVAILABLE,
                "detail": e.to_string(),
            })
        }
    }
}

/// Tailscale probe. Never fails the request: a missing or logged-out
/// `tailscale` binary is reported as a status so the CLI can render it.
async fn probe_tailscale() -> serde_json::Value {
    let client = TailscaleClient::new();
    match client.get_status().await {
        Ok(status) => {
            let peers: Vec<serde_json::Value> = status
                .peers
                .iter()
                .map(|p| {
                    serde_json::json!({
                        "hostname": p.hostname,
                        // The CLI reads `tailscale_ip`; the internal field is `ip`.
                        "tailscale_ip": p.ip,
                        "os": p.os,
                        "online": p.online,
                    })
                })
                .collect();

            serde_json::json!({
                "status": if status.self_node.online {
                    "connected"
                } else {
                    STATUS_ERROR
                },
                "hostname": status.self_node.hostname,
                "local_ip": status.self_node.ip,
                "tailnet": serde_json::Value::Null,
                "peer_count": peers.len(),
                "peers": peers,
            })
        }
        Err(e) => {
            tracing::debug!(error = %e, "tailscale: status unavailable");
            serde_json::json!({
                "status": STATUS_DISABLED,
                "hostname": serde_json::Value::Null,
                "tailnet": serde_json::Value::Null,
                "peer_count": 0,
                "peers": [],
                "detail": e.to_string(),
            })
        }
    }
}

fn warn_stun(e: &crate::network::NetworkError) {
    tracing::debug!(error = %e, "stun: query failed");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every response must round-trip through JSON so the wire server can
    /// write it as a single line.
    fn assert_single_line(json: &str) {
        assert!(!json.contains('\n'), "response must be a single line");
        serde_json::from_str::<serde_json::Value>(json).expect("must be valid JSON");
    }

    #[tokio::test]
    async fn status_response_has_the_three_sections_cli_reads() {
        let resp = handle_network_status_request().await.unwrap();
        assert_single_line(&resp);
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();

        for section in ["upnp", "stun", "tailscale"] {
            let obj = v[section]
                .as_object()
                .unwrap_or_else(|| panic!("{section} must be an object, got {:?}", v[section]));
            assert!(
                obj["status"].is_string(),
                "{section}.status must be a string for the CLI's fmt_status"
            );
        }
        // The CLI prints the external IP only when `stun.external_addr` is a
        // string; Null must be allowed for the unavailable case.
        assert!(v["stun"]["external_addr"].is_string() || v["stun"]["external_addr"].is_null());
    }

    #[tokio::test]
    async fn status_response_never_sets_error_key() {
        // Any `error` key becomes a DaemonError in wire_client and makes the
        // CLI exit non-zero, which would be wrong for an offline STUN server.
        let resp = handle_network_status_request().await.unwrap();
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert!(v.get("error").is_none());
    }

    #[tokio::test]
    async fn stun_response_has_addr_port_and_mapped_fields() {
        let resp = handle_network_stun_request().await.unwrap();
        assert_single_line(&resp);
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert!(v.get("error").is_none(), "must not set error key");
        assert!(v["status"].is_string());
        assert!(v["external_addr"].is_string() || v["external_addr"].is_null());
        assert!(v["external_port"].is_number());
        assert!(v["mapped_addr"].is_string() || v["mapped_addr"].is_null());
    }

    #[tokio::test]
    async fn tailscale_response_peer_shape_matches_cli_parser() {
        let resp = handle_network_tailscale_request().await.unwrap();
        assert_single_line(&resp);
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert!(v.get("error").is_none());
        assert!(v["status"].is_string());
        assert!(v["hostname"].is_string() || v["hostname"].is_null());
        assert!(v["tailnet"].is_string() || v["tailnet"].is_null());
        // The CLI iterates `peers` unconditionally.
        let peers = v["peers"]
            .as_array()
            .expect("peers must always be an array, even when unavailable");
        assert_eq!(v["peer_count"].as_u64(), Some(peers.len() as u64));
        for peer in peers {
            // CLI reads these four keys per peer.
            assert!(peer["hostname"].is_string() || peer["hostname"].is_null());
            assert!(peer["tailscale_ip"].is_string() || peer["tailscale_ip"].is_null());
            assert!(peer["os"].is_string() || peer["os"].is_null());
            assert!(peer["online"].is_boolean());
        }
    }

    #[tokio::test]
    async fn tailscale_status_is_in_the_cli_recognised_set() {
        // CLI colours "connected" green and everything else red; the daemon
        // must not invent a status string outside this vocabulary.
        let resp = handle_network_tailscale_request().await.unwrap();
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        let status = v["status"].as_str().unwrap();
        assert!(
            matches!(status, "connected" | "error" | "disabled"),
            "unexpected status: {status}"
        );
    }

    #[tokio::test]
    async fn aggregate_status_stays_in_the_cli_status_vocabulary() {
        // `fmt_status` special-cases these; anything else falls through to a
        // plain render, so keep the daemon's vocabulary inside the known set.
        let resp = handle_network_status_request().await.unwrap();
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        for section in ["upnp", "stun", "tailscale"] {
            let status = v[section]["status"].as_str().unwrap();
            assert!(
                matches!(
                    status,
                    STATUS_OK
                        | "available"
                        | "connected"
                        | "degraded"
                        | STATUS_UNAVAILABLE
                        | STATUS_ERROR
                        | STATUS_DISABLED
                        | "not_running"
                ),
                "unexpected {section} status: {status}"
            );
        }
    }
}
