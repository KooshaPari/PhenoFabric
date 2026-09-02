//! Map an `odin.nvms` v0.2 manifest to the Fabric capabilities a
//! target host must have to satisfy it.

use fabric_capability::descriptor::{
    AudioCapabilities, ComputeCapabilities, NetworkCapabilities,
};
use phenotype_manifest::Manifest;
use thiserror::Error;

/// Result of mapping an NVMS manifest to a required-capabilities block.
#[derive(Debug, Clone)]
pub struct RequiredCapabilities {
    /// Compute shape the host must satisfy.
    pub compute: ComputeCapabilities,
    /// Network shape the host must satisfy.
    pub network: NetworkCapabilities,
    /// Optional: if the manifest exposes agent/MCP tools, the host
    /// must run an audio backend (used for spoken feedback in some
    /// agent workflows).
    pub audio: Option<AudioCapabilities>,
}

/// Errors produced by [`required_capabilities`].
#[derive(Debug, Error)]
pub enum RequiredCapabilitiesError {
    /// The manifest is valid JSON and parses, but cannot be mapped
    /// to a required-capabilities block (e.g. missing fields).
    #[error("manifest cannot be mapped to required capabilities: {0}")]
    Invalid(String),
}

/// Map an `odin.nvms` v0.2 manifest to a Fabric `RequiredCapabilities`.
///
/// The mapping is *intentionally lossy*: an NVMS manifest only declares
/// what the application *wants*, not what the host *has*. The route
/// compiler will later intersect this with a real `CapabilityDescriptor`.
///
/// # Errors
///
/// Returns [`RequiredCapabilitiesError::Invalid`] if the manifest is
/// missing a field that the mapping needs. (Currently the mapping is
/// always defined for well-formed v0.2 manifests, so this is a
/// forward-compat error for future v0.3+ fields.)
///
/// # Example
///
/// ```
/// use phenotype_manifest::Manifest;
/// use phenotype_nvms_adapter::required_capabilities;
///
/// let json = r#"{
///   "app":   {"name": "demo", "runtime": "node"},
///   "infra": {"engine": "docker", "resources": {"memory": "512Mi"}}
/// }"#;
/// let m: Manifest = serde_json::from_str(json).expect("manifest parse");
/// let req = required_capabilities(&m).expect("map");
/// assert_eq!(req.compute.cores_physical, 1); // default minimum
/// ```
pub fn required_capabilities(
    manifest: &Manifest,
) -> Result<RequiredCapabilities, RequiredCapabilitiesError> {
    // CPU: derive from infra.resources.cpu (JSON value) or default to 1 core.
    let cores_physical: u32 = match manifest.infra.resources.as_ref() {
        Some(r) => match r.cpu.as_ref() {
            Some(serde_json::Value::Number(n)) => n
                .as_u64()
                .map(|v| v as u32)
                .ok_or_else(|| {
                    RequiredCapabilitiesError::Invalid(format!(
                        "infra.resources.cpu is not an integer: {}",
                        n
                    ))
                })?,
            Some(serde_json::Value::String(s)) => s.parse::<u32>().map_err(|_| {
                RequiredCapabilitiesError::Invalid(format!(
                    "infra.resources.cpu is a string but not parseable as integer: {}",
                    s
                ))
            })?,
            Some(_) => {
                return Err(RequiredCapabilitiesError::Invalid(
                    "infra.resources.cpu must be a number or numeric string".into(),
                ));
            }
            None => 1,
        },
        None => 1,
    };

    // Memory: parse k8s-style resource strings (Ki, Mi, Gi, Ti, K, M, G, T)
    // or treat as raw bytes if no suffix.
    let memory_bytes: u64 = match manifest.infra.resources.as_ref() {
        Some(r) => match r.memory.as_ref() {
            Some(s) => parse_k8s_memory(s).map_err(|e| {
                RequiredCapabilitiesError::Invalid(format!(
                    "infra.resources.memory: {}",
                    e
                ))
            })?,
            None => 256 * 1024 * 1024, // 256 MiB default minimum
        },
        None => 256 * 1024 * 1024,
    };

    // Network: from manifest.network.ports (host must accept traffic
    // on these ports — Fabric surface plane will map them).
    let mut interfaces = Vec::new();
    if let Some(net) = &manifest.network {
        for port in &net.ports {
            interfaces.push(
                fabric_capability::descriptor::NetworkInterface {
                    name: format!("nvms-port-{}", port),
                    mac_address: None,
                    link_speed_mbps: None,
                    mtu: 1500,
                    rdma_capable: false,
                    zerocopy_capable: false,
                    rss_queues: 1,
                    ipv4: None,
                    ipv6: None,
                },
            );
            // Note: the `port` itself is encoded via the interface name
            // and surfaced to the route compiler as a port allocation
            // request; the descriptor struct doesn't have a port field
            // (that's a surface-plane concern, R1/PF-WP-030).
            let _ = port;
        }
    }

    // Agent section: presence of MCP tools or A2A skills implies a
    // minimal audio backend (for spoken feedback in agent workflows).
    let audio = if manifest
        .agent
        .as_ref()
        .map(|a| !a.mcp_tools.is_empty() || !a.a2a_skills.is_empty())
        .unwrap_or(false)
    {
        Some(AudioCapabilities {
            backend: "alsa-stub".to_string(),
            sinks: vec![],
            sources: vec![],
            midi_ports: 0,
        })
    } else {
        None
    };

    Ok(RequiredCapabilities {
        compute: ComputeCapabilities {
            processor: format!("required-for-{}", manifest.app.name),
            cores_physical,
            cores_logical: cores_physical, // conservative
            numa_nodes: 1,                 // conservative
            cache: vec![],
            memory_bytes,
            memory_bandwidth_mbps: None,
            hyperthread_pairs: vec![],
            tdp_watts: None,
        },
        network: NetworkCapabilities { interfaces },
        audio,
    })
}

/// Parse a k8s-style memory string into bytes.
///
/// Supports: Ki, Mi, Gi, Ti, Pi, Ei (binary) and K, M, G, T, P, E (decimal).
/// Falls back to raw bytes if no suffix.
fn parse_k8s_memory(s: &str) -> Result<u64, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty string".into());
    }
    let (num_str, multiplier) = if let Some(stripped) = s.strip_suffix("Ki") {
        (stripped, 1024u64)
    } else if let Some(stripped) = s.strip_suffix("Mi") {
        (stripped, 1024u64 * 1024)
    } else if let Some(stripped) = s.strip_suffix("Gi") {
        (stripped, 1024u64 * 1024 * 1024)
    } else if let Some(stripped) = s.strip_suffix("Ti") {
        (stripped, 1024u64 * 1024 * 1024 * 1024)
    } else if let Some(stripped) = s.strip_suffix("Pi") {
        (stripped, 1024u64 * 1024 * 1024 * 1024 * 1024)
    } else if let Some(stripped) = s.strip_suffix("Ei") {
        (stripped, 1024u64 * 1024 * 1024 * 1024 * 1024 * 1024)
    } else if let Some(stripped) = s.strip_suffix('K') {
        (stripped, 1000u64)
    } else if let Some(stripped) = s.strip_suffix('M') {
        (stripped, 1000u64 * 1000)
    } else if let Some(stripped) = s.strip_suffix('G') {
        (stripped, 1000u64 * 1000 * 1000)
    } else if let Some(stripped) = s.strip_suffix('T') {
        (stripped, 1000u64 * 1000 * 1000 * 1000)
    } else if let Some(stripped) = s.strip_suffix('P') {
        (stripped, 1000u64 * 1000 * 1000 * 1000 * 1000)
    } else if let Some(stripped) = s.strip_suffix('E') {
        (stripped, 1000u64 * 1000 * 1000 * 1000 * 1000 * 1000)
    } else {
        (s, 1u64)
    };
    num_str
        .parse::<u64>()
        .map(|n| n * multiplier)
        .map_err(|e| format!("cannot parse number '{}': {}", num_str, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_manifest() -> Manifest {
        let json = r#"{
            "app":   {"name": "demo", "runtime": "node"},
            "infra": {"engine": "docker"}
        }"#;
        serde_json::from_str(json).expect("parse minimal manifest")
    }

    #[test]
    fn minimal_manifest_maps_to_minimum_capabilities() {
        let m = minimal_manifest();
        let req = required_capabilities(&m).expect("map");
        assert_eq!(req.compute.cores_physical, 1);
        assert_eq!(req.compute.memory_bytes, 256 * 1024 * 1024);
        assert!(req.network.interfaces.is_empty());
        assert!(req.audio.is_none());
    }

    #[test]
    fn memory_gi_parses() {
        let json = r#"{
            "app":   {"name": "demo"},
            "infra": {"engine": "docker", "resources": {"memory": "1Gi"}}
        }"#;
        let m: Manifest = serde_json::from_str(json).expect("parse");
        let req = required_capabilities(&m).expect("map");
        assert_eq!(req.compute.memory_bytes, 1024 * 1024 * 1024);
    }

    #[test]
    fn memory_mi_parses() {
        let json = r#"{
            "app":   {"name": "demo"},
            "infra": {"engine": "docker", "resources": {"memory": "512Mi"}}
        }"#;
        let m: Manifest = serde_json::from_str(json).expect("parse");
        let req = required_capabilities(&m).expect("map");
        assert_eq!(req.compute.memory_bytes, 512 * 1024 * 1024);
    }

    #[test]
    fn cpu_as_number() {
        let json = r#"{
            "app":   {"name": "demo"},
            "infra": {"engine": "docker", "resources": {"cpu": 4}}
        }"#;
        let m: Manifest = serde_json::from_str(json).expect("parse");
        let req = required_capabilities(&m).expect("map");
        assert_eq!(req.compute.cores_physical, 4);
    }

    #[test]
    fn network_ports_become_interfaces() {
        let json = r#"{
            "app":     {"name": "demo"},
            "infra":   {"engine": "docker"},
            "network": {"ports": [80, 443], "domains": []}
        }"#;
        let m: Manifest = serde_json::from_str(json).expect("parse");
        let req = required_capabilities(&m).expect("map");
        assert_eq!(req.network.interfaces.len(), 2);
        assert_eq!(req.network.interfaces[0].name, "nvms-port-80");
        assert_eq!(req.network.interfaces[1].name, "nvms-port-443");
    }

    #[test]
    fn agent_section_implies_audio_backend() {
        let json = r#"{
            "app":   {"name": "demo"},
            "infra": {"engine": "docker"},
            "agent": {"mcp_tools": ["foo"]}
        }"#;
        let m: Manifest = serde_json::from_str(json).expect("parse");
        let req = required_capabilities(&m).expect("map");
        assert!(req.audio.is_some());
        assert_eq!(req.audio.unwrap().backend, "alsa-stub");
    }

    #[test]
    fn empty_agent_section_no_audio() {
        let json = r#"{
            "app":   {"name": "demo"},
            "infra": {"engine": "docker"},
            "agent": {"mcp_tools": [], "a2a_skills": []}
        }"#;
        let m: Manifest = serde_json::from_str(json).expect("parse");
        let req = required_capabilities(&m).expect("map");
        assert!(req.audio.is_none());
    }

    #[test]
    fn invalid_cpu_returns_error() {
        let json = r#"{
            "app":   {"name": "demo"},
            "infra": {"engine": "docker", "resources": {"cpu": "not-a-number"}}
        }"#;
        let m: Manifest = serde_json::from_str(json).expect("parse");
        let err = required_capabilities(&m).unwrap_err();
        assert!(matches!(err, RequiredCapabilitiesError::Invalid(_)));
    }
}
