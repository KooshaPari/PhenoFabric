use uuid::Uuid;

use crate::surface::{LeaseExitReason, SurfaceHandle, SurfaceLease, SurfaceSpec};

// ---------------------------------------------------------------------------
// RegistryEntry
// ---------------------------------------------------------------------------

/// A single entry in the [`super::SurfaceRegistry`]: the handle, lease, and
/// declarative spec for one active surface.
#[derive(Debug, Clone)]
pub struct RegistryEntry {
    /// The opaque user-facing handle (survives re-binding).
    pub handle: SurfaceHandle,
    /// The lease FSM state.
    pub lease: SurfaceLease,
    /// The spec that was originally requested.
    pub spec: SurfaceSpec,
}

// ---------------------------------------------------------------------------
// Invalidation
// ---------------------------------------------------------------------------

/// Returned by [`super::SurfaceRegistry::notify_node_failure`] for each surface
/// that was invalidated.
///
/// Field names match the Go `cmd/wire` `SurfaceInvalidate` wire shape
/// (spec 025 §3.3) to enable zero-copy JSON bridging.
#[derive(Debug, Clone)]
pub struct Invalidation {
    /// The handle whose lease was terminated.
    pub handle: SurfaceHandle,
    /// The route binding id that was active at invalidation time.
    /// Maps to `SurfaceInvalidate.lease_id` on the wire.
    pub binding_id: Uuid,
    /// Why the surface was invalidated.
    pub reason: LeaseExitReason,
    /// The topology epoch at binding time.
    /// Maps to `SurfaceInvalidate.epoch` on the wire.
    pub epoch: u64,
}

impl Invalidation {
    /// Serialize to the Go `cmd/wire` `SurfaceInvalidate` JSON wire shape.
    ///
    /// The output is ready to be wrapped in a `WireEnvelope` with
    /// `msg_type: "surface.invalidate"` by the Go wire codec.
    ///
    /// ```json
    /// {
    ///   "surface_handle": "01abcdef...",
    ///   "lease_id": "01112233...",
    ///   "reason": "HostFailure",
    ///   "failed_node": "gpu-node-1",
    ///   "epoch": 42
    /// }
    /// ```
    pub fn to_wire_json(&self) -> serde_json::Value {
        let reason_str = match &self.reason {
            LeaseExitReason::NormalCompletion => "NormalCompletion",
            LeaseExitReason::HostFailure { .. } => "HostFailure",
            LeaseExitReason::EpochDrift { .. } => "EpochDrift",
            LeaseExitReason::OperatorRevoked => "Revoked",
            LeaseExitReason::Expired => "Expired",
            LeaseExitReason::WorkloadReported { .. } => "Failed",
        };

        let failed_node = match &self.reason {
            LeaseExitReason::HostFailure { host_node } => host_node.to_string(),
            _ => String::new(),
        };

        let mut map = serde_json::Map::new();
        map.insert(
            "surface_handle".into(),
            serde_json::Value::String(self.handle.0.to_string()),
        );
        map.insert(
            "lease_id".into(),
            serde_json::Value::String(self.binding_id.to_string()),
        );
        map.insert(
            "reason".into(),
            serde_json::Value::String(reason_str.into()),
        );
        if !failed_node.is_empty() {
            map.insert(
                "failed_node".into(),
                serde_json::Value::String(failed_node),
            );
        }
        map.insert(
            "epoch".into(),
            serde_json::Value::Number(self.epoch.into()),
        );

        serde_json::Value::Object(map)
    }
}
