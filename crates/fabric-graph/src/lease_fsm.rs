//! Lease finite-state machine (FSM) guard for PF-WP-015 (spec 019).
//!
//! Implements the FSM transition table documented in spec 019 §3
//! and verified by the contract tests in `surface_fsm_test` (below).
//!
//! ```text
//!   Pending ──bind──> Active ──complete──> Completed
//!                       │
//!                       ├──fail──> Failed
//!                       │
//!                       ├──revoke──> Revoked
//!                       │
//!                       └──expire──> Expired
//! ```
//!
//! All transitions return the new `LeaseState` or a typed `LeaseTransitionError`
//! if the requested transition is illegal. Callers (`surface_ops.rs`) use this
//! guard as the single source of truth for "can transition X → Y".

/// A lease FSM transition error (typed so callers can pattern-match).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeaseTransitionError {
    /// The transition was illegal for the current state.
    IllegalTransition { from: LeaseState, attempted: &'static str },
    /// A guard predicate (e.g. expired-by-clock) rejected the transition.
    GuardRejected { reason: &'static str },
}

/// Verify a state transition is legal.
///
/// This is a pure check — no mutation. Use `next_state` for the side-effecting
/// version.
pub fn can_transition(from: LeaseState, to: LeaseState) -> bool {
    use LeaseState::*;
    matches!(
        (from, to),
        (Pending, Active)
            | (Pending, Failed)
            | (Pending, Revoked)
            | (Pending, Expired)
            | (Active, Completed)
            | (Active, Failed)
            | (Active, Revoked)
            | (Active, Expired)
    )
}

/// Compute the new state after a successful transition; returns
/// `LeaseTransitionError::IllegalTransition` if the requested transition
/// isn't allowed.
pub fn next_state(
    from: LeaseState,
    to: LeaseState,
) -> Result<LeaseState, LeaseTransitionError> {
    if can_transition(from, to) {
        Ok(to)
    } else {
        Err(LeaseTransitionError::IllegalTransition {
            from,
            attempted: transition_name(to),
        })
    }
}

/// Convenience: name of the transition for error messages.
fn transition_name(state: LeaseState) -> &'static str {
    use LeaseState::*;
    match state {
        Pending => "pending",
        Active => "active",
        Completed => "completed",
        Failed => "failed",
        Revoked => "revoked",
        Expired => "expired",
    }
}

// Re-export the LeaseState enum so callers don't need a second use statement.
pub use crate::surface::LeaseState;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_can_go_active_failed_revoked_expired() {
        for to in [
            LeaseState::Active,
            LeaseState::Failed,
            LeaseState::Revoked,
            LeaseState::Expired,
        ] {
            assert!(
                can_transition(LeaseState::Pending, to),
                "Pending -> {to:?} should be legal"
            );
        }
    }

    #[test]
    fn active_can_go_completed_failed_revoked_expired() {
        for to in [
            LeaseState::Completed,
            LeaseState::Failed,
            LeaseState::Revoked,
            LeaseState::Expired,
        ] {
            assert!(
                can_transition(LeaseState::Active, to),
                "Active -> {to:?} should be legal"
            );
        }
    }

    #[test]
    fn terminal_states_cannot_transition_anywhere() {
        for terminal in [
            LeaseState::Completed,
            LeaseState::Failed,
            LeaseState::Revoked,
            LeaseState::Expired,
        ] {
            for to in [
                LeaseState::Pending,
                LeaseState::Active,
                LeaseState::Completed,
                LeaseState::Failed,
                LeaseState::Revoked,
                LeaseState::Expired,
            ] {
                assert!(
                    !can_transition(terminal, to),
                    "{terminal:?} -> {to:?} should be illegal"
                );
            }
        }
    }

    #[test]
    fn pending_cannot_self_transition_to_terminal_except_expired_failed_revoked() {
        // Spec 019 §3: Pending may also expire/fail/revoke without ever becoming
        // active (e.g. spec rejected by validator, operator aborts before bind,
        // upstream resource vanished).
        assert!(can_transition(LeaseState::Pending, LeaseState::Expired));
        assert!(can_transition(LeaseState::Pending, LeaseState::Failed));
        assert!(can_transition(LeaseState::Pending, LeaseState::Revoked));
    }

    #[test]
    fn next_state_returns_error_for_illegal_transition() {
        let err = next_state(LeaseState::Pending, LeaseState::Completed).unwrap_err();
        assert_eq!(
            err,
            LeaseTransitionError::IllegalTransition {
                from: LeaseState::Pending,
                attempted: "completed",
            }
        );
    }
}
