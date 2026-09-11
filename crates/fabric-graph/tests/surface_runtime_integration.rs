//! Integration tests for surface_runtime (spec 024, PF-WP-030).
//!
//! These tests verify end-to-end workflows combining SurfaceRegistry,
//! topology compilation, lease binding, and node failure invalidation.

use fabric_graph::builder::{make_step, IntentBuilder, TopologyBuilder};
use fabric_graph::surface::{CaptureDirection, LeaseState, SurfaceProtocol};
use fabric_graph::surface_ops::{bind, new_lease};
use fabric_graph::{compile, LocalityTier, SurfaceHandle, SurfaceRegistry, SurfaceSpec, TrustLevel};

fn sample_spec(name: &str) -> SurfaceSpec {
    SurfaceSpec {
        name: name.to_string(),
        protocol: SurfaceProtocol::Posix,
        capture: Some(CaptureDirection::Source),
        locality_floor: LocalityTier::L5Loopback,
        refresh_hz: None,
        audio_sample_rate_hz: None,
        requires_rt_island: false,
        strict_epoch_binding: false,
        min_host_trust: TrustLevel::Untrusted,
        expires_at: None,
    }
}

/// Build a two-node topology and compile an intent against it.
fn two_node_setup() -> (
    fabric_graph::model::Topology,
    fabric_graph::model::NodeId,
    fabric_graph::model::NodeId,
) {
    let a = fabric_graph::model::NodeId::new("node-a");
    let b = fabric_graph::model::NodeId::new("node-b");
    let topo = TopologyBuilder::new()
        .with_name("integration-test")
        .add(fabric_graph::model::Node::new(
            a.clone(),
            LocalityTier::L5Loopback,
        ))
        .add(fabric_graph::model::Node::new(
            b.clone(),
            LocalityTier::L5Loopback,
        ))
        .connect("node-a", "node-b", LocalityTier::L1SameNuma)
        .build();
    (topo, a, b)
}

#[test]
fn surface_runtime_end_to_end_bind_then_invalidate() {
    let (topo, a, _b) = two_node_setup();
    let intent = IntentBuilder::new()
        .name("test-intent")
        .min_trust(TrustLevel::Untrusted)
        .build();
    let plan = compile(&topo, &intent).expect("compile should succeed");

    // Create a lease bound to node-a.
    let mut lease = new_lease(sample_spec("e2e-1")).expect("new_lease");
    bind(&mut lease, plan.id.clone(), make_step("node-a", "compute"))
        .expect("bind");
    assert_eq!(lease.state, LeaseState::Active);
    assert_eq!(
        lease.current.as_ref().unwrap().step_node,
        a
    );

    // Insert into registry.
    let handle = lease.handle;
    let mut reg = SurfaceRegistry::new();
    reg.insert(handle, lease, sample_spec("e2e-1"));
    assert_eq!(reg.len(), 1);

    // Node-a fails → invalidation.
    let invalidations = reg.notify_node_failure(&[a]);
    assert_eq!(invalidations.len(), 1);
    assert_eq!(invalidations[0].handle, handle);
    assert!(reg.is_empty());
    let _ = _b;
}

#[test]
fn surface_runtime_multi_lease_invalidates_only_touching() {
    let (topo, a, b) = two_node_setup();
    let intent = IntentBuilder::new()
        .name("test-intent")
        .min_trust(TrustLevel::Untrusted)
        .build();
    let plan_a = compile(&topo, &intent).expect("compile");
    let plan_b = compile(&topo, &intent).expect("compile");

    // Lease on node-a.
    let mut lease_a = new_lease(sample_spec("lease-a")).expect("new_lease");
    bind(&mut lease_a, plan_a.id.clone(), make_step("node-a", "compute")).unwrap();
    let h_a = lease_a.handle;

    // Lease on node-b.
    let mut lease_b = new_lease(sample_spec("lease-b")).expect("new_lease");
    bind(&mut lease_b, plan_b.id.clone(), make_step("node-b", "compute")).unwrap();
    let h_b = lease_b.handle;

    let mut reg = SurfaceRegistry::new();
    reg.insert(h_a, lease_a, sample_spec("lease-a"));
    reg.insert(h_b, lease_b, sample_spec("lease-b"));
    assert_eq!(reg.len(), 2);

    // Fail only node-a.
    let invalidations = reg.notify_node_failure(&[a]);
    assert_eq!(invalidations.len(), 1);
    assert_eq!(invalidations[0].handle, h_a);

    // node-b lease must still be in registry.
    assert_eq!(reg.len(), 1);
    assert!(reg.get(&h_b).is_some());
    let _ = b;
}

#[test]
fn surface_runtime_handle_drop_after_invalidation() {
    let (topo, a, _b) = two_node_setup();
    let intent = IntentBuilder::new()
        .name("test-intent")
        .min_trust(TrustLevel::Untrusted)
        .build();
    let plan = compile(&topo, &intent).expect("compile");

    let mut lease = new_lease(sample_spec("drop-1")).expect("new_lease");
    bind(&mut lease, plan.id.clone(), make_step("node-a", "compute")).unwrap();
    let handle = lease.handle;

    let mut reg = SurfaceRegistry::new();
    reg.insert(handle, lease, sample_spec("drop-1"));
    assert!(!reg.is_empty());

    // Invalidate.
    let inv = reg.notify_node_failure(&[a.clone()]);
    assert_eq!(inv.len(), 1);

    // Entry must be removed.
    assert!(reg.get(&handle).is_none());
    assert!(reg.is_empty());
    let _ = _b;
}

#[test]
fn surface_runtime_snapshot_serde_round_trip() {
    let (topo, _a, _b) = two_node_setup();
    let intent = IntentBuilder::new()
        .name("test-intent")
        .min_trust(TrustLevel::Untrusted)
        .build();
    let plan = compile(&topo, &intent).expect("compile");

    let mut lease = new_lease(sample_spec("serde-1")).expect("new_lease");
    bind(&mut lease, plan.id.clone(), make_step("node-a", "compute")).unwrap();

    // Serialize → deserialize round-trip.
    let json = serde_json::to_string(&lease).expect("serialize");
    let restored: fabric_graph::surface::SurfaceLease =
        serde_json::from_str(&json).expect("deserialize");

    assert_eq!(restored.handle, lease.handle);
    assert_eq!(restored.state, LeaseState::Active);
    assert_eq!(restored.spec.name, "serde-1");
    assert!(restored.current.is_some());
}

#[test]
fn surface_runtime_notify_empty_failed_nodes_is_noop() {
    let (topo, _a, _b) = two_node_setup();
    let intent = IntentBuilder::new()
        .name("test-intent")
        .min_trust(TrustLevel::Untrusted)
        .build();
    let plan = compile(&topo, &intent).expect("compile");

    let mut lease = new_lease(sample_spec("noop-1")).expect("new_lease");
    bind(&mut lease, plan.id.clone(), make_step("node-a", "compute")).unwrap();
    let handle = lease.handle;

    let mut reg = SurfaceRegistry::new();
    reg.insert(handle, lease, sample_spec("noop-1"));

    // Notify with empty list → no-op.
    let inv = reg.notify_node_failure(&[]);
    assert!(inv.is_empty());
    assert_eq!(reg.len(), 1);
    assert!(reg.get(&handle).is_some());
}
