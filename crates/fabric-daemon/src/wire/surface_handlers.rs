//! Wire handler for `surface_lease_request`.
//!
//! Creates a real [`fabric_graph::surface::SurfaceLease`] via the existing
//! surface-plane code and registers it with the coordinator's active-lease set
//! (which is what `Coordinator::flush` persists to SQLite).
//!
//! ## Lease state is honest, not optimistic
//!
//! The lease is returned in the **`Pending`** state, because that is the state
//! it is actually in. `Pending → Active` happens in
//! [`fabric_graph::surface_ops::bind`], which requires a concrete `RouteStep`
//! and `RoutePlanId`; a `surface_lease_request` carries neither, so there is
//! nothing real to bind to yet. Fabricating an `Active` lease here would
//! violate the spec 019 no-steal invariant — a client would hold a handle that
//! claims a live presentation binding which does not exist.
//!
//! This matches what the CLI already does on its daemon-unreachable path
//! (`crates/fabric-cli/src/commands/surface.rs`, the `ConnectionRefused` arm):
//! it emits `state: "pending"`. So the daemon path is consistent with the
//! client's own model of an un-bound lease.
//!
//! ## Lease protocol
//!
//! Request (from `crates/fabric-cli/src/commands/surface.rs`):
//! ```json
//! {"type":"surface_lease_request","name":"primary-display",
//!  "protocol":"posix","locality_floor":"L5Loopback","requires_rt":false}
//! ```
//! Response (read by the same file): `handle` (string), `state` (string).

use fabric_graph::surface::{SurfaceProtocol, SurfaceSpec};
use fabric_graph::surface_ops;

use crate::coordinator::Coordinator;

/// Handle a `surface_lease_request`: create a Pending surface lease.
pub(crate) fn handle_surface_lease_request(
    parsed: &serde_json::Value,
    coordinator: &Coordinator,
) -> Option<String> {
    let name = parsed
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let protocol_str = parsed
        .get("protocol")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let locality_str = parsed
        .get("locality_floor")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let requires_rt = parsed
        .get("requires_rt")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let protocol = parse_protocol(protocol_str);
    let locality = match parse_locality(locality_str) {
        Ok(tier) => tier,
        Err(msg) => {
            return Some(
                serde_json::json!({
                    "type": "surface_lease_error",
                    "error": "invalid_locality",
                    "message": msg,
                })
                .to_string(),
            );
        }
    };

    let spec = SurfaceSpec {
        name: name.to_string(),
        protocol,
        capture: None,
        locality_floor: locality,
        refresh_hz: None,
        audio_sample_rate_hz: None,
        requires_rt_island: requires_rt,
        strict_epoch_binding: false,
        min_host_trust: fabric_graph::TrustLevel::Untrusted,
        expires_at: None,
    };

    // `new_lease` validates the spec and returns a lease in the Pending state.
    let lease = match surface_ops::new_lease(spec) {
        Ok(l) => l,
        Err(e) => {
            return Some(
                serde_json::json!({
                    "type": "surface_lease_error",
                    "error": "invalid_spec",
                    "message": e.to_string(),
                })
                .to_string(),
            );
        }
    };

    let handle = lease.handle;
    let state = lease.state;
    // Register in the coordinator's active set so it is flushed to SQLite.
    coordinator.insert_lease(lease);

    Some(
        serde_json::json!({
            "type": "surface_lease_response",
            "handle": handle,
            "state": state,
        })
        .to_string(),
    )
}

/// Parse a surface protocol name, mirroring the CLI's `parse_protocol`
/// (`crates/fabric-cli/src/commands/surface.rs`) so both sides agree.
///
/// Unknown names are not rejected here: `SurfaceProtocol::Custom` is the
/// spec's escape hatch for user-registered protocols.
fn parse_protocol(s: &str) -> SurfaceProtocol {
    match s.to_lowercase().as_str() {
        "posix" => SurfaceProtocol::Posix,
        "vnc" => SurfaceProtocol::Vnc,
        "rdp" => SurfaceProtocol::Rdp,
        "webrtc" | "web_rtc" | "web-rtc" => SurfaceProtocol::WebRtc,
        "asp" => SurfaceProtocol::Asp,
        other => SurfaceProtocol::Custom(other.to_string()),
    }
}

/// Parse a locality floor.
///
/// The CLI sends either the short code (`"L5"`, which
/// `LocalityTier::from_str` accepts) or the long variant name
/// (`"L5Loopback"`, which is the CLI's own `default_value` in
/// `crates/fabric-cli/src/commands/surface.rs`). Both must work here, so this
/// falls back to matching the long names when `from_str` rejects the input.
fn parse_locality(s: &str) -> Result<fabric_graph::LocalityTier, String> {
    use fabric_graph::LocalityTier;
    use std::str::FromStr;

    if let Ok(tier) = LocalityTier::from_str(s) {
        return Ok(tier);
    }

    let normalized = s.trim().to_lowercase();
    let tier = match normalized.as_str() {
        "l0sameprocess" => LocalityTier::L0SameProcess,
        "l1samenuma" => LocalityTier::L1SameNuma,
        "l2crossnumashm" => LocalityTier::L2CrossNumaShm,
        "l3pciep2p" => LocalityTier::L3PcieP2P,
        "l4rdma" => LocalityTier::L4Rdma,
        "l5loopback" => LocalityTier::L5Loopback,
        "l6lan" => LocalityTier::L6Lan,
        "l7wan" => LocalityTier::L7Wan,
        "l8oob" => LocalityTier::L8Oob,
        _ => {
            return Err(format!(
                "unknown locality tier '{s}' (expected L0..L8 or a long name such as L5Loopback)"
            ));
        }
    };
    Ok(tier)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn make_coordinator() -> Arc<Coordinator> {
        let dir = tempfile::tempdir().unwrap();
        let config = crate::config::DaemonConfig {
            database: crate::config::DatabaseConfig {
                path: dir.path().join("test.db"),
                ..Default::default()
            },
            ..Default::default()
        };
        Arc::new(Coordinator::new(config).unwrap())
    }

    fn lease_request(name: &str, protocol: &str, locality: &str) -> serde_json::Value {
        serde_json::json!({
            "type": "surface_lease_request",
            "name": name,
            "protocol": protocol,
            "locality_floor": locality,
            "requires_rt": false,
        })
    }

    #[test]
    fn creates_a_pending_lease_and_returns_handle_and_state() {
        let coord = make_coordinator();
        let resp =
            handle_surface_lease_request(&lease_request("primary-display", "posix", "L5"), &coord)
                .unwrap();
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();

        // The CLI reads exactly these two keys as strings.
        assert!(v["handle"].is_string(), "handle must be a string");
        assert!(
            v["state"].is_string(),
            "state must be a string, got {:?}",
            v["state"]
        );
        // serde serializes LeaseState::Pending as the variant name.
        assert_eq!(v["state"], "Pending");
        // A real UUID, not a placeholder.
        assert!(uuid::Uuid::parse_str(v["handle"].as_str().unwrap()).is_ok());
    }

    #[test]
    fn accepts_the_clis_long_form_locality_default() {
        // surface.rs ships `default_value = "L5Loopback"`; the daemon must
        // accept it even though LocalityTier::from_str only knows "L5".
        let coord = make_coordinator();
        let resp = handle_surface_lease_request(&lease_request("d", "posix", "L5Loopback"), &coord)
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["state"], "Pending");
    }

    #[test]
    fn accepts_every_short_and_long_locality_form() {
        let pairs = [
            ("L0", "L0SameProcess"),
            ("L1", "L1SameNuma"),
            ("L2", "L2CrossNumaShm"),
            ("L3", "L3PcieP2P"),
            ("L4", "L4Rdma"),
            ("L5", "L5Loopback"),
            ("L6", "L6Lan"),
            ("L7", "L7Wan"),
            ("L8", "L8Oob"),
        ];
        for (short, long) in pairs {
            assert_eq!(
                parse_locality(short).unwrap(),
                parse_locality(long).unwrap(),
                "{short} and {long} must resolve to the same tier"
            );
        }
    }

    #[test]
    fn rejects_an_unparseable_locality_with_an_error_response() {
        let coord = make_coordinator();
        let resp = handle_surface_lease_request(&lease_request("d", "posix", "L99Nowhere"), &coord)
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["error"], "invalid_locality");
    }

    #[test]
    fn empty_name_is_rejected_by_spec_validation() {
        let coord = make_coordinator();
        let resp = handle_surface_lease_request(&lease_request("", "posix", "L5"), &coord).unwrap();
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["error"], "invalid_spec");
    }

    #[test]
    fn rt_island_on_l8_oob_is_rejected() {
        // SurfaceSpec::validate forbids this; confirm the daemon surfaces it
        // rather than silently creating an invalid lease.
        let coord = make_coordinator();
        let mut msg = lease_request("d", "posix", "L8");
        msg["requires_rt"] = serde_json::json!(true);
        let resp = handle_surface_lease_request(&msg, &coord).unwrap();
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["error"], "invalid_spec");
    }

    #[test]
    fn custom_protocols_are_accepted_not_rejected() {
        let coord = make_coordinator();
        let resp =
            handle_surface_lease_request(&lease_request("d", "my-proto", "L5"), &coord).unwrap();
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["state"], "Pending");
    }

    #[test]
    fn each_request_gets_a_distinct_handle() {
        let coord = make_coordinator();
        let a = handle_surface_lease_request(&lease_request("a", "posix", "L5"), &coord).unwrap();
        let b = handle_surface_lease_request(&lease_request("b", "posix", "L5"), &coord).unwrap();
        let a: serde_json::Value = serde_json::from_str(&a).unwrap();
        let b: serde_json::Value = serde_json::from_str(&b).unwrap();
        assert_ne!(a["handle"], b["handle"]);
    }

    #[test]
    fn protocol_parsing_matches_the_cli_table() {
        assert_eq!(parse_protocol("posix"), SurfaceProtocol::Posix);
        assert_eq!(parse_protocol("VNC"), SurfaceProtocol::Vnc);
        assert_eq!(parse_protocol("rdp"), SurfaceProtocol::Rdp);
        assert_eq!(parse_protocol("web-rtc"), SurfaceProtocol::WebRtc);
        assert_eq!(parse_protocol("asp"), SurfaceProtocol::Asp);
        assert_eq!(
            parse_protocol("weird"),
            SurfaceProtocol::Custom("weird".into())
        );
    }
}
