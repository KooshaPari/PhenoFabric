//! Route lease integration — the single integration seam that ties
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
//! ## Contract (spec 020 §3)
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
//! `LeaseExitReason::HostFailure`) — the topology passed in is the
//! post-failure topology.
//!
//! ## Reader's guide
//!
//! Read these four modules before changing this file (ADR-0028):
//!
//! - `crate::failover` — `replan`, `FailoverError`, `FailoverOutcome`
//! - `crate::surface` — `SurfaceLease`, `LeaseState`, `LeaseExitReason`,
//!   `SurfaceError`
//! - `crate::surface_ops` — `bind`, `fail`, `new_lease`
//! - `crate::lease_fsm` — `can_transition` (the FSM table this module
//!   implicitly obeys)

use serde::{Deserialize, Serialize};

use crate::failover::{replan, FailoverError, FailoverOutcome};
use crate::model::{Intent, NodeId, RoutePlan, RoutePlanId, RouteStep, Topology};
use crate::surface::{LeaseExitReason, SurfaceError, SurfaceLease, SurfaceSpecError};
use crate::surface_ops::{bind, fail};

// ---------------------------------------------------------------------------
// RebindOutcome
// ---------------------------------------------------------------------------

/// The result of [`rebind_or_fail`]: did the lease silently re-bind, or
/// did it fail (caller must drop the `SurfaceHandle`)?
///
/// `Rebound` is the "everything worked" case: the handle is unchanged,
/// the prior binding has been rotated into `lease.history`, and a new
/// `RoutePlan` is now bound.
///
/// `Failed` is the "we tried, no replacement exists" case: the lease is
/// now in `LeaseState::Failed` with `exit_reason` populated; the caller
/// MUST drop the handle and re-admit if desired.
///
/// Both variants are `Serialize` so they can flow into the per-surface
/// event log that ADR-0030 §6 reserves for a future wedge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RebindOutcome {
    /// The lease was silently re-bound; prior binding moved to history.
    Rebound { new_plan_id: RoutePlanId },
    /// No replacement was available; the lease is now `Failed` and the
    /// caller MUST drop the handle.
    Failed { reason: LeaseExitReason },
}

// ---------------------------------------------------------------------------
// rebind_or_fail
// ---------------------------------------------------------------------------

/// Rebind or fail the lease, depending on whether `failover::replan`
/// could find a replacement route on the (caller-pruned) post-failure
/// topology.
///
/// See the [module-level documentation](self) for the contract.
///
/// # Errors
///
/// - `SurfaceError::EpochDrift` — strict-epoch check failed. Lease unchanged.
/// - `SurfaceError::NoMatchingRoute` — `failover::replan` returned
///   `FailoverError::AllCandidatesFailed`. Lease unchanged.
/// - `SurfaceError::InvalidSpec(SurfaceSpecError::EmptyName)` —
///   `failover::replan` returned `FailoverError::EmptyIntent`. Lease
///   unchanged.
/// - `SurfaceError::IllegalTransition` — the lease FSM guard rejected the
///   bind/fail the integration wanted to perform (e.g., the lease was
///   already terminal). Lease state depends on which side of the call.
/// - `SurfaceError::NoMatchingRoute` propagated from `surface_ops::bind`
///   (only reachable if the new step doesn't satisfy the spec; not
///   expected under normal operation since `replan` re-uses the
///   post-failure topology).
pub fn rebind_or_fail(
    lease: &mut SurfaceLease,
    plan_id: RoutePlanId,
    new_step: RouteStep,
    post_failure_topology: &Topology,
    intent: &Intent,
    old_plan: &RoutePlan,
    failed_nodes: &[NodeId],
) -> Result<RebindOutcome, SurfaceError> {
    // `plan_id` is reserved for callers that want to assert the new plan's
    // id matched the result of replan(); `rebind_or_fail` re-binds to
    // whatever replan returned (the new plan id), so we don't enforce
    // equality here. Keeping the parameter preserves the spec 020 §3
    // signature for future audits.
    let _ = plan_id;

    // ---- Step 1: strict-epoch pre-check (saves a needless compile()) ----
    let prior_epoch = lease
        .current
        .as_ref()
        .map(|b| b.bound_at_epoch)
        .unwrap_or(0);
    if lease.spec.strict_epoch_binding && post_failure_topology.epoch.0 != prior_epoch {
        return Err(SurfaceError::EpochDrift {
            previous: prior_epoch,
            current: post_failure_topology.epoch.0,
        });
    }

    // ---- Step 2: replan on the post-failure topology ----
    match replan(post_failure_topology, intent, old_plan, failed_nodes) {
        Ok(FailoverOutcome::Replaced(new_plan)) => {
            // ---- Step 3a: silent re-bind ----
            let new_plan_id = new_plan.id.clone();
            bind(lease, new_plan_id.clone(), new_step)?;
            Ok(RebindOutcome::Rebound { new_plan_id })
        }
        Ok(FailoverOutcome::NoReplacement) => {
            // ---- Step 3b: terminate the lease ----
            let host_node = failed_nodes
                .first()
                .cloned()
                .unwrap_or_else(|| NodeId::new(""));
            let reason = LeaseExitReason::HostFailure {
                host_node: host_node.clone(),
            };
            fail(lease, reason.clone())?;
            Ok(RebindOutcome::Failed { reason })
        }
        Err(e) => Err(map_failover_error(e)),
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Map a `FailoverError` to the closest `SurfaceError`.
///
/// This is a best-effort mapping: `FailoverError` is about topology-level
/// futures and `SurfaceError` is about surface-level futures. The two
/// domains overlap at exactly two points — empty intent and no
/// candidates — and those are the mappings below. Other
/// `FailoverError` variants (none exist today) would get a
/// `NoMatchingRoute` mapping.
fn map_failover_error(e: FailoverError) -> SurfaceError {
    match e {
        FailoverError::EmptyIntent => {
            // Empty intent name is a spec-level issue; map to InvalidSpec(EmptyName).
            SurfaceError::InvalidSpec(SurfaceSpecError::EmptyName)
        }
        FailoverError::AllCandidatesFailed => SurfaceError::NoMatchingRoute,
    }
}

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

    fn intent(name: &str) -> Intent {
        IntentBuilder::new()
            .name(name)
            .min_trust(TrustLevel::Untrusted)
            .build()
    }

    /// Two-node topology with a single edge. Compile picks the lower
    /// locality tier node first (`L1SameNuma`), so the route step will
    /// be on `a`.
    fn two_node_topology() -> (Topology, NodeId, NodeId) {
        let a = NodeId::new("a");
        let b = NodeId::new("b");
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
        plan_id: RoutePlanId,
        step: RouteStep,
        epoch: u64,
    ) -> SurfaceLease {
        let mut lease = new_lease(spec).expect("spec validated");
        // We can't use the public `surface_ops::bind` path here without a
        // compiled plan, so synthesize a valid RouteBinding manually via
        // the public path: bind first, then patch the binding's epoch.
        let _ = plan_id; // bind() generates its own id; we keep the user's id out
        crate::surface_ops::bind(&mut lease, RoutePlanId::new(), step)
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
        // 2-node topology, both nodes L1 — compile picks one, the other
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
            RoutePlanId::new(),
            step_b,
            &post,
            &i,
            &original_plan,
            &[NodeId::new("a")],
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
            NodeId::new("extra"),
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
            RoutePlanId::new(),
            make_step("b", "compute"),
            &post_topo,
            &i,
            &original_plan,
            &[NodeId::new("a")],
        );

        match result {
            Err(SurfaceError::EpochDrift { previous, current }) => {
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
        let original_plan = compile(&orig_topo, &intent("rebinds-4")).expect("compile");

        // Empty intent name → FailoverError::EmptyIntent → SurfaceError.
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
            RoutePlanId::new(),
            make_step("b", "compute"),
            &post,
            &empty_intent,
            &original_plan,
            &[NodeId::new("a")],
        );

        match result {
            Err(SurfaceError::InvalidSpec(SurfaceSpecError::EmptyName)) => {}
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
            new_plan_id: RoutePlanId::new(),
        };
        let json_rebound = serde_json::to_string(&rebound).expect("serialize Rebound");
        let back: RebindOutcome =
            serde_json::from_str(&json_rebound).expect("deserialize Rebound");
        assert_eq!(back, rebound);

        let failed = RebindOutcome::Failed {
            reason: LeaseExitReason::HostFailure {
                host_node: NodeId::new("dead-host"),
            },
        };
        let json_failed = serde_json::to_string(&failed).expect("serialize Failed");
        let back2: RebindOutcome =
            serde_json::from_str(&json_failed).expect("deserialize Failed");
        assert_eq!(back2, failed);

        // make_plan is here to silence the unused-import warning when no
        // integration tests reference it; the integration suite uses it.
        let _ = make_plan(vec![make_step("x", "y")], crate::model::TopologyEpoch(0));
    }
}
