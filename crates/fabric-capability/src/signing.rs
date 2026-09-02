//! Ed25519 signing and verification for capability descriptors.

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use ed25519_dalek::{Signature as DalekSig, Signer, SigningKey as DalekSigningKey, Verifier};
use rand::rngs::OsRng;

use crate::descriptor::{CapabilityDescriptor, Signature};
use crate::error::{Error, Result};

/// A signing key — keep this secret on the node.
#[derive(Clone)]
pub struct SigningKey {
    inner: DalekSigningKey,
    key_id: String,
}

/// A verification key — can be shared with peers.
#[derive(Clone)]
pub struct VerificationKey {
    inner: ed25519_dalek::VerifyingKey,
    key_id: String,
}

impl SigningKey {
    /// Generates a new random signing key.
    pub fn generate() -> Self {
        let inner = DalekSigningKey::generate(&mut OsRng);
        let key_id = B64.encode(inner.to_bytes());
        Self { inner, key_id }
    }

    /// Deserializes a signing key from a 32-byte seed.
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        let inner = DalekSigningKey::from_bytes(bytes);
        let key_id = B64.encode(bytes);
        Self { inner, key_id }
    }

    /// Returns the key fingerprint (base64 of the 32-byte key).
    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    /// Returns the corresponding verification key.
    pub fn verification_key(&self) -> VerificationKey {
        VerificationKey {
            inner: (&self.inner).into(),
            key_id: self.key_id.clone(),
        }
    }

    /// Serializes the signing key as 32 bytes.
    pub fn to_bytes(&self) -> [u8; 32] {
        self.inner.to_bytes()
    }
}

impl VerificationKey {
    /// Deserializes a verification key from 32 bytes.
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        let inner = ed25519_dalek::VerifyingKey::from_bytes(bytes).expect("invalid key");
        let key_id = B64.encode(bytes);
        Self { inner, key_id }
    }

    /// Returns the key fingerprint.
    pub fn key_id(&self) -> &str {
        &self.key_id
    }
}

/// Signs a capability descriptor with the given key.
pub fn sign(descriptor: &mut CapabilityDescriptor, key: &SigningKey) -> Result<()> {
    let bytes = descriptor
        .canonical_bytes()
        .map_err(|e| Error::Serde(e.to_string()))?;
    let sig = key.inner.sign(&bytes);
    descriptor.signatures.push(Signature {
        key_id: key.key_id.clone(),
        alg: "Ed25519".to_string(),
        sig: B64.encode(sig.to_bytes()),
        signed_at: chrono::Utc::now(),
    });
    Ok(())
}

/// Verifies the signatures on a descriptor against the given key.
pub fn verify(descriptor: &CapabilityDescriptor, key: &VerificationKey) -> Result<()> {
    if descriptor.signatures.is_empty() {
        return Err(Error::Signature("no signatures on descriptor".into()));
    }
    let bytes = descriptor
        .canonical_bytes()
        .map_err(|e| Error::Serde(e.to_string()))?;
    for sig_entry in &descriptor.signatures {
        if sig_entry.key_id != key.key_id {
            continue;
        }
        let sig_bytes = B64
            .decode(&sig_entry.sig)
            .map_err(|e| Error::Crypto(format!("invalid base64: {e}")))?;
        let sig = DalekSig::from_slice(&sig_bytes)
            .map_err(|e| Error::Crypto(format!("invalid signature: {e}")))?;
        if key.inner.verify(&bytes, &sig).is_ok() {
            return Ok(());
        }
    }
    Err(Error::Signature(
        "no valid signature found for trusted key".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptor::Capabilities;

    fn test_descriptor() -> CapabilityDescriptor {
        CapabilityDescriptor {
            node_id: uuid::Uuid::now_v7(),
            epoch: 1,
            schema_version: "1.0.0".into(),
            probed_at: chrono::Utc::now(),
            probe_version: "0.1.0".into(),
            topology_hash: blake3::hash(&[]).to_hex().to_string(),
            capabilities: Capabilities::default(),
            signatures: vec![],
        }
    }

    #[test]
    fn sign_and_verify() {
        let key = SigningKey::generate();
        let vk = key.verification_key();
        let mut d = test_descriptor();
        sign(&mut d, &key).unwrap();
        assert_eq!(d.signatures.len(), 1);
        assert_eq!(d.signatures[0].alg, "Ed25519");
        verify(&d, &vk).unwrap();
    }

    #[test]
    fn verify_wrong_key_fails() {
        let key1 = SigningKey::generate();
        let key2 = SigningKey::generate();
        let vk1 = key1.verification_key();
        let mut d = test_descriptor();
        sign(&mut d, &key1).unwrap();
        verify(&d, &vk1).unwrap();
        let vk2 = key2.verification_key();
        assert!(verify(&d, &vk2).is_err());
    }

    #[test]
    fn verify_unsigned_fails() {
        let d = test_descriptor();
        let key = SigningKey::generate();
        let vk = key.verification_key();
        assert!(verify(&d, &vk).is_err());
    }

    #[test]
    fn signing_key_roundtrip() {
        let key = SigningKey::generate();
        let bytes = key.to_bytes();
        let restored = SigningKey::from_bytes(&bytes);
        assert_eq!(restored.key_id(), key.key_id());
    }
}
