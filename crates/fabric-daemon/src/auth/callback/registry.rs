//! In-memory state registry for issued OAuth login states.
//!
//! One-shot semantics: a state can be redeemed exactly once, which is
//! what RFC 6749 section 10.12 calls for and what makes the CSRF check
//! meaningful. The TTL caps the window during which a leaked state can
//! be replayed.
//!
//! PKCE binding: each state also carries the verifier the daemon minted
//! for that login (`record_state_with_verifier` / `take_state_with_verifier`).
//! The verifier is never sent to the browser; it stays server-side until
//! the matching `/auth/callback` request redeems it. The verifier-to-state
//! binding is what makes PKCE safe: an attacker who steals an auth code
//! cannot redeem it without also presenting the verifier that matches
//! the `code_challenge` the daemon put on the authorize URL.

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
static STATE_REGISTRY: OnceLock<Mutex<HashMap<String, IssuedState>>> = OnceLock::new();

#[derive(Clone)]
struct IssuedState {
    issued_at: Instant,
    /// PKCE verifier minted at issue time. `None` for legacy callers that
    /// don't bind a verifier (e.g. when running tests against a flow that
    /// doesn't exercise PKCE).
    verifier: Option<String>,
}

fn registry() -> &'static Mutex<HashMap<String, IssuedState>> {
    STATE_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock_registry() -> std::sync::MutexGuard<'static, HashMap<String, IssuedState>> {
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
/// Does NOT bind a PKCE verifier. Prefer `record_state_with_verifier`
/// in production; this exists so non-PKCE callers (tests, debugging
/// harnesses) can still register a state.
#[allow(dead_code)] // Legacy wrapper kept for non-PKCE callers; tests still exercise it.
pub fn record_state(state: &str) {
    record_state_with_verifier(state, None);
}

/// Record a state with a PKCE verifier bound to it.
///
/// `verifier` is the value the daemon minted via `generate_pkce_pair`
/// at the same time as the matching authorize URL. It is returned by
/// `take_state_with_verifier` on a successful one-shot redemption and
/// must then be sent on the token exchange as `code_verifier`.
///
/// Public so the single call site in `wire/auth_handlers.rs` can reach it.
pub fn record_state_with_verifier(state: &str, verifier: Option<String>) {
    if state.is_empty() {
        return;
    }
    lock_registry().insert(
        state.to_string(),
        IssuedState {
            issued_at: Instant::now(),
            verifier,
        },
    );
}

/// Validate a state presented by the callback and remove it on success.
///
/// Returns `true` iff the state was issued by this daemon, is still
/// within the TTL window, and has not already been redeemed. One-shot
/// semantics: a state can be exchanged for tokens exactly once.
///
/// `pub` because `super` (callback.rs) re-exports it for tests; sibling
/// modules within `auth::callback` access it via `super::registry::take_state`.
#[allow(dead_code)] // Legacy wrapper kept for non-PKCE callers; tests still exercise it.
pub fn take_state(state: &str) -> bool {
    take_state_with_verifier(state).is_some()
}

/// What a state was bound to when it was issued.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssuedVerifier {
    /// The state was issued with a PKCE verifier. The verifier must be
    /// sent on the token exchange as `code_verifier`.
    With(String),
    /// The state was recorded with `record_state` (legacy path) or with
    /// `record_state_with_verifier(state, None)`. There is no daemon-
    /// minted verifier; the caller is responsible for supplying one if
    /// the WorkOS side demands it.
    None,
}

impl IssuedVerifier {
    /// Borrow the inner verifier, if any.
    #[allow(dead_code)] // Convenience accessor; `match` is used at call sites today.
    pub fn as_deref(&self) -> Option<&str> {
        match self {
            IssuedVerifier::With(v) => Some(v.as_str()),
            IssuedVerifier::None => None,
        }
    }
}

/// Validate a state presented by the callback and remove it on success,
/// returning what was bound to the state at issue time.
///
/// `None` means the state is unknown, expired, or already redeemed.
///
/// On `Some(IssuedVerifier::With(v))`:
/// - `v` is the PKCE verifier the daemon minted. The token exchange
///   must use it.
///
/// On `Some(IssuedVerifier::None)`:
/// - The state was recorded without a verifier. The caller may supply
///   one (e.g. an integration test that drives the daemon directly);
///   the daemon will use it if it is `Some` and reject the request if
///   it is `None` and WorkOS is configured to require PKCE.
///
/// Callers that pass a caller-supplied verifier (e.g. the HTTP callback
/// pulled one out of the query string) should compare against the
/// daemon-minted value before proceeding — that is the only way to
/// detect a tampering attempt.
pub fn take_state_with_verifier(state: &str) -> Option<IssuedVerifier> {
    if state.is_empty() {
        return None;
    }
    let entry = lock_registry().remove(state)?;
    if Instant::now().saturating_duration_since(entry.issued_at) > STATE_TTL {
        return None;
    }
    Some(match entry.verifier {
        Some(v) => IssuedVerifier::With(v),
        None => IssuedVerifier::None,
    })
}

/// Clear the state registry. Test-only helper; never call from production.
#[cfg(test)]
pub(crate) fn clear_registry_for_tests() {
    lock_registry().clear();
}
