//! Workspace — a named container of seat leases sharing a lifecycle and trust scope.
//!
//! See `adr/0025-route-lease-semantic-model.md` §Workspace lifecycle.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use crate::error::WorkspaceError;
use crate::lease::{LeaseId, LeaseState, SeatLease, Transition};

/// Workspace identifier (human-readable name or UUID v7 string).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(pub String);

impl fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::ops::Deref for WorkspaceId {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

/// Trust scope of a workspace.
///
/// Ephemeral workspaces are local-only and cleared on reboot.
/// Persistent workspaces survive host restarts and can span surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrustScope {
    /// Local to the initiating surface; cleared on reboot.
    Ephemeral,
    /// Survives host restarts; may be visible to surface-plane agents.
    Persistent,
    /// Cross-surface trust. Requires out-of-band verification.
    CrossSurface,
}

impl TrustScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            TrustScope::Ephemeral => "ephemeral",
            TrustScope::Persistent => "persistent",
            TrustScope::CrossSurface => "cross-surface",
        }
    }
}

impl fmt::Display for TrustScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Lifecycle state of a workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecycleState {
    /// Workspace created; not yet assigned to a host.
    Unassigned,
    /// Compiled and assigned to at least one host.
    Assigned,
    /// Active leases exist and the plan is running.
    Active,
    /// Plan completed normally; all seats returned.
    Completed,
    /// Plan exited with an error; seats returned.
    Failed,
    /// Workspace was cancelled before any seats were bound.
    Cancelled,
}

impl LifecycleState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            LifecycleState::Completed | LifecycleState::Failed | LifecycleState::Cancelled
        )
    }
}

/// Workspace — a named collection of seat leases sharing lifecycle and trust.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: Option<String>,
    pub plan: Option<RoutePlan>,
    pub topology: Option<Topology>,
    pub state: LifecycleState,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub trust_scope: TrustScope,
    /// Active leases keyed by `LeaseId`.
    #[serde(default)]
    pub leases: HashMap<LeaseId, SeatLease>,
}

impl Workspace {
    /// Create a new workspace. The workspace starts in `Unassigned` state and
    /// must receive a plan via [`assign_plan`](Self::assign_plan) before leases
    /// can be created.
    pub fn new(id: WorkspaceId, name: Option<String>, ttl: Duration, trust_scope: TrustScope) -> Self {
        let now = Utc::now();
        Self {
            id,
            name,
            plan: None,
            topology: None,
            state: LifecycleState::Unassigned,
            created_at: now,
            expires_at: now + ttl,
            trust_scope,
            leases: Default::default(),
        }
    }

    /// Assign a compiled route plan and topology. Moves workspace to `Assigned`.
    /// Must be called before any seat leases can be created.
    pub fn assign_plan(&mut self, plan: RoutePlan, topology: Topology) {
        self.plan = Some(plan);
        self.topology = Some(topology);
        self.state = LifecycleState::Assigned;
    }

    /// Is the workspace active? (has at least one Active lease)
    pub fn is_active(&self) -> bool {
        self.leases.values().any(|l| l.state == LeaseState::Active)
    }

    /// Is the workspace expired?
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires_at
    }

    /// Check if a seat is already held (any non-terminal lease for host+tier).
    pub fn seat_is_held(&self, host: &str, locality_tier: &str) -> bool {
        self.leases.values().any(|l| {
            l.state != LeaseState::Released
                && l.state != LeaseState::Failed
                && l.state != LeaseState::Revoked
                && l.host == host
                && l.locality_tier == locality_tier
        })
    }

    /// Claim a seat for a plan step. Returns the new `SeatLease`.
    ///
    /// Fails if the seat is already held or the workspace is terminal.
    pub fn claim_seat(
        &mut self,
        plan_id: &PlanId,
        host: String,
        locality_tier: String,
        ttl: Duration,
    ) -> Result<&SeatLease> {
        if self.state.is_terminal() {
            return Err(crate::error::Error::WorkspaceTerminal {
                id: self.id.clone(),
                state: self.state,
            });
        }
        if self.seat_is_held(&host, &locality_tier) {
            return Err(crate::error::Error::SeatConflict {
                host: host.clone(),
                locality_tier: locality_tier.clone(),
                holder: "another workspace".into(),
            });
        }
        let lease = SeatLease::new(
            self.id.clone(),
            plan_id,
            host,
            locality_tier,
            ttl,
            self.trust_scope,
        );
        let id = lease.id.clone();
        let slot = self.leases.entry(id.clone()).or_insert(lease);
        // bump to Active
        let now = Utc::now();
        slot.apply(Transition::Activate, now, None)?;
        // workspace state: Assigned → Active (if not already Active)
        if self.state == LifecycleState::Assigned {
            self.state = LifecycleState::Active;
        }
        Ok(self.leases.get(&id).expect("just inserted"))
    }

    /// Release all seats held by this workspace and move to `Completed`.
    pub fn complete(&mut self) -> Result<()> {
        if self.state.is_terminal() {
            return Err(crate::error::Error::WorkspaceTerminal {
                id: self.id.clone(),
                state: self.state,
            });
        }
        let now = Utc::now();
        for lease in self.leases.values_mut() {
            if !lease.state.is_terminal() {
                let _ = lease.apply(Transition::Release, now, Some("workspace completed".into()));
            }
        }
        self.state = LifecycleState::Completed;
        Ok(())
    }

    /// Mark the workspace failed (plan error). Releases all seats.
    pub fn fail(&mut self, reason: &str) -> Result<()> {
        if self.state.is_terminal() {
            return Err(crate::error::Error::WorkspaceTerminal {
                id: self.id.clone(),
                state: self.state,
            });
        }
        let now = Utc::now();
        for lease in self.leases.values_mut() {
            if !lease.state.is_terminal() {
                let _ = lease.apply(
                    Transition::MarkFailed,
                    now,
                    Some(format!("workspace failed: {reason}")),
                );
            }
        }
        self.state = LifecycleState::Failed;
        Ok(())
    }

    /// Cancel the workspace before any seats are bound.
    pub fn cancel(&mut self) -> Result<()> {
        if self.state == LifecycleState::Active {
            return Err(crate::error::Error::WorkspaceTerminal {
                id: self.id.clone(),
                state: self.state,
            });
        }
        self.state = LifecycleState::Cancelled;
        Ok(())
    }

    /// Expire any expired leases and check workspace-level expiry.
    pub fn reap_expired(&mut self, now: DateTime<Utc>) {
        for lease in self.leases.values_mut() {
            if !lease.state.is_terminal() && lease.is_expired(now) {
                let _ = lease.apply(
                    Transition::MarkFailed,
                    now,
                    Some("lease TTL expired".into()),
                );
            }
        }
        if self.is_expired(now) && !self.state.is_terminal() {
            let _ = self.fail("workspace TTL expired");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_plan() -> RoutePlan {
        RoutePlan {
            id: PlanId("plan-test".into()),
            steps: vec![],
            locality_tier: "L1".into(),
            score_breakdown: None,
        }
    }

    fn make_workspace() -> Workspace {
        Workspace::new(
            WorkspaceId("ws-1".into()),
            Some("test".into()),
            Duration::minutes(10),
            TrustScope::Ephemeral,
        )
    }

    #[test]
    fn new_workspace_unassigned() {
        let ws = make_workspace();
        assert_eq!(ws.state, LifecycleState::Unassigned);
        assert!(!ws.is_active());
        assert!(!ws.is_expired(Utc::now()));
    }

    #[test]
    fn assign_plan_moves_to_assigned() {
        let mut ws = make_workspace();
        let plan = make_plan();
        let topo = Topology::new();
        ws.assign_plan(plan.clone(), topo.clone());
        assert_eq!(ws.state, LifecycleState::Assigned);
        assert_eq!(ws.plan.as_ref(), Some(&plan));
        assert_eq!(ws.topology.as_ref(), Some(&topo));
    }

    #[test]
    fn seat_claim_inactive_without_plan() {
        let mut ws = make_workspace();
        let err = ws.claim_seat(&PlanId("plan-1".into()), "host-1".into(), "L0".into(), Duration::minutes(5))
            .expect_err("must fail without plan");
        match err {
            crate::error::Error::WorkspaceTerminal { .. } => {}
            other => panic!("expected WorkspaceTerminal, got {other:?}"),
        }
    }

    #[test]
    fn seat_claim_then_complete() {
        let mut ws = make_workspace();
        ws.assign_plan(make_plan(), Topology::new());

        let lease = ws
            .claim_seat(&PlanId("plan-1".into()), "host-1".into(), "L0".into(), Duration::minutes(5))
            .expect("claim must succeed");
        assert_eq!(lease.state, LeaseState::Active);
        assert_eq!(ws.state, LifecycleState::Active);
        assert!(ws.seat_is_held("host-1", "L0"));

        ws.complete().expect("complete must succeed");
        assert_eq!(ws.state, LifecycleState::Completed);
        assert!(!ws.seat_is_held("host-1", "L0"));
    }

    #[test]
    fn double_claim_fails() {
        let mut ws = make_workspace();
        ws.assign_plan(make_plan(), Topology::new());
        ws.claim_seat(&PlanId("plan-1".into()), "host-1".into(), "L0".into(), Duration::minutes(5))
            .expect("first claim");
        let err = ws
            .claim_seat(&PlanId("plan-2".into()), "host-1".into(), "L0".into(), Duration::minutes(5))
            .expect_err("double claim must fail");
        match err {
            crate::error::Error::SeatConflict { host, locality_tier, .. } => {
                assert_eq!(host, "host-1");
                assert_eq!(locality_tier, "L0");
            }
            other => panic!("expected SeatConflict, got {other:?}"),
        }
    }

    #[test]
    fn cancel_before_active_ok() {
        let mut ws = make_workspace();
        ws.cancel().expect("cancel before active must succeed");
        assert_eq!(ws.state, LifecycleState::Cancelled);
    }

    #[test]
    fn cancel_after_active_fails() {
        let mut ws = make_workspace();
        ws.assign_plan(make_plan(), Topology::new());
        ws.claim_seat(&PlanId("plan-1".into()), "host-1".into(), "L0".into(), Duration::minutes(5))
            .expect("claim");
        let err = ws.cancel().expect_err("cancel after active must fail");
        match err {
            crate::error::Error::WorkspaceTerminal { state, .. } => {
                assert_eq!(state, LifecycleState::Active);
            }
            other => panic!("expected WorkspaceTerminal, got {other:?}"),
        }
    }

    #[test]
    fn fail_releases_all() {
        let mut ws = make_workspace();
        ws.assign_plan(make_plan(), Topology::new());
        ws.claim_seat(&PlanId("plan-1".into()), "host-1".into(), "L0".into(), Duration::minutes(5))
            .expect("claim");
        ws.fail("boom").expect("fail must succeed");
        assert_eq!(ws.state, LifecycleState::Failed);
        assert!(!ws.seat_is_held("host-1", "L0"));
    }

    #[test]
    fn lease_reap_on_expiry() {
        let mut ws = make_workspace();
        ws.assign_plan(make_plan(), Topology::new());
        ws.claim_seat(&PlanId("plan-1".into()), "host-1".into(), "L0".into(), Duration::minutes(1))
            .expect("claim");
        // Advance time past the lease TTL (1 minute)
        let later = ws.created_at + chrono::Duration::minutes(2);
        ws.reap_expired(later);
        assert_eq!(ws.state, LifecycleState::Failed);
    }
}
