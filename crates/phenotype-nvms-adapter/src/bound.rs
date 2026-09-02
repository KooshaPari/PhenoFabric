//! Bind an `odin.nvms` manifest to a target host and produce a
//! `BoundManifest` (the deployment runtime's input).

use chrono::{DateTime, Utc};
use fabric_capability::descriptor::CapabilityDescriptor;
use phenotype_manifest::Manifest;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// A manifest bound to a target host, signed by the manifest author,
/// witnessed by the host.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct BoundManifest {
    /// Unique ID for this binding (UUIDv7).
    pub binding_id: Uuid,
    /// The host this manifest is bound to.
    pub host_node_id: Uuid,
    /// The host's descriptor epoch at the time of binding.
    pub host_epoch: u64,
    /// The host's topology hash at the time of binding.
    pub host_topology_hash: String,
    /// The manifest being bound.
    pub manifest: serde_json::Value,
    /// When the binding was created.
    pub bound_at: DateTime<Utc>,
    /// Who/what created the binding (operator, agent, scheduler).
    pub bound_by: String,
    /// The manifest author's signature over (binding_id, host_node_id,
    /// host_epoch, host_topology_hash, manifest).
    pub author_signature: Option<SignatureEntry>,
    /// The host's witness signature over the same fields.
    pub host_witness: Option<SignatureEntry>,
}

/// A signature in the `BoundManifest`.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SignatureEntry {
    pub key_id: String,
    pub alg: String,
    pub sig: String,
    pub signed_at: DateTime<Utc>,
}

/// Errors produced by the bound-manifest builder.
#[derive(Debug, Error)]
pub enum BoundManifestError {
    /// The manifest is invalid (caught upstream; surfaced here for
    /// forward compat with v0.3+ validation).
    #[error("invalid manifest: {0}")]
    Invalid(String),
    /// The host descriptor is not compatible with the manifest's
    /// required capabilities.
    #[error("host does not satisfy manifest requirements: {0}")]
    Unsatisfied(String),
}

/// Builder for `BoundManifest`.
///
/// The builder pattern makes it impossible to construct a `BoundManifest`
/// without both a manifest and a host descriptor, and lets us add
/// signatures as a separate step after construction.
pub struct BoundManifestBuilder {
    manifest: Manifest,
    host: CapabilityDescriptor,
    bound_by: String,
    author_signature: Option<SignatureEntry>,
    host_witness: Option<SignatureEntry>,
}

impl BoundManifestBuilder {
    /// Create a new builder.
    pub fn new(manifest: Manifest, host: CapabilityDescriptor, bound_by: impl Into<String>) -> Self {
        Self {
            manifest,
            host,
            bound_by: bound_by.into(),
            author_signature: None,
            host_witness: None,
        }
    }

    /// Set the manifest author's signature.
    pub fn with_author_signature(mut self, sig: SignatureEntry) -> Self {
        self.author_signature = Some(sig);
        self
    }

    /// Set the host's witness signature.
    pub fn with_host_witness(mut self, sig: SignatureEntry) -> Self {
        self.host_witness = Some(sig);
        self
    }

    /// Build the `BoundManifest`.
    ///
    /// Validates that the host descriptor is compatible with the
    /// manifest's required capabilities.
    pub fn build(self) -> Result<BoundManifest, BoundManifestError> {
        // Validate the host can satisfy the manifest's requirements.
        let req = crate::required::required_capabilities(&self.manifest)
            .map_err(|e| BoundManifestError::Invalid(e.to_string()))?;

        // Memory check: host memory must be >= required memory.
        let host_memory = self
            .host
            .capabilities
            .compute
            .as_ref()
            .map(|c| c.memory_bytes)
            .unwrap_or(0);
        if host_memory < req.compute.memory_bytes {
            return Err(BoundManifestError::Unsatisfied(format!(
                "host memory {} bytes < required {} bytes",
                host_memory, req.compute.memory_bytes
            )));
        }

        // CPU check: host cores must be >= required cores.
        let host_cores = self
            .host
            .capabilities
            .compute
            .as_ref()
            .map(|c| c.cores_physical)
            .unwrap_or(0);
        if host_cores < req.compute.cores_physical {
            return Err(BoundManifestError::Unsatisfied(format!(
                "host cores {} < required {}",
                host_cores, req.compute.cores_physical
            )));
        }

        // Audio check: if manifest implies audio, host must have audio.
        if req.audio.is_some() && self.host.capabilities.audio.is_none() {
            return Err(BoundManifestError::Unsatisfied(
                "manifest requires audio backend, host has none".into(),
            ));
        }

        // Note: port/network compat check is deferred to R1 (PF-WP-030,
        // surface plane). For now, ports are advertised in
        // RequiredCapabilities but not yet enforced at binding time.

        let manifest_json = serde_json::to_value(&self.manifest)
            .map_err(|e| BoundManifestError::Invalid(e.to_string()))?;

        Ok(BoundManifest {
            binding_id: Uuid::now_v7(),
            host_node_id: self.host.node_id,
            host_epoch: self.host.epoch,
            host_topology_hash: self.host.topology_hash.clone(),
            manifest: manifest_json,
            bound_at: Utc::now(),
            bound_by: self.bound_by,
            author_signature: self.author_signature,
            host_witness: self.host_witness,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fabric_capability::descriptor::{
        AudioCapabilities, Capabilities, ComputeCapabilities,
    };

    fn minimal_host() -> CapabilityDescriptor {
        CapabilityDescriptor {
            node_id: Uuid::now_v7(),
            epoch: 1,
            schema_version: "1.0.0".into(),
            probed_at: Utc::now(),
            probe_version: "0.1.0".into(),
            topology_hash: "abc".into(),
            capabilities: Capabilities {
                compute: Some(ComputeCapabilities {
                    processor: "test".into(),
                    cores_physical: 4,
                    cores_logical: 4,
                    numa_nodes: 1,
                    cache: vec![],
                    memory_bytes: 1024 * 1024 * 1024, // 1 GiB
                    memory_bandwidth_mbps: None,
                    hyperthread_pairs: vec![],
                    tdp_watts: None,
                }),
                audio: Some(AudioCapabilities {
                    backend: "alsa".into(),
                    sinks: vec![],
                    sources: vec![],
                    midi_ports: 0,
                }),
                ..Default::default()
            },
            signatures: vec![],
        }
    }

    fn minimal_manifest() -> Manifest {
        let json = r#"{
            "app":   {"name": "demo"},
            "infra": {"engine": "docker"}
        }"#;
        serde_json::from_str(json).expect("parse")
    }

    #[test]
    fn build_succeeds_when_host_satisfies_minimum() {
        let m = minimal_manifest();
        let h = minimal_host();
        let bound = BoundManifestBuilder::new(m, h, "test")
            .build()
            .expect("build");
        assert_eq!(bound.host_epoch, 1);
        assert_eq!(bound.host_topology_hash, "abc");
        assert!(bound.author_signature.is_none());
    }

    #[test]
    fn build_fails_when_host_memory_too_small() {
        let m_json = r#"{
            "app":   {"name": "demo"},
            "infra": {"engine": "docker", "resources": {"memory": "10Gi"}}
        }"#;
        let m: Manifest = serde_json::from_str(m_json).expect("parse");
        let mut h = minimal_host();
        h.capabilities.compute.as_mut().unwrap().memory_bytes = 1024 * 1024 * 1024; // 1 GiB
        let err = BoundManifestBuilder::new(m, h, "test").build().unwrap_err();
        assert!(matches!(err, BoundManifestError::Unsatisfied(_)));
    }

    #[test]
    fn build_fails_when_host_cores_too_few() {
        let m_json = r#"{
            "app":   {"name": "demo"},
            "infra": {"engine": "docker", "resources": {"cpu": 8}}
        }"#;
        let m: Manifest = serde_json::from_str(m_json).expect("parse");
        let h = minimal_host(); // 4 cores
        let err = BoundManifestBuilder::new(m, h, "test").build().unwrap_err();
        assert!(matches!(err, BoundManifestError::Unsatisfied(_)));
    }

    #[test]
    fn build_fails_when_manifest_needs_audio_but_host_has_none() {
        let m_json = r#"{
            "app":   {"name": "demo"},
            "infra": {"engine": "docker"},
            "agent": {"mcp_tools": ["x"]}
        }"#;
        let m: Manifest = serde_json::from_str(m_json).expect("parse");
        let mut h = minimal_host();
        h.capabilities.audio = None;
        let err = BoundManifestBuilder::new(m, h, "test").build().unwrap_err();
        assert!(matches!(err, BoundManifestError::Unsatisfied(_)));
    }

    #[test]
    fn signatures_attach_when_provided() {
        let m = minimal_manifest();
        let h = minimal_host();
        let sig = SignatureEntry {
            key_id: "key-1".into(),
            alg: "ed25519".into(),
            sig: "deadbeef".into(),
            signed_at: Utc::now(),
        };
        let bound = BoundManifestBuilder::new(m, h, "test")
            .with_author_signature(sig.clone())
            .with_host_witness(sig)
            .build()
            .expect("build");
        assert!(bound.author_signature.is_some());
        assert!(bound.host_witness.is_some());
    }
}
