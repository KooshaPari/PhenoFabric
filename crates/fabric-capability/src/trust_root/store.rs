use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::descriptor::{CapabilityDescriptor, Signature};

use super::authority::Authority;
use super::types::{RevocationList, TrustError, MAX_CHAIN_DEPTH};

// ---------------------------------------------------------------------------
// ChainVerification
// ---------------------------------------------------------------------------

/// The successful result of `verify_chain`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainVerification {
    /// The leaf authority whose key signed the descriptor.
    pub node_authority: Authority,
    /// Number of hops from TrustRoot: 1 = direct, 2 = via intermediate.
    /// Capped at [`MAX_CHAIN_DEPTH`].
    pub chain_depth: usize,
}

// ---------------------------------------------------------------------------
// TrustStore
// ---------------------------------------------------------------------------

/// The runtime verifier. Constructed with a TrustRoot; the operator
/// extends it with intermediates / NodeAuthorities and (optionally) a
/// revocation list. `verify_chain` is the entry point used by callers
/// that want the new R1 guarantees.
#[derive(Debug, Clone)]
pub struct TrustStore {
    root_key_id: String,
    by_key_id: HashMap<String, Authority>,
    revocation_list: Option<RevocationList>,
}

impl TrustStore {
    /// Construct a new `TrustStore` anchored at `root`. The root must
    /// have `parent_key_id == None` (a TrustRoot is, by definition, the
    /// top of the chain). Its `signature` field is ignored.
    ///
    /// # Errors
    /// - [`TrustError::RootAlreadySet`] if the store is reused (not
    ///   possible through the public API since we only have `new`, but
    ///   kept for future extensibility).
    pub fn new(root: Authority) -> std::result::Result<Self, TrustError> {
        if root.parent_key_id.is_some() {
            return Err(TrustError::ChainNotAnchored(root.key_id));
        }
        let root_key_id = root.key_id.clone();
        let mut by_key_id = HashMap::new();
        by_key_id.insert(root_key_id.clone(), root);
        Ok(Self {
            root_key_id,
            by_key_id,
            revocation_list: None,
        })
    }

    /// Adds an authority to the store. Validates that:
    /// 1. The authority's `key_id` is not already present.
    /// 2. The authority's `parent_key_id` references an existing
    ///    authority in the store.
    /// 3. The authority's `signature` field is present and verifies
    ///    against the parent's `VerificationKey`.
    /// 4. The resulting chain depth does not exceed [`MAX_CHAIN_DEPTH`].
    pub fn add_authority(
        &mut self,
        auth: Authority,
    ) -> std::result::Result<(), TrustError> {
        if self.by_key_id.contains_key(&auth.key_id) {
            return Err(TrustError::Crypto(format!(
                "duplicate key_id {}",
                auth.key_id
            )));
        }
        let parent_key_id = auth
            .parent_key_id
            .clone()
            .ok_or_else(|| TrustError::ChainNotAnchored(auth.key_id.clone()))?;
        let parent = self
            .by_key_id
            .get(&parent_key_id)
            .ok_or_else(|| {
                TrustError::ParentNotFound(parent_key_id.clone(), auth.key_id.clone())
            })?
            .clone();
        // Chain-depth check: walk from the parent up to the root,
        // counting hops. Reject before signature verification (cheaper)
        // and before allowing the parent itself to be at the cap.
        let mut depth = 0usize;
        let mut cursor = parent.clone();
        loop {
            if depth > MAX_CHAIN_DEPTH {
                return Err(TrustError::ChainTooDeep {
                    depth,
                    cap: MAX_CHAIN_DEPTH,
                });
            }
            match &cursor.parent_key_id {
                None => break,
                Some(pid) => {
                    depth += 1;
                    cursor = self
                        .by_key_id
                        .get(pid)
                        .ok_or_else(|| {
                            TrustError::ParentNotFound(pid.clone(), auth.key_id.clone())
                        })?
                        .clone();
                }
            }
        }
        // The child would add one more hop.
        depth += 1;
        if depth > MAX_CHAIN_DEPTH {
            return Err(TrustError::ChainTooDeep {
                depth,
                cap: MAX_CHAIN_DEPTH,
            });
        }
        let bytes = auth.canonical_bytes()?;
        let signature = auth
            .signature
            .as_ref()
            .ok_or_else(|| TrustError::BadAuthoritySignature(auth.key_id.clone()))?;
        parent
            .verification_key
            .verify_bytes(&bytes, signature)
            .map_err(|_| TrustError::BadAuthoritySignature(auth.key_id.clone()))?;
        self.by_key_id.insert(auth.key_id.clone(), auth);
        Ok(())
    }

    /// Installs a revocation list. Validates that the list's signature
    /// is from the TrustRoot's signing key.
    pub fn set_revocation_list(
        &mut self,
        list: RevocationList,
    ) -> std::result::Result<(), TrustError> {
        let root = self
            .by_key_id
            .get(&self.root_key_id)
            .ok_or_else(|| TrustError::ChainNotAnchored(self.root_key_id.clone()))?;
        let bytes = list.canonical_bytes()?;
        let actual_key_id = list.signature.key_id.clone();
        root.verification_key
            .verify_bytes(&bytes, &list.signature)
            .map_err(|_| TrustError::RevocationListNotFromRoot {
                expected: self.root_key_id.clone(),
                actual: actual_key_id,
            })?;
        self.revocation_list = Some(list);
        Ok(())
    }

    /// Returns the TrustRoot's key_id (base64 of the root public key).
    pub fn root_key_id(&self) -> &str {
        &self.root_key_id
    }

    /// Verifies that `descriptor` carries at least one signature whose
    /// authority is anchored at the TrustRoot, not revoked, not expired,
    /// and within the chain-depth cap. Returns the deepest valid chain
    /// found.
    ///
    /// The first error encountered is returned (more informative than
    /// the last — see spec 021 §4 step 2).
    pub fn verify_chain(
        &self,
        descriptor: &CapabilityDescriptor,
    ) -> std::result::Result<ChainVerification, TrustError> {
        let mut first_error: Option<TrustError> = None;
        let mut best: Option<ChainVerification> = None;
        for sig in &descriptor.signatures {
            // a. Look up the authority for this signature.
            let auth = match self.by_key_id.get(&sig.key_id) {
                Some(a) => a.clone(),
                None => {
                    if first_error.is_none() {
                        first_error =
                            Some(TrustError::UnknownAuthority(sig.key_id.clone()));
                    }
                    continue;
                }
            };
            // b. Revocation check.
            if let Some(list) = &self.revocation_list {
                if let Some(entry) =
                    list.revocations.iter().find(|e| e.key_id == sig.key_id)
                {
                    if first_error.is_none() {
                        first_error = Some(TrustError::KeyRevoked {
                            key_id: sig.key_id.clone(),
                            reason: entry.reason.clone(),
                        });
                    }
                    continue;
                }
            }
            // c. Walk parent chain.
            let chain = match self.walk_chain(&auth, descriptor, sig) {
                Ok(c) => c,
                Err(e) => {
                    if first_error.is_none() {
                        first_error = Some(e);
                    }
                    continue;
                }
            };
            // Pick the deepest valid chain (more informative).
            match &best {
                Some(prev) if prev.chain_depth >= chain.chain_depth => {}
                _ => best = Some(chain),
            }
        }
        best.ok_or_else(|| {
            first_error.unwrap_or_else(|| {
                TrustError::UnknownAuthority(String::from("<none>"))
            })
        })
    }

    /// Internal: walks the parent chain from `auth`, verifying
    /// authority signatures, time validity, and chain depth. On success
    /// returns the descriptor-signature verification result.
    fn walk_chain(
        &self,
        auth: &Authority,
        descriptor: &CapabilityDescriptor,
        desc_sig: &Signature,
    ) -> std::result::Result<ChainVerification, TrustError> {
        // Walk UP, collecting authorities and checking time + depth.
        let mut chain: Vec<Authority> = Vec::new();
        let mut current = auth.clone();
        loop {
            // Time validity check at every level.
            if let Some(not_after) = current.not_after {
                if not_after <= Utc::now() {
                    return Err(TrustError::Expired {
                        key_id: current.key_id.clone(),
                        expired_at: not_after,
                    });
                }
            }
            chain.push(current.clone());
            match &current.parent_key_id {
                None => {
                    // Must be the root.
                    if current.key_id != self.root_key_id {
                        return Err(TrustError::ChainNotAnchored(
                            auth.key_id.clone(),
                        ));
                    }
                    break;
                }
                Some(parent_id) => {
                    let parent = self
                        .by_key_id
                        .get(parent_id)
                        .ok_or_else(|| {
                            TrustError::UnknownAuthority(parent_id.clone())
                        })?
                        .clone();
                    current = parent;
                }
            }
        }
        // chain is [leaf, ..., root]. depth = chain.len() - 1 hops (root has 0 hops).
        let depth = chain.len() - 1;
        if depth > MAX_CHAIN_DEPTH {
            return Err(TrustError::ChainTooDeep {
                depth,
                cap: MAX_CHAIN_DEPTH,
            });
        }
        // Walk DOWN, verifying each authority's signature against its
        // parent's verification key. chain[0] is leaf, chain[last] is root.
        // For each level i, parent is chain[i+1].
        for i in 0..chain.len() - 1 {
            let child = &chain[i];
            let parent = &chain[i + 1];
            let sig = child
                .signature
                .as_ref()
                .ok_or_else(|| {
                    TrustError::BadAuthoritySignature(child.key_id.clone())
                })?;
            let bytes = child.canonical_bytes()?;
            parent
                .verification_key
                .verify_bytes(&bytes, sig)
                .map_err(|_| {
                    TrustError::BadAuthoritySignature(child.key_id.clone())
                })?;
        }
        // Verify the descriptor signature against the leaf's key.
        let leaf = &chain[0];
        let desc_bytes = descriptor
            .canonical_bytes()
            .map_err(|e| TrustError::Codec(e.to_string()))?;
        leaf.verification_key
            .verify_bytes(&desc_bytes, desc_sig)
            .map_err(|_| {
                TrustError::BadDescriptorSignature(desc_sig.key_id.clone())
            })?;
        Ok(ChainVerification {
            node_authority: leaf.clone(),
            chain_depth: depth,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptor::Capabilities;
    use crate::signing::SigningKey;
    use crate::trust_root::types::{RevocationEntry, RevocationList, RevocationReason};
    use chrono::{Duration, Utc};

    fn minimal_descriptor() -> CapabilityDescriptor {
        CapabilityDescriptor {
            node_id: uuid::Uuid::now_v7(),
            epoch: 1,
            schema_version: "1.0.0".into(),
            probed_at: Utc::now(),
            probe_version: "0.1.0".into(),
            topology_hash: blake3::hash(&[]).to_hex().to_string(),
            capabilities: Capabilities::default(),
            signatures: vec![],
        }
    }

    /// Builds a 3-level chain: root → intermediate → node, each signed
    /// by the previous. Returns the keys + authorities.
    fn build_chain_3(
    ) -> (SigningKey, SigningKey, SigningKey, Authority, Authority, Authority) {
        let root_key = SigningKey::generate();
        let inter_key = SigningKey::generate();
        let node_key = SigningKey::generate();
        let root = Authority::trust_root(&root_key, "test-root");
        let inter = Authority::signed_by(
            &inter_key,
            &root_key,
            &root,
            "test-intermediate",
            None,
        )
        .unwrap();
        let node = Authority::signed_by(
            &node_key,
            &inter_key,
            &inter,
            "test-node",
            None,
        )
        .unwrap();
        (root_key, inter_key, node_key, root, inter, node)
    }

    fn build_chain_2() -> (SigningKey, SigningKey, Authority, Authority) {
        let root_key = SigningKey::generate();
        let node_key = SigningKey::generate();
        let root = Authority::trust_root(&root_key, "test-root");
        let node = Authority::signed_by(
            &node_key,
            &root_key,
            &root,
            "test-node",
            None,
        )
        .unwrap();
        (root_key, node_key, root, node)
    }

    #[test]
    fn tr01_root_must_have_no_parent() {
        let root_key = SigningKey::generate();
        let bogus_root = Authority {
            verification_key: root_key.verification_key(),
            key_id: root_key.key_id().to_string(),
            parent_key_id: Some("not-root".into()),
            name: "fake".into(),
            issued_at: Utc::now(),
            not_after: None,
            signature: None,
        };
        let result = TrustStore::new(bogus_root);
        assert!(matches!(result, Err(TrustError::ChainNotAnchored(_))));
    }

    #[test]
    fn tr02_add_authority_and_lookup() {
        let (_root_key, _node_key, root, node) = build_chain_2();
        let mut store = TrustStore::new(root).unwrap();
        assert_eq!(store.root_key_id(), store.root_key_id());
        store.add_authority(node.clone()).unwrap();
        // Lookup by key_id should yield the node authority.
        assert!(store.by_key_id.contains_key(&node.key_id));
    }

    #[test]
    fn tr03_revocation_list_round_trip() {
        let (root_key, _node_key, root, _node) = build_chain_2();
        let store = TrustStore::new(root.clone()).unwrap();
        let entries = vec![RevocationEntry {
            key_id: "deadbeef".into(),
            reason: RevocationReason::Compromised,
            revoked_at: Utc::now(),
        }];
        let list =
            RevocationList::build_and_sign(entries, &root_key).unwrap();
        let bytes = list.canonical_bytes().unwrap();
        store
            .by_key_id
            .get(&root.key_id)
            .unwrap()
            .verification_key
            .verify_bytes(&bytes, &list.signature)
            .unwrap();
    }

    #[test]
    fn tr04_revocation_list_wrong_signer_rejected() {
        let (_root_key, _node_key, root, _node) = build_chain_2();
        let wrong_key = SigningKey::generate();
        let entries = vec![RevocationEntry {
            key_id: "deadbeef".into(),
            reason: RevocationReason::Compromised,
            revoked_at: Utc::now(),
        }];
        let list =
            RevocationList::build_and_sign(entries, &wrong_key).unwrap();
        let mut store = TrustStore::new(root).unwrap();
        let result = store.set_revocation_list(list);
        assert!(matches!(
            result,
            Err(TrustError::RevocationListNotFromRoot { .. })
        ));
    }

    #[test]
    fn tr05_chain_too_deep() {
        let (root_key, inter_key, node_key, root, inter, node) = build_chain_3();
        let mut store = TrustStore::new(root.clone()).unwrap();
        store.add_authority(inter.clone()).unwrap();
        store.add_authority(node.clone()).unwrap();
        // Now build a 4th authority signed by `node` — this is depth 3
        // which exceeds MAX_CHAIN_DEPTH=2.
        let extra_key = SigningKey::generate();
        let extra = Authority::signed_by(
            &extra_key,
            &node_key,
            &node,
            "too-deep",
            None,
        )
        .unwrap();
        let result = store.add_authority(extra);
        // Should fail because parent (node) is at depth 2, and adding
        // child makes depth 3.
        assert!(matches!(result, Err(TrustError::ChainTooDeep { .. })));
        // Silence unused.
        let _ = (root_key, inter_key, node_key);
    }

    #[test]
    fn tr06_verify_chain_happy_path() {
        let (_root_key, node_key, root, node) = build_chain_2();
        let mut store = TrustStore::new(root).unwrap();
        store.add_authority(node.clone()).unwrap();
        let mut d = minimal_descriptor();
        crate::signing::sign(&mut d, &node_key).unwrap();
        let v = store.verify_chain(&d).unwrap();
        assert_eq!(v.chain_depth, 1, "direct chain R -> A = 1 hop");
        assert_eq!(v.node_authority.key_id, node.key_id);
    }

    #[test]
    fn tr07_verify_chain_revoked() {
        let (root_key, node_key, root, node) = build_chain_2();
        let mut store = TrustStore::new(root.clone()).unwrap();
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
        let mut d = minimal_descriptor();
        crate::signing::sign(&mut d, &node_key).unwrap();
        let result = store.verify_chain(&d);
        assert!(matches!(result, Err(TrustError::KeyRevoked { .. })));
    }

    #[test]
    fn tr08_verify_chain_expired() {
        // Rebuild with an expired not_after.
        let root_key = SigningKey::generate();
        let node_key = SigningKey::generate();
        let root = Authority::trust_root(&root_key, "test-root");
        let node = Authority::signed_by(
            &node_key,
            &root_key,
            &root,
            "expired-node",
            Some(Utc::now() - Duration::hours(1)),
        )
        .unwrap();
        let mut store = TrustStore::new(root).unwrap();
        store.add_authority(node).unwrap();
        let mut d = minimal_descriptor();
        crate::signing::sign(&mut d, &node_key).unwrap();
        let result = store.verify_chain(&d);
        assert!(matches!(result, Err(TrustError::Expired { .. })));
    }

    #[test]
    fn tr09_verify_chain_unknown_authority() {
        let (_root_key, _node_key, root, _node) = build_chain_2();
        let store = TrustStore::new(root).unwrap();
        let mut d = minimal_descriptor();
        let bogus_key = SigningKey::generate();
        crate::signing::sign(&mut d, &bogus_key).unwrap();
        let result = store.verify_chain(&d);
        assert!(matches!(result, Err(TrustError::UnknownAuthority(_))));
    }

    #[test]
    fn tr10_authority_canonical_bytes_strip_signature() {
        let root_key = SigningKey::generate();
        let auth = Authority::trust_root(&root_key, "x");
        let bytes1 = auth.canonical_bytes().unwrap();
        // Mutate signature field (none on root, but ensure deterministic).
        let bytes2 = auth.canonical_bytes().unwrap();
        assert_eq!(bytes1, bytes2);
        // Add a signature and re-canonicalize — bytes should not include it.
        let mut with_sig = auth.clone();
        with_sig.signature = Some(Signature {
            key_id: "k".into(),
            alg: "Ed25519".into(),
            sig: "x".into(),
            signed_at: Utc::now(),
        });
        let bytes3 = with_sig.canonical_bytes().unwrap();
        assert_eq!(bytes1, bytes3, "canonical bytes must ignore signature");
    }
}
