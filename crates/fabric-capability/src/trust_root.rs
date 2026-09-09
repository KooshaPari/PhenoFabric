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
//! 1. For each signature in `descriptor.signatures`:
//!    a. Look up `key_id` in `by_key_id`. Missing → `UnknownAuthority`.
//!    b. If revocation list is set, check `key_id` is not in it. Found → `KeyRevoked`.
//!    c. Walk parent chain; at each step check `not_after > now()`,
//!       depth ≤ cap, parent exists. Depth > cap → `ChainTooDeep`.
//!    d. Walk back down, verifying each `Authority.signature` against
//!       the parent's `VerificationKey`.
//!    e. Verify the descriptor signature against the leaf.
//!    f. Return `Ok(ChainVerification { node_authority, chain_depth })`.
//! 2. If no signature yields a valid chain, return the FIRST error
//!    (more informative than the last).
//!
//! ## Backwards compatibility
//!
//! R0 `sign` / `verify` / `has_trusted_signature` are unchanged. The
//! `TrustStore` is opt-in.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::descriptor::{CapabilityDescriptor, Signature};
use crate::error::{Error, Result};
use crate::signing::{SigningKey, VerificationKey};

/// Maximum chain depth in R1. Compile-time constant; R2 may make it
/// configurable per-deployment.
pub const MAX_CHAIN_DEPTH: usize = 2;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors that [`TrustStore`] can produce during construction, authority
/// insertion, revocation-list installation, or chain verification.
#[derive(Debug, thiserror::Error)]
pub enum TrustError {
    #[error("unknown authority: key_id {0} is not in the TrustStore")]
    UnknownAuthority(String),

    #[error("key revoked: key_id {key_id}, reason: {reason:?}")]
    KeyRevoked {
        key_id: String,
        reason: RevocationReason,
    },

    #[error("authority expired: key_id {key_id}, expired_at {expired_at}")]
    Expired {
        key_id: String,
        expired_at: DateTime<Utc>,
    },

    #[error("chain not anchored at trust root: leaf key_id {0}")]
    ChainNotAnchored(String),

    #[error("chain too deep: depth {depth}, cap {cap}")]
    ChainTooDeep { depth: usize, cap: usize },

    #[error("bad signature on authority: key_id {0}")]
    BadAuthoritySignature(String),

    #[error("bad descriptor signature: key_id {0}")]
    BadDescriptorSignature(String),

    #[error("crypto error: {0}")]
    Crypto(String),

    #[error("trust root already set; cannot replace")]
    RootAlreadySet,

    #[error("parent key_id {0} not found when adding authority {1}")]
    ParentNotFound(String, String),

    #[error(
        "revocation list signature is not from trust root (expected {expected}, got {actual})"
    )]
    RevocationListNotFromRoot { expected: String, actual: String },

    #[error("io / serde: {0}")]
    Codec(String),
}

impl From<Error> for TrustError {
    fn from(e: Error) -> Self {
        TrustError::Crypto(e.to_string())
    }
}

// ---------------------------------------------------------------------------
// Revocation
// ---------------------------------------------------------------------------

/// Why a key was revoked. Used in [`RevocationEntry::reason`] and surfaced
/// in the [`TrustError::KeyRevoked`] error variant.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RevocationReason {
    /// Key material leaked or suspected compromised.
    Compromised,
    /// Key rotated; the new key replaces this one.
    Superseded,
    /// Host decommissioned; the key is no longer needed.
    Retired,
    /// Manual operator action (no automatic reason).
    OperatorRevoked,
}

/// A single revocation record. Multiple entries for the same `key_id`
/// are allowed; only the most recent reason wins in practice (the
/// revocation-list consumer picks one).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevocationEntry {
    pub key_id: String,
    pub reason: RevocationReason,
    pub revoked_at: DateTime<Utc>,
}

/// A signed bundle of revocation entries. The `signature` MUST be from
/// the TrustRoot's signing key; `TrustStore::set_revocation_list`
/// enforces this.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevocationList {
    pub revocations: Vec<RevocationEntry>,
    pub signed_at: DateTime<Utc>,
    pub signature: Signature,
}

impl RevocationList {
    /// Builds, signs (by the supplied root signing key), and returns a
    /// fresh `RevocationList`. The signature is over the canonical bytes
    /// of `(revocations, signed_at)` — same "no self-signature" pattern
    /// as `CapabilityDescriptor::canonical_bytes` and
    /// `Authority::canonical_bytes`.
    pub fn build_and_sign(
        revocations: Vec<RevocationEntry>,
        root_signing_key: &SigningKey,
    ) -> Result<Self> {
        let signed_at = Utc::now();
        // Build a stub for canonicalization (without signature).
        let stub = Self {
            revocations: revocations.clone(),
            signed_at,
            signature: Signature {
                key_id: String::new(),
                alg: String::new(),
                sig: String::new(),
                signed_at: Utc::now(),
            },
        };
        let bytes = stub.canonical_bytes()?;
        let signature = root_signing_key.sign_bytes(&bytes);
        Ok(Self {
            revocations,
            signed_at,
            signature,
        })
    }

    /// Returns the canonical bytes used as input to the signature.
    /// Strips the `signature` field.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
        let mut value = serde_json::to_value(self).map_err(|e| Error::Serde(e.to_string()))?;
        if let Some(obj) = value.as_object_mut() {
            obj.remove("signature");
        }
        serde_json::to_vec(&value).map_err(|e| Error::Serde(e.to_string()))
    }
}

// ---------------------------------------------------------------------------
// Authority
// ---------------------------------------------------------------------------

/// A single node in the trust chain. The TrustRoot is itself an
/// `Authority` with `parent_key_id == None` and `signature == None`
/// (or ignored — see spec §4 step 3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Authority {
    pub verification_key: VerificationKey,
    pub key_id: String,
    /// `None` iff this authority is the TrustRoot.
    pub parent_key_id: Option<String>,
    pub name: String,
    pub issued_at: DateTime<Utc>,
    /// `None` = no expiry.
    pub not_after: Option<DateTime<Utc>>,
    /// Parent's signature on this authority's canonical bytes.
    /// `None` iff this authority is the TrustRoot.
    pub signature: Option<Signature>,
}

impl Authority {
    /// Returns the canonical bytes used as input to the parent's
    /// signature. Strips the `signature` field before serializing.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
        let mut value = serde_json::to_value(self).map_err(|e| Error::Serde(e.to_string()))?;
        if let Some(obj) = value.as_object_mut() {
            obj.remove("signature");
        }
        serde_json::to_vec(&value).map_err(|e| Error::Serde(e.to_string()))
    }

    /// Builds a self-signed TrustRoot. The `signature` field is left
    /// `None` (a TrustRoot anchors by definition).
    pub fn trust_root(key: &SigningKey, name: impl Into<String>) -> Self {
        let verification_key = key.verification_key();
        let key_id = key.key_id().to_string();
        Self {
            verification_key,
            key_id,
            parent_key_id: None,
            name: name.into(),
            issued_at: Utc::now(),
            not_after: None,
            signature: None,
        }
    }

    /// Builds a non-root Authority signed by `parent`. The parent's
    /// `VerificationKey` is used to sign the canonical bytes of the new
    /// authority — i.e. the parent delegates authority to this key.
    /// The new authority holds the `key`'s verification counterpart.
    ///
    /// In a real deployment the parent's signing key would be held in a
    /// secure enclave / HSM; here we accept it as a `&SigningKey` for
    /// construction-site convenience. `TrustStore::add_authority`
    /// re-verifies the signature against the parent's *verification*
    /// key before insertion.
    pub fn signed_by(
        key: &SigningKey,
        parent_signing_key: &SigningKey,
        parent: &Authority,
        name: impl Into<String>,
        not_after: Option<DateTime<Utc>>,
    ) -> Result<Self> {
        let verification_key = key.verification_key();
        let key_id = key.key_id().to_string();
        let issued_at = Utc::now();
        let name: String = name.into();
        let stub = Self {
            verification_key: verification_key.clone(),
            key_id: key_id.clone(),
            parent_key_id: Some(parent.key_id.clone()),
            name: name.clone(),
            issued_at,
            not_after,
            signature: None,
        };
        let bytes = stub.canonical_bytes()?;
        let signature = parent_signing_key.sign_bytes(&bytes);
        Ok(Self {
            verification_key,
            key_id,
            parent_key_id: Some(parent.key_id.clone()),
            name,
            issued_at,
            not_after,
            signature: Some(signature),
        })
    }

    /// Returns the parent's verification key, if this authority has a
    /// parent (i.e., is not the TrustRoot).
    pub fn parent_key_id(&self) -> Option<&str> {
        self.parent_key_id.as_deref()
    }
}

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
    pub fn add_authority(&mut self, auth: Authority) -> std::result::Result<(), TrustError> {
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
            .ok_or_else(|| TrustError::ParentNotFound(parent_key_id.clone(), auth.key_id.clone()))?
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
                        first_error = Some(TrustError::UnknownAuthority(sig.key_id.clone()));
                    }
                    continue;
                }
            };
            // b. Revocation check.
            if let Some(list) = &self.revocation_list {
                if let Some(entry) = list.revocations.iter().find(|e| e.key_id == sig.key_id) {
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
            first_error.unwrap_or_else(|| TrustError::UnknownAuthority(String::from("<none>")))
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
                        return Err(TrustError::ChainNotAnchored(auth.key_id.clone()));
                    }
                    break;
                }
                Some(parent_id) => {
                    let parent = self
                        .by_key_id
                        .get(parent_id)
                        .ok_or_else(|| TrustError::UnknownAuthority(parent_id.clone()))?
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
                .ok_or_else(|| TrustError::BadAuthoritySignature(child.key_id.clone()))?;
            let bytes = child.canonical_bytes()?;
            parent
                .verification_key
                .verify_bytes(&bytes, sig)
                .map_err(|_| TrustError::BadAuthoritySignature(child.key_id.clone()))?;
        }
        // Verify the descriptor signature against the leaf's key.
        let leaf = &chain[0];
        let desc_bytes = descriptor
            .canonical_bytes()
            .map_err(|e| TrustError::Codec(e.to_string()))?;
        leaf.verification_key
            .verify_bytes(&desc_bytes, desc_sig)
            .map_err(|_| TrustError::BadDescriptorSignature(desc_sig.key_id.clone()))?;
        Ok(ChainVerification {
            node_authority: leaf.clone(),
            chain_depth: depth,
        })
    }
}

// ---------------------------------------------------------------------------
// Unit tests (spec 021 §7, T-TR01..05)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptor::Capabilities;
    use chrono::Duration;

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
    fn build_chain_3() -> (SigningKey, SigningKey, SigningKey, Authority, Authority, Authority) {
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
        let node =
            Authority::signed_by(&node_key, &inter_key, &inter, "test-node", None).unwrap();
        (root_key, inter_key, node_key, root, inter, node)
    }

    fn build_chain_2() -> (SigningKey, SigningKey, Authority, Authority) {
        let root_key = SigningKey::generate();
        let node_key = SigningKey::generate();
        let root = Authority::trust_root(&root_key, "test-root");
        let node = Authority::signed_by(&node_key, &root_key, &root, "test-node", None).unwrap();
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
        let list = RevocationList::build_and_sign(entries, &root_key).unwrap();
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
        let list = RevocationList::build_and_sign(entries, &wrong_key).unwrap();
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
        let extra = Authority::signed_by(&extra_key, &node_key, &node, "too-deep", None).unwrap();
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
