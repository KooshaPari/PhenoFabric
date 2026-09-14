//! Route lease integration -- the single integration seam that ties
//! `failover::replan()` (ADR-0030, PF-WP-021) to the surface plane
//! (spec 019, PF-WP-015).
//!
//! Spec 020 (PF-WP-022) defines `rebind_or_fail` as the canonical entry
//! point where a runtime can wire a failover event to a held surface
//! lease. The function takes a currently-bound lease plus a post-failure
//! topology, and returns a `RebindOutcome` reporting whether the lease
//! was silently re-bound (handle unchanged, prior binding rotated into
//! history) or whether it must now be dropped (lease transitioned to
//! `LeaseState::Failed`).
//!
//! ## Contract (spec 020 \u00a73)
//!
//! * On `FailoverOutcome::Replaced(new_plan)`:
//!   - If `lease.spec.strict_epoch_binding` is true AND the post-failure
//!     topology's `epoch` differs from the lease's prior `bound_at_epoch`,
//!     return `SurfaceError::EpochDrift { previous, current }` (the
//!     surface must be invalidated; **no silent re-bind**). The check
//!     runs *before* `failover::replan` is even called, so we don't waste
//!     a `compile()` on a binding that would be thrown away.
//!   - Otherwise, call `surface_ops::bind(&mut lease, new_plan.id, new_step)`
//!     to re-bind. The user's `SurfaceHandle` is unchanged; the prior
//!     binding moves to `lease.history`. The lease stays in `Active`.
//!   - Return `RebindOutcome::Rebound { new_plan_id }`.
//! * On `FailoverOutcome::NoReplacement`:
//!   - Call `surface_ops::fail(&mut lease, LeaseExitReason::HostFailure {
//!     host_node: first_failed_node })`. The lease transitions to
//!     `LeaseState::Failed` and the caller is expected to drop the
//!     `SurfaceHandle` and re-admit if desired.
//!   - Return `RebindOutcome::Failed { reason }`.
//! * On `Err(FailoverError::*)`:
//!   - Map to `SurfaceError` (see `map_failover_error`). The lease is
//!     unchanged.
//!
//! `failed_nodes` is the list of `NodeId`s pruned from the topology
//! before this call. It is informational (used to populate
//! `LeaseExitReason::HostFailure`) -- the topology passed in is the
//! post-failure topology.
//!
//! ## Reader's guide
//!
//! Read these four modules before changing this file (ADR-0028):
//!
//! - `crate::failover` -- `replan`, `FailoverError`, `FailoverOutcome`
//! - `crate::surface` -- `SurfaceLease`, `LeaseState`, `LeaseExitReason`,
//!   `SurfaceError`
//! - `crate::surface_ops` -- `bind`, `fail`, `new_lease`
//! - `crate::lease_fsm` -- `can_transition` (the FSM table this module
//!   implicitly obeys)

mod rebind;
mod types;

// Re-export public items for backward compatibility.
pub use rebind::rebind_or_fail;
pub use types::RebindOutcome;

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::{make_plan, make_step, IntentBuilder, TopologyBuilder};
    use crate::compile;
    use crate::surface::{CaptureDirection, SurfaceProtocol, SurfaceSpec};
    use crate::surface_ops::{is_terminal, new_lease};
    use crate::TrustLevel;
    use fabric_capability::LocalityTier;

    // ---------------- helpers ----------------

    fn valid_spec(name: &str) -> SurfaceSpec {
        SurfaceSpec {
            name: name.to_string(),
            protocol: SurfaceProtocol::WebRtc,
            capture: Some(CaptureDirection::Bidirectional),
            locality_floor: LocalityTier::L2CrossNumaShm,
            refresh_hz: Some(60),
            audio_sample_rate_hz: None,
            requires_rt_island: false,
            strict_epoch_binding: false,
            min_host_trust: TrustLevel::Attested,
            expires_at: None,
        }
    }

    fn intent(name: &str) -> crate::model::Intent {
        IntentBuilder::new()
            .name(name)
            .min_trust(TrustLevel::Untrusted)
            .build()
    }

    /// Two-node topology with a single edge. Compile picks the lower
    /// locality tier node first (`L1SameNuma`), so the route step will
    /// be on `a`.
    fn two_node_topology() -> (crate::model::Topology, crate::model::NodeId, crate::model::NodeId) {
        let a = crate::model::NodeId::new("a");
        let b = crate::model::NodeId::new("b");
        let topo = TopologyBuilder::new()
            .with_name("leases-test")
            .add_simple_node("a", LocalityTier::L1SameNuma)
            .add_simple_node("b", LocalityTier::L1SameNuma)
            .connect("a", "b", LocalityTier::L1SameNuma)
            .build();
        (topo, a, b)
    }

    /// Build a lease that's pre-bound to `step` on a plan whose topology
    /// epoch is `epoch`. The `bound_at_epoch` field is set to `epoch` so
    /// the strict-epoch check can compare against the new topology's
    /// epoch deterministically.
    fn prebound_lease(
        spec: SurfaceSpec,
        plan_id: crate::model::RoutePlanId,
        step: crate::model::RouteStep,
        epoch: u64,
    ) -> crate::surface::SurfaceLease {
        let mut lease = new_lease(spec).expect("spec validated");
        // We can't use the public `surface_ops::bind` path here without a
        // compiled plan, so synthesize a valid RouteBinding manually via
        // the public path: bind first, then patch the binding's epoch.
        let _ = plan_id; // bind() generates its own id; we keep the user's id out
        crate::surface_ops::bind(&mut lease, crate::model::RoutePlanId::new(), step)
            .expect("Pending -> Active is allowed");
        // Patch epoch on the resulting binding so tests are deterministic.
        if let Some(ref mut binding) = lease.current {
            binding.bound_at_epoch = epoch;
        }
        lease
    }

    // ---------------- T-L01 ----------------

    #[test]
    fn rebind_replaces_binding_when_replan_succeeds() {
        // 2-node topology, both nodes L1 -- compile picks one, the other
        // is what we'd fall back to if the first is pruned.
        let (orig_topo, _, b) = two_node_topology();
        let i = intent("rebinds-1");
        let original_plan = compile(&orig_topo, &i).expect("compile 2-node");

        // Post-failure topology: only `b` remains.
        let post = TopologyBuilder::new()
            .with_name("rebinds-1-post")
            .add_simple_node("b", LocalityTier::L1SameNuma)
            .build();

        let mut lease = prebound_lease(
            valid_spec("rebinds-1-spec"),
            original_plan.id.clone(),
            make_step("a", "compute"),
            orig_topo.epoch.0,
        );
        let original_handle = lease.handle;
        let original_intent_id = original_plan.intent_id.clone();

        let step_b = make_step("b", "compute");
        let outcome = rebind_or_fail(
            &mut lease,
            crate::model::RoutePlanId::new(),
            step_b,
            &post,
            &i,
            &original_plan,
            &[crate::model::NodeId::new("a")],
        )
        .expect("rebind should succeed");

        match outcome {
            RebindOutcome::Rebound { new_plan_id } => {
                assert_ne!(new_plan_id, original_plan.id, "must be a new plan");
            }
            other => panic!("expected Rebound, got {other:?}"),
        }
        assert_eq!(lease.state, crate::surface::LeaseState::Active);
        assert_eq!(lease.handle, original_handle, "handle must be preserved");
        assert!(lease.current.is_some(), "must have a new binding");
        assert_eq!(
            lease.history.len(),
            1,
            "prior binding should be in history (1 entry)"
        );
        // The new binding should reference the surviving node.
        let binding = lease.current.as_ref().expect("just checked");
        assert_eq!(binding.step_node, b);

        // Sanity: original_plan.intent_id should still equal i.id (we
        // didn't mutate either).
        assert_eq!(original_plan.intent_id, original_intent_id);
    }

    // ---------------- T-L02 ----------------

    #[test]
    fn rebind_returns_failed_when_replan_has_no_replacement() {
        let (orig_topo, a, _b) = two_node_topology();
        let i = intent("rebinds-2");
        let original_plan = compile(&orig_topo, &i).expect("compile 2-node");

        // Post-failure topology: empty (no candidates).
        let post = TopologyBuilder::new().with_name("rebinds-2-post").build();

        let mut lease = prebound_lease(
            valid_spec("rebinds-2-spec"),
            original_plan.id.clone(),
            make_step("a", "compute"),
            orig_topo.epoch.0,
        );
        let original_handle = lease.handle;

        let outcome = rebind_or_fail(
            &mut lease,
            crate::model::RoutePlanId::new(),
            make_step("none", "compute"),
            &post,
            &i,
            &original_plan,
            &[a.clone()],
        )
        .expect("rebind should return Ok(Failed), not Err");

        match &outcome {
            RebindOutcome::Failed { reason } => match reason {
                crate::surface::LeaseExitReason::HostFailure { host_node } => {
                    assert_eq!(host_node, &a, "reason should name the failed node");
                }
                other => panic!("expected HostFailure, got {other:?}"),
            },
            other => panic!("expected Failed, got {other:?}"),
        }
        assert_eq!(
            lease.state,
            crate::surface::LeaseState::Failed,
            "lease must be Failed"
        );
        assert!(
            is_terminal(lease.state),
            "Failed is a terminal state per spec 019"
        );
        assert!(
            lease.exit_reason.is_some(),
            "exit_reason must be populated"
        );
        assert_eq!(
            lease.handle, original_handle,
            "handle must be preserved even on Failed"
        );
    }

    // ---------------- T-L03 ----------------

    #[test]
    fn rebind_returns_epoch_drift_when_strict_binding_and_epoch_advanced() {
        let (orig_topo, _, _b) = two_node_topology();
        let i = intent("rebinds-3");
        let original_plan = compile(&orig_topo, &i).expect("compile 2-node");

        // Build a post-failure topology whose epoch is greater than the
        // lease's bound_at_epoch. Simulate this by adding an extra node
        // so add_node() bumps the epoch.
        let mut post_topo = TopologyBuilder::new()
            .with_name("rebinds-3-post")
            .add_simple_node("b", LocalityTier::L1SameNuma)
            .build();
        // add_node bumps epoch; we use the post-topo's epoch in the call.
        post_topo.add_node(crate::Node::new(
            crate::model::NodeId::new("extra"),
            LocalityTier::L2CrossNumaShm,
        ));
        let post_epoch = post_topo.epoch.0;

        // Lease was bound at orig_topo's epoch (which is less than post_epoch
        // because post_topo added at least one node).
        let mut spec = valid_spec("rebinds-3-spec");
        spec.strict_epoch_binding = true;
        let mut lease = prebound_lease(
            spec,
            original_plan.id.clone(),
            make_step("a", "compute"),
            orig_topo.epoch.0,
        );
        let original_handle = lease.handle;
        let original_state = lease.state;

        let result = rebind_or_fail(
            &mut lease,
            crate::model::RoutePlanId::new(),
            make_step("b", "compute"),
            &post_topo,
            &i,
            &original_plan,
            &[crate::model::NodeId::new("a")],
        );

        match result {
            Err(crate::surface::SurfaceError::EpochDrift { previous, current }) => {
                assert_eq!(previous, orig_topo.epoch.0);
                assert_eq!(current, post_epoch);
            }
            other => panic!("expected EpochDrift err, got {other:?}"),
        }
        // Lease must be unchanged (Active, handle preserved, no history).
        assert_eq!(lease.state, original_state, "lease must be unchanged");
        assert_eq!(lease.handle, original_handle);
        assert_eq!(
            lease.history.len(),
            0,
            "no re-bind should have happened"
        );
    }

    // ---------------- T-L04 ----------------

    #[test]
    fn rebind_propagates_failover_error() {
        let (orig_topo, _, _) = two_node_topology();
        let original_plan =
            compile(&orig_topo, &intent("rebinds-4")).expect("compile");

        // Empty intent name -> FailoverError::EmptyIntent -> SurfaceError.
        let empty_intent = IntentBuilder::new()
            .name("") // empty!
            .min_trust(TrustLevel::Untrusted)
            .build();
        let post = TopologyBuilder::new()
            .with_name("rebinds-4-post")
            .add_simple_node("b", LocalityTier::L1SameNuma)
            .build();

        let mut lease = prebound_lease(
            valid_spec("rebinds-4-spec"),
            original_plan.id.clone(),
            make_step("a", "compute"),
            orig_topo.epoch.0,
        );
        let original_handle = lease.handle;

        let result = rebind_or_fail(
            &mut lease,
            crate::model::RoutePlanId::new(),
            make_step("b", "compute"),
            &post,
            &empty_intent,
            &original_plan,
            &[crate::model::NodeId::new("a")],
        );

        match result {
            Err(crate::surface::SurfaceError::InvalidSpec(
                crate::surface::SurfaceSpecError::EmptyName,
            )) => {}
            other => panic!("expected InvalidSpec(EmptyName), got {other:?}"),
        }
        assert_eq!(
            lease.handle, original_handle,
            "handle must be preserved on error"
        );
        assert_eq!(
            lease.history.len(),
            0,
            "no re-bind should have happened"
        );
    }

    // ---------------- T-L05 ----------------

    #[test]
    fn rebind_outcome_round_trip_via_serde() {
        let rebound = RebindOutcome::Rebound {
            new_plan_id: crate::model::RoutePlanId::new(),
        };
        let json_rebound = serde_json::to_string(&rebound).expect("serialize Rebound");
        let back: RebindOutcome =
            serde_json::from_str(&json_rebound).expect("deserialize Rebound");
        assert_eq!(back, rebound);

        let failed = RebindOutcome::Failed {
            reason: crate::surface::LeaseExitReason::HostFailure {
                host_node: crate::model::NodeId::new("dead-host"),
            },
        };
        let json_failed = serde_json::to_string(&failed).expect("serialize Failed");
        let back2: RebindOutcome =
            serde_json::from_str(&json_failed).expect("deserialize Failed");
        assert_eq!(back2, failed);

        // make_plan is here to silence the unused-import warning when no
        // integration tests reference it; the integration suite uses it.
        let _ = make_plan(
            vec![make_step("x", "y")],
            crate::model::TopologyEpoch(0),
        );
    }
}
