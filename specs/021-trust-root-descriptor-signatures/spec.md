# Spec 021 — Trust-Root Model for Capability Descriptor Signatures (PF-WP-016, R1)

**Status:** draft
**WP:** PF-WP-016
**Release gate:** R1
**Depends on:** [adr/0023-capability-inventory-schema.md](../../adr/0023-capability-inventory-schema.md) (signature format), [adr/0028-testdata-verification-pattern.md](../../adr/0028-testdata-verification-pattern.md) (fixtures unchanged)
**Enables:** PF-WP-017 (workspace persistence — wire revocation events into the workspace event log), PF-WP-022 R3 (multi-tenant fairness)

## 1. Background

ADR-0023 ships `CapabilityDescriptor` with Ed25519 signatures. The R0
trust model is "the operator configures a list of trusted key_ids" —
`descriptor.has_trusted_signature(&[key_id_string])`. There is no:

1. **Revocation.** A compromised key keeps being trusted.
2. **Chain.** A key is either trusted or not — no delegation, no
   intermediates.
3. **Time validity.** A key is trusted forever, regardless of when it
   was issued.
4. **Audit trail.** No way to ask "why is this key trusted?" — the
   trust decision is a boolean lookup.

WORKLOG.md:107 (R0 risk ledger) flagged this explicitly. ADR-0031
Accepted (this spec's parent ADR) ratifies the TrustRoot model.

## 2. Scope

In:
- `fabric_capability::trust_root` module — `Authority`, `TrustStore`,
  `RevocationList`, `RevocationEntry`, `RevocationReason`, `ChainVerification`,
  `TrustError`, and `verify_chain`.
- Bounded chain depth (cap = 2 in R1; compile-time constant).
- `not_after` time validity on every Authority.
- Self-signed TrustRoot (`parent_key_id == None`).
- Revocation list signed by the TrustRoot.

Out:
- Sidecar fetch of revocation lists (R2).
- Multiple trust roots (R2).
- Chain depth > 2 (R3+; compile-time constant keeps the door open).
- OCSP / CRL DP / CRL Number (R3+).
- Changing the on-the-wire signature format (ADR-0023 §5 stays).

## 3. Locked types

Read `crates/fabric-capability/src/{signing, descriptor, error}.rs` first
per ADR-0028.

```rust
// trust_root.rs — read signing.rs and descriptor.rs BEFORE editing.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::descriptor::{CapabilityDescriptor, Signature};
use crate::error::{Error, Result};
use crate::signing::{SigningKey, VerificationKey};

/// Maximum chain depth in R1. Compile-time constant; R2 may make it
/// configurable.
pub const MAX_CHAIN_DEPTH: usize = 2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RevocationReason {
    Compromised,
    Superseded,
    Retired,
    OperatorRevoked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevocationEntry {
    pub key_id: String,
    pub reason: RevocationReason,
    pub revoked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevocationList {
    pub revocations: Vec<RevocationEntry>,
    pub signed_at: DateTime<Utc>,
    pub signature: Signature,           // MUST be from TrustRoot
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Authority {
    pub verification_key: VerificationKey,
    pub key_id: String,
    pub parent_key_id: Option<String>,  // None iff TrustRoot
    pub name: String,
    pub issued_at: DateTime<Utc>,
    pub not_after: Option<DateTime<Utc>>,
    pub signature: Option<Signature>,   // parent's signature on canonical bytes
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainVerification {
    pub node_authority: Authority,      // the leaf that signed the descriptor
    pub chain_depth: usize,             // 1 = direct, 2 = via intermediate
}

#[derive(Debug, thiserror::Error)]
pub enum TrustError {
    #[error("unknown authority: key_id {0} is not in the TrustStore")]
    UnknownAuthority(String),
    #[error("key revoked: key_id {key_id}, reason: {reason:?}")]
    KeyRevoked { key_id: String, reason: RevocationReason },
    #[error("authority expired: key_id {key_id}, expired_at {expired_at}")]
    Expired { key_id: String, expired_at: DateTime<Utc> },
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
    #[error("revocation list signature is not from trust root (expected {expected}, got {actual})")]
    RevocationListNotFromRoot { expected: String, actual: String },
    #[error("io / serde: {0}")]
    Codec(String),
}

impl From<Error> for TrustError {
    fn from(e: Error) -> Self { TrustError::Crypto(e.to_string()) }
}

pub struct TrustStore {
    root_key_id: String,
    by_key_id: HashMap<String, Authority>,
    revocation_list: Option<RevocationList>,
}

impl TrustStore {
    pub fn new(root: Authority) -> std::result::Result<Self, TrustError>;
    pub fn add_authority(&mut self, auth: Authority) -> std::result::Result<(), TrustError>;
    pub fn set_revocation_list(&mut self, list: RevocationList)
        -> std::result::Result<(), TrustError>;
    pub fn root_key_id(&self) -> &str;
    pub fn verify_chain(&self, descriptor: &CapabilityDescriptor)
        -> std::result::Result<ChainVerification, TrustError>;
}

impl Authority {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>>;   // strips `signature`
}
```

## 4. Verification contract

`verify_chain` MUST follow the algorithm in ADR-0031 §5:

1. For each signature in `descriptor.signatures` (in order):
   a. Look up signature.key_id in `by_key_id`. Missing → `UnknownAuthority`.
   b. If revocation list is set, check signature.key_id is not in it. Found → `KeyRevoked`.
   c. Walk the parent chain (depth = 0 starts at the signature's authority):
      - At each step, check `authority.not_after > now()`. Expired → `Expired`.
      - If `parent_key_id == None`, the key_id MUST equal `root_key_id`. Otherwise → `ChainNotAnchored`.
      - Else look up the parent; missing → `UnknownAuthority`.
      - depth += 1; if depth > MAX_CHAIN_DEPTH → `ChainTooDeep`.
   d. Walk back down, verifying each `Authority.signature` against the parent's verification key. Bad → `BadAuthoritySignature`.
   e. Verify the descriptor signature against the leaf authority's verification key. Bad → `BadDescriptorSignature`.
   f. Return `Ok(ChainVerification { node_authority: leaf, chain_depth })`.

2. If no signature yields a valid chain, return the *first* error encountered
   (not the last — the first is more informative).

3. The TrustRoot itself is allowed to be self-signed: when constructing
   `TrustStore::new(root)`, the TrustRoot's `signature` field is ignored
   (a TrustRoot anchors by definition; there is no parent to verify
   against).

## 5. Backwards compatibility

R0 callers (`sign`, `verify`, `has_trusted_signature`) are unchanged.
This spec adds a NEW path (`TrustStore::verify_chain`); it does not
remove or modify the old path. R0 fixture verification (ADR-0028) is
unaffected because fixtures don't carry chain metadata.

## 6. Acceptance criteria

1. `fabric_capability::trust_root::TrustStore::new(root)` returns
   `Ok(TrustStore)` for a TrustRoot with `parent_key_id == None`.
2. Given:
   - TrustRoot R
   - NodeAuthority A signed by R (parent_key_id == R.key_id)
   - CapabilityDescriptor D signed by A's signing key
   Then `TrustStore::new(R).add_authority(A)?.verify_chain(&D)` returns
   `Ok(ChainVerification { chain_depth: 1, .. })`.
3. Same setup, but A is added to `RevocationList` with reason
   `Compromised`: `verify_chain` returns `Err(KeyRevoked { .. })`.
4. Same setup, but A.not_after = some past timestamp: `verify_chain`
   returns `Err(Expired { .. })`.
5. Same setup, but A is signed by Intermediate I, and I is signed by R:
   `verify_chain` returns `Ok(ChainVerification { chain_depth: 2, .. })`.
6. A 3-level chain (R → I1 → I2 → A) returns `Err(ChainTooDeep)`.
7. A descriptor signed by an unknown key returns `Err(UnknownAuthority)`.
8. R0 `verify()` and `has_trusted_signature()` continue to work
   (signing.rs tests pass unchanged).

## 7. Test plan

Unit tests in `trust_root.rs` (5):
- T-TR01: TrustStore::new rejects duplicate root
- T-TR02: Authority build + add to store + lookup by key_id
- T-TR03: RevocationList sign + verify round-trip (signed by root key)
- T-TR04: RevocationList with wrong signer is rejected
- T-TR05: ChainTooDeep is reported for depth > 2

Integration tests in `tests/trust_root_chain.rs` (3):
- T-TC01: happy path — R → A → D, all signatures valid → Ok
- T-TC02: revocation — R → A → D, A in revocation list → KeyRevoked
- T-TC03: expired — R → A → D, A.not_after = past → Expired

## 8. Rollout

- ADR-0031 lands first (Accepted).
- This spec lands second (draft).
- `trust_root.rs` lands third (impl + 5 unit tests).
- `tests/trust_root_chain.rs` lands fourth (3 integration tests).
- `lib.rs` adds `pub mod trust_root;` and re-exports.
- 4/4 spec checks remain green; MANIFEST is regen'd.
- Commit + WORKLOG addendum + meta/PHENOTYPE_ARCHITECTURE.md addendum.

The actual Rust implementation lands in this same turn per the
operator direction "poc on all".
