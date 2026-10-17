//! Cross-crate integration test for audit finding G-11:
//! "capability revocation is recorded but not enforced on live handles".
//!
//! This test exercises the *public* `CapabilityHandle` API end-to-end
//! across crates: it builds a TrustStore, grants two non-root authorities,
//! acquires a `CapabilityHandle` to each through the public path, then
//! revokes one and proves:
//!   1. the very next `check()` on the same handle returns `KeyRevoked`;
//!   2. the sibling non-revoked authority's handle is unaffected (the
//!      revocation is targeted, not blanket).
//!
//! The test goes through `fabric_capability::CapabilityHandle` and
//! `fabric_capability::Authority` — the same symbols a downstream crate
//! like `fabric-daemon` would import.

use chrono::Utc;
use fabric_capability::{
    Authority, CapabilityHandle, RevocationEntry, RevocationList, RevocationReason, SigningKey,
    TrustError, TrustStore,
};

fn two_authority_chain() -> (SigningKey, Authority, Authority) {
    let root_key = SigningKey::generate();
    let root = Authority::trust_root(&root_key, "root");
    let key_a = SigningKey::generate();
    let key_b = SigningKey::generate();
    let auth_a = Authority::signed_by(&key_a, &root_key, &root, "node-a", None).expect("sign A");
    let auth_b = Authority::signed_by(&key_b, &root_key, &root, "node-b", None).expect("sign B");
    (root_key, auth_a, auth_b)
}

#[test]
fn same_handle_check_after_revoke_returns_key_revoked() {
    let (root_key, auth_a, _auth_b) = two_authority_chain();
    let mut store = TrustStore::new(Authority::trust_root(&root_key, "root"))
        .expect("trusted root constructs a store");
    store.add_authority(auth_a.clone()).expect("add A");
    let handle = CapabilityHandle::new(&store, auth_a.key_id.clone());
    assert!(handle.check().is_ok(), "pre-revoke handle must check clean");

    let list = RevocationList::build_and_sign(
        vec![RevocationEntry {
            key_id: auth_a.key_id.clone(),
            reason: RevocationReason::OperatorRevoked,
            revoked_at: Utc::now(),
        }],
        &root_key,
    )
    .expect("root signs the revocation list");
    store
        .set_revocation_list(list)
        .expect("set_revocation_list on shared &store must succeed");

    let result = handle.check();
    assert!(
        matches!(result, Err(TrustError::KeyRevoked { .. })),
        "post-revoke handle must return KeyRevoked, got {:?}",
        result
    );
}

#[test]
fn sibling_authority_handle_unaffected_by_targeted_revocation() {
    // Negative control: a revocation targeted at auth_a must not affect
    // a live handle to auth_b, proving the per-call check is targeted
    // (not blanket) and the new handle abstraction did not weaken
    // isolation between distinct authorities.
    let (root_key, auth_a, auth_b) = two_authority_chain();
    let mut store = TrustStore::new(Authority::trust_root(&root_key, "root"))
        .expect("trusted root constructs a store");
    store.add_authority(auth_a.clone()).expect("add A");
    store.add_authority(auth_b.clone()).expect("add B");

    let handle_a = CapabilityHandle::new(&store, auth_a.key_id.clone());
    let handle_b = CapabilityHandle::new(&store, auth_b.key_id.clone());
    assert!(handle_a.check().is_ok());
    assert!(handle_b.check().is_ok());

    let list = RevocationList::build_and_sign(
        vec![RevocationEntry {
            key_id: auth_a.key_id.clone(),
            reason: RevocationReason::Compromised,
            revoked_at: Utc::now(),
        }],
        &root_key,
    )
    .expect("root signs the revocation list");
    store
        .set_revocation_list(list)
        .expect("set_revocation_list must succeed through a shared &store");

    assert!(matches!(
        handle_a.check(),
        Err(TrustError::KeyRevoked { .. })
    ));
    assert!(
        handle_b.check().is_ok(),
        "auth_b must remain valid; revocation must be targeted"
    );
}
