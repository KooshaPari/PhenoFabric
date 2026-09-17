//! Property-based tests for the Fabric lease FSM.
//!
//! Uses `proptest` to verify the FSM transition table invariants that
//! must hold for any sequence of valid transitions.

use fabric_graph::lease_fsm::{can_transition, next_state, LeaseTransitionError};
use fabric_graph::surface::LeaseState;
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Strategy: generate any LeaseState
// ---------------------------------------------------------------------------

fn arb_lease_state() -> impl Strategy<Value = LeaseState> {
    prop_oneof![
        Just(LeaseState::Pending),
        Just(LeaseState::Active),
        Just(LeaseState::Completed),
        Just(LeaseState::Failed),
        Just(LeaseState::Revoked),
        Just(LeaseState::Expired),
    ]
}

fn arb_terminal_state() -> impl Strategy<Value = LeaseState> {
    prop_oneof![
        Just(LeaseState::Completed),
        Just(LeaseState::Failed),
        Just(LeaseState::Revoked),
        Just(LeaseState::Expired),
    ]
}

// ---------------------------------------------------------------------------
// Property: any sequence of valid transitions from Pending always reaches
// a terminal state in at most 2 steps.
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn pending_reaches_terminal_in_at_most_two_steps(
        first in arb_lease_state(),
        second in arb_lease_state(),
    ) {
        // Start at Pending, try first transition.
        if !can_transition(LeaseState::Pending, first) {
            // Can't even make the first move — that's fine, Pending is non-terminal.
            // But this property is about valid sequences, so we only test reachable states.
            return Ok(());
        }

        let after_first = next_state(LeaseState::Pending, first);
        prop_assert!(after_first.is_ok());

        let state_after_first = after_first.unwrap();

        // If we reached a terminal state, we're done.
        if is_terminal(state_after_first) {
            return Ok(());
        }

        // If not terminal, we must be able to reach a terminal state from here.
        if can_transition(state_after_first, second) {
            let after_second = next_state(state_after_first, second);
            prop_assert!(after_second.is_ok());
            let state_after_second = after_second.unwrap();
            prop_assert!(
                is_terminal(state_after_second),
                "second step from {:?} should reach terminal, got {:?}",
                state_after_first,
                state_after_second
            );
        }
        // If can_transition returns false, that's also valid — some transitions
        // are blocked by guards (like expiry), but structurally the FSM allows
        // reaching terminal from any non-terminal state.
    }
}

// ---------------------------------------------------------------------------
// Property: no transition from Released/Revoked/Expired is valid
// (note: Completed is also terminal — the FSM spec uses Revoked, Expired,
// Failed, Completed as terminal)
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn terminal_states_have_no_valid_transitions(
        terminal in arb_terminal_state(),
        target in arb_lease_state(),
    ) {
        prop_assert!(
            !can_transition(terminal, target),
            "transition from terminal state {:?} to {:?} should be invalid",
            terminal,
            target
        );
    }
}

// ---------------------------------------------------------------------------
// Property: can_transition is consistent with next_state
// (if can_transition says yes, next_state returns Ok with the target state)
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn can_transition_matches_next_state(
        from in arb_lease_state(),
        to in arb_lease_state(),
    ) {
        let can = can_transition(from, to);
        let next = next_state(from, to);

        if can {
            prop_assert!(
                next.is_ok(),
                "can_transition({:?}, {:?}) is true but next_state returned Err",
                from,
                to
            );
            let result_state = next.unwrap();
            prop_assert_eq!(
                result_state,
                to,
                "next_state should return the target state"
            );
        } else {
            prop_assert!(
                next.is_err(),
                "can_transition({:?}, {:?}) is false but next_state returned Ok",
                from,
                to
            );
            match next.unwrap_err() {
                LeaseTransitionError::IllegalTransition { from: f, attempted: _ } => {
                    prop_assert_eq!(f, from, "error should report the correct source state");
                }
                LeaseTransitionError::GuardRejected { .. } => {
                    // Guard rejection can happen even when can_transition says true
                    // if there's a clock guard. In our implementation, can_transition
                    // doesn't check guards, so this shouldn't happen.
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Property: lease expiry is monotonic (once expired, stays expired)
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn expiry_is_monotonic(target in arb_lease_state()) {
        // Once in Expired state, no transition is valid.
        prop_assert!(
            !can_transition(LeaseState::Expired, target),
            "Expired -> {:?} should be invalid (expiry is terminal)",
            target
        );
    }
}

// ---------------------------------------------------------------------------
// Property: Pending can only transition to specific states
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn pending_only_transitions_to_valid_targets(target in arb_lease_state()) {
        let valid = matches!(
            target,
            LeaseState::Active
                | LeaseState::Failed
                | LeaseState::Revoked
                | LeaseState::Expired
        );
        prop_assert_eq!(
            can_transition(LeaseState::Pending, target),
            valid,
            "Pending -> {:?} should be {}",
            target,
            valid
        );
    }
}

// ---------------------------------------------------------------------------
// Property: Active can only transition to specific states
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn active_only_transitions_to_valid_targets(target in arb_lease_state()) {
        let valid = matches!(
            target,
            LeaseState::Completed
                | LeaseState::Failed
                | LeaseState::Revoked
                | LeaseState::Expired
        );
        prop_assert_eq!(
            can_transition(LeaseState::Active, target),
            valid,
            "Active -> {:?} should be {}",
            target,
            valid
        );
    }
}

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn is_terminal(state: LeaseState) -> bool {
    matches!(
        state,
        LeaseState::Completed | LeaseState::Failed | LeaseState::Revoked | LeaseState::Expired
    )
}
