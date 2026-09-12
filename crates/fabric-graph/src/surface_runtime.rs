//! Surface-plane runtime registry (PF-WP-030, spec 024).
//!
//! Provides an in-memory registry of active [`SurfaceLease`]s so the failover
//! hook can invalidate leases when a node fails, and so the wire transport
//! can enumerate active surfaces.
//!
//! ## Design rules
//!
//! - Single-threaded, in-memory; no persistence (R3 concern).
//! - `insert` is idempotent (overwrites if handle already present).
//! - `notify_node_failure` returns the list of invalidations — the caller
//!   is responsible for external notification (event log emission, Go-side
//!   handle drop).
//! - `bind_with_topology` (in `surface_ops`) replaces the
//!   `derive_endpoint_for_step` placeholder with a real topology lookup.

use std::collections::HashMap;

use uuid::Uuid;

use crate::model::NodeId;
use crate::surface::{
    LeaseExitReason, LeaseState, SurfaceError, SurfaceHandle, SurfaceLease, SurfaceSpec,
};
use crate::surface_ops;

// ---------------------------------------------------------------------------
// RegistryEntry
// ---------------------------------------------------------------------------

/// A single entry in the [`SurfaceRegistry`]: the handle, lease, and
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

/// Returned by [`SurfaceRegistry::notify_node_failure`] for each surface
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

// ---------------------------------------------------------------------------
// SurfaceRegistry
// ---------------------------------------------------------------------------

/// In-memory registry of active [`SurfaceLease`]s keyed by [`SurfaceHandle`].
///
/// The failover hook calls [`notify_node_failure`] to invalidate all
/// leases whose current binding touches any of the failed nodes.
#[derive(Debug)]
pub struct SurfaceRegistry {
    inner: HashMap<SurfaceHandle, RegistryEntry>,
}

impl Default for SurfaceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SurfaceRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    /// Insert a surface entry. If `handle` is already present, the old
    /// entry is silently replaced (idempotent — spec 024 §Semantics).
    pub fn insert(
        &mut self,
        handle: SurfaceHandle,
        lease: SurfaceLease,
        spec: SurfaceSpec,
    ) {
        self.inner.insert(
            handle,
            RegistryEntry {
                handle,
                lease,
                spec,
            },
        );
    }

    /// Remove and return an entry by handle, if present.
    pub fn remove(&mut self, handle: &SurfaceHandle) -> Option<RegistryEntry> {
        self.inner.remove(handle)
    }

    /// Look up an entry by handle.
    pub fn get(&self, handle: &SurfaceHandle) -> Option<&RegistryEntry> {
        self.inner.get(handle)
    }

    /// Number of registered surfaces.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// True iff the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// All currently registered handles.
    pub fn handles(&self) -> Vec<SurfaceHandle> {
        self.inner.keys().copied().collect()
    }

    /// Invalidate all leases whose current binding node is in `failed_nodes`.
    ///
    /// For each affected lease:
    /// 1. Transition `Active → Failed` via [`surface_ops::fail`].
    /// 2. Record the invalidation.
    /// 3. Remove the entry from the registry.
    ///
    /// Returns the list of [`Invalidation`]s performed. The caller is
    /// responsible for any external notification (event log emission,
    /// Go-side handle drop).
    pub fn notify_node_failure(
        &mut self,
        failed_nodes: &[NodeId],
    ) -> Vec<Invalidation> {
        if failed_nodes.is_empty() {
            return Vec::new();
        }

        // Collect handles that need invalidation first (to avoid borrow
        // conflicts on `self.inner`).
        let handles_to_invalidate: Vec<SurfaceHandle> = self
            .inner
            .iter()
            .filter_map(|(handle, entry)| {
                let current_binding = entry.lease.current.as_ref()?;
                if failed_nodes.contains(&current_binding.step_node) {
                    Some(*handle)
                } else {
                    None
                }
            })
            .collect();

        let mut invalidations = Vec::with_capacity(handles_to_invalidate.len());

        for handle in handles_to_invalidate {
            if let Some(entry) = self.inner.get_mut(&handle) {
                let host_node = entry
                    .lease
                    .current
                    .as_ref()
                    .map(|b| b.step_node.clone())
                    .unwrap_or_else(|| NodeId::new(""));
                let reason = LeaseExitReason::HostFailure { host_node };

                // Extract binding_id and epoch from the current binding
                // before failing the lease.
                let (binding_id, epoch) = entry
                    .lease
                    .current
                    .as_ref()
                    .map(|b| (b.binding_id, b.bound_at_epoch))
                    .unwrap_or_else(|| (Uuid::nil(), 0));

                // Fail the lease (Active → Failed).
                let _ = surface_ops::fail(&mut entry.lease, reason.clone());

                invalidations.push(Invalidation {
                    handle,
                    binding_id,
                    reason,
                    epoch,
                });
            }

            // Remove from registry (entry is now in Failed state).
            self.inner.remove(&handle);
        }

        invalidations
    }
}

// ===========================================================================
// Unit tests (spec 024, §S024-06)
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::{make_step, make_plan, TopologyBuilder};
    use crate::surface_ops::{bind, new_lease};
    use crate::{LocalityTier, TrustLevel};

    fn sample_spec(name: &str) -> SurfaceSpec {
        SurfaceSpec {
            name: name.to_string(),
            protocol: crate::surface::SurfaceProtocol::Posix,
            capture: Some(crate::surface::CaptureDirection::Source),
            locality_floor: LocalityTier::L5Loopback,
            refresh_hz: None,
            audio_sample_rate_hz: None,
            requires_rt_island: false,
            strict_epoch_binding: false,
            min_host_trust: TrustLevel::Untrusted,
            expires_at: None,
        }
    }

    fn two_node_topology() -> (crate::model::Topology, NodeId, NodeId) {
        let a = NodeId::new("node-a");
        let b = NodeId::new("node-b");
        let topo = TopologyBuilder::new()
            .with_name("test-registry")
            .add(crate::model::Node::new(a.clone(), LocalityTier::L5Loopback))
            .add(crate::model::Node::new(b.clone(), LocalityTier::L5Loopback))
            .connect("node-a", "node-b", LocalityTier::L1SameNuma)
            .build();
        (topo, a, b)
    }

    #[test]
    fn registry_insert_get_remove_round_trip() {
        let mut reg = SurfaceRegistry::new();
        assert!(reg.is_empty());

        let handle = SurfaceHandle::new();
        let spec = sample_spec("rt-1");
        let lease = new_lease(spec.clone()).unwrap();

        reg.insert(handle, lease, spec.clone());
        assert_eq!(reg.len(), 1);
        assert!(!reg.is_empty());

        let entry = reg.get(&handle).expect("entry must exist");
        assert_eq!(entry.handle, handle);
        assert_eq!(entry.spec.name, "rt-1");

        let removed = reg.remove(&handle).expect("must return removed");
        assert_eq!(removed.handle, handle);
        assert!(reg.is_empty());
    }

    #[test]
    fn registry_notify_node_failure_invalidates_touching_leases() {
        let (topo, a, _b) = two_node_topology();
        let mut reg = SurfaceRegistry::new();

        let spec = sample_spec("rt-a");
        let mut lease = new_lease(spec.clone()).unwrap();
        let plan = crate::compile(&topo, &crate::builder::IntentBuilder::new().name("t").min_trust(TrustLevel::Untrusted).build())
            .unwrap();
        let step = make_step("node-a", "compute");
        bind(&mut lease, plan.id.clone(), step).unwrap();

        let handle = lease.handle;
        reg.insert(handle, lease, spec);

        let invalidations = reg.notify_node_failure(&[a.clone()]);
        assert_eq!(invalidations.len(), 1);
        assert_eq!(invalidations[0].handle, handle);
        assert!(reg.is_empty());
    }

    #[test]
    fn registry_notify_node_failure_returns_invalidation_list() {
        let (topo, a, b) = two_node_topology();
        let mut reg = SurfaceRegistry::new();

        // Lease on node-a
        let spec_a = sample_spec("rt-a");
        let mut lease_a = new_lease(spec_a.clone()).unwrap();
        let intent = crate::builder::IntentBuilder::new().name("t").min_trust(TrustLevel::Untrusted).build();
        let plan = crate::compile(&topo, &intent).unwrap();
        bind(&mut lease_a, plan.id.clone(), make_step("node-a", "compute")).unwrap();
        let h_a = lease_a.handle;
        reg.insert(h_a, lease_a, spec_a);

        // Lease on node-b
        let spec_b = sample_spec("rt-b");
        let mut lease_b = new_lease(spec_b.clone()).unwrap();
        let plan2 = crate::compile(&topo, &intent).unwrap();
        bind(&mut lease_b, plan2.id.clone(), make_step("node-b", "compute")).unwrap();
        let h_b = lease_b.handle;
        reg.insert(h_b, lease_b, spec_b);

        // Fail only node-a
        let invalidations = reg.notify_node_failure(&[a]);
        assert_eq!(invalidations.len(), 1);
        assert_eq!(invalidations[0].handle, h_a);
        // node-b lease must still be in registry
        assert_eq!(reg.len(), 1);
        assert!(reg.get(&h_b).is_some());
        let _ = b;
    }

    #[test]
    fn bind_with_topology_resolves_endpoint() {
        let (topo, a, _b) = two_node_topology();
        let mut lease = new_lease(sample_spec("rt-resolve")).unwrap();

        let result = crate::surface_ops::bind_with_topology(
            &mut lease,
            crate::model::RoutePlanId::new(),
            make_step("node-a", "compute"),
            &topo,
        );
        assert!(result.is_ok());
        assert_eq!(lease.state, LeaseState::Active);
        let binding = lease.current.as_ref().expect("must have binding");
        assert_eq!(binding.step_node, a);
        let _ = _b;
    }

    #[test]
    fn bind_with_topology_rejects_unknown_node() {
        let (topo, _a, _b) = two_node_topology();
        let mut lease = new_lease(sample_spec("rt-unknown")).unwrap();

        let result = crate::surface_ops::bind_with_topology(
            &mut lease,
            crate::model::RoutePlanId::new(),
            make_step("nonexistent-node", "compute"),
            &topo,
        );
        assert!(matches!(result, Err(SurfaceError::UnknownNode { .. })));
        assert_eq!(lease.state, LeaseState::Pending);
    }

    #[test]
    fn invalidation_to_wire_json_matches_spec025_shape() {
        use crate::model::RoutePlanId;

        let (topo, _a, _b) = two_node_topology();
        let mut reg = SurfaceRegistry::new();

        let spec = sample_spec("rt-wire");
        let mut lease = new_lease(spec.clone()).unwrap();
        let plan = crate::compile(
            &topo,
            &crate::builder::IntentBuilder::new()
                .name("t")
                .min_trust(TrustLevel::Untrusted)
                .build(),
        )
        .unwrap();
        bind(&mut lease, plan.id.clone(), make_step("node-a", "compute")).unwrap();

        let binding_id = lease.current.as_ref().unwrap().binding_id;
        let epoch = lease.current.as_ref().unwrap().bound_at_epoch;
        let handle = lease.handle;
        reg.insert(handle, lease, spec);

        let invalidations = reg.notify_node_failure(&[NodeId::new("node-a")]);
        assert_eq!(invalidations.len(), 1);

        let wire = invalidations[0].to_wire_json();

        // Verify wire shape matches spec 025 SurfaceInvalidate
        assert!(wire.get("surface_handle").is_some());
        assert!(wire.get("lease_id").is_some());
        assert_eq!(wire["reason"], "HostFailure");
        assert_eq!(wire["failed_node"], "node-a");
        assert_eq!(wire["epoch"], epoch);

        // Verify UUIDs are strings
        assert!(wire["surface_handle"].is_string());
        assert!(wire["lease_id"].is_string());

        // Verify lease_id matches the binding_id from the lease
        assert_eq!(
            wire["lease_id"].as_str().unwrap(),
            binding_id.to_string()
        );
    }

    #[test]
    fn invalidation_operator_revoked_maps_to_revoked_wire_reason() {
        let inv = Invalidation {
            handle: SurfaceHandle::new(),
            binding_id: Uuid::nil(),
            reason: LeaseExitReason::OperatorRevoked,
            epoch: 7,
        };
        let wire = inv.to_wire_json();
        assert_eq!(wire["reason"], "Revoked");
        assert!(wire.get("failed_node").is_none() || wire["failed_node"].as_str().unwrap().is_empty());
        assert_eq!(wire["epoch"], 7);
    }

    #[test]
    fn invalidation_expired_maps_to_expired_wire_reason() {
        let inv = Invalidation {
            handle: SurfaceHandle::new(),
            binding_id: Uuid::nil(),
            reason: LeaseExitReason::Expired,
            epoch: 0,
        };
        let wire = inv.to_wire_json();
        assert_eq!(wire["reason"], "Expired");
        assert!(wire.get("failed_node").is_none() || wire["failed_node"].as_str().unwrap().is_empty());
    }

    #[test]
    fn invalidation_workload_reported_maps_to_failed_wire_reason() {
        let inv = Invalidation {
            handle: SurfaceHandle::new(),
            binding_id: Uuid::nil(),
            reason: LeaseExitReason::WorkloadReported {
                code: "OOM".to_string(),
                message: "out of memory".to_string(),
            },
            epoch: 0,
        };
        let wire = inv.to_wire_json();
        assert_eq!(wire["reason"], "Failed");
        assert!(wire.get("failed_node").is_none() || wire["failed_node"].as_str().unwrap().is_empty());
    }

    #[test]
    fn invalidation_normal_completion_maps_to_normal_completion() {
        let inv = Invalidation {
            handle: SurfaceHandle::new(),
            binding_id: Uuid::nil(),
            reason: LeaseExitReason::NormalCompletion,
            epoch: 0,
        };
        let wire = inv.to_wire_json();
        assert_eq!(wire["reason"], "NormalCompletion");
        assert!(wire.get("failed_node").is_none() || wire["failed_node"].as_str().unwrap().is_empty());
    }

    #[test]
    fn invalidation_epoch_drift_maps_to_epoch_drift() {
        let inv = Invalidation {
            handle: SurfaceHandle::new(),
            binding_id: Uuid::nil(),
            reason: LeaseExitReason::EpochDrift {
                previous_epoch: 1,
                new_epoch: 5,
            },
            epoch: 5,
        };
        let wire = inv.to_wire_json();
        assert_eq!(wire["reason"], "EpochDrift");
        assert!(wire.get("failed_node").is_none() || wire["failed_node"].as_str().unwrap().is_empty());
        assert_eq!(wire["epoch"], 5);
    }
}
