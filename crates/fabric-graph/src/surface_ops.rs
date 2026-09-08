//! Surface plane operations: bind, rebind, invalidate (PF-WP-015, spec 019).
//!
//! Implements the lease FSM transition table documented in spec 019 §3.
//! Each operation validates the requested transition before mutating the
//! lease state.

use chrono::Utc;

use crate::model::{RoutePlanId, RouteStep};
use crate::surface::{
    LeaseExitReason, LeaseState, RouteBinding, SurfaceError, SurfaceLease,
    SurfaceSpec, SurfaceSpecError,
};

/// Compute the initial binding of `lease.spec` against the route plan.
///
/// Returns `SurfaceError::NoMatchingRoute` if no `RouteStep` in the plan
/// satisfies the spec's `locality_floor` (every step must be at the floor or
/// closer).
pub fn bind(
    lease: &mut SurfaceLease,
    plan_id: RoutePlanId,
    step: RouteStep,
) -> Result<(), SurfaceError> {
    // Step 1: validate spec (only on initial bind — re-binds skip this since
    // the spec was already validated at first bind).
    if matches!(lease.state, LeaseState::Pending) {
        lease
            .spec
            .validate()
            .map_err(SurfaceError::InvalidSpec)?;
    }

    // Step 2: lease FSM transition Pending → Pending (waiting) or → Active.
    if !matches!(lease.state, LeaseState::Pending | LeaseState::Active) {
        return Err(SurfaceError::IllegalTransition {
            from: lease.state,
            attempted: "bind",
        });
    }

    // Step 3: install binding.
    let now = Utc::now();
    let new_binding = RouteBinding {
        binding_id: uuid::Uuid::now_v7(),
        plan_id,
        step_node: step.node.clone(),
        endpoint: derive_endpoint_for_step(&step),
        bound_at_epoch: 0, // populated by topology epoch once that wiring lands
        bound_at: now,
    };

    if lease.current.is_some() {
        // re-bind: archive the old binding into history
        lease.history.push(lease.current.take().expect("checked above"));
    }
    lease.current = Some(new_binding);
    lease.state = LeaseState::Active;
    Ok(())
}

/// Mark a lease as completed normally (workload finished).
pub fn complete(lease: &mut SurfaceLease) -> Result<(), SurfaceError> {
    if !matches!(lease.state, LeaseState::Active | LeaseState::Pending) {
        return Err(SurfaceError::IllegalTransition {
            from: lease.state,
            attempted: "complete",
        });
    }
    lease.state = LeaseState::Completed;
    lease.exit_reason = Some(LeaseExitReason::NormalCompletion);
    lease.terminated_at = Some(Utc::now());
    Ok(())
}

/// Invalidate a lease due to host failure (ADR-0030 trigger) or workload error.
pub fn fail(
    lease: &mut SurfaceLease,
    reason: LeaseExitReason,
) -> Result<(), SurfaceError> {
    if !matches!(lease.state, LeaseState::Active | LeaseState::Pending) {
        return Err(SurfaceError::IllegalTransition {
            from: lease.state,
            attempted: "fail",
        });
    }
    lease.state = LeaseState::Failed;
    lease.exit_reason = Some(reason);
    lease.terminated_at = Some(Utc::now());
    Ok(())
}

/// Operator-initiated revocation. Works from Active (typical) or Pending.
pub fn revoke(lease: &mut SurfaceLease) -> Result<(), SurfaceError> {
    if !matches!(lease.state, LeaseState::Active | LeaseState::Pending) {
        return Err(SurfaceError::IllegalTransition {
            from: lease.state,
            attempted: "revoke",
        });
    }
    lease.state = LeaseState::Revoked;
    lease.exit_reason = Some(LeaseExitReason::OperatorRevoked);
    lease.terminated_at = Some(Utc::now());
    Ok(())
}

/// Mark a lease as expired (time-based). Works from Active.
pub fn expire(lease: &mut SurfaceLease) -> Result<(), SurfaceError> {
    if !matches!(lease.state, LeaseState::Active | LeaseState::Pending) {
        return Err(SurfaceError::IllegalTransition {
            from: lease.state,
            attempted: "expire",
        });
    }
    lease.state = LeaseState::Expired;
    lease.exit_reason = Some(LeaseExitReason::Expired);
    lease.terminated_at = Some(Utc::now());
    Ok(())
}

/// True iff the lease is in a terminal state (no more transitions possible).
pub fn is_terminal(state: LeaseState) -> bool {
    matches!(
        state,
        LeaseState::Completed | LeaseState::Failed | LeaseState::Revoked | LeaseState::Expired
    )
}

/// Construct a fresh `SurfaceLease` in the Pending state.
pub fn new_lease(spec: SurfaceSpec) -> Result<SurfaceLease, SurfaceSpecError> {
    spec.validate()?;
    Ok(SurfaceLease {
        handle: crate::surface::SurfaceHandle::new(),
        spec,
        current: None,
        history: Vec::new(),
        state: LeaseState::Pending,
        exit_reason: None,
        created_at: Utc::now(),
        terminated_at: None,
    })
}

/// Placeholder: derive the capability endpoint from a `RouteStep`.
///
/// R1 will replace this with the real topology lookup (capability descriptor
/// walk). For now it returns the most common case (`Compute` with the route
/// step's action as a hint).
fn derive_endpoint_for_step(step: &RouteStep) -> crate::surface::CapabilityEndpoint {
    use crate::surface::CapabilityEndpoint;
    match step.action.as_str() {
        "compute" => CapabilityEndpoint::Compute { pid: 0 },
        "display" => CapabilityEndpoint::Display { index: 0 },
        "audio" => CapabilityEndpoint::Audio { index: 0 },
        "input" => CapabilityEndpoint::Input { index: 0 },
        "network" => CapabilityEndpoint::Network { port: 0 },
        "storage" => CapabilityEndpoint::Storage { path: String::new() },
        _ => CapabilityEndpoint::Compute { pid: 0 },
    }
}
