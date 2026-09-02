// Copyright 2026 Phenotype authors
//! Seat lease and workspace lifecycle types.

use std::time::Duration;

use fabric_capability::locality::LocalityTier;

/// Unique identifier for a seat within a workspace.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SeatId(pub String);

impl SeatId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Trust scope of a seat lease — ephemeral (local process only) or persistent
/// (survives process restarts).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustScope {
    /// Ephemeral lease; local process only.
    Ephemeral,
    /// Persistent lease; survives process restarts.
    Persistent,
}

/// Lifecycle state of a seat lease.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleState {
    Pending,
    Active,
    Released,
    Revoked,
    Expired,
}

/// Lifecycle transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    Activate,
    Release,
    Revoke,
    Expire,
}

/// Seat lease — binds a named seat to a workspace with an expiry time.
#[derive(Debug, Clone)]
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
    /// Lease expiry instant (UTC epoch millis).
    pub expires_at_ms: i64,
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
            && chrono::Utc::now().timestamp_millis() < self.expires_at_ms
    }

    /// Whether this lease has expired based on wall-clock time.
    pub fn is_expired(&self) -> bool {
        chrono::Utc::now().timestamp_millis() >= self.expires_at_ms
    }

    /// Whether the given transition is valid from the current state.
    pub fn can_transition(&self, t: Transition) -> bool {
        match (&self.state, t) {
            (LifecycleState::Pending, Transition::Activate) => true,
            (LifecycleState::Active, Transition::Release) => true,
            (LifecycleState::Active, Transition::Revoke) => true,
            (LifecycleState::Active, Transition::Expire) => true,
            _ => false,
        }
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

    fn make_lease(state: LifecycleState, expires_ms: i64) -> SeatLease {
        SeatLease {
            id: SeatId::new("ws:gpu0"),
            name: "gpu0".into(),
            workspace_id: "ws".into(),
            locality_tier: LocalityTier::L2SameAsic,
            trust_scope: TrustScope::Ephemeral,
            ttl: Duration::from_secs(3600),
            expires_at_ms: expires_ms,
            state,
            required_capabilities: vec!["GPU:1".into()],
        }
    }

    #[test]
    fn test_active_when_pending() {
        let lease = make_lease(LifecycleState::Pending, i64::MAX);
        assert!(!lease.is_active());
    }

    #[test]
    fn test_active_when_released() {
        let lease = make_lease(LifecycleState::Released, i64::MAX);
        assert!(!lease.is_active());
    }

    #[test]
    fn test_is_expired() {
        let lease = make_lease(LifecycleState::Active, 0);
        assert!(lease.is_expired());
    }

    #[test]
    fn test_transition_pending_to_active() {
        let mut lease = make_lease(LifecycleState::Pending, i64::MAX);
        assert_eq!(lease.transition(Transition::Activate), Some(LifecycleState::Active));
        assert_eq!(lease.state, LifecycleState::Active);
    }

    #[test]
    fn test_transition_active_to_released() {
        let mut lease = make_lease(LifecycleState::Active, i64::MAX);
        assert_eq!(lease.transition(Transition::Release), Some(LifecycleState::Released));
    }

    #[test]
    fn test_invalid_transition_pending_to_released() {
        let mut lease = make_lease(LifecycleState::Pending, i64::MAX);
        assert_eq!(lease.transition(Transition::Release), None);
        assert_eq!(lease.state, LifecycleState::Pending);
    }

    #[test]
    fn test_expire_sets_state() {
        let mut lease = make_lease(LifecycleState::Active, i64::MAX);
        assert_eq!(lease.transition(Transition::Expire), Some(LifecycleState::Expired));
        assert_eq!(lease.state, LifecycleState::Expired);
    }

    #[test]
    fn test_id_derivation() {
        let id = SeatLease::derive_id("ws", "gpu0");
        assert_eq!(id.0, "ws:gpu0");
    }
}
