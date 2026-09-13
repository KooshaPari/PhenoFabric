# ADR-0031 — Trust-Root Model for Descriptor Signatures (PF-WP-016, R1)

**Status:** Accepted (impl in `crates/fabric-capability/src/trust_root.rs`, spec 021)
**Date:** 2026-09-08
**Deciders:** fabric-runtime
**Supersedes:** ADR-0023 §6 (direct-key trust model) — the schema is unchanged; only the trust *model* changes

## Context

ADR-0023 (Accepted, R0) established the Ed25519 + blake3 + JSON Schema
signature model for `CapabilityDescriptor`. It also said (§6):

> **Trust model:** a `VerificationKey` is trusted because the operator
> configured it. No revocation. R1 needs a trust-root or CA model.

ADR-0028 (Accepted) made fixture verification a hard gate but did not
address how a peer knows "the key that signed this descriptor is still
trustworthy". A compromised key in the R0 model could keep signing
descriptors indefinitely because there is no way to revoke it.

WORKLOG.md:107 (R0 risk ledger) flagged this explicitly:

> **No adversary model for signed descriptors.** A node can lie about
> its capabilities. R0 has a trust model (direct key) but no
> revocation. R1 needs a trust-root or CA model.

## Decision

Promote the trust model from direct-key to **single-level trust-root
with optional intermediates**, plus a signed **revocation list**. The
on-the-wire signature format (Ed25519 + key_id + base64 sig + UTC
timestamp) is unchanged — only the *verification* path changes.

### 1. Roles

| Role | Definition | Holds |
|:--|:--|:--|
| `TrustRoot` | A self-signed authority at the top of the chain. The operator pins it out-of-band (CLI arg, config file, TPM). | Ed25519 keypair; root can issue any number of sub-authorities and sign revocation lists. |
| `IntermediateAuthority` | Optional. Signed by TrustRoot. Can issue sub-authorities. Used for fleet-segmentation (e.g., one intermediate per datacenter). | Ed25519 keypair; chain length cap = 2 in R1. |
| `NodeAuthority` | The leaf authority. Signed directly by TrustRoot *or* by an Intermediate. The descriptor's signing key is `NodeAuthority.key`. | Ed25519 verification key (the node's public key). |

### 2. `Authority` shape

```rust
pub struct Authority {
    pub verification_key: VerificationKey,   // free fns use this
    pub key_id: String,                       // base64(public_key_bytes)
    pub parent_key_id: Option<String>,        // None iff TrustRoot
    pub name: String,                         // human-readable: "edge-pop-iad"
    pub issued_at: DateTime<Utc>,
    pub not_after: Option<DateTime<Utc>>,     // None = no expiry
    pub signature: Option<Signature>,         // signature from parent on (key_id, name, issued_at, not_after)
}
```

The signature is over `Authority.canonical_bytes()` which is the JSON
serialization of the *other* fields (key_id, parent_key_id, name,
issued_at, not_after) — the same "no self-signature" pattern
`CapabilityDescriptor` uses (ADR-0023 §5).

### 3. `RevocationList`

```rust
pub struct RevocationList {
    pub revocations: Vec<RevocationEntry>,
    pub signed_at: DateTime<Utc>,
    pub signature: Signature,                 // MUST be from TrustRoot
}

pub struct RevocationEntry {
    pub key_id: String,
    pub reason: RevocationReason,
    pub revoked_at: DateTime<Utc>,
}

pub enum RevocationReason {
    Compromised,        // key material leaked
    Superseded,         // rotated to a new key
    Retired,            // host decommissioned
    OperatorRevoked,    // manual operator action
}
```

The list is itself a signed artifact: any verifier that holds the
TrustRoot can verify a list it didn't fetch from the publisher. R1
ships the list inline (passed to `TrustStore::add_revocation_list`).
R2 can add a sidecar fetch path.

### 4. `TrustStore`

```rust
pub struct TrustStore {
    root: Authority,                          // TrustRoot; parent_key_id == None
    by_key_id: HashMap<String, Authority>,    // all known authorities, keyed by key_id
    revocation_list: Option<RevocationList>,  // if present, checked at verify_chain
}

impl TrustStore {
    pub fn new(root: Authority) -> Result<Self, TrustError>;
    pub fn add_authority(&mut self, auth: Authority) -> Result<(), TrustError>;
    pub fn set_revocation_list(&mut self, list: RevocationList) -> Result<(), TrustError>;
    pub fn root_key_id(&self) -> &str;
    pub fn verify_chain(&self, descriptor: &CapabilityDescriptor)
        -> Result<ChainVerification, TrustError>;
}
```

### 5. Verification contract (`verify_chain`)

For each signature in `descriptor.signatures`:

1. Look up the signature's `key_id` in `self.by_key_id`.
   - If absent → `Err(TrustError::UnknownAuthority { key_id })`.
2. If `self.revocation_list.is_some()`, check the key_id is not in it.
   - If present → `Err(TrustError::KeyRevoked { key_id, reason })`.
3. Check `authority.not_after > now()`. If expired → `Err(TrustError::Expired { key_id, expired_at })`.
4. Walk up the parent chain:
   - If `authority.parent_key_id == None`, it MUST equal `self.root.key_id`. Otherwise the chain has no anchor → `Err(TrustError::ChainNotAnchored { key_id })`.
   - If `parent_key_id != None`, look up the parent in `self.by_key_id`, repeat steps 2–4.
   - **Chain depth cap: 2** (TrustRoot → Intermediate → NodeAuthority). Deeper chains → `Err(TrustError::ChainTooDeep { depth })`.
5. Verify the parent signature on this Authority (the `Authority.signature` field).
6. Verify the descriptor signature against `authority.verification_key`.
   - If valid → return `Ok(ChainVerification { node_authority, chain_depth })`.
   - Otherwise → `Err(TrustError::BadDescriptorSignature { key_id })`.

The first signature that passes wins (descending priority order: most-recent-first in `descriptor.signatures` if any). The function returns the **deepest** valid chain, which is what the caller cares about (a 2-hop chain is more informative than a 1-hop chain).

### 6. Backwards compatibility

The R0 `sign()` / `verify()` / `has_trusted_signature()` API is unchanged.
R0 callers that pass `&[String]` of trusted key_ids continue to work —
they just don't get the new chain + revocation guarantees. The
`TrustStore` is opt-in: a node that wants the new model constructs one
and calls `verify_chain`; everyone else keeps calling `verify()`.

This is deliberate. ADR-0023's signature format is the on-the-wire
contract; R1 only adds a richer *verifier* path. No fixture changes
(ADR-0028 still gates fixtures).

### 7. Out of scope for R1

- Sidecar fetch of revocation lists (R2; revocation lists in R1 are
  passed in by the operator or by `fabric workspace`).
- CRL DP / OCSP / CRL Number extension (R2+; R1 ships a flat list).
- Multi-root trust (R2; R1 is single-root).
- Cross-organization trust roots (R3; the chain depth cap = 2 is the
  R1 ceiling).

## Consequences

Positive:
- **Revocation is now possible.** A compromised `NodeAuthority` can be
  revoked at any time by the operator appending to the TrustRoot-signed
  revocation list. A node trying to use the revoked key gets
  `Err(TrustError::KeyRevoked)` instead of being trusted.
- **Chain depth is bounded.** Cap = 2 prevents accidental infinite
  chains and forces the operator to think about trust segmentation.
- **Time-bounded validity.** `not_after` lets a fleet rotate keys on a
  schedule instead of waiting for an incident.
- **Backwards compatible.** R0 callers keep working; new callers opt in.

Negative:
- **Boilerplate at the call site.** Constructing a `TrustStore`
  requires knowing the TrustRoot key. This is documented but adds
  friction for small-scale deployments.
- **Clock dependency.** `not_after` requires wall-clock time. A node
  with a wildly skewed clock will reject otherwise-valid signatures.
  Mitigated by `Authority` carrying `issued_at`; the verifier falls
  back to checking only revocation if the clock is unavailable (R2).
- **Chain depth = 2 limits multi-org federation.** Acceptable for R1;
  the depth cap is configurable (compile-time constant) for R2.

## Alternatives considered

1. **Keep direct-key forever, add a "known-bad" list as a sidecar JSON.**
   Rejected: doesn't scale to fleet-segmentation (every node carries the
   same list, no room for per-segment CAs).
2. **Web PKI / X.509 chain.** Rejected: massive surface area, no benefit
   for our Ed25519-only model, and pulls in a whole ASN.1 dependency.
3. **Per-descriptor trust attestation attached by the consumer.** Rejected:
   shifts the trust burden to every consumer; the TrustRoot model is a
   well-understood one-write-many-reads pattern.
4. **Trust-on-first-use (TOFU) with manual pin.** Rejected: requires
   operator intervention on every new node. Breaks the auto-admit flow
   spec 018/019 rely on.

## Compliance

- ADR-0023 signature format is the wire contract; this ADR does not
  change it.
- ADR-0028 fixture-verification gate is unchanged; `verify_chain` does
  not run against fixtures because fixtures don't carry the chain
  metadata.
- `fabric-graph::leases::rebind_or_fail` (commit `70146c1`) does NOT
  call `verify_chain` — that is the R2 topology-validation hook.
- PF-WP-016 (trust-root) and PF-WP-022 (route leases) are separate WPs;
  this ADR closes PF-WP-016.

## Rollout

- ADR-0031 lands first (this doc).
- `crates/fabric-capability/src/trust_root.rs` lands second.
- `tests/trust_root_chain.rs` integration tests land third (3 tests).
- 5 unit tests in `trust_root.rs` pass.
- 4/4 spec checks remain green; MANIFEST is regen'd.
- Commit + WORKLOG addendum + meta/PHENOTYPE_ARCHITECTURE.md addendum.
- Spec 021 documents the contract that this ADR enforces.

The actual Rust implementation lands in this same turn per the
operator direction "poc on all".
