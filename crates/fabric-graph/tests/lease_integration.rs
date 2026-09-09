//! Integration tests for `fabric_graph::leases::rebind_or_fail`.
//!
//! Spec 020 §5 mandates the integration suite. Each test exercises the
//! full pipeline (`failover::replan` + `surface_ops::bind`/`fail` + the
//! `SurfaceError`/`RebindOutcome` contract) end-to-end on a real
//! `Topology` built by `TopologyBuilder`.
//!
//! Reader's guide (per ADR-0028): tests use only the public API.
//! Internal helpers are tested in the in-module `tests` mod.

use fabric_capability::LocalityTier;
use fabric_graph::{
    builder::{make_step, IntentBuilder, TopologyBuilder},
    compile,
    leases::{rebind_or_fail, RebindOutcome},
    surface::{
        LeaseExitReason, LeaseState, SurfaceError, SurfaceLease, SurfaceProtocol, SurfaceSpec,
        SurfaceSpecError,
    },
    surface_ops::{is_terminal, new_lease},
    NodeId, RoutePlanId, Topology, TopologyEpoch, TrustLevel,
};

// ---------------------------------------------------------------------------
// Helpers (mirror of in-module helpers; duplicates are intentional so this
// file is independently readable).
// ---------------------------------------------------------------------------

fn valid_spec(name: &str) -> SurfaceSpec {
    SurfaceSpec {
        name: name.to_string(),
        protocol: SurfaceProtocol::WebRtc,
        capture: None,
        locality_floor: LocalityTier::L2CrossNumaShm,
        refresh_hz: Some(60),
        audio_sample_rate_hz: None,
        requires_rt_island: false,
        strict_epoch_binding: false,
        min_host_trust: TrustLevel::Attested,
        expires_at: None,
    }
}

fn intent(name: &str) -> fabric_graph::Intent {
    IntentBuilder::new()
        .name(name)
        .min_trust(TrustLevel::Untrusted)
        .build()
}

/// 3-node topology `a -- b -- c`. After `a` fails, only `b -> c` remains.
/// After `a` and `b` fail, only `c` remains.
fn three_node_topology() -> (Topology, NodeId, NodeId, NodeId) {
    let a = NodeId::new("a");
    let b = NodeId::new("b");
    let c = NodeId::new("c");
    let topo = TopologyBuilder::new()
        .with_name("integration-test")
        .add_simple_node("a", LocalityTier::L1SameNuma)
        .add_simple_node("b", LocalityTier::L1SameNuma)
        .add_simple_node("c", LocalityTier::L1SameNuma)
        .connect("a", "b", LocalityTier::L1SameNuma)
        .connect("b", "c", LocalityTier::L1SameNuma)
        .build();
    (topo, a, b, c)
}

fn topology_with_only(nodes: &[(&str, LocalityTier)]) -> Topology {
    let mut b = TopologyBuilder::new().with_name("integration-post-failure");
    for (name, tier) in nodes {
        b = b.add_simple_node(name, *tier);
    }
    b.build()
}

/// Pre-bind a lease to (node_a, tier) on `original_plan` at the given epoch.
/// Mirrors the in-module helper for the integration path.
fn prebound_lease(spec: SurfaceSpec, step_node: NodeId, epoch: u64) -> SurfaceLease {
    let step = make_step(step_node.0.as_str(), "compute");
    let mut lease = new_lease(spec).expect("spec validated");
    fabric_graph::surface_ops::bind(&mut lease, RoutePlanId::new(), step)
        .expect("Pending -> Active is allowed");
    if let Some(ref mut binding) = lease.current {
        binding.bound_at_epoch = epoch;
    }
    lease
}

// ---------------------------------------------------------------------------
// T-L01 — silent re-bind on replacement
// ---------------------------------------------------------------------------

#[test]
fn integration_silent_rebind_preserves_handle_and_history() {
    let (orig_topo, a, b, _c) = three_node_topology();
    let i = intent("i-t01");
    let original_plan = compile(&orig_topo, &i).expect("compile 3-node");

    // Post-failure: only `b` and `c` remain (a failed).
    let post = topology_with_only(&[("b", LocalityTier::L1), ("c", LocalityTier::L1)]);
    let original_intent_id = original_plan.intent_id.clone();

    let mut lease = prebound_lease(valid_spec("i-t01-spec"), a.clone(), orig_topo.epoch.0);
    let original_handle = lease.handle;

    let outcome = rebind_or_fail(
        &mut lease,
        RoutePlanId::new(),
        make_step("b", "compute"),
        &post,
        &i,
        &original_plan,
        &[a.clone()],
    )
    .expect("rebind should succeed");

    // The outcome should be Rebound.
    let new_plan_id = match outcome {
        RebindOutcome::Rebound { new_plan_id } => new_plan_id,
        other => panic!("expected Rebound, got: {other:?}"),
    };
    assert_ne!(new_plan_id, original_plan.id, "must be a new plan");

    // Lease invariants after silent re-bind (spec 020 §3):
    assert_eq!(lease.state, LeaseState::Active);
    assert_eq!(lease.handle, original_handle, "handle must be preserved");
    assert!(lease.current.is_some(), "must have a new binding");

    // The new binding must reference the surviving node.
    let binding = lease.current.as_ref().expect("just checked");
    assert_eq!(binding.step_node, b);

    // The prior binding must be in history.
    assert_eq!(lease.history.len(), 1, "prior binding should be in history");
    assert_eq!(
        lease.history[0].step_node, a,
        "history entry should reference the original node"
    );

    // Caller invariants: original_plan unchanged, intent unchanged.
    assert_eq!(original_plan.intent_id, original_intent_id);
}

// ---------------------------------------------------------------------------
// T-L02 — loud fail when no replacement
// ---------------------------------------------------------------------------

#[test]
fn integration_loud_fail_on_no_replacement_terminates_lease() {
    let (orig_topo, a, _b, _c) = three_node_topology();
    let i = intent("i-t02");
    let original_plan = compile(&orig_topo, &i).expect("compile 3-node");

    // Post-failure: empty topology — no replacement possible.
    let post = TopologyBuilder::new().with_name("i-t02-empty").build();
    let mut lease = prebound_lease(valid_spec("i-t02-spec"), a.clone(), orig_topo.epoch.0);
    let original_handle = lease.handle;

    let outcome = rebind_or_fail(
        &mut lease,
        RoutePlanId::new(),
        make_step("none", "compute"),
        &post,
        &i,
        &original_plan,
        &[a.clone()],
    )
    .expect("rebind should return Ok(Failed), not Err");

    match &outcome {
        RebindOutcome::Failed { reason } => match reason {
            LeaseExitReason::HostFailure { host_node } => {
                assert_eq!(
                    host_node, &a,
                    "reason must name the failed node from the input list"
                );
            }
            other => panic!("expected HostFailure, got: {other:?}"),
        },
        other => panic!("expected Failed, got: {other:?}"),
    }

    // Lease invariants after loud fail (spec 020 §3):
    assert_eq!(lease.state, LeaseState::Failed);
    assert!(
        is_terminal(lease.state),
        "Failed is a terminal state per spec 019"
    );
    assert!(lease.exit_reason.is_some(), "exit_reason must be populated");
    assert_eq!(
        lease.handle, original_handle,
        "handle must be preserved even on Failed"
    );
}

// ---------------------------------------------------------------------------
// T-L03 — strict-epoch enforcement
// ---------------------------------------------------------------------------

#[test]
fn integration_strict_epoch_drift_short_circuits_replan() {
    let (orig_topo, a, _b, _c) = three_node_topology();
    let i = intent("i-t03");
    let original_plan = compile(&orig_topo, &i).expect("compile 3-node");

    // Post-failure topology whose epoch differs from `orig_topo.epoch`.
    let mut post_topo = TopologyBuilder::new()
        .with_name("i-t03-post")
        .add_simple_node("b", LocalityTier::L1SameNuma)
        .add_simple_node("c", LocalityTier::L1SameNuma)
        .build();
    // Add an extra node to bump epoch (TopologyBuilder::add_node bumps epoch).
    post_topo.add_node(fabric_graph::Node::new(
        NodeId::new("extra"),
        LocalityTier::L2CrossNumaShm,
    ));
    let post_epoch = post_topo.epoch.0;

    let mut spec = valid_spec("i-t03-spec");
    spec.strict_epoch_binding = true;
    let mut lease = prebound_lease(spec, a.clone(), orig_topo.epoch.0);
    let original_state = lease.state;
    let original_history_len = lease.history.len();

    let result = rebind_or_fail(
        &mut lease,
        RoutePlanId::new(),
        make_step("b", "compute"),
        &post_topo,
        &i,
        &original_plan,
        &[a.clone()],
    );

    match result {
        Err(SurfaceError::EpochDrift { previous, current }) => {
            assert_eq!(previous, orig_topo.epoch.0);
            assert_eq!(current, post_epoch);
        }
        other => panic!("expected EpochDrift err, got: {other:?}"),
    }

    // Lease must be unchanged — strict-epoch check runs BEFORE replan(),
    // so no bind/fail should have happened.
    assert_eq!(lease.state, original_state);
    assert_eq!(
        lease.history.len(),
        original_history_len,
        "no re-bind should have happened"
    );
}

// ---------------------------------------------------------------------------
// T-L04 — non-strict epoch allows re-bind across epochs
// ---------------------------------------------------------------------------

#[test]
fn integration_non_strict_epoch_allows_rebind_across_epochs() {
    let (orig_topo, a, _b, _c) = three_node_topology();
    let i = intent("i-t04");
    let original_plan = compile(&orig_topo, &i).expect("compile 3-node");

    // Post-failure across an epoch bump — without strict_epoch_binding.
    let mut post_topo = TopologyBuilder::new()
        .with_name("i-t04-post")
        .add_simple_node("b", LocalityTier::L1SameNuma)
        .build();
    post_topo.add_node(fabric_graph::Node::new(
        NodeId::new("extra"),
        LocalityTier::L2CrossNumaShm,
    ));

    let mut spec = valid_spec("i-t04-spec");
    spec.strict_epoch_binding = false;
    let mut lease = prebound_lease(spec, a.clone(), orig_topo.epoch.0);

    let outcome = rebind_or_fail(
        &mut lease,
        RoutePlanId::new(),
        make_step("b", "compute"),
        &post_topo,
        &i,
        &original_plan,
        &[a.clone()],
    )
    .expect("non-strict re-bind should succeed even across epoch");

    assert!(
        matches!(outcome, RebindOutcome::Rebound { .. }),
        "non-strict must allow re-bind; got: {outcome:?}"
    );
    assert_eq!(lease.state, LeaseState::Active);
    assert_eq!(lease.history.len(), 1, "prior binding in history");
}

// ---------------------------------------------------------------------------
// T-L05 — lease remains intact when replan errors out
// ---------------------------------------------------------------------------

#[test]
fn integration_lease_intact_when_failover_error_returns() {
    let (orig_topo, a, _b, _c) = three_node_topology();
    let original_plan = compile(&orig_topo, &intent("i-t05")).expect("compile");

    // Empty intent name → FailoverError::EmptyIntent → SurfaceError.
    let empty_intent = IntentBuilder::new().name("").min_trust(TrustLevel::Untrusted).build();
    let post = topology_with_only(&[("b", LocalityTier::L1)]);

    let mut lease = prebound_lease(valid_spec("i-t05-spec"), a.clone(), orig_topo.epoch.0);
    let original_handle = lease.handle;
    let original_history_len = lease.history.len();

    let result = rebind_or_fail(
        &mut lease,
        RoutePlanId::new(),
        make_step("b", "compute"),
        &post,
        &empty_intent,
        &original_plan,
        &[a.clone()],
    );

    match result {
        Err(SurfaceError::InvalidSpec(SurfaceSpecError::EmptyName)) => {}
        other => panic!("expected InvalidSpec(EmptyName), got: {other:?}"),
    }

    // Lease must be entirely unchanged.
    assert_eq!(lease.handle, original_handle);
    assert_eq!(lease.history.len(), original_history_len);
    assert_eq!(
        lease.state,
        LeaseState::Active,
        "lease was Active before call, must still be Active"
    );
}

// ---------------------------------------------------------------------------
// T-L06 — end-to-end: serde round-trip of the outcome
// ---------------------------------------------------------------------------

#[test]
fn integration_outcome_serde_round_trip_end_to_end() {
    // Round-trip the two outcome variants through JSON to confirm the wire
    // format a future workspace event log would consume is stable.
    let rebound = RebindOutcome::Rebound {
        new_plan_id: RoutePlanId::new(),
    };
    let s = serde_json::to_string(&rebound).unwrap();
    let back: RebindOutcome = serde_json::from_str(&s).unwrap();
    assert_eq!(back, rebound);

    let failed = RebindOutcome::Failed {
        reason: LeaseExitReason::HostFailure {
            host_node: NodeId::new("dead"),
        },
    };
    let s2 = serde_json::to_string(&failed).unwrap();
    let back2: RebindOutcome = serde_json::from_str(&s2).unwrap();
    assert_eq!(back2, failed);
}

// ---------------------------------------------------------------------------
// T-L07 — epoch monotonicity: TopologyEpoch zero-initialized works
// ---------------------------------------------------------------------------

#[test]
fn integration_zero_epoch_round_trips_through_rebind() {
    // TopologyEpoch::default() is (0, "v1"). Compile on this topology then
    // rebind on the same topology (no failure) must NOT trigger epoch drift
    // even with strict_epoch_binding=true, because the prior_epoch == current.
    let orig_topo = TopologyBuilder::new()
        .with_name("i-t07")
        .add_simple_node("a", LocalityTier::L1SameNuma)
        .add_simple_node("b", LocalityTier::L1SameNuma)
        .connect("a", "b", LocalityTier::L1SameNuma)
        .build();
    let i = intent("i-t07");
    let original_plan = compile(&orig_topo, &i).expect("compile");

    let mut spec = valid_spec("i-t07-spec");
    spec.strict_epoch_binding = true;

    let mut lease = prebound_lease(spec, NodeId::new("a"), orig_topo.epoch.0);
    // Sanity: prior_epoch captured on the lease matches the topology's epoch.
    let prior_epoch = lease
        .current
        .as_ref()
        .expect("just bound")
        .bound_at_epoch;
    assert_eq!(prior_epoch, orig_topo.epoch.0);

    let outcome = rebind_or_fail(
        &mut lease,
        RoutePlanId::new(),
        make_step("b", "compute"),
        &orig_topo,
        &i,
        &original_plan,
        &[NodeId::new("a")],
    )
    .expect("strict re-bind on same epoch must succeed");

    assert!(
        matches!(outcome, RebindOutcome::Rebound { .. }),
        "same-epoch strict re-bind must succeed; got: {outcome:?}"
    );
}
