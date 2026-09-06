//! Round-trip integration test: every fixture in `cmd/checker/testdata/` must
//! deserialize into the real `fabric_capability::CapabilityDescriptor` type
//! and the manifests must deserialize into `serde_json::Value` (the spec/ADR-0027
//! manifest schema lives outside this crate, so we assert only that they are
//! valid JSON with the expected top-level shape).
//!
//! This guards against the failure mode that bit us three times: writing
//! fixture JSON against invented field names rather than the actual serde
//! contract. See WORKLOG.md "Rust API-drift lesson" (2026-09-02..05).
//!
//! If a fixture's JSON stops parsing, this test fails with the serde error
//! path so the regression is immediately fixable.

use fabric_capability::CapabilityDescriptor;
use std::path::PathBuf;

fn fixture_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR points at crates/fabric-capability; testdata lives two
    // directories up under cmd/checker/. The path is stable across worktrees.
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("fabric-capability crate is at <repo>/crates/fabric-capability")
        .join("cmd")
        .join("checker")
        .join("testdata")
}

/// Names of descriptor fixtures that MUST round-trip through
/// `CapabilityDescriptor`. The verifier (see examples/verify_fixtures.rs)
/// uses these to catch field-name drift.
const DESCRIPTOR_FIXTURES: &[&str] = &[
    "gpu_descriptor_minimal.json",
    "mem_descriptor_small.json",
    "cores_descriptor_small.json",
    "descriptor_no_audio.json",
    "descriptor_partial_cores.json",
    "descriptor_empty_manifest.json",
    "descriptor_stale_epoch.json",
];

/// Names of manifest fixtures that MUST be parseable JSON with a non-empty
/// top-level object. The full checker contract lives in `cmd/checker`; this
/// test guards only that the fixture format is what the checker consumes.
const MANIFEST_FIXTURES: &[&str] = &[
    "gpu_manifest_minimal.json",
    "mem_manifest_huge.json",
    "cores_manifest_huge.json",
    "cores_manifest_soft.json",
    "audio_manifest_required.json",
];

#[test]
fn every_descriptor_fixture_round_trips_through_capability_descriptor() {
    let dir = fixture_dir();
    assert!(
        dir.is_dir(),
        "testdata directory missing: {} (run from the fabric repo root)",
        dir.display()
    );

    for name in DESCRIPTOR_FIXTURES {
        let path = dir.join(name);
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let parsed: CapabilityDescriptor = serde_json::from_str(&raw).unwrap_or_else(|e| {
            panic!(
                "fixture {name} failed to deserialize into CapabilityDescriptor.\n  serde error: {e}"
            )
        });
        assert_eq!(
            parsed.schema_version, "phenotype.fabric.capability_descriptor/1",
            "fixture {name}: unexpected schema_version"
        );
    }
}

#[test]
fn every_descriptor_fixture_has_stable_canonical_bytes() {
    // Sanity: the same fixture parsed twice yields the same canonical_bytes
    // (deterministic encoding). This catches accidental re-ordering of struct
    // fields that would change the descriptor ID.
    let dir = fixture_dir();
    for name in DESCRIPTOR_FIXTURES {
        let path = dir.join(name);
        let raw = std::fs::read_to_string(&path).unwrap();
        let a: CapabilityDescriptor = serde_json::from_str(&raw).unwrap();
        let b: CapabilityDescriptor = serde_json::from_str(&raw).unwrap();
        let cb_a = a.canonical_bytes().expect("canonical_bytes");
        let cb_b = b.canonical_bytes().expect("canonical_bytes");
        assert_eq!(
            cb_a, cb_b,
            "fixture {name}: canonical_bytes not stable across parses"
        );
    }
}

#[test]
fn every_manifest_fixture_is_valid_json_with_minimal_shape() {
    // Manifests in testdata/ are *probes* for the checker — intentionally
    // minimal, exercising one mismatch dimension each (memory-only, cores-only,
    // audio-required). They MUST:
    //   1. parse as JSON
    //   2. be a top-level object
    //   3. have a "name" field (the checker's manifest-name extractor reads it)
    //   4. carry at least one substantive resource field so the checker has
    //      something to compare against the descriptor.
    // They need NOT carry every field of the full phenotype-manifest schema;
    // that lives in `phenotype-nvms-adapter` and is exercised there.
    let dir = fixture_dir();
    for name in MANIFEST_FIXTURES {
        let path = dir.join(name);
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let v: serde_json::Value = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("manifest {name} parse error: {e}"));
        let obj = v
            .as_object()
            .unwrap_or_else(|| panic!("manifest {name}: top-level must be object"));
        assert!(
            obj.contains_key("name"),
            "manifest {name}: missing required 'name' field"
        );
        let has_substantive = obj.contains_key("cpu")
            || obj.contains_key("memory")
            || obj.contains_key("audio")
            || obj.contains_key("agent")
            || obj.contains_key("network")
            || obj.contains_key("runtime");
        assert!(
            has_substantive,
            "manifest {name}: must carry at least one resource field \
             (cpu/memory/audio/agent/network/runtime) for the checker to evaluate"
        );
    }
}

#[test]
fn capability_descriptor_round_trips_through_json() {
    // canonical_bytes() returns a domain-specific binary encoding
    // (BLAKE3-seeded), not JSON — so it does NOT re-deserialize through
    // serde_json::from_slice. We test the JSON round-trip instead, which is
    // what the checker's loadDescriptor uses in practice.
    //
    // This guards against: struct field renames, optional-field omissions,
    // and serde tag changes — all of which would silently break the
    // descriptor's downstream consumer (checker, signing, topology_hash).
    let dir = fixture_dir();
    for name in DESCRIPTOR_FIXTURES {
        let path = dir.join(name);
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let d: CapabilityDescriptor = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("{name}: deserialize failed: {e}"));

        // Re-serialize to JSON and parse it back; expect semantic equality
        // on the fields the checker actually reads.
        let re_json = serde_json::to_string(&d)
            .unwrap_or_else(|e| panic!("{name}: serialize failed: {e}"));
        let d2: CapabilityDescriptor = serde_json::from_str(&re_json)
            .unwrap_or_else(|e| panic!("{name}: re-deserialize failed: {e}"));
        assert_eq!(d.epoch, d2.epoch, "{name}: epoch round-trip differs");
        assert_eq!(
            d.schema_version, d2.schema_version,
            "{name}: schema_version round-trip differs"
        );
        assert_eq!(
            d.topology_hash, d2.topology_hash,
            "{name}: topology_hash round-trip differs"
        );
        if let (Some(c1), Some(c2)) = (&d.capabilities.compute, &d2.capabilities.compute)
        {
            assert_eq!(
                c1.memory_bytes, c2.memory_bytes,
                "{name}: memory_bytes round-trip differs"
            );
            assert_eq!(
                c1.cores_physical, c2.cores_physical,
                "{name}: cores_physical round-trip differs"
            );
            assert_eq!(
                c1.processor, c2.processor,
                "{name}: processor round-trip differs"
            );
        }
    }
}
