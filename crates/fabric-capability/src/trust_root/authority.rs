use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::descriptor::Signature;
use crate::error::Result;
use crate::signing::{SigningKey, VerificationKey};

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
        let mut value =
            serde_json::to_value(self).map_err(|e| crate::error::Error::Serde(e.to_string()))?;
        if let Some(obj) = value.as_object_mut() {
            obj.remove("signature");
        }
        serde_json::to_vec(&value).map_err(|e| crate::error::Error::Serde(e.to_string()))
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
    /// construction-site convenience. `super::TrustStore::add_authority`
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
