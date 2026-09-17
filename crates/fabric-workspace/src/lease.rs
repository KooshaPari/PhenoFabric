// Copyright 2026 Phenotype authors
//! Seat lease and workspace lifecycle types.

use std::time::Duration;

use fabric_capability::locality::LocalityTier;

/// Unique identifier for a seat within a workspace.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SeatId(pub String);

impl SeatId {
    /// Create a new seat ID from a string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Trust scope of a seat lease — ephemeral (local process only) or persistent
/// (survives process restarts).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TrustScope {
    /// Ephemeral lease; local process only.
    Ephemeral,
    /// Persistent lease; survives process restarts.
    Persistent,
}

/// Lifecycle state of a seat lease.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LifecycleState {
    /// The lease is pending activation.
    Pending,
    /// The lease is active and the seat is in use.
    Active,
    /// The seat has been voluntarily released.
    Released,
    /// The seat was revoked by an administrator.
    Revoked,
    /// The lease has expired.
    Expired,
}

/// Lifecycle transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Transition {
    /// Transition the lease to active.
    Activate,
    /// Transition the lease to released.
    Release,
    /// Transition the lease to revoked.
    Revoke,
    /// Transition the lease to expired.
    Expire,
}

/// Seat lease — binds a named seat to a workspace with an expiry time.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SeatLease {
    /// Unique identifier.
    pub id: SeatId,
    /// Display name of the seat.
    pub name: String,
    /// Parent workspace identifier.
    pub workspace_id: String,
    /// Locality tier at which this seat is allocated.
    pub locality_tier: LocalityTier,
    /// Trust scope.
    pub trust_scope: TrustScope,
    /// Lease TTL.
    pub ttl: Duration,
    /// Lease expiry instant in UTC epoch millis. `None` means no expiry.
    pub expires_at_ms: Option<i64>,
    /// Creation timestamp in UTC epoch millis.
    pub created_at_ms: i64,
    /// Current lifecycle state.
    pub state: LifecycleState,
    /// Optional capability requirements (e.g. GPU:1, audio:1).
    pub required_capabilities: Vec<String>,
}

impl SeatLease {
    /// Derive a unique seat ID from workspace name and seat name.
    pub fn derive_id(workspace: &str, seat: &str) -> SeatId {
        SeatId(format!("{}:{}", workspace, seat))
    }

    /// Whether this lease is currently active and not expired.
    pub fn is_active(&self) -> bool {
        self.state == LifecycleState::Active
            && self
                .expires_at_ms
                .map_or(true, |exp| chrono::Utc::now().timestamp_millis() < exp)
    }

    /// Whether this lease has expired based on wall-clock time.
    /// Leases without an expiry are never considered expired.
    pub fn is_expired(&self) -> bool {
        self.expires_at_ms
            .is_some_and(|exp| chrono::Utc::now().timestamp_millis() >= exp)
    }

    /// Renew the lease by extending `expires_at_ms` from now by `duration`.
    /// If the lease had no expiry, sets one from now.
    pub fn renew(&mut self, duration: Duration) {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let extend_ms = duration.as_millis() as i64;
        self.expires_at_ms = Some(match self.expires_at_ms {
            Some(current) => current.max(now_ms) + extend_ms,
            None => now_ms + extend_ms,
        });
        self.ttl = duration;
    }

    /// Whether the given transition is valid from the current state.
    pub fn can_transition(&self, t: Transition) -> bool {
        matches!((&self.state, t),
            (LifecycleState::Pending, Transition::Activate)
            | (LifecycleState::Active, Transition::Release)
            | (LifecycleState::Active, Transition::Revoke)
            | (LifecycleState::Active, Transition::Expire)
        )
    }

    /// Apply the given transition and return the next state, or None if invalid.
    #[must_use]
    pub fn transition(&mut self, t: Transition) -> Option<LifecycleState> {
        if !self.can_transition(t) {
            return None;
        }
        let next = match t {
            Transition::Activate => LifecycleState::Active,
            Transition::Release => LifecycleState::Released,
            Transition::Revoke => LifecycleState::Revoked,
            Transition::Expire => {
                self.state = LifecycleState::Expired;
                return Some(LifecycleState::Expired);
            }
        };
        self.state = next.clone();
        Some(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_lease(state: LifecycleState, expires_ms: Option<i64>) -> SeatLease {
        SeatLease {
            id: SeatId::new("ws:gpu0"),
            name: "gpu0".into(),
            workspace_id: "ws".into(),
            locality_tier: LocalityTier::L2CrossNumaShm,
            trust_scope: TrustScope::Ephemeral,
            ttl: Duration::from_secs(3600),
            expires_at_ms: expires_ms,
            created_at_ms: 1_000_000,
            state,
            required_capabilities: vec!["GPU:1".into()],
        }
    }

    #[test]
    fn test_active_when_pending() {
        let lease = make_lease(LifecycleState::Pending, Some(i64::MAX));
        assert!(!lease.is_active());
    }

    #[test]
    fn test_active_when_released() {
        let lease = make_lease(LifecycleState::Released, Some(i64::MAX));
        assert!(!lease.is_active());
    }

    #[test]
    fn test_is_expired() {
        let lease = make_lease(LifecycleState::Active, Some(0));
        assert!(lease.is_expired());
    }

    #[test]
    fn test_no_expiry_never_expired() {
        let lease = make_lease(LifecycleState::Active, None);
        assert!(!lease.is_expired());
        assert!(lease.is_active());
    }

    #[test]
    fn test_transition_pending_to_active() {
        let mut lease = make_lease(LifecycleState::Pending, Some(i64::MAX));
        assert_eq!(lease.transition(Transition::Activate), Some(LifecycleState::Active));
        assert_eq!(lease.state, LifecycleState::Active);
    }

    #[test]
    fn test_transition_active_to_released() {
        let mut lease = make_lease(LifecycleState::Active, Some(i64::MAX));
        assert_eq!(lease.transition(Transition::Release), Some(LifecycleState::Released));
    }

    #[test]
    fn test_invalid_transition_pending_to_released() {
        let mut lease = make_lease(LifecycleState::Pending, Some(i64::MAX));
        assert_eq!(lease.transition(Transition::Release), None);
        assert_eq!(lease.state, LifecycleState::Pending);
    }

    #[test]
    fn test_expire_sets_state() {
        let mut lease = make_lease(LifecycleState::Active, Some(i64::MAX));
        assert_eq!(lease.transition(Transition::Expire), Some(LifecycleState::Expired));
        assert_eq!(lease.state, LifecycleState::Expired);
    }

    #[test]
    fn test_renew_extends_expiry() {
        let mut lease = make_lease(LifecycleState::Active, Some(1000));
        let before = chrono::Utc::now().timestamp_millis();
        lease.renew(Duration::from_secs(3600));
        let after = chrono::Utc::now().timestamp_millis();
        let expected_min = before + 3_600_000;
        let expected_max = after + 3_600_000;
        let exp = lease.expires_at_ms.unwrap();
        assert!(exp >= expected_min, "exp={exp} < min={expected_min}");
        assert!(exp <= expected_max, "exp={exp} > max={expected_max}");
    }

    #[test]
    fn test_renew_sets_expiry_when_none() {
        let mut lease = make_lease(LifecycleState::Active, None);
        assert!(lease.expires_at_ms.is_none());
        lease.renew(Duration::from_secs(60));
        assert!(lease.expires_at_ms.is_some());
        assert!(lease.is_active());
    }

    #[test]
    fn test_id_derivation() {
        let id = SeatLease::derive_id("ws", "gpu0");
        assert_eq!(id.0, "ws:gpu0");
    }
}
