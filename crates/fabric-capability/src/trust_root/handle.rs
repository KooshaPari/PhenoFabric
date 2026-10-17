//! `CapabilityHandle` — a live, revocation-aware capability grant.
//!
//! A handle is the explicit representation of an open capability grant.
//! It is created with a [`TrustStore`] reference and the granted
//! authority's `key_id`. The handle exposes a [`CapabilityHandle::check`]
//! method that consults the live revocation list on every call, so a
//! revocation recorded via [`TrustStore::set_revocation_list`] is observed
//! by the very next `check()` — the "next operation on the same handle"
//! contract that closes audit finding G-11.
//!
//! ## Design: consult authoritative state, do not cache
//!
//! The handle holds a shared reference to the [`TrustStore`] (not a clone
//! of the verified Authority). On each `check()` it asks the store for the
//! current state, including the current revocation list. The store is the
//! single source of truth — the handle itself stores nothing beyond the
//! granted `key_id`. This is the simplest correct design: a revocation
//! posted to the same store is observed on the very next call without any
//! handle-side invalidation protocol.

use super::authority::Authority;
use super::store::TrustStore;
use super::types::TrustError;

/// A live capability handle: a reference to a granted authority whose
/// validity is re-checked against the authoritative [`TrustStore`] on
/// every use.
pub struct CapabilityHandle<'a> {
    store: &'a TrustStore,
    key_id: String,
}

impl<'a> CapabilityHandle<'a> {
    /// Build a handle for `key_id` against `store`.
    pub fn new(store: &'a TrustStore, key_id: impl Into<String>) -> Self {
        Self {
            store,
            key_id: key_id.into(),
        }
    }

    /// Returns the granted authority's `key_id`.
    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    /// Returns the [`TrustStore`] this handle consults.
    pub fn store(&self) -> &'a TrustStore {
        self.store
    }

    /// Check if the granted capability is still valid. Consults the
    /// authoritative [`TrustStore`] state on every call.
    pub fn check(&self) -> std::result::Result<&'a Authority, TrustError> {
        self.store.authority(&self.key_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signing::SigningKey;
    use crate::trust_root::types::{RevocationEntry, RevocationList, RevocationReason};
    use chrono::Utc;

    fn two_chain() -> (SigningKey, SigningKey, Authority, Authority) {
        let root_key = SigningKey::generate();
        let node_key = SigningKey::generate();
        let root = Authority::trust_root(&root_key, "root");
        let node = Authority::signed_by(&node_key, &root_key, &root, "node", None).unwrap();
        (root_key, node_key, root, node)
    }

    #[test]
    fn handle_check_succeeds_when_authority_is_known_and_unrevoked() {
        let (_root_key, _node_key, root, node) = two_chain();
        let mut store = TrustStore::new(root).unwrap();
        store.add_authority(node.clone()).unwrap();
        let handle = CapabilityHandle::new(&store, node.key_id.clone());
        let auth = handle.check().expect("unrevoked handle must succeed");
        assert_eq!(auth.key_id, node.key_id);
    }

    #[test]
    fn handle_check_after_revoke_returns_key_revoked() {
        let (root_key, _node_key, root, node) = two_chain();
        let mut store = TrustStore::new(root).unwrap();
        store.add_authority(node.clone()).unwrap();
        let handle = CapabilityHandle::new(&store, node.key_id.clone());
        assert!(handle.check().is_ok());
        let list = RevocationList::build_and_sign(
            vec![RevocationEntry {
                key_id: node.key_id.clone(),
                reason: RevocationReason::OperatorRevoked,
                revoked_at: Utc::now(),
            }],
            &root_key,
        )
        .unwrap();
        store.set_revocation_list(list).unwrap();
        let result = handle.check();
        assert!(
            matches!(result, Err(TrustError::KeyRevoked { .. })),
            "revoked handle must return KeyRevoked, got {:?}",
            result
        );
    }

    #[test]
    fn handle_check_unknown_key_returns_unknown_authority() {
        let (_root_key, _node_key, root, _node) = two_chain();
        let store = TrustStore::new(root).unwrap();
        let handle = CapabilityHandle::new(&store, "not-a-real-key-id");
        assert!(matches!(
            handle.check(),
            Err(TrustError::UnknownAuthority(_))
        ));
    }

    #[test]
    fn handle_check_already_revoked_returns_key_revoked() {
        let (root_key, _node_key, root, node) = two_chain();
        let mut store = TrustStore::new(root).unwrap();
        store.add_authority(node.clone()).unwrap();
        let list = RevocationList::build_and_sign(
            vec![RevocationEntry {
                key_id: node.key_id.clone(),
                reason: RevocationReason::Compromised,
                revoked_at: Utc::now(),
            }],
            &root_key,
        )
        .unwrap();
        store.set_revocation_list(list).unwrap();
        let handle = CapabilityHandle::new(&store, node.key_id.clone());
        assert!(matches!(handle.check(), Err(TrustError::KeyRevoked { .. })));
    }

    #[test]
    fn handle_targets_specific_key_revocation_is_not_blanket() {
        let root_key = SigningKey::generate();
        let root = Authority::trust_root(&root_key, "root");
        let key_a = SigningKey::generate();
        let key_b = SigningKey::generate();
        let auth_a = Authority::signed_by(&key_a, &root_key, &root, "node-a", None).unwrap();
        let auth_b = Authority::signed_by(&key_b, &root_key, &root, "node-b", None).unwrap();
        let mut store = TrustStore::new(root).unwrap();
        store.add_authority(auth_a.clone()).unwrap();
        store.add_authority(auth_b.clone()).unwrap();
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
        .unwrap();
        store.set_revocation_list(list).unwrap();
        assert!(matches!(
            handle_a.check(),
            Err(TrustError::KeyRevoked { .. })
        ));
        assert!(
            handle_b.check().is_ok(),
            "revocation must be targeted, not blanket"
        );
    }
}
