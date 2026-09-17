use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Maximum chain depth in R1. Compile-time constant; R2 may make it
/// configurable per-deployment.
pub const MAX_CHAIN_DEPTH: usize = 2;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors that [`super::TrustStore`] can produce during construction, authority
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
/// the TrustRoot's signing key; `super::TrustStore::set_revocation_list`
/// enforces this.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevocationList {
    pub revocations: Vec<RevocationEntry>,
    pub signed_at: DateTime<Utc>,
    pub signature: crate::descriptor::Signature,
}

impl RevocationList {
    /// Builds, signs (by the supplied root signing key), and returns a
    /// fresh `RevocationList`. The signature is over the canonical bytes
    /// of `(revocations, signed_at)` — same "no self-signature" pattern
    /// as `CapabilityDescriptor::canonical_bytes` and
    /// `super::Authority::canonical_bytes`.
    pub fn build_and_sign(
        revocations: Vec<RevocationEntry>,
        root_signing_key: &crate::signing::SigningKey,
    ) -> Result<Self> {
        let signed_at = Utc::now();
        // Build a stub for canonicalization (without signature).
        let stub = Self {
            revocations: revocations.clone(),
            signed_at,
            signature: crate::descriptor::Signature {
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
