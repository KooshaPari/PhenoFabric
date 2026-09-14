//! The `rebind_or_fail` function -- canonical failover integration entry point.

use crate::failover::{replan, FailoverError, FailoverOutcome};
use crate::model::{Intent, NodeId, RoutePlan, RoutePlanId, RouteStep, Topology};
use crate::surface::{SurfaceError, SurfaceLease, SurfaceSpecError};
use crate::surface_ops::{bind, fail};

use super::types::RebindOutcome;
use crate::surface::LeaseExitReason;

/// Rebind or fail the lease, depending on whether `failover::replan`
/// could find a replacement route on the (caller-pruned) post-failure
/// topology.
///
/// See the [module-level documentation](super) for the contract.
///
/// # Errors
///
/// - `SurfaceError::EpochDrift` -- strict-epoch check failed. Lease unchanged.
/// - `SurfaceError::NoMatchingRoute` -- `failover::replan` returned
///   `FailoverError::AllCandidatesFailed`. Lease unchanged.
/// - `SurfaceError::InvalidSpec(SurfaceSpecError::EmptyName)` --
///   `failover::replan` returned `FailoverError::EmptyIntent`. Lease
///   unchanged.
/// - `SurfaceError::IllegalTransition` -- the lease FSM guard rejected the
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
    // equality here. Keeping the parameter preserves the spec 020 \u00a73
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

/// Map a `FailoverError` to the closest `SurfaceError`.
///
/// This is a best-effort mapping: `FailoverError` is about topology-level
/// futures and `SurfaceError` is about surface-level futures. The two
/// domains overlap at exactly two points -- empty intent and no
/// candidates -- and those are the mappings below. Other
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
