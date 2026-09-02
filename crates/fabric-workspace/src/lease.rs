//! Seat-lease state machine.
//!
//! A `SeatLease` represents the binding between a [`RoutePlan`](fabric_graph::RoutePlan) and the
//! physical seat(s) it claims on a host. Leases are tracked per-(host, locality-tier) pair and
//! guarantee that at most one active workspace holds a given seat at any time.
//!
//! See `adr/0025-route-lease-semantic-model.md` for the semantic model.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// State of a seat lease.
///
/// The state machine is:
///
/// ```text
///                  release()
///   Pending ───────────────────▶ Released
///     │
///     │  activate()  (seat bound; plan is now executing)
///     ▼
///  Active  ────────────────▶ Revoked (operator action or trust failure)
///            state: Transition::Pending,│
///     │  release()  (plan completed normally)
///     ▼
///  Released
///
///  Any state can transition to Failed (via mark_failed).
///  Active, Revoked, Failed, and Released are terminal.
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LeaseState {
    /// Plan compiled; lease reserved but execution not yet bound.
    Pending,
    /// Plan is executing; seat is held.
    Active,
    /// Plan completed normally (or was never activated); seat returned.
    Released,
    /// Plan completed with an error; seat returned.
    Failed,
    /// Operator or trust failure revoked the seat before plan completion.
    Revoked,
}

impl LeaseState {
    /// Is this state terminal? (no further transitions accepted)
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            LeaseState::Released | LeaseState::Failed | LeaseState::Revoked
        )
    }

    /// Human-readable name (for log lines and CLI output).
    pub fn as_str(self) -> &'static str {
        match self {
            LeaseState::Pending => "pending",
            LeaseState::Active => "active",
            LeaseState::Released => "released",
            LeaseState::Failed => "failed",
            LeaseState::Revoked => "revoked",
        }
    }
}

impl fmt::Display for LeaseState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A transition request applied to a lease.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Transition {
    /// Bind the seat; move Pending → Active.
    Activate,
    /// Complete normally; move Active → Released.
    Release,
    /// Complete with error; move any non-terminal → Failed.
    MarkFailed,
    /// Operator revoke; move Active → Revoked.
    Revoke,
}

impl Transition {
    /// Verify the transition is valid from `from`.
    pub fn is_valid(self, from: LeaseState) -> bool {
        use LeaseState::*;
        match (from, self) {
            (Pending, Activate) => true,
            (Active, Release) => true,
            (Active, Revoke) => true,
            (Pending, MarkFailed) => true,
            (Active, MarkFailed) => true,
            (Released, _) | (Failed, _) | (Revoked, _) => false,
            (Pending, Release) | (Pending, Revoke) => false,
        }
    }
}

/// Identifier of a seat lease (UUID v7 string).
pub type LeaseId = String;

/// A binding of a RoutePlan to the physical seats it occupies.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeatLease {
    /// Unique lease identifier.
    pub id: LeaseId,
    /// Workspace that owns this lease.
    pub workspace_id: crate::workspace::WorkspaceId,
    /// `RoutePlan.id` (UUID v7 string from the route compiler).
    pub plan_id: String,
    /// Host identifier (`CapabilityDescriptor.node_id`).
    pub host: String,
    /// Locality tier of the claimed seat ("L0", "L1", etc.).
    pub locality_tier: String,
    /// Current state of the lease.
    pub state: LeaseState,
    /// When the lease was created.
    pub created_at: DateTime<Utc>,
    /// When the lease was activated (None if still pending).
    pub activated_at: Option<DateTime<Utc>>,
    /// Hard expiry (TTL). After this point, the seat is reclaimed automatically.
    pub expires_at: DateTime<Utc>,
    /// Time the lease entered a terminal state (None if still in flight).
    pub terminated_at: Option<DateTime<Utc>>,
    /// Trust scope required for the seat. See [`crate::workspace::TrustScope`].
    pub trust_scope: crate::workspace::TrustScope,
    /// Optional human-readable reason for the most recent state change.
    pub last_reason: Option<String>,
}

impl SeatLease {
    /// Create a new pending lease. `ttl` is the time-to-live from now.
    pub fn new(
        workspace_id: crate::workspace::WorkspaceId,
        plan_id: impl Into<String>,
        host: impl Into<String>,
        locality_tier: impl Into<String>,
        ttl: Duration,
        trust_scope: crate::workspace::TrustScope,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::now_v7().to_string(),
            workspace_id,
            plan_id: plan_id.into(),
            host: host.into(),
            locality_tier: locality_tier.into(),
            state: LeaseState::Pending,
            created_at: now,
            activated_at: None,
            expires_at: now + ttl,
            terminated_at: None,
            trust_scope,
            last_reason: None,
        }
    }

    /// Has this lease expired relative to `now`?
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires_at
    }

    /// Apply a state transition. Returns the previous state on success.
    pub fn apply(
        &mut self,
        transition: Transition,
        now: DateTime<Utc>,
        reason: Option<String>,
    ) -> crate::error::Result<LeaseState> {
        if !transition.is_valid(self.state) {
            return Err(crate::error::Error::InvalidTransition {
                lease_id: self.id.clone(),
                transition,
            });
        }
        let prev = self.state;
        self.state = match transition {
            Transition::Activate => {
                self.activated_at = Some(now);
                LeaseState::Active
            }
            Transition::Release => {
                self.terminated_at = Some(now);
                LeaseState::Released
            }
            Transition::MarkFailed => {
                self.terminated_at = Some(now);
                LeaseState::Failed
            }
            Transition::Revoke => {
                self.terminated_at = Some(now);
                LeaseState::Revoked
            }
        };
        self.last_reason = reason;
        Ok(prev)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::{TrustScope, WorkspaceId};

    fn ws() -> WorkspaceId {
        WorkspaceId("ws-test".into())
    }

    #[test]
    fn state_terminal_predicate() {
        assert!(!LeaseState::Pending.is_terminal());
        assert!(!LeaseState::Active.is_terminal());
        for s in [
            LeaseState::Released,
            LeaseState::Failed,
            LeaseState::Revoked,
        ] {
            assert!(s.is_terminal());
        }
    }

    #[test]
    fn transition_validity_table() {
        // pending → activate ok
        assert!(Transition::Activate.is_valid(LeaseState::Pending));
        assert!(!Transition::Release.is_valid(LeaseState::Pending));
        assert!(!Transition::Revoke.is_valid(LeaseState::Pending));
        assert!(Transition::MarkFailed.is_valid(LeaseState::Pending));

        // active → release/revoke/mark_failed ok
        assert!(Transition::Release.is_valid(LeaseState::Active));
        assert!(Transition::Revoke.is_valid(LeaseState::Active));
        assert!(Transition::MarkFailed.is_valid(LeaseState::Active));
        assert!(!Transition::Activate.is_valid(LeaseState::Active));

        // terminal: nothing valid
        for t in [
            Transition::Activate,
            Transition::Release,
            Transition::MarkFailed,
            Transition::Revoke,
        ] {
            assert!(!t.is_valid(LeaseState::Released));
            assert!(!t.is_valid(LeaseState::Failed));
            assert!(!t.is_valid(LeaseState::Revoked));
        }
    }

    #[test]
    fn lease_full_lifecycle() {
        let mut lease = SeatLease::new(
            ws(),
            "plan-1",
            "host-1",
            "L0",
            Duration::seconds(60),
            TrustScope::Ephemeral,
        );
        assert_eq!(lease.state, LeaseState::Pending);
        let now = lease.created_at + Duration::seconds(1);

        let prev = lease.apply(Transition::Activate, now, Some("go".into())).unwrap();
        assert_eq!(prev, LeaseState::Pending);
        assert_eq!(lease.state, LeaseState::Active);
        assert_eq!(lease.activated_at, Some(now));
        assert!(!lease.is_expired(now));

        let later = now + Duration::seconds(30);
        let prev = lease.apply(Transition::Release, later, None).unwrap();
        assert_eq!(prev, LeaseState::Active);
        assert_eq!(lease.state, LeaseState::Released);
        assert_eq!(lease.terminated_at, Some(later));
        assert!(lease.is_terminal());
    }

    #[test]
    fn lease_rejects_invalid_transition() {
        let mut lease = SeatLease::new(
            ws(),
            "plan-1",
            "host-1",
            "L0",
            Duration::seconds(60),
            TrustScope::Ephemeral,
        );
        let now = lease.created_at;
        let err = lease
            .apply(Transition::Release, now, None)
            .expect_err("Release on Pending must fail");
        match err {
            crate::error::Error::InvalidTransition {
                lease_id,
                transition,
            } => {
                assert_eq!(lease_id, lease.id);
                assert_eq!(transition, Transition::Release);
            }
            other => panic!("expected InvalidTransition, got {other:?}"),
        }
    }

    #[test]
    fn lease_expiry_predicate() {
        let lease = SeatLease::new(
            ws(),
            "plan-1",
            "host-1",
            "L0",
            Duration::seconds(10),
            TrustScope::Ephemeral,
        );
        let now = lease.created_at + Duration::seconds(5);
        assert!(!lease.is_expired(now));
        let after = lease.created_at + Duration::seconds(11);
        assert!(lease.is_expired(after));
    }
}
