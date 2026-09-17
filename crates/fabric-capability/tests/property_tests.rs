//! Property-based tests for the Fabric capability descriptor.
//!
//! Uses `proptest` to verify roundtrip serialization and locality tier
//! index stability for arbitrary inputs.

use fabric_capability::descriptor::{Capabilities, CapabilityDescriptor};
use fabric_capability::locality::LocalityTier;
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Strategy: generate an arbitrary CapabilityDescriptor
// ---------------------------------------------------------------------------

fn arb_uuid() -> impl Strategy<Value = uuid::Uuid> {
    // Use arbitrary bytes to construct a UUID.
    ("[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}")
        .prop_map(|s| uuid::Uuid::parse_str(&s).unwrap_or_else(|_| uuid::Uuid::nil()))
}

fn arb_descriptor() -> impl Strategy<Value = CapabilityDescriptor> {
    (
        arb_uuid(),
        0u64..u64::MAX,
        "[a-z0-9\\-]{1,20}",
        "[a-z0-9]{1,40}",
    )
        .prop_map(
            |(node_id, epoch, schema_version, topology_hash)| CapabilityDescriptor {
                node_id,
                epoch,
                schema_version,
                probed_at: chrono::Utc::now(),
                probe_version: "proptest-0.1.0".into(),
                topology_hash,
                capabilities: Capabilities::default(),
                signatures: vec![],
            },
        )
}

// ---------------------------------------------------------------------------
// Property: serialize -> deserialize of CapabilityDescriptor preserves all fields
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn descriptor_roundtrip_preserves_fields(desc in arb_descriptor()) {
        let json = serde_json::to_string(&desc).expect("serialization should succeed");
        let restored: CapabilityDescriptor = serde_json::from_str(&json)
            .expect("deserialization should succeed");

        prop_assert_eq!(desc.node_id, restored.node_id);
        prop_assert_eq!(desc.epoch, restored.epoch);
        prop_assert_eq!(desc.schema_version, restored.schema_version);
        prop_assert_eq!(desc.probe_version, restored.probe_version);
        prop_assert_eq!(desc.topology_hash, restored.topology_hash);
        // capabilities default is the same
        prop_assert_eq!(
            desc.capabilities.compute.is_some(),
            restored.capabilities.compute.is_some()
        );
        prop_assert_eq!(
            desc.capabilities.accelerator.is_some(),
            restored.capabilities.accelerator.is_some()
        );
        prop_assert_eq!(
            desc.capabilities.display.is_some(),
            restored.capabilities.display.is_some()
        );
        prop_assert_eq!(
            desc.capabilities.pcie.is_some(),
            restored.capabilities.pcie.is_some()
        );
        prop_assert_eq!(
            desc.capabilities.audio.is_some(),
            restored.capabilities.audio.is_some()
        );
        prop_assert_eq!(
            desc.capabilities.storage.is_some(),
            restored.capabilities.storage.is_some()
        );
        prop_assert_eq!(
            desc.capabilities.network.is_some(),
            restored.capabilities.network.is_some()
        );
        prop_assert_eq!(
            desc.capabilities.topology.is_some(),
            restored.capabilities.topology.is_some()
        );
        prop_assert_eq!(desc.signatures.len(), restored.signatures.len());
    }
}

// ---------------------------------------------------------------------------
// Property: LocalityTier from_index(index) -> short_code roundtrip is stable
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn locality_tier_index_roundtrip(index in 0u8..=8) {
        let tier = LocalityTier::from_index(index);
        prop_assert!(tier.is_some(), "from_index({index}) should return Some");

        let tier = tier.unwrap();
        let short_code = tier.short_code();

        // Short code should be exactly 2 characters starting with 'L'.
        prop_assert_eq!(short_code.len(), 2);
        prop_assert_eq!(&short_code[0..1], "L");

        // The short code should parse back to the same tier.
        let parsed: Result<LocalityTier, _> = short_code.parse();
        prop_assert!(parsed.is_ok(), "short_code '{}' should parse", short_code);
        prop_assert_eq!(parsed.unwrap(), tier);
    }
}

// ---------------------------------------------------------------------------
// Property: LocalityTier from_index out-of-range returns None
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn locality_tier_invalid_index_returns_none(index in 9u8..=255u8) {
        prop_assert!(
            LocalityTier::from_index(index).is_none(),
            "from_index({index}) should return None"
        );
    }
}

// ---------------------------------------------------------------------------
// Property: LocalityTier as_f64 is consistent with index
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn locality_tier_as_f64_matches_index(index in 0u8..=8u8) {
        let tier = LocalityTier::from_index(index).unwrap();
        prop_assert_eq!(tier.as_f64(), f64::from(index));
    }
}

// ---------------------------------------------------------------------------
// Property: LocalityTier short_code is unique per tier
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn locality_tier_short_codes_are_unique(a in 0u8..=8u8, b in 0u8..=8u8) {
        let tier_a = LocalityTier::from_index(a).unwrap();
        let tier_b = LocalityTier::from_index(b).unwrap();

        if a == b {
            prop_assert_eq!(tier_a.short_code(), tier_b.short_code());
        } else {
            prop_assert_ne!(tier_a.short_code(), tier_b.short_code());
        }
    }
}

// ---------------------------------------------------------------------------
// Property: JSON roundtrip for non-default Capabilities
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn capabilities_json_roundtrip(
        compute_cores in 1u32..256,
        memory_gb in 1u64..1024,
    ) {
        use fabric_capability::descriptor::{ComputeCapabilities, CacheInfo};

        let caps = Capabilities {
            compute: Some(ComputeCapabilities {
                processor: "proptest-cpu".into(),
                cores_physical: compute_cores,
                cores_logical: compute_cores * 2,
                numa_nodes: 1,
                cache: vec![CacheInfo {
                    level: 3,
                    size_bytes: 16 * 1024 * 1024,
                    line_size_bytes: 64,
                    cores_sharing: compute_cores,
                    numa_node: None,
                }],
                memory_bytes: memory_gb * 1024 * 1024 * 1024,
                memory_bandwidth_mbps: None,
                hyperthread_pairs: vec![],
                tdp_watts: None,
            }),
            ..Default::default()
        };

        let json = serde_json::to_string(&caps).expect("serialization should succeed");
        let restored: Capabilities = serde_json::from_str(&json)
            .expect("deserialization should succeed");

        let compute = restored.compute.expect("compute should be present");
        prop_assert_eq!(compute.cores_physical, compute_cores);
        prop_assert_eq!(compute.cores_logical, compute_cores * 2);
        prop_assert_eq!(compute.memory_bytes, memory_gb * 1024 * 1024 * 1024);
        prop_assert_eq!(compute.cache.len(), 1);
        prop_assert_eq!(compute.cache[0].level, 3);
    }
}
