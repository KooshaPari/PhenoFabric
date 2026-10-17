//! In-memory state registry for issued OAuth login states.
//!
//! One-shot semantics: a state can be redeemed exactly once, which is
//! what RFC 6749 section 10.12 calls for and what makes the CSRF check
//! meaningful. The TTL caps the window during which a leaked state can
//! be replayed.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// How long an issued state remains valid for redemption.
///
/// Picked to cover the longest realistic human pause (read email, switch
/// tabs, click through WorkOS consent) while keeping the CSRF replay
/// window tight.
pub(crate) const STATE_TTL: Duration = Duration::from_secs(600);

/// In-memory registry of states the daemon has issued via the wire
/// `auth_start` path. Inserted from `record_state`, removed-and-validated
/// from `take_state`.
///
/// `OnceLock` because the registry is process-wide (one daemon, one
/// registry) and `Mutex` because the table is mutated from both the wire
/// accept loop and the HTTP accept loop. Throughput is low (one entry per
/// login attempt), so a single `Mutex<HashMap>` is plenty.
static STATE_REGISTRY: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();

fn registry() -> &'static Mutex<HashMap<String, Instant>> {
    STATE_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock_registry() -> std::sync::MutexGuard<'static, HashMap<String, Instant>> {
    // A poison from a prior panicking test must not cascade — the
    // listener thread would die and the client would see a connection
    // reset. Recover the inner guard and keep serving. Tests that need
    // strict isolation can still call `clear_registry_for_tests`.
    let result = registry().lock();
    match result {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Record a state that the daemon has just issued for a fresh login.
///
/// Called from `wire::auth_handlers::handle_auth_start` so the matching
/// `/auth/callback` request can be validated. Overwrites any prior
/// occurrence with the current timestamp so the TTL window restarts.
///
/// Public so the single call site in `wire/auth_handlers.rs` can reach it.
pub fn record_state(state: &str) {
    if state.is_empty() {
        return;
    }
    lock_registry().insert(state.to_string(), Instant::now());
}

/// Validate a state presented by the callback and remove it on success.
///
/// Returns `true` iff the state was issued by this daemon, is still
/// within the TTL window, and has not already been redeemed. One-shot
/// semantics: a state can be exchanged for tokens exactly once.
///
/// `pub` because `super` (callback.rs) re-exports it for tests; sibling
/// modules within `auth::callback` access it via `super::registry::take_state`.
pub fn take_state(state: &str) -> bool {
    if state.is_empty() {
        return false;
    }
    match lock_registry().remove(state) {
        Some(issued) => Instant::now().saturating_duration_since(issued) <= STATE_TTL,
        None => false,
    }
}

/// Clear the state registry. Test-only helper; never call from production.
#[cfg(test)]
pub(crate) fn clear_registry_for_tests() {
    lock_registry().clear();
}
