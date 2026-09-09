//! Integration tests: end-to-end trust chain construction and verification.
//!
//! These tests exercise the *external* API surface only — `TrustStore`,
//! `Authority`, `RevocationList` — without poking into private internals.
//! They demonstrate:
//!
//! - 3-tier chain (root → intermediate → leaf) verifies successfully
//! - Revocation propagates from root to leaf
//! - Expiration rejection
//! - Two-trust-store isolation (a descriptor signed by store A's chain
//!   must NOT verify against store B's root)
//! - Wire-format round-trip (Authority/RevocationList serialize +
//!   deserialize without losing signature validity)
//! - Chain depth cap enforced at construction

use chrono::{Duration, Utc};
use fabric_capability::descriptor::Capabilities;
use fabric_capability::{
    sign, Authority, CapabilityDescriptor, RevocationEntry, RevocationList, RevocationReason,
    SigningKey, TrustError, TrustStore,
};
use uuid::Uuid;

fn test_descriptor() -> CapabilityDescriptor {
    CapabilityDescriptor {
        node_id: Uuid::now_v7(),
        epoch: 1,
        schema_version: "1.0.0".to_string(),
        probed_at: Utc::now(),
        probe_version: "0.1.0".to_string(),
        topology_hash: "abc".to_string(),
        capabilities: Capabilities::default(),
        signatures: vec![],
    }
}

#[test]
fn it01_three_tier_chain_verifies() {
    // root -> intermediate -> leaf
    let root_key = SigningKey::generate();
    let inter_key = SigningKey::generate();
    let leaf_key = SigningKey::generate();

    let root = Authority::trust_root(&root_key, "test-root");
    let inter = Authority::signed_by(&inter_key, &root_key, &root, "intermediate", None).unwrap();
    let leaf = Authority::signed_by(&leaf_key, &inter_key, &inter, "leaf", None).unwrap();

    let mut store = TrustStore::new(root).unwrap();
    store.add_authority(inter.clone()).unwrap();
    store.add_authority(leaf.clone()).unwrap();

    let mut d = test_descriptor();
    sign(&mut d, &leaf_key).unwrap();

    let v = store.verify_chain(&d).expect("3-tier chain should verify");
    assert_eq!(v.chain_depth, 2, "root -> inter -> leaf = 2 hops");
    assert_eq!(v.node_authority.key_id, leaf.key_id);
}

#[test]
fn it02_revocation_propagates_from_root() {
    let root_key = SigningKey::generate();
    let inter_key = SigningKey::generate();
    let leaf_key = SigningKey::generate();

    let root = Authority::trust_root(&root_key, "test-root");
    let inter = Authority::signed_by(&inter_key, &root_key, &root, "intermediate", None).unwrap();
    let leaf = Authority::signed_by(&leaf_key, &inter_key, &inter, "leaf", None).unwrap();

    let mut store = TrustStore::new(root).unwrap();
    store.add_authority(inter).unwrap();
    store.add_authority(leaf.clone()).unwrap();

    // Root revokes the leaf.
    let list = RevocationList::build_and_sign(
        vec![RevocationEntry {
            key_id: leaf.key_id.clone(),
            reason: RevocationReason::Compromised,
            revoked_at: Utc::now(),
        }],
        &root_key,
    )
    .unwrap();
    store.set_revocation_list(list).unwrap();

    let mut d = test_descriptor();
    sign(&mut d, &leaf_key).unwrap();
    let result = store.verify_chain(&d);
    assert!(
        matches!(result, Err(TrustError::KeyRevoked { .. })),
        "revoked leaf must be rejected: got {:?}",
        result
    );
}

#[test]
fn it03_expired_authority_rejected() {
    let root_key = SigningKey::generate();
    let leaf_key = SigningKey::generate();

    let root = Authority::trust_root(&root_key, "test-root");
    let leaf = Authority::signed_by(
        &leaf_key,
        &root_key,
        &root,
        "leaf",
        Some(Utc::now() - Duration::hours(1)),
    )
    .unwrap();
    let mut store = TrustStore::new(root).unwrap();
    store.add_authority(leaf).unwrap();

    let mut d = test_descriptor();
    sign(&mut d, &leaf_key).unwrap();
    let result = store.verify_chain(&d);
    assert!(matches!(result, Err(TrustError::Expired { .. })));
}

#[test]
fn it04_stores_are_isolated() {
    // Build two independent trust worlds and verify cross-root rejection.
    let root_a_key = SigningKey::generate();
    let root_b_key = SigningKey::generate();
    let leaf_a_key = SigningKey::generate();

    let root_a = Authority::trust_root(&root_a_key, "root-a");
    let root_b = Authority::trust_root(&root_b_key, "root-b");

    let leaf_a = Authority::signed_by(&leaf_a_key, &root_a_key, &root_a, "leaf-a", None).unwrap();

    let mut store_a = TrustStore::new(root_a).unwrap();
    store_a.add_authority(leaf_a.clone()).unwrap();

    // store_b only has root_b. It should NOT verify a descriptor signed
    // by leaf_a — even though leaf_a's signature is valid cryptographically,
    // the chain is anchored at root_a, not root_b.
    let store_b = TrustStore::new(root_b).unwrap();
    let mut d = test_descriptor();
    sign(&mut d, &leaf_a_key).unwrap();
    let result = store_b.verify_chain(&d);
    assert!(
        matches!(result, Err(TrustError::UnknownAuthority(_))),
        "cross-store verification must be rejected: got {:?}",
        result
    );

    // Sanity: store_a does accept the descriptor.
    let v = store_a.verify_chain(&d).expect("store_a should verify");
    assert_eq!(v.chain_depth, 1);
}

#[test]
fn it05_authority_wire_format_roundtrip() {
    // Sign an Authority, serialize to JSON, deserialize, verify the
    // signature still works. Critical for storage and transport.
    let root_key = SigningKey::generate();
    let node_key = SigningKey::generate();

    let root = Authority::trust_root(&root_key, "test-root");
    let node = Authority::signed_by(&node_key, &root_key, &root, "node", None).unwrap();

    let json = serde_json::to_string(&node).unwrap();
    let restored: Authority = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.key_id, node.key_id);
    assert_eq!(restored.parent_key_id, node.parent_key_id);
    let r_sig = restored.signature.as_ref().expect("restored signature");
    let n_sig = node.signature.as_ref().expect("original signature");
    assert_eq!(r_sig.key_id, n_sig.key_id);
    assert_eq!(r_sig.sig, n_sig.sig);

    // Verify the restored Authority's signature is still valid against
    // the parent's key.
    let mut store = TrustStore::new(root).unwrap();
    store.add_authority(restored).expect("restored auth must be acceptable");
}

#[test]
fn it06_revocation_list_wire_format_roundtrip() {
    let root_key = SigningKey::generate();
    let entries = vec![
        RevocationEntry {
            key_id: "v1.deadbeef".to_string(),
            reason: RevocationReason::Compromised,
            revoked_at: Utc::now(),
        },
        RevocationEntry {
            key_id: "v1.cafebabe".to_string(),
            reason: RevocationReason::Superseded,
            revoked_at: Utc::now(),
        },
    ];
    let list = RevocationList::build_and_sign(entries, &root_key).unwrap();
    let json = serde_json::to_string(&list).unwrap();
    let restored: RevocationList = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.revocations.len(), 2);
    assert_eq!(restored.signature.sig, list.signature.sig);

    let root = Authority::trust_root(&root_key, "test-root");
    let mut store = TrustStore::new(root).unwrap();
    store.set_revocation_list(restored).expect("restored list must be accepted");
}

#[test]
fn it07_chain_depth_enforced() {
    // root -> inter -> leaf -> extra = depth 3, exceeds MAX_CHAIN_DEPTH (2)
    let root_key = SigningKey::generate();
    let inter_key = SigningKey::generate();
    let leaf_key = SigningKey::generate();
    let extra_key = SigningKey::generate();

    let root = Authority::trust_root(&root_key, "test-root");
    let inter = Authority::signed_by(&inter_key, &root_key, &root, "inter", None).unwrap();
    let leaf = Authority::signed_by(&leaf_key, &inter_key, &inter, "leaf", None).unwrap();
    let extra = Authority::signed_by(&extra_key, &leaf_key, &leaf, "extra", None).unwrap();

    let mut store = TrustStore::new(root).unwrap();
    store.add_authority(inter).unwrap();
    store.add_authority(leaf).unwrap();
    let result = store.add_authority(extra);
    assert!(
        matches!(result, Err(TrustError::ChainTooDeep { .. })),
        "3-deep chain must be rejected: got {:?}",
        result
    );
}
