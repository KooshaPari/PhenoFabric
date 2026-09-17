//! Trust-root model for capability descriptor signatures.
//!
//! Per ADR-0031 and spec 021. R0's [`crate::signing`] API is unchanged;
//! this module is an *additional* verification path that gives the
//! operator chain anchoring, revocation, and time-bounded validity.
//!
//! ## Roles
//!
//! | Role | `parent_key_id` | Held in `by_key_id`? |
//! |:--|:--|:--|
//! | `TrustRoot` | `None` | yes (it's the anchor) |
//! | `IntermediateAuthority` | `Some(root.key_id)` | yes |
//! | `NodeAuthority` | `Some(intermediate.key_id)` or `Some(root.key_id)` | yes |
//!
//! Chain depth cap = [`MAX_CHAIN_DEPTH`] (2 in R1).
//!
//! ## Verification contract
//!
//! See spec 021 §4 for the full algorithm. Summary:
//!
//! 1. For each signature in `descriptor.signatures`:
//!    1. Look up `key_id` in `by_key_id`. Missing → `UnknownAuthority`.
//!    2. If revocation list is set, check `key_id` is not in it. Found → `KeyRevoked`.
//!    3. Walk parent chain; at each step check `not_after > now()`, depth ≤ cap, parent exists. Depth > cap → `ChainTooDeep`.
//!    4. Walk back down, verifying each `Authority.signature` against the parent's `VerificationKey`.
//!    5. Verify the descriptor signature against the leaf.
//!    6. Return `Ok(ChainVerification { node_authority, chain_depth })`.
//! 2. If no signature yields a valid chain, return the FIRST error (more informative than the last).
//!
//! ## Backwards compatibility
//!
//! R0 `sign` / `verify` / `has_trusted_signature` are unchanged. The
//! `TrustStore` is opt-in.

mod authority;
mod store;
mod types;

pub use authority::Authority;
pub use store::{ChainVerification, TrustStore};
pub use types::{
    MAX_CHAIN_DEPTH, RevocationEntry, RevocationList, RevocationReason, TrustError,
};
